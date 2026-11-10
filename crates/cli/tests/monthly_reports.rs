use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn run(home: &std::path::Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut cmd = bin();
    cmd.arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn seal_input(date: &str, key: &str, amount: &str) -> String {
    format!(
        r#"{{"draft":{{"document_date":"{date}","posting_date":"{date}","description":"Fiktiivinen & palvelu","postings":[{{"account":"1910","amount":{{"amount":"{amount}","currency":"EUR"}},"side":"debit"}},{{"account":"3000","amount":{{"amount":"{amount}","currency":"EUR"}},"side":"credit"}}]}},"natural_key":"{key}"}}"#
    )
}

#[test]
fn cli_writes_explicit_native_and_html_without_mutating_ledger() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(run(home.path(), &["init"], "").0, 0);
    assert_eq!(
        run(
            home.path(),
            &["voucher", "seal"],
            &seal_input("2026-07-15", "sale", "1234.50")
        )
        .0,
        0
    );
    assert_eq!(
        run(
            home.path(),
            &["voucher", "seal"],
            &seal_input("2026-08-01", "future", "999.00")
        )
        .0,
        0
    );
    let before: Value =
        serde_json::from_str(&run(home.path(), &["voucher", "verify"], "").1).unwrap();

    let native = home.path().join("july.json");
    let html = home.path().join("july.html");
    let native_s = native.to_str().unwrap();
    let html_s = html.to_str().unwrap();
    let common = [
        "report",
        "statements",
        "--period",
        "2026-07",
        "--company",
        "Fiktiivinen <Yhtiö> & Kumppanit",
        "--history-start",
        "2026-01-01",
        "--limitation",
        "Historia on ilmoitettu, ei varmennettu",
    ];
    let mut native_args = common.to_vec();
    native_args.extend(["--format", "native-json", "--output", native_s]);
    let (code, stdout, error) = run(home.path(), &native_args, "");
    assert_eq!(code, 0, "{error}");
    let metadata: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(metadata["format"], "native-json");
    assert_eq!(metadata["completeness"], "unverified");

    let mut html_args = common.to_vec();
    html_args.extend(["--format", "html", "--output", html_s]);
    assert_eq!(run(home.path(), &html_args, "").0, 0);

    let report: Value = serde_json::from_slice(&std::fs::read(&native).unwrap()).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["sections"]["month_income"]["result"], "1234.50");
    assert_eq!(
        report["sections"]["year_to_date_income"]["result"],
        "1234.50"
    );
    assert_eq!(report["sources"].as_array().unwrap().len(), 1);
    let html_text = std::fs::read_to_string(&html).unwrap();
    assert!(html_text.contains("1 234,50 EUR"));
    assert!(html_text.contains("Fiktiivinen &lt;Yhtiö&gt; &amp; Kumppanit"));
    assert!(html_text.contains(report["provenance"]["source_set_hash"].as_str().unwrap()));

    let (code, _, error) = run(home.path(), &native_args, "");
    assert_eq!(code, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&error).unwrap()["error_code"],
        "OUTPUT_EXISTS"
    );
    let after: Value =
        serde_json::from_str(&run(home.path(), &["voucher", "verify"], "").1).unwrap();
    assert_eq!(before["head_hash"], after["head_hash"]);
    assert_eq!(before["voucher_count"], after["voucher_count"]);
}

#[test]
fn report_cli_rejects_quarters_and_requires_explicit_format() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(run(home.path(), &["init"], "").0, 0);
    let output = home.path().join("x.json");
    let (code, _, error) = run(
        home.path(),
        &[
            "report",
            "statements",
            "--period",
            "2026-Q3",
            "--company",
            "Test Oy",
            "--format",
            "native-json",
            "--output",
            output.to_str().unwrap(),
        ],
        "",
    );
    assert_eq!(code, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&error).unwrap()["error_code"],
        "INVALID_PERIOD_ID"
    );

    let (code, _, error) = run(
        home.path(),
        &[
            "report",
            "statements",
            "--period",
            "2026-07",
            "--company",
            "Test Oy",
            "--output",
            output.to_str().unwrap(),
        ],
        "",
    );
    assert_eq!(code, 1);
    assert!(error.contains("--format"));
}
