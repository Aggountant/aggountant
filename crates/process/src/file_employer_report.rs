//! The monthly employer's separate report (työnantajan erillisilmoitus) through
//! the same own-organisation Incomes Register channel as `file-wage`.
//!
//! One Run is one record sent for one month. Its id is the month plus the hash of
//! the exact unsigned record, so a preview, its approval and the bytes that leave
//! are the same thing: the caller approves by naming the previewed `record_hash`,
//! and the record is rebuilt and re-hashed before dispatch. The report carries no
//! PII, so unlike `file-wage` the hash itself is safe to keep in process state.
//!
//! A month is judged from all of its Runs. A Run whose dispatch was journalled
//! and has no definitive answer blocks every further send for the month until a
//! signed status query (or e-service evidence) settles it. An accepted Run
//! fences the month: the only further send is an explicit replacement of the
//! accepted report.
use crate::{
    employer_report_history::{
        compare_registration, register_entries, EmployerRegistration, RegistrationComparison,
        TranscriptEvidence, TranscriptImports,
    },
    fold,
    journal::{EventPayload, Journal},
    ProcessEngine, ProcessError, RunStatus,
};
use aggountant_broker::{
    tulorekisteri::{
        employer_envelope, ir_reference, read_wage_profile, wage_profile_path,
        with_employer_report, write_wage_profile, EmployerReportProfile, EmployerReportRequest,
        SVC_EMPLOYER,
    },
    tulorekisteri_xml::{payer_summary_record, record_hash},
    ytj::YtjLookup,
    AttemptOutcome, Broker, BrokerError, RemoteOutcome,
};
use aggountant_domain::payroll::{
    EmployerReportAction, EmployerReportContent, EmployerSeparateReport, MonthPayments, ReportMonth,
};
use aggountant_model::{BusinessId, Instant};
use jiff::civil::Date as CivilDate;
use jiff::SignedDuration;
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

const WORKFLOW: &str = "file-employer-report/v1: Preview(bind unsigned PayerSummaryReportRequestToIR) -> ApproveEmployerReport -> SendPayerSummaryReport(at-most-once) -> signed GetDeliveryDataStatus";
pub(crate) const WORKFLOW_NAME: &str = "file-employer-report";
const GATE: &str = "ApproveEmployerReport";
pub(crate) const EFFECT: &str = "submit-employer-report";
pub(crate) const UNKNOWN_CODE: &str = "OUTCOME_UNKNOWN_RECONCILE_ONLY";
const STATE_FILE: &str = "employer-report.json";
/// How far the record timestamp may be from the moment of sending. The record's
/// `ReportDate` is taken from it and must state when the report was given.
const MAX_TIMESTAMP_AGE_HOURS: i64 = 24;

fn version() -> String {
    record_hash(WORKFLOW)
}
fn refused(code: &'static str, message: impl Into<String>) -> ProcessError {
    ProcessError::EmployerReport {
        code,
        message: message.into(),
    }
}
fn store(e: impl std::fmt::Display) -> ProcessError {
    ProcessError::Store(e.to_string())
}

/// What the caller asks for: the month, what was paid in it, and whether this
/// replaces the month's accepted report.
#[derive(Debug, Clone, Copy)]
pub struct EmployerReportInput {
    pub month: ReportMonth,
    pub payments: MonthPayments,
    pub replace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Filing {
    /// A record this Run builds and sends.
    Sent {
        report: EmployerSeparateReport,
        timestamp: Instant,
        record_hash: String,
    },
    /// A report given outside aggountant (the e-service), recorded from evidence.
    External { receipt: EmployerReportReceipt },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunState {
    month: ReportMonth,
    /// 1 for the month's first accepted report, n+1 for a replacement of n.
    sequence: u32,
    filing: Filing,
}

/// Proof that the Incomes Register holds a report for the month.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployerReportReceipt {
    pub ir_delivery_id: Option<String>,
    pub ir_report_id: String,
    /// Report version in the register, when known.
    pub version: Option<u32>,
    /// `api` (processing feedback) or `e_service` (evidence given by a person).
    pub source: String,
}

impl EmployerReportReceipt {
    fn note(&self) -> String {
        let d = self.ir_delivery_id.as_deref().unwrap_or("-");
        let v = self.version.map(|v| v.to_string()).unwrap_or("-".into());
        match self.source.as_str() {
            "api" => format!("{d}/{}/{v}", self.ir_report_id),
            _ => format!("manual-e-service:{d}/{}/{v}", self.ir_report_id),
        }
    }
    pub(crate) fn parse(note: &str) -> Option<Self> {
        let (source, rest) = match note.strip_prefix("manual-e-service:") {
            Some(rest) => ("e_service", rest),
            None => ("api", note),
        };
        let mut parts = rest.split('/');
        let (d, r, v) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || !ir_reference(r) {
            return None;
        }
        Some(Self {
            ir_delivery_id: (d != "-")
                .then(|| d.to_string())
                .filter(|d| ir_reference(d)),
            ir_report_id: r.into(),
            version: if v == "-" {
                None
            } else {
                Some(v.parse().ok()?)
            },
            source: source.into(),
        })
    }
}

/// The outcome of one Run, derived from its journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EmployerRunOutcome {
    /// Built and approved at most; nothing has left.
    Pending,
    /// Dispatch was journalled with no definitive answer. Never resent.
    OutcomeUnknown,
    Rejected {
        error_code: String,
    },
    Accepted {
        receipt: EmployerReportReceipt,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct EmployerReportRun {
    pub run_id: String,
    pub sequence: u32,
    pub status: RunStatus,
    pub outcome: EmployerRunOutcome,
    /// `None` for a report recorded from e-service evidence.
    pub report: Option<EmployerSeparateReport>,
    pub record_hash: Option<String>,
    pub delivery_id: Option<String>,
}

/// The month as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployerMonthState {
    NotFiled,
    Pending,
    Rejected,
    OutcomeUnknown,
    Accepted,
    /// No report of aggountant's own is accepted, but an imported Incomes
    /// Register transcript counts at least one: reported outside aggountant,
    /// without a report reference.
    FiledOutside,
    /// Imported transcripts disagree: one counts a report, another none. Listed
    /// as missing, but nothing is sent until the e-service shows which is true.
    TranscriptConflict,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmployerMonthStatus {
    pub month: ReportMonth,
    pub deadline: CivilDate,
    pub state: EmployerMonthState,
    /// The report the register holds now: the accepted Run with the highest sequence.
    pub current: Option<EmployerReportReceipt>,
    pub runs: Vec<EmployerReportRun>,
    /// Transcript evidence for the month, when any imported transcript counts a report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<TranscriptEvidence>,
}

impl EmployerMonthStatus {
    fn in_doubt(&self) -> Option<&EmployerReportRun> {
        self.runs
            .iter()
            .find(|r| r.outcome == EmployerRunOutcome::OutcomeUnknown)
    }
    fn current_run(&self) -> Option<&EmployerReportRun> {
        self.runs
            .iter()
            .filter(|r| matches!(r.outcome, EmployerRunOutcome::Accepted { .. }))
            .max_by_key(|r| r.sequence)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EmployerReportPreview {
    pub month: ReportMonth,
    pub deadline: CivilDate,
    pub payments: MonthPayments,
    pub timestamp: Instant,
    pub report: EmployerSeparateReport,
    pub delivery_id: String,
    /// SHA-256 of the exact unsigned record. `submit` sends only these bytes.
    pub record_hash: String,
    /// Facts the approver should look at; they do not block the report.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MissingEmployerMonth {
    pub month: ReportMonth,
    pub deadline: CivilDate,
    pub overdue: bool,
    pub state: EmployerMonthState,
    /// A transcript counts a report for a month whose own send is in doubt; it
    /// may be that send, so the month stays listed until `status` settles it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<TranscriptEvidence>,
}

/// A month left out of `missing` because a transcript shows it reported.
#[derive(Debug, Clone, Serialize)]
pub struct FiledOutsideMonth {
    pub month: ReportMonth,
    pub evidence: TranscriptEvidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct MissingEmployerReports {
    pub registered_from: ReportMonth,
    pub as_of: CivilDate,
    pub months: Vec<MissingEmployerMonth>,
    /// Months reported outside aggountant, with the transcript that shows it.
    pub filed_outside: Vec<FiledOutsideMonth>,
}

struct Plan {
    preview: EmployerReportPreview,
    sequence: u32,
}

impl ProcessEngine {
    fn employer_profile(&self) -> Result<(BusinessId, EmployerReportProfile), ProcessError> {
        let profile = read_wage_profile(&self.home)
            .map_err(|e| refused("EMPLOYER_REPORT_NOT_CONFIGURED", e))?;
        let settings = profile.employer_report.ok_or_else(|| {
            refused(
                "EMPLOYER_REPORT_NOT_CONFIGURED",
                "add employer_report (registered_from, contact_name, contact_phone, pension_insurances) to .aggountant/tulorekisteri.json",
            )
        })?;
        if settings.registered_from < ReportMonth::parse("2019-01").expect("valid") {
            return Err(refused("EMPLOYER_REPORT_NOT_CONFIGURED", "registered_from"));
        }
        Ok((profile.business_id, settings))
    }

    fn employer_runs(&self, month: ReportMonth) -> Result<Vec<EmployerReportRun>, ProcessError> {
        let prefix = format!("er-{month}-");
        let mut runs = Vec::new();
        for run_id in self.list()? {
            let dir = self.run_dir(&run_id);
            // A directory left by a lease taken just before a crash holds nothing.
            if !run_id.starts_with(&prefix)
                || (!dir.join(STATE_FILE).exists() && Journal::new(&dir).read_all()?.is_empty())
            {
                continue;
            }
            let state = self.read_employer_state(&run_id)?;
            if state.month != month {
                return Err(ProcessError::JournalCorrupt(format!(
                    "{run_id}: month changed"
                )));
            }
            let events = Journal::new(&self.run_dir(&run_id)).read_all()?;
            let f = fold(&run_id, &events)?;
            if f.workflow != WORKFLOW_NAME || f.effective_version != version() {
                return Err(refused(
                    "EMPLOYER_REPORT_VERSION_MISMATCH",
                    format!("{run_id} was started under another workflow version"),
                ));
            }
            let outcome = classify(&events);
            let (report, hash, delivery) = match state.filing {
                Filing::Sent {
                    report,
                    timestamp,
                    record_hash,
                } => {
                    let delivery = report.delivery_id(timestamp);
                    (Some(report), Some(record_hash), Some(delivery))
                }
                Filing::External { .. } => (None, None, None),
            };
            runs.push(EmployerReportRun {
                run_id,
                sequence: state.sequence,
                status: f.to_view().status,
                outcome,
                report,
                record_hash: hash,
                delivery_id: delivery,
            });
        }
        runs.sort_by(|a, b| (a.sequence, &a.run_id).cmp(&(b.sequence, &b.run_id)));
        Ok(runs)
    }

    /// Read and verify one Run's state: the id must be derived from the content.
    fn read_employer_state(&self, run_id: &str) -> Result<RunState, ProcessError> {
        let bytes = fs::read(self.run_dir(run_id).join(STATE_FILE)).map_err(store)?;
        let state: RunState = serde_json::from_slice(&bytes)
            .map_err(|_| ProcessError::JournalCorrupt(format!("{run_id}: corrupt state")))?;
        let expected = match &state.filing {
            Filing::Sent {
                report,
                timestamp,
                record_hash: hash,
            } => {
                let rebuilt = payer_summary_record(report, *timestamp)
                    .map(|xml| record_hash(&xml))
                    .ok();
                if rebuilt.as_deref() != Some(hash.as_str()) {
                    return Err(ProcessError::JournalCorrupt(format!(
                        "{run_id}: stored report no longer produces its approved record"
                    )));
                }
                run_id_for(state.month, hash)
            }
            Filing::External { receipt } => run_id_for(state.month, &external_hash(receipt)),
        };
        if expected != run_id {
            return Err(ProcessError::JournalCorrupt(format!(
                "{run_id}: identity does not match content"
            )));
        }
        Ok(state)
    }

    /// The month's combined state (`file-wage employer-report status`, offline part).
    pub fn employer_report_status(
        &self,
        month: ReportMonth,
    ) -> Result<EmployerMonthStatus, ProcessError> {
        self.employer_report_status_with(month, &self.transcript_imports()?)
    }

    /// The month's state against already loaded transcript imports.
    pub(crate) fn employer_report_status_with(
        &self,
        month: ReportMonth,
        imports: &TranscriptImports,
    ) -> Result<EmployerMonthStatus, ProcessError> {
        let runs = self.employer_runs(month)?;
        let any = |p: fn(&EmployerRunOutcome) -> bool| runs.iter().any(|r| p(&r.outcome));
        let state = if any(|o| *o == EmployerRunOutcome::OutcomeUnknown) {
            EmployerMonthState::OutcomeUnknown
        } else if any(|o| matches!(o, EmployerRunOutcome::Accepted { .. })) {
            EmployerMonthState::Accepted
        } else if any(|o| matches!(o, EmployerRunOutcome::Rejected { .. })) {
            EmployerMonthState::Rejected
        } else if any(|o| *o == EmployerRunOutcome::Pending) {
            EmployerMonthState::Pending
        } else {
            EmployerMonthState::NotFiled
        };
        let transcript = imports.evidence(month);
        let state = match state {
            EmployerMonthState::NotFiled
            | EmployerMonthState::Pending
            | EmployerMonthState::Rejected
                if transcript.is_some() =>
            {
                if transcript
                    .as_ref()
                    .is_some_and(|t| t.contradicted_by.is_empty())
                {
                    EmployerMonthState::FiledOutside
                } else {
                    EmployerMonthState::TranscriptConflict
                }
            }
            other => other,
        };
        let mut status = EmployerMonthStatus {
            month,
            deadline: month.deadline(),
            state,
            current: None,
            runs,
            transcript,
        };
        status.current = status.current_run().and_then(|r| match &r.outcome {
            EmployerRunOutcome::Accepted { receipt } => Some(receipt.clone()),
            _ => None,
        });
        Ok(status)
    }

    /// Whether the month has a dispatched report without a definitive answer.
    pub fn employer_report_in_doubt(&self, month: ReportMonth) -> Result<bool, ProcessError> {
        Ok(self.employer_report_status(month)?.in_doubt().is_some())
    }

    /// Build the report the inputs describe and check that the month allows it.
    fn plan_employer_report(
        &self,
        input: EmployerReportInput,
        timestamp: Instant,
    ) -> Result<Plan, ProcessError> {
        let (business_id, profile) = self.employer_profile()?;
        let month = input.month;
        if month < profile.registered_from {
            return Err(refused(
                "EMPLOYER_REPORT_BEFORE_REGISTRATION",
                format!(
                    "{month} is before the employer-register entry ({}); no report is due",
                    profile.registered_from
                ),
            ));
        }
        if month > ReportMonth::of(timestamp.utc_date()) {
            return Err(refused(
                "EMPLOYER_REPORT_FUTURE_MONTH",
                "a month is reported during or after it, never in advance",
            ));
        }
        let content = EmployerReportContent::for_payments(input.payments).ok_or_else(|| {
            refused(
                "EMPLOYER_REPORT_WAGES_UNSUPPORTED",
                "a month with wages needs the health insurance contribution computed from its wages; aggountant does not compute it yet, so give this month's report in the Incomes Register e-service",
            )
        })?;
        let mut warnings = Vec::new();
        let travel_paid = self.wage_payment_months()?.contains(&month);
        match input.payments {
            MonthPayments::None if travel_paid => {
                return Err(refused(
                    "EMPLOYER_REPORT_PAYMENTS_CONFLICT",
                    format!("a file-wage report has a payment date in {month}, so the month is not 'no wages payable'; use tax-free-reimbursements-only"),
                ))
            }
            MonthPayments::TaxFreeReimbursementsOnly if !travel_paid => warnings.push(format!(
                "no file-wage report has a payment date in {month}; check that the reimbursements were reported to the Incomes Register"
            )),
            _ => {}
        }

        let status = self.employer_report_status(month)?;
        if let Some(run) = status.in_doubt() {
            return Err(refused(
                "EMPLOYER_REPORT_OUTCOME_UNKNOWN",
                format!("{} was sent without a definitive answer; run `file-wage employer-report status --month {month}` and never send again before it is settled", run.run_id),
            ));
        }
        let current = status.current_run();
        if let (None, Some(evidence)) = (current, &status.transcript) {
            return Err(if !evidence.contradicted_by.is_empty() {
                refused(
                    "EMPLOYER_REPORT_TRANSCRIPT_CONFLICT",
                    format!("imported Incomes Register transcripts disagree about {month}: one counts {} report(s), {} count(s) none. Check the month in the Incomes Register e-service: record an existing report's reference with `complete-manual`, or give the missing report in the e-service and record it", evidence.report_count, evidence.contradicted_by.len()),
                )
            } else if input.replace {
                refused(
                    "EMPLOYER_REPORT_NO_REFERENCE",
                    format!("{month} is known only from Incomes Register transcript 630 ({} report(s)), which has no report reference to replace; correct the report in the Incomes Register e-service, or record its reference with `complete-manual` first", evidence.report_count),
                )
            } else {
                refused(
                    "EMPLOYER_REPORT_FILED_OUTSIDE",
                    format!("Incomes Register transcript 630 shows {} report(s) for {month} given outside aggountant; do not send another. A correction is made in the Incomes Register e-service, or after recording the report reference with `complete-manual`", evidence.report_count),
                )
            });
        }
        let (action, sequence) = match (current, input.replace) {
            (None, false) => (EmployerReportAction::New, 1),
            (Some(run), false) => {
                return Err(refused(
                    "EMPLOYER_REPORT_ALREADY_FILED",
                    format!("{month} already has an accepted report ({}); a correction is a replacement (--replace)", run.run_id),
                ))
            }
            (None, true) => {
                return Err(refused(
                    "EMPLOYER_REPORT_NOTHING_TO_REPLACE",
                    format!("{month} has no accepted report to replace"),
                ))
            }
            (Some(run), true) => {
                let EmployerRunOutcome::Accepted { receipt } = &run.outcome else {
                    unreachable!("current run is accepted")
                };
                (
                    EmployerReportAction::Replacement {
                        ir_report_id: receipt.ir_report_id.clone(),
                        replaces_version: receipt.version,
                    },
                    run.sequence + 1,
                )
            }
        };
        let previous = current.and_then(|r| r.report.as_ref());
        // Register processing rule (Data delivery – Schemas – Employer's separate
        // reports, §2.9): in a replacement the pension insurances match the
        // previous version. A contribution replacement carries them forward; a
        // "no wages payable" replacement must already state the same ones.
        let pension_insurances = match (previous, &content) {
            (Some(previous), EmployerReportContent::HealthInsuranceContribution { .. }) => {
                previous.sent_pension_insurances().to_vec()
            }
            _ => profile.pension_insurances,
        };
        let report = EmployerSeparateReport {
            business_id,
            month,
            content,
            action,
            contact_name: profile.contact_name,
            contact_phone: profile.contact_phone,
            pension_insurances,
        };
        report
            .validate()
            .map_err(|e| refused("EMPLOYER_REPORT_INVALID", e.to_string()))?;
        if let Some(previous) = previous {
            if previous.content == report.content
                && previous.contact_name == report.contact_name
                && previous.contact_phone == report.contact_phone
                && previous.sent_pension_insurances() == report.sent_pension_insurances()
            {
                return Err(refused(
                    "EMPLOYER_REPORT_UNCHANGED",
                    "the replacement would repeat the accepted report",
                ));
            }
            // A changed pension provider or policy is corrected by cancelling and
            // re-sending (employer's separate report guidance §5), never by a
            // replacement, which must repeat the previous version's insurances.
            if previous.sent_pension_insurances() != report.sent_pension_insurances() {
                return Err(refused(
                    "EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED",
                    "pension insurance details cannot be corrected with a replacement; cancel the report in the e-service and send a new one",
                ));
            }
        }
        let xml = payer_summary_record(&report, timestamp)
            .map_err(|_| refused("EMPLOYER_REPORT_INVALID", "the record could not be built"))?;
        Ok(Plan {
            preview: EmployerReportPreview {
                month,
                deadline: month.deadline(),
                payments: input.payments,
                timestamp,
                delivery_id: report.delivery_id(timestamp),
                report,
                record_hash: record_hash(&xml),
                warnings,
            },
            sequence,
        })
    }

    /// Offline and read-only: the exact report and the hash an approval names.
    pub fn preview_employer_report(
        &self,
        input: EmployerReportInput,
        timestamp: Instant,
    ) -> Result<EmployerReportPreview, ProcessError> {
        Ok(self.plan_employer_report(input, timestamp)?.preview)
    }

    /// Approve exactly `approved_hash` and send once. A repeated call never sends
    /// again: a dispatched Run is settled only by `reconcile_employer_report`.
    pub fn submit_employer_report(
        &self,
        input: EmployerReportInput,
        timestamp: Instant,
        approved_hash: &str,
        broker: &Broker,
        now: Instant,
    ) -> Result<EmployerMonthStatus, ProcessError> {
        let plan = self.plan_employer_report(input, timestamp)?;
        let preview = plan.preview;
        if approved_hash != preview.record_hash {
            return Err(refused(
                "EMPLOYER_REPORT_HASH_MISMATCH",
                "approval must name the record_hash of a preview with the same month, payments, timestamp and settings",
            ));
        }
        if now.duration_since(timestamp).abs() > SignedDuration::from_hours(MAX_TIMESTAMP_AGE_HOURS)
        {
            return Err(refused(
                "EMPLOYER_REPORT_STALE_TIMESTAMP",
                "the timestamp sets the report date; preview again with the current time",
            ));
        }
        let run_id = run_id_for(preview.month, &preview.record_hash);
        let dir = self.run_dir(&run_id);
        let state_path = dir.join(STATE_FILE);
        let _lease = self.acquire_lease(&run_id)?;
        let journal = Journal::new(&dir);
        if state_path.exists() {
            if classify(&journal.read_all()?) != EmployerRunOutcome::Pending {
                return Err(refused(
                    "EMPLOYER_REPORT_ALREADY_SENT",
                    format!("{run_id} has already been sent; preview again with a new timestamp after correcting the cause of a rejection"),
                ));
            }
        } else {
            let state = RunState {
                month: preview.month,
                sequence: plan.sequence,
                filing: Filing::Sent {
                    report: preview.report.clone(),
                    timestamp,
                    record_hash: preview.record_hash.clone(),
                },
            };
            write_state(&state_path, &state)?;
        }
        self.read_employer_state(&run_id)?;
        let events = journal.read_all()?;
        let has = |p: fn(&EventPayload) -> bool| events.iter().any(|e| p(&e.payload));
        if events.is_empty() {
            journal.append(EventPayload::RunStarted {
                workflow: WORKFLOW_NAME.into(),
                workflow_version: version(),
                git_commit: crate::git_commit(),
                policy_set: "employer-separate-report-2026".into(),
                period: preview.month.to_string(),
            })?;
        }
        if !has(|p| matches!(p, EventPayload::EffectPlanned { .. })) {
            journal.append(EventPayload::EffectPlanned {
                effect_key: EFFECT.into(),
                effect_type: "SendPayerSummaryReport".into(),
            })?;
        }
        if !has(|p| matches!(p, EventPayload::GateOpened { .. })) {
            journal.append(EventPayload::GateOpened {
                gate_id: GATE.into(),
                gate_kind: "Preview".into(),
                snapshot_hash: preview.record_hash.clone(),
            })?;
        }
        if !has(|p| matches!(p, EventPayload::GateAnswered { .. })) {
            journal.append(EventPayload::GateAnswered {
                gate_id: GATE.into(),
                answer: "approve".into(),
            })?;
        }
        let f = fold(&run_id, &journal.read_all()?)?;
        if f.gates.len() != 1 || f.gates[0].snapshot_hash != preview.record_hash {
            return Err(ProcessError::JournalCorrupt(format!(
                "{run_id}: approval binding changed"
            )));
        }
        // From here on the outcome is uncertain until the register answers.
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT.into(),
        })?;
        let request = EmployerReportRequest {
            report: preview.report.clone(),
            timestamp,
            status_timestamp: None,
        };
        let envelope = employer_envelope(&request, false).map_err(|e| store(e.to_string()))?;
        let outcome = broker
            .submit(
                "process:file-employer-report",
                &preview.delivery_id,
                &envelope,
            )
            .map_err(store)?;
        record_outcome(&journal, outcome, false)?;
        self.employer_report_status(preview.month)
    }

    /// Settle a dispatched Run with an unknown outcome: first from the local
    /// outbox (a crash after an accepted send), otherwise with the signed status
    /// query. Never sends the report again.
    pub fn reconcile_employer_report(
        &self,
        month: ReportMonth,
        broker: &Broker,
        now: Instant,
    ) -> Result<EmployerMonthStatus, ProcessError> {
        let status = self.employer_report_status(month)?;
        let Some(run) = status.in_doubt() else {
            return Ok(status);
        };
        let run_id = run.run_id.clone();
        let _lease = self.acquire_lease(&run_id)?;
        let state = self.read_employer_state(&run_id)?;
        let Filing::Sent {
            report, timestamp, ..
        } = state.filing
        else {
            return Err(ProcessError::JournalCorrupt(format!(
                "{run_id}: an e-service record cannot be in doubt"
            )));
        };
        let journal = Journal::new(&self.run_dir(&run_id));
        let delivery_id = report.delivery_id(timestamp);
        if let Some(entry) = broker
            .outbox_entry(SVC_EMPLOYER, &delivery_id)
            .map_err(store)?
        {
            if let Some(success) = entry.terminal_success() {
                if let AttemptOutcome::Delivered { receipt }
                | AttemptOutcome::RecoveredAlreadyProcessed { receipt } = &success.outcome
                {
                    record_outcome(
                        &journal,
                        RemoteOutcome::Delivered {
                            receipt: receipt.clone(),
                        },
                        true,
                    )?;
                    return self.employer_report_status(month);
                }
            }
        }
        let request = EmployerReportRequest {
            report,
            timestamp,
            status_timestamp: Some(now),
        };
        let envelope = employer_envelope(&request, true).map_err(|e| store(e.to_string()))?;
        match broker.fetch(&envelope) {
            Ok(body) => {
                let field = |k: &str| body.get(k).and_then(|v| v.as_str()).unwrap_or_default();
                let version = body.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
                let receipt = format!(
                    "{}/{}/{version}",
                    field("ir_delivery_id"),
                    field("ir_report_id")
                );
                if EmployerReportReceipt::parse(&receipt).is_none() || version == 0 {
                    return Err(refused(
                        "EMPLOYER_REPORT_STATUS_UNKNOWN",
                        "status answer lacked the register references; keep the month parked and ask again later",
                    ));
                }
                record_outcome(&journal, RemoteOutcome::Delivered { receipt }, true)?;
            }
            Err(BrokerError::Fetch { code, message }) if code == "TULOREKISTERI_REJECTED" => {
                record_outcome(&journal, RemoteOutcome::Rejected { code, message }, true)?;
            }
            Err(_) => {
                return Err(refused(
                    "EMPLOYER_REPORT_STATUS_UNKNOWN",
                    "the register gave no definitive status; keep the month parked and ask again later, or record e-service evidence with complete-manual",
                ))
            }
        }
        self.employer_report_status(month)
    }

    /// Record e-service evidence: either the answer to a dispatched Run whose
    /// outcome is unknown, or a report for the month given outside aggountant.
    /// Check the e-service for an existing report before sending anything.
    pub fn complete_employer_report_manual(
        &self,
        month: ReportMonth,
        receipt: EmployerReportReceipt,
        now: Instant,
    ) -> Result<EmployerMonthStatus, ProcessError> {
        let receipt = EmployerReportReceipt {
            source: "e_service".into(),
            ..receipt
        };
        if !ir_reference(&receipt.ir_report_id)
            || receipt
                .ir_delivery_id
                .as_deref()
                .is_some_and(|d| !ir_reference(d))
            || receipt.version == Some(0)
        {
            return Err(refused(
                "EMPLOYER_REPORT_INVALID_EVIDENCE",
                "evidence needs the 32-character report reference from the e-service (and optionally the record reference and a positive version)",
            ));
        }
        if month > ReportMonth::of(now.utc_date()) {
            return Err(refused(
                "EMPLOYER_REPORT_FUTURE_MONTH",
                "month is in the future",
            ));
        }
        let status = self.employer_report_status(month)?;
        if let Some(run) = status.in_doubt() {
            let run_id = run.run_id.clone();
            if let Some(EmployerReportAction::Replacement {
                ir_report_id,
                replaces_version,
            }) = run.report.as_ref().map(|r| &r.action)
            {
                if *ir_report_id != receipt.ir_report_id {
                    return Err(refused(
                        "EMPLOYER_REPORT_INVALID_EVIDENCE",
                        "a replacement keeps the register's report reference; the evidence names another report",
                    ));
                }
                // The reference alone also matches the version being replaced, so
                // only a newer version proves the replacement arrived; without the
                // replaced version no e-service version can prove it.
                let Some(replaced) = *replaces_version else {
                    return Err(refused(
                        "EMPLOYER_REPORT_INVALID_EVIDENCE",
                        "the replaced report's version was not recorded, so e-service evidence cannot show that this replacement arrived; settle it with `file-wage employer-report status`",
                    ));
                };
                if receipt.version.is_none_or(|v| v <= replaced) {
                    return Err(refused(
                        "EMPLOYER_REPORT_INVALID_EVIDENCE",
                        format!("settling a replacement needs the e-service version of the report, newer than version {replaced}; an older version shows only that the replacement has not arrived"),
                    ));
                }
            }
            let _lease = self.acquire_lease(&run_id)?;
            Journal::new(&self.run_dir(&run_id)).append(EventPayload::EffectReconciled {
                effect_key: EFFECT.into(),
                note: receipt.note(),
            })?;
            return self.employer_report_status(month);
        }
        if let Some(run) = status.current_run() {
            return Err(refused(
                "EMPLOYER_REPORT_ALREADY_FILED",
                format!("{month} already has an accepted report ({})", run.run_id),
            ));
        }
        let run_id = run_id_for(month, &external_hash(&receipt));
        let dir = self.run_dir(&run_id);
        let _lease = self.acquire_lease(&run_id)?;
        write_state(
            &dir.join(STATE_FILE),
            &RunState {
                month,
                sequence: 1,
                filing: Filing::External {
                    receipt: receipt.clone(),
                },
            },
        )?;
        let journal = Journal::new(&dir);
        journal.append(EventPayload::RunStarted {
            workflow: WORKFLOW_NAME.into(),
            workflow_version: version(),
            git_commit: crate::git_commit(),
            policy_set: "employer-separate-report-2026".into(),
            period: month.to_string(),
        })?;
        journal.append(EventPayload::EffectPlanned {
            effect_key: EFFECT.into(),
            effect_type: "RecordEServiceReport".into(),
        })?;
        journal.append(EventPayload::EffectReconciled {
            effect_key: EFFECT.into(),
            note: receipt.note(),
        })?;
        self.employer_report_status(month)
    }

    /// Read-only: months from the employer-register entry up to the month before
    /// `as_of` with no accepted report, each with its deadline.
    pub fn missing_employer_reports(
        &self,
        as_of: CivilDate,
    ) -> Result<MissingEmployerReports, ProcessError> {
        let (_, profile) = self.employer_profile()?;
        let imports = self.transcript_imports()?;
        let until = ReportMonth::of(as_of);
        let mut months = Vec::new();
        let mut filed_outside = Vec::new();
        let mut month = profile.registered_from;
        while month < until {
            let status = self.employer_report_status_with(month, &imports)?;
            match (status.state, status.transcript) {
                (EmployerMonthState::Accepted, _) => {}
                (EmployerMonthState::FiledOutside, Some(evidence)) => {
                    filed_outside.push(FiledOutsideMonth { month, evidence })
                }
                (state, transcript) => months.push(MissingEmployerMonth {
                    month,
                    deadline: month.deadline(),
                    overdue: as_of > month.deadline(),
                    state,
                    transcript,
                }),
            }
            month = month.next();
        }
        Ok(MissingEmployerReports {
            registered_from: profile.registered_from,
            as_of,
            months,
            filed_outside,
        })
    }
}

fn run_id_for(month: ReportMonth, hash: &str) -> String {
    format!("er-{month}-{hash}")
}

fn external_hash(receipt: &EmployerReportReceipt) -> String {
    record_hash(&format!("external:{}", receipt.note()))
}

fn write_state(path: &std::path::Path, state: &RunState) -> Result<(), ProcessError> {
    let bytes = serde_json::to_vec_pretty(state).map_err(store)?;
    fs::create_dir_all(path.parent().expect("run dir")).map_err(store)?;
    let tmp = path.with_extension("tmp");
    let mut f = fs::File::create(&tmp).map_err(store)?;
    f.write_all(&bytes).map_err(store)?;
    f.sync_all().map_err(store)?;
    fs::rename(&tmp, path).map_err(store)
}

fn classify(events: &[crate::JournalEvent]) -> EmployerRunOutcome {
    let mut outcome = EmployerRunOutcome::Pending;
    for e in events {
        match &e.payload {
            EventPayload::EffectSucceeded { note, .. }
            | EventPayload::EffectReconciled { note, .. } => {
                return match EmployerReportReceipt::parse(note) {
                    Some(receipt) => EmployerRunOutcome::Accepted { receipt },
                    // An acceptance without readable references proves nothing new
                    // can be sent safely; treat it as unsettled.
                    None => EmployerRunOutcome::OutcomeUnknown,
                };
            }
            EventPayload::EffectExecuting { .. } => outcome = EmployerRunOutcome::OutcomeUnknown,
            EventPayload::EffectRejected { error_code, .. } if error_code == UNKNOWN_CODE => {
                outcome = EmployerRunOutcome::OutcomeUnknown
            }
            EventPayload::EffectRejected { error_code, .. } => {
                outcome = EmployerRunOutcome::Rejected {
                    error_code: error_code.clone(),
                }
            }
            _ => {}
        }
    }
    outcome
}

fn record_outcome(
    journal: &Journal,
    outcome: RemoteOutcome,
    reconciled: bool,
) -> Result<(), ProcessError> {
    journal.append(match outcome {
        RemoteOutcome::Delivered { receipt }
        | RemoteOutcome::RecoveredAlreadyProcessed { receipt } => {
            if reconciled {
                EventPayload::EffectReconciled {
                    effect_key: EFFECT.into(),
                    note: receipt,
                }
            } else {
                EventPayload::EffectSucceeded {
                    effect_key: EFFECT.into(),
                    note: receipt,
                }
            }
        }
        RemoteOutcome::Unknown => EventPayload::EffectRejected {
            effect_key: EFFECT.into(),
            error_code: UNKNOWN_CODE.into(),
        },
        RemoteOutcome::Rejected { code, .. } => EventPayload::EffectRejected {
            effect_key: EFFECT.into(),
            error_code: code,
        },
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// configure: the employer_report block of the Tulorekisteri profile
// ---------------------------------------------------------------------------

/// Where the registration check of `configure` gets its answer.
pub enum RegistrationCheck<'a> {
    /// Ask YTJ and judge its entries on `today`.
    Online {
        lookup: &'a dyn YtjLookup,
        today: CivilDate,
    },
    /// The caller chose to write without the check.
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationVerdict {
    /// `registered_from` is the current employer-register entry's first month.
    Match,
    /// The register disagrees and the caller accepted it explicitly.
    MismatchAccepted,
    /// The check was not made (`--offline`).
    NotChecked,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmployerReportConfigured {
    pub path: PathBuf,
    pub dry_run: bool,
    /// The file was replaced. False on a dry run and when nothing changed.
    pub written: bool,
    pub changed: bool,
    /// The block as it was written before, valid or not.
    pub before: Option<serde_json::Value>,
    pub after: EmployerReportProfile,
    pub registration_verdict: RegistrationVerdict,
    /// The YTJ comparison for the new `registered_from`; absent offline.
    pub registration: Option<EmployerRegistration>,
}

impl ProcessEngine {
    /// Validate `settings`, check `registered_from` against the employer
    /// register and write the profile's `employer_report` block, keeping every
    /// other byte of the profile. A register disagreement is refused unless
    /// `accept_register_mismatch`; the YTJ check is skipped only on
    /// [`RegistrationCheck::Offline`].
    pub fn configure_employer_report(
        &self,
        settings: EmployerReportProfile,
        check: RegistrationCheck<'_>,
        accept_register_mismatch: bool,
        dry_run: bool,
    ) -> Result<EmployerReportConfigured, ProcessError> {
        settings
            .validate()
            .map_err(|m| refused("EMPLOYER_REPORT_INVALID_SETTINGS", m))?;
        let path = wage_profile_path(&self.home);
        let raw = fs::read_to_string(&path).map_err(|_| {
            refused(
                "TULOREKISTERI_NOT_READY",
                "configure .aggountant/tulorekisteri.json (business_id, mode, reporting_permission_active) first",
            )
        })?;
        let spliced = with_employer_report(&raw, &settings)
            .map_err(|m| refused("TULOREKISTERI_NOT_READY", m))?;
        let business_id = &spliced.profile.business_id;
        let (registration, registration_verdict) = match check {
            RegistrationCheck::Offline => (None, RegistrationVerdict::NotChecked),
            RegistrationCheck::Online { lookup, today } => {
                let registration = compare_registration(
                    business_id.as_str().to_string(),
                    register_entries(lookup, business_id).map_err(|e| match e {
                        ProcessError::EmployerReport { code, message } => {
                            ProcessError::EmployerReport {
                                code,
                                message: format!("{message}; nothing was written (pass --offline to write without the check)"),
                            }
                        }
                        other => other,
                    })?,
                    Some(settings.registered_from),
                    today,
                )?;
                let verdict = if registration.comparison == RegistrationComparison::Match {
                    RegistrationVerdict::Match
                } else if accept_register_mismatch {
                    RegistrationVerdict::MismatchAccepted
                } else {
                    return Err(refused(
                        "EMPLOYER_REGISTER_MISMATCH",
                        format!(
                            "registered_from {} disagrees with the employer register in YTJ: {}. Nothing was written; use the register's month, or pass --accept-register-mismatch on the user's say-so (file-wage employer-report registration shows the entries)",
                            settings.registered_from, registration.message
                        ),
                    ));
                };
                (Some(registration), verdict)
            }
        };
        let changed = spliced.text != raw;
        if changed && !dry_run {
            write_wage_profile(&self.home, spliced.text.as_bytes()).map_err(store)?;
        }
        Ok(EmployerReportConfigured {
            path,
            dry_run,
            written: changed && !dry_run,
            changed,
            before: spliced.before,
            after: settings,
            registration_verdict,
            registration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receipt_notes_round_trip() {
        let api = EmployerReportReceipt {
            ir_delivery_id: Some("11111111222233334444555555555555".into()),
            ir_report_id: "22222222222233334444555555555555".into(),
            version: Some(2),
            source: "api".into(),
        };
        assert_eq!(EmployerReportReceipt::parse(&api.note()), Some(api.clone()));
        let manual = EmployerReportReceipt {
            ir_delivery_id: None,
            version: None,
            source: "e_service".into(),
            ..api
        };
        assert_eq!(EmployerReportReceipt::parse(&manual.note()), Some(manual));
        for bad in ["", "a/b/c", "x/22222222222233334444555555555555/one"] {
            assert_eq!(EmployerReportReceipt::parse(bad), None);
        }
    }
}
