//! The **eligibility engine** (`assess_purchase`, F4 extension) — the concrete site of
//! the S5 **LLM/law boundary** (`s5-spec-ahead.md` §3, `candidate.md §5 A2`).
//!
//! > *"Domain doesn't guess, the agent guesses, the law lives in code."*
//!
//! The **agent** interprets the real-world facts and *proposes* an [`ExpenseNature`]
//! classification (a passenger car? an apportionable telecom bill? an EU reverse-charge
//! service?). The **Domain** — this function — computes every legally-determined
//! consequence deterministically from that classification: the deductible VAT, the share
//! the law actually admits, and whether the user's deductibility **claim** is honored.
//! A user's claim is an *input*, never the answer: for a passenger car in mixed use the
//! law clamps the deduction to **zero** regardless of a 70% claim; for an apportionable
//! expense the same law *admits* a reasonable business share. The boundary is not "always
//! distrust the user" — it is "the law decides whether the claim is admissible."

use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use aggountant_model::{Currency, Date, Money};

use crate::DomainError;

/// The **agent's** interpretation of what an expense *is* — a proposal, confirmed at a
/// human gate. The Domain takes it as given and computes the lawful consequence; it does
/// not itself guess the nature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpenseNature {
    /// A passenger car (henkilöauto) with **any** private use. Finnish VAT law denies
    /// deduction on the vehicle's VAT → deductible is **0**, whatever the claim.
    PassengerCarMixedUse,
    /// An expense class where the law admits a **reasonable business share** (e.g. a
    /// telecom bill used partly privately). The claim is honored within `[0, 1]`.
    Apportionable,
    /// An ordinary, wholly-business purchase — fully deductible.
    FullyDeductible,
    /// An EU **reverse-charge** service (e.g. a foreign digital subscription). The buyer
    /// self-assesses output VAT and deducts the corresponding input VAT per law.
    ReverseChargeEuService,
}

/// The input to [`assess_purchase`]: the agent's classification + the receipt's VAT
/// amount + the **user's claim** (a business-use share) + the document date (for a
/// dated-law hook, unused in this slice but recorded so the signature is stable).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseAssessmentInput {
    pub nature: ExpenseNature,
    /// The VAT amount stated on the receipt (non-negative EUR).
    pub receipt_vat: Money,
    /// The user's claimed business-use share as a canonical decimal string, e.g.
    /// `"0.70"` (ADR-2: decimals cross the wire as strings). A **claim**, not the answer.
    pub claimed_business_share: String,
    pub document_date: Date,
}

/// The **Domain's** lawful output. `deductible_vat` is a Domain result, never an agent
/// input; `claim_honored` records whether the law admitted the user's claim as stated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseAssessment {
    pub nature: ExpenseNature,
    pub receipt_vat: Money,
    /// What the user claimed (echoed as provenance — recorded, not necessarily applied).
    pub claimed_business_share: String,
    /// The share the **law** admitted (may differ from the claim).
    pub applied_share: String,
    /// The lawful deductible VAT = `applied_share × receipt_vat`.
    pub deductible_vat: Money,
    /// `true` iff the law applied the user's claim exactly as stated.
    pub claim_honored: bool,
    /// `true` for an EU reverse-charge purchase (self-assessed output + input VAT).
    pub reverse_charge: bool,
    /// A short, human-readable reason (for the approval gate + provenance).
    pub rationale: String,
}

fn eur() -> Currency {
    Currency::new("EUR").expect("EUR is a valid currency code")
}

fn eur_2dp(mut d: Decimal) -> Decimal {
    d.rescale(2);
    d
}

/// Parse the claimed business share under the **canonical** decimal grammar (ADR-2),
/// not `Decimal::from_str` (which would accept `"+0.5"`, `"1e0"`, leading zeros — forms
/// the Model layer forbids). We reuse the Model's own round-trip rule: the value must
/// re-render byte-identically to its canonical `Display` form.
fn parse_canonical_share(s: &str) -> Result<Decimal, DomainError> {
    let bad = || DomainError::VatCurrency(format!("claimed_business_share {s:?} is not canonical"));
    let value = Decimal::from_str(s).map_err(|_| bad())?;
    if value.to_string() != s {
        return Err(bad());
    }
    Ok(value)
}

/// Compute the lawful VAT deductibility for a purchase. Deterministic and total over
/// valid input. The user's `claimed_business_share` is an input; the returned
/// `deductible_vat` is the law's answer.
pub fn assess_purchase(input: &PurchaseAssessmentInput) -> Result<PurchaseAssessment, DomainError> {
    if input.receipt_vat.currency() != &eur() {
        return Err(DomainError::VatCurrency(format!(
            "eligibility is EUR-only in this slice, got {}",
            input.receipt_vat.currency()
        )));
    }
    if input.receipt_vat.amount() < Decimal::ZERO {
        return Err(DomainError::VatCurrency(
            "receipt_vat must be a non-negative total".into(),
        ));
    }

    let vat = input.receipt_vat.amount();
    let claim = parse_canonical_share(&input.claimed_business_share)?;

    // The law per class. `applied_share` is what the law admits; `claim_honored` says
    // whether that equals the claim as stated.
    let (applied_share, claim_honored, reverse_charge, rationale): (Decimal, bool, bool, String) =
        match input.nature {
            // Example 1 — the canonical boundary case. A passenger car with any private
            // use → the deduction is denied entirely, whatever the claim.
            ExpenseNature::PassengerCarMixedUse => (
                Decimal::ZERO,
                claim == Decimal::ZERO,
                false,
                "passenger car in private use — VAT deduction denied by law (claim not applied)"
                    .to_string(),
            ),
            // Example 2 — a genuinely apportionable class. The claim is admitted within
            // the legal frame `[0, 1]`; a claim outside that is clamped (not honored).
            ExpenseNature::Apportionable => {
                let clamped = claim.clamp(Decimal::ZERO, Decimal::ONE);
                let honored = clamped == claim;
                let why = if honored {
                    "apportionable expense — business-use claim admitted within the legal frame"
                        .to_string()
                } else {
                    format!(
                        "apportionable expense — claim {claim} outside [0,1], clamped to {clamped}"
                    )
                };
                (clamped, honored, false, why)
            }
            // A wholly-business purchase: fully deductible. A claim below 1.0 under a
            // "fully deductible" classification is an interpretation mismatch — the law
            // for this class is 100%, so the claim is not what applies.
            ExpenseNature::FullyDeductible => (
                Decimal::ONE,
                claim == Decimal::ONE,
                false,
                "wholly-business purchase — fully deductible".to_string(),
            ),
            // Example 3 — EU reverse-charge. The buyer self-assesses output VAT and
            // deducts the input VAT per law; for a wholly-business service that is 100%.
            ExpenseNature::ReverseChargeEuService => (
                Decimal::ONE,
                claim == Decimal::ONE,
                true,
                "EU reverse-charge service — self-assessed output VAT + full input-VAT \
                 deduction per law"
                    .to_string(),
            ),
        };

    let deductible = eur_2dp(applied_share * vat);
    Ok(PurchaseAssessment {
        nature: input.nature,
        receipt_vat: input.receipt_vat.clone(),
        claimed_business_share: claim.to_string(),
        applied_share: applied_share.to_string(),
        deductible_vat: Money::new(deductible, eur()),
        claim_honored,
        reverse_charge,
        rationale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eur_money(s: &str) -> Money {
        Money::parse(s, eur()).unwrap()
    }
    fn date() -> Date {
        Date::parse("2026-05-01").unwrap()
    }

    #[test]
    fn example_1_passenger_car_claim_of_70pct_is_not_honored_deductible_is_zero() {
        // The load-bearing vector: the claim is 70% but the law clamps to 0.
        let a = assess_purchase(&PurchaseAssessmentInput {
            nature: ExpenseNature::PassengerCarMixedUse,
            receipt_vat: eur_money("240.00"),
            claimed_business_share: "0.70".to_string(),
            document_date: date(),
        })
        .unwrap();
        assert_eq!(a.deductible_vat.amount_str(), "0.00");
        assert_eq!(a.applied_share, "0");
        assert!(!a.claim_honored);
        // The claim is *recorded* as provenance, not applied.
        assert_eq!(a.claimed_business_share, "0.70");
    }

    #[test]
    fn example_2_apportionable_80pct_claim_is_honored() {
        let a = assess_purchase(&PurchaseAssessmentInput {
            nature: ExpenseNature::Apportionable,
            receipt_vat: eur_money("100.00"),
            claimed_business_share: "0.80".to_string(),
            document_date: date(),
        })
        .unwrap();
        assert_eq!(a.deductible_vat.amount_str(), "80.00");
        assert_eq!(a.applied_share, "0.80");
        assert!(a.claim_honored);
    }

    #[test]
    fn apportionable_claim_above_one_is_clamped_and_not_honored() {
        let a = assess_purchase(&PurchaseAssessmentInput {
            nature: ExpenseNature::Apportionable,
            receipt_vat: eur_money("100.00"),
            claimed_business_share: "1.50".to_string(),
            document_date: date(),
        })
        .unwrap();
        assert_eq!(a.deductible_vat.amount_str(), "100.00");
        assert!(!a.claim_honored);
    }

    #[test]
    fn example_3_reverse_charge_is_computed_deterministically() {
        let a = assess_purchase(&PurchaseAssessmentInput {
            nature: ExpenseNature::ReverseChargeEuService,
            receipt_vat: eur_money("12.40"),
            claimed_business_share: "1".to_string(),
            document_date: date(),
        })
        .unwrap();
        assert!(a.reverse_charge);
        assert_eq!(a.deductible_vat.amount_str(), "12.40");
        assert!(a.claim_honored);
    }

    #[test]
    fn non_canonical_claim_is_rejected() {
        // The Model layer forbids these; the eligibility engine must too (ADR-2).
        for claim in ["+0.5", "1e0", "01.0", ".5", "0.5 "] {
            let err = assess_purchase(&PurchaseAssessmentInput {
                nature: ExpenseNature::Apportionable,
                receipt_vat: eur_money("100.00"),
                claimed_business_share: claim.to_string(),
                document_date: date(),
            })
            .unwrap_err();
            assert_eq!(
                err.error_code(),
                "VAT_CURRENCY",
                "claim {claim:?} slipped through"
            );
        }
    }

    #[test]
    fn claim_honored_is_scale_insensitive() {
        // "1.00" is logically 1.0 → honored for a fully-deductible purchase (rust_decimal
        // `==` is scale-insensitive; this locks that behavior against a regression).
        let a = assess_purchase(&PurchaseAssessmentInput {
            nature: ExpenseNature::FullyDeductible,
            receipt_vat: eur_money("100.00"),
            claimed_business_share: "1.00".to_string(),
            document_date: date(),
        })
        .unwrap();
        assert!(a.claim_honored);
        assert_eq!(a.deductible_vat.amount_str(), "100.00");
    }

    #[test]
    fn non_eur_receipt_vat_is_rejected() {
        let usd = Currency::new("USD").unwrap();
        let err = assess_purchase(&PurchaseAssessmentInput {
            nature: ExpenseNature::FullyDeductible,
            receipt_vat: Money::parse("10.00", usd).unwrap(),
            claimed_business_share: "1".to_string(),
            document_date: date(),
        })
        .unwrap_err();
        assert_eq!(err.error_code(), "VAT_CURRENCY");
    }
}
