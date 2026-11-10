use super::*;
use aggountant_model::{
    GeneratedVatLeg, GeneratedVatRole, VatExpansion, VatExpansionDeductibility,
    VatExpansionDeductibilityProvenance as ModelDeductibilityProvenance, VatExpansionScope,
    VatExpansionTreatment, VAT_EXPANSION_VERSION,
};
use rust_decimal::prelude::ToPrimitive;
use sha2::{Digest, Sha256};

/// Deductible **domestic** input VAT (ostovähennys-ALV) — Pattern B, and the input
/// leg of a **non-EU** reverse charge. `1763 Arvonlisäverosaaminen`.
pub(super) const ACCT_INPUT_VAT_DOMESTIC: &str = "1763";
/// Deductible input VAT on an **EU reverse-charge service** acquisition (`1798`) —
/// the 350↔350 pair partner of `2992` in the real Briox ledger (ADR-7 §Context).
pub(super) const ACCT_INPUT_VAT_RC_EU_SERVICES: &str = "1798";
/// Deductible input VAT on an **EU reverse-charge goods** acquisition (`1797`) —
/// the 29↔29 pair partner of `2991`.
pub(super) const ACCT_INPUT_VAT_RC_EU_GOODS: &str = "1797";
/// Self-assessed **output** VAT on an EU reverse-charge *service* acquisition (`2992`).
pub(super) const ACCT_OUTPUT_VAT_RC_EU_SERVICES: &str = "2992";
/// Self-assessed **output** VAT on an EU reverse-charge *goods* acquisition (`2991`).
pub(super) const ACCT_OUTPUT_VAT_RC_EU_GOODS: &str = "2991";
/// Self-assessed **output** VAT on a **non-EU** reverse-charge acquisition — the
/// domestic-output account the real books reuse for the non-EU variant (`2982`,
/// ADR-7 §Context "ei-EU-variantti: ALV-jalat 1763/2982").
pub(super) const ACCT_OUTPUT_VAT_RC_NON_EU: &str = "2982";

/// Which supported reverse-charge treatment a self-assessed acquisition uses.
/// The sealed treatment owns filing semantics; the exact active chart independently
/// selects the account pair that receives the generated legs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReverseChargeScope {
    /// EU service acquisition (AVL 9 §): input `1798`, output `2992`.
    EuServices,
    /// EU goods acquisition (AVL 2 a / 8 a §): input `1797`, output `2991`.
    EuGoods,
    /// A reverse-charge service bought from a business established outside the EU.
    /// Imported goods are a different, unsupported treatment and cannot select this
    /// explicit service scope.
    NonEuServices,
}

impl ReverseChargeScope {
    /// Legacy chartless expander target. Chart-aware production paths resolve the
    /// exact active chart instead.
    pub(super) fn input_account(self) -> &'static str {
        match self {
            ReverseChargeScope::EuServices => ACCT_INPUT_VAT_RC_EU_SERVICES,
            ReverseChargeScope::EuGoods => ACCT_INPUT_VAT_RC_EU_GOODS,
            ReverseChargeScope::NonEuServices => ACCT_INPUT_VAT_DOMESTIC,
        }
    }

    /// Legacy chartless expander target. Chart-aware production paths resolve the
    /// exact active chart instead.
    pub(super) fn output_account(self) -> &'static str {
        match self {
            ReverseChargeScope::EuServices => ACCT_OUTPUT_VAT_RC_EU_SERVICES,
            ReverseChargeScope::EuGoods => ACCT_OUTPUT_VAT_RC_EU_GOODS,
            ReverseChargeScope::NonEuServices => ACCT_OUTPUT_VAT_RC_NON_EU,
        }
    }
}

/// Audit provenance for a caller-declared AVL 117 § deductible fraction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VatDeductibilityProvenance {
    DocumentedUseAllocation { reference: String },
}

/// An exact, typed AVL 117 § deductible fraction and its audit provenance.
///
/// Partial treatments accept only a proper positive fraction (`0 < numerator <
/// denominator`). Full deductibility uses the ordinary treatment variants. Zero
/// deductibility is a separate treatment case not modelled by this partial-share API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatDeductibility {
    numerator: u64,
    denominator: u64,
    provenance: VatDeductibilityProvenance,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVatDeductibility {
    numerator: u64,
    denominator: u64,
    provenance: VatDeductibilityProvenance,
}

impl VatDeductibility {
    pub fn new(
        numerator: u64,
        denominator: u64,
        provenance: VatDeductibilityProvenance,
    ) -> Result<Self, DomainError> {
        if numerator == 0 || denominator == 0 || numerator >= denominator {
            return Err(DomainError::VatCode(format!(
                "AVL 117 § partial deductibility must be an exact proper fraction, got {numerator}/{denominator}"
            )));
        }
        let VatDeductibilityProvenance::DocumentedUseAllocation { reference } = &provenance;
        if reference.trim().is_empty() || reference.trim() != reference {
            return Err(DomainError::VatCode(
                "AVL 117 § provenance reference must be non-empty and trimmed".into(),
            ));
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
            provenance,
        })
    }

    pub fn numerator(&self) -> u64 {
        self.numerator
    }

    pub fn denominator(&self) -> u64 {
        self.denominator
    }

    pub fn provenance(&self) -> &VatDeductibilityProvenance {
        &self.provenance
    }

    /// Apply the exact rational to a full VAT amount already frozen to cents. Integer
    /// rational arithmetic preserves the fraction through the second rounding step.
    fn apply_to(&self, full_vat: Decimal) -> Result<Decimal, DomainError> {
        let full_cents = exact_nonnegative_cents(full_vat)?;
        let product = u128::from(full_cents)
            .checked_mul(u128::from(self.numerator))
            .ok_or_else(|| DomainError::VatCode("deductibility calculation overflow".into()))?;
        let denominator = u128::from(self.denominator);
        let quotient = product / denominator;
        let remainder = product % denominator;
        let rounded = if remainder * 2 >= denominator {
            quotient + 1
        } else {
            quotient
        };
        let cents = u64::try_from(rounded)
            .map_err(|_| DomainError::VatCode("deductible VAT exceeds supported range".into()))?;
        let mut amount = Decimal::from(cents) / Decimal::from(100);
        amount.rescale(2);
        Ok(amount)
    }
}

impl<'de> Deserialize<'de> for VatDeductibility {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawVatDeductibility::deserialize(deserializer)?;
        Self::new(raw.numerator, raw.denominator, raw.provenance).map_err(serde::de::Error::custom)
    }
}

fn exact_nonnegative_cents(value: Decimal) -> Result<u64, DomainError> {
    let scaled = value * Decimal::from(100);
    if scaled < Decimal::ZERO || scaled.fract() != Decimal::ZERO {
        return Err(DomainError::VatStructure(format!(
            "VAT amount {value} is not an exact non-negative cent amount"
        )));
    }
    scaled.to_u64().ok_or_else(|| {
        DomainError::VatStructure(format!(
            "VAT amount {value} exceeds the supported cent range"
        ))
    })
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The VAT treatment that drives [`vat_expand`]: which of ADR-7's three patterns a
/// purchase is, and — for the ones that bear VAT — the `rate_class` whose dated
/// percentage the `document_date` selects. Partial variants carry the exact AVL 117 §
/// fraction and its provenance as typed data, never as a `-deductNN` rate-class code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum VatTreatment {
    /// Pattern C — no VAT. The amount is booked in full to the net account; no VAT leg.
    NoVat,
    /// Pattern B — a domestic purchase with deductible input VAT. The intake amount
    /// is **VAT-inclusive** (the gross cash paid): the VAT is split out to `1763`
    /// (`round(gross × rate / (100 + rate))`) and the net remainder to the net account.
    DomesticInput { rate_class: String },
    /// Pattern B with an AVL 117 § partially deductible input-VAT share.
    DomesticInputPartial {
        rate_class: String,
        deductibility: VatDeductibility,
    },
    /// Pattern A — a reverse-charge acquisition (self-assessed VAT, AVL 8 a–c / 9 §).
    /// The intake amount is the **net** (no VAT rides in the cash): the VAT is
    /// self-assessed both ways — output and deductible input VAT go to the exact
    /// active chart's selected targets — so it nets to zero in cash.
    ReverseCharge {
        rate_class: String,
        scope: ReverseChargeScope,
    },
    /// Pattern A with full self-assessed output VAT but only an AVL 117 § fraction
    /// deductible as input VAT.
    ReverseChargePartial {
        rate_class: String,
        scope: ReverseChargeScope,
        deductibility: VatDeductibility,
    },
}

/// The input to [`vat_expand`]: one intake purchase to expand into a sealed voucher's
/// worth of postings. Mirrors ADR-7 §1's `(gross, VatCode, net account, document_date,
/// PolicySet)`, with the contra (cash) account and the draft's descriptive fields
/// added because the output is a complete, balanced [`VoucherDraft`]. The pinned
/// `PolicySet` is the bundled dated rate table (the S1 policy stub), same as every
/// other figure in this module — no per-call policy handle is threaded (that follows
/// the `derive_vat_figures` convention).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VatExpandInput {
    /// The intake cash amount. For [`VatTreatment::DomesticInput`] this is the
    /// **gross** (VAT-inclusive) amount; for [`VatTreatment::ReverseCharge`] and
    /// [`VatTreatment::NoVat`] it is the **net** amount (no VAT rides in the cash).
    pub amount: Money,
    /// The expense/asset account the net leg posts to (ADR-7 §1's "net account").
    pub net_account: AccountId,
    /// The account the cash leg posts to (the bank / payable), squaring the entry.
    pub contra_account: AccountId,
    pub treatment: VatTreatment,
    /// Selects the rate (with the treatment's `rate_class`) and freezes the
    /// *historical* percentage into the sealed legs.
    pub document_date: Date,
    pub posting_date: Date,
    pub description: String,
    pub attachment_ref: Option<Hash>,
}

/// One expanded posting.
pub(super) fn leg(
    account: AccountId,
    amount: Decimal,
    eur: &Currency,
    side: Side,
    vat_code: Option<VatCode>,
) -> Posting {
    Posting {
        account,
        amount: Money::new(amount, eur.clone()),
        side,
        vat_code,
    }
}

// Why the expanded legs carry **no** `vat_code` (they are all untagged): under
// VAT-A the authoritative VAT figure is the euro on the provenance-marked generated
// leg, while the sealed treatment owns its return category. The active chart only
// selects the destination accounts. Stamping the net leg with a
// `purchase-<rate_class>` code was
// considered and rejected: it does **not** distinguish the treatments (domestic vs.
// the three reverse-charge scopes all resolve the same rate class) and it silently
// disagrees with the legacy [`derive_vat_figures`] base×rate path — that path treats
// a tagged amount as the VAT *base*, but `gross → round(gross × r/(100+r))` and
// `net → round(net × r/100)` are **not** inverse after cent rounding (e.g. gross
// `0.12` at 25,5 % seals VAT `0.02` yet a net-base re-derivation yields `0.03`), and
// for a reverse charge it would see the input side only and drop the self-assessed
// output. Leaving the legs untagged keeps `derive_vat_figures` (which only sums
// tagged postings) cleanly blind to VAT-A vouchers rather than half-reading them;
// the authoritative filing sums only hash-bound generated VAT-leg markers (ADR-7 §3).
// The recompute cross-check (ADR-7 §2) re-runs `vat_expand` from the exact sealed
// treatment, source amount, accounts and `document_date`; it never infers treatment
// from an account pair or rounded cents.

/// Expand one intake purchase into the complete, balanced `VoucherDraft` ADR-7 (VAT-A)
/// seals — the net leg, the deductible input-VAT leg, and (for a reverse charge) the
/// self-assessed output-VAT leg — with the rate resolved from the treatment's
/// `rate_class` + `document_date`. Pure and deterministic: re-running it with the same
/// inputs yields byte-identical legs (the ADR-7 §2 recompute cross-check).
///
/// The draft is assembled through the one [`crate::build_draft`] builder, so the same
/// `Σ = 0` / `≥ 2`-posting / structural guards apply to an expanded voucher as to any
/// hand-entered one. A non-EUR, non-positive, or non-cent (scale > 2) amount, or a
/// `rate_class`/`document_date` with no dated rate edition, fails loudly rather than
/// sealing a wrong image — the intake amount is source money and is never silently
/// re-scaled.
pub fn vat_expand(input: &VatExpandInput) -> Result<VoucherDraft, DomainError> {
    let percent = resolved_treatment_percent(input)?;
    vat_expand_resolved(
        input,
        percent,
        crate::PolicySet::default().id,
        Hash::from_bytes(Sha256::digest(VAT_RATES_JSON.as_bytes()).into()),
    )
}

/// Expand against one exact active chart's VAT roles. Unlike [`vat_expand`], this
/// entry point has no account-number fallback: missing or inactive metadata and a
/// missing treatment target fail closed.
pub fn vat_expand_with_role_table(
    input: &VatExpandInput,
    chart_version: &str,
    role_table: &VatAccountRoleTable,
) -> Result<VoucherDraft, DomainError> {
    let roles = role_table.resolve(chart_version)?;
    let percent = resolved_treatment_percent(input)?;
    vat_expand_resolved_with_roles(
        input,
        percent,
        crate::PolicySet::default().id,
        Hash::from_bytes(Sha256::digest(VAT_RATES_JSON.as_bytes()).into()),
        Some(roles),
    )
}

fn resolved_treatment_percent(input: &VatExpandInput) -> Result<Option<Decimal>, DomainError> {
    match &input.treatment {
        VatTreatment::NoVat => Ok(None),
        VatTreatment::ReverseCharge {
            rate_class,
            scope: ReverseChargeScope::NonEuServices,
        }
        | VatTreatment::ReverseChargePartial {
            rate_class,
            scope: ReverseChargeScope::NonEuServices,
            ..
        } => {
            // Filing can place non-EU service tax only in the three generic domestic
            // rate slots. Refuse any other class before a draft can be sealed.
            vat_rate_class_field(rate_class)?;
            Ok(Some(resolve_vat_rate(rate_class, input.document_date)?))
        }
        VatTreatment::DomesticInput { rate_class }
        | VatTreatment::DomesticInputPartial { rate_class, .. }
        | VatTreatment::ReverseCharge { rate_class, .. }
        | VatTreatment::ReverseChargePartial { rate_class, .. } => {
            Ok(Some(resolve_vat_rate(rate_class, input.document_date)?))
        }
    }
}

/// Re-run the expansion arithmetic from hash-bound resolved provenance. Historical
/// verification must not re-resolve an ambient rate table and thereby reinterpret an
/// intact sealed voucher after policy masterdata changes.
pub(super) fn vat_expand_resolved(
    input: &VatExpandInput,
    resolved_percent: Option<Decimal>,
    policy_set: String,
    vat_rates_hash: Hash,
) -> Result<VoucherDraft, DomainError> {
    vat_expand_resolved_with_roles(input, resolved_percent, policy_set, vat_rates_hash, None)
}

pub(super) fn vat_expand_resolved_with_roles(
    input: &VatExpandInput,
    resolved_percent: Option<Decimal>,
    policy_set: String,
    vat_rates_hash: Hash,
    roles: Option<&ChartVatRoles>,
) -> Result<VoucherDraft, DomainError> {
    let eur = require_eur(&[&input.amount])?;
    if input.amount.amount() <= Decimal::ZERO {
        return Err(DomainError::VatCurrency(format!(
            "vat_expand amount must be > 0, got {}",
            input.amount.amount_str()
        )));
    }
    // Reject sub-cent source money rather than silently rescaling it (`eur_2dp`
    // would alter the amount, breaking reconciliation with the bank statement /
    // attachment). A EUR amount is exact cents by construction.
    if input.amount.amount() != round_eur(input.amount.amount()) {
        return Err(DomainError::VatCurrency(format!(
            "vat_expand amount must be an exact EUR-cent amount, got {}",
            input.amount.amount_str()
        )));
    }
    // The net and contra accounts must not themselves be VAT-role accounts: a VAT
    // account used as the expense/cash leg would collide with the expanded VAT legs —
    // the account-sum projection would over-count it as VAT, and the recompute verifier
    // would mis-reconstruct the treatment from an ambiguous pair. Refuse it before the
    // seal freezes the ambiguity (ADR-7: the VAT accounts carry VAT movements only).
    for (label, account) in [
        ("net", &input.net_account),
        ("contra", &input.contra_account),
    ] {
        let is_vat_account = roles
            .map(|chart| chart.role_of(account.as_str()).is_some())
            .unwrap_or_else(|| is_default_chart_vat_account(account.as_str()));
        if is_vat_account {
            return Err(DomainError::VatStructure(format!(
                "vat_expand {label} account {account} is a VAT-role account — VAT-role accounts carry VAT legs only, not the net/contra of the entry"
            )));
        }
    }

    let amount = eur_2dp(input.amount.amount());
    let mut postings = Vec::new();
    let mut generated_vat_legs = Vec::new();

    match &input.treatment {
        VatTreatment::NoVat => {
            // Pattern C: the whole amount is the net expense; no VAT account moves.
            postings.push(leg(
                input.net_account.clone(),
                amount,
                &eur,
                Side::Debit,
                None,
            ));
            postings.push(leg(
                input.contra_account.clone(),
                amount,
                &eur,
                Side::Credit,
                None,
            ));
        }
        VatTreatment::DomesticInput { .. } => {
            expand_domestic_input(
                &mut postings,
                &mut generated_vat_legs,
                input,
                &eur,
                amount,
                resolved_percent.expect("VAT-bearing treatment has a resolved rate"),
                None,
                domestic_input_account(roles)?,
            )?;
        }
        VatTreatment::DomesticInputPartial { deductibility, .. } => {
            expand_domestic_input(
                &mut postings,
                &mut generated_vat_legs,
                input,
                &eur,
                amount,
                resolved_percent.expect("VAT-bearing treatment has a resolved rate"),
                Some(deductibility),
                domestic_input_account(roles)?,
            )?;
        }
        VatTreatment::ReverseCharge { scope, .. } => {
            expand_reverse_charge(
                &mut postings,
                &mut generated_vat_legs,
                input,
                &eur,
                amount,
                resolved_percent.expect("VAT-bearing treatment has a resolved rate"),
                None,
                reverse_charge_accounts(roles, *scope)?,
            )?;
        }
        VatTreatment::ReverseChargePartial {
            scope,
            deductibility,
            ..
        } => {
            expand_reverse_charge(
                &mut postings,
                &mut generated_vat_legs,
                input,
                &eur,
                amount,
                resolved_percent.expect("VAT-bearing treatment has a resolved rate"),
                Some(deductibility),
                reverse_charge_accounts(roles, *scope)?,
            )?;
        }
    }

    let expansion = VatExpansion::new(
        VAT_EXPANSION_VERSION,
        provenance_treatment(&input.treatment)?,
        input.amount.clone(),
        input.net_account.clone(),
        input.contra_account.clone(),
        resolved_percent.map(|percent| percent.to_string()),
        policy_set,
        vat_rates_hash,
        generated_vat_legs,
    )?;
    crate::build_draft(crate::DraftInput {
        document_date: input.document_date,
        posting_date: input.posting_date,
        description: input.description.clone(),
        postings,
        attachment_ref: input.attachment_ref,
        buyer_vat_id: None,
    })?
    .with_vat_expansion(expansion)
    .map_err(Into::into)
}

fn domestic_input_account(roles: Option<&ChartVatRoles>) -> Result<&str, DomainError> {
    match roles {
        Some(chart) => chart.domestic_input_target().ok_or_else(|| {
            DomainError::VatStructure(format!(
                "chart_version {:?} has no unambiguous domestic input-VAT role target",
                chart.chart_version()
            ))
        }),
        None => Ok(ACCT_INPUT_VAT_DOMESTIC),
    }
}

struct ReverseChargeAccounts<'a> {
    input: &'a str,
    output: &'a str,
}

fn reverse_charge_accounts(
    roles: Option<&ChartVatRoles>,
    scope: ReverseChargeScope,
) -> Result<ReverseChargeAccounts<'_>, DomainError> {
    let (input, output) = match roles {
        Some(chart) => chart.reverse_charge_target(scope).ok_or_else(|| {
            DomainError::VatStructure(format!(
                "chart_version {:?} has no reverse-charge role target for {scope:?}",
                chart.chart_version()
            ))
        })?,
        None => (scope.input_account(), scope.output_account()),
    };
    Ok(ReverseChargeAccounts { input, output })
}

#[allow(clippy::too_many_arguments)]
fn expand_domestic_input(
    postings: &mut Vec<Posting>,
    generated_vat_legs: &mut Vec<GeneratedVatLeg>,
    input: &VatExpandInput,
    eur: &Currency,
    gross: Decimal,
    pct: Decimal,
    deductibility: Option<&VatDeductibility>,
    input_account: &str,
) -> Result<(), DomainError> {
    // AVL 117 § requires two distinct rounding steps: first freeze the full invoice
    // VAT, then round the deductible share of that concrete VAT euro amount.
    let full_vat = round_eur(gross * pct / (Decimal::from(100) + pct));
    let deductible_vat = match deductibility {
        Some(share) => share.apply_to(full_vat)?,
        None => full_vat,
    };
    let non_deductible = eur_2dp(full_vat - deductible_vat);
    let invoice_net = eur_2dp(gross - full_vat);
    // The non-deductible remainder belongs in cost. Keeping it on the same net
    // account gives expense + deductible VAT == gross exactly.
    let booked_cost = eur_2dp(invoice_net + non_deductible);
    postings.push(leg(
        input.net_account.clone(),
        booked_cost,
        eur,
        Side::Debit,
        None,
    ));
    if deductible_vat > Decimal::ZERO {
        let posting_index = postings.len();
        postings.push(leg(
            AccountId::new(input_account)?,
            deductible_vat,
            eur,
            Side::Debit,
            None,
        ));
        generated_vat_legs.push(GeneratedVatLeg {
            posting_index,
            role: GeneratedVatRole::Input,
        });
    }
    postings.push(leg(
        input.contra_account.clone(),
        gross,
        eur,
        Side::Credit,
        None,
    ));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn expand_reverse_charge(
    postings: &mut Vec<Posting>,
    generated_vat_legs: &mut Vec<GeneratedVatLeg>,
    input: &VatExpandInput,
    eur: &Currency,
    source_net: Decimal,
    pct: Decimal,
    deductibility: Option<&VatDeductibility>,
    accounts: ReverseChargeAccounts<'_>,
) -> Result<(), DomainError> {
    let full_vat = round_eur(source_net * pct / Decimal::from(100));
    let deductible_vat = match deductibility {
        Some(share) => share.apply_to(full_vat)?,
        None => full_vat,
    };
    let non_deductible = eur_2dp(full_vat - deductible_vat);
    postings.push(leg(
        input.net_account.clone(),
        eur_2dp(source_net + non_deductible),
        eur,
        Side::Debit,
        None,
    ));
    postings.push(leg(
        input.contra_account.clone(),
        source_net,
        eur,
        Side::Credit,
        None,
    ));
    if deductible_vat > Decimal::ZERO {
        let posting_index = postings.len();
        postings.push(leg(
            AccountId::new(accounts.input)?,
            deductible_vat,
            eur,
            Side::Debit,
            None,
        ));
        generated_vat_legs.push(GeneratedVatLeg {
            posting_index,
            role: GeneratedVatRole::Input,
        });
    }
    if full_vat > Decimal::ZERO {
        let posting_index = postings.len();
        postings.push(leg(
            AccountId::new(accounts.output)?,
            full_vat,
            eur,
            Side::Credit,
            None,
        ));
        generated_vat_legs.push(GeneratedVatLeg {
            posting_index,
            role: GeneratedVatRole::Output,
        });
    }
    Ok(())
}

fn provenance_treatment(treatment: &VatTreatment) -> Result<VatExpansionTreatment, DomainError> {
    Ok(match treatment {
        VatTreatment::NoVat => VatExpansionTreatment::NoVat,
        VatTreatment::DomesticInput { rate_class } => VatExpansionTreatment::DomesticInput {
            rate_class: rate_class.clone(),
        },
        VatTreatment::DomesticInputPartial {
            rate_class,
            deductibility,
        } => VatExpansionTreatment::DomesticInputPartial {
            rate_class: rate_class.clone(),
            deductibility: provenance_deductibility(deductibility)?,
        },
        VatTreatment::ReverseCharge { rate_class, scope } => VatExpansionTreatment::ReverseCharge {
            rate_class: rate_class.clone(),
            scope: provenance_scope(*scope),
        },
        VatTreatment::ReverseChargePartial {
            rate_class,
            scope,
            deductibility,
        } => VatExpansionTreatment::ReverseChargePartial {
            rate_class: rate_class.clone(),
            scope: provenance_scope(*scope),
            deductibility: provenance_deductibility(deductibility)?,
        },
    })
}
fn provenance_scope(scope: ReverseChargeScope) -> VatExpansionScope {
    match scope {
        ReverseChargeScope::EuServices => VatExpansionScope::EuServices,
        ReverseChargeScope::EuGoods => VatExpansionScope::EuGoods,
        ReverseChargeScope::NonEuServices => VatExpansionScope::NonEuServices,
    }
}
fn provenance_deductibility(
    value: &VatDeductibility,
) -> Result<VatExpansionDeductibility, DomainError> {
    let VatDeductibilityProvenance::DocumentedUseAllocation { reference } = value.provenance();
    Ok(VatExpansionDeductibility::new(
        value.numerator(),
        value.denominator(),
        ModelDeductibilityProvenance::DocumentedUseAllocation {
            reference: reference.clone(),
        },
    )?)
}

// ---------------------------------------------------------------------------
// ADR-7 (VAT-A) §3 — the account-sum ALV filing projection
// ---------------------------------------------------------------------------
//
// [`vat_expand`] seals the VAT legs as **untagged** postings on the VAT accounts, so
// the legacy [`derive_vat_figures`] — which reads `base × rate` off `vat_code`-tagged
// postings only — is blind to them and reports 0,00 for a reverse-charge or
// domestic-input-VAT period. ADR-7 §3 closes that: the filing is a **projection that
// sums the real euros sitting on the VAT accounts**, keyed by each account's role,
// rather than a re-derivation from a code + rate. The authoritative figure is the
// euro physically on the sealed account (ADR-7 §Consequences), and this is where the
// filing reads it.
//
// The role metadata is chart-bound and read from **reviewable masterdata**, not a
// hard-coded match arm (F25, `vat-account-role-chart-metadata`): `data/vat-account-
// roles.json` maps, per chart, each VAT account to its role. Resolution is keyed by
// the voucher's `chart_version`, so a chart the masterdata does not recognise is
// **refused loudly** (Guard A) rather than silently run through the default table and
// filed 0,00 — the worst business outcome for a filing. The default chart's role tags
// mirror the constants [`vat_expand`] posts to (a load-time / unit-test invariant), so
// expansion and the account-sum projection still agree by construction.
//
// **Chart-adoption boundary (do not pre-empt).** The full company-chart fix — how a
// company's own per-year chart (and its own VAT account numbers) enters masterdata —
// is the open `chart-adoption` ADR (SIE4 import vs init regeneration, undecided). This
// slice ships the role-tag *mechanism* + the loud-failure guards for the pinned
// default chart `fi-general-2026`; it does **not** add the reverse-charge accounts to
// the chart-of-accounts file, and its role table is keyed by chart *name* so a company
// chart is refused (Guard A) until `chart-adoption` lands its roles.
