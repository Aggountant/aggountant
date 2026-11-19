//! What the employer's separate report needs to know about the past: when the
//! company entered the employer register (PRH/YTJ open data) and which months
//! were already reported outside aggountant (Incomes Register transcript 630).
//!
//! Neither writes the books. The registration check writes nothing at all; the
//! transcript import stores the file as an artifact and one evidence record in
//! the process store, and `missing` and `preview` read the evidence back.
//!
//! A transcript has no report references, so a month known only from it can
//! be neither sent again (it would be a second report) nor replaced from
//! aggountant (there is no reference to replace): both go to the e-service.
use crate::{
    file_employer_report::EmployerMonthState, transcript_630, ProcessEngine, ProcessError,
};
use aggountant_broker::{
    tulorekisteri::read_wage_profile,
    ytj::{RegisterEntry, YtjError, YtjLookup, YTJ_COMPANIES_URL},
};
use aggountant_domain::payroll::ReportMonth;
use aggountant_ledger::Ledger;
use aggountant_model::BusinessId;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

/// `REK` code of Työnantajarekisteri.
const EMPLOYER_REGISTER: &str = "7";
/// `REK_KDI` codes `7_41` and `7_42`: registered.
const REGISTERED_TYPES: [&str; 2] = ["41", "42"];
/// `REK_KDI` code `7_99`: an erroneous line, not a registration.
const ERRONEOUS_TYPE: &str = "99";
/// The first month the Incomes Register takes employer's separate reports.
const FIRST_REPORT_MONTH: &str = "2019-01";
const SOURCE_630: &str = "incomes_register_transcript_630";
const IMPORT_SCHEMA: u32 = 1;
/// The e-service delivers transcripts of at most 100 MB.
const MAX_TRANSCRIPT_BYTES: usize = 100 * 1024 * 1024;

fn refused(code: &'static str, message: impl Into<String>) -> ProcessError {
    ProcessError::EmployerReport {
        code,
        message: message.into(),
    }
}
fn store(e: impl std::fmt::Display) -> ProcessError {
    ProcessError::Store(e.to_string())
}
fn first_report_month() -> ReportMonth {
    ReportMonth::parse(FIRST_REPORT_MONTH).expect("valid")
}

// ---------------------------------------------------------------------------
// Registration from YTJ
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegisterPeriodState {
    Current,
    Ended,
    /// Starts after the check date.
    Future,
}

/// One employer-register period from YTJ.
#[derive(Debug, Clone, Serialize)]
pub struct RegisterPeriod {
    pub from: NaiveDate,
    pub to: Option<NaiveDate>,
    pub state: RegisterPeriodState,
    /// The register's own status text (Finnish when given).
    pub status: Option<String>,
    pub entry_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationComparison {
    /// The profile's `registered_from` is the current entry's first month.
    Match,
    /// The profile starts later than the register: months are hidden from `missing`.
    ProfileLaterThanRegister,
    /// The profile starts earlier than the register: `missing` lists months
    /// the register does not cover.
    ProfileEarlierThanRegister,
    /// The company was in the register but no entry covers the check date.
    RegisterEnded,
    /// The company is in YTJ but has no employer-register entry.
    NotInRegister,
    /// The company is in the register; the profile has no `employer_report` yet.
    ProfileNotSet,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataSource {
    pub name: &'static str,
    pub url: &'static str,
    pub license: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmployerRegistration {
    pub business_id: String,
    pub checked_on: NaiveDate,
    pub source: DataSource,
    pub comparison: RegistrationComparison,
    pub current: Option<RegisterPeriod>,
    /// Every recognised employer-register period, oldest first, the current one included.
    pub periods: Vec<RegisterPeriod>,
    /// Entries YTJ marks as erroneous lines (`7_99`), shown but not used.
    pub ignored_entries: Vec<RegisterPeriod>,
    pub profile_registered_from: Option<ReportMonth>,
    /// The `registered_from` the register implies: the current entry's first
    /// month, never before 2019-01. Written only on the user's say-so.
    pub implied_registered_from: Option<ReportMonth>,
    /// Months the profile hides from `missing` (profile later than register).
    pub hidden_months: u32,
    pub message: String,
}

fn months_between(from: ReportMonth, to: ReportMonth) -> u32 {
    let index = |m: ReportMonth| m.year() * 12 + m.month() as i32;
    (index(to) - index(from)).max(0) as u32
}

fn report_month_of(date: NaiveDate) -> ReportMonth {
    ReportMonth::of(date).max(first_report_month())
}

/// Classify YTJ entries for the employer register against the profile.
pub(crate) fn compare_registration(
    business_id: String,
    entries: Option<Vec<RegisterEntry>>,
    profile: Option<ReportMonth>,
    today: NaiveDate,
) -> Result<EmployerRegistration, ProcessError> {
    let Some(entries) = entries else {
        return Err(refused(
            "YTJ_COMPANY_NOT_FOUND",
            format!("YTJ knows no company with Business ID {business_id}; check business_id in .aggountant/tulorekisteri.json"),
        ));
    };
    let mut periods = Vec::new();
    let mut ignored_entries = Vec::new();
    for entry in entries
        .into_iter()
        .filter(|e| e.register == EMPLOYER_REGISTER)
    {
        let erroneous = entry.entry_type == ERRONEOUS_TYPE;
        if !erroneous && !REGISTERED_TYPES.contains(&entry.entry_type.as_str()) {
            return Err(refused(
                "YTJ_UNKNOWN_ENTRY_TYPE",
                format!(
                    "employer-register entry type {:?} ({}) is not a known REK_KDI code; check the company in ytj.fi",
                    entry.entry_type,
                    entry.status_text().unwrap_or("no description")
                ),
            ));
        }
        let Some(from) = entry.registration_date else {
            if erroneous {
                continue;
            }
            return Err(refused(
                "YTJ_RESPONSE_INVALID",
                "an employer-register entry has no registration date",
            ));
        };
        if entry.end_date.is_some_and(|end| end < from) {
            return Err(refused(
                "YTJ_RESPONSE_INVALID",
                format!("an employer-register entry ends before it starts ({from})"),
            ));
        }
        let state = if from > today {
            RegisterPeriodState::Future
        } else if entry.end_date.is_some_and(|end| end < today) {
            RegisterPeriodState::Ended
        } else {
            RegisterPeriodState::Current
        };
        let period = RegisterPeriod {
            from,
            to: entry.end_date,
            state,
            status: entry.status_text().map(str::to_string),
            entry_type: entry.entry_type.clone(),
        };
        if erroneous {
            ignored_entries.push(period);
        } else {
            periods.push(period);
        }
    }
    periods.sort_by_key(|p| p.from);
    let current: Vec<&RegisterPeriod> = periods
        .iter()
        .filter(|p| p.state == RegisterPeriodState::Current)
        .collect();
    if current.len() > 1 {
        return Err(refused(
            "YTJ_RESPONSE_INVALID",
            "several employer-register entries cover today; check the company in ytj.fi",
        ));
    }
    let current = current.first().map(|p| (*p).clone());
    let implied = current.as_ref().map(|p| report_month_of(p.from));
    let mut hidden_months = 0;
    let since = current.as_ref().map(|p| {
        if report_month_of(p.from) == ReportMonth::of(p.from) {
            format!("since {}", p.from)
        } else {
            format!("since {} (before the Incomes Register, so reports are due from {FIRST_REPORT_MONTH})", p.from)
        }
    });
    let since = since.unwrap_or_default();
    let (comparison, message) = match (&current, implied, profile) {
        (Some(_), Some(implied), None) => (
            RegistrationComparison::ProfileNotSet,
            format!("in the employer register {since}; employer_report is not in the profile yet, and registered_from would be {implied}"),
        ),
        (Some(_), Some(implied), Some(p)) if p == implied => (
            RegistrationComparison::Match,
            format!("registered_from {p} matches the employer register"),
        ),
        (Some(_), Some(implied), Some(p)) if p > implied => {
            hidden_months = months_between(implied, p);
            (
                RegistrationComparison::ProfileLaterThanRegister,
                format!("in the employer register {since}, but registered_from is {p}: {hidden_months} months are hidden from `missing`; set registered_from to {implied} if the user agrees"),
            )
        }
        (Some(_), Some(implied), Some(p)) => (
            RegistrationComparison::ProfileEarlierThanRegister,
            format!("registered_from {p} is earlier than the register entry ({implied}); `missing` lists months the current entry does not cover"),
        ),
        (None, _, _) if periods.iter().any(|p| p.state == RegisterPeriodState::Ended) => {
            let last = periods
                .iter()
                .filter_map(|p| p.to)
                .max()
                .expect("an ended period has an end date");
            (
                RegistrationComparison::RegisterEnded,
                format!("the employer-register entry ended on {last}; no monthly report is due after it unless the company registers again"),
            )
        }
        (None, _, _) if periods.is_empty() => (
            RegistrationComparison::NotInRegister,
            "the company has no employer-register entry in YTJ; no monthly report is due for months without payments".into(),
        ),
        (None, _, _) => (
            RegistrationComparison::NotInRegister,
            "the company's only employer-register entry starts in the future".into(),
        ),
        (Some(_), None, _) => unreachable!("a current period implies a month"),
    };
    Ok(EmployerRegistration {
        business_id,
        checked_on: today,
        source: DataSource {
            name: "PRH/YTJ open data",
            url: YTJ_COMPANIES_URL,
            license: "CC BY 4.0",
        },
        comparison,
        current,
        periods,
        ignored_entries,
        profile_registered_from: profile,
        implied_registered_from: implied,
        hidden_months,
        message,
    })
}

/// The company's YTJ register entries. A network failure means the
/// registration is unknown, never absent.
pub(crate) fn register_entries(
    lookup: &dyn YtjLookup,
    id: &BusinessId,
) -> Result<Option<Vec<RegisterEntry>>, ProcessError> {
    lookup.register_entries(id).map_err(|e| match e {
        YtjError::Unavailable(m) => refused(
            "YTJ_UNAVAILABLE",
            format!(
                "{m}; the registration is unknown, not absent. Try again later or check ytj.fi"
            ),
        ),
        YtjError::InvalidResponse(m) => refused("YTJ_RESPONSE_INVALID", m),
    })
}

impl ProcessEngine {
    /// Read-only: the employer-register entries YTJ holds for the profile's
    /// Business ID, compared with `registered_from`. Writes nothing.
    pub fn employer_registration(
        &self,
        lookup: &dyn YtjLookup,
        today: NaiveDate,
    ) -> Result<EmployerRegistration, ProcessError> {
        let profile = read_wage_profile(&self.home)
            .map_err(|e| refused("EMPLOYER_REPORT_NOT_CONFIGURED", e))?;
        compare_registration(
            profile.business_id.as_str().to_string(),
            register_entries(lookup, &profile.business_id)?,
            profile.employer_report.map(|e| e.registered_from),
            today,
        )
    }
}

// ---------------------------------------------------------------------------
// Transcript 630 import
// ---------------------------------------------------------------------------

/// One reporting period as the transcript states it. Amounts are decimal
/// strings with a point (`"12.34"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptMonthRecord {
    pub month: ReportMonth,
    pub report_count: u32,
    pub health_insurance_contribution: String,
    pub deductions: String,
}

/// The stored record of one imported transcript. Its name is the artifact hash,
/// so the same file imported twice is the same record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TranscriptImportRecord {
    schema_version: u32,
    artifact: String,
    imported_at: DateTime<Utc>,
    covers_from: ReportMonth,
    covers_to: ReportMonth,
    months: Vec<TranscriptMonthRecord>,
}

impl TranscriptImportRecord {
    fn covers(&self, month: ReportMonth) -> bool {
        self.covers_from <= month && month <= self.covers_to
    }
    fn count(&self, month: ReportMonth) -> u32 {
        self.months
            .iter()
            .find(|m| m.month == month)
            .map(|m| m.report_count)
            .unwrap_or(0)
    }
}

/// Evidence that the register holds a report for a month, from a transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranscriptEvidence {
    pub source: &'static str,
    /// SHA-256 of the stored transcript file (`artifacts/`).
    pub artifact: String,
    pub imported_at: DateTime<Utc>,
    pub covers_from: ReportMonth,
    pub covers_to: ReportMonth,
    pub report_count: u32,
    pub health_insurance_contribution: String,
    pub deductions: String,
    /// Other imported transcripts covering the month that count no report.
    /// Non-empty makes the month a conflict that only the e-service settles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contradicted_by: Vec<String>,
}

/// All imported transcripts, read once for a whole `missing` listing.
pub(crate) struct TranscriptImports(Vec<TranscriptImportRecord>);

impl TranscriptImports {
    /// Evidence exists when any imported transcript counts a report for the
    /// month. Imports are not ranked: a transcript carries no generation time,
    /// and one ordered with a reception-time range leaves out reports received
    /// outside it, so a zero never cancels a positive count. Transcripts that
    /// disagree are reported in `contradicted_by` instead, and the month is a
    /// conflict rather than either filed or missing.
    pub(crate) fn evidence(&self, month: ReportMonth) -> Option<TranscriptEvidence> {
        let covering = || self.0.iter().filter(|r| r.covers(month));
        let (record, count) = covering()
            .filter_map(|r| {
                let count = r.count(month);
                (count > 0).then_some((r, count))
            })
            .max_by(|(a, _), (b, _)| {
                (a.imported_at, &a.artifact).cmp(&(b.imported_at, &b.artifact))
            })?;
        let row = record
            .months
            .iter()
            .find(|m| m.month == month)
            .expect("a positive count has a row");
        Some(TranscriptEvidence {
            source: SOURCE_630,
            artifact: record.artifact.clone(),
            imported_at: record.imported_at,
            covers_from: record.covers_from,
            covers_to: record.covers_to,
            report_count: count,
            health_insurance_contribution: row.health_insurance_contribution.clone(),
            deductions: row.deductions.clone(),
            contradicted_by: covering()
                .filter(|r| r.count(month) == 0)
                .map(|r| r.artifact.clone())
                .collect(),
        })
    }

    /// Whether any import covering the month counts no report for it.
    fn counts_none(&self, month: ReportMonth) -> bool {
        self.0
            .iter()
            .any(|r| r.covers(month) && r.count(month) == 0)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HeldMonth {
    pub month: ReportMonth,
    /// `api` or `e_service`: the receipt aggountant already holds.
    pub source: String,
    pub report_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptConflictKind {
    /// aggountant holds an accepted report; the transcript counts none.
    AcceptedInAggountantNoneInTranscript,
    /// Transcripts disagree: one counts a report, another counts none. The
    /// order they were generated in is unknown and one may have been ordered
    /// with a reception-time range, so the month is a conflict until the
    /// e-service shows which is true.
    TranscriptsDisagree,
    /// aggountant sent a report whose outcome is unknown; the transcript counts
    /// one. It may be that report, so the month stays parked until `status`
    /// settles it.
    InDoubtSendTranscriptCountsReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptConflict {
    pub month: ReportMonth,
    pub kind: TranscriptConflictKind,
    pub report_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptImport {
    pub dry_run: bool,
    /// The same file was imported before; nothing changed.
    pub already_imported: bool,
    pub artifact: String,
    pub imported_at: DateTime<Utc>,
    pub covers_from: ReportMonth,
    pub covers_to: ReportMonth,
    pub separator: String,
    pub encoding: &'static str,
    pub months: Vec<TranscriptMonthRecord>,
    /// Months now recorded as reported outside aggountant.
    pub recorded: Vec<ReportMonth>,
    /// Months the transcript counts that aggountant already holds; not overwritten.
    pub already_held: Vec<HeldMonth>,
    /// Covered months with no report in the transcript and no other evidence;
    /// they stay missing.
    pub not_reported: Vec<ReportMonth>,
    /// Months to show the user; a `transcripts_disagree` month is listed by
    /// `missing` as `transcript_conflict` and refuses every send.
    pub conflicts: Vec<TranscriptConflict>,
    /// The transcript does not name the payer; the user confirms it is this company's.
    pub warnings: Vec<String>,
}

impl ProcessEngine {
    fn transcripts_dir(&self) -> PathBuf {
        self.home
            .join("process")
            .join("employer-report-transcripts")
    }

    pub(crate) fn transcript_imports(&self) -> Result<TranscriptImports, ProcessError> {
        let dir = self.transcripts_dir();
        if !dir.exists() {
            return Ok(TranscriptImports(Vec::new()));
        }
        let mut records = Vec::new();
        for entry in fs::read_dir(&dir).map_err(store)? {
            let path = entry.map_err(store)?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let bytes = fs::read(&path).map_err(store)?;
            let record: TranscriptImportRecord = serde_json::from_slice(&bytes).map_err(|_| {
                ProcessError::JournalCorrupt(format!(
                    "{}: corrupt transcript record",
                    path.display()
                ))
            })?;
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if record.schema_version != IMPORT_SCHEMA || record.artifact != name {
                return Err(ProcessError::JournalCorrupt(format!(
                    "{}: transcript record does not match its name or schema",
                    path.display()
                )));
            }
            records.push(record);
        }
        Ok(TranscriptImports(records))
    }

    /// Import an Incomes Register transcript 630 (CSV): store the file as an
    /// artifact and record which months it shows reported. A month aggountant
    /// already holds is not overwritten; the same file twice is a no-op.
    pub fn import_employer_transcript(
        &self,
        bytes: &[u8],
        now: DateTime<Utc>,
        dry_run: bool,
    ) -> Result<TranscriptImport, ProcessError> {
        if bytes.len() > MAX_TRANSCRIPT_BYTES {
            return Err(refused(
                "TRANSCRIPT_TOO_LARGE",
                "the file is larger than the 100 MB the e-service delivers; unzip the transcript and import the CSV",
            ));
        }
        if bytes.starts_with(b"PK\x03\x04") {
            return Err(refused(
                "TRANSCRIPT_UNKNOWN_HEADER",
                "this is a ZIP file; unzip it and import the CSV inside",
            ));
        }
        let transcript = transcript_630::parse(bytes).map_err(|e| refused(e.code, e.message))?;
        let artifact = hex::encode(<sha2::Sha256 as sha2::Digest>::digest(bytes));
        let imports = self.transcript_imports()?;
        let existing = imports.0.iter().find(|r| r.artifact == artifact).cloned();
        let already_imported = existing.is_some();
        let imported_at = existing.as_ref().map(|r| r.imported_at).unwrap_or(now);
        let months: Vec<TranscriptMonthRecord> = transcript
            .months
            .iter()
            .map(|m| TranscriptMonthRecord {
                month: m.month,
                report_count: m.report_count,
                health_insurance_contribution: m.health_insurance_contribution.to_string(),
                deductions: m.deductions.to_string(),
            })
            .collect();
        let record = TranscriptImportRecord {
            schema_version: IMPORT_SCHEMA,
            artifact: artifact.clone(),
            imported_at,
            covers_from: transcript.covers_from,
            covers_to: transcript.covers_to,
            months: months.clone(),
        };

        // Judge each covered month against what aggountant holds, without this
        // transcript (an earlier import of the same file is the same evidence).
        let earlier = TranscriptImports(
            imports
                .0
                .into_iter()
                .filter(|r| r.artifact != artifact)
                .collect(),
        );
        let mut recorded = Vec::new();
        let mut already_held = Vec::new();
        let mut not_reported = Vec::new();
        let mut conflicts = Vec::new();
        let mut month = transcript.covers_from;
        while month <= transcript.covers_to {
            let count = record.count(month);
            let own = self.employer_report_status_with(month, &TranscriptImports(Vec::new()))?;
            // The receipt of the month's current accepted report, if any.
            let held_source = own.current.map(|receipt| receipt.source);
            let disagrees = if count == 0 {
                earlier.evidence(month).is_some()
            } else {
                earlier.counts_none(month)
            };
            match (count, held_source, own.state) {
                (0, Some(_), _) => conflicts.push(TranscriptConflict {
                    month,
                    kind: TranscriptConflictKind::AcceptedInAggountantNoneInTranscript,
                    report_count: 0,
                }),
                (n, Some(source), _) => already_held.push(HeldMonth {
                    month,
                    source,
                    report_count: n,
                }),
                (n, None, EmployerMonthState::OutcomeUnknown) if n > 0 => {
                    conflicts.push(TranscriptConflict {
                        month,
                        kind: TranscriptConflictKind::InDoubtSendTranscriptCountsReport,
                        report_count: n,
                    })
                }
                (n, None, _) if disagrees => conflicts.push(TranscriptConflict {
                    month,
                    kind: TranscriptConflictKind::TranscriptsDisagree,
                    report_count: n,
                }),
                (0, None, _) => not_reported.push(month),
                (_, None, _) => recorded.push(month),
            }
            month = month.next();
        }

        if !dry_run && !already_imported {
            let stored = Ledger::open(&self.home)?.attach_artifact(bytes)?;
            if stored.to_hex() != artifact {
                return Err(store("artifact hash differs from the transcript's SHA-256"));
            }
            let dir = self.transcripts_dir();
            fs::create_dir_all(&dir).map_err(store)?;
            let path = dir.join(format!("{artifact}.json"));
            let tmp = path.with_extension("tmp");
            let mut f = fs::File::create(&tmp).map_err(store)?;
            f.write_all(&serde_json::to_vec_pretty(&record).map_err(store)?)
                .map_err(store)?;
            f.sync_all().map_err(store)?;
            fs::rename(&tmp, &path).map_err(store)?;
        }
        Ok(TranscriptImport {
            dry_run,
            already_imported,
            artifact,
            imported_at,
            covers_from: transcript.covers_from,
            covers_to: transcript.covers_to,
            separator: transcript.separator.to_string(),
            encoding: transcript.encoding,
            months,
            recorded,
            already_held,
            not_reported,
            conflicts,
            warnings: vec![
                "transcript 630 does not name the payer: confirm with the user that it is this company's transcript".into(),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aggountant_broker::ytj::EntryDescription;

    fn entry(register: &str, t: &str, from: &str, to: Option<&str>) -> RegisterEntry {
        RegisterEntry {
            entry_type: t.into(),
            descriptions: vec![EntryDescription {
                language_code: "1".into(),
                description: Some("Rekisterissä".into()),
            }],
            registration_date: Some(from.parse().unwrap()),
            end_date: to.map(|d| d.parse().unwrap()),
            register: register.into(),
            authority: "1".into(),
        }
    }
    fn m(s: &str) -> ReportMonth {
        ReportMonth::parse(s).unwrap()
    }
    fn today() -> NaiveDate {
        "2026-11-18".parse().unwrap()
    }

    #[test]
    fn several_entries_use_the_current_one_and_list_the_history() {
        let entries = vec![
            entry("6", "80", "2015-01-01", None),
            entry("7", "41", "2016-03-01", Some("2018-06-30")),
            entry("7", "99", "2017-01-01", None),
            entry("7", "41", "2020-02-01", None),
        ];
        let r = compare_registration(
            "1234567-1".into(),
            Some(entries.clone()),
            Some(m("2026-07")),
            today(),
        )
        .unwrap();
        assert_eq!(
            r.comparison,
            RegistrationComparison::ProfileLaterThanRegister
        );
        assert_eq!(r.implied_registered_from, Some(m("2020-02")));
        assert_eq!(r.hidden_months, 77);
        assert_eq!(r.periods.len(), 2);
        assert_eq!(r.periods[0].state, RegisterPeriodState::Ended);
        assert_eq!(r.ignored_entries.len(), 1);
        let same = compare_registration(
            "1234567-1".into(),
            Some(entries.clone()),
            Some(m("2020-02")),
            today(),
        )
        .unwrap();
        assert_eq!(same.comparison, RegistrationComparison::Match);
        let early = compare_registration(
            "1234567-1".into(),
            Some(entries.clone()),
            Some(m("2019-05")),
            today(),
        )
        .unwrap();
        assert_eq!(
            early.comparison,
            RegistrationComparison::ProfileEarlierThanRegister
        );
        let unset = compare_registration("1234567-1".into(), Some(entries), None, today()).unwrap();
        assert_eq!(unset.comparison, RegistrationComparison::ProfileNotSet);
    }

    #[test]
    fn an_entry_before_the_incomes_register_implies_2019_01() {
        let r = compare_registration(
            "1234567-1".into(),
            Some(vec![entry("7", "42", "2010-05-01", None)]),
            Some(m("2019-01")),
            today(),
        )
        .unwrap();
        assert_eq!(r.comparison, RegistrationComparison::Match);
    }

    #[test]
    fn ended_missing_and_unknown_entries() {
        let ended = compare_registration(
            "1234567-1".into(),
            Some(vec![entry("7", "41", "2020-02-01", Some("2025-12-31"))]),
            Some(m("2020-02")),
            today(),
        )
        .unwrap();
        assert_eq!(ended.comparison, RegistrationComparison::RegisterEnded);
        assert_eq!(ended.implied_registered_from, None);
        let none = compare_registration(
            "1234567-1".into(),
            Some(vec![entry("6", "80", "2020-01-01", None)]),
            None,
            today(),
        )
        .unwrap();
        assert_eq!(none.comparison, RegistrationComparison::NotInRegister);
        let unknown = compare_registration("1234567-1".into(), None, None, today()).unwrap_err();
        assert_eq!(unknown.error_code(), "YTJ_COMPANY_NOT_FOUND");
        let odd = compare_registration(
            "1234567-1".into(),
            Some(vec![entry("7", "43", "2020-01-01", None)]),
            None,
            today(),
        )
        .unwrap_err();
        assert_eq!(odd.error_code(), "YTJ_UNKNOWN_ENTRY_TYPE");
    }
}
