//! Corrections of an accepted travel report (`file-wage correction`): a
//! replacement report or a cancellation, through the same own-organisation
//! Incomes Register channel as the report itself.
//!
//! A correction is a filing of its own, so it is its own Run: previewed,
//! approved by naming the previewed `record_hash`, sent once, and settled only
//! by the signed status query or by e-service evidence. Its id is the digest of
//! the correction and its creation time, which carries no personal identity
//! code; the PII-derived hash of a replacement record is checked at approval and
//! never stored, as in `file-wage`.
//!
//! What the register holds for a report is judged from the report's Run and all
//! of its correction Runs. A correction whose dispatch was journalled without a
//! definitive answer blocks every further correction until it is settled. A
//! correction that was made in the e-service is recorded from evidence as a Run
//! without a send, so the chain names the version the register actually holds.
use crate::{
    authority::incomes_register_receipt,
    file_employer_report::{
        classify, record_outcome, write_state, EmployerReportReceipt, EmployerRunOutcome,
    },
    file_wage::{WagePreviewSensitive, RECORD_HASH_SENSITIVE},
    fold,
    journal::{EventPayload, Journal},
    ProcessEngine, ProcessError,
};
use aggountant_broker::{
    tulorekisteri::{
        ir_reference, wage_correction_envelope, WageCorrectionRequest, SVC_WAGE_CORRECTION,
    },
    tulorekisteri_xml::{
        invalidation_record, record_hash, wage_replacement_record, DELIVERY_TYPE_WAGE_CANCELLATION,
    },
    AttemptOutcome, Broker, BrokerError, RemoteOutcome,
};
use aggountant_domain::{WageCorrection, WageCorrectionAction, WageReport};
use aggountant_ledger::Ledger;
use aggountant_model::Instant;
use jiff::SignedDuration;
use serde::{Deserialize, Serialize};
use std::fs;

const WORKFLOW: &str = "file-wage-correction/v1: Preview(bind unsigned replacement WageReportRequestToIR or InvalidationRequestToIR) -> ApproveWageCorrection -> SendWageReport|SendInvalidation(at-most-once) -> signed GetDeliveryDataStatus";
pub(crate) const WORKFLOW_NAME: &str = "file-wage-correction";
const GATE: &str = "ApproveWageCorrection";
pub(crate) const EFFECT: &str = "submit-wage-correction";
pub(crate) const UNKNOWN_CODE: &str = "OUTCOME_UNKNOWN_RECONCILE_ONLY";
const STATE_FILE: &str = "wage-correction.json";
/// How far the record timestamp may be from the moment of sending, so that an
/// approval is not spent on a preview made for another day.
const MAX_TIMESTAMP_AGE_HOURS: i64 = 24;

fn version() -> String {
    record_hash(WORKFLOW)
}
fn refused(code: &'static str, message: impl Into<String>) -> ProcessError {
    ProcessError::IncomesRegister {
        code,
        message: message.into(),
    }
}
fn store(e: impl std::fmt::Display) -> ProcessError {
    ProcessError::Store(e.to_string())
}
fn corrupt(run_id: &str, what: &str) -> ProcessError {
    ProcessError::JournalCorrupt(format!("{run_id}: {what}"))
}

/// What the caller asks for. A replacement gives the whole corrected report; a
/// cancellation gives none.
#[derive(Debug, Clone)]
pub struct WageCorrectionInput {
    pub action: WageCorrectionAction,
    pub report: Option<WageReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Filing {
    /// A correction record this Run builds and sends.
    Sent {
        correction: WageCorrection,
        timestamp: Instant,
    },
    /// A correction made in the e-service, recorded from evidence.
    External {
        action: WageCorrectionAction,
        receipt: EmployerReportReceipt,
    },
}
impl Filing {
    fn action(&self) -> WageCorrectionAction {
        match self {
            Filing::Sent { correction, .. } => correction.action,
            Filing::External { action, .. } => *action,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionState {
    /// The `file-wage` Run of the report being corrected.
    original_run_id: String,
    /// The report itself is 1; its nth accepted correction is n + 1.
    sequence: u32,
    filing: Filing,
}

/// One filing in a report's history: the report itself or a correction of it.
#[derive(Debug, Clone, Serialize)]
pub struct WageFiling {
    pub run_id: String,
    pub sequence: u32,
    pub kind: WageFilingKind,
    pub outcome: EmployerRunOutcome,
    /// False for a correction recorded from e-service evidence.
    pub sent_by_aggountant: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WageFilingKind {
    Report,
    Replacement,
    Cancellation,
}

/// What the Incomes Register holds for the report, as far as local records know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WageReportState {
    /// The report itself has not been sent.
    Pending,
    /// The report itself was refused by the register.
    Rejected,
    /// The report itself was sent without a definitive answer.
    OutcomeUnknown,
    /// Accepted and never corrected.
    Accepted,
    /// The accepted report was replaced; `current` is the latest version.
    Replaced,
    /// The report was cancelled; the register holds no valid report.
    Cancelled,
    /// A correction was sent without a definitive answer. Nothing more is sent
    /// before it is settled.
    CorrectionOutcomeUnknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct WageReportStatus {
    /// The `file-wage` Run of the report.
    pub run_id: String,
    pub state: WageReportState,
    /// The valid report the register holds now. `None` when there is none
    /// (never accepted, or cancelled).
    pub current: Option<EmployerReportReceipt>,
    /// The content of the report as last sent through aggountant. `false` in
    /// `report_is_current` when a later e-service replacement changed it.
    pub report: WageReport,
    pub report_is_current: bool,
    pub filings: Vec<WageFiling>,
}

impl WageReportStatus {
    fn in_doubt(&self) -> Option<&WageFiling> {
        self.filings
            .iter()
            .skip(1)
            .find(|f| f.outcome == EmployerRunOutcome::OutcomeUnknown)
    }
    fn accepted(&self) -> impl Iterator<Item = (&WageFiling, &EmployerReportReceipt)> {
        self.filings.iter().filter_map(|f| match &f.outcome {
            EmployerRunOutcome::Accepted { receipt } => Some((f, receipt)),
            _ => None,
        })
    }
    /// The latest accepted filing, the report itself included.
    fn latest(&self) -> Option<(&WageFiling, &EmployerReportReceipt)> {
        self.accepted().max_by_key(|(f, _)| f.sequence)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WageCorrectionPreview {
    /// The `file-wage` Run of the report being corrected.
    pub run_id: String,
    pub action: WageCorrectionAction,
    pub timestamp: Instant,
    /// The register's reference of the report and the version being corrected.
    pub ir_report_id: String,
    pub corrects_version: Option<u32>,
    /// The report as the register holds it according to local records.
    pub previous_report: WageReport,
    /// The report as it will read after a replacement; absent for a cancellation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<WageReport>,
    pub delivery_id: String,
    /// SHA-256 of the exact unsigned record. `submit` sends only these bytes.
    pub record_hash: String,
    /// Present for a replacement, whose record carries the personal identity code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<WagePreviewSensitive>,
    /// Facts the approver should look at; they do not block the correction.
    pub warnings: Vec<String>,
}

struct Plan {
    preview: WageCorrectionPreview,
    correction: WageCorrection,
    hetu: String,
    sequence: u32,
}

impl ProcessEngine {
    fn read_correction_state(&self, run_id: &str) -> Result<CorrectionState, ProcessError> {
        let bytes = fs::read(self.run_dir(run_id).join(STATE_FILE)).map_err(store)?;
        let state: CorrectionState =
            serde_json::from_slice(&bytes).map_err(|_| corrupt(run_id, "corrupt state"))?;
        let expected = match &state.filing {
            Filing::Sent {
                correction,
                timestamp,
            } => {
                let original = format!("wr-{}", correction.report.identity_hash());
                if original != state.original_run_id {
                    return Err(corrupt(run_id, "correction names another report"));
                }
                sent_run_id(correction, *timestamp)
            }
            Filing::External { action, receipt } => {
                external_run_id(&state.original_run_id, *action, receipt)
            }
        };
        if expected != run_id {
            return Err(corrupt(run_id, "identity does not match content"));
        }
        Ok(state)
    }

    /// The `file-wage` Run a correction Run corrects.
    pub(crate) fn wage_correction_original(&self, run_id: &str) -> Result<String, ProcessError> {
        Ok(self.read_correction_state(run_id)?.original_run_id)
    }

    /// Every correction Run of the report, in order.
    fn wage_corrections(
        &self,
        original_run_id: &str,
    ) -> Result<Vec<(WageFiling, CorrectionState)>, ProcessError> {
        let mut out = Vec::new();
        for run_id in self.list()? {
            let dir = self.run_dir(&run_id);
            // A directory left by a lease taken just before a crash holds nothing.
            if !run_id.starts_with("wc-")
                || (!dir.join(STATE_FILE).exists() && Journal::new(&dir).read_all()?.is_empty())
            {
                continue;
            }
            let state = self.read_correction_state(&run_id)?;
            if state.original_run_id != original_run_id {
                continue;
            }
            let events = Journal::new(&dir).read_all()?;
            let f = fold(&run_id, &events)?;
            if f.workflow != WORKFLOW_NAME || f.effective_version != version() {
                return Err(refused(
                    "WAGE_CORRECTION_VERSION_MISMATCH",
                    format!("{run_id} was started under another workflow version"),
                ));
            }
            let (sent, delivery_id) = match &state.filing {
                Filing::Sent {
                    correction,
                    timestamp,
                } => (true, Some(correction.delivery_id(*timestamp))),
                Filing::External { .. } => (false, None),
            };
            out.push((
                WageFiling {
                    run_id,
                    sequence: state.sequence,
                    kind: match state.filing.action() {
                        WageCorrectionAction::Replacement => WageFilingKind::Replacement,
                        WageCorrectionAction::Cancellation => WageFilingKind::Cancellation,
                    },
                    outcome: classify(&events, EmployerReportReceipt::parse),
                    sent_by_aggountant: sent,
                    delivery_id,
                },
                state,
            ));
        }
        out.sort_by(|a, b| (a.0.sequence, &a.0.run_id).cmp(&(b.0.sequence, &b.0.run_id)));
        Ok(out)
    }

    /// What the register holds for the report of `run_id` (`file-wage correction
    /// status`, offline part): the report and every correction of it.
    pub fn wage_report_status(&self, run_id: &str) -> Result<WageReportStatus, ProcessError> {
        let original = self.read_wage(run_id)?;
        let events = Journal::new(&self.run_dir(run_id)).read_all()?;
        let original_outcome = classify(&events, incomes_register_receipt);
        let corrections = self.wage_corrections(run_id)?;
        let mut filings = vec![WageFiling {
            run_id: run_id.to_string(),
            sequence: 1,
            kind: WageFilingKind::Report,
            outcome: original_outcome.clone(),
            sent_by_aggountant: true,
            delivery_id: Some(original.report.delivery_id()),
        }];
        // The content as last sent through aggountant, and whether an e-service
        // replacement has changed it since.
        let mut report = original.report;
        let mut report_is_current = true;
        for (filing, state) in &corrections {
            if matches!(filing.outcome, EmployerRunOutcome::Accepted { .. })
                && filing.kind == WageFilingKind::Replacement
            {
                match &state.filing {
                    Filing::Sent { correction, .. } => {
                        report = correction.report.clone();
                        report_is_current = true;
                    }
                    Filing::External { .. } => report_is_current = false,
                }
            }
        }
        filings.extend(corrections.into_iter().map(|(f, _)| f));
        let mut status = WageReportStatus {
            run_id: run_id.to_string(),
            state: WageReportState::Pending,
            current: None,
            report,
            report_is_current,
            filings,
        };
        let latest = status.latest().map(|(f, r)| (f.kind, r.clone()));
        status.state = match (&original_outcome, &latest) {
            (EmployerRunOutcome::Pending, _) => WageReportState::Pending,
            (EmployerRunOutcome::Rejected { .. }, _) => WageReportState::Rejected,
            (EmployerRunOutcome::OutcomeUnknown, _) => WageReportState::OutcomeUnknown,
            (EmployerRunOutcome::Accepted { .. }, _) if status.in_doubt().is_some() => {
                WageReportState::CorrectionOutcomeUnknown
            }
            (_, Some((WageFilingKind::Cancellation, _))) => WageReportState::Cancelled,
            (_, Some((WageFilingKind::Replacement, _))) => WageReportState::Replaced,
            _ => WageReportState::Accepted,
        };
        status.current = match latest {
            Some((WageFilingKind::Cancellation, _)) | None => None,
            Some((_, receipt)) => Some(receipt),
        };
        Ok(status)
    }

    /// Whether the report has a correction sent without a definitive answer.
    pub fn wage_correction_in_doubt(&self, run_id: &str) -> Result<bool, ProcessError> {
        Ok(self.wage_report_status(run_id)?.in_doubt().is_some())
    }

    /// The valid report a correction would act on, or the reason there is none.
    fn correctable(
        &self,
        status: &WageReportStatus,
    ) -> Result<(EmployerReportReceipt, u32), ProcessError> {
        let run_id = &status.run_id;
        if let Some(filing) = status.in_doubt() {
            return Err(refused(
                "WAGE_CORRECTION_OUTCOME_UNKNOWN",
                format!("{} was sent without a definitive answer; run `file-wage correction status {run_id}` and never send again before it is settled", filing.run_id),
            ));
        }
        match status.state {
            WageReportState::Accepted | WageReportState::Replaced => {}
            WageReportState::Cancelled => {
                return Err(refused(
                    "WAGE_CORRECTION_REPORT_CANCELLED",
                    format!("the report of {run_id} is cancelled, so there is nothing to correct; a new report is a new `file-wage start` with its own source_reference"),
                ))
            }
            _ => {
                return Err(refused(
                    "WAGE_CORRECTION_NOT_ACCEPTED",
                    format!("{run_id} has no recorded acceptance, so there is nothing to correct; see `run show --run-id {run_id}` (an unknown outcome is settled with `file-wage reconcile`)"),
                ))
            }
        }
        let (filing, receipt) = status.latest().expect("an accepted state has a receipt");
        Ok((receipt.clone(), filing.sequence + 1))
    }

    fn plan_wage_correction(
        &self,
        run_id: &str,
        input: &WageCorrectionInput,
        timestamp: Instant,
    ) -> Result<Plan, ProcessError> {
        let status = self.wage_report_status(run_id)?;
        let (receipt, sequence) = self.correctable(&status)?;
        let previous = status.report.clone();
        let mut warnings = Vec::new();
        if !status.report_is_current {
            warnings.push(
                "the report was last replaced in the e-service, so previous_report is not what the register holds now; compare with the e-service before approving".to_string(),
            );
        }
        if receipt.version.is_none() {
            warnings.push(
                "the version of the report in the register is not recorded, so the correction applies to its latest version, whatever that is".to_string(),
            );
        }
        let report = match (input.action, &input.report) {
            (WageCorrectionAction::Replacement, Some(report)) => {
                // Data delivery – Schemas – Earnings payment reports 2026: in a
                // replacement the pay period (§2.1), the payer (§2.4) and the
                // income earner (§2.13) repeat the previous version. Changing one
                // of them is a cancellation and a new report.
                if report.identity_hash() != previous.identity_hash()
                    || report.period_start != previous.period_start
                    || report.period_end != previous.period_end
                {
                    return Err(refused(
                        "WAGE_CORRECTION_REPLACEMENT_NOT_ALLOWED",
                        "a replacement keeps business_id, employee_id, source_reference, payment_date, period_start and period_end of the accepted report; to change the payer, the income earner, the payment date or the pay period, cancel the report (--action cancel) and send a new one with `file-wage start`",
                    ));
                }
                if status.report_is_current && *report == previous {
                    return Err(refused(
                        "WAGE_CORRECTION_UNCHANGED",
                        "the replacement would repeat the accepted report",
                    ));
                }
                report.clone()
            }
            (WageCorrectionAction::Replacement, None) => {
                return Err(refused(
                    "WAGE_CORRECTION_INVALID",
                    "a replacement needs the whole corrected report",
                ))
            }
            (WageCorrectionAction::Cancellation, Some(_)) => {
                return Err(refused(
                    "WAGE_CORRECTION_INVALID",
                    "a cancellation takes no report",
                ))
            }
            (WageCorrectionAction::Cancellation, None) => previous.clone(),
        };
        let correction = WageCorrection {
            action: input.action,
            report,
            ir_report_id: receipt.ir_report_id.clone(),
            corrects_version: receipt.version,
        };
        correction
            .validate(timestamp.utc_date())
            .map_err(|e| refused("WAGE_CORRECTION_INVALID", e.to_string()))?;
        let delivery_id = correction.delivery_id(timestamp);
        let invalid = |_| {
            refused("WAGE_CORRECTION_INVALID", "the record could not be built from the report and the income earner's stored identity code")
        };
        let (xml, hetu) = match correction.action {
            WageCorrectionAction::Replacement => {
                let hetu = Ledger::open(&self.home)?
                    .get_employee(&correction.report.employee_id)?
                    .hetu;
                let xml =
                    wage_replacement_record(&correction, &hetu, timestamp).map_err(invalid)?;
                (xml, hetu)
            }
            WageCorrectionAction::Cancellation => (
                invalidation_record(
                    DELIVERY_TYPE_WAGE_CANCELLATION,
                    &delivery_id,
                    correction.report.business_id.as_str(),
                    &correction.ir_report_id,
                    correction.corrects_version,
                    timestamp,
                ),
                String::new(),
            ),
        };
        let replacement = correction.action == WageCorrectionAction::Replacement;
        Ok(Plan {
            preview: WageCorrectionPreview {
                run_id: run_id.to_string(),
                action: correction.action,
                timestamp,
                ir_report_id: correction.ir_report_id.clone(),
                corrects_version: correction.corrects_version,
                previous_report: previous,
                report: replacement.then(|| correction.report.clone()),
                delivery_id,
                record_hash: record_hash(&xml),
                sensitive: replacement.then_some(WagePreviewSensitive {
                    record_hash: RECORD_HASH_SENSITIVE,
                }),
                warnings,
            },
            correction,
            hetu,
            sequence,
        })
    }

    /// Offline and read-only: the exact correction and the hash an approval names.
    pub fn preview_wage_correction(
        &self,
        run_id: &str,
        input: &WageCorrectionInput,
        timestamp: Instant,
    ) -> Result<WageCorrectionPreview, ProcessError> {
        Ok(self.plan_wage_correction(run_id, input, timestamp)?.preview)
    }

    /// Approve exactly `approved_hash` and send once. A repeated call never sends
    /// again: a dispatched correction is settled only by
    /// `reconcile_wage_correction` or e-service evidence.
    pub fn submit_wage_correction(
        &self,
        run_id: &str,
        input: &WageCorrectionInput,
        timestamp: Instant,
        approved_hash: &str,
        broker: &Broker,
        now: Instant,
    ) -> Result<WageReportStatus, ProcessError> {
        let plan = self.plan_wage_correction(run_id, input, timestamp)?;
        if approved_hash != plan.preview.record_hash {
            return Err(refused(
                "WAGE_CORRECTION_HASH_MISMATCH",
                "approval must name the record_hash of a preview with the same action, report and timestamp; the report, the income earner's identity code or the register's version changed",
            ));
        }
        if now.duration_since(timestamp).abs() > SignedDuration::from_hours(MAX_TIMESTAMP_AGE_HOURS)
        {
            return Err(refused(
                "WAGE_CORRECTION_STALE_TIMESTAMP",
                "the timestamp is the record's creation time; preview again with the current time",
            ));
        }
        let correction_run = sent_run_id(&plan.correction, timestamp);
        let dir = self.run_dir(&correction_run);
        let state_path = dir.join(STATE_FILE);
        let _lease = self.acquire_lease(&correction_run)?;
        let journal = Journal::new(&dir);
        if state_path.exists() {
            if classify(&journal.read_all()?, EmployerReportReceipt::parse)
                != EmployerRunOutcome::Pending
            {
                return Err(refused(
                    "WAGE_CORRECTION_ALREADY_SENT",
                    format!("{correction_run} has already been sent; preview again with a new timestamp after correcting the cause of a rejection"),
                ));
            }
        } else {
            write_state(
                &state_path,
                &CorrectionState {
                    original_run_id: run_id.to_string(),
                    sequence: plan.sequence,
                    filing: Filing::Sent {
                        correction: plan.correction.clone(),
                        timestamp,
                    },
                },
            )?;
        }
        self.read_correction_state(&correction_run)?;
        // The gate binds the PII-free digest of the correction; the approved
        // record hash was checked above against the record rebuilt from it.
        let binding = plan.correction.correction_hash(timestamp);
        let events = journal.read_all()?;
        let has = |p: fn(&EventPayload) -> bool| events.iter().any(|e| p(&e.payload));
        if events.is_empty() {
            journal.append(EventPayload::RunStarted {
                workflow: WORKFLOW_NAME.into(),
                workflow_version: version(),
                git_commit: crate::git_commit(),
                policy_set: "travel-only-2026".into(),
                period: plan.correction.report.payment_date.to_string(),
            })?;
        }
        if !has(|p| matches!(p, EventPayload::EffectPlanned { .. })) {
            journal.append(EventPayload::EffectPlanned {
                effect_key: EFFECT.into(),
                effect_type: match plan.correction.action {
                    WageCorrectionAction::Replacement => "SendWageReport(replacement)",
                    WageCorrectionAction::Cancellation => "SendInvalidation",
                }
                .into(),
            })?;
        }
        if !has(|p| matches!(p, EventPayload::GateOpened { .. })) {
            journal.append(EventPayload::GateOpened {
                gate_id: GATE.into(),
                gate_kind: "Preview".into(),
                snapshot_hash: binding.clone(),
            })?;
        }
        if !has(|p| matches!(p, EventPayload::GateAnswered { .. })) {
            journal.append(EventPayload::GateAnswered {
                gate_id: GATE.into(),
                answer: "approve".into(),
            })?;
        }
        let f = fold(&correction_run, &journal.read_all()?)?;
        if f.gates.len() != 1 || f.gates[0].snapshot_hash != binding {
            return Err(corrupt(&correction_run, "approval binding changed"));
        }
        // From here on the outcome is uncertain until the register answers.
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT.into(),
        })?;
        let request = WageCorrectionRequest {
            correction: plan.correction,
            hetu: plan.hetu,
            timestamp,
            status_timestamp: None,
        };
        let envelope = wage_correction_envelope(&request, false).map_err(store)?;
        let outcome = broker
            .submit(
                "process:file-wage-correction",
                &plan.preview.delivery_id,
                &envelope,
            )
            .map_err(store)?;
        record_outcome(&journal, EFFECT, outcome, false)?;
        self.wage_report_status(run_id)
    }

    /// Settle a correction sent without a definitive answer: first from the
    /// local outbox (a crash after an accepted send), otherwise with the signed
    /// status query. Never sends the correction again.
    pub fn reconcile_wage_correction(
        &self,
        run_id: &str,
        broker: &Broker,
        now: Instant,
    ) -> Result<WageReportStatus, ProcessError> {
        let status = self.wage_report_status(run_id)?;
        let Some(filing) = status.in_doubt() else {
            return Ok(status);
        };
        let correction_run = filing.run_id.clone();
        let _lease = self.acquire_lease(&correction_run)?;
        let Filing::Sent {
            correction,
            timestamp,
        } = self.read_correction_state(&correction_run)?.filing
        else {
            return Err(corrupt(
                &correction_run,
                "an e-service record cannot be in doubt",
            ));
        };
        let journal = Journal::new(&self.run_dir(&correction_run));
        if let Some(entry) = broker
            .outbox_entry(SVC_WAGE_CORRECTION, &correction.delivery_id(timestamp))
            .map_err(store)?
        {
            if let Some(success) = entry.terminal_success() {
                if let AttemptOutcome::Delivered { receipt }
                | AttemptOutcome::RecoveredAlreadyProcessed { receipt } = &success.outcome
                {
                    record_outcome(
                        &journal,
                        EFFECT,
                        RemoteOutcome::Delivered {
                            receipt: receipt.clone(),
                        },
                        true,
                    )?;
                    return self.wage_report_status(run_id);
                }
            }
        }
        // The status query carries no personal identity code.
        let request = WageCorrectionRequest {
            correction,
            hetu: String::new(),
            timestamp,
            status_timestamp: Some(now),
        };
        let envelope = wage_correction_envelope(&request, true).map_err(store)?;
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
                        "WAGE_CORRECTION_STATUS_UNKNOWN",
                        "status answer lacked the register references; keep the correction parked and ask again later",
                    ));
                }
                record_outcome(
                    &journal,
                    EFFECT,
                    RemoteOutcome::Delivered { receipt },
                    true,
                )?;
            }
            Err(BrokerError::Fetch { code, message }) if code == "TULOREKISTERI_REJECTED" => {
                record_outcome(
                    &journal,
                    EFFECT,
                    RemoteOutcome::Rejected { code, message },
                    true,
                )?;
            }
            Err(_) => {
                return Err(refused(
                    "WAGE_CORRECTION_STATUS_UNKNOWN",
                    "the register gave no definitive status; keep the correction parked and ask again later, or record e-service evidence with `file-wage correction complete-manual`",
                ))
            }
        }
        self.wage_report_status(run_id)
    }

    /// Record e-service evidence of a correction: either the answer to a
    /// correction sent without a definitive answer, or a replacement or
    /// cancellation the user made in the e-service. Sends nothing.
    pub fn complete_wage_correction_manual(
        &self,
        run_id: &str,
        action: WageCorrectionAction,
        receipt: EmployerReportReceipt,
    ) -> Result<WageReportStatus, ProcessError> {
        let receipt = EmployerReportReceipt {
            source: "e_service".into(),
            ..receipt
        };
        let invalid = |m: String| refused("WAGE_CORRECTION_INVALID_EVIDENCE", m);
        if !ir_reference(&receipt.ir_report_id)
            || receipt
                .ir_delivery_id
                .as_deref()
                .is_some_and(|d| !ir_reference(d))
            || receipt.version == Some(0)
        {
            return Err(invalid(
                "evidence needs the 32-character report reference from the e-service (and optionally the record reference and a positive version)".into(),
            ));
        }
        let status = self.wage_report_status(run_id)?;
        if let Some(filing) = status.in_doubt() {
            let correction_run = filing.run_id.clone();
            let Filing::Sent { correction, .. } =
                self.read_correction_state(&correction_run)?.filing
            else {
                return Err(corrupt(
                    &correction_run,
                    "an e-service record cannot be in doubt",
                ));
            };
            if correction.action != action || correction.ir_report_id != receipt.ir_report_id {
                return Err(invalid(format!(
                    "{correction_run} is an unsettled {} of report {}; the evidence describes something else. Settle it first with `file-wage correction status {run_id}`",
                    action_word(correction.action),
                    correction.ir_report_id
                )));
            }
            // The reference alone also matches the version being corrected, so
            // only a newer version proves the correction arrived.
            let Some(corrected) = correction.corrects_version else {
                return Err(invalid(format!(
                    "the corrected report's version was not recorded, so e-service evidence cannot show that this correction arrived; settle it with `file-wage correction status {run_id}`"
                )));
            };
            if receipt.version.is_none_or(|v| v <= corrected) {
                return Err(invalid(format!(
                    "settling a correction needs the e-service version of the report, newer than version {corrected}; an older version shows only that the correction has not arrived"
                )));
            }
            let _lease = self.acquire_lease(&correction_run)?;
            Journal::new(&self.run_dir(&correction_run)).append(
                EventPayload::EffectReconciled {
                    effect_key: EFFECT.into(),
                    note: receipt.note(),
                },
            )?;
            return self.wage_report_status(run_id);
        }
        let (current, sequence) = self.correctable(&status)?;
        // A replacement keeps the register's report reference, and a
        // cancellation is of that report: evidence naming another report is a
        // different report, recorded with its own `file-wage` Run.
        if receipt.ir_report_id != current.ir_report_id {
            return Err(invalid(format!(
                "the register holds this report as {}; the evidence names {}. A report given anew in the e-service is recorded with its own `file-wage start` and `complete-manual`",
                current.ir_report_id, receipt.ir_report_id
            )));
        }
        if let (Some(held), Some(given)) = (current.version, receipt.version) {
            if given <= held {
                return Err(invalid(format!(
                    "version {held} of the report is already recorded; a correction made in the e-service has a newer version"
                )));
            }
        }
        let correction_run = external_run_id(run_id, action, &receipt);
        let dir = self.run_dir(&correction_run);
        let _lease = self.acquire_lease(&correction_run)?;
        write_state(
            &dir.join(STATE_FILE),
            &CorrectionState {
                original_run_id: run_id.to_string(),
                sequence,
                filing: Filing::External {
                    action,
                    receipt: receipt.clone(),
                },
            },
        )?;
        let journal = Journal::new(&dir);
        journal.append(EventPayload::RunStarted {
            workflow: WORKFLOW_NAME.into(),
            workflow_version: version(),
            git_commit: crate::git_commit(),
            policy_set: "travel-only-2026".into(),
            period: status.report.payment_date.to_string(),
        })?;
        journal.append(EventPayload::EffectPlanned {
            effect_key: EFFECT.into(),
            effect_type: "RecordEServiceCorrection".into(),
        })?;
        journal.append(EventPayload::EffectReconciled {
            effect_key: EFFECT.into(),
            note: receipt.note(),
        })?;
        self.wage_report_status(run_id)
    }
}

fn action_word(action: WageCorrectionAction) -> &'static str {
    match action {
        WageCorrectionAction::Replacement => "replacement",
        WageCorrectionAction::Cancellation => "cancellation",
    }
}

fn sent_run_id(correction: &WageCorrection, timestamp: Instant) -> String {
    format!("wc-{}", correction.correction_hash(timestamp))
}

fn external_run_id(
    original_run_id: &str,
    action: WageCorrectionAction,
    receipt: &EmployerReportReceipt,
) -> String {
    format!(
        "wc-{}",
        record_hash(&format!(
            "external:{original_run_id}:{}:{}",
            action_word(action),
            receipt.note()
        ))
    )
}
