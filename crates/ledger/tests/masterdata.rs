//! Git-native reference masterdata — the `aggountant init` write path and the
//! ADR-1 §Masterdata contract guarantees, exercised through the public `Ledger`
//! API (the store ports are `pub(crate)`).
//!
//! Covers the slice's done-criteria: init happy path, the git-native read/write
//! round-trip, `chart_version` content-hash stability, re-init safety — plus the
//! two ADR-1 binding constraints for plaintext reference masterdata: a
//! **fail-closed classification** whitelist and a **PII canary** scan of the
//! actual git bytes.

use std::fs;
use std::path::Path;

use aggountant_domain::{
    vat_expand_with_role_table, PolicySet, ReverseChargeScope, VatExpandInput, VatReturnField,
    VatTreatment,
};
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

fn reference_dir(home: &Path) -> std::path::PathBuf {
    home.join("masterdata").join("reference")
}

#[test]
fn init_writes_the_real_chart_as_git_native_plaintext() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    // A real Finnish chart, not the S1 5-account stub.
    let accounts = ledger.chart_list(None).unwrap();
    assert!(accounts.len() > 50, "real chart, got {}", accounts.len());
    assert!(accounts.iter().any(|a| a.id.as_str() == "1910"));
    assert!(accounts.iter().any(|a| a.id.as_str() == "7690"));
    // All account kinds are represented (vastaavaa / vastattavaa / tulot / menot).
    use aggountant_model::AccountKind::*;
    for kind in [Asset, Liability, Equity, Income, Expense] {
        assert!(accounts.iter().any(|a| a.kind == kind), "missing {kind:?}");
    }

    // The tree is on disk as diffable plaintext files.
    let ref_dir = reference_dir(home.path());
    assert!(ref_dir.join("chart-of-accounts.json").is_file());
    assert!(ref_dir.join("chart_version").is_file());
    assert!(ref_dir.join("vat-account-roles.json").is_file());
    assert!(ref_dir.join("README.md").is_file());

    assert!(ledger
        .chart_version()
        .unwrap()
        .starts_with("fi-general-2026@"));
}

#[test]
fn custom_chart_roles_are_joined_to_the_active_chart_and_drive_projection() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let reference = reference_dir(home.path());

    let chart_path = reference.join("chart-of-accounts.json");
    let mut chart: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&chart_path).unwrap()).unwrap();
    chart["chart"] = serde_json::json!("company-chart");
    let accounts = chart["accounts"].as_array_mut().unwrap();
    let input_template = accounts
        .iter()
        .find(|account| account["id"] == "1763")
        .unwrap()
        .clone();
    let output_template = accounts
        .iter()
        .find(|account| account["id"] == "2939")
        .unwrap()
        .clone();
    for account in accounts.iter_mut() {
        match account["id"].as_str().unwrap() {
            "1763" => account["id"] = serde_json::json!("1801"),
            "2939" => account["id"] = serde_json::json!("2901"),
            _ => {}
        }
    }
    for (mut account, id) in [
        (input_template.clone(), "1802"),
        (input_template, "1803"),
        (output_template.clone(), "2902"),
        (output_template, "2903"),
    ] {
        account["id"] = serde_json::json!(id);
        accounts.push(account);
    }
    accounts.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let canonical = aggountant_wire::canonical_bytes(&chart).unwrap();
    let version = format!("company-chart@{}", hex::encode(Sha256::digest(canonical)));
    let chart_bytes = serde_json::to_vec_pretty(&chart).unwrap();
    fs::write(&chart_path, &chart_bytes).unwrap();
    fs::write(reference.join("chart_version"), format!("{version}\n")).unwrap();
    let digest = version.rsplit_once('@').unwrap().1;
    fs::write(
        reference
            .join("chart-snapshots")
            .join(format!("{digest}.json")),
        &chart_bytes,
    )
    .unwrap();
    fs::write(
        reference.join("vat-account-roles.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "table": "company-import",
            "charts": [{
                "chart_version": version,
                "active": true,
                "accounts": [
                    { "account": "1801", "role": "input-vat" },
                    { "account": "1802", "role": "input-vat" },
                    { "account": "1803", "role": "input-vat" },
                    { "account": "2901", "role": "reverse-charge-output-vat", "reverse_charge_scope": "non-eu-services", "paired_input_account": "1801" },
                    { "account": "2902", "role": "reverse-charge-output-vat", "reverse_charge_scope": "eu-goods", "paired_input_account": "1802" },
                    { "account": "2903", "role": "reverse-charge-output-vat", "reverse_charge_scope": "eu-services", "paired_input_account": "1803" }
                ],
                "mistag_guards": []
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    let ledger = Ledger::open(home.path()).unwrap();
    let eur = Currency::new("EUR").unwrap();
    let money = |amount: i64| Money::new(Decimal::from(amount), eur.clone());
    let draft = VoucherDraft::new(
        Date::parse("2026-07-15").unwrap(),
        Date::parse("2026-07-15").unwrap(),
        "custom VAT roles".into(),
        vec![
            Posting {
                account: AccountId::new("7690").unwrap(),
                amount: money(100),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1801").unwrap(),
                amount: money(25),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: money(125),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    assert_eq!(
        ledger
            .draft_deductible_vat(&draft, &ledger.chart_version().unwrap())
            .unwrap()
            .amount_str(),
        "25.00"
    );

    let d = Date::parse("2026-07-15").unwrap();
    let expanded = vat_expand_with_role_table(
        &VatExpandInput {
            amount: Money::parse("100.00", eur).unwrap(),
            net_account: AccountId::new("7690").unwrap(),
            contra_account: AccountId::new("1910").unwrap(),
            treatment: VatTreatment::ReverseCharge {
                rate_class: "domestic-standard".into(),
                scope: ReverseChargeScope::EuServices,
            },
            document_date: d,
            posting_date: d,
            description: "synthetic EU service".into(),
            attachment_ref: None,
        },
        &version,
        ledger.vat_role_table(),
    )
    .unwrap();
    assert!(expanded
        .postings()
        .iter()
        .any(|posting| posting.account.as_str() == "2903"));
    ledger.validate(&expanded).unwrap();
    ledger
        .seal(
            &expanded,
            &SealRequest {
                natural_key: "custom-rc".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    let period = PeriodId::parse("2026-07").unwrap();
    let figures = ledger
        .derive_vat_filing_figures(&period, &ledger.period_vouchers(&period).unwrap())
        .unwrap();
    assert_eq!(
        figures
            .fields
            .get(VatReturnField::Field306)
            .unwrap()
            .amount_str(),
        "25.50"
    );
    assert_eq!(
        figures
            .fields
            .get(VatReturnField::Field313)
            .unwrap()
            .amount_str(),
        "100.00"
    );
}

#[test]
fn chart_activation_refuses_missing_incomplete_or_foreign_roles() {
    for mutation in ["missing", "incomplete", "foreign-account"] {
        let home = tempfile::tempdir().unwrap();
        Ledger::init(home.path()).unwrap();
        let reference = reference_dir(home.path());
        let role_path = reference.join("vat-account-roles.json");
        match mutation {
            "missing" => fs::remove_file(&role_path).unwrap(),
            "incomplete" => {
                let mut roles: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(&role_path).unwrap()).unwrap();
                roles["charts"][0]["accounts"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|role| role["role"].as_str() != Some("tagged-only"));
                fs::write(&role_path, serde_json::to_vec_pretty(&roles).unwrap()).unwrap();
            }
            "foreign-account" => {
                let mut roles: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(&role_path).unwrap()).unwrap();
                roles["charts"][0]["accounts"][0]["account"] = serde_json::json!("9998");
                fs::write(&role_path, serde_json::to_vec_pretty(&roles).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        let err = Ledger::open(home.path()).map(|_| ()).unwrap_err();
        assert_eq!(err.error_code(), "MASTERDATA_CORRUPT", "case {mutation}");
    }
}

#[test]
fn git_native_round_trips_through_open() {
    let home = tempfile::tempdir().unwrap();
    let written = Ledger::init(home.path()).unwrap();
    let written_version = written.chart_version().unwrap();
    let written_accounts = written.chart_list(None).unwrap();
    drop(written);

    // A fresh process-equivalent open reads the identical chart + version.
    let reopened = Ledger::open(home.path()).unwrap();
    assert_eq!(reopened.chart_version().unwrap(), written_version);
    assert_eq!(reopened.chart_list(None).unwrap(), written_accounts);
    assert!(reopened
        .chart_list(None)
        .unwrap()
        .iter()
        .any(|a| a.id == AccountId::new("7690").unwrap()));
}

#[test]
fn chart_version_is_a_stable_content_hash() {
    // Same chart content on two independent homes → identical content hash,
    // and the hash suffix is the full 64 hex chars (32-byte sha256).
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let va = Ledger::init(a.path()).unwrap().chart_version().unwrap();
    let vb = Ledger::init(b.path()).unwrap().chart_version().unwrap();
    assert_eq!(va, vb);
    let suffix = va.rsplit('@').next().unwrap();
    assert_eq!(suffix.len(), 64);
    assert!(suffix.bytes().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn chart_date_is_a_human_readable_edition_marker() {
    // A person can tell which edition a home carries by reading the plaintext
    // `chart_date` — no hashing required — and it round-trips through open().
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let raw =
        fs::read_to_string(reference_dir(home.path()).join("chart-of-accounts.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();

    let date = doc["chart_date"].as_str().expect("chart_date is a string");
    // A concrete ISO date a human can read without recomputing the content hash.
    assert_eq!(
        date, "2026-01-01",
        "the bundled fi-general-2026 edition date"
    );

    // open() must accept the edition-marked chart (drift check passes).
    assert!(Ledger::open(home.path()).is_ok());
}

#[test]
fn open_detects_chart_date_edited_without_reversioning() {
    // `chart_date` participates in the content hash, so a hand-edit of the edition
    // marker alone (without re-deriving chart_version) must be caught as drift —
    // this guards against a future refactor dropping it from the hashed view.
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();

    let chart_path = reference_dir(home.path()).join("chart-of-accounts.json");
    let raw = fs::read_to_string(&chart_path).unwrap();
    let tampered = raw.replace("2026-01-01", "2026-02-01");
    assert_ne!(raw, tampered, "the replacement must change bytes");
    fs::write(&chart_path, tampered).unwrap();

    let err = Ledger::open(home.path()).map(|_| ()).unwrap_err();
    assert_eq!(err.error_code(), "MASTERDATA_CORRUPT");
}

#[test]
fn re_init_refuses_and_does_not_clobber() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();

    let chart_path = reference_dir(home.path()).join("chart-of-accounts.json");
    let before = fs::read(&chart_path).unwrap();

    let err = Ledger::init(home.path()).map(|_| ()).unwrap_err();
    assert_eq!(err.error_code(), "ALREADY_INITIALIZED");

    // The existing (possibly hand-curated, git-tracked) chart is untouched.
    let after = fs::read(&chart_path).unwrap();
    assert_eq!(before, after, "re-init must not rewrite the chart file");
}

#[test]
fn open_before_init_is_a_clear_error() {
    let home = tempfile::tempdir().unwrap();
    let err = Ledger::open(home.path()).map(|_| ()).unwrap_err();
    assert_eq!(err.error_code(), "MASTERDATA_NOT_INITIALIZED");
}

#[test]
fn init_publishes_atomically_and_writes_a_home_gitignore() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();

    // No staging directory is left behind after a successful publish.
    let leftovers: Vec<_> = fs::read_dir(home.path().join("masterdata"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".reference.staging")
        })
        .collect();
    assert!(leftovers.is_empty(), "staging dir leaked: {leftovers:?}");

    // A home .gitignore keeps the sensitive stores out of git while allowing the
    // reference masterdata to be committed.
    let gi = fs::read_to_string(home.path().join(".gitignore")).unwrap();
    assert!(gi.contains("/fiscal.sqlite"));
    assert!(gi.contains("/artifacts/"));
    assert!(gi.contains("/masterdata/pii.sqlite"));
}

#[test]
fn concurrent_init_has_exactly_one_winner() {
    use std::sync::Arc;
    let home = Arc::new(tempfile::tempdir().unwrap());
    let mut handles = Vec::new();
    for _ in 0..8 {
        let home = Arc::clone(&home);
        handles.push(std::thread::spawn(move || {
            match Ledger::init(home.path()) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.error_code().to_string()),
            }
        }));
    }
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let winners = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(winners, 1, "exactly one init wins; results={results:?}");
    // Every loser refused cleanly with ALREADY_INITIALIZED (no clobber, no panic).
    for r in &results {
        if let Err(code) = r {
            assert_eq!(code, "ALREADY_INITIALIZED");
        }
    }
    // The published chart is intact and readable.
    assert!(
        Ledger::open(home.path())
            .unwrap()
            .chart_list(None)
            .unwrap()
            .len()
            > 50
    );
}

/// ADR-1 binding constraint: reference masterdata is a **positive whitelist** of
/// known-safe fields. This fails closed if the on-disk schema grows a field
/// without a classification decision.
#[test]
fn reference_schema_is_a_positive_non_pii_whitelist() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let raw =
        fs::read_to_string(reference_dir(home.path()).join("chart-of-accounts.json")).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();

    // `chart_date` is a classified non-PII edition marker (an ISO date, no
    // personal data); `chart`/`accounts` are the existing document fields.
    const TOP_ALLOWED: &[&str] = &["chart", "chart_date", "accounts"];
    const ACCOUNT_ALLOWED: &[&str] = &["id", "name", "kind"];

    for key in doc.as_object().unwrap().keys() {
        assert!(
            TOP_ALLOWED.contains(&key.as_str()),
            "unclassified top-level field {key:?} in plaintext reference masterdata"
        );
    }
    for acc in doc["accounts"].as_array().unwrap() {
        for key in acc.as_object().unwrap().keys() {
            assert!(
                ACCOUNT_ALLOWED.contains(&key.as_str()),
                "unclassified account field {key:?} — classify it before it ships as plaintext"
            );
        }
    }
}

/// ADR-1 canary: no HETU/IBAN-shaped token may appear in the plaintext git
/// bytes. The bundled standard chart has none; this guards against a future edit
/// leaking one.
#[test]
fn no_pii_token_in_the_git_bytes() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let bytes =
        fs::read_to_string(reference_dir(home.path()).join("chart-of-accounts.json")).unwrap();
    assert!(!looks_like_iban(&bytes), "IBAN-shaped token in chart bytes");
    assert!(!looks_like_hetu(&bytes), "HETU-shaped token in chart bytes");
}

/// Finnish IBAN: `FI` + 16 digits (18 chars total).
fn looks_like_iban(s: &str) -> bool {
    let b = s.as_bytes();
    b.windows(18)
        .any(|w| w[0] == b'F' && w[1] == b'I' && w[2..].iter().all(|c| c.is_ascii_digit()))
}

/// Finnish HETU: 6 digits (ddmmyy) + a century marker (`+`, `-`, `A`, …) + 3
/// digits + a checksum char.
fn looks_like_hetu(s: &str) -> bool {
    let b = s.as_bytes();
    b.windows(11).any(|w| {
        w[0..6].iter().all(|c| c.is_ascii_digit())
            && matches!(w[6], b'+' | b'-' | b'A' | b'B' | b'X' | b'Y')
            && w[7..10].iter().all(|c| c.is_ascii_digit())
            && (w[10].is_ascii_digit() || w[10].is_ascii_uppercase())
    })
}
