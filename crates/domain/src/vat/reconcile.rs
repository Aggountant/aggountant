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
    /// debited, input VAT credited) in a voucher whose other postings are only VAT
    /// accounts, settlement-role accounts (bank and tax accounts) of the voucher's
    /// chart, and interest or penalties smaller than the VAT it reduces: a VAT
    /// payment, refund or clearing between VAT accounts. It does not belong in a
    /// return.
    Settlement,
    /// Untagged and unowned VAT the return leaves out: VAT accrued on a VAT account
    /// (output VAT credited, input VAT debited), a reduction booked together with
    /// any other balance-sheet account or with income or expense that is not
    /// interest or penalties (an untagged credit note, return, refund of an advance,
    /// credit loss or deduction correction), a payment whose money account has no
    /// settlement role, a leg on a coded voucher that its codes do not explain, or a
    /// posting whose account has no known VAT direction.
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
    /// Vouchers with an untagged reduction of a VAT account whose chart snapshot has
    /// no settlement roles, so a VAT payment or refund among them cannot be told
    /// apart from a correction and is counted as untagged.
    pub settlement_roles_missing: Vec<SettlementRolesMissing>,
}

/// A voucher whose untagged VAT reduction could not be checked for a settlement
/// because its chart snapshot has no settlement-role accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SettlementRolesMissing {
    pub voucher: u64,
    pub chart_version: String,
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
/// tells which postings are income, expense or balance-sheet. An account unknown to
/// the chart never lets a posting become a settlement.
///
/// A reduction-side untagged posting is a settlement when every other posting in
/// the voucher is on a VAT-role account, on a settlement-role account of the
/// voucher's own chart (`settlement`: the bank and tax accounts VAT is paid and
/// refunded through), or an interest or penalty line: beside an output-VAT
/// reduction an expense or liability debit (interest or a penalty, expensed or
/// accrued earlier), beside an input-VAT reduction an income credit or asset credit
/// (refund interest). The interest and penalty lines together must be smaller than
/// the voucher's untagged reduction-side VAT. Any other balance-sheet leg (advances
/// received, fixed assets, payables, receivables, equity) makes the reduction
/// untagged, and so does a money leg whose account has no settlement role: without
/// roles (`settlement` is `None`) only a clearing between VAT accounts is a
/// settlement, and no payment or refund, even netted with a clearing. A credit
/// note or credit loss books a net several times its VAT at any rate (at most
/// 25,5 %). The settlement role is detection evidence only; it never decides a
/// treatment or a filing figure.
///
/// A voucher with a `vat_code` owns its VAT legs only as far as its codes explain
/// them: per direction, legs that differ from the voucher's own `base × rate` by more
/// than a cent per coded line are `Untagged` (a line whose code was forgotten, or a
/// code of the other direction next to an uncoded leg). Codes with no VAT leg at all
/// stay `Tagged`; the return then follows the codes.
pub fn classify_vat_account_postings(
    voucher: &VoucherBody,
    roles: &ChartVatRoles,
    settlement: Option<&ChartSettlementAccounts>,
    kind_of: &dyn Fn(&str) -> Option<AccountKind>,
) -> Result<Vec<(usize, Option<GeneratedVatRole>, VatAccountPostingClass)>, DomainError> {
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
    let postings = voucher.postings();
    let reduces = |index: usize| {
        let posting = &postings[index];
        !generated.contains(&index)
            && roles.role_of(posting.account.as_str()).is_some()
            && reduces_vat(direction_of(posting.account.as_str()), posting.side)
    };
    let settles = settles_vat(voucher, roles, settlement, &reduces, kind_of);
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
        } else if reduces(index)
            && match direction {
                Some(GeneratedVatRole::Output) => settles[0],
                Some(GeneratedVatRole::Input) => settles[1],
                None => false,
            }
        {
            VatAccountPostingClass::Settlement
        } else {
            VatAccountPostingClass::Untagged
        };
        classified.push((index, direction, class));
    }
    Ok(classified)
}

/// Per direction `[output, input]`, whether an untagged voucher's reduction-side VAT
/// postings of that direction are a settlement: every account is known to the
/// chart, every amount is in one currency, every posting off the VAT-role and
/// settlement-role accounts is an interest or penalty line of that direction
/// (expense or liability debits for output, income or asset credits for input), and
/// those lines sum to less than the reduction-side VAT postings.
fn settles_vat(
    voucher: &VoucherBody,
    roles: &ChartVatRoles,
    settlement: Option<&ChartSettlementAccounts>,
    reduces: &dyn Fn(usize) -> bool,
    kind_of: &dyn Fn(&str) -> Option<AccountKind>,
) -> [bool; 2] {
    let postings = voucher.postings();
    let Some(currency) = postings.first().map(|p| p.amount.currency()) else {
        return [false; 2];
    };
    let mut reduced = Decimal::ZERO;
    let mut interest = Decimal::ZERO;
    let mut foreign = [false; 2];
    for (index, posting) in postings.iter().enumerate() {
        if posting.amount.currency() != currency {
            return [false; 2];
        }
        let account = posting.account.as_str();
        let Some(kind) = kind_of(account) else {
            return [false; 2];
        };
        if roles.role_of(account).is_some() {
            if reduces(index) {
                reduced += posting.amount.amount();
            }
            continue;
        }
        // Without settlement roles only a clearing between VAT accounts settles: any
        // other leg, a money leg included, could be a correction's and fails closed.
        let Some(settlement) = settlement else {
            return [false; 2];
        };
        if settlement.contains(account) {
            continue;
        }
        interest += posting.amount.amount();
        match (kind, posting.side) {
            (AccountKind::Expense | AccountKind::Liability, Side::Debit) => foreign[1] = true,
            (AccountKind::Income | AccountKind::Asset, Side::Credit) => foreign[0] = true,
            _ => foreign = [true; 2],
        }
    }
    let small = interest.is_zero() || interest < reduced;
    foreign.map(|foreign| small && !foreign)
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
/// exact chart snapshot, VAT roles and settlement roles, as the filing projection
/// does; a snapshot without settlement roles makes its payments untagged.
pub fn reconcile_vat_accounts(
    period: &PeriodId,
    vouchers: &[VoucherBody],
    derived: &DerivedVatFigures,
    role_table: &VatAccountRoleTable,
    settlement_table: &SettlementAccountRoleTable,
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
    let mut settlement_roles_missing = Vec::new();

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
        let settlement = settlement_table.resolve(voucher.chart_version());
        let classified = classify_vat_account_postings(voucher, roles, settlement, kind_of)?;
        if settlement.is_none()
            && classified.iter().any(|(index, direction, class)| {
                *class == VatAccountPostingClass::Untagged
                    && reduces_vat(*direction, voucher.postings()[*index].side)
            })
        {
            settlement_roles_missing.push(SettlementRolesMissing {
                voucher: voucher.number(),
                chart_version: voucher.chart_version().to_string(),
            });
        }
        for (index, direction, class) in classified {
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
    // A reduction that may be a payment only because its chart has no settlement roles
    // never reconciles by netting against another untagged posting of the period: the
    // roles are missing, so the comparison cannot be trusted either way.
    let unreconciled = uncomparable
        || !settlement_roles_missing.is_empty()
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
        settlement_roles_missing,
    })
}

/// Whether a posting of this VAT direction and side reduces the VAT balance (output
/// VAT debited, input VAT credited).
pub fn reduces_vat(direction: Option<GeneratedVatRole>, side: Side) -> bool {
    matches!(
        (direction, side),
        (Some(GeneratedVatRole::Output), Side::Debit)
            | (Some(GeneratedVatRole::Input), Side::Credit)
    )
}

const SETTLEMENT_ACCOUNT_ROLES_JSON: &str =
    include_str!("../../data/settlement-account-roles.json");

/// The settlement-role accounts of one exact chart snapshot: the bank and tax
/// accounts VAT is paid to and refunded from. Detection evidence for the VAT
/// reconciliation only; it never enters a filing figure or a VAT treatment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartSettlementAccounts {
    chart_version: String,
    accounts: Vec<String>,
}

impl ChartSettlementAccounts {
    pub fn chart_version(&self) -> &str {
        &self.chart_version
    }

    pub fn accounts(&self) -> &[String] {
        &self.accounts
    }

    pub fn contains(&self, account: &str) -> bool {
        self.accounts.iter().any(|a| a == account)
    }
}

/// Settlement-account roles keyed by exact `chart_version`, kept in their own
/// masterdata file so that `home upgrade` can add them to an existing home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementAccountRoleTable {
    table: String,
    charts: Vec<ChartSettlementAccounts>,
}

impl SettlementAccountRoleTable {
    /// The table bundled for the standard chart.
    pub fn bundled() -> &'static Self {
        static TABLE: OnceLock<SettlementAccountRoleTable> = OnceLock::new();
        TABLE.get_or_init(|| {
            Self::from_json(SETTLEMENT_ACCOUNT_ROLES_JSON)
                .expect("bundled settlement-account-roles.json is a valid role table")
        })
    }

    /// A home without the role file: no chart has settlement roles.
    pub fn empty() -> Self {
        Self {
            table: String::new(),
            charts: Vec::new(),
        }
    }

    /// Parse and validate the role file's own shape. Whether each account exists in
    /// its chart snapshot with a balance-sheet kind and carries no VAT role is
    /// checked by the masterdata, which holds the snapshots.
    pub fn from_json(json: &str) -> Result<Self, DomainError> {
        let table: Self = serde_json::from_str(json).map_err(|error| {
            DomainError::VatStructure(format!("invalid settlement-account roles: {error}"))
        })?;
        let mut versions = std::collections::BTreeSet::new();
        for chart in &table.charts {
            let canonical = chart
                .chart_version
                .split_once('@')
                .is_some_and(|(name, rev)| {
                    !name.is_empty()
                        && !rev.is_empty()
                        && !rev.contains('@')
                        && name.trim() == name
                        && rev.trim() == rev
                });
            if !canonical {
                return Err(DomainError::VatStructure(format!(
                    "settlement-account roles: {:?} is not a canonical name@revision chart_version",
                    chart.chart_version
                )));
            }
            if !versions.insert(chart.chart_version.as_str()) {
                return Err(DomainError::VatStructure(format!(
                    "settlement-account roles: chart_version {:?} is listed twice",
                    chart.chart_version
                )));
            }
            if chart.accounts.is_empty() {
                return Err(DomainError::VatStructure(format!(
                    "settlement-account roles: chart_version {:?} lists no accounts",
                    chart.chart_version
                )));
            }
            let mut accounts = std::collections::BTreeSet::new();
            for account in &chart.accounts {
                AccountId::new(account.as_str()).map_err(|error| {
                    DomainError::VatStructure(format!(
                        "settlement-account roles: chart {:?} account {account:?}: {error}",
                        chart.chart_version
                    ))
                })?;
                if !accounts.insert(account.as_str()) {
                    return Err(DomainError::VatStructure(format!(
                        "settlement-account roles: chart {:?} lists account {account} twice",
                        chart.chart_version
                    )));
                }
            }
        }
        Ok(table)
    }

    pub fn snapshots(&self) -> &[ChartSettlementAccounts] {
        &self.charts
    }

    /// The settlement roles of exactly this chart version, or `None`. There is no
    /// fallback to another edition: account numbers mean nothing outside their chart.
    pub fn resolve(&self, chart_version: &str) -> Option<&ChartSettlementAccounts> {
        self.charts
            .iter()
            .find(|c| c.chart_version == chart_version)
    }
}
