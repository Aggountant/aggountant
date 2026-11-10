//! A single own-organisation travel report; shared journal/gates/outbox, no new engine.
//! Start/preview are offline. State contains NO HETU or XML: re-resolve PII and
//! re-bind exact unsigned canonical bytes at approval. The PII-derived full hash
//! is an ephemeral caller token, never part of git-native process state. Once dispatch is journalled,
//! only status reconciliation or evidenced manual completion is possible.
use crate::{
    fold,
    journal::{EventPayload, Journal},
    ProcessEngine, ProcessError, RunView,
};
use aggountant_broker::{
    tulorekisteri::{wage_envelope, WageRequest, SVC_WAGE},
    tulorekisteri_xml::{record_hash, wage_record},
    Broker, RemoteOutcome,
};
use aggountant_domain::WageReport;
use aggountant_ledger::Ledger;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write};
const WORKFLOW: &str = "file-wage/v1: Preview(bind canonical unsigned record) -> ApproveWage -> SendWageReport(at-most-once) -> signed GetDeliveryDataStatus";
const GATE: &str = "ApproveWage";
const EFFECT: &str = "submit-wage";
fn invalid(s: &str) -> ProcessError {
    ProcessError::InvalidAnswer(s.into())
}
fn version() -> String {
    aggountant_broker::tulorekisteri_xml::record_hash(WORKFLOW)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WageState {
    report: WageReport,
    timestamp: DateTime<Utc>,
    proposal_hash: String,
    identity_hash: String,
}
#[derive(Serialize)]
pub struct WagePreview {
    pub report: WageReport,
    pub timestamp: DateTime<Utc>,
    pub deadline: String,
    pub delivery_id: String,
    pub report_id: String,
    /// Sensitive PII-derived token: SHA-256 of the exact unsigned record with HETU.
    /// Do not persist/log it. Git-native state uses a separate non-PII proposal hash.
    /// Signature is added with the current valid credential ONLY after approval.
    pub record_hash: String,
}
impl ProcessEngine {
    fn wage_request(
        &self,
        report: &WageReport,
        timestamp: DateTime<Utc>,
    ) -> Result<WageRequest, ProcessError> {
        report
            .validate(timestamp.date_naive())
            .map_err(|e| invalid(&e.to_string()))?;
        let ledger = Ledger::open(&self.home)?;
        let employee = ledger.get_employee(&report.employee_id)?;
        Ok(WageRequest {
            report: report.clone(),
            hetu: employee.hetu,
            timestamp,
        })
    }
    pub fn preview_wage(
        &self,
        report: &WageReport,
        timestamp: DateTime<Utc>,
    ) -> Result<WagePreview, ProcessError> {
        let request = self.wage_request(report, timestamp)?;
        let xml = wage_record(report, &request.hetu, timestamp)
            .map_err(|_| invalid("invalid income earner HETU or wage record"))?;
        Ok(WagePreview {
            report: report.clone(),
            timestamp,
            deadline: report.deadline().to_string(),
            delivery_id: report.delivery_id(),
            report_id: report.report_id(),
            record_hash: record_hash(&xml),
        })
    }
    /// Natural identity also owns the local Run. Repeated start returns the same
    /// proposal; changed amounts/contact/PII/timestamp cannot create a second send.
    pub fn start_wage(
        &self,
        report: &WageReport,
        timestamp: DateTime<Utc>,
    ) -> Result<RunView, ProcessError> {
        // Validate PII without persisting its low-entropy-derived digest.
        self.preview_wage(report, timestamp)?;
        let proposal_hash = proposal_hash(report, timestamp)?;
        let run_id = format!("wr-{}", report.identity_hash());
        let dir = self.run_dir(&run_id);
        fs::create_dir_all(&dir).map_err(store)?;
        let _lease = self.acquire_lease(&run_id)?;
        let path = dir.join("wage.json");
        if path.exists() {
            let state = self.read_wage(&run_id)?;
            if state.proposal_hash != proposal_hash || state.identity_hash != report.identity_hash()
            {
                return Err(invalid("existing wage proposal differs; inspect existing run, do not create a new delivery identity"));
            }
        } else {
            let state = WageState {
                report: report.clone(),
                timestamp,
                proposal_hash,
                identity_hash: report.identity_hash(),
            };
            let bytes = serde_json::to_vec_pretty(&state)
                .map_err(|e| ProcessError::Store(e.to_string()))?;
            let tmp = path.with_extension("tmp");
            let mut f = fs::File::create(&tmp).map_err(store)?;
            f.write_all(&bytes).map_err(store)?;
            f.sync_all().map_err(store)?;
            fs::rename(&tmp, &path).map_err(store)?;
        }
        let journal = Journal::new(&dir);
        // Finish a partially initialized proposal, never rewrite its immutable state.
        let events = journal.read_all()?;
        if events.is_empty() {
            journal.append(EventPayload::RunStarted {
                workflow: "file-wage".into(),
                workflow_version: version(),
                git_commit: crate::git_commit(),
                policy_set: "travel-only-2026".into(),
                period: report.payment_date.to_string(),
            })?;
        }
        let events = journal.read_all()?;
        if !events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::EffectPlanned { .. }))
        {
            journal.append(EventPayload::EffectPlanned {
                effect_key: EFFECT.into(),
                effect_type: "SendWageReport".into(),
            })?;
        }
        let state = self.read_wage(&run_id)?;
        if !journal
            .read_all()?
            .iter()
            .any(|e| matches!(e.payload, EventPayload::GateOpened { .. }))
        {
            journal.append(EventPayload::GateOpened {
                gate_id: GATE.into(),
                gate_kind: "Preview".into(),
                snapshot_hash: state.proposal_hash,
            })?;
        }
        self.show(&run_id)
    }
    fn read_wage(&self, run_id: &str) -> Result<WageState, ProcessError> {
        // Identity is a full digest, never a caller-controlled path.
        if !run_id.starts_with("wr-")
            || run_id.len() != 67
            || !run_id[3..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid("invalid file-wage run id"));
        }
        let bytes = fs::read(self.run_dir(run_id).join("wage.json")).map_err(store)?;
        let state: WageState =
            serde_json::from_slice(&bytes).map_err(|_| invalid("corrupt wage state"))?;
        if run_id != format!("wr-{}", state.report.identity_hash())
            || state.identity_hash != state.report.identity_hash()
        {
            return Err(invalid("wage natural identity changed"));
        }
        Ok(state)
    }
    fn bound_wage(
        &self,
        run_id: &str,
    ) -> Result<(WageState, Vec<crate::JournalEvent>), ProcessError> {
        let state = self.read_wage(run_id)?;
        let events = Journal::new(&self.run_dir(run_id)).read_all()?;
        let f = fold(run_id, &events)?;
        if f.workflow != "file-wage" || f.effective_version != version() {
            return Err(invalid("file-wage workflow version mismatch"));
        }
        if f.gates.len() != 1
            || f.gates[0].snapshot_hash != state.proposal_hash
            || proposal_hash(&state.report, state.timestamp)? != state.proposal_hash
        {
            return Err(invalid("wage approval binding changed"));
        }
        Ok((state, events))
    }
    pub fn approve_wage(
        &self,
        run_id: &str,
        expected_hash: &str,
        broker: &Broker,
    ) -> Result<RunView, ProcessError> {
        self.read_wage(run_id)?;
        let _lease = self.acquire_lease(run_id)?;
        let (state, events) = self.bound_wage(run_id)?;
        // A crash anywhere after this marker is uncertain. Never call submit again,
        // even though the generic broker permits deliberate Unknown retries.
        if events.iter().any(|e| {
            matches!(
                e.payload,
                EventPayload::EffectExecuting { .. }
                    | EventPayload::EffectSucceeded { .. }
                    | EventPayload::EffectReconciled { .. }
            )
        }) {
            return self.show(run_id);
        }
        let request = self.wage_request(&state.report, state.timestamp)?;
        let current_hash = record_hash(
            &wage_record(&request.report, &request.hetu, request.timestamp)
                .map_err(|_| invalid("invalid wage PII or record"))?,
        );
        if expected_hash != current_hash {
            return Err(invalid(
                "approval must name the exact preview record_hash; report or PII changed",
            ));
        }
        let journal = Journal::new(&self.run_dir(run_id));
        if !events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::GateAnswered { .. }))
        {
            journal.append(EventPayload::GateAnswered {
                gate_id: GATE.into(),
                answer: "approve".into(),
            })?;
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT.into(),
        })?;
        let envelope =
            wage_envelope(&request, false).map_err(|_| invalid("invalid wage request"))?;
        let outcome = broker
            .submit("process:file-wage", &state.report.delivery_id(), &envelope)
            .map_err(|e| ProcessError::Store(e.to_string()))?;
        self.record_wage_outcome(run_id, outcome, false)?;
        self.show(run_id)
    }
    fn record_wage_outcome(
        &self,
        run_id: &str,
        outcome: RemoteOutcome,
        reconciled: bool,
    ) -> Result<(), ProcessError> {
        let journal = Journal::new(&self.run_dir(run_id));
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
                error_code: "OUTCOME_UNKNOWN_RECONCILE_ONLY".into(),
            },
            RemoteOutcome::Rejected { code, .. } => EventPayload::EffectRejected {
                effect_key: EFFECT.into(),
                error_code: code,
            },
        })?;
        Ok(())
    }
    pub fn reconcile_wage(
        &self,
        run_id: &str,
        broker: &Broker,
        now: DateTime<Utc>,
    ) -> Result<RunView, ProcessError> {
        self.read_wage(run_id)?;
        let _lease = self.acquire_lease(run_id)?;
        let (state, events) = self.bound_wage(run_id)?;
        if !events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::EffectExecuting { .. }))
        {
            return Err(invalid("no wage dispatch to reconcile"));
        }
        if events.iter().any(|e| {
            matches!(
                e.payload,
                EventPayload::EffectSucceeded { .. } | EventPayload::EffectReconciled { .. }
            )
        }) {
            return self.show(run_id);
        }
        // If dispatch succeeded before a process crash, the existing terminal outbox
        // success is sufficient; otherwise only the signed status operation is used.
        if let Some(entry) = broker
            .outbox_entry(SVC_WAGE, &state.report.delivery_id())
            .map_err(|e| ProcessError::Store(e.to_string()))?
        {
            if let Some(success) = entry.terminal_success() {
                let receipt = match &success.outcome {
                    aggountant_broker::AttemptOutcome::Delivered { receipt }
                    | aggountant_broker::AttemptOutcome::RecoveredAlreadyProcessed { receipt } => {
                        receipt.clone()
                    }
                    _ => unreachable!(),
                };
                self.record_wage_outcome(run_id, RemoteOutcome::Delivered { receipt }, true)?;
                return self.show(run_id);
            }
        }
        // A status query contains no employee identity. It remains possible after
        // employee PII correction/erasure, including across a process restart.
        let request = WageRequest {
            report: state.report.clone(),
            timestamp: now,
            hetu: String::new(),
        };
        let envelope =
            wage_envelope(&request, true).map_err(|_| invalid("invalid wage status request"))?;
        let body = match broker.fetch(&envelope) {
            Ok(body) => body,
            Err(aggountant_broker::BrokerError::Fetch { code, message })
                if code == "TULOREKISTERI_REJECTED" =>
            {
                self.record_wage_outcome(run_id, RemoteOutcome::Rejected { code, message }, true)?;
                return self.show(run_id);
            }
            Err(_) => {
                return Err(invalid(
                    "status not definitive; keep run parked and reconcile later",
                ))
            }
        };
        let delivery = body
            .get("ir_delivery_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid("missing reconciled delivery reference"))?;
        let report = body
            .get("ir_report_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid("missing reconciled report reference"))?;
        self.record_wage_outcome(
            run_id,
            RemoteOutcome::Delivered {
                receipt: format!("{delivery}/{report}/1"),
            },
            true,
        )?;
        self.show(run_id)
    }
    /// No automatic fallback. Operator must check e-service for a possibly accepted
    /// delivery FIRST and provide its remote delivery/report references.
    pub fn complete_wage_manual(
        &self,
        run_id: &str,
        ir_delivery_id: &str,
        ir_report_id: &str,
    ) -> Result<RunView, ProcessError> {
        self.read_wage(run_id)?;
        if !aggountant_broker::tulorekisteri::ir_reference(ir_delivery_id)
            || !aggountant_broker::tulorekisteri::ir_reference(ir_report_id)
        {
            return Err(invalid(
                "manual evidence requires remote delivery and report GUIDs from the e-service",
            ));
        }
        let _lease = self.acquire_lease(run_id)?;
        let (_, events) = self.bound_wage(run_id)?;
        if events.iter().any(|e| {
            matches!(
                e.payload,
                EventPayload::EffectSucceeded { .. } | EventPayload::EffectReconciled { .. }
            )
        }) {
            return self.show(run_id);
        }
        let journal = Journal::new(&self.run_dir(run_id));
        if !events
            .iter()
            .any(|e| matches!(e.payload, EventPayload::GateAnswered { .. }))
        {
            journal.append(EventPayload::GateAnswered {
                gate_id: GATE.into(),
                answer: "approve".into(),
            })?;
        }
        journal.append(EventPayload::EffectReconciled {
            effect_key: EFFECT.into(),
            note: format!("manual-e-service:{ir_delivery_id}/{ir_report_id}"),
        })?;
        self.show(run_id)
    }
}
fn store(e: std::io::Error) -> ProcessError {
    ProcessError::Store(e.to_string())
}

/// Git-native approval state binds ONLY non-PII content. The full record hash is
/// supplied ephemerally by the approving caller and checked against resolved PII
/// immediately before dispatch; only the private outbox keeps a PII-bearing digest.
fn proposal_hash(report: &WageReport, timestamp: DateTime<Utc>) -> Result<String, ProcessError> {
    let bytes = serde_json::to_string(&(report, timestamp))
        .map_err(|e| ProcessError::Store(e.to_string()))?;
    Ok(record_hash(&bytes))
}
