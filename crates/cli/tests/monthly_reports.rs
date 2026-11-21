mod common;

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
    common::feed_stdin(&mut child, stdin.as_bytes());
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

fn run_text(home: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let output = bin().arg("--home").arg(home).args(args).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn report_statements_defaults_to_native_json_on_stdout() {
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
    let base = [
        "report",
        "statements",
        "--period",
        "2026-07",
        "--company",
        "Esimerkki Oy",
    ];
    let file = home.path().join("july.json");
    let mut file_args = base.to_vec();
    file_args.extend([
        "--format",
        "native-json",
        "--output",
        file.to_str().unwrap(),
    ]);
    assert_eq!(run(home.path(), &file_args, "").0, 0);
    let file_bytes = std::fs::read_to_string(&file).unwrap();

    // Text mode without --output: stdout is the report document, byte-identical
    // to the file form.
    let (code, stdout, stderr) = run_text(home.path(), &base);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, file_bytes);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["sections"]["month_income"]["result"], "1234.50");

    // `--output -` is the same as omitting it.
    let mut dash = base.to_vec();
    dash.extend(["--output", "-"]);
    let (code, stdout, _) = run_text(home.path(), &dash);
    assert_eq!(code, 0);
    assert_eq!(stdout, file_bytes);
    assert!(!home.path().join("-").exists());

    // Explicit html to stdout.
    let mut html = base.to_vec();
    html.extend(["--format", "html"]);
    let (code, stdout, _) = run_text(home.path(), &html);
    assert_eq!(code, 0);
    assert!(stdout.contains("1 234,50 EUR"));

    // --json keeps one JSON object on stdout with the document in `content`.
    let (code, stdout, stderr) = run(home.path(), &base, "");
    assert_eq!(code, 0, "{stderr}");
    let envelope: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(envelope["output"], "-");
    assert_eq!(envelope["format"], "native-json");
    assert_eq!(envelope["content"], file_bytes.as_str());
    assert_eq!(envelope["bytes"], file_bytes.len());

    // The file form's summary shape is unchanged: no `content`.
    let other = home.path().join("again.json");
    let mut again = base.to_vec();
    again.extend([
        "--format",
        "native-json",
        "--output",
        other.to_str().unwrap(),
    ]);
    let summary: Value = serde_json::from_str(&run(home.path(), &again, "").1).unwrap();
    assert!(summary.get("content").is_none());

    let (code, help, _) = run_text(home.path(), &["report", "statements", "--help"]);
    assert_eq!(code, 0);
    assert!(help.contains("Quick look"));
}
