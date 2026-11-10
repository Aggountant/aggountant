//! The **`run-payroll` Run** (RF4, scenario C1) — the process orchestration of the
//! S4 build-spine slice. It composes the deterministic Domain (payslip law), the
//! Ledger (the F7 PII master-data + the sealed payroll voucher + YTD), and the
//! Broker (bank payment + the two Tulorekisteri reports, over the injected
//! transport) into one interruptible, stateful Run whose whole point is to validate
//! **partial-failure ergonomics at scale**: one employee fails while the rest
//! succeed.
//!
//! ## Shape (the `PAYROLL_WORKFLOW` definition)
//! Per employee, a **sequenced** chain — `Compute` (HETU-preflight + deterministic
//! payslip) → `RecordVoucher` (seal + YTD) → `Pay` (pankki-maksu) → `Report`
//! (tulorekisteri palkkatietoilmoitus) — and those per-employee chains are
//! **independent across employees** (the effect graph keys each by employee id, so a
//! failure in one never blocks another). A single `ApproveRun` `Preview` gate binds
//! the whole computed batch; a run-level `erillisilmoitus` (monthly Tulorekisteri
//! report) fires once every employee is paid + reported.
//!
//! Execution is a sequential iteration over employees (single-user / single-process
//! v1 — no threads, per the project's no-parallel-tools rule); the **parallelism is
//! modeled in the effect graph**, not the scheduler. That is exactly what the M4
//! gate probes: does per-effect `EffectState` + `NeedsAttention` scale across many
//! parallel units?
//!
//! ## The M4 invariants at the Run level
//! - A `pay`/`report` submit `Unknown` (timeout) parks **that employee's** effect in
//!   `NeedsAttention` — no blind retry — while the other employees complete;
//!   `handle_payment_failure(employee)` is the deliberate per-employee retry (the
//!   outbox + the authority's `409` recovery make it safe).
//! - **PII never enters the process store.** The run state persists only non-PII
//!   (amounts, tulolaji, bound `sent_hash`es). The `pay`/`report` effects **re-read
//!   the employee's PII through the Ledger** and rebuild the envelope, requiring its
//!   `sent_hash` to equal the hash the batch approval bound — the exact-bytes binding
//!   of `file-vat`, extended so IBAN/HETU stay in the encrypted store.
//! - A bad-master-data compute (`INVALID_HETU` / `EMPLOYEE_NOT_FOUND`) parks the
//!   employee **before** the gate; `resolve_errors(employee)` re-runs the compute
//!   after a master-data fix, and the gate opens only once every compute succeeds.

use std::fs;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use aggountant_broker::{Broker, Envelope, RemoteOutcome, SVC_PANKKI_MAKSU, SVC_TULOREKISTERI};
use aggountant_domain::{
    build_payroll_voucher, compute_payslip, erillisilmoitus, palkkatietoilmoitus, resolve_rates,
    validate_hetu, EmployeeIntent, PayrollIntent, Payslip, PolicySet, TulorekisteriReport,
};
use aggountant_ledger::{EmployeeState, Ledger, SealRequest};
use aggountant_model::{Date, Money, PeriodId};

use crate::journal::{EventPayload, Journal};
use crate::{fold, new_run_id_with, EffectStatus, ProcessEngine, ProcessError, RunView};

const GATE_APPROVE_RUN: &str = "ApproveRun";
const EFFECT_ERILLISILMOITUS: &str = "erillisilmoitus";
const ACTOR: &str = "process:run-payroll";

/// Per-employee effect-key prefixes.
fn k_compute(id: &str) -> String {
    format!("compute:{id}")
}
fn k_record(id: &str) -> String {
    format!("record:{id}")
}
fn k_pay(id: &str) -> String {
    format!("pay:{id}")
}
fn k_report(id: &str) -> String {
    format!("report:{id}")
}

// ---------------------------------------------------------------------------
// Run-scoped state (NON-PII) — persisted so answer/retry/resume rebuild the
// approved envelopes deterministically. PII (IBAN/HETU) is deliberately absent;
// it is re-read from the Ledger's encrypted store at dispatch time.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PayrollState {
    payment_date: String,
    posting_date: String,
    period: String,
    policy_set: String,
    rates_version: String,
    employees: Vec<EmployeePlan>,
    /// Populated when the batch is finalized (gate open); `None` before that.
    erillisilmoitus: Option<Erillisilmoitus>,
}

/// One successfully-computed employee's non-PII plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmployeePlan {
    employee_id: String,
    tulolaji: String,
    /// The computed payslip (all Money/strings — no PII).
    payslip: Payslip,
    /// `sent_hash` of the (PII-bearing) pay envelope — the bytes, not the body.
    pay_sent_hash: String,
    /// `sent_hash` of the (PII-bearing) palkkatietoilmoitus envelope.
    report_sent_hash: String,
    /// The palkkatietoilmoitus filing deadline (payment_date + 5 days).
    report_deadline: String,
}

/// The finalized monthly employer summary (non-PII aggregate).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Erillisilmoitus {
    month: String,
    total_employer_tyel: String,
    total_employer_sava: String,
    total_employer_unemployment: String,
    total_withholding: String,
    sent_hash: String,
}

impl ProcessEngine {
    fn payroll_state_path(&self, run_id: &str) -> std::path::PathBuf {
        self.run_dir(run_id).join("payroll_state.json")
    }

    fn write_payroll_state(&self, run_id: &str, state: &PayrollState) -> Result<(), ProcessError> {
        let bytes =
            serde_json::to_vec_pretty(state).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = self.payroll_state_path(run_id);
        let tmp = path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| ProcessError::Store(e.to_string()))?;
            f.write_all(&bytes)
                .map_err(|e| ProcessError::Store(e.to_string()))?;
            f.sync_all()
                .map_err(|e| ProcessError::Store(e.to_string()))?;
        }
        fs::rename(&tmp, &path).map_err(|e| ProcessError::Store(e.to_string()))?;
        Ok(())
    }

    fn read_payroll_state(&self, run_id: &str) -> Result<PayrollState, ProcessError> {
        let bytes = fs::read(self.payroll_state_path(run_id))
            .map_err(|e| ProcessError::Store(format!("payroll state unreadable: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("payroll state corrupt: {e}")))
    }

    /// Start a `run-payroll` Run from a [`PayrollIntent`]: resolve the dated rates,
    /// run each employee's `Compute` effect (HETU-preflight + deterministic payslip),
    /// and — once **every** compute succeeds — finalize the batch and open the
    /// `ApproveRun` gate. A compute that fails on master data parks that employee
    /// `NeedsAttention` (no gate opens) for `resolve_errors`.
    pub fn start_run_payroll(
        &self,
        intent: &PayrollIntent,
        policy_set: &PolicySet,
    ) -> Result<RunView, ProcessError> {
        // Resolve the statutory rates before creating the run — a run with no rate
        // edition for its payment_date is a precondition failure, not a parked Run.
        let rates = resolve_rates(intent.payment_date).map_err(map_domain)?;
        if intent.employees.is_empty() {
            return Err(ProcessError::InvalidAnswer(
                "a payroll run needs at least one employee".into(),
            ));
        }
        // Reject duplicate employee ids: they would collide on effect keys, voucher /
        // payment natural keys, and silently drop a pay line on the plan upsert.
        let mut seen = std::collections::HashSet::new();
        for e in &intent.employees {
            if !seen.insert(e.employee_id.as_str()) {
                return Err(ProcessError::InvalidAnswer(format!(
                    "duplicate employee_id {:?} in the payroll intent",
                    e.employee_id
                )));
            }
        }

        let run_id = new_run_id_with("rp");
        let run_dir = self.run_dir(&run_id);
        fs::create_dir_all(&run_dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let _lease = self.acquire_lease(&run_id)?;
        let journal = Journal::new(&run_dir);

        // Persist the original intent (non-PII: gross + tulolaji) so `resolve_errors`
        // can re-run a compute that never produced a plan.
        self.write_intent_sidecar(&run_id, intent)?;

        let period = PeriodId::containing(intent.payment_date);
        journal.append(EventPayload::RunStarted {
            workflow: "run-payroll".to_string(),
            workflow_version: crate::payroll_workflow_version(),
            git_commit: crate::git_commit(),
            policy_set: policy_set.id.clone(),
            period: period.to_string(),
        })?;

        // Plan the per-employee effect graph + the run-level erillisilmoitus.
        for e in &intent.employees {
            for (key, ty) in [
                (k_compute(&e.employee_id), "Compute"),
                (k_record(&e.employee_id), "RecordVoucher"),
                (k_pay(&e.employee_id), "ExchangeAdapter(pankki-maksu)"),
                (
                    k_report(&e.employee_id),
                    "ExchangeAdapter(tulorekisteri:palkkatietoilmoitus)",
                ),
            ] {
                journal.append(EventPayload::EffectPlanned {
                    effect_key: key,
                    effect_type: ty.to_string(),
                })?;
            }
        }
        journal.append(EventPayload::EffectPlanned {
            effect_key: EFFECT_ERILLISILMOITUS.to_string(),
            effect_type: "ExchangeAdapter(tulorekisteri:erillisilmoitus)".to_string(),
        })?;

        let mut state = PayrollState {
            payment_date: intent.payment_date.to_string(),
            posting_date: intent.payment_date.to_string(),
            period: period.to_string(),
            policy_set: policy_set.id.clone(),
            rates_version: rates.rates_version.clone(),
            employees: Vec::new(),
            erillisilmoitus: None,
        };

        let ledger = Ledger::open(&self.home)?;
        for e in &intent.employees {
            self.run_compute(&journal, &e.employee_id, e, &rates, &ledger, &mut state)?;
        }
        self.write_payroll_state(&run_id, &state)?;

        // Open the gate only when the whole batch computed cleanly.
        self.finalize_batch_if_ready(&journal, &run_id, &mut state)?;

        Ok(fold(&run_id, &journal.read_all()?)?.to_view())
    }

    /// One employee's `Compute` effect: HETU-preflight + deterministic payslip +
    /// envelope binding. On success, appends/updates the employee's plan in `state`.
    /// Bad master data (`EMPLOYEE_NOT_FOUND` / `INVALID_HETU`) or a compute error
    /// parks the effect Rejected — it does not abort the run.
    fn run_compute(
        &self,
        journal: &Journal,
        emp_id: &str,
        intent: &EmployeeIntent,
        rates: &aggountant_domain::ResolvedRates,
        ledger: &Ledger,
        state: &mut PayrollState,
    ) -> Result<(), ProcessError> {
        let key = k_compute(emp_id);
        journal.append(EventPayload::EffectExecuting {
            effect_key: key.clone(),
        })?;

        let employee = match ledger.get_employee(emp_id) {
            Ok(e) => e,
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: key,
                    error_code: e.error_code().to_string(),
                })?;
                return Ok(());
            }
        };
        // HETU-preflight — a bad HETU parks the employee here, before any authority.
        if let Err(e) = validate_hetu(&employee.hetu) {
            journal.append(EventPayload::EffectRejected {
                effect_key: key,
                error_code: e.error_code().to_string(),
            })?;
            return Ok(());
        }
        let payslip = match compute_payslip(intent, &employee.tax_card, rates) {
            Ok(p) => p,
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: key,
                    error_code: e.error_code().to_string(),
                })?;
                return Ok(());
            }
        };

        let payment_date = Date::parse(&state.payment_date)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad payment_date: {e}")))?;
        let pay_env = build_pay_env(emp_id, &employee, &payslip, &state.payment_date);
        let report_env = build_report_env(
            emp_id,
            &employee,
            &payslip,
            &state.payment_date,
            payment_date,
        );
        let pay_sent_hash = pay_env.sent_hash().map_err(map_broker)?;
        let report_sent_hash = report_env.sent_hash().map_err(map_broker)?;
        let deadline = match palkkatietoilmoitus(payment_date) {
            TulorekisteriReport::Palkkatietoilmoitus { deadline, .. } => deadline.to_string(),
            _ => unreachable!(),
        };

        let plan = EmployeePlan {
            employee_id: emp_id.to_string(),
            tulolaji: intent.tulolaji.clone(),
            payslip: payslip.clone(),
            pay_sent_hash,
            report_sent_hash,
            report_deadline: deadline,
        };
        // Upsert (a resolve re-runs compute for an already-planned employee).
        match state.employees.iter_mut().find(|p| p.employee_id == emp_id) {
            Some(existing) => *existing = plan,
            None => state.employees.push(plan),
        }

        journal.append(EventPayload::EffectSucceeded {
            effect_key: key,
            note: format!(
                "net {} (rates {})",
                payslip.net.amount_str(),
                state.rates_version
            ),
        })?;
        Ok(())
    }

    /// If every planned `Compute` effect has succeeded and the gate is not yet open,
    /// finalize the monthly aggregate and open the `ApproveRun` preview gate bound to
    /// the batch hash. Idempotent (no-op if a compute is still parked, or if already
    /// open) — so `start` and `resolve_errors` share it.
    fn finalize_batch_if_ready(
        &self,
        journal: &Journal,
        run_id: &str,
        state: &mut PayrollState,
    ) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.gate(GATE_APPROVE_RUN).is_some() {
            return Ok(());
        }
        let all_computed = f
            .effects
            .iter()
            .all(|e| !e.key.starts_with("compute:") || e.status == EffectStatus::Succeeded)
            && f.effects.iter().any(|e| e.key.starts_with("compute:"));
        if !all_computed {
            return Ok(());
        }

        // Finalize the monthly employer summary from the computed payslips.
        let payment_date = Date::parse(&state.payment_date)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad payment_date: {e}")))?;
        let month = match erillisilmoitus(payment_date) {
            TulorekisteriReport::TyonantajanErillisilmoitus { month } => month,
            _ => unreachable!(),
        };
        let sum = |f: fn(&Payslip) -> &Money| -> String {
            let mut total = rust_decimal::Decimal::ZERO;
            for p in &state.employees {
                total += f(&p.payslip).amount();
            }
            total.rescale(2);
            total.to_string()
        };
        let eril = Erillisilmoitus {
            month: month.clone(),
            total_employer_tyel: sum(|p| &p.employer_tyel),
            total_employer_sava: sum(|p| &p.employer_sava),
            total_employer_unemployment: sum(|p| &p.employer_unemployment),
            total_withholding: sum(|p| &p.withholding),
            sent_hash: String::new(),
        };
        let eril_env = build_erillisilmoitus_env(run_id, &eril);
        let eril_sent_hash = eril_env.sent_hash().map_err(map_broker)?;
        let eril = Erillisilmoitus {
            sent_hash: eril_sent_hash,
            ..eril
        };
        state.erillisilmoitus = Some(eril);
        self.write_payroll_state(run_id, state)?;

        let batch_hash = batch_hash(state);
        journal.append(EventPayload::GateOpened {
            gate_id: GATE_APPROVE_RUN.to_string(),
            gate_kind: "Preview".to_string(),
            snapshot_hash: batch_hash,
        })?;
        Ok(())
    }

    /// Re-run a parked `Compute` effect for `employee_id` after its master data was
    /// fixed (`ResolveErrors`). Re-reads the employee through the Ledger; if the
    /// compute now succeeds and the whole batch is ready, opens the `ApproveRun` gate.
    pub fn resolve_errors(&self, run_id: &str, employee_id: &str) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;

        let compute_key = k_compute(employee_id);
        if f.effect(&compute_key).map(|e| e.status) != Some(EffectStatus::Rejected) {
            return Err(ProcessError::InvalidAnswer(format!(
                "compute for {employee_id} is not parked — nothing to resolve"
            )));
        }
        if f.gate(GATE_APPROVE_RUN).is_some() {
            return Err(ProcessError::InvalidAnswer(
                "the batch is already approved — use handle-payment-failure for a parked effect"
                    .into(),
            ));
        }

        let mut state = self.read_payroll_state(run_id)?;
        let rates = resolve_rates(
            Date::parse(&state.payment_date)
                .map_err(|e| ProcessError::JournalCorrupt(format!("bad payment_date: {e}")))?,
        )
        .map_err(map_domain)?;
        // Pin the rate edition: if the bundled rate data changed while this run was
        // parked, refuse rather than silently mix editions within one run (a
        // determinism failure). The operator starts a fresh run under the new rates.
        if rates.rates_version != state.rates_version {
            return Err(ProcessError::WorkflowVersionMismatch(format!(
                "statutory rates changed since this run started (run {}, binary {}) — \
                 start a new payroll run under the current rates",
                state.rates_version, rates.rates_version
            )));
        }
        // The gross + tulolaji are re-read from the original intent sidecar: a
        // compute that failed on master data (e.g. EMPLOYEE_NOT_FOUND) never produced
        // a plan, so the intent is the only place its gross still lives.
        let intent = self.read_intent_line(run_id, employee_id)?;
        let ledger = Ledger::open(&self.home)?;
        self.run_compute(&journal, employee_id, &intent, &rates, &ledger, &mut state)?;
        self.write_payroll_state(run_id, &state)?;
        self.finalize_batch_if_ready(&journal, run_id, &mut state)?;

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Answer the `ApproveRun` gate. `approve` runs the whole batch (each employee's
    /// `record → pay → report` chain, independent across employees); `reject`
    /// declines the run. Approve requires every compute resolved and re-verifies the
    /// batch hash (the approval signs the exact computed batch).
    pub fn answer_run_payroll(
        &self,
        run_id: &str,
        answer: &str,
        broker: &Broker,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;

        let gate = f.gate(GATE_APPROVE_RUN).ok_or_else(|| {
            ProcessError::GateNotOpen(format!("ApproveRun is not open on run {run_id}"))
        })?;
        if gate.answered.is_some() {
            return Err(ProcessError::GateNotOpen(
                "ApproveRun already answered".into(),
            ));
        }
        if answer != "approve" && answer != "reject" {
            return Err(ProcessError::InvalidAnswer(format!(
                "ApproveRun accepts approve|reject, got {answer:?}"
            )));
        }

        let mut state = self.read_payroll_state(run_id)?;
        if answer == "approve" {
            let recomputed = batch_hash(&state);
            if recomputed != gate.snapshot_hash {
                return Err(ProcessError::JournalCorrupt(format!(
                    "the payroll batch changed after approval was bound (bound {}, now {recomputed})",
                    gate.snapshot_hash
                )));
            }
        }

        journal.append(EventPayload::GateAnswered {
            gate_id: GATE_APPROVE_RUN.to_string(),
            answer: answer.to_string(),
        })?;

        if answer == "approve" {
            // Drive every employee's chain. A parked (Unknown) effect leaves that
            // employee NeedsAttention but never blocks the next employee.
            let ids: Vec<String> = state
                .employees
                .iter()
                .map(|p| p.employee_id.clone())
                .collect();
            for id in ids {
                self.drive_employee(&journal, run_id, &id, &mut state, broker)?;
            }
            self.maybe_run_erillisilmoitus(&journal, run_id, &state, broker)?;
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Deliberately re-drive a parked employee chain (`HandlePaymentFailure`): the
    /// only retry, never automatic. Safe because the bank/Tulorekisteri submits are
    /// idempotent (outbox + `409` recovery). Continues the employee from wherever it
    /// parked, then runs the monthly report if the whole batch is now done.
    pub fn handle_payment_failure(
        &self,
        run_id: &str,
        employee_id: &str,
        broker: &Broker,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;

        if f.gate(GATE_APPROVE_RUN).and_then(|g| g.answered.as_deref()) != Some("approve") {
            return Err(ProcessError::GateNotOpen(
                "the run is not approved — nothing to re-drive".into(),
            ));
        }
        // Recover any employee whose chain is not fully done — a Rejected effect
        // (Unknown/parked) OR an Executing/Planned one (a crash mid-chain). This is
        // what makes a crash mid-batch recoverable per-employee; the re-drive is
        // idempotent (outbox/409 + idempotent seal/YTD).
        let incomplete = [
            k_record(employee_id),
            k_pay(employee_id),
            k_report(employee_id),
        ]
        .iter()
        .any(|k| f.effect(k).map(|e| e.status) != Some(EffectStatus::Succeeded));
        if !incomplete {
            return Err(ProcessError::InvalidAnswer(format!(
                "{employee_id} is already fully processed — nothing to handle"
            )));
        }

        let mut state = self.read_payroll_state(run_id)?;
        self.drive_employee(&journal, run_id, employee_id, &mut state, broker)?;
        self.maybe_run_erillisilmoitus(&journal, run_id, &state, broker)?;

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Resume an interrupted `run-payroll` Run (crash recovery). Reconciles the whole
    /// batch to the durable state: if the `ApproveRun` gate was lost after the batch
    /// was finalized, re-open it; if the batch was already approved, re-drive **every**
    /// employee whose chain is incomplete (Planned/Executing/Rejected) and then the
    /// monthly report. Every step is idempotent (outbox/409 + idempotent seal/YTD), so
    /// resume is safe to run repeatedly. Mirrors `close-period`/`file-vat` resume.
    pub fn resume_run_payroll(
        &self,
        run_id: &str,
        broker: &Broker,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        let mut state = self.read_payroll_state(run_id)?;

        match f.gate(GATE_APPROVE_RUN).map(|g| g.answered.clone()) {
            // No gate yet: a crash after the state was finalized but before GateOpened.
            // Re-open it if the batch is ready (idempotent — no-op if a compute parked).
            None => {
                self.finalize_batch_if_ready(&journal, run_id, &mut state)?;
            }
            // Approved: re-drive incomplete employees + the monthly report.
            Some(Some(a)) if a == "approve" => {
                let ids: Vec<String> = state
                    .employees
                    .iter()
                    .map(|p| p.employee_id.clone())
                    .collect();
                for id in ids {
                    let ff = fold(run_id, &journal.read_all()?)?;
                    let done = [k_record(&id), k_pay(&id), k_report(&id)]
                        .iter()
                        .all(|k| ff.effect(k).map(|e| e.status) == Some(EffectStatus::Succeeded));
                    if !done {
                        self.drive_employee(&journal, run_id, &id, &mut state, broker)?;
                    }
                }
                self.maybe_run_erillisilmoitus(&journal, run_id, &state, broker)?;
            }
            // Open-but-unanswered (awaiting the human) or rejected: nothing to drive.
            _ => {}
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Drive one employee's sequenced chain from wherever it stands: `record → pay →
    /// report`. Each step is idempotent within the Run; the chain stops at the first
    /// parked/failed step (a payment that never landed does not trigger a report).
    fn drive_employee(
        &self,
        journal: &Journal,
        run_id: &str,
        employee_id: &str,
        state: &mut PayrollState,
        broker: &Broker,
    ) -> Result<(), ProcessError> {
        let plan = state
            .employees
            .iter()
            .find(|p| p.employee_id == employee_id)
            .cloned()
            .ok_or_else(|| {
                ProcessError::InvalidAnswer(format!("no computed plan for {employee_id}"))
            })?;

        // --- RecordVoucher: seal the payroll voucher + advance YTD ---
        if !self.effect_ok(journal, run_id, &k_record(employee_id))?
            && !self.record_voucher(journal, &plan, state)?
        {
            return Ok(()); // record failed → do not pay
        }
        // --- Pay: bank payment (re-read PII, rebind, submit) ---
        if !self.effect_ok(journal, run_id, &k_pay(employee_id))?
            && !self.submit_pay(journal, &plan, state, broker)?
        {
            return Ok(()); // parked → do not report
        }
        // --- Report: palkkatietoilmoitus ---
        if !self.effect_ok(journal, run_id, &k_report(employee_id))? {
            self.submit_report(journal, &plan, state, broker)?;
        }
        Ok(())
    }

    fn effect_ok(&self, journal: &Journal, run_id: &str, key: &str) -> Result<bool, ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        Ok(f.effect(key).map(|e| e.status) == Some(EffectStatus::Succeeded))
    }

    /// Seal one employee's payroll voucher (idempotent on its natural key) and
    /// advance YTD. Returns `true` on success, `false` if it parked (e.g. the period
    /// is not Open).
    fn record_voucher(
        &self,
        journal: &Journal,
        plan: &EmployeePlan,
        state: &PayrollState,
    ) -> Result<bool, ProcessError> {
        let key = k_record(&plan.employee_id);
        journal.append(EventPayload::EffectExecuting {
            effect_key: key.clone(),
        })?;
        let document_date = Date::parse(&state.posting_date)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad date: {e}")))?;
        let voucher = match build_payroll_voucher(
            &plan.payslip,
            document_date,
            document_date,
            format!("Palkka {} {}", plan.employee_id, state.payment_date),
        ) {
            Ok(v) => v,
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: key,
                    error_code: e.error_code().to_string(),
                })?;
                return Ok(false);
            }
        };
        let ledger = Ledger::open(&self.home)?;
        let natural_key = format!("payroll:{}:{}", plan.employee_id, state.payment_date);
        let seal = ledger.seal(
            &voucher,
            &SealRequest {
                natural_key: natural_key.clone(),
                actor: ACTOR.to_string(),
                policy_set: PolicySet {
                    id: state.policy_set.clone(),
                },
            },
        );
        match seal {
            Ok(sealed) => {
                // Advance YTD for the pay run's tax year (the year of the payment
                // date — Finnish YTD is attributed by maksupäivä), keyed by the pay
                // run's natural key. The seal (fiscal store) and this advance (PII
                // store) are separate transactions, so a crash in between leaves the
                // voucher sealed but the YTD row missing — a transient understatement,
                // NOT a lost write: `accumulate_ytd` is idempotent on the natural key,
                // so the re-drive (resume / handle-payment-failure) replays the seal
                // and applies the YTD exactly once, repairing the window. Year-scoping
                // keeps a new tax year from accumulating onto the prior one. A failure
                // here parks the effect (it does not abort the batch); the re-drive is
                // safe.
                let payment_date = Date::parse(&state.payment_date)
                    .map_err(|e| ProcessError::JournalCorrupt(format!("bad payment_date: {e}")))?;
                let tax_year = PeriodId::containing(payment_date).year();
                if let Err(e) = ledger.accumulate_ytd(
                    &plan.employee_id,
                    tax_year,
                    &natural_key,
                    &plan.payslip.gross,
                ) {
                    journal.append(EventPayload::EffectRejected {
                        effect_key: key,
                        error_code: e.error_code().to_string(),
                    })?;
                    return Ok(false);
                }
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: key,
                    note: format!("voucher #{} sealed", sealed.number),
                })?;
                Ok(true)
            }
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: key,
                    error_code: e.error_code().to_string(),
                })?;
                Ok(false)
            }
        }
    }

    /// Submit one employee's bank payment: re-read PII, rebuild the envelope, require
    /// its `sent_hash` to equal the approved binding, then submit. `Unknown` parks
    /// `NeedsAttention` (no blind retry). Returns `true` on terminal success. Any
    /// per-employee error (PII drift, deleted employee, outbox failure) **parks the
    /// effect** rather than aborting the whole batch (the M4 isolation invariant).
    fn submit_pay(
        &self,
        journal: &Journal,
        plan: &EmployeePlan,
        state: &PayrollState,
        broker: &Broker,
    ) -> Result<bool, ProcessError> {
        let key = k_pay(&plan.employee_id);
        let ledger = Ledger::open(&self.home)?;
        let employee = match ledger.get_employee(&plan.employee_id) {
            Ok(e) => e,
            Err(e) => return self.park(journal, &key, e.error_code()),
        };
        let env = build_pay_env(
            &plan.employee_id,
            &employee,
            &plan.payslip,
            &state.payment_date,
        );
        if let Err(code) = self.check_bound(&env, &plan.pay_sent_hash) {
            return self.park(journal, &key, &code);
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: key.clone(),
        })?;
        match broker.submit(
            ACTOR,
            &format!("pay:{}:{}", plan.employee_id, state.payment_date),
            &env,
        ) {
            Ok(outcome) => self.record_submit_outcome(journal, &key, outcome),
            Err(e) => self.park(journal, &key, e.error_code()),
        }
    }

    /// Submit one employee's palkkatietoilmoitus (per-payment Tulorekisteri report,
    /// 5-day deadline). Same PII re-read + rebind discipline as the payment, and the
    /// same per-employee parking (never aborts the batch).
    fn submit_report(
        &self,
        journal: &Journal,
        plan: &EmployeePlan,
        state: &PayrollState,
        broker: &Broker,
    ) -> Result<bool, ProcessError> {
        let key = k_report(&plan.employee_id);
        let ledger = Ledger::open(&self.home)?;
        let employee = match ledger.get_employee(&plan.employee_id) {
            Ok(e) => e,
            Err(e) => return self.park(journal, &key, e.error_code()),
        };
        let payment_date = Date::parse(&state.payment_date)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad payment_date: {e}")))?;
        let env = build_report_env(
            &plan.employee_id,
            &employee,
            &plan.payslip,
            &state.payment_date,
            payment_date,
        );
        if let Err(code) = self.check_bound(&env, &plan.report_sent_hash) {
            return self.park(journal, &key, &code);
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: key.clone(),
        })?;
        match broker.submit(
            ACTOR,
            &format!("palkkatieto:{}:{}", plan.employee_id, state.payment_date),
            &env,
        ) {
            Ok(outcome) => self.record_submit_outcome(journal, &key, outcome),
            Err(e) => self.park(journal, &key, e.error_code()),
        }
    }

    /// Park an effect Rejected with `error_code` and return `Ok(false)` — the "this
    /// employee failed, keep driving the rest" outcome (never an `Err` that would
    /// abort the batch).
    fn park(&self, journal: &Journal, key: &str, error_code: &str) -> Result<bool, ProcessError> {
        journal.append(EventPayload::EffectRejected {
            effect_key: key.to_string(),
            error_code: error_code.to_string(),
        })?;
        Ok(false)
    }

    /// The run-level monthly report: fire the työnantajan erillisilmoitus once every
    /// employee is paid + reported. Idempotent (a re-drive after a parked
    /// erillisilmoitus retries it).
    fn maybe_run_erillisilmoitus(
        &self,
        journal: &Journal,
        run_id: &str,
        state: &PayrollState,
        broker: &Broker,
    ) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect(EFFECT_ERILLISILMOITUS).map(|e| e.status) == Some(EffectStatus::Succeeded) {
            return Ok(());
        }
        // Every employee's pay + report must be done first.
        let all_done = state.employees.iter().all(|p| {
            f.effect(&k_pay(&p.employee_id)).map(|e| e.status) == Some(EffectStatus::Succeeded)
                && f.effect(&k_report(&p.employee_id)).map(|e| e.status)
                    == Some(EffectStatus::Succeeded)
        });
        if !all_done {
            return Ok(());
        }
        let Some(eril) = &state.erillisilmoitus else {
            return Ok(());
        };
        let env = build_erillisilmoitus_env(run_id, eril);
        if let Err(code) = self.check_bound(&env, &eril.sent_hash) {
            self.park(journal, EFFECT_ERILLISILMOITUS, &code)?;
            return Ok(());
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_ERILLISILMOITUS.to_string(),
        })?;
        // The monthly report's outbox/natural key is run-scoped so two payroll runs in
        // the same month never collide (which would let dedup silently drop the second).
        match broker.submit(
            ACTOR,
            &format!("erillisilmoitus:{run_id}:{}", eril.month),
            &env,
        ) {
            Ok(outcome) => {
                self.record_submit_outcome(journal, EFFECT_ERILLISILMOITUS, outcome)?;
            }
            Err(e) => {
                self.park(journal, EFFECT_ERILLISILMOITUS, e.error_code())?;
            }
        }
        Ok(())
    }

    /// Map a Broker submit outcome to the effect's terminal event. `Unknown` →
    /// Rejected(`OUTCOME_UNKNOWN`) (parked, retryable). Returns `true` on success.
    fn record_submit_outcome(
        &self,
        journal: &Journal,
        key: &str,
        outcome: RemoteOutcome,
    ) -> Result<bool, ProcessError> {
        match outcome {
            RemoteOutcome::Delivered { receipt }
            | RemoteOutcome::RecoveredAlreadyProcessed { receipt } => {
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: key.to_string(),
                    // The receipt is remote-controlled text landing in the git-native
                    // journal — sanitize it (bounded, alphanumeric/-/_/. only) so an
                    // authority cannot echo PII or markup into a committable file.
                    note: format!("receipt {}", sanitize_receipt(&receipt)),
                })?;
                Ok(true)
            }
            RemoteOutcome::Unknown => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: key.to_string(),
                    error_code: "OUTCOME_UNKNOWN".to_string(),
                })?;
                Ok(false)
            }
            RemoteOutcome::Rejected { code, message } => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: key.to_string(),
                    error_code: format!("{code}: {message}"),
                })?;
                Ok(false)
            }
        }
    }

    /// Check that a rebuilt envelope still hashes to the bound `sent_hash` — the
    /// approval signs the exact bytes, even though the PII in them is re-read from the
    /// encrypted store rather than persisted in the process state. On mismatch (the
    /// employee's master data changed after approval) returns the error code to park
    /// with, so the caller refuses to send unapproved bytes without aborting the batch.
    fn check_bound(&self, env: &Envelope, bound: &str) -> Result<(), String> {
        let recomputed = env.sent_hash().map_err(|e| e.error_code().to_string())?;
        if recomputed != bound {
            return Err("PAYLOAD_CHANGED_AFTER_APPROVAL".to_string());
        }
        Ok(())
    }

    /// Persist the original per-employee intent (gross + tulolaji) as a sidecar so
    /// `resolve_errors` can re-run a compute that never produced a plan (e.g. an
    /// `EMPLOYEE_NOT_FOUND` reject). Non-PII.
    fn write_intent_sidecar(
        &self,
        run_id: &str,
        intent: &PayrollIntent,
    ) -> Result<(), ProcessError> {
        let bytes =
            serde_json::to_vec_pretty(intent).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = self.run_dir(run_id).join("payroll_intent.json");
        let tmp = path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| ProcessError::Store(e.to_string()))?;
            f.write_all(&bytes)
                .map_err(|e| ProcessError::Store(e.to_string()))?;
            f.sync_all()
                .map_err(|e| ProcessError::Store(e.to_string()))?;
        }
        fs::rename(&tmp, &path).map_err(|e| ProcessError::Store(e.to_string()))?;
        Ok(())
    }

    fn read_intent_line(
        &self,
        run_id: &str,
        employee_id: &str,
    ) -> Result<EmployeeIntent, ProcessError> {
        let bytes = fs::read(self.run_dir(run_id).join("payroll_intent.json"))
            .map_err(|e| ProcessError::Store(format!("payroll intent unreadable: {e}")))?;
        let intent: PayrollIntent = serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("payroll intent corrupt: {e}")))?;
        intent
            .employees
            .into_iter()
            .find(|e| e.employee_id == employee_id)
            .ok_or_else(|| {
                ProcessError::InvalidAnswer(format!("{employee_id} not in the original intent"))
            })
    }
}

// ---------------------------------------------------------------------------
// Envelope builders (deterministic; PII is read from the employee record and
// hashed into sent_hash but never persisted in the process store)
// ---------------------------------------------------------------------------

fn build_pay_env(
    emp_id: &str,
    employee: &EmployeeState,
    payslip: &Payslip,
    payment_date: &str,
) -> Envelope {
    Envelope::new(
        SVC_PANKKI_MAKSU,
        "pay-salary",
        format!("pay:{emp_id}:{payment_date}"),
        json!({
            "employee_id": emp_id,
            "iban": employee.iban,
            "amount": payslip.net.amount_str(),
            "payment_date": payment_date,
        }),
    )
}

fn build_report_env(
    emp_id: &str,
    employee: &EmployeeState,
    payslip: &Payslip,
    payment_date: &str,
    payment_date_typed: Date,
) -> Envelope {
    let deadline = match palkkatietoilmoitus(payment_date_typed) {
        TulorekisteriReport::Palkkatietoilmoitus { deadline, .. } => deadline.to_string(),
        _ => unreachable!(),
    };
    Envelope::new(
        SVC_TULOREKISTERI,
        "palkkatietoilmoitus",
        format!("palkkatieto:{emp_id}:{payment_date}"),
        json!({
            "report_kind": "palkkatietoilmoitus",
            "employee_id": emp_id,
            "hetu": employee.hetu,
            "payment_date": payment_date,
            "deadline": deadline,
            "tulolaji": payslip.tulolaji,
            "gross": payslip.gross.amount_str(),
            "withholding": payslip.withholding.amount_str(),
            "employee_tyel": payslip.employee_tyel.amount_str(),
            "employee_unemployment": payslip.employee_unemployment.amount_str(),
            "employer_tyel": payslip.employer_tyel.amount_str(),
            "employer_sava": payslip.employer_sava.amount_str(),
            "employer_unemployment": payslip.employer_unemployment.amount_str(),
        }),
    )
}

fn build_erillisilmoitus_env(run_id: &str, eril: &Erillisilmoitus) -> Envelope {
    Envelope::new(
        SVC_TULOREKISTERI,
        "erillisilmoitus",
        // Run-scoped so two payroll runs in the same month get distinct natural keys.
        format!("erillisilmoitus:{run_id}:{}", eril.month),
        json!({
            "report_kind": "tyonantajan_erillisilmoitus",
            "run_id": run_id,
            "month": eril.month,
            "total_employer_tyel": eril.total_employer_tyel,
            "total_employer_sava": eril.total_employer_sava,
            "total_employer_unemployment": eril.total_employer_unemployment,
            "total_withholding": eril.total_withholding,
        }),
    )
}

/// The batch-approval binding: a hash over the run-level fields + the ordered
/// per-employee bindings + the monthly report binding. Each employee's
/// `pay_sent_hash`/`report_sent_hash` already covers its (PII-bearing) envelope bytes,
/// so this binds the whole approved batch without the process store holding PII; the
/// run-level fields (posting_date/period/policy_set/rates_version) close the gap where
/// editing the sidecar could otherwise change a sealed voucher under a stale approval.
fn batch_hash(state: &PayrollState) -> String {
    let mut h = Sha256::new();
    let mut feed = |s: &str| {
        h.update((s.len() as u64).to_be_bytes());
        h.update(s.as_bytes());
    };
    feed(&state.payment_date);
    feed(&state.posting_date);
    feed(&state.period);
    feed(&state.policy_set);
    feed(&state.rates_version);
    for p in &state.employees {
        feed(&p.employee_id);
        feed(&p.payslip.net.amount_str());
        feed(&p.pay_sent_hash);
        feed(&p.report_sent_hash);
    }
    if let Some(eril) = &state.erillisilmoitus {
        feed(&eril.sent_hash);
    }
    hex::encode(h.finalize())
}

/// Bound, character-restricted sanitization of a remote-controlled receipt before it
/// is written into the git-native journal (keeps PII/markup out of a committable file).
fn sanitize_receipt(receipt: &str) -> String {
    let cleaned: String = receipt
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(64)
        .collect();
    if cleaned.is_empty() {
        "<none>".to_string()
    } else {
        cleaned
    }
}

/// A domain error surfaced by the payroll Run is a user/config error (a missing rate
/// edition, a bad tulolaji), not a system failure — classify it as `user` so the CLI
/// exits 1, not 2.
fn map_domain(e: aggountant_domain::DomainError) -> ProcessError {
    ProcessError::InvalidAnswer(format!("{}: {e}", e.error_code()))
}

fn map_broker(e: aggountant_broker::BrokerError) -> ProcessError {
    ProcessError::Store(format!("{}: {e}", e.error_code()))
}
