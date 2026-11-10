//! Structural guards for the two S5 type-level boundaries (`s5-spec-ahead.md` §2.1/§2.2,
//! `candidate.md §4`):
//!
//! 1. **The disclosure leak barrier.** `ConsultQuestion` is built only from
//!    `DisclosureSafe` values; the PII newtypes (`Iban`, `FinnishPersonId`) do **not**
//!    implement `DisclosureSafe`, so a raw IBAN/HETU in a disclosure is a *compile error*.
//!    We can't assert non-compilation from a normal test without `trybuild`, so we prove
//!    the barrier two ways: (a) a *positive* test that a projection only ever contains the
//!    allowlisted keys, and (b) a *structural* source check that `consult.rs` grants no PII
//!    type an `impl DisclosureSafe`.
//! 2. **`Advice` never reaches the ledger.** A structural source check that no
//!    `From<Advice>` / `Advice → VoucherDraft` conversion exists anywhere in the Domain,
//!    plus a type-shape check that `Advice` carries no accounting field.

use std::fs;

use aggountant_domain::{Advice, ConsultQuestion};

fn read_src(name: &str) -> String {
    let path = format!("{}/src/{}", env!("CARGO_MANIFEST_DIR"), name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn disclosure_projection_only_carries_allowlisted_facts() {
    // A default-deny projection: only the facts explicitly projected in are present.
    let q = ConsultQuestion::new("vat-deductibility", "Is this deductible?")
        .with_fact("expense_nature", "passenger-car leasing")
        .with_fact("amount_band", "1000-5000 EUR");
    let body = q.to_body();
    let obj = body.as_object().unwrap();
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["facts", "question", "topic"]);
    assert_eq!(q.fact_keys(), vec!["expense_nature", "amount_band"]);
    // The serialized body carries no PII newtype token — there is no path to place one.
    let serialized = serde_json::to_string(&body).unwrap();
    assert!(
        !serialized.contains("iban"),
        "projection must not carry an IBAN field"
    );
    assert!(
        !serialized.contains("hetu"),
        "projection must not carry a HETU field"
    );
}

#[test]
fn consult_source_grants_no_pii_type_disclosure_safe() {
    // The leak barrier: `DisclosureSafe` (the fact-value bound) is never implemented for a
    // PII newtype. If a future edit adds one, this fails loudly.
    let src = read_src("consult.rs");
    assert!(
        !src.contains("impl DisclosureSafe for Iban"),
        "Iban must never be DisclosureSafe"
    );
    assert!(
        !src.contains("impl DisclosureSafe for FinnishPersonId"),
        "FinnishPersonId must never be DisclosureSafe"
    );
    assert!(
        !src.contains("impl sealed::Sealed for Iban")
            && !src.contains("impl sealed::Sealed for FinnishPersonId"),
        "the sealed trait must not admit a PII newtype"
    );
}

#[test]
fn advice_has_no_ledger_conversion() {
    // `Advice` never reaches the ledger: no From<Advice>, no Advice→voucher/posting path,
    // anywhere in the Domain.
    for file in [
        "consult.rs",
        "lib.rs",
        "eligibility.rs",
        "vat.rs",
        "payroll.rs",
    ] {
        // Ignore comment lines (the doc-comment on `Advice` deliberately *names* the
        // absent conversion); assert no real `impl` introduces one.
        for line in read_src(file).lines() {
            let code = line.trim_start();
            if code.starts_with("//") || code.starts_with("*") {
                continue;
            }
            assert!(
                !code.contains("impl From<Advice>"),
                "{file}: Advice must not convert into any ledger type ({line:?})"
            );
        }
    }
}

#[test]
fn advice_carries_no_accounting_field() {
    // A type-shape guard: Advice's JSON is only summary + citations — no Money, account,
    // posting, or VAT figure could ride along into a booking.
    let advice = Advice::new(
        "Apple monthly invoice is an EU reverse-charge purchase",
        vec!["AVL 9 §".to_string()],
    );
    let v = serde_json::to_value(&advice).unwrap();
    let obj = v.as_object().unwrap();
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, vec!["citations", "summary"]);
}

#[test]
fn advice_becomes_a_pii_free_knowledge_candidate() {
    let advice = Advice::new(
        "Apple monthly = reverse-charge; deductible per law",
        vec!["AVL 9 §".to_string()],
    );
    let candidate = advice.into_knowledge_candidate("apple-monthly-reverse-charge");
    let doc = candidate.to_document();
    assert!(doc.contains("apple-monthly-reverse-charge"));
    assert!(doc.contains("AVL 9 §"));
    // A candidate is built from Advice, which structurally carries no PII.
    assert!(!doc.to_lowercase().contains("hetu"));
    assert!(!doc.to_lowercase().contains("iban"));
}
