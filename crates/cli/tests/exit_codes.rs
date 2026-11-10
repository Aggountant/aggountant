//! Canon §2 exit-code contract across the complete clap command surface.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn invoke(args: &[&str]) -> Output {
    invoke_with_stdin(args, &[])
}

fn invoke_with_stdin(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(args)
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run aggountant");
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().unwrap()
}

/// Every command group and executable leaf. Keep this inventory in sync with
/// `Command` and its nested subcommand enums so parser exit behavior cannot
/// silently diverge on an overlooked branch.
const COMMAND_PATHS: &[&[&str]] = &[
    &["config"],
    &["config", "path"],
    &["config", "show"],
    &["skill"],
    &["skill", "list"],
    &["skill", "install"],
    &["skill", "print"],
    &["init"],
    &["chart"],
    &["chart", "list"],
    &["voucher"],
    &["voucher", "draft"],
    &["voucher", "validate"],
    &["voucher", "seal"],
    &["voucher", "verify"],
    &["artifact"],
    &["artifact", "attach"],
    &["period"],
    &["period", "show"],
    &["run"],
    &["run", "start"],
    &["run", "show"],
    &["run", "answer"],
    &["run", "resume"],
    &["run", "list"],
    &["run", "migrate"],
    &["file-vat"],
    &["file-vat", "preview"],
    &["file-vat", "complete-manual"],
    &["file-vat", "start"],
    &["file-vat", "approve-filing"],
    &["file-vat", "approve-payment"],
    &["file-vat", "reconcile"],
    &["file-vat", "retry"],
    &["employee"],
    &["employee", "add"],
    &["employee", "show"],
    &["employee", "list"],
    &["run-payroll"],
    &["run-payroll", "start"],
    &["run-payroll", "approve-run"],
    &["run-payroll", "resolve-errors"],
    &["run-payroll", "handle-payment-failure"],
    &["run-payroll", "resume"],
    &["capture-receipt"],
    &["capture-receipt", "start"],
    &["capture-receipt", "answer"],
    &["capture-receipt", "resume"],
    &["hunt-receipts"],
    &["hunt-receipts", "start"],
    &["hunt-receipts", "answer"],
    &["hunt-receipts", "resume"],
    &["obligation"],
    &["obligation", "list"],
    &["obligation", "resolve"],
    &["knowledge"],
    &["knowledge", "list"],
    &["knowledge", "show"],
    &["intake"],
    &["intake", "list"],
    &["close-year"],
    &["close-year", "start"],
    &["close-year", "answer"],
    &["close-year", "seal-correction"],
    &["close-year", "resume"],
    &["fiscal-year"],
    &["fiscal-year", "show"],
    &["import"],
    &["import", "camt053"],
    &["import", "camt053", "parse"],
    &["import", "camt053", "seal"],
    // The public compatibility alias is part of the parser surface too.
    &["import", "camt052"],
    &["import", "camt052", "parse"],
    &["import", "camt052", "seal"],
];

#[test]
fn help_is_successful_on_every_command_path_and_root_version_is_successful() {
    for path in COMMAND_PATHS {
        let mut args = path.to_vec();
        args.push("--help");
        let output = invoke(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "help failed for {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    for path in COMMAND_PATHS {
        let mut args = vec!["help"];
        args.extend_from_slice(path);
        let output = invoke(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "help subcommand failed for {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    for args in [["--help"].as_slice(), ["--version"].as_slice()] {
        let output = invoke(args);
        assert_eq!(output.status.code(), Some(0), "display failed for {args:?}");
    }
}

#[test]
fn retired_period_mark_vat_filed_is_not_advertised_or_accepted() {
    let help = invoke(&["period", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&help.stdout).contains("mark-vat-filed"));

    let retired = invoke(&["period", "mark-vat-filed", "--period", "2026-Q1"]);
    assert_eq!(retired.status.code(), Some(1));
    assert!(retired.stdout.is_empty());
    assert!(String::from_utf8_lossy(&retired.stderr).contains("unrecognized subcommand"));
}

#[test]
fn usage_errors_exit_one_on_every_command_path() {
    let output = invoke(&[]);
    assert_eq!(output.status.code(), Some(1), "missing command is usage");

    let output = invoke(&["not-a-command"]);
    assert_eq!(output.status.code(), Some(1), "unknown command is usage");

    for path in COMMAND_PATHS {
        let mut args = path.to_vec();
        args.push("--definitely-not-a-real-option");
        let output = invoke(&args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "usage error had wrong exit for {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "usage error wrote data to stdout for {args:?}"
        );
    }
}

#[test]
fn domain_not_found_and_incomplete_vero_setup_are_caller_actionable() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path().to_str().unwrap();

    let not_found = invoke(&["--home", home, "run", "show", "--run-id", "does-not-exist"]);
    assert_eq!(not_found.status.code(), Some(1));
    assert!(not_found.stdout.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&not_found.stderr).unwrap();
    assert_eq!(payload["error_code"], "RUN_NOT_FOUND");
    assert_eq!(payload["error_kind"], "user");

    let dependency = invoke_with_stdin(
        &["--home", home, "file-vat", "start"],
        br#"{
          "period":"2026-Q1",
          "sales_vat":{"amount":"0.00","currency":"EUR"},
          "purchase_vat":{"amount":"0.00","currency":"EUR"},
          "payee_iban":"FI2112345600000785"
        }"#,
    );
    assert_eq!(dependency.status.code(), Some(1));
    assert!(dependency.stdout.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&dependency.stderr).unwrap();
    assert_eq!(payload["error_code"], "VERO_NOT_READY");
    assert_eq!(payload["error_kind"], "user");
}

#[test]
fn malformed_encoding_and_missing_input_file_are_caller_actionable() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path().to_str().unwrap();

    let invalid_utf8 = invoke_with_stdin(
        &["--home", home, "--json", "voucher", "draft"],
        &[0xff, 0xfe],
    );
    assert_eq!(invalid_utf8.status.code(), Some(1));
    let payload: serde_json::Value = serde_json::from_slice(&invalid_utf8.stderr).unwrap();
    assert_eq!(payload["error_code"], "INVALID_INPUT");
    assert_eq!(payload["error_kind"], "user");

    let missing = invoke(&[
        "--home",
        home,
        "--json",
        "artifact",
        "attach",
        "--file",
        "/no/such/aggountant-input.pdf",
    ]);
    assert_eq!(missing.status.code(), Some(1));
    let payload: serde_json::Value = serde_json::from_slice(&missing.stderr).unwrap();
    assert_eq!(payload["error_code"], "FILE_NOT_FOUND");
    assert_eq!(payload["error_kind"], "user");
}

#[test]
fn corrupt_process_journal_is_a_system_error() {
    let home = tempfile::tempdir().unwrap();
    let run_id = "corrupt-run";
    let events = home
        .path()
        .join("process")
        .join("runs")
        .join(run_id)
        .join("events");
    std::fs::create_dir_all(&events).unwrap();
    std::fs::write(events.join("000001.json"), b"not json").unwrap();

    let output = invoke(&[
        "--home",
        home.path().to_str().unwrap(),
        "--json",
        "run",
        "show",
        "--run-id",
        run_id,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(payload["error_code"], "JOURNAL_CORRUPT");
    assert_eq!(payload["error_kind"], "system");
}
