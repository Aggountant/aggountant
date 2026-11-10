//! Regression tests for `closeyear-statement-completeness` (part 3): the canonical
//! **close-package hash** the write-port composes and `SealYear` should bind. Today
//! `seal_year` binds only the statements hash, so a sealed year does not attest the
//! `tase-erittelyt` or `liitetiedot` it was closed with. `close_package_hash` folds all
//! three components under one domain-separated, tamper-evident sha256, and
//! `Ledger::close_package_hash_for_year` recomputes the two ledger-derivable components
//! from the year's sealed vouchers (derived from the ledger, not trusted from input;
//! reproducibility is conditional on the same chart/policy context — see the method doc).

use aggountant_domain::{
    compute_statements, kpa_policy, kpa_policy_version, tase_erittelyt, PolicySet,
    StatementProvenance,
};
use aggountant_ledger::{close_package_hash, Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, Posting, Side, VoucherDraft};

const POLICY_SET: &str = "fi.core.2026-01@stub";
/// A stand-in for the determined `liitetiedot` component (a real caller passes
/// `Liitetiedot::statements_hash()`; the composition treats it as opaque 64-hex input).
const LIITE_HASH: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn eur() -> Currency {
    Currency::new("EUR").unwrap()
}

fn seal(ledger: &Ledger, date: &str, debit: &str, credit: &str, amount: &str, nk: &str) {
    let d = Date::parse(date).unwrap();
    let postings = vec![
        Posting {
            account: AccountId::new(debit).unwrap(),
            amount: Money::parse(amount, eur()).unwrap(),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: AccountId::new(credit).unwrap(),
            amount: Money::parse(amount, eur()).unwrap(),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    let draft = VoucherDraft::new(d, d, "test".into(), postings, None).unwrap();
    ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: nk.into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
}

/// Seal a self-contained 2026 year: capital injection, a sale, an expense.
fn seal_year_2026(ledger: &Ledger) {
    seal(ledger, "2026-01-05", "1910", "2000", "5000.00", "cap");
    seal(ledger, "2026-03-31", "1910", "3000", "10000.00", "sale");
    seal(ledger, "2026-06-30", "7690", "1910", "2000.00", "exp");
}

/// Independently recompute the two ledger-derivable component hashes the way the
/// close-year Run does, so a test can prove the ledger method composes exactly this.
fn expected_component_hashes(ledger: &Ledger, year: i32) -> (String, String) {
    let data = ledger.fiscal_year_data(year).unwrap();
    let vouchers = &data.vouchers;
    let chart = ledger.chart_list(None).unwrap();
    let provenance = StatementProvenance {
        chart_version: ledger.chart_version().unwrap(),
        policy_set: POLICY_SET.to_string(),
        kpa_policy_version: kpa_policy_version().to_string(),
    };
    let s = compute_statements(
        year,
        data.opening_balance.as_ref(),
        vouchers,
        &chart,
        kpa_policy(),
        provenance.clone(),
    )
    .unwrap();
    let te = tase_erittelyt(
        year,
        data.opening_balance.as_ref(),
        vouchers,
        &chart,
        provenance,
    )
    .unwrap();
    (s.statements_hash(), te.statements_hash())
}

#[test]
fn for_year_reproduces_an_independent_domain_composition() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_year_2026(&ledger);

    let (sh, teh) = expected_component_hashes(&ledger, 2026);
    let expected = close_package_hash(&sh, &teh, LIITE_HASH).unwrap();

    let got = ledger
        .close_package_hash_for_year(2026, POLICY_SET, LIITE_HASH)
        .unwrap();
    assert_eq!(
        got, expected,
        "close_package_hash_for_year must fold exactly the sealed-year statements + \
         erittelyt component hashes"
    );
}

#[test]
fn for_year_is_deterministic() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_year_2026(&ledger);

    let a = ledger
        .close_package_hash_for_year(2026, POLICY_SET, LIITE_HASH)
        .unwrap();
    let b = ledger
        .close_package_hash_for_year(2026, POLICY_SET, LIITE_HASH)
        .unwrap();
    assert_eq!(a, b);
}

#[test]
fn every_component_changing_changes_the_package_and_it_is_domain_separated() {
    let a = "a".repeat(64);
    let b = "b".repeat(64);
    let c = "c".repeat(64);
    let base = close_package_hash(&a, &b, &c).unwrap();

    // Changing any single component changes the package hash (tamper-evidence).
    assert_ne!(base, close_package_hash(&b, &b, &c).unwrap());
    assert_ne!(base, close_package_hash(&a, &c, &c).unwrap());
    assert_ne!(base, close_package_hash(&a, &b, &a).unwrap());

    // Order matters: swapping statements and erittelyt is a different package.
    assert_ne!(base, close_package_hash(&b, &a, &c).unwrap());

    // Domain-separated: the package hash is never equal to a bare component hash.
    assert_ne!(base, a);
    assert_ne!(base, b);
    assert_ne!(base, c);
    // ...and it is itself a well-formed sha256 digest.
    assert_eq!(base.len(), 64);
    assert!(base
        .bytes()
        .all(|x| x.is_ascii_hexdigit() && !x.is_ascii_uppercase()));
}

#[test]
fn a_malformed_component_is_refused() {
    let ok = "0".repeat(64);
    // A malformed digest is a structural input-format failure (INVALID_HASH), not a
    // statements-projection failure. Exercise every rejection the shared validator makes:
    let bad = [
        "".to_string(),                  // empty
        "deadbeef".to_string(),          // too short
        "0".repeat(63),                  // 63 chars
        "0".repeat(65),                  // 65 chars
        "A".repeat(64),                  // uppercase (non-canonical)
        format!("{}A", "0".repeat(63)),  // mixed case
        "z".repeat(64),                  // non-hex
        format!("{}\n", "0".repeat(63)), // trailing newline (paste artifact)
    ];
    for b in &bad {
        // In each of the three component positions.
        for triple in [
            (b.as_str(), ok.as_str(), ok.as_str()),
            (ok.as_str(), b.as_str(), ok.as_str()),
            (ok.as_str(), ok.as_str(), b.as_str()),
        ] {
            let err = close_package_hash(triple.0, triple.1, triple.2).unwrap_err();
            assert_eq!(
                err.error_code(),
                "INVALID_HASH",
                "for malformed input {b:?}"
            );
        }
    }
    // A well-formed triple composes cleanly.
    assert!(close_package_hash(&ok, &ok, &ok).is_ok());
}

#[test]
fn for_year_refuses_a_malformed_liitetiedot_hash() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_year_2026(&ledger);
    let err = ledger
        .close_package_hash_for_year(2026, POLICY_SET, "not-a-hash")
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_HASH");
}

#[test]
fn for_year_folds_the_liitetiedot_component() {
    // Changing only the liitetiedot hash must change the package hash — guards against a
    // future refactor accidentally dropping the notes from the fold.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_year_2026(&ledger);
    let with_a = ledger
        .close_package_hash_for_year(2026, POLICY_SET, &"a".repeat(64))
        .unwrap();
    let with_b = ledger
        .close_package_hash_for_year(2026, POLICY_SET, &"b".repeat(64))
        .unwrap();
    assert_ne!(with_a, with_b);
}

#[test]
fn the_pure_composition_encoding_is_pinned() {
    // A focused golden over ONLY the composition encoding (fixed component inputs, no
    // ledger/chart/domain involvement). If this holds but the end-to-end golden drifts,
    // the drift is in the domain projection, not the package encoding — and vice versa.
    let got = close_package_hash(&"0".repeat(64), &"1".repeat(64), &"2".repeat(64)).unwrap();
    assert_eq!(
        got, "7a75eba970f468b80fb33274098769847ab6cc7e991a50f59296f24d8459023d",
        "close-package composition encoding drifted — update deliberately if intended"
    );
}

#[test]
fn a_golden_close_package_hash_is_pinned() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_year_2026(&ledger);

    let got = ledger
        .close_package_hash_for_year(2026, POLICY_SET, LIITE_HASH)
        .unwrap();
    // Pin the exact package hash: any drift in the composition encoding, the KPA
    // layout/labels, the erittelyt fold, or the provenance stamping breaks this literal
    // (a real golden — update deliberately if the change is intended). The chart_version
    // is content-addressed from the bundled default chart, so it is stable across runs.
    assert_eq!(
        got, "01369deeeb63dbd15118cd1db4962ddb320a294cc42aef99f310b64351e4a1bb",
        "golden close-package hash drifted — update deliberately if the change is intended"
    );
}
