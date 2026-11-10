use super::*;

/// Record-time structural guard for the F4 same-direction reverse-charge hole,
/// run by [`crate::build_draft`] **before** a voucher is sealed. A self-assessed
/// EU reverse-charge acquisition books output VAT (a `sales-` line on Credit) and
/// the equal deductible input VAT (a `purchase-` line on Debit); if both lines are
/// mistakenly tagged the *same* direction, the accrual (+) and reversal (−) bases
/// cancel *within* that direction and the self-assessed VAT would later aggregate
/// to a plausible `0.00` — worse than a direction swap, which the per-direction
/// file-vat cross-check already catches via negative figures.
///
/// The check is **rate-free** (it works on the VAT base, i.e. the posting amount):
/// a same-base debit/credit cancellation is independent of the rate, and keeping
/// `build_draft` rate-agnostic matches its existing design. Per direction, if the
/// VAT-tagged bases have `gross > 0` yet `net == 0`, the draft is refused with
/// `VAT_STRUCTURE`.
///
/// **Scope (deliberately narrow, honestly stated).** Exact base cancellation
/// within one direction is a *necessary* condition for this malformation but not a
/// *sufficient* proof of it: without scheme/`VatKind` metadata the domain cannot
/// tell a mistagged reverse charge from a legitimate full same-voucher reversal
/// (the codebase convention books corrections as separate vouchers, so that shape
/// is anomalous — but not provably wrong). Refusing at record time makes a false
/// positive cheap (re-enter the draft) rather than a sealed-period brick. It does
/// not catch cancellation masked by an unrelated same-direction line, an unequal-
/// base mistag, or a pair split across vouchers — those *generally* leave a non-zero
/// figure the file-vat cross-check surfaces, so they are not the silent-zero hole.
/// (Partial deductibility weakens that "generally": since the derived VAT of a
/// `-deduct<NN>` line is fraction-scaled while this guard weighs the raw base, an
/// unequal-base pair can now coincidentally cancel in derived VAT while the base
/// nets non-zero — an incidental case, distinct from the same-direction hole this
/// guards, and covered by the same follow-up.) The targeted same-direction case
/// (equal base, opposite sides) still cancels at the base level regardless of any
/// fraction, so it is always caught. The complete structural fix (scheme metadata +
/// group pairing) is spun off as `vat-reverse-charge-scheme-model` (needs the model
/// layer).
pub fn check_voucher_vat_structure(postings: &[Posting]) -> Result<(), DomainError> {
    let mut sales = DirectionAccum::default();
    let mut purchase = DirectionAccum::default();
    for posting in postings {
        let Some(code) = &posting.vat_code else {
            continue;
        };
        let (direction, _rate_class, _deductibility, _scheme) = parse_vat_code(code.as_str())?;
        let signed = signed_by_side(direction, posting.side, posting.amount.amount());
        match direction {
            VatDirection::Sales => sales.add(signed),
            VatDirection::Purchase => purchase.add(signed),
        }
    }
    check_direction_cancellation(VatDirection::Sales, &sales)?;
    check_direction_cancellation(VatDirection::Purchase, &purchase)?;
    Ok(())
}

/// Refuse a direction whose VAT-tagged bases cancel to zero (the F4 signature).
/// See [`check_voucher_vat_structure`].
pub(super) fn check_direction_cancellation(
    direction: VatDirection,
    accum: &DirectionAccum,
) -> Result<(), DomainError> {
    if accum.cancels_to_zero() {
        return Err(DomainError::VatStructure(format!(
            "this voucher's {}- VAT-tagged postings cancel to zero within one \
             direction (gross base {} nets to 0.00). A same-direction reverse-charge \
             booking would silently drop the self-assessed VAT; if this is an EU \
             reverse-charge acquisition, tag the output line sales- and the equal \
             deductible line purchase-. If it is a genuine same-voucher reversal, \
             book the correction as a separate voucher.",
            direction.label(),
            eur_2dp(accum.gross),
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AVL 72 b § — intra-Community (EU) sale buyer-VAT-ID enforcement
// ---------------------------------------------------------------------------
//
// A zero-rated intra-Community supply of goods/services is conditioned on the
// buyer being VAT-registered in *another* EU member state and the seller holding
// a valid buyer VAT-ID (AVL 72 b §). The domain models this as two zero-rated
// `sales-` rate classes (`eu-goods`, `eu-services`; see `data/vat-rates.json`).
// Without a buyer-VAT-ID gate a domestic sale mis-tagged `sales-eu-goods` silently
// zeroes its output VAT and under-declares the domestic-sales field — exactly the
// v1.x correctness gap `build_draft` closes below at record time.
//
// Two follow-ups this slice deliberately does NOT close (single-crate lane), each
// spun off as its own issue:
//   * the buyer VAT-ID is enforced at record time but not *persisted* on the sealed
//     voucher — the yhteenvetoilmoitus (EC recap statement) needs it stored, which
//     needs the model layer (`eu-sale-buyer-vat-id-persistence`);
//   * `DerivedVatFigures` aggregates only VAT euros, so the taxable *bases* of the
//     intra-Community fields (ALV-ilmoitus 309/310) are not yet derived — a filing-
//     pipeline concern (`eu-sale-reporting-fields`). A 0 % rate class computes the
//     right VAT (0) here; the base breakdown is the open part.

/// The intra-Community treatment a `sales-eu-*` code denotes. Goods and services
/// share the buyer-VAT-ID requirement but differ on which buyer registrations are
/// eligible (Northern Ireland `XI` numbers cover goods only — see
/// [`check_eu_sale_buyer_vat_id`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EuSaleKind {
    Goods,
    Services,
}

/// Classify a VAT code as an intra-Community (EU) *sale* and its goods/services
/// treatment. Only `sales-` codes qualify; a malformed or non-sales code is simply
/// "not an EU sale" here (its own error surfaces through `parse_vat_code`
/// elsewhere), so this never fails. The `(rate_class, kind)` pairs are the single
/// source of truth the rate table (`data/vat-rates.json`) mirrors.
pub(super) fn eu_sale_kind(code: &str) -> Option<EuSaleKind> {
    match parse_vat_code(code) {
        Ok((VatDirection::Sales, "eu-goods", _, _)) => Some(EuSaleKind::Goods),
        Ok((VatDirection::Sales, "eu-services", _, _)) => Some(EuSaleKind::Services),
        _ => None,
    }
}

/// The 2-letter prefixes an EU VAT number may carry: the 27 member-state ISO
/// 3166-1 alpha-2 codes with the VAT-specific substitution `EL` for Greece (VIES
/// uses `EL`, never `GR`). Northern Ireland's `XI` is handled separately because
/// it is valid for goods only, not services, so it is deliberately *not* in this
/// member-state list. A change here is data, not logic.
pub(super) const EU_VAT_COUNTRY_CODES: &[&str] = &[
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "EL", "ES", "FI", "FR", "HR", "HU", "IE", "IT",
    "LT", "LU", "LV", "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK",
];

/// Canonicalise a raw VAT-ID once: drop *all* whitespace and upper-case (ASCII).
/// Both the format check and the FI/XI country tests derive their verdict from this
/// single normalised form, so an interior space (`"F I01234567"`) cannot pass one
/// check and evade another. Non-ASCII characters survive unchanged and are rejected
/// downstream — never fed to a byte slice (which would panic on a char boundary).
pub(super) fn normalize_vat_id(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Structural format-check of an EU buyer VAT-ID; returns the **normalised**
/// (whitespace-stripped, upper-cased) form on success so callers reuse one canonical
/// value rather than re-deriving it. A recognised 2-letter prefix (a member state,
/// or `XI` for Northern Ireland) followed by 2..=12 ASCII-alphanumeric characters
/// (the union of the national formats). This validates *shape*, not a per-country
/// checksum or a live VIES registration — the domain "never guesses", so a genuine
/// registration check belongs to an online adapter, not here.
pub fn validate_eu_buyer_vat_id(vat_id: &str) -> Result<String, DomainError> {
    let normalized = normalize_vat_id(vat_id);
    // Slice by bytes below, so reject any non-ASCII first (a VAT-ID is ASCII by
    // construction). This also stops `split_at` from panicking on a char boundary.
    if !normalized.is_ascii() {
        return Err(DomainError::VatBuyerId(format!(
            "buyer VAT-ID {vat_id:?} contains non-ASCII characters — an EU VAT number \
             is a 2-letter prefix plus an alphanumeric national part"
        )));
    }
    if normalized.len() < 4 {
        return Err(DomainError::VatBuyerId(format!(
            "buyer VAT-ID {vat_id:?} is too short — expected a 2-letter EU country \
             prefix followed by the national number"
        )));
    }
    let (prefix, body) = normalized.split_at(2);
    if prefix != "XI" && !EU_VAT_COUNTRY_CODES.contains(&prefix) {
        return Err(DomainError::VatBuyerId(format!(
            "buyer VAT-ID {vat_id:?} has an unrecognised country prefix {prefix:?} — \
             an intra-Community sale needs an EU member-state VAT number"
        )));
    }
    if !(2..=12).contains(&body.len()) || !body.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(DomainError::VatBuyerId(format!(
            "buyer VAT-ID {vat_id:?} has a malformed national part {body:?} — expected \
             2 to 12 alphanumeric characters after the {prefix} prefix"
        )));
    }
    Ok(normalized)
}

/// Record-time AVL 72 b § gate, run by [`crate::build_draft`] before a voucher is
/// sealed. If any posting carries an EU-sale VAT code (`sales-eu-goods` /
/// `sales-eu-services`), a buyer VAT-ID must be supplied *and* pass
/// [`validate_eu_buyer_vat_id`], *and* name a buyer registered in **another** EU
/// member state:
///
/// - a Finnish (`FI`) registration is refused — a domestic buyer is not an intra-
///   Community supply, and booking one as such is the very misposting this guards;
/// - a Northern Ireland (`XI`) registration is accepted for **goods** only, not
///   services (the Windsor-framework EU VAT treatment covers goods).
///
/// Returns the normalized ID for persistence after validation, or `None` when no
/// ID was supplied.
///
/// A *stray* buyer VAT-ID on a voucher with no EU-sale code is still format-checked
/// (a present-but-malformed ID signals a mistag — a `sales-eu-*` code the operator
/// forgot to set), but the "another member state" rule only applies to actual EU
/// sales.
pub fn check_eu_sale_buyer_vat_id(
    postings: &[Posting],
    buyer_vat_id: Option<&str>,
) -> Result<Option<String>, DomainError> {
    let mut has_goods = false;
    let mut has_services = false;
    for posting in postings {
        match posting
            .vat_code
            .as_ref()
            .and_then(|c| eu_sale_kind(c.as_str()))
        {
            Some(EuSaleKind::Goods) => has_goods = true,
            Some(EuSaleKind::Services) => has_services = true,
            None => {}
        }
    }
    let supplied = buyer_vat_id.map(str::trim).filter(|s| !s.is_empty());

    if !has_goods && !has_services {
        // No EU sale: a stray ID is only format-checked, never subjected to the
        // member-state rule (it may legitimately be, e.g., an FI domestic buyer).
        return supplied.map(validate_eu_buyer_vat_id).transpose();
    }

    let id = supplied.ok_or_else(|| {
        DomainError::VatBuyerId(
            "an EU-sale code (sales-eu-goods / sales-eu-services) zero-rates the \
             output VAT under AVL 72 b §, which requires a buyer VAT-ID registered \
             in another EU member state; none was supplied. If this is a domestic \
             sale, tag it sales-domestic-* so its output VAT is declared."
                .into(),
        )
    })?;
    let normalized = validate_eu_buyer_vat_id(id)?;
    if normalized.starts_with("FI") {
        return Err(DomainError::VatBuyerId(format!(
            "buyer VAT-ID {id:?} is Finnish (FI) — a domestic buyer is not an intra-\
             Community supply under AVL 72 b §; tag the sale sales-domestic-* so its \
             output VAT is declared"
        )));
    }
    if normalized.starts_with("XI") && has_services {
        return Err(DomainError::VatBuyerId(format!(
            "buyer VAT-ID {id:?} is Northern Irish (XI), which is valid for intra-\
             Community supplies of goods only, not services (sales-eu-services)"
        )));
    }
    Ok(Some(normalized))
}

/// Record-time guard against an EU *acquisition* mis-booked as a zero-rated
/// purchase. The `eu-goods` / `eu-services` rate classes exist so an EU *sale* can
/// resolve (to 0 %); the same classes on a `purchase-` code (`purchase-eu-goods`)
/// would silently resolve to a zero-rated deduction, but an intra-Community
/// acquisition must be **reverse-charged** (self-assessed output + deductible input
/// VAT, per AVL 9 §), not booked at 0 %. Refuse it at record time so the hole the
/// new rate classes open cannot be entered. Run by [`crate::build_draft`].
pub fn check_no_eu_acquisition_code(postings: &[Posting]) -> Result<(), DomainError> {
    for posting in postings {
        let Some(code) = &posting.vat_code else {
            continue;
        };
        if matches!(
            parse_vat_code(code.as_str()),
            Ok((VatDirection::Purchase, "eu-goods" | "eu-services", _, _))
        ) {
            return Err(DomainError::VatCode(format!(
                "VAT code {:?} books an EU acquisition as a zero-rated purchase; an \
                 intra-Community acquisition must be reverse-charged (self-assessed \
                 output sales- + deductible input purchase-domestic-*, AVL 9 §), not \
                 tagged purchase-eu-*",
                code.as_str()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ADR-7 (VAT-A) — explicit VAT-leg expansion before the seal
// ---------------------------------------------------------------------------
//
// ADR-7 decides that a sealed voucher physically carries every VAT posting leg:
// the deductible-input accounts (`1763`/`1798`/`1797`) and the reverse-charge output
// accounts (`2992`/`2991`/`2982`) move for real, rather than being derived on read.
// [`vat_expand`] is the pure, I/O-free Domain function that turns an intake amount +
// a VAT treatment + the document date into the complete, balanced `VoucherDraft` —
// the image that is then sealed unchanged through the one S1 seal path.
//
// Expansion is date-driven: the rate resolves from the treatment's `rate_class` +
// the `document_date` against the same bundled dated table the rest of this module
// uses (the pinned S1 policy), so the same reverse-charge purchase before and after
// the 2024-09-01 24 % → 25,5 % change expands to the *historical* rate (ADR-7 §4).
// Because the legs are frozen bytes, a later rate/policy change can never restate a
// past voucher — immutability by construction (ADR-7 §Consequences).
