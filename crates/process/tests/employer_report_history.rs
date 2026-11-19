//! Employer-register start from YTJ and months reported outside aggountant from
//! Incomes Register transcript 630. No test touches the network: YTJ answers
//! are recorded synthetic JSON behind the `YtjLookup` seam.
use aggountant_broker::ytj::{parse_companies, RegisterEntry, YtjError, YtjLookup};
use aggountant_domain::payroll::{MonthPayments, ReportMonth};
use aggountant_ledger::Ledger;
use aggountant_model::BusinessId;
use aggountant_process::{
    EmployerMonthState, EmployerReportInput, EmployerReportReceipt, ProcessEngine,
    RegistrationCheck, RegistrationComparison, RegistrationVerdict, TranscriptConflictKind,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use std::path::{Path, PathBuf};

const IR_REPORT: &str = "22222222222233334444555555555555";

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/employer_report")
        .join(name);
    std::fs::read(path).unwrap()
}

/// A recorded YTJ answer, or a network failure.
enum Recorded {
    Body(&'static str),
    Down,
}
impl YtjLookup for Recorded {
    fn register_entries(&self, id: &BusinessId) -> Result<Option<Vec<RegisterEntry>>, YtjError> {
        match self {
            Recorded::Body(name) => parse_companies(&fixture(name), id),
            Recorded::Down => Err(YtjError::Unavailable("connection refused".into())),
        }
    }
}

fn home(employer_report: Option<serde_json::Value>) -> tempfile::TempDir {
    let h = tempfile::tempdir().unwrap();
    Ledger::init(h.path()).unwrap();
    std::fs::create_dir_all(h.path().join(".aggountant")).unwrap();
    let mut profile = json!({"schema_version":1,"mode":"production","business_id":"1234567-1",
        "reporting_permission_active":true,"credentials_file":null});
    if let Some(er) = employer_report {
        profile["employer_report"] = er;
    }
    std::fs::write(
        h.path().join(".aggountant/tulorekisteri.json"),
        profile.to_string(),
    )
    .unwrap();
    h
}
fn settings(registered_from: &str) -> serde_json::Value {
    json!({"registered_from":registered_from,"contact_name":"Fictional Contact",
        "contact_phone":"+358 40 1234567","pension_insurances":[]})
}
fn month(s: &str) -> ReportMonth {
    ReportMonth::parse(s).unwrap()
}
fn months(list: &[&str]) -> Vec<ReportMonth> {
    list.iter().map(|m| month(m)).collect()
}
fn day(s: &str) -> NaiveDate {
    s.parse().unwrap()
}
fn t(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}
fn manual(e: &ProcessEngine, m: &str) {
    e.complete_employer_report_manual(
        month(m),
        EmployerReportReceipt {
            ir_delivery_id: None,
            ir_report_id: IR_REPORT.into(),
            version: Some(1),
            source: "e_service".into(),
        },
        t("2026-03-01T10:00:00Z"),
    )
    .unwrap();
}
fn files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    walk(dir)
}
fn walk(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p)
            } else {
                1
            }
        })
        .sum()
}

#[test]
fn registration_compares_the_current_employer_entry_with_the_profile() {
    let h = home(Some(settings("2026-07")));
    let e = ProcessEngine::open(h.path());
    let before = files(h.path());
    let r = e
        .employer_registration(
            &Recorded::Body("ytj-several-employer-entries.json"),
            day("2026-11-18"),
        )
        .unwrap();
    assert_eq!(
        r.comparison,
        RegistrationComparison::ProfileLaterThanRegister
    );
    assert_eq!(r.implied_registered_from, Some(month("2020-02")));
    assert_eq!(r.profile_registered_from, Some(month("2026-07")));
    assert_eq!(r.hidden_months, 77);
    assert_eq!(r.current.as_ref().unwrap().from, day("2020-02-01"));
    assert_eq!(
        r.current.as_ref().unwrap().status.as_deref(),
        Some("Rekisterissä")
    );
    assert_eq!(
        r.periods.len(),
        2,
        "the earlier, ended period is reported too"
    );
    assert_eq!(r.periods[0].to, Some(day("2018-06-30")));
    assert_eq!(r.ignored_entries.len(), 1, "7_99 is an erroneous line");
    assert_eq!(files(h.path()), before, "the check writes nothing");

    let ended = e
        .employer_registration(
            &Recorded::Body("ytj-employer-entry-ended.json"),
            day("2026-11-18"),
        )
        .unwrap();
    assert_eq!(ended.comparison, RegistrationComparison::RegisterEnded);
    assert_eq!(ended.implied_registered_from, None);

    let none = e
        .employer_registration(
            &Recorded::Body("ytj-not-in-employer-register.json"),
            day("2026-11-18"),
        )
        .unwrap();
    assert_eq!(none.comparison, RegistrationComparison::NotInRegister);
    assert!(none.periods.is_empty());

    let unknown = e
        .employer_registration(
            &Recorded::Body("ytj-unknown-company.json"),
            day("2026-11-18"),
        )
        .unwrap_err();
    assert_eq!(unknown.error_code(), "YTJ_COMPANY_NOT_FOUND");
}

#[test]
fn a_network_failure_is_never_read_as_not_registered() {
    let h = home(None);
    let e = ProcessEngine::open(h.path());
    let err = e
        .employer_registration(&Recorded::Down, day("2026-11-18"))
        .unwrap_err();
    assert_eq!(err.error_code(), "YTJ_UNAVAILABLE");
    // Without employer_report the check still answers what to set.
    let r = e
        .employer_registration(
            &Recorded::Body("ytj-several-employer-entries.json"),
            day("2026-11-18"),
        )
        .unwrap();
    assert_eq!(r.comparison, RegistrationComparison::ProfileNotSet);
    assert_eq!(r.implied_registered_from, Some(month("2020-02")));
}

#[test]
fn transcript_months_leave_missing_with_their_evidence_and_held_months_are_kept() {
    let h = home(Some(settings("2025-01")));
    let e = ProcessEngine::open(h.path());
    // aggountant already holds March (reported in the transcript too) and June
    // (which the transcript shows as unreported: a conflict).
    manual(&e, "2025-03");
    manual(&e, "2025-06");
    let csv = fixture("transcript-630-2025.csv");

    let dry = e
        .import_employer_transcript(&csv, t("2026-03-10T09:00:00Z"), true)
        .unwrap();
    assert!(dry.dry_run);
    assert!(!h
        .path()
        .join("process/employer-report-transcripts")
        .exists());
    let artifacts = files(&h.path().join("artifacts"));

    let imported = e
        .import_employer_transcript(&csv, t("2026-03-10T09:00:00Z"), false)
        .unwrap();
    assert!(!imported.already_imported);
    assert_eq!(imported.covers_from, month("2025-01"));
    assert_eq!(imported.covers_to, month("2025-12"));
    assert_eq!(imported.encoding, "utf-8");
    assert_eq!(
        imported.recorded,
        months(&[
            "2025-01", "2025-02", "2025-05", "2025-07", "2025-08", "2025-09", "2025-10", "2025-11",
            "2025-12"
        ])
    );
    assert_eq!(imported.already_held.len(), 1);
    assert_eq!(imported.already_held[0].month, month("2025-03"));
    assert_eq!(imported.already_held[0].source, "e_service");
    assert_eq!(imported.not_reported, months(&["2025-04"]));
    assert_eq!(imported.conflicts.len(), 1);
    assert_eq!(imported.conflicts[0].month, month("2025-06"));
    assert_eq!(
        imported.conflicts[0].kind,
        TranscriptConflictKind::AcceptedInAggountantNoneInTranscript
    );
    assert_eq!(
        files(&h.path().join("artifacts")),
        artifacts + 1,
        "the file is kept as an artifact"
    );

    let missing = e.missing_employer_reports(day("2026-03-10")).unwrap();
    let listed: Vec<ReportMonth> = missing.months.iter().map(|m| m.month).collect();
    assert_eq!(listed, months(&["2025-04", "2026-01", "2026-02"]));
    assert_eq!(missing.filed_outside.len(), 9);
    let jan = &missing.filed_outside[0];
    assert_eq!(jan.month, month("2025-01"));
    assert_eq!(jan.evidence.source, "incomes_register_transcript_630");
    assert_eq!(jan.evidence.artifact, imported.artifact);
    assert_eq!(jan.evidence.report_count, 1);
    // March stays aggountant's own accepted month, not transcript evidence.
    assert!(missing
        .filed_outside
        .iter()
        .all(|m| m.month != month("2025-03")));
    let march = e.employer_report_status(month("2025-03")).unwrap();
    assert_eq!(march.state, EmployerMonthState::Accepted);

    // The same file again changes nothing.
    let store = files(&h.path().join("process"));
    let again = e
        .import_employer_transcript(&csv, t("2026-03-11T09:00:00Z"), false)
        .unwrap();
    assert!(again.already_imported);
    assert_eq!(again.imported_at, imported.imported_at);
    assert_eq!(again.recorded, imported.recorded);
    assert_eq!(files(&h.path().join("process")), store);
    assert_eq!(files(&h.path().join("artifacts")), artifacts + 1);
}

#[test]
fn a_month_known_only_from_a_transcript_is_neither_resent_nor_replaced() {
    let h = home(Some(settings("2025-01")));
    let e = ProcessEngine::open(h.path());
    e.import_employer_transcript(
        &fixture("transcript-630-2025.csv"),
        t("2026-03-10T09:00:00Z"),
        false,
    )
    .unwrap();
    let at = t("2026-03-10T10:00:00Z");
    let input = |replace| EmployerReportInput {
        month: month("2025-01"),
        payments: MonthPayments::None,
        replace,
    };
    let status = e.employer_report_status(month("2025-01")).unwrap();
    assert_eq!(status.state, EmployerMonthState::FiledOutside);
    assert_eq!(status.transcript.as_ref().unwrap().report_count, 1);
    let new = e.preview_employer_report(input(false), at).unwrap_err();
    assert_eq!(new.error_code(), "EMPLOYER_REPORT_FILED_OUTSIDE");
    let replace = e.preview_employer_report(input(true), at).unwrap_err();
    assert_eq!(replace.error_code(), "EMPLOYER_REPORT_NO_REFERENCE");
    assert!(replace.to_string().contains("e-service"));
    // A month the transcript shows unreported can still be previewed.
    let april = EmployerReportInput {
        month: month("2025-04"),
        ..input(false)
    };
    assert!(e.preview_employer_report(april, at).is_ok());

    // Recording the e-service reference turns it into a month aggountant can replace.
    manual(&e, "2025-01");
    let status = e.employer_report_status(month("2025-01")).unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    assert!(e.preview_employer_report(input(true), at).is_ok());
}

#[test]
fn transcripts_that_disagree_make_a_visible_conflict_whatever_the_import_order() {
    // The 2025 transcript counts a February report, the first-half one counts
    // none. Neither order may silently decide: February is listed, and no
    // report can be sent for it until the e-service shows which is true.
    let full = fixture("transcript-630-2025.csv");
    let half = fixture("transcript-630-2025-h1-later.csv");
    for (first, second) in [(&full, &half), (&half, &full)] {
        let h = home(Some(settings("2025-01")));
        let e = ProcessEngine::open(h.path());
        e.import_employer_transcript(first, t("2026-03-10T09:00:00Z"), false)
            .unwrap();
        let later = e
            .import_employer_transcript(second, t("2026-04-02T09:00:00Z"), false)
            .unwrap();
        assert_eq!(later.conflicts.len(), 1);
        assert_eq!(later.conflicts[0].month, month("2025-02"));
        assert_eq!(
            later.conflicts[0].kind,
            TranscriptConflictKind::TranscriptsDisagree
        );
        assert!(!later.recorded.contains(&month("2025-02")));
        assert!(!later.not_reported.contains(&month("2025-02")));

        let missing = e.missing_employer_reports(day("2026-04-02")).unwrap();
        let listed: Vec<ReportMonth> = missing.months.iter().map(|m| m.month).collect();
        assert_eq!(
            listed,
            months(&["2025-02", "2025-04", "2025-06", "2026-01", "2026-02", "2026-03"])
        );
        let feb = &missing.months[0];
        assert_eq!(feb.state, EmployerMonthState::TranscriptConflict);
        assert_eq!(feb.transcript.as_ref().unwrap().contradicted_by.len(), 1);
        // July is outside the first-half transcript, so it stays filed outside.
        assert!(missing
            .filed_outside
            .iter()
            .any(|m| m.month == month("2025-07")));

        let preview = e
            .preview_employer_report(
                EmployerReportInput {
                    month: month("2025-02"),
                    payments: MonthPayments::None,
                    replace: false,
                },
                t("2026-04-02T10:00:00Z"),
            )
            .unwrap_err();
        assert_eq!(preview.error_code(), "EMPLOYER_REPORT_TRANSCRIPT_CONFLICT");

        // The e-service reference settles it.
        manual(&e, "2025-02");
        let feb = e.employer_report_status(month("2025-02")).unwrap();
        assert_eq!(feb.state, EmployerMonthState::Accepted);
    }
}

#[test]
fn an_unknown_header_is_refused_and_nothing_is_written() {
    let h = home(Some(settings("2025-01")));
    let e = ProcessEngine::open(h.path());
    let artifacts = files(&h.path().join("artifacts"));
    let english = "\u{feff}Type of report;Period;Time range\r\nEmployer's separate report;;\r\n";
    let err = e
        .import_employer_transcript(english.as_bytes(), t("2026-03-10T09:00:00Z"), false)
        .unwrap_err();
    assert_eq!(err.error_code(), "TRANSCRIPT_UNKNOWN_HEADER");
    assert!(err.to_string().contains("Finnish"));
    assert_eq!(files(&h.path().join("artifacts")), artifacts);
    assert!(!h.path().join("process").exists());
}

fn profile_settings(
    registered_from: &str,
) -> aggountant_broker::tulorekisteri::EmployerReportProfile {
    serde_json::from_value(settings(registered_from)).unwrap()
}
fn profile_bytes(h: &tempfile::TempDir) -> String {
    std::fs::read_to_string(h.path().join(".aggountant/tulorekisteri.json")).unwrap()
}

#[test]
fn configure_refuses_a_registered_from_the_register_disagrees_with() {
    let h = home(None);
    let before = profile_bytes(&h);
    let e = ProcessEngine::open(h.path());
    let ytj = Recorded::Body("ytj-several-employer-entries.json");
    let online = || RegistrationCheck::Online {
        lookup: &ytj,
        today: day("2026-11-18"),
    };

    let err = e
        .configure_employer_report(profile_settings("2026-07"), online(), false, false)
        .unwrap_err();
    assert_eq!(err.error_code(), "EMPLOYER_REGISTER_MISMATCH");
    assert!(err.to_string().contains("77 months"), "{err}");
    assert!(err.to_string().contains("2020-02"), "{err}");
    assert_eq!(profile_bytes(&h), before, "a refusal writes nothing");

    // The register's month is accepted; a dry run still writes nothing.
    let dry = e
        .configure_employer_report(profile_settings("2020-02"), online(), false, true)
        .unwrap();
    assert!(dry.dry_run && dry.changed && !dry.written);
    assert_eq!(dry.registration_verdict, RegistrationVerdict::Match);
    assert!(dry.before.is_none());
    assert_eq!(profile_bytes(&h), before);

    let done = e
        .configure_employer_report(profile_settings("2020-02"), online(), false, false)
        .unwrap();
    assert!(done.written);
    let after = profile_bytes(&h);
    assert!(
        after.starts_with(before.trim_end_matches('}')),
        "other bytes are kept"
    );
    let r = e.employer_registration(&ytj, day("2026-11-18")).unwrap();
    assert_eq!(r.comparison, RegistrationComparison::Match);

    // The same settings again change nothing.
    let again = e
        .configure_employer_report(profile_settings("2020-02"), online(), false, false)
        .unwrap();
    assert!(!again.changed && !again.written);
    assert_eq!(again.before.unwrap().registered_from, month("2020-02"));

    // A disagreement on the user's say-so is written, with the comparison.
    let accepted = e
        .configure_employer_report(profile_settings("2026-07"), online(), true, false)
        .unwrap();
    assert_eq!(
        accepted.registration_verdict,
        RegistrationVerdict::MismatchAccepted
    );
    assert_eq!(
        accepted.registration.unwrap().comparison,
        RegistrationComparison::ProfileLaterThanRegister
    );
    assert_eq!(
        e.missing_employer_reports(day("2026-11-18"))
            .unwrap()
            .registered_from,
        month("2026-07")
    );
}

#[test]
fn configure_needs_offline_explicitly_when_ytj_fails_and_validates_settings() {
    let h = home(None);
    let before = profile_bytes(&h);
    let e = ProcessEngine::open(h.path());
    let down = RegistrationCheck::Online {
        lookup: &Recorded::Down,
        today: day("2026-11-18"),
    };
    let err = e
        .configure_employer_report(profile_settings("2026-07"), down, false, false)
        .unwrap_err();
    assert_eq!(err.error_code(), "YTJ_UNAVAILABLE");
    assert!(err.to_string().contains("--offline"), "{err}");
    assert_eq!(profile_bytes(&h), before);

    // Not in the register at all is a disagreement too.
    let none = RegistrationCheck::Online {
        lookup: &Recorded::Body("ytj-not-in-employer-register.json"),
        today: day("2026-11-18"),
    };
    let err = e
        .configure_employer_report(profile_settings("2026-07"), none, false, false)
        .unwrap_err();
    assert_eq!(err.error_code(), "EMPLOYER_REGISTER_MISMATCH");

    let mut bad = profile_settings("2026-07");
    bad.pension_insurances
        .push(aggountant_domain::payroll::PensionInsurance {
            provider_code: 46,
            policy_number: "far too long policy".into(),
        });
    let err = e
        .configure_employer_report(bad, RegistrationCheck::Offline, false, false)
        .unwrap_err();
    assert_eq!(err.error_code(), "EMPLOYER_REPORT_INVALID_SETTINGS");

    let offline = e
        .configure_employer_report(
            profile_settings("2026-07"),
            RegistrationCheck::Offline,
            false,
            false,
        )
        .unwrap();
    assert_eq!(
        offline.registration_verdict,
        RegistrationVerdict::NotChecked
    );
    assert!(offline.registration.is_none() && offline.written);
}
