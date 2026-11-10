//! F7 employee master-data (S4) — the PII-isolation contract at the Ledger level
//! (must-spec #2). Personal data (HETU/IBAN) goes only into the erasable encrypted
//! store; it never reaches the git-native reference tree, and the home `.gitignore`
//! excludes the PII database.

use aggountant_domain::TaxCard;
use aggountant_ledger::{EmployeeState, Ledger};
use aggountant_model::{Currency, Money};

fn eur(s: &str) -> Money {
    Money::parse(s, Currency::new("EUR").unwrap()).unwrap()
}

fn sample() -> EmployeeState {
    EmployeeState {
        employee_id: "emp-1".into(),
        name: "Maija Meikäläinen".into(),
        hetu: "131052-308T".into(),
        iban: "FI2112345600000785".into(),
        tax_card: TaxCard {
            withholding_pct: "22.00".into(),
        },
    }
}

#[test]
fn employee_round_trips_and_ytd_accumulates() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    ledger.upsert_employee(&sample()).unwrap();
    assert_eq!(ledger.get_employee("emp-1").unwrap(), sample());
    assert_eq!(ledger.list_employees().unwrap(), vec!["emp-1".to_string()]);

    let after = ledger
        .accumulate_ytd("emp-1", 2026, "payroll:emp-1:2026-02-27", &eur("3000.00"))
        .unwrap();
    assert_eq!(after.amount_str(), "3000.00");
    // A reopened Ledger sees the durable YTD (master-data read through the Ledger).
    let reopened = Ledger::open(home.path()).unwrap();
    assert_eq!(
        reopened.ytd_gross("emp-1", 2026).unwrap().amount_str(),
        "3000.00"
    );
    // A different tax year is a fresh accumulator, not a continuation of 2026.
    assert_eq!(
        reopened.ytd_gross("emp-1", 2027).unwrap().amount_str(),
        "0.00"
    );
}

#[test]
fn unknown_employee_is_a_clear_error() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let err = ledger.get_employee("ghost").unwrap_err();
    assert_eq!(err.error_code(), "EMPLOYEE_NOT_FOUND");
}

#[test]
fn pii_never_reaches_the_git_native_reference_tree() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    ledger.upsert_employee(&sample()).unwrap();

    // The reviewable, committable reference tree holds the chart — never PII.
    let reference = home.path().join("masterdata").join("reference");
    for name in ["chart-of-accounts.json", "chart_version", "README.md"] {
        let text = std::fs::read_to_string(reference.join(name)).unwrap();
        assert!(!text.contains("131052-308T"), "HETU leaked into {name}");
        assert!(
            !text.contains("FI2112345600000785"),
            "IBAN leaked into {name}"
        );
    }

    // The home `.gitignore` excludes the erasable PII database.
    let gitignore = std::fs::read_to_string(home.path().join(".gitignore")).unwrap();
    assert!(
        gitignore.contains("masterdata/pii.sqlite"),
        ".gitignore must exclude the PII store so it is never committed"
    );
    // The PII store itself exists (created by init/open).
    assert!(home.path().join("masterdata").join("pii.sqlite").exists());
}
