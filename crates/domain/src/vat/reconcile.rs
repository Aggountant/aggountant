use super::*;

/// How one posting on a VAT-role account relates to the filing projection.
///
/// The return is computed from `vat_code` tags and from hash-bound generated VAT
/// legs, never from account numbers. A posting on a VAT-role account that neither
/// path owns is therefore invisible to the return; this classification only
/// detects that disagreement, it never decides a treatment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VatAccountPostingClass {
    /// The voucher carries a `vat_code` whose `base × rate` explains this leg, so the
    /// tagged path owns its VAT.
    Tagged,
    /// A generated VAT leg of a sealed VAT expansion; the account-sum path owns it.
    Generated,
    /// Untagged and unowned, on the side that reduces the VAT balance (output VAT
    /// debited, input VAT credited) in a voucher that touches no income or expense
    /// account: a VAT payment, refund or clearing between VAT accounts. It does not
    /// belong in a return.
    Settlement,
    /// Untagged and unowned VAT the return leaves out: VAT accrued on a VAT account
    /// (output VAT credited, input VAT debited), a reduction booked together with
    /// income or expense (an untagged credit note or return), a leg on a coded
    /// voucher that its codes do not explain, or a posting whose account has no
    /// known VAT direction.
    Untagged,
}

/// One posting on a VAT-role account that the filing projection does not own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatAccountPosting {
    pub voucher: u64,
    pub posting_index: usize,
    pub account: String,
    /// The VAT balance the account carries (`output` or `input`); `None` when the
    /// chart says neither.
    pub direction: Option<GeneratedVatRole>,
    pub side: Side,
    pub amount: Money,
}

/// Period debits and credits on one VAT-role account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatAccountMovement {
    pub account: String,
    pub direction: Option<GeneratedVatRole>,
    pub debit: Money,
    pub credit: Money,
}

/// One direction's comparison. Amounts are signed so that accrued VAT is positive
/// (output: credit − debit, input: debit − credit). `difference` is
/// `ledger_movement − settlements − derived`; it is the untagged VAT plus whatever
/// tagged vouchers' VAT legs disagree with their `base × rate` (per-line rounding,
/// a leg on a non-VAT account, or a leg that does not match its tag).
/// `rounding_tolerance` is one cent per tagged posting of this direction, the most
/// per-line `round(base × rate)` can differ from an invoice's VAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatDirectionReconciliation {
    pub ledger_movement: Money,
    pub settlements: Money,
    pub derived: Money,
    pub difference: Money,
    pub untagged: Money,
    pub rounding_tolerance: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VatReconciliationStatus {
    /// Every untagged VAT-account posting is matched by the return: none exist, or
    /// a VAT-coded correction in the same period brings the direction within
    /// rounding tolerance.
    Reconciled,
    /// Untagged VAT-account postings leave a direction's ledger VAT and its return
    /// figure apart by more than rounding, or their direction or currency is unknown.
    Unreconciled,
}

/// Period-level comparison of the VAT-role accounts with the derived VAT figures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatAccountReconciliation {
    pub period: PeriodId,
    pub status: VatReconciliationStatus,
    pub output: VatDirectionReconciliation,
    pub input: VatDirectionReconciliation,
    pub accounts: Vec<VatAccountMovement>,
    /// Postings neither the tagged nor the generated path owns. They make the
    /// period `unreconciled` unless the direction still reconciles.
    pub untagged_postings: Vec<VatAccountPosting>,
    /// VAT payments, refunds and clearings: shown for audit, never filed.
    pub settlement_postings: Vec<VatAccountPosting>,
}

impl VatAccountReconciliation {
    /// The sorted, distinct voucher numbers of the untagged postings.
    pub fn untagged_vouchers(&self) -> Vec<u64> {
        let mut vouchers: Vec<u64> = self.untagged_postings.iter().map(|p| p.voucher).collect();
        vouchers.sort_unstable();
        vouchers.dedup();
        vouchers
    }
}

/// Classify every posting on a VAT-role account of one voucher (a draft is checked
/// as a transient body). Postings on ordinary accounts are skipped. `kind_of` gives
/// the chart's account kind; it supplies the VAT direction of a role account whose
/// chart snapshot names no generated role (liability → output, asset → input) and
/// tells whether the voucher touches income or expense. An unknown account counts as
/// income or expense, so a gap in the chart never turns a posting into a settlement.
///
/// A voucher with a `vat_code` owns its VAT legs only as far as its codes explain
/// them: per direction, legs that differ from the voucher's own `base × rate` by more
/// than a cent per coded line are `Untagged` (a line whose code was forgotten, or a
/// code of the other direction next to an uncoded leg). Codes with no VAT leg at all
/// stay `Tagged`; the return then follows the codes.
pub fn classify_vat_account_postings(
    voucher: &VoucherBody,
    roles: &ChartVatRoles,
    kind_of: &dyn Fn(&str) -> Option<AccountKind>,
) -> Result<Vec<(usize, Option<GeneratedVatRole>, VatAccountPostingClass)>, DomainError> {
    let postings = voucher.postings();
    let touches_result = postings.iter().any(|p| {
        !matches!(
            kind_of(p.account.as_str()),
            Some(AccountKind::Asset | AccountKind::Liability | AccountKind::Equity)
        )
    });
    let generated: Vec<usize> = voucher
        .vat_expansion()
        .map(|e| {
            e.generated_vat_legs()
                .iter()
                .map(|leg| leg.posting_index)
                .collect()
        })
        .unwrap_or_default();
    let direction_of = |account: &str| {
        roles
            .generated_role_of(account)
            .or_else(|| match kind_of(account) {
                Some(AccountKind::Liability) => Some(GeneratedVatRole::Output),
                Some(AccountKind::Asset) => Some(GeneratedVatRole::Input),
                _ => None,
            })
    };
    let unexplained = tagged_legs_unexplained(voucher, roles, &direction_of)?;
    let mut classified = Vec::new();
    for (index, posting) in postings.iter().enumerate() {
        let account = posting.account.as_str();
        if roles.role_of(account).is_none() {
            continue;
        }
        let direction = direction_of(account);
        let class = if let Some(unexplained) = unexplained {
            match direction {
                Some(GeneratedVatRole::Output) if unexplained[0] => {
                    VatAccountPostingClass::Untagged
                }
                Some(GeneratedVatRole::Input) if unexplained[1] => VatAccountPostingClass::Untagged,
                _ => VatAccountPostingClass::Tagged,
            }
        } else if generated.contains(&index) {
            VatAccountPostingClass::Generated
        } else {
            let reduces = matches!(
                (direction, posting.side),
                (Some(GeneratedVatRole::Output), Side::Debit)
                    | (Some(GeneratedVatRole::Input), Side::Credit)
            );
            if reduces && !touches_result {
                VatAccountPostingClass::Settlement
            } else {
                VatAccountPostingClass::Untagged
            }
        };
        classified.push((index, direction, class));
    }
    Ok(classified)
}

/// For a voucher with any `vat_code`: per direction `[output, input]`, whether its
/// VAT legs hold VAT its own codes do not explain. `None` for an untagged voucher.
fn tagged_legs_unexplained(
    voucher: &VoucherBody,
    roles: &ChartVatRoles,
    direction_of: &dyn Fn(&str) -> Option<GeneratedVatRole>,
) -> Result<Option<[bool; 2]>, DomainError> {
    let mut lines = [0i64; 2];
    for code in voucher
        .postings()
        .iter()
        .filter_map(|p| p.vat_code.as_ref())
    {
        match parse_vat_code(code.as_str())?.0 {
            VatDirection::Sales => lines[0] += 1,
            VatDirection::Purchase => lines[1] += 1,
        }
    }
    if lines == [0, 0] {
        return Ok(None);
    }
    let mut legs = [Decimal::ZERO; 2];
    for posting in voucher.postings() {
        let account = posting.account.as_str();
        if roles.role_of(account).is_none() {
            continue;
        }
        match direction_of(account) {
            Some(GeneratedVatRole::Output) => legs[0] -= posting.signed_amount(),
            Some(GeneratedVatRole::Input) => legs[1] += posting.signed_amount(),
            None => {}
        }
    }
    let own = derive_vat_figures(
        &PeriodId::containing(voucher.posting_date()),
        std::slice::from_ref(voucher),
    )?;
    let derived = [own.sales_vat.amount(), own.purchase_vat.amount()];
    Ok(Some(std::array::from_fn(|d| {
        !legs[d].is_zero() && (legs[d] - derived[d]).abs() > Decimal::new(lines[d], 2)
    })))
}

/// Compare a period's VAT-role account movement with its derived VAT figures.
///
/// Read-only and total over sealed vouchers: it never refuses a voucher shape, it
/// only reports postings the return would leave out. Each voucher resolves its own
/// exact chart snapshot, as the filing projection does.
pub fn reconcile_vat_accounts(
    period: &PeriodId,
    vouchers: &[VoucherBody],
    derived: &DerivedVatFigures,
    role_table: &VatAccountRoleTable,
    kind_of: &dyn Fn(&str) -> Option<AccountKind>,
) -> Result<VatAccountReconciliation, DomainError> {
    use std::collections::BTreeMap;
    let eur = Currency::new("EUR").expect("EUR is a valid currency code");
    // account → (direction, debit, credit). Two chart snapshots that disagree on an
    // account's direction leave it without one.
    let mut accounts: BTreeMap<String, (Option<GeneratedVatRole>, Decimal, Decimal)> =
        BTreeMap::new();
    // [output, input] accumulators: movement, settlements, untagged.
    let mut movement = [Decimal::ZERO; 2];
    let mut settlements = [Decimal::ZERO; 2];
    let mut untagged = [Decimal::ZERO; 2];
    let mut tagged_postings = [0u32; 2];
    // An untagged posting whose direction or currency is unknown cannot be compared.
    let mut uncomparable = false;
    let mut untagged_postings = Vec::new();
    let mut settlement_postings = Vec::new();

    for voucher in vouchers {
        if !period.contains(voucher.posting_date()) {
            continue;
        }
        let roles = role_table
            .resolve(voucher.chart_version())
            .map_err(|error| {
                DomainError::VatStructure(format!("voucher {}: {error}", voucher.number()))
            })?;
        for code in voucher
            .postings()
            .iter()
            .filter_map(|p| p.vat_code.as_ref())
        {
            match parse_vat_code(code.as_str())?.0 {
                VatDirection::Sales => tagged_postings[0] += 1,
                VatDirection::Purchase => tagged_postings[1] += 1,
            }
        }
        for (index, direction, class) in classify_vat_account_postings(voucher, roles, kind_of)? {
            let posting = &voucher.postings()[index];
            let line = VatAccountPosting {
                voucher: voucher.number(),
                posting_index: index,
                account: posting.account.as_str().to_string(),
                direction,
                side: posting.side,
                amount: posting.amount.clone(),
            };
            // A foreign-currency VAT posting cannot be summed into EUR figures, and
            // the EUR-only return cannot carry it either.
            if posting.amount.currency() != &eur {
                uncomparable = true;
                untagged_postings.push(line);
                continue;
            }
            let amount = posting.amount.amount();
            let entry = accounts.entry(line.account.clone()).or_insert((
                direction,
                Decimal::ZERO,
                Decimal::ZERO,
            ));
            if entry.0 != direction {
                entry.0 = None;
            }
            match posting.side {
                Side::Debit => entry.1 += amount,
                Side::Credit => entry.2 += amount,
            }
            let slot = match direction {
                Some(GeneratedVatRole::Output) => Some(0),
                Some(GeneratedVatRole::Input) => Some(1),
                None => None,
            };
            let accrued = match direction {
                Some(GeneratedVatRole::Output) => -posting.signed_amount(),
                _ => posting.signed_amount(),
            };
            if let Some(slot) = slot {
                movement[slot] += accrued;
            }
            match class {
                VatAccountPostingClass::Tagged | VatAccountPostingClass::Generated => {}
                VatAccountPostingClass::Settlement => {
                    if let Some(slot) = slot {
                        settlements[slot] += accrued;
                    }
                    settlement_postings.push(line);
                }
                VatAccountPostingClass::Untagged => {
                    match slot {
                        Some(slot) => untagged[slot] += accrued,
                        None => uncomparable = true,
                    }
                    untagged_postings.push(line);
                }
            }
        }
    }

    let money = |d: Decimal| Money::new(eur_2dp(d), eur.clone());
    let direction = |slot: usize, derived: &Money| VatDirectionReconciliation {
        ledger_movement: money(movement[slot]),
        settlements: money(settlements[slot]),
        derived: derived.clone(),
        difference: money(movement[slot] - settlements[slot] - derived.amount()),
        untagged: money(untagged[slot]),
        rounding_tolerance: money(Decimal::new(i64::from(tagged_postings[slot]), 2)),
    };
    let output = direction(0, &derived.sales_vat);
    let input = direction(1, &derived.purchase_vat);
    // A direction with untagged postings reconciles only when its ledger VAT still
    // matches the return within rounding, e.g. after a VAT-coded correction in the
    // same period. Presence, not the untagged sum, decides: a booking and its
    // untagged reversal net to zero but still need the comparison.
    let apart = |role: GeneratedVatRole, d: &VatDirectionReconciliation| {
        untagged_postings.iter().any(|p| p.direction == Some(role))
            && d.difference.amount().abs() > d.rounding_tolerance.amount()
    };
    let unreconciled = uncomparable
        || apart(GeneratedVatRole::Output, &output)
        || apart(GeneratedVatRole::Input, &input);
    Ok(VatAccountReconciliation {
        period: period.clone(),
        status: if unreconciled {
            VatReconciliationStatus::Unreconciled
        } else {
            VatReconciliationStatus::Reconciled
        },
        output,
        input,
        accounts: accounts
            .into_iter()
            .map(|(account, (direction, debit, credit))| VatAccountMovement {
                account,
                direction,
                debit: money(debit),
                credit: money(credit),
            })
            .collect(),
        untagged_postings,
        settlement_postings,
    })
}
