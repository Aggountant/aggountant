use super::*;

/// Derive a period's VAT aggregates from its sealed vouchers (F4). Every posting
/// that carries a `vat_code` in a voucher whose `posting_date` maps to `period`
/// contributes its VAT to the sales or purchase total, with the rate resolved from
/// the code + the voucher's `document_date`. Sale-only tagged vouchers also populate
/// generic domestic rate-slot return fields from those same contributions.
/// A normal code levies VAT on top of the
/// base (`round(base × rate)`); a margin-scheme code (AVL 79 a–k §) instead extracts
/// it from *within* the VAT-inclusive margin the tagged amount carries (`round(margin
/// × rate / (100 + rate))`). Deterministic and **total** over well-formed EUR
/// vouchers; a foreign-currency VAT posting or an unresolvable code fails loudly
/// rather than being silently omitted.
///
/// This is a pure aggregator with **no** per-voucher structural policy: it must
/// stay total over already-sealed, immutable ledger data (a hard structural
/// refusal here would permanently brick a period, since a sealed voucher cannot
/// be edited out of the scan). The same-direction reverse-charge malformation
/// (F4) is instead refused at **record time** by [`check_voucher_vat_structure`],
/// which `build_draft` runs before a voucher can be sealed — so the malformed
/// booking never reaches this aggregate in the first place.
///
/// **Margin-scheme booking convention (important).** A `sales-margin-*` line's
/// amount is the VAT-**inclusive profit margin** (`selling − purchase`), *not* the
/// selling price. Deriving trusts that convention: if an operator tags the full
/// selling price with a margin code, this extracts VAT from the whole price and
/// **silently over-declares** output VAT (and, symmetrically, tagging the net-of-VAT
/// margin under-declares). The domain "never guesses" — it cannot see the true
/// selling/purchase prices from a single tagged amount, the same derive-from-what-
/// you-tag boundary every VAT figure here already sits behind. Use
/// [`compute_margin_vat`] to obtain the correct margin/base/VAT split before booking.
/// The durable fix — carrying the two prices as structured on-ledger data so the
/// margin is computed, not asserted — needs the model layer and is spun off as
/// `vat-margin-model-field`.
pub fn derive_vat_figures(
    period: &PeriodId,
    vouchers: &[VoucherBody],
) -> Result<DerivedVatFigures, DomainError> {
    let eur = Currency::new("EUR").expect("EUR is a valid currency code");
    let mut sales = Decimal::ZERO;
    let mut purchase = Decimal::ZERO;
    let mut fields = VatReturnFields::new();

    for voucher in vouchers {
        if !period.contains(voucher.posting_date()) {
            continue;
        }
        let mut domestic_sales_fields = VatReturnFields::new();
        let mut has_tagged_purchase = false;
        for posting in voucher.postings() {
            let Some(code) = &posting.vat_code else {
                continue;
            };
            if posting.amount.currency() != &eur {
                return Err(DomainError::VatCurrency(format!(
                    "VAT posting on voucher {} is {}, but derivation is EUR-only",
                    voucher.number(),
                    posting.amount.currency()
                )));
            }
            let (direction, rate_class, deductibility, scheme) = parse_vat_code(code.as_str())?;
            let pct = resolve_vat_rate(rate_class, voucher.document_date())?;
            // The posting amount is unsigned (direction lives in `Side`), so fold
            // the double-entry sign in: output VAT accrues on the credit side of a
            // sale and is *reduced* by a debit (a credit note); deductible input VAT
            // accrues on the debit side of a purchase and is reduced by a credit (a
            // return). Getting this wrong would silently over-report a period that
            // contains any correction — the exact case this cross-check must catch.
            let signed = signed_by_side(direction, posting.side, posting.amount.amount());
            // AVL 117 § partial deductibility scales the deductible input VAT by the
            // code's `-deduct<NN>` fraction. **Two rounding steps**: first round the
            // full invoice VAT (`round(base × rate)` — the concrete euro on the source
            // document, and what a fully-deductible code already yields), then round
            // the deductible share (`round(full_vat × fraction)`). This applies the
            // fraction to the *actual* invoice VAT rather than an unrounded theoretical
            // product, and guarantees deductible + non-deductible = full_vat exactly,
            // so the operator's cost split of the remainder always balances. `Full`
            // scales by ×1, so every existing sales-/purchase- code derives unchanged.
            // The scheme decides how the rate meets the base. Under the *normal*
            // scheme output/input VAT is levied on top of the base (`base × rate`).
            // Under the *margin* scheme (AVL 79 k §) the tagged amount is the VAT-
            // **inclusive** profit margin, so the VAT is extracted from within it
            // (`margin × rate / (100 + rate)`) — a smaller figure on the same base.
            //
            // `signed` already folds in the `Side`: a Debit-side margin line is a
            // **reversal** (a credit note / return of an earlier margin sale) and so
            // contributes *negatively*, exactly as a normal sales credit note does —
            // this is intentional and needed (a margin credit note must reduce the
            // period's output VAT). It is NOT the per-item "loss item" case: a loss on
            // a single resale (selling < purchase) yields **zero** margin VAT and books
            // nothing here — that AVL 79 k § clamp lives in [`compute_margin_vat`], the
            // pre-booking helper, not in this stateless aggregator. Capping an over-
            // reversal against the original item's VAT needs per-item identity (model
            // layer) and is deferred with the rest of the margin structural work
            // (`vat-margin-model-field`); the same item-identity gap already applies to
            // normal credit notes, so it is not new to the margin scheme.
            let full_vat = match scheme {
                VatScheme::Normal => round_eur(signed * pct / Decimal::from(100)),
                VatScheme::Margin(_) => round_eur(signed * pct / (Decimal::from(100) + pct)),
            };
            let vat = round_eur(full_vat * deductibility.factor());
            match direction {
                VatDirection::Sales => {
                    sales += vat;
                    // Ordinary domestic tagged sales own both the aggregate and
                    // their rate-slot tax. Reuse the same signed, rounded cents;
                    // purchase and other treatment routing stays unchanged.
                    if matches!(scheme, VatScheme::Normal)
                        && GENERIC_DOMESTIC_BANDS.contains(&rate_class)
                    {
                        domestic_sales_fields
                            .add_computed(
                                vat_rate_class_field(rate_class)?,
                                Money::new(eur_2dp(vat), eur.clone()),
                            )
                            .map_err(|error| DomainError::VatStructure(error.to_string()))?;
                    }
                }
                VatDirection::Purchase => {
                    purchase += vat;
                    has_tagged_purchase = true;
                }
            }
        }
        // A mixed-direction tagged voucher may be legacy self-assessed purchase
        // VAT. Its tags do not identify the acquisition scope, so keep its prior
        // field behavior rather than guessing that its output is domestic sales.
        // Aggregates remain unchanged; provenance-owned VAT-A purchases are
        // independently slotted by the existing account-sum projection.
        if !has_tagged_purchase {
            fields
                .merge_from(&domestic_sales_fields)
                .map_err(|error| DomainError::VatStructure(error.to_string()))?;
        }
    }

    Ok(DerivedVatFigures {
        period: period.clone(),
        sales_vat: Money::new(eur_2dp(sales), eur.clone()),
        purchase_vat: Money::new(eur_2dp(purchase), eur),
        fields,
    })
}

/// The margin-scheme split of one resale (**AVL 79 a–k §**): the VAT-inclusive
/// profit margin, the taxable base (the margin *without* the tax portion, AVL 79 k §)
/// and the output VAT extracted from within the margin. Produced by
/// [`compute_margin_vat`] so the agent can book the split correctly before sealing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MarginVat {
    /// The category the margin code named (echoed for reporting/audit).
    pub category: MarginCategory,
    /// `selling_price − purchase_price`, VAT-inclusive. Signed: **negative on a loss
    /// item**, where no VAT is due (a per-item negative margin cannot offset another
    /// item under the per-item method).
    pub margin: Money,
    /// The taxable base — the margin without the tax portion (`margin − vat`, AVL
    /// 79 k §). `0.00` on a non-positive margin.
    pub taxable_base: Money,
    /// The output VAT extracted from within the margin (`round(margin × rate /
    /// (100 + rate))`). `0.00` on a non-positive margin.
    pub vat: Money,
}

/// Compute the margin-scheme VAT split for a single second-hand resale under the
/// **per-item** method (**AVL 79 a–k §**). `margin_code` must be a `sales-margin-*`
/// code (its category is echoed and its standard rate resolves from `document_date`);
/// `selling_price` and `purchase_price` are the item's VAT-inclusive prices.
///
/// The margin (`selling − purchase`) is VAT-inclusive, so the output VAT is extracted
/// from *within* it: `vat = round(margin × rate / (100 + rate))` and the taxable base
/// is `margin − vat` (AVL 79 k §). **A non-positive margin yields no VAT** (base and
/// VAT are `0.00`): under the per-item method a loss on one item cannot reduce the VAT
/// on another — the negative margin is simply not creditable. The signed `margin` is
/// still returned so the caller can see the loss.
///
/// The domain "never guesses": this computes the split from the prices the agent
/// supplies; whether the two prices are the correct margin-scheme figures, and the
/// per-**period** simplified margin method (AVL 79 j §, which nets a whole period's
/// margin purchases and sales and carries a negative period margin forward), are not
/// decided here — the latter needs cross-voucher period state and is tracked as the
/// `vat-margin-period-method` follow-up.
pub fn compute_margin_vat(
    margin_code: &str,
    selling_price: &Money,
    purchase_price: &Money,
    document_date: Date,
) -> Result<MarginVat, DomainError> {
    let eur = require_eur(&[selling_price, purchase_price])?;
    let (_direction, rate_class, _deductibility, scheme) = parse_vat_code(margin_code)?;
    let VatScheme::Margin(category) = scheme else {
        return Err(DomainError::VatCode(format!(
            "compute_margin_vat needs a sales-margin-<category> code (AVL 79 a–k §), \
             got {margin_code:?}"
        )));
    };
    if selling_price.amount() < Decimal::ZERO || purchase_price.amount() < Decimal::ZERO {
        return Err(DomainError::VatCurrency(
            "margin-scheme selling_price and purchase_price must be non-negative".into(),
        ));
    }
    let pct = resolve_vat_rate(rate_class, document_date)?;
    let margin = selling_price.amount() - purchase_price.amount();
    // Per-item rule (AVL 79 k §): a non-positive margin is not creditable, so it
    // yields no VAT and a zero base rather than a negative (refund-like) figure.
    let (base, vat) = if margin <= Decimal::ZERO {
        (Decimal::ZERO, Decimal::ZERO)
    } else {
        let vat = round_eur(margin * pct / (Decimal::from(100) + pct));
        (margin - vat, vat)
    };
    Ok(MarginVat {
        category,
        margin: Money::new(eur_2dp(margin), eur.clone()),
        taxable_base: Money::new(eur_2dp(base), eur.clone()),
        vat: Money::new(eur_2dp(vat), eur),
    })
}

/// Fold the double-entry `Side` into a signed contribution for `direction`: the
/// accrual side (sales→Credit, purchase→Debit) is positive, its reversal negative.
/// Shared by the euro aggregator ([`derive_vat_figures`]) and the record-time
/// base-level structural check ([`check_voucher_vat_structure`]).
pub(super) fn signed_by_side(direction: VatDirection, side: Side, amount: Decimal) -> Decimal {
    match (direction, side) {
        (VatDirection::Sales, Side::Credit) | (VatDirection::Purchase, Side::Debit) => amount,
        (VatDirection::Sales, Side::Debit) | (VatDirection::Purchase, Side::Credit) => -amount,
    }
}
