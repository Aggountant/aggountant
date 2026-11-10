//! Hash-bound provenance for VAT-A expansions.  This is deliberately Model data:
//! it records a Domain calculation without making Model depend on Domain.

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{AccountId, Hash, ModelError, Money, Posting, Side};

pub const VAT_EXPANSION_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VatExpansionScope {
    EuServices,
    EuGoods,
    NonEuServices,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VatExpansionDeductibilityProvenance {
    DocumentedUseAllocation { reference: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatExpansionDeductibility {
    numerator: u64,
    denominator: u64,
    provenance: VatExpansionDeductibilityProvenance,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDeductibility {
    numerator: u64,
    denominator: u64,
    provenance: VatExpansionDeductibilityProvenance,
}

impl VatExpansionDeductibility {
    pub fn new(
        numerator: u64,
        denominator: u64,
        provenance: VatExpansionDeductibilityProvenance,
    ) -> Result<Self, ModelError> {
        if numerator == 0 || denominator == 0 || numerator >= denominator {
            return Err(invalid("deductibility must be a proper positive fraction"));
        }
        let VatExpansionDeductibilityProvenance::DocumentedUseAllocation { reference } =
            &provenance;
        if reference.is_empty() || reference.trim() != reference {
            return Err(invalid(
                "deductibility reference must be non-empty and trimmed",
            ));
        }
        let d = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / d,
            denominator: denominator / d,
            provenance,
        })
    }
    pub fn numerator(&self) -> u64 {
        self.numerator
    }
    pub fn denominator(&self) -> u64 {
        self.denominator
    }
    pub fn provenance(&self) -> &VatExpansionDeductibilityProvenance {
        &self.provenance
    }
}
impl<'de> Deserialize<'de> for VatExpansionDeductibility {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawDeductibility::deserialize(d)?;
        Self::new(raw.numerator, raw.denominator, raw.provenance).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VatExpansionTreatment {
    NoVat,
    DomesticInput {
        rate_class: String,
    },
    DomesticInputPartial {
        rate_class: String,
        deductibility: VatExpansionDeductibility,
    },
    ReverseCharge {
        rate_class: String,
        scope: VatExpansionScope,
    },
    ReverseChargePartial {
        rate_class: String,
        scope: VatExpansionScope,
        deductibility: VatExpansionDeductibility,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeneratedVatRole {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedVatLeg {
    pub posting_index: usize,
    pub role: GeneratedVatRole,
}

/// Facts frozen with a VAT-A expansion. `resolved_percent` is absent only for NoVat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VatExpansion {
    version: u32,
    treatment: VatExpansionTreatment,
    source_amount: Money,
    net_account: AccountId,
    contra_account: AccountId,
    resolved_percent: Option<String>,
    policy_set: String,
    vat_rates_hash: Hash,
    generated_vat_legs: Vec<GeneratedVatLeg>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVatExpansion {
    version: u32,
    treatment: VatExpansionTreatment,
    source_amount: Money,
    net_account: AccountId,
    contra_account: AccountId,
    resolved_percent: Option<String>,
    policy_set: String,
    vat_rates_hash: Hash,
    generated_vat_legs: Vec<GeneratedVatLeg>,
}

impl VatExpansion {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u32,
        treatment: VatExpansionTreatment,
        source_amount: Money,
        net_account: AccountId,
        contra_account: AccountId,
        resolved_percent: Option<String>,
        policy_set: String,
        vat_rates_hash: Hash,
        generated_vat_legs: Vec<GeneratedVatLeg>,
    ) -> Result<Self, ModelError> {
        if version != VAT_EXPANSION_VERSION {
            return Err(invalid("unsupported VAT expansion version"));
        }
        if source_amount.currency().as_str() != "EUR"
            || source_amount.amount() <= Decimal::ZERO
            || (source_amount.amount() * Decimal::from(100)).fract() != Decimal::ZERO
        {
            return Err(invalid("source amount must be positive exact EUR cents"));
        }
        if policy_set.is_empty() || policy_set.trim() != policy_set {
            return Err(invalid("policy_set must be non-empty and trimmed"));
        }
        let bears_vat = !matches!(treatment, VatExpansionTreatment::NoVat);
        let mut indices = std::collections::BTreeSet::new();
        if generated_vat_legs
            .iter()
            .any(|leg| !indices.insert(leg.posting_index))
        {
            return Err(invalid("generated VAT posting indices must be unique"));
        }
        match (&treatment, &resolved_percent) {
            (VatExpansionTreatment::NoVat, None) => {}
            (VatExpansionTreatment::NoVat, Some(_)) => {
                return Err(invalid("NoVat must not contain a resolved percentage"))
            }
            (_, Some(percent)) => {
                canonical_percent(percent)?;
            }
            (_, None) => {
                return Err(invalid(
                    "VAT-bearing treatment requires resolved percentage",
                ))
            }
        }
        let input_markers = generated_vat_legs
            .iter()
            .filter(|leg| leg.role == GeneratedVatRole::Input)
            .count();
        let output_markers = generated_vat_legs
            .iter()
            .filter(|leg| leg.role == GeneratedVatRole::Output)
            .count();
        if input_markers > 1 || output_markers > 1 {
            return Err(invalid(
                "at most one generated VAT marker is allowed per role",
            ));
        }
        match &treatment {
            VatExpansionTreatment::NoVat => {
                if !generated_vat_legs.is_empty() {
                    return Err(invalid("NoVat must not have generated VAT legs"));
                }
            }
            VatExpansionTreatment::DomesticInput { rate_class }
            | VatExpansionTreatment::DomesticInputPartial { rate_class, .. } => {
                nonempty_trimmed(rate_class, "rate_class")?;
                if output_markers != 0 {
                    return Err(invalid(
                        "domestic treatment must not have an output VAT marker",
                    ));
                }
            }
            VatExpansionTreatment::ReverseCharge { rate_class, .. } => {
                nonempty_trimmed(rate_class, "rate_class")?;
                if !((input_markers == 0 && output_markers == 0)
                    || (input_markers == 1 && output_markers == 1))
                {
                    return Err(invalid(
                        "full reverse-charge treatment requires both VAT markers or neither",
                    ));
                }
            }
            VatExpansionTreatment::ReverseChargePartial { rate_class, .. } => {
                nonempty_trimmed(rate_class, "rate_class")?;
                if input_markers > output_markers {
                    return Err(invalid(
                        "partial reverse-charge treatment must not have an input marker without output",
                    ));
                }
            }
        }
        debug_assert_eq!(bears_vat, resolved_percent.is_some());
        Ok(Self {
            version,
            treatment,
            source_amount,
            net_account,
            contra_account,
            resolved_percent,
            policy_set,
            vat_rates_hash,
            generated_vat_legs,
        })
    }
    pub(crate) fn validate_postings(&self, postings: &[Posting]) -> Result<(), ModelError> {
        if postings.iter().any(|posting| posting.vat_code.is_some()) {
            return Err(invalid(
                "VAT expansion provenance cannot coexist with tagged postings",
            ));
        }
        for leg in &self.generated_vat_legs {
            let Some(posting) = postings.get(leg.posting_index) else {
                return Err(invalid("generated VAT posting index is out of range"));
            };
            if posting.vat_code.is_some() || posting.amount.currency().as_str() != "EUR" {
                return Err(invalid("generated VAT posting must be untagged EUR"));
            }
            let expected = match leg.role {
                GeneratedVatRole::Input => Side::Debit,
                GeneratedVatRole::Output => Side::Credit,
            };
            if posting.side != expected {
                return Err(invalid("generated VAT posting has wrong direction"));
            }
        }
        Ok(())
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn treatment(&self) -> &VatExpansionTreatment {
        &self.treatment
    }
    pub fn source_amount(&self) -> &Money {
        &self.source_amount
    }
    pub fn net_account(&self) -> &AccountId {
        &self.net_account
    }
    pub fn contra_account(&self) -> &AccountId {
        &self.contra_account
    }
    pub fn resolved_percent(&self) -> Option<&str> {
        self.resolved_percent.as_deref()
    }
    pub fn policy_set(&self) -> &str {
        &self.policy_set
    }
    pub fn vat_rates_hash(&self) -> Hash {
        self.vat_rates_hash
    }
    pub fn generated_vat_legs(&self) -> &[GeneratedVatLeg] {
        &self.generated_vat_legs
    }
}
impl<'de> Deserialize<'de> for VatExpansion {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let r = RawVatExpansion::deserialize(d)?;
        Self::new(
            r.version,
            r.treatment,
            r.source_amount,
            r.net_account,
            r.contra_account,
            r.resolved_percent,
            r.policy_set,
            r.vat_rates_hash,
            r.generated_vat_legs,
        )
        .map_err(serde::de::Error::custom)
    }
}

fn invalid(message: &str) -> ModelError {
    ModelError::InvalidVatExpansion(message.into())
}
fn nonempty_trimmed(value: &str, name: &str) -> Result<(), ModelError> {
    if value.is_empty() || value.trim() != value {
        Err(invalid(&format!("{name} must be non-empty and trimmed")))
    } else {
        Ok(())
    }
}
fn canonical_percent(value: &str) -> Result<(), ModelError> {
    nonempty_trimmed(value, "resolved_percent")?;
    let parsed = value
        .parse::<Decimal>()
        .map_err(|_| invalid("resolved_percent must be a decimal"))?;
    if parsed < Decimal::ZERO || parsed.to_string() != value {
        return Err(invalid(
            "resolved_percent must be canonical and nonnegative",
        ));
    }
    Ok(())
}
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
