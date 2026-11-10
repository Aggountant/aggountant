use super::*;

/// Generated VAT balance direction exposed for callers inspecting chart roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VatAccountRole {
    InputVat,
    OutputVat,
}

/// The pinned v1 default chart name (the prefix of its `chart_version`). The role
/// masterdata and [`vat_expand`]'s account constants are both bound to it.
pub(super) const DEFAULT_CHART: &str = "fi-general-2026";

pub(super) const VAT_ACCOUNT_ROLES_JSON: &str = include_str!("../../data/vat-account-roles.json");

/// A VAT account's legacy role tag in a chart's masterdata role table. New snapshots
/// may add `generated_role` independently, allowing an account such as `2939` to retain
/// its tagged-path meaning while also carrying provenance-marked generated output VAT.
/// Treatment selection belongs to explicit chart targets, not to this account tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VatRoleTag {
    InputVat,
    ReverseChargeOutputVat,
    TaggedOnly,
}

impl VatRoleTag {
    fn implied_generated_role(self) -> Option<GeneratedVatRole> {
        match self {
            Self::InputVat => Some(GeneratedVatRole::Input),
            Self::ReverseChargeOutputVat => Some(GeneratedVatRole::Output),
            Self::TaggedOnly => None,
        }
    }
}

/// One VAT account's role tag in a chart snapshot's masterdata.
///
/// Reverse-charge output roles carry their scope and paired deductible-input account
/// explicitly. The pair is chart metadata: `1798 ↔ 2992` is true for the imported
/// Briox chart, not a universal meaning of those account numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartVatAccount {
    pub(super) account: String,
    pub(super) role: VatRoleTag,
    /// Generated-leg capability, separate from the legacy tagged-path role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) generated_role: Option<GeneratedVatRole>,
    /// Legacy target metadata retained so already-pinned role snapshots keep meaning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) reverse_charge_scope: Option<ReverseChargeScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) paired_input_account: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReverseChargeTarget {
    pub(super) scope: ReverseChargeScope,
    pub(super) input_account: String,
    pub(super) output_account: String,
}

impl ChartVatAccount {
    pub fn account(&self) -> &str {
        &self.account
    }

    pub fn role(&self) -> VatRoleTag {
        self.role
    }

    pub fn reverse_charge_scope(&self) -> Option<ReverseChargeScope> {
        self.reverse_charge_scope
    }

    pub fn paired_input_account(&self) -> Option<&str> {
        self.paired_input_account.as_deref()
    }
}

/// One `chart-account → dedicated rate class` expectation driving the record-time
/// mistag guard ([`check_vat_rate_class_mistag`], Guard C). The account's supply has a
/// dedicated VAT treatment; a generic band code on it silently files the wrong rate in
/// any window where the dedicated class diverges from the generic one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MistagGuard {
    pub(super) account: String,
    pub(super) dedicated_rate_class: String,
}

impl MistagGuard {
    pub fn account(&self) -> &str {
        &self.account
    }

    pub fn dedicated_rate_class(&self) -> &str {
        &self.dedicated_rate_class
    }
}

/// One content-addressed chart snapshot's VAT metadata.
///
/// This is keyed by the complete `chart_version`, not the chart-name prefix. An
/// imported snapshot owns these roles, and every new version is reviewed independently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartVatRoles {
    pub(super) chart_version: String,
    pub(super) active: bool,
    pub(super) accounts: Vec<ChartVatAccount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) domestic_input_account: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) reverse_charge_targets: Vec<ReverseChargeTarget>,
    #[serde(default)]
    pub(super) mistag_guards: Vec<MistagGuard>,
}

impl ChartVatRoles {
    pub fn chart_version(&self) -> &str {
        &self.chart_version
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn accounts(&self) -> &[ChartVatAccount] {
        &self.accounts
    }

    pub fn mistag_guards(&self) -> &[MistagGuard] {
        &self.mistag_guards
    }

    /// Resolve the explicit chart-owned input/output target for a treatment. Legacy
    /// account-attached targets remain readable only for old snapshots.
    pub fn reverse_charge_target(&self, scope: ReverseChargeScope) -> Option<(&str, &str)> {
        if !self.reverse_charge_targets.is_empty() {
            let target = self
                .reverse_charge_targets
                .iter()
                .find(|target| target.scope == scope)?;
            return Some((&target.input_account, &target.output_account));
        }
        let output = self.accounts.iter().find(|account| {
            account.role == VatRoleTag::ReverseChargeOutputVat
                && account.reverse_charge_scope == Some(scope)
        })?;
        Some((output.paired_input_account.as_deref()?, &output.account))
    }

    /// Resolve the chart-owned deductible domestic-input VAT account. New snapshots
    /// select it explicitly; the old inference is retained only for old snapshots.
    pub fn domestic_input_target(&self) -> Option<&str> {
        if !self.reverse_charge_targets.is_empty() {
            return self.domestic_input_account.as_deref();
        }
        if let Some(account) = self.domestic_input_account.as_deref() {
            return Some(account);
        }
        let mut unpaired = self.accounts.iter().filter(|account| {
            account.role == VatRoleTag::InputVat
                && !self.accounts.iter().any(|output| {
                    output.role == VatRoleTag::ReverseChargeOutputVat
                        && output.paired_input_account.as_deref() == Some(account.account.as_str())
                })
        });
        let first_unpaired = unpaired.next();
        let has_multiple_unpaired = unpaired.next().is_some();
        match self.reverse_charge_target(ReverseChargeScope::NonEuServices) {
            Some((input, _)) if first_unpaired.is_none() => Some(input),
            Some(_) => None,
            None if !has_multiple_unpaired => first_unpaired.map(|role| role.account.as_str()),
            None => None,
        }
    }

    /// The role tag of `account` under this chart, or `None` for an ordinary
    /// (non-VAT) account.
    pub fn role_of(&self, account: &str) -> Option<VatRoleTag> {
        self.accounts
            .iter()
            .find(|a| a.account == account)
            .map(|a| a.role)
    }

    /// Generated-leg capability under this exact chart snapshot. Old account tags
    /// imply the same capability they historically carried.
    pub(super) fn generated_role_of(&self, account: &str) -> Option<GeneratedVatRole> {
        let account = self.accounts.iter().find(|a| a.account == account)?;
        account
            .generated_role
            .or(account.role.implied_generated_role())
    }

    /// Reverse-charge semantics of an output account in this exact snapshot.
    pub fn reverse_charge_pair(&self, account: &str) -> Option<(ReverseChargeScope, &str)> {
        let role = self.accounts.iter().find(|a| a.account == account)?;
        if role.role != VatRoleTag::ReverseChargeOutputVat {
            return None;
        }
        Some((
            role.reverse_charge_scope?,
            role.paired_input_account.as_deref()?,
        ))
    }
}

/// The bundled VAT account-role masterdata (all charts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatAccountRoleTable {
    pub(super) table: String,
    pub(super) charts: Vec<ChartVatRoles>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVatAccountRoleTable {
    table: String,
    charts: Vec<ChartVatRoles>,
}

impl VatAccountRoleTable {
    pub fn snapshots(&self) -> &[ChartVatRoles] {
        &self.charts
    }

    /// The validated role metadata bundled for the standard chart bootstrap.
    pub fn bundled() -> &'static Self {
        vat_account_role_table()
    }

    /// Parse and validate role metadata supplied by a chart importer.
    pub fn from_json(json: &str) -> Result<Self, DomainError> {
        let raw: RawVatAccountRoleTable = serde_json::from_str(json).map_err(|error| {
            DomainError::VatStructure(format!("invalid VAT account-role metadata: {error}"))
        })?;
        let table = Self {
            table: raw.table,
            charts: raw.charts,
        };
        validate_vat_account_role_table(&table)?;
        Ok(table)
    }

    /// Resolve one exact, activated chart snapshot. Prefix fallback is deliberately
    /// forbidden: roles from another edition can silently turn a filing into 0,00.
    pub fn resolve(&self, chart_version: &str) -> Result<&ChartVatRoles, DomainError> {
        let snapshot = self
            .charts
            .iter()
            .find(|chart| chart.chart_version == chart_version)
            .ok_or_else(|| {
                DomainError::VatStructure(format!(
                    "chart_version {chart_version:?} has no VAT account-role snapshot — refusing to reuse roles from another edition or the default chart"
                ))
            })?;
        if !snapshot.active {
            return Err(DomainError::VatStructure(format!(
                "chart_version {chart_version:?} has VAT metadata but is not activated"
            )));
        }
        if snapshot.accounts.is_empty() {
            return Err(DomainError::VatStructure(format!(
                "chart_version {chart_version:?} is active but has no VAT roles"
            )));
        }
        Ok(snapshot)
    }
}

pub(super) fn vat_account_role_table() -> &'static VatAccountRoleTable {
    static TABLE: OnceLock<VatAccountRoleTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let table = VatAccountRoleTable::from_json(VAT_ACCOUNT_ROLES_JSON)
            .expect("bundled vat-account-roles.json is a valid role table");
        validate_bundled_default(&table)
            .expect("bundled role table agrees with the standard chart");
        table
    })
}

/// Reject a malformed role table at load time (bundled data → a violation is a
/// build-time bug the `bundled_vat_role_masterdata_loads_and_validates` test exercises):
/// a syntactically invalid account id (a typo that would silently never match a
/// posting), a duplicated chart name, a duplicated account within a chart, a duplicated
/// mistag guard, a mistag guard naming a `dedicated_rate_class` with no edition in the
/// dated rate table (a typo that would make the guard silently never fire), a missing
/// default chart, or a default chart that has drifted from [`vat_expand`]'s account
/// constants (the "expansion and projection agree" invariant).
pub fn validate_vat_account_role_table(table: &VatAccountRoleTable) -> Result<(), DomainError> {
    use std::collections::BTreeSet;
    let known_rate_classes: BTreeSet<&str> = vat_rate_table()
        .editions
        .iter()
        .map(|e| e.rate_class.as_str())
        .collect();
    let mut seen_versions: BTreeSet<&str> = BTreeSet::new();
    for chart in &table.charts {
        if !seen_versions.insert(chart.chart_version.as_str()) {
            return Err(DomainError::VatStructure(format!(
                "VAT account-role masterdata has a duplicated chart_version {:?}",
                chart.chart_version
            )));
        }
        let Some((name, revision)) = chart.chart_version.split_once('@') else {
            return Err(DomainError::VatStructure(format!(
                "VAT role snapshot {:?} is not a complete chart_version (expected name@revision)",
                chart.chart_version
            )));
        };
        if name.is_empty()
            || revision.is_empty()
            || revision.contains('@')
            || name.trim() != name
            || revision.trim() != revision
        {
            return Err(DomainError::VatStructure(format!(
                "VAT role snapshot {:?} is not a canonical name@revision chart_version",
                chart.chart_version
            )));
        }
        if chart.active && chart.accounts.is_empty() {
            return Err(DomainError::VatStructure(format!(
                "active chart snapshot {:?} has no VAT roles",
                chart.chart_version
            )));
        }

        let mut seen_accounts: BTreeSet<&str> = BTreeSet::new();
        let mut seen_rc_scopes = BTreeSet::new();
        for account in &chart.accounts {
            AccountId::new(&account.account).map_err(|e| {
                DomainError::VatStructure(format!(
                    "chart {:?} has an invalid VAT-role account id {:?}: {e}",
                    chart.chart_version, account.account
                ))
            })?;
            if !seen_accounts.insert(account.account.as_str()) {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} has a duplicated VAT-role account {:?}",
                    chart.chart_version, account.account
                )));
            }
            let implied_generated_role = account.role.implied_generated_role();
            if account.generated_role.is_some()
                && implied_generated_role.is_some()
                && account.generated_role != implied_generated_role
            {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} account {:?} has conflicting legacy and generated VAT roles",
                    chart.chart_version, account.account
                )));
            }
            match account.role {
                VatRoleTag::ReverseChargeOutputVat => {
                    let (Some(scope), Some(pair)) = (
                        account.reverse_charge_scope,
                        account.paired_input_account.as_deref(),
                    ) else {
                        return Err(DomainError::VatStructure(format!(
                            "chart {:?} output VAT account {:?} lacks reverse-charge scope or paired input account",
                            chart.chart_version, account.account
                        )));
                    };
                    if !seen_rc_scopes.insert(scope) {
                        return Err(DomainError::VatStructure(format!(
                            "chart {:?} has more than one output target for reverse-charge scope {scope:?}",
                            chart.chart_version
                        )));
                    }
                    AccountId::new(pair).map_err(|e| {
                        DomainError::VatStructure(format!(
                            "chart {:?} output VAT account {:?} has invalid paired input account {:?}: {e}",
                            chart.chart_version, account.account, pair
                        ))
                    })?;
                }
                _ if account.reverse_charge_scope.is_some()
                    || account.paired_input_account.is_some() =>
                {
                    return Err(DomainError::VatStructure(format!(
                        "chart {:?} account {:?} has reverse-charge pairing metadata but is not output-vat",
                        chart.chart_version, account.account
                    )));
                }
                _ => {}
            }
        }
        for output in chart
            .accounts
            .iter()
            .filter(|a| a.role == VatRoleTag::ReverseChargeOutputVat)
        {
            let pair = output
                .paired_input_account
                .as_deref()
                .expect("output metadata validated above");
            if chart.generated_role_of(pair) != Some(GeneratedVatRole::Input) {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} output VAT account {:?} pairs with {:?}, which is not capable of generated input VAT",
                    chart.chart_version, output.account, pair
                )));
            }
        }

        if !chart.reverse_charge_targets.is_empty() && chart.domestic_input_account.is_none() {
            return Err(DomainError::VatStructure(format!(
                "chart {:?} uses explicit reverse-charge targets but selects no domestic input account",
                chart.chart_version
            )));
        }
        if !chart.reverse_charge_targets.is_empty()
            && chart.accounts.iter().any(|account| {
                account.reverse_charge_scope.is_some() || account.paired_input_account.is_some()
            })
        {
            return Err(DomainError::VatStructure(format!(
                "chart {:?} mixes explicit and legacy reverse-charge targets",
                chart.chart_version
            )));
        }
        let mut explicit_scopes = BTreeSet::new();
        for target in &chart.reverse_charge_targets {
            if !explicit_scopes.insert(target.scope) {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} has more than one target for reverse-charge scope {:?}",
                    chart.chart_version, target.scope
                )));
            }
            AccountId::new(&target.input_account).map_err(|error| {
                DomainError::VatStructure(format!(
                    "chart {:?} reverse-charge target has invalid input account {:?}: {error}",
                    chart.chart_version, target.input_account
                ))
            })?;
            AccountId::new(&target.output_account).map_err(|error| {
                DomainError::VatStructure(format!(
                    "chart {:?} reverse-charge target has invalid output account {:?}: {error}",
                    chart.chart_version, target.output_account
                ))
            })?;
            if chart.generated_role_of(&target.input_account) != Some(GeneratedVatRole::Input)
                || chart.generated_role_of(&target.output_account) != Some(GeneratedVatRole::Output)
            {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} reverse-charge target {:?} does not reference generated input/output capable accounts",
                    chart.chart_version, target.scope
                )));
            }
        }
        if let Some(account) = chart.domestic_input_account.as_deref() {
            AccountId::new(account).map_err(|error| {
                DomainError::VatStructure(format!(
                    "chart {:?} has invalid domestic input target {:?}: {error}",
                    chart.chart_version, account
                ))
            })?;
            if chart.generated_role_of(account) != Some(GeneratedVatRole::Input) {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} domestic input target {:?} is not generated-input capable",
                    chart.chart_version, account
                )));
            }
        }

        let mut seen_mistag: BTreeSet<&str> = BTreeSet::new();
        for guard in &chart.mistag_guards {
            AccountId::new(&guard.account).map_err(|e| {
                DomainError::VatStructure(format!(
                    "chart {:?} mistag guard has an invalid account id {:?}: {e}",
                    chart.chart_version, guard.account
                ))
            })?;
            if !seen_mistag.insert(guard.account.as_str()) {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} has a duplicated mistag guard on account {:?}",
                    chart.chart_version, guard.account
                )));
            }
            if !known_rate_classes.contains(guard.dedicated_rate_class.as_str()) {
                return Err(DomainError::VatStructure(format!(
                    "chart {:?} mistag guard on account {:?} names dedicated rate class {:?}, which has no edition in the dated rate table",
                    chart.chart_version, guard.account, guard.dedicated_rate_class
                )));
            }
        }
    }

    Ok(())
}

/// Extra invariant for the roles bundled with the standard chart. Imported tables are
/// valid without carrying the standard chart alongside them; only the bundled registry
/// is required to agree with the standard bootstrap.
pub(super) fn validate_bundled_default(table: &VatAccountRoleTable) -> Result<(), DomainError> {
    let defaults: Vec<_> = table
        .charts
        .iter()
        .filter(|c| c.active && chart_name(&c.chart_version) == DEFAULT_CHART)
        .collect();
    if defaults.is_empty() {
        return Err(DomainError::VatStructure(format!(
            "VAT account-role masterdata is missing an active {DEFAULT_CHART:?} snapshot"
        )));
    }
    for default in defaults {
        for (account, expected) in EXPECTED_DEFAULT_CHART_ROLES {
            if default.role_of(account) != Some(*expected) {
                return Err(DomainError::VatStructure(format!(
                    "default snapshot {:?} must tag account {account:?} as {expected:?}",
                    default.chart_version
                )));
            }
        }
    }
    Ok(())
}

/// The default chart's VAT-account roles that must match [`vat_expand`]'s constants —
/// the VAT accounts that really exist in the bundled default chart. Reverse-charge
/// accounts are deliberately absent: they enter through an imported company snapshot
/// and must never become assumptions of the default chart.
pub(super) const EXPECTED_DEFAULT_CHART_ROLES: &[(&str, VatRoleTag)] = &[
    (ACCT_INPUT_VAT_DOMESTIC, VatRoleTag::InputVat),
    ("2939", VatRoleTag::TaggedOnly),
];

/// The chart name embedded in a `chart_version` (`"fi-general-2026@<hash>"` →
/// `"fi-general-2026"`). Used only for diagnostics and default-family validation;
/// role resolution itself always compares the complete version.
pub(super) fn chart_name(chart_version: &str) -> &str {
    chart_version
        .split_once('@')
        .map(|(name, _)| name)
        .expect("validated chart_version contains '@'")
}

/// True iff `account` carries **any** VAT role (input/output/tagged-only) in the pinned
/// default chart. Used by `vat_expand`'s net/contra guard, which is default-chart-bound
/// by construction. `TaggedOnly` (domestic output VAT, `2939`) is included: it too is a
/// VAT account, so accepting it as an expansion's net/contra would seal a voucher the
/// account-sum projection (Guard B) later refuses at filing — reject it here, at the
/// cheaper record-time boundary, rather than let a doomed voucher seal.
const LEGACY_EXPANDER_RC_ACCOUNTS: &[&str] = &[
    ACCT_INPUT_VAT_RC_EU_SERVICES,
    ACCT_INPUT_VAT_RC_EU_GOODS,
    ACCT_OUTPUT_VAT_RC_EU_SERVICES,
    ACCT_OUTPUT_VAT_RC_EU_GOODS,
    ACCT_OUTPUT_VAT_RC_NON_EU,
];

pub(super) fn is_default_chart_vat_account(account: &str) -> bool {
    // Keep the legacy expander's net/contra safety guard until expansion itself accepts
    // chart metadata. These numbers are not default roles; they are forbidden business
    // legs because the old expander can still emit them as VAT legs.
    LEGACY_EXPANDER_RC_ACCOUNTS.contains(&account)
        || vat_account_role_table()
            .charts
            .iter()
            .filter(|c| c.active && chart_name(&c.chart_version) == DEFAULT_CHART)
            .any(|c| c.role_of(account).is_some())
}

/// Sum a period's hash-bound **generated VAT-A legs** into its filing figures.
/// Marker indices identify the generated postings; their input/output roles are checked
/// against the voucher's exact `chart_version` before their sealed EUR cents are summed.
/// Ordinary VAT-account settlements, clearing entries, corrections and reclassifications
/// have no generated marker and contribute nothing.
///
/// A voucher carrying any `vat_code` remains wholly owned by [`derive_vat_figures`]'s
/// legacy base×rate projection, keeping the two filing paths disjoint. Only vouchers
/// whose `posting_date` maps to `period` are counted.
pub fn sum_vat_account_figures(
    period: &PeriodId,
    vouchers: &[VoucherBody],
) -> Result<DerivedVatFigures, DomainError> {
    sum_vat_account_figures_with_role_table(period, vouchers, vat_account_role_table())
}

/// Account-sum projection against role metadata supplied by a chart importer.
/// Every voucher resolves by its complete `chart_version`; inactive, roleless, and
/// missing snapshots fail closed through [`VatAccountRoleTable::resolve`].
pub fn sum_vat_account_figures_with_role_table(
    period: &PeriodId,
    vouchers: &[VoucherBody],
    role_table: &VatAccountRoleTable,
) -> Result<DerivedVatFigures, DomainError> {
    let eur = Currency::new("EUR").expect("EUR is a valid currency code");
    let mut sales = Decimal::ZERO;
    let mut purchase = Decimal::ZERO;
    let mut fields = VatReturnFields::new();
    for voucher in vouchers {
        if !period.contains(voucher.posting_date()) {
            continue;
        }
        // Preserve the existing exact chart-activation gate for every filing voucher.
        let roles = role_table
            .resolve(voucher.chart_version())
            .map_err(|error| {
                DomainError::VatStructure(format!("voucher {}: {error}", voucher.number()))
            })?;
        // Tagged vouchers remain wholly owned by the legacy base×rate projection.
        if voucher.postings().iter().any(|p| p.vat_code.is_some()) {
            continue;
        }
        let Some(expansion) = voucher.vat_expansion() else {
            // Ordinary settlements, clearing entries and manual reclassifications may
            // legitimately move a VAT account. They are not generated VAT-A legs and
            // therefore do not belong in the filing projection.
            continue;
        };
        // Return category comes only from the sealed treatment. Account identity is
        // deliberately absent: two service treatments may use the same chart pair.
        let category_fields = match expansion.treatment() {
            VatExpansionTreatment::ReverseCharge {
                scope: VatExpansionScope::EuGoods,
                ..
            }
            | VatExpansionTreatment::ReverseChargePartial {
                scope: VatExpansionScope::EuGoods,
                ..
            } => (
                Some(VatReturnField::Field312),
                Some(VatReturnField::Field305),
            ),
            VatExpansionTreatment::ReverseCharge {
                scope: VatExpansionScope::EuServices,
                ..
            }
            | VatExpansionTreatment::ReverseChargePartial {
                scope: VatExpansionScope::EuServices,
                ..
            } => (
                Some(VatReturnField::Field313),
                Some(VatReturnField::Field306),
            ),
            VatExpansionTreatment::ReverseCharge {
                rate_class,
                scope: VatExpansionScope::NonEuServices,
            }
            | VatExpansionTreatment::ReverseChargePartial {
                rate_class,
                scope: VatExpansionScope::NonEuServices,
                ..
            } => (None, Some(vat_rate_class_field(rate_class)?)),
            VatExpansionTreatment::NoVat
            | VatExpansionTreatment::DomesticInput { .. }
            | VatExpansionTreatment::DomesticInputPartial { .. } => (None, None),
        };
        let (base_field, tax_field) = category_fields;
        if base_field.is_some() || tax_field.is_some() {
            if expansion.source_amount().currency() != &eur {
                return Err(DomainError::VatCurrency(format!(
                    "VAT expansion source amount on voucher {} is {}, but filing is EUR-only",
                    voucher.number(),
                    expansion.source_amount().currency()
                )));
            }
            if let Some(base_field) = base_field {
                fields
                    .add_computed(base_field, expansion.source_amount().clone())
                    .map_err(|error| DomainError::VatStructure(error.to_string()))?;
            }
            if let Some(tax_field) = tax_field {
                fields
                    .add_computed(tax_field, Money::new(eur_2dp(Decimal::ZERO), eur.clone()))
                    .map_err(|error| DomainError::VatStructure(error.to_string()))?;
            }
        }
        for marker in expansion.generated_vat_legs() {
            let posting = &voucher.postings()[marker.posting_index];
            let actual_role = roles.generated_role_of(posting.account.as_str());
            if actual_role != Some(marker.role) {
                return Err(DomainError::VatStructure(format!(
                    "voucher {} marks posting {} on account {} as generated {:?}, but chart_version {:?} does not assign that VAT role",
                    voucher.number(), marker.posting_index, posting.account, marker.role, voucher.chart_version()
                )));
            }
            if posting.amount.currency() != &eur {
                return Err(DomainError::VatCurrency(format!(
                    "generated VAT leg on voucher {} is {}, but the filing projection is EUR-only",
                    voucher.number(),
                    posting.amount.currency()
                )));
            }
            if posting.amount.amount() != round_eur(posting.amount.amount()) {
                return Err(DomainError::VatCurrency(format!(
                    "generated VAT leg on voucher {} is {}, not an exact EUR-cent amount",
                    voucher.number(),
                    posting.amount.amount_str()
                )));
            }
            match marker.role {
                GeneratedVatRole::Input => purchase += posting.signed_amount(),
                GeneratedVatRole::Output => {
                    let tax = -posting.signed_amount();
                    sales += tax;
                    if let Some(tax_field) = tax_field {
                        fields
                            .add_computed(tax_field, Money::new(eur_2dp(tax), eur.clone()))
                            .map_err(|error| DomainError::VatStructure(error.to_string()))?;
                    }
                }
            }
        }
    }
    Ok(DerivedVatFigures {
        period: period.clone(),
        sales_vat: Money::new(eur_2dp(sales), eur.clone()),
        purchase_vat: Money::new(eur_2dp(purchase), eur),
        fields,
    })
}

/// The authoritative aggregated VAT figures for a period's ALV filing (ADR-7 §3): the
/// **sum** of the legacy tagged `base × rate` figures ([`derive_vat_figures`], for
/// domestic sales/purchases still recorded on the tagged path) and the account-sum
/// over the sealed VAT-A legs ([`sum_vat_account_figures`], for reverse charge +
/// domestic input VAT whose euros sit on the VAT accounts). The two are **disjoint at
/// the voucher level**: a voucher carrying any `vat_code` is owned by the tagged path
/// and skipped in full by the account-sum, while a pure-untagged VAT-A voucher is
/// owned by the account-sum and ignored by the tagged path — so no voucher's VAT is
/// counted twice, even a legacy reverse charge whose deductible leg is tagged on a VAT
/// account.
///
/// This is what `file-vat` files. A VAT-A period that would derive to 0,00 on the
/// tagged path alone now files the real self-assessed / deductible amounts.
pub fn derive_filing_figures(
    period: &PeriodId,
    vouchers: &[VoucherBody],
) -> Result<DerivedVatFigures, DomainError> {
    derive_filing_figures_with_role_table(period, vouchers, vat_account_role_table())
}

/// Combined filing projection using an importer-owned chart-role table.
pub fn derive_filing_figures_with_role_table(
    period: &PeriodId,
    vouchers: &[VoucherBody],
    role_table: &VatAccountRoleTable,
) -> Result<DerivedVatFigures, DomainError> {
    for voucher in vouchers {
        if period.contains(voucher.posting_date()) {
            verify_vat_expansion_with_role_table(voucher, role_table)?;
        }
    }
    let tagged = derive_vat_figures(period, vouchers)?;
    let accounts = sum_vat_account_figures_with_role_table(period, vouchers, role_table)?;
    let eur = Currency::new("EUR").expect("EUR is a valid currency code");
    let mut fields = tagged.fields.clone();
    fields
        .merge_from(&accounts.fields)
        .map_err(|error| DomainError::VatStructure(error.to_string()))?;
    Ok(DerivedVatFigures {
        period: period.clone(),
        sales_vat: Money::new(
            eur_2dp(tagged.sales_vat.amount() + accounts.sales_vat.amount()),
            eur.clone(),
        ),
        purchase_vat: Money::new(
            eur_2dp(tagged.purchase_vat.amount() + accounts.purchase_vat.amount()),
            eur,
        ),
        fields,
    })
}

/// Project the deductible input-VAT effect of a draft exactly as the filing path will
/// read it after sealing under `chart_version`.
///
/// This is the pre-seal counterpart of [`derive_filing_figures`]: it freezes a transient
/// voucher body against the exact chart snapshot selected by the ledger, then reuses the
/// same voucher-level path selection. A tagged draft is therefore read through the
/// legacy `base × rate` path, while a pure VAT-A draft is read from chart-owned input-VAT
/// account roles. Unknown or inactive chart metadata fails closed rather than certifying
/// a zero effect.
pub fn draft_deductible_vat(
    draft: &VoucherDraft,
    chart_version: &str,
) -> Result<Money, DomainError> {
    draft_deductible_vat_with_role_table(draft, chart_version, vat_account_role_table())
}

/// Pre-seal deductible-VAT projection against the company chart's imported role table.
pub fn draft_deductible_vat_with_role_table(
    draft: &VoucherDraft,
    chart_version: &str,
    role_table: &VatAccountRoleTable,
) -> Result<Money, DomainError> {
    // Preserve receipt capture's pre-seal diagnostic for legacy drafts. Sealed filing
    // is intentionally stricter and counts only hash-bound generated markers; changing
    // capture input or its approval behavior is outside this issue.
    if draft.vat_expansion().is_none() && draft.postings().iter().all(|p| p.vat_code.is_none()) {
        let roles = role_table.resolve(chart_version)?;
        let eur = Currency::new("EUR").expect("EUR is valid");
        let mut purchase = Decimal::ZERO;
        for posting in draft.postings() {
            match roles.role_of(posting.account.as_str()) {
                Some(VatRoleTag::TaggedOnly) => {
                    return Err(DomainError::VatStructure(format!(
                        "voucher 1 is a pure VAT-A (untagged) voucher but moves VAT account {}, whose VAT is owned by the tagged base×rate path",
                        posting.account
                    )));
                }
                Some(VatRoleTag::InputVat) => {
                    if posting.amount.currency() != &eur
                        || posting.amount.amount() != round_eur(posting.amount.amount())
                    {
                        return Err(DomainError::VatCurrency(format!(
                            "VAT-account leg is not an exact EUR-cent amount: {} {}",
                            posting.amount.amount_str(),
                            posting.amount.currency()
                        )));
                    }
                    purchase += posting.signed_amount();
                }
                _ => {}
            }
        }
        return Ok(Money::new(eur_2dp(purchase), eur));
    }
    let body = VoucherBody::from_draft(draft, 1, chart_version.to_string())?;
    let period = PeriodId::containing(draft.posting_date());
    Ok(derive_filing_figures_with_role_table(&period, &[body], role_table)?.purchase_vat)
}

// ---------------------------------------------------------------------------
// F25 — record-time rate-class mistag guard
// ---------------------------------------------------------------------------
//
// The dated rate table carries **dedicated** rate classes for supplies whose rate has
// diverged from the generic band over time — `domestic-passenger-transport` and
// `domestic-electricity` (temporary-measure windows, e.g. passenger transport at 0 %
// in Jan–Apr 2023, electricity at 10 % in the 2022-12…2023-04 window). Those classes
// only *offer* the correct spelling; nothing forces it. A generic band code on such a
// supply (`purchase-domestic-standard` on an electricity invoice in the temp window)
// is a valid class on that date, so it resolves *silently* to the **wrong** rate —
// exactly the mistag hazard all four `vat-policy-full-rate-history` reviewers flagged
// as this issue's twin.
//
// The guard maps, per chart, an account whose supply has a dedicated treatment to that
// dedicated rate class (masterdata: `mistag_guards`). At record time it refuses a
// posting on such an account whose generic code resolves to a *different* rate than the
// dedicated class on the voucher's `document_date` — forcing the dedicated class where,
// and only where, the choice actually changes the filed euro. Where the two coincide
// (electricity = standard outside the temp window) either spelling files the same rate,
// so the guard stays silent and does not block ordinary bookings.
//
// **Scope / known trade-offs (default chart, honestly stated).**
//   * The mapping is account-granular, but the default chart's `7010 Sähkö ja lämmitys`
//     lumps electricity *and* heating, while the temporary 10 % measure was
//     electricity-only. So a genuinely standard-rated district-heating (kaukolämpö)
//     invoice on `7010` inside the window is *refused* as a suspected electricity
//     mistag. That is a deliberate fail-loud-cheap trade-off (a refused draft is a
//     cheap re-entry — use the dedicated electricity class, or split the heating leg —
//     versus a silently mis-rated filing). A single-supply account (per-line supply
//     metadata) removes it and belongs to `chart-adoption`.
//   * `domestic-passenger-transport` (temp 0 % Jan–Apr 2023) has **no** unambiguous
//     account in the default chart (`7300 Matkakulut` also carries lodging/meals), so
//     no mistag guard is registered for it here — the boundary recorded on the issue.
//     Its guard enters with a company chart that has a dedicated transport account
//     (`chart-adoption`).
//   * This is the *rate* mistag only. A wrong-direction / wrong-scheme code that
//     resolves to the same euro (e.g. `sales-domestic-electricity` on an expense
//     account) is a different hazard, left to a future per-account allowed-treatment
//     guard (spun off) that carries the full expected treatment tuple.

/// The generic domestic rate-class bands — the ordinary standard/reduced codes an
/// operator might reach for *instead of* a dedicated domestic class. Only these are
/// subject to the rate-mistag guard; a structurally different treatment (reverse
/// charge, EU acquisition, margin, zero/exempt) booked to a guarded account is not a
/// rate mistag and is left to a future per-account allowed-treatment guard.
pub(super) const GENERIC_DOMESTIC_BANDS: &[&str] = &[
    "domestic-standard",
    "domestic-reduced-1",
    "domestic-reduced-2",
];

/// Record-time rate-class mistag guard (F25). Refuse a generic band VAT code on a
/// posting whose chart account has a **dedicated** rate-class treatment that resolves
/// to a *different* percentage on `document_date` — a mistag that would otherwise file
/// the wrong rate silently. The ledger runs this at seal time (it knows the resolved
/// `chart_version`); a chart with no mistag masterdata has nothing to check and passes.
///
/// The comparison is by resolved percentage, not by class name: a generic code is
/// refused only in a window where the dedicated class actually diverges (a temporary
/// measure / reclassification). Where they coincide, either spelling is correct, so the
/// guard does not block. If either class has no dated edition on the date, a divergence
/// cannot be proven, so the guard defers (a rate-table gap is a separate concern).
pub fn check_vat_rate_class_mistag(
    chart_version: &str,
    document_date: Date,
    postings: &[Posting],
) -> Result<(), DomainError> {
    check_vat_rate_class_mistag_with_role_table(
        vat_account_role_table(),
        chart_version,
        document_date,
        postings,
    )
}

/// Apply record-time mistag guards from importer-owned chart metadata.
pub fn check_vat_rate_class_mistag_with_role_table(
    role_table: &VatAccountRoleTable,
    chart_version: &str,
    document_date: Date,
    postings: &[Posting],
) -> Result<(), DomainError> {
    // Unknown metadata remains a no-op here because this guard can run before chart
    // adoption; filing projection is the hard fail-closed boundary.
    let Some(chart) = role_table
        .charts
        .iter()
        .find(|c| c.active && c.chart_version == chart_version)
    else {
        return Ok(());
    };
    for posting in postings {
        let Some(code) = &posting.vat_code else {
            continue;
        };
        let Some(rule) = chart
            .mistag_guards
            .iter()
            .find(|g| g.account == posting.account.as_str())
        else {
            continue;
        };
        let (_direction, rate_class, _deductibility, _scheme) = parse_vat_code(code.as_str())?;
        if rate_class == rule.dedicated_rate_class {
            continue; // already using the dedicated class — correct.
        }
        // This guard is the **rate** mistag: a *generic domestic band* on a supply whose
        // dedicated class diverges in rate. It fires only for the generic domestic bands
        // — a code the operator could plausibly have reached for *instead of* the
        // dedicated domestic class. A structurally different treatment (reverse charge,
        // EU acquisition, margin, zero/exempt) on the same account is NOT a rate mistag
        // and may be entirely legitimate (an EU reverse-charge electricity acquisition
        // is not "domestic-electricity"), so it is left to a future per-account
        // allowed-treatment guard (spun off) rather than falsely refused here.
        if !GENERIC_DOMESTIC_BANDS.contains(&rate_class) {
            continue;
        }
        // Compare the rate the generic band resolves to against the dedicated class on
        // this date. Only a genuine divergence (a temp-measure / reclassification
        // window) means the generic code would file the wrong euro.
        let (Ok(generic_pct), Ok(dedicated_pct)) = (
            resolve_vat_rate(rate_class, document_date),
            resolve_vat_rate(&rule.dedicated_rate_class, document_date),
        ) else {
            continue;
        };
        if generic_pct != dedicated_pct {
            return Err(DomainError::VatStructure(format!(
                "posting on account {} carries the generic rate class {rate_class:?} \
                 ({generic_pct} %) on {document_date}, but this account's supply has a \
                 dedicated treatment {:?} ({dedicated_pct} %) for that date — the generic \
                 code would silently file the wrong rate. Use a VAT code with rate class \
                 {:?} (or correct the account).",
                posting.account, rule.dedicated_rate_class, rule.dedicated_rate_class
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ADR-7 (VAT-A) §2 — the recompute cross-check
// ---------------------------------------------------------------------------
//
// The filing (above) sums only sealed generated-leg markers. The recompute verifier
// uses the same hash-bound expansion record as its exact input: treatment (including
// partial allocation provenance), source amount, accounts, resolved rate and policy.
// It never infers a treatment from rounded cents.

/// The outcome of the ADR-7 §2 recompute cross-check on one sealed voucher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum VatRecompute {
    /// The voucher has no hash-bound VAT-A expansion provenance.
    NotApplicable,
    /// Re-running [`vat_expand`] from the sealed treatment and resolved provenance
    /// reproduced both postings and provenance exactly.
    Verified { treatment: VatTreatment },
    /// Legacy response shape retained for callers reading historical results. Exact
    /// provenance-backed verification no longer emits this variant.
    ConsistentIndeterminate {
        pattern: VatExpansionPattern,
        source_amount: Money,
        deductible_vat: Money,
        candidates: Vec<VatRecomputeCandidate>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum VatExpansionPattern {
    DomesticInput,
    ReverseCharge { scope: ReverseChargeScope },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatRecomputeCandidate {
    pub rate_class: String,
    pub full_vat: Money,
}

/// Resolve reverse-charge semantics from this exact chart snapshot. No account number
/// has a global reverse-charge meaning.
#[cfg(test)]
pub(super) fn reverse_charge_pair<'a>(
    roles: &'a ChartVatRoles,
    account: &str,
) -> Option<(ReverseChargeScope, &'a str)> {
    roles.reverse_charge_pair(account)
}

fn domain_treatment(treatment: &VatExpansionTreatment) -> Result<VatTreatment, DomainError> {
    Ok(match treatment {
        VatExpansionTreatment::NoVat => VatTreatment::NoVat,
        VatExpansionTreatment::DomesticInput { rate_class } => VatTreatment::DomesticInput {
            rate_class: rate_class.clone(),
        },
        VatExpansionTreatment::DomesticInputPartial {
            rate_class,
            deductibility,
        } => VatTreatment::DomesticInputPartial {
            rate_class: rate_class.clone(),
            deductibility: domain_deductibility(deductibility)?,
        },
        VatExpansionTreatment::ReverseCharge { rate_class, scope } => VatTreatment::ReverseCharge {
            rate_class: rate_class.clone(),
            scope: domain_scope(scope),
        },
        VatExpansionTreatment::ReverseChargePartial {
            rate_class,
            scope,
            deductibility,
        } => VatTreatment::ReverseChargePartial {
            rate_class: rate_class.clone(),
            scope: domain_scope(scope),
            deductibility: domain_deductibility(deductibility)?,
        },
    })
}

fn domain_scope(scope: &VatExpansionScope) -> ReverseChargeScope {
    match scope {
        VatExpansionScope::EuServices => ReverseChargeScope::EuServices,
        VatExpansionScope::EuGoods => ReverseChargeScope::EuGoods,
        VatExpansionScope::NonEuServices => ReverseChargeScope::NonEuServices,
    }
}

fn domain_deductibility(
    value: &VatExpansionDeductibility,
) -> Result<VatDeductibility, DomainError> {
    let VatExpansionDeductibilityProvenance::DocumentedUseAllocation { reference } =
        value.provenance();
    VatDeductibility::new(
        value.numerator(),
        value.denominator(),
        VatDeductibilityProvenance::DocumentedUseAllocation {
            reference: reference.clone(),
        },
    )
}

/// Recompute a sealed VAT-A voucher from its hash-bound treatment and resolved-policy
/// provenance. Unlike the legacy verifier, this never searches rate classes or infers
/// an allocation from rounded cents: the exact treatment, source amount and accounts
/// are the input. A mismatch in postings or in resolved rate/policy provenance fails
/// loudly. Vouchers without VAT-A provenance are outside this verifier.
pub fn verify_vat_expansion(voucher: &VoucherBody) -> Result<VatRecompute, DomainError> {
    verify_vat_expansion_with_role_table(voucher, vat_account_role_table())
}

/// Record-time verification additionally requires the current resolver to reproduce
/// every resolved policy field. Historical verification uses the sealed resolution so
/// later masterdata changes cannot reinterpret an intact voucher.
pub fn verify_vat_expansion_for_seal(voucher: &VoucherBody) -> Result<VatRecompute, DomainError> {
    verify_vat_expansion_for_seal_with_role_table(voucher, vat_account_role_table())
}

/// Record-time verification against the same company-chart roles used by expansion
/// and filing.
pub fn verify_vat_expansion_for_seal_with_role_table(
    voucher: &VoucherBody,
    role_table: &VatAccountRoleTable,
) -> Result<VatRecompute, DomainError> {
    verify_vat_expansion_impl(voucher, role_table, false)
}

/// Exact recomputation against importer-owned chart roles for the voucher's sealed
/// `chart_version`.
pub fn verify_vat_expansion_with_role_table(
    voucher: &VoucherBody,
    role_table: &VatAccountRoleTable,
) -> Result<VatRecompute, DomainError> {
    verify_vat_expansion_impl(voucher, role_table, true)
}

fn verify_vat_expansion_impl(
    voucher: &VoucherBody,
    role_table: &VatAccountRoleTable,
    use_sealed_resolution: bool,
) -> Result<VatRecompute, DomainError> {
    let Some(expansion) = voucher.vat_expansion() else {
        return Ok(VatRecompute::NotApplicable);
    };

    // Resolve and validate every generated marker against the exact sealed chart
    // snapshot. The marker says which posting was generated; chart metadata says what
    // that account means for this company/chart version.
    let roles = role_table
        .resolve(voucher.chart_version())
        .map_err(|error| {
            DomainError::VatStructure(format!("voucher {}: {error}", voucher.number()))
        })?;
    for marker in expansion.generated_vat_legs() {
        let posting = &voucher.postings()[marker.posting_index];
        if roles.generated_role_of(posting.account.as_str()) != Some(marker.role) {
            return Err(DomainError::VatStructure(format!(
                "voucher {} generated VAT marker {} disagrees with chart_version {:?}",
                voucher.number(),
                marker.posting_index,
                voucher.chart_version()
            )));
        }
    }

    let treatment = domain_treatment(expansion.treatment())?;
    let resolved_percent = expansion
        .resolved_percent()
        .map(|value| {
            value.parse::<Decimal>().map_err(|_| {
                DomainError::VatStructure(format!(
                    "voucher {} has an invalid sealed resolved VAT percentage",
                    voucher.number()
                ))
            })
        })
        .transpose()?;
    let recompute_input = VatExpandInput {
        amount: expansion.source_amount().clone(),
        net_account: expansion.net_account().clone(),
        contra_account: expansion.contra_account().clone(),
        treatment: treatment.clone(),
        document_date: voucher.document_date(),
        posting_date: voucher.posting_date(),
        description: voucher.description().to_string(),
        attachment_ref: voucher.attachment_ref(),
    };
    let recomputed = if use_sealed_resolution {
        vat_expand_resolved_with_roles(
            &recompute_input,
            resolved_percent,
            expansion.policy_set().to_string(),
            expansion.vat_rates_hash(),
            Some(roles),
        )?
    } else {
        vat_expand_with_role_table(&recompute_input, voucher.chart_version(), role_table)?
    };

    if recomputed.postings() != voucher.postings() || recomputed.vat_expansion() != Some(expansion)
    {
        return Err(DomainError::VatStructure(format!(
            "voucher {} does not exactly reproduce from its sealed VAT treatment and resolved policy provenance",
            voucher.number()
        )));
    }

    Ok(VatRecompute::Verified { treatment })
}
