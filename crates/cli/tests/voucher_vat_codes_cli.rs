//! `voucher vat-codes` through the built binary: offline, needs no home, lists
//! the codes of the dated rate table for the document date, and the seal help
//! points to it.

use std::process::Command;

use serde_json::Value;

fn aggountant(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(args)
        .output()
        .expect("spawn aggountant");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn vat_codes_lists_the_codes_valid_on_the_document_date() {
    let (code, stdout, stderr) = aggountant(&[
        "--json",
        "voucher",
        "vat-codes",
        "--document-date",
        "2025-12-31",
    ]);
    assert_eq!(code, 0, "{stderr}");
    let value: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["document_date"], "2025-12-31");
    let codes = value["codes"].as_array().unwrap();
    let reduced = codes
        .iter()
        .find(|c| c["code"] == "purchase-domestic-reduced-1")
        .unwrap();
    assert_eq!(reduced["percent"], "14", "the rate before 2026-01-01");
    assert_eq!(reduced["accepts_partial_deduction"], true);
    assert!(codes.iter().all(|c| c["code"] != "purchase-eu-goods"));
}

#[test]
fn vat_codes_refuses_a_malformed_date() {
    let (code, _, stderr) = aggountant(&[
        "--json",
        "voucher",
        "vat-codes",
        "--document-date",
        "2026-1-5",
    ]);
    assert_eq!(code, 1, "{stderr}");
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["error_code"], "INVALID_DATE");
}

#[test]
fn seal_and_draft_help_say_where_vat_code_goes() {
    for command in ["seal", "draft"] {
        let (code, stdout, _) = aggountant(&["voucher", command, "--help"]);
        assert_eq!(code, 0);
        assert!(
            stdout.contains("never") && stdout.contains("VAT account line"),
            "{stdout}"
        );
        assert!(
            stdout.contains("voucher vat-codes --document-date"),
            "{stdout}"
        );
    }
}
