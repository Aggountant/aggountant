use super::*;

/// A monetary field on Finland's VAT return, identified by Verohallinto's stable
/// field number rather than its rate-dependent display label.
///
/// This is the complete monetary field set for accounting periods beginning on
/// or after 2025-01-01, following Verohallinto's *Arvonlisäveroilmoituksen
/// täyttöohje*. The pre-2025 lower-limit-relief fields 319–321 are deliberately
/// outside this edition; field 304 is an intentional numbering gap.
///
/// The three domestic-rate slots (301–303) deliberately do not encode percentages:
/// rates change over time while the return slots remain the same. Fields 309–318
/// cover zero-rated turnover, EU trade, imports, and domestic reverse charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u16)]
pub enum VatReturnField {
    /// Tax on domestic sales, standard-rate slot.
    #[serde(rename = "301")]
    Field301 = 301,
    /// Tax on domestic sales, first reduced-rate slot.
    #[serde(rename = "302")]
    Field302 = 302,
    /// Tax on domestic sales, second reduced-rate slot.
    #[serde(rename = "303")]
    Field303 = 303,
    /// Tax on intra-Community acquisitions of goods.
    #[serde(rename = "305")]
    Field305 = 305,
    /// Tax on intra-Community purchases of services.
    #[serde(rename = "306")]
    Field306 = 306,
    /// Deductible tax for the period.
    #[serde(rename = "307")]
    Field307 = 307,
    /// Tax payable (positive) or refundable (negative).
    #[serde(rename = "308")]
    Field308 = 308,
    /// Zero-rated turnover other than the EU supplies reported separately in
    /// fields 310 and 311 (including non-EU exports).
    #[serde(rename = "309")]
    Field309 = 309,
    /// Intra-Community supplies of goods.
    #[serde(rename = "310")]
    Field310 = 310,
    /// Intra-Community supplies of services.
    #[serde(rename = "311")]
    Field311 = 311,
    /// Intra-Community acquisitions of goods, taxable amount.
    #[serde(rename = "312")]
    Field312 = 312,
    /// Intra-Community purchases of services, taxable amount.
    #[serde(rename = "313")]
    Field313 = 313,
    /// Imports of goods from outside the EU, taxable amount.
    #[serde(rename = "314")]
    Field314 = 314,
    /// Tax on imports of goods from outside the EU.
    #[serde(rename = "315")]
    Field315 = 315,
    /// Sales of construction services and scrap metal subject to domestic
    /// reverse charge.
    #[serde(rename = "316")]
    Field316 = 316,
    /// Purchases of construction services and scrap metal subject to domestic
    /// reverse charge, taxable amount.
    #[serde(rename = "317")]
    Field317 = 317,
    /// Tax on purchases reported in field 317.
    #[serde(rename = "318")]
    Field318 = 318,
}

impl VatReturnField {
    pub const ALL: &'static [Self] = &[
        Self::Field301,
        Self::Field302,
        Self::Field303,
        Self::Field305,
        Self::Field306,
        Self::Field307,
        Self::Field308,
        Self::Field309,
        Self::Field310,
        Self::Field311,
        Self::Field312,
        Self::Field313,
        Self::Field314,
        Self::Field315,
        Self::Field316,
        Self::Field317,
        Self::Field318,
    ];

    /// Verohallinto's numeric identifier for the field.
    pub const fn number(self) -> u16 {
        self as u16
    }
}

/// Map the closed generic Finnish VAT rate classes to the stable domestic-tax slots.
/// The slot follows the rate class, not an account number or the historical percentage
/// (the standard slot remained 301 when 24% changed to 25.5%).
pub(super) fn vat_rate_class_field(rate_class: &str) -> Result<VatReturnField, DomainError> {
    match rate_class {
        "domestic-standard" => Ok(VatReturnField::Field301),
        "domestic-reduced-1" => Ok(VatReturnField::Field302),
        "domestic-reduced-2" => Ok(VatReturnField::Field303),
        other => Err(DomainError::VatStructure(format!(
            "VAT rate class {other:?} has no Finnish return rate slot (expected domestic-standard, domestic-reduced-1, or domestic-reduced-2)"
        ))),
    }
}

/// Sparse, number-keyed VAT-return fields. Missing means "not derived", which is
/// distinct from a reported zero. A map avoids baking the form into another flat
/// struct and lets later derivation slices populate EU and reverse-charge fields
/// independently without changing this API.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VatReturnFieldsError {
    #[error("VAT return field {field} must be EUR, got {currency}")]
    NonEur { field: u16, currency: String },
    #[error("VAT return field {field} was populated more than once")]
    Duplicate { field: u16 },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VatReturnFields(std::collections::BTreeMap<VatReturnField, Money>);

impl VatReturnFields {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one independently derived field. Duplicate contributions fail loudly
    /// instead of allowing one derivation slice to overwrite another.
    pub fn try_insert(
        &mut self,
        field: VatReturnField,
        amount: Money,
    ) -> Result<(), VatReturnFieldsError> {
        if amount.currency().as_str() != "EUR" {
            return Err(VatReturnFieldsError::NonEur {
                field: field.number(),
                currency: amount.currency().as_str().to_owned(),
            });
        }
        if self.0.contains_key(&field) {
            return Err(VatReturnFieldsError::Duplicate {
                field: field.number(),
            });
        }
        self.0.insert(field, amount);
        Ok(())
    }

    pub fn get(&self, field: VatReturnField) -> Option<&Money> {
        self.0.get(&field)
    }

    pub fn iter(&self) -> impl Iterator<Item = (VatReturnField, &Money)> {
        self.0.iter().map(|(field, amount)| (*field, amount))
    }

    fn set_computed(&mut self, field: VatReturnField, amount: Money) {
        self.0.insert(field, amount);
    }

    pub(super) fn add_computed(
        &mut self,
        field: VatReturnField,
        amount: Money,
    ) -> Result<(), VatReturnFieldsError> {
        if amount.currency().as_str() != "EUR" {
            return Err(VatReturnFieldsError::NonEur {
                field: field.number(),
                currency: amount.currency().as_str().to_owned(),
            });
        }
        match self.0.get_mut(&field) {
            Some(current) => {
                *current = Money::new(
                    eur_2dp(current.amount() + amount.amount()),
                    current.currency().clone(),
                );
            }
            None => {
                self.0.insert(
                    field,
                    Money::new(eur_2dp(amount.amount()), amount.currency().clone()),
                );
            }
        }
        Ok(())
    }

    pub(super) fn merge_from(
        &mut self,
        other: &VatReturnFields,
    ) -> Result<(), VatReturnFieldsError> {
        for (field, amount) in other.iter() {
            self.add_computed(field, amount.clone())?;
        }
        Ok(())
    }
}

/// Aggregated period VAT figures the agent extracts from the sealed ledger, the
/// input to [`compute_vat_filing`]. Both amounts are non-negative totals; the
/// *net* position is derived, never guessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VatFilingInput {
    pub period: PeriodId,
    /// Output VAT collected on sales (myynnin ALV).
    pub sales_vat: Money,
    /// Deductible input VAT on purchases (vähennettävä ALV).
    pub purchase_vat: Money,
}

/// The computed VAT filing for a period (the `CorePayload::Filing` figures). The
/// `net_vat` is signed: **positive = payable to Vero**, negative = refund/credit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatFiling {
    pub period: PeriodId,
    pub sales_vat: Money,
    pub purchase_vat: Money,
    /// Number-keyed return fields derived by this filing stage. The legacy
    /// two-bucket input can authoritatively populate only 307 and 308; later
    /// category derivation fills the remaining field slots without changing the
    /// return shape.
    pub fields: VatReturnFields,
    /// `sales_vat - purchase_vat`. Positive = payable, negative = refund.
    pub net_vat: Money,
}

/// A snapshot of the tax account (verotili / OmaVero) at a point in time,
/// produced by the `vero-verotili` pull adapter (F12). `balance` is signed:
/// **positive = credit already on the account** (reduces the bank payment),
/// negative = an existing debt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxAccountSnapshot {
    pub balance: Money,
    /// Free-form `as_of` marker echoed from the authority (e.g. an RFC3339 time).
    pub as_of: String,
}

/// The reconciliation of a filing against the tax-account snapshot: how much must
/// actually leave the bank (`bank_payment_required`) once any credit already
/// sitting on the tax account is applied. This is the figure the payment order's
/// amount must equal (the net-invariant, `PAYMENT_EXCEEDS_RECONCILED_AMOUNT`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reconciliation {
    pub period: PeriodId,
    pub net_vat: Money,
    pub account_balance: Money,
    /// `max(0, net_vat - max(0, account_credit))` — never negative (a refund or a
    /// fully-covered filing requires **no** bank payment).
    pub bank_payment_required: Money,
}

/// Force a decimal to exactly 2 fractional digits (the EUR minor unit), so
/// `amount_str()` is always canonical — `"0.00"`, never `"0"`. `rescale` sets the
/// scale exactly (unlike `round_dp`, which leaves an already-integer zero at scale 0).
pub(super) fn eur_2dp(mut d: Decimal) -> Decimal {
    d.rescale(2);
    d
}

/// Require the currency to match `EUR` for the S3 slice (a single-currency VAT
/// filing), and return the shared currency. Mixed/foreign currency is rejected
/// loudly rather than silently coerced.
pub(super) fn require_eur(amounts: &[&Money]) -> Result<Currency, DomainError> {
    let eur = Currency::new("EUR").expect("EUR is a valid currency code");
    for m in amounts {
        if m.currency() != &eur {
            return Err(DomainError::VatCurrency(format!(
                "VAT filing is EUR-only in this slice, got {}",
                m.currency()
            )));
        }
    }
    Ok(eur)
}

/// Compute a period's VAT filing from its aggregated figures. Deterministic and
/// total over valid input: `net_vat = sales_vat - purchase_vat`.
pub fn compute_vat_filing(input: &VatFilingInput) -> Result<VatFiling, DomainError> {
    compute_vat_filing_with_fields(input, VatReturnFields::new())
}

/// Build a filing from ledger-derived figures while preserving category fields that
/// were read from sealed VAT-A treatment provenance.
pub fn compute_derived_vat_filing(figures: &DerivedVatFigures) -> Result<VatFiling, DomainError> {
    compute_vat_filing_with_fields(
        &VatFilingInput {
            period: figures.period.clone(),
            sales_vat: figures.sales_vat.clone(),
            purchase_vat: figures.purchase_vat.clone(),
        },
        figures.fields.clone(),
    )
}

fn compute_vat_filing_with_fields(
    input: &VatFilingInput,
    mut fields: VatReturnFields,
) -> Result<VatFiling, DomainError> {
    let eur = require_eur(&[&input.sales_vat, &input.purchase_vat])?;
    if input.sales_vat.amount() < Decimal::ZERO || input.purchase_vat.amount() < Decimal::ZERO {
        return Err(DomainError::VatCurrency(
            "sales_vat and purchase_vat must be non-negative totals".into(),
        ));
    }
    // Rescale to the EUR minor unit (2 dp) so `amount_str()` is always canonical
    // ("0.00", not "0") — the payment amount and its `sent_hash` depend on it.
    let net = eur_2dp(input.sales_vat.amount() - input.purchase_vat.amount());
    let net_vat = Money::new(net, eur);
    fields.set_computed(VatReturnField::Field307, input.purchase_vat.clone());
    fields.set_computed(VatReturnField::Field308, net_vat.clone());
    Ok(VatFiling {
        period: input.period.clone(),
        sales_vat: input.sales_vat.clone(),
        purchase_vat: input.purchase_vat.clone(),
        fields,
        net_vat,
    })
}

/// Reconcile a filing against the tax-account snapshot, yielding the bank payment
/// actually required. A credit already on the account offsets the payable; a
/// refund (or a fully-covered payable) requires no bank payment (`0.00`).
pub fn reconcile_verotili(
    filing: &VatFiling,
    snapshot: &TaxAccountSnapshot,
) -> Result<Reconciliation, DomainError> {
    let eur = require_eur(&[&filing.net_vat, &snapshot.balance])?;
    let net = filing.net_vat.amount();
    let credit = snapshot.balance.amount().max(Decimal::ZERO);
    let required = eur_2dp((net - credit).max(Decimal::ZERO));
    Ok(Reconciliation {
        period: filing.period.clone(),
        net_vat: filing.net_vat.clone(),
        account_balance: snapshot.balance.clone(),
        bank_payment_required: Money::new(required, eur),
    })
}

#[cfg(test)]
mod return_field_tests {
    use super::*;

    fn eur(amount: &str) -> Money {
        Money::parse(amount, Currency::new("EUR").unwrap()).unwrap()
    }

    #[test]
    fn current_edition_has_the_exact_numbered_field_set() {
        let numbers: Vec<_> = VatReturnField::ALL
            .iter()
            .map(|field| field.number())
            .collect();
        assert_eq!(
            numbers,
            [
                301, 302, 303, 305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317,
                318,
            ]
        );
    }

    #[test]
    fn every_field_serializes_and_round_trips_by_its_number() {
        for &field in VatReturnField::ALL {
            let mut fields = VatReturnFields::new();
            fields.try_insert(field, eur("1.00")).unwrap();
            let json = serde_json::to_value(&fields).unwrap();
            assert!(json.get(field.number().to_string()).is_some());
            assert_eq!(
                serde_json::from_value::<VatReturnFields>(json).unwrap(),
                fields
            );
        }
    }

    #[test]
    fn independently_derived_fields_cannot_overwrite_each_other() {
        let mut fields = VatReturnFields::new();
        fields
            .try_insert(VatReturnField::Field310, eur("300.00"))
            .unwrap();
        let err = fields
            .try_insert(VatReturnField::Field310, eur("400.00"))
            .unwrap_err();
        assert_eq!(err, VatReturnFieldsError::Duplicate { field: 310 });
    }

    #[test]
    fn legacy_filing_populates_only_authoritative_return_fields() {
        let filing = compute_vat_filing(&VatFilingInput {
            period: q1(),
            sales_vat: eur("100.00"),
            purchase_vat: eur("25.00"),
        })
        .unwrap();
        assert_eq!(
            filing
                .fields
                .get(VatReturnField::Field307)
                .unwrap()
                .amount_str(),
            "25.00"
        );
        assert_eq!(
            filing
                .fields
                .get(VatReturnField::Field308)
                .unwrap()
                .amount_str(),
            "75.00"
        );
        assert!(filing.fields.get(VatReturnField::Field301).is_none());
    }
}

// ---------------------------------------------------------------------------
// F4 — VAT-code resolution + in-Run derivation from the sealed ledger
// ---------------------------------------------------------------------------
//
// `compute_vat_filing` above trusts the caller's aggregated figures; the
// accounting-integrity follow-up (issue `vat-figures-from-sealed-ledger`) is to
// **derive** those aggregates from the sealed vouchers so the `file-vat` Run can
// cross-check its CLI input against what the ledger actually holds.
//
// The model (`candidate.md §3`): a `VatCode` is **semantic** (`sales-domestic-
// standard`, `purchase-domestic-standard`), never a baked-in percentage. The
// *direction* is the code's first segment (`sales-`/`purchase-`); the *rate* is
// resolved from the remaining `rate_class` + the voucher's `document_date` against
// a dated table (the same reviewable-data pattern as the chart / payroll rates —
// the general rate changed 24 % → 25,5 % on 2024-09-01, so an embedded rate would
// silently misfile historical vouchers). A VAT-bearing posting's `amount` is the
// VAT **base**; the euro VAT is `round(base × rate)` (half away from zero, per line).
