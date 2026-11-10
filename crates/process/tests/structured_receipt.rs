use std::fs;
use std::path::{Path, PathBuf};

use aggountant_domain::{verify_vat_expansion_with_role_table, VatRecompute};
use aggountant_ledger::Ledger;
use aggountant_process::{
    prepare_structured_receipt, record_structured_receipt, StructuredReceiptInput,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn input(vat: Value) -> StructuredReceiptInput {
    serde_json::from_value(json!({
        "natural_key": "receipt:fictional-2026-07-15",
        "actor": "test-agent",
        "source": {
            "amount": {"amount": "125.50", "currency": "EUR"},
            "document_date": "2026-07-15",
            "description": "Fictional office service"
        },
        "proposal": {
            "posting_date": "2026-07-16",
            "net_account": "7690",
            "contra_account": "1910",
            "vat": vat
        }
    }))
    .unwrap()
}

fn voucher_count(home: &Path) -> u64 {
    Ledger::open(home).unwrap().verify().unwrap().voucher_count
}

#[test]
fn domestic_input_preview_record_retry_and_recompute_are_one_projection() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let input = input(json!({
        "type": "domestic-input",
        "rate_class": "domestic-standard"
    }));

    let prepared = prepare_structured_receipt(&ledger, &input).unwrap();
    assert!(prepared.projection_valid && prepared.ledger_preflight_valid);
    assert_eq!(voucher_count(home.path()), 0, "preview must not write");
    assert_eq!(prepared.draft.postings()[0].amount.amount_str(), "100.00");
    assert_eq!(prepared.draft.postings()[1].amount.amount_str(), "25.50");

    let first = record_structured_receipt(&ledger, &input, &prepared.preparation_hash).unwrap();
    assert!(!first.replayed);
    let retry = record_structured_receipt(&ledger, &input, &prepared.preparation_hash).unwrap();
    assert!(retry.replayed);
    assert_eq!(first.number, retry.number);
    assert_eq!(voucher_count(home.path()), 1);

    let sealed = ledger.voucher(first.number).unwrap();
    assert_eq!(sealed.body.postings(), prepared.draft.postings());
    assert!(matches!(
        verify_vat_expansion_with_role_table(&sealed.body, ledger.vat_role_table()).unwrap(),
        VatRecompute::Verified { .. }
    ));
    assert_eq!(
        sealed.provenance.source_fingerprint.as_deref(),
        Some(prepared.source_fingerprint.as_str())
    );
}

#[test]
fn no_vat_is_explicit_balanced_and_recomputable() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let mut input = input(json!({"type": "no-vat"}));
    input.source.amount = serde_json::from_value(json!({
        "amount": "80.00", "currency": "EUR"
    }))
    .unwrap();
    let prepared = prepare_structured_receipt(&ledger, &input).unwrap();
    assert_eq!(prepared.draft.postings().len(), 2);
    assert!(prepared.draft.vat_expansion().is_some());
    let sealed = record_structured_receipt(&ledger, &input, &prepared.preparation_hash).unwrap();
    let body = ledger.voucher(sealed.number).unwrap().body;
    assert!(matches!(
        verify_vat_expansion_with_role_table(&body, ledger.vat_role_table()).unwrap(),
        VatRecompute::Verified { .. }
    ));
}

#[test]
fn missing_date_and_explicit_unsupported_or_unclear_treatment_refuse_without_write() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();

    let mut missing = input(json!({"type": "no-vat"}));
    missing.source.document_date = None;
    let err = prepare_structured_receipt(&ledger, &missing).unwrap_err();
    assert_eq!(err.error_code(), "CAPTURE_FACT_REQUIRED");

    let unsupported = input(json!({
        "type": "unsupported",
        "treatment": "margin-scheme",
        "reason": "requires a separate margin calculation"
    }));
    let err = prepare_structured_receipt(&ledger, &unsupported).unwrap_err();
    assert_eq!(err.error_code(), "CAPTURE_UNSUPPORTED_TREATMENT");

    let unclear = input(json!({
        "type": "clarification-required",
        "question": "Is the supplier established in Finland?"
    }));
    let err = prepare_structured_receipt(&ledger, &unclear).unwrap_err();
    assert_eq!(err.error_code(), "CAPTURE_CLARIFICATION_REQUIRED");
    assert_eq!(voucher_count(home.path()), 0);
}

#[test]
fn approval_hash_binds_the_proposal() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let original = input(json!({"type": "no-vat"}));
    let prepared = prepare_structured_receipt(&ledger, &original).unwrap();

    let mut changed_actor = input(json!({"type": "no-vat"}));
    changed_actor.actor = "different-agent".into();
    let err =
        record_structured_receipt(&ledger, &changed_actor, &prepared.preparation_hash).unwrap_err();
    assert_eq!(err.error_code(), "CAPTURE_PREPARATION_CHANGED");

    let changed = input(json!({
        "type": "domestic-input",
        "rate_class": "domestic-standard"
    }));
    let err = record_structured_receipt(&ledger, &changed, &prepared.preparation_hash).unwrap_err();
    assert_eq!(err.error_code(), "CAPTURE_PREPARATION_CHANGED");
    assert_eq!(voucher_count(home.path()), 0);
}

fn reference_dir(home: &Path) -> PathBuf {
    home.join("masterdata/reference")
}

fn install_reverse_charge_chart(home: &Path) {
    let reference = reference_dir(home);
    let chart_path = reference.join("chart-of-accounts.json");
    let mut chart: Value = serde_json::from_slice(&fs::read(&chart_path).unwrap()).unwrap();
    chart["chart"] = json!("capture-rc-chart");
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
    for (mut template, id) in [
        (input_template.clone(), "1798"),
        (input_template.clone(), "1797"),
        (output_template.clone(), "2982"),
        (output_template.clone(), "2992"),
        (output_template, "2991"),
    ] {
        template["id"] = json!(id);
        accounts.push(template);
    }
    accounts.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    // serde_json maps are key-sorted in this workspace; for this integer/string-only
    // document its compact encoding is the chart's canonical JSON hash preimage.
    let canonical = serde_json::to_vec(&chart).unwrap();
    let version = format!(
        "capture-rc-chart@{}",
        hex::encode(Sha256::digest(&canonical))
    );
    let pretty = serde_json::to_vec_pretty(&chart).unwrap();
    fs::write(&chart_path, &pretty).unwrap();
    fs::write(reference.join("chart_version"), format!("{version}\n")).unwrap();
    fs::write(
        reference
            .join("chart-snapshots")
            .join(format!("{}.json", version.rsplit_once('@').unwrap().1)),
        &pretty,
    )
    .unwrap();
    fs::write(
        reference.join("vat-account-roles.json"),
        serde_json::to_vec_pretty(&json!({
            "table": "capture-reverse-charge",
            "charts": [{
                "chart_version": version,
                "active": true,
                "accounts": [
                    {"account":"1763","role":"input-vat"},
                    {"account":"1798","role":"input-vat"},
                    {"account":"1797","role":"input-vat"},
                    {"account":"2939","role":"tagged-only"},
                    {"account":"2982","role":"reverse-charge-output-vat","reverse_charge_scope":"non-eu-services","paired_input_account":"1763"},
                    {"account":"2992","role":"reverse-charge-output-vat","reverse_charge_scope":"eu-services","paired_input_account":"1798"},
                    {"account":"2991","role":"reverse-charge-output-vat","reverse_charge_scope":"eu-goods","paired_input_account":"1797"}
                ],
                "mistag_guards": [{"account":"7010","dedicated_rate_class":"domestic-electricity"}]
            }]
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn chart_owned_reverse_charge_scopes_seal_and_recompute() {
    for (scope, input_account, output_account) in [
        ("eu-services", "1798", "2992"),
        ("eu-goods", "1797", "2991"),
        ("non-eu-services", "1763", "2982"),
    ] {
        let home = init_home();
        install_reverse_charge_chart(home.path());
        let ledger = Ledger::open(home.path()).unwrap();
        let mut input = input(json!({
            "type": "reverse-charge",
            "rate_class": "domestic-standard",
            "scope": scope
        }));
        input.source.amount = serde_json::from_value(json!({
            "amount": "100.00", "currency": "EUR"
        }))
        .unwrap();
        let prepared = prepare_structured_receipt(&ledger, &input).unwrap();
        assert!(prepared
            .draft
            .postings()
            .iter()
            .any(|posting| posting.account.as_str() == input_account));
        assert!(prepared
            .draft
            .postings()
            .iter()
            .any(|posting| posting.account.as_str() == output_account));
        let sealed =
            record_structured_receipt(&ledger, &input, &prepared.preparation_hash).unwrap();
        let body = ledger.voucher(sealed.number).unwrap().body;
        assert!(matches!(
            verify_vat_expansion_with_role_table(&body, ledger.vat_role_table()).unwrap(),
            VatRecompute::Verified { .. }
        ));
    }
}
