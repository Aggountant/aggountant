//! Fiscal years through the built binary (issue `non-calendar-fiscal-year`): a
//! synthetic July–June company (Esimerkki Oy, made-up figures) books, closes months,
//! reports year to date and records opening balances; the 18-month rule; and a home
//! written by the `main` binary before fiscal-year definitions keeps working.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

fn run(home: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aggountant");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn ok(home: &Path, args: &[&str], stdin: &str) -> Value {
    let (code, stdout, stderr) = run(home, args, stdin);
    assert_eq!(code, 0, "{args:?}: {stderr}");
    serde_json::from_str(&stdout).unwrap()
}

fn error_code(home: &Path, args: &[&str], stdin: &str) -> (i32, Value) {
    let (code, _, stderr) = run(home, args, stdin);
    (code, serde_json::from_str(&stderr).unwrap())
}

fn seal_input(date: &str, key: &str, lines: &[(&str, &str, &str)]) -> String {
    let postings: Vec<String> = lines
        .iter()
        .map(|(account, amount, side)| {
            format!(
                r#"{{"account":"{account}","amount":{{"amount":"{amount}","currency":"EUR"}},"side":"{side}"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"draft":{{"document_date":"{date}","posting_date":"{date}","description":"Esimerkki Oy {key}","postings":[{}]}},"natural_key":"{key}","actor":"test"}}"#,
        postings.join(",")
    )
}

fn seal(home: &Path, date: &str, key: &str, lines: &[(&str, &str, &str)]) {
    ok(home, &["voucher", "seal"], &seal_input(date, key, lines));
}

fn close_month(home: &Path, period: &str) {
    let started = ok(
        home,
        &["run", "start", "close-period", "--period", period],
        "",
    );
    let run_id = started["run_id"].as_str().unwrap().to_string();
    ok(
        home,
        &[
            "run",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveClose",
            "--answer",
            "approve",
        ],
        "",
    );
    let view = ok(home, &["period", "show", "--period", period], "");
    assert_eq!(view["state"], "sealed", "{period}: {view}");
}

fn report(home: &Path, period: &str) -> Value {
    let path = home.join(format!("report-{period}.json"));
    ok(
        home,
        &[
            "report",
            "statements",
            "--period",
            period,
            "--company",
            "Esimerkki Oy",
            "--format",
            "native-json",
            "--output",
            path.to_str().unwrap(),
        ],
        "",
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn a_july_to_june_company_books_closes_reports_and_opens() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    ok(home, &["init"], "");
    assert_eq!(
        ok(home, &["fiscal-year", "list"], "")["calendar_years"],
        true
    );

    // Esimerkki Oy moves to aggountant on 1.3.2027, in the middle of its fiscal year
    // 1.7.2026–30.6.2027.
    let defined = ok(
        home,
        &[
            "fiscal-year",
            "define",
            "--start",
            "2026-07-01",
            "--end",
            "2027-06-30",
        ],
        "",
    );
    assert_eq!(defined["year"], 2027);
    assert_eq!(defined["next_start"], "2027-07-01");
    let listed = ok(home, &["fiscal-year", "list"], "");
    assert_eq!(listed["calendar_years"], false);
    assert_eq!(listed["definitions"][0]["start"], "2026-07-01");

    // Opening balances: the old system's balances on its last day, 28.2.2027, with the
    // fiscal year's result so far (sales 3 000, costs 1 200) in them.
    seal(
        home,
        "2027-02-28",
        "opening",
        &[
            ("1910", "4300.00", "debit"),
            ("7690", "1200.00", "debit"),
            ("2000", "2500.00", "credit"),
            ("3000", "3000.00", "credit"),
        ],
    );
    seal(
        home,
        "2027-03-10",
        "sale-march",
        &[("1910", "500.00", "debit"), ("3000", "500.00", "credit")],
    );
    seal(
        home,
        "2027-07-01",
        "sale-next-year",
        &[("1910", "999.00", "debit"), ("3000", "999.00", "credit")],
    );

    let show = ok(home, &["fiscal-year", "show", "--date", "2027-03-10"], "");
    assert_eq!(show["year"], 2027);
    assert_eq!(show["start"], "2026-07-01");
    assert_eq!(show["end"], "2027-06-30");
    assert_eq!(show["state"], "open");

    close_month(home, "2027-02");
    close_month(home, "2027-03");

    let february = report(home, "2027-02");
    assert_eq!(
        february["sections"]["year_to_date_income"]["result"],
        "1800.00"
    );
    let march = report(home, "2027-03");
    assert_eq!(march["sections"]["month_income"]["result"], "500.00");
    // Year to date from 1.7.2026, not from 1.1.2027; the July 2027 sale is next year.
    assert_eq!(
        march["sections"]["year_to_date_income"]["result"],
        "2300.00"
    );
    assert_eq!(
        march["sections"]["year_to_date_income"]["title"],
        "Tuloslaskelma, tilikauden alusta"
    );

    // A voucher before the first fiscal year has no year to belong to.
    let (code, err) = error_code(
        home,
        &["voucher", "seal"],
        &seal_input(
            "2026-06-30",
            "too-early",
            &[("1910", "1.00", "debit"), ("2000", "1.00", "credit")],
        ),
    );
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "FISCAL_YEAR_UNDEFINED");

    assert_eq!(ok(home, &["voucher", "verify"], "")["ok"], true);
}

#[test]
fn first_year_length_follows_kirjanpitolaki() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    ok(home, &["init"], "");
    let (code, err) = error_code(
        home,
        &[
            "fiscal-year",
            "define",
            "--start",
            "2026-07-01",
            "--end",
            "2028-01-31",
        ],
        "",
    );
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "INVALID_FISCAL_YEAR");
    let message = err["message"].as_str().unwrap();
    assert!(message.contains("19 months"), "{message}");
    assert!(message.contains("kirjanpitolaki 1:4"), "{message}");

    let defined = ok(
        home,
        &[
            "fiscal-year",
            "define",
            "--start",
            "2026-07-01",
            "--end",
            "2027-12-31",
        ],
        "",
    );
    assert_eq!(defined["months"], 18);
    assert_eq!(defined["next_start"], "2028-01-01");
    let replay = ok(
        home,
        &[
            "fiscal-year",
            "define",
            "--start",
            "2026-07-01",
            "--end",
            "2027-12-31",
        ],
        "",
    );
    assert_eq!(replay["replayed"], true);
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn a_home_written_by_the_main_binary_keeps_calendar_years() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/main-home-calendar-2026");
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    copy_dir(&fixture, &home);
    let home = home.as_path();

    let verify = ok(home, &["voucher", "verify"], "");
    assert_eq!(verify["ok"], true);
    assert_eq!(verify["voucher_count"], 3);
    assert_eq!(
        ok(home, &["fiscal-year", "list"], "")["calendar_years"],
        true
    );
    let year = ok(home, &["fiscal-year", "show", "--year", "2026"], "");
    assert_eq!(year["start"], "2026-01-01");
    assert_eq!(year["end"], "2026-12-31");
    assert_eq!(year["months"], 12);
    assert_eq!(
        ok(home, &["period", "show", "--period", "2026-01"], "")["state"],
        "sealed"
    );

    // Books on as before: a new voucher, a month close and a year-to-date report.
    seal(
        home,
        "2026-02-20",
        "after-upgrade",
        &[("7690", "30.00", "debit"), ("1910", "30.00", "credit")],
    );
    close_month(home, "2026-02");
    let february = report(home, "2026-02");
    assert_eq!(february["sections"]["month_income"]["result"], "370.00");
    assert_eq!(
        february["sections"]["year_to_date_income"]["result"],
        "346.00"
    );
    assert_eq!(ok(home, &["voucher", "verify"], "")["voucher_count"], 4);
}
