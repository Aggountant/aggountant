//! # aggountant-process (F10)
//!
//! The **git-native process engine** for workflow Runs (ADR-1 process namespace),
//! proven by the S2 thin `close-period` Run. It owns the non-authoritative
//! orchestration around a period close — the Run's `Journal`, `EffectState`,
//! `Lease` and `Gate` — and drives the **Ledger** for the authoritative fiscal
//! transitions (ADR-6). It never writes fiscal state directly and never stores the
//! `ClosingAuthority` (non-serialisable): on resume it re-derives authority from
//! the authoritative fiscal `PeriodState`, and **fiscal always wins**.
//!
//! Two v3 bets this crate exists to test (M2):
//! - **Derived Run state.** [`RunView`] status is *folded from effect states*
//!   (`run show`), never read from a materialized `status` field (`candidate.md
//!   §3`). See [`fold`].
//! - **Crash/resume correctness.** A crash between the fiscal COMMIT and the
//!   process `EffectState` write reconciles to fiscal on [`ProcessEngine::resume`]
//!   (ADR-6 §3). The crash seam is [`Seam::CrashAfterFiscalCommit`].
//!
//! The `close-period` workflow (thin S2 slice): `TransitionPeriod(Open→Closing)`
//! → `ApproveClose` gate → `SealPeriod(Closing→Sealed)`.

#![forbid(unsafe_code)]

mod capture_receipt;
mod close_year;
mod company_knowledge;
mod error;
mod file_vat;
mod hunt_receipts;
mod intake;
mod journal;
mod knowledge;
mod obligation;
mod run_payroll;

pub use capture_receipt::{
    prepare_structured_receipt, record_structured_receipt, CaptureReceiptInput, PreparedReceipt,
    ReceiptSourceFacts, ReceiptTreatmentProposal, ReceiptVatProposal, StructuredReceiptInput,
    ToolResult,
};
pub use close_year::{CloseYearInput, CloseYearStart, CorrectionInput};
pub use company_knowledge::{
    AddCompanyKnowledge, CompanyKnowledgeDocument, CompanyKnowledgeItem, CompanyKnowledgeRevision,
    CompanyKnowledgeSource, CompanyKnowledgeStatus, RetireCompanyKnowledge, UpdateCompanyKnowledge,
};
pub use file_vat::{FileVatInput, FileVatStart};
pub use hunt_receipts::{BankTransaction, HuntReceiptsInput};
pub use intake::{Intake, MatchCandidate, MatchStatus};
pub use knowledge::Knowledge;
pub use obligation::{Obligation, ObligationStatus, Obligations};

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sha2::{Digest, Sha256};

use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, PeriodState};
use aggountant_model::PeriodId;

pub use error::ProcessError;
pub use journal::{EventPayload, JournalEvent};

/// The `close-period` workflow definition source (the typed state machine lives
/// in code — `candidate.md §3`). Its hash is the `workflow_version` the resume
/// check pins. Bumping the definition bumps the hash → old Runs need `run migrate`.
const CLOSE_PERIOD_WORKFLOW: &str =
    "close-period/v2: TransitionPeriod(Open->Closing) -> gate:ApproveClose -> SealPeriod(Closing->Sealed)";

/// The `file-vat` workflow definition source (S3/RF3). Its hash is the version the
/// resume/answer check pins for a `file-vat` Run.
const FILE_VAT_WORKFLOW: &str = "file-vat/v2: Reconcile(vero-verotili) -> \
     gate:ApproveFiling(bind sent_hash) -> Submit(vero-api) -> file_vat(audited) -> \
     gate:ApprovePayment(bind sent_hash) -> Submit(pankki-maksu)";

/// The `run-payroll` workflow definition source (S4/RF4). Its hash is the version
/// the resume/answer check pins for a `run-payroll` Run.
const PAYROLL_WORKFLOW: &str = "run-payroll/v1: \
     per-employee[Compute(hetu-preflight + deterministic payslip)] -> \
     gate:ApproveRun(bind batch_hash) -> \
     per-employee[RecordVoucher(seal + ytd) -> Pay(pankki-maksu) -> \
     Report(tulorekisteri:palkkatietoilmoitus)] -> \
     Report(tulorekisteri:erillisilmoitus)";

/// The `capture-receipt` workflow definition source (S5/RF1). Its hash is the version
/// the resume/answer check pins for a `capture-receipt` Run.
const CAPTURE_RECEIPT_WORKFLOW: &str = "capture-receipt/v1: BuildDisclosure(projection) -> \
     gate:ApproveConsultDisclosure(bind sent_hash) -> Consult(consult-llm stream, re-bind per turn) -> \
     AssessEligibility(deterministic law) -> gate:ApproveVoucher(bind draft_hash) -> \
     Seal(voucher) + OpenObligation + gate:ApproveKnowledgePR(review-gated knowledge)";

/// The `close-year` workflow definition source (S6/RF6). Its hash is the version the
/// resume/answer check pins for a `close-year` Run (version-pin: a started Run
/// finishes on its start version — `workflow-migration-options.md §4` M1).
const CLOSE_YEAR_WORKFLOW: &str = "close-year/v2: \
     Precheck(subperiods-sealed + must-seal + D7-keys) -> Compute(poistot/jaksotukset) -> \
     gate:ApproveAdjustments -> Seal(adjustments to next-open) -> Compute(statements) -> \
     gate:ApproveTaseErittelyt -> gate:ApproveLiitetiedot -> \
     gate:ApproveStatements(bind statements_hash) -> SealYear(bind statements_hash)";

/// The `hunt-receipts` workflow definition source (S5/RF2). Its hash is the version the
/// resume/answer check pins for a `hunt-receipts` Run.
const HUNT_RECEIPTS_WORKFLOW: &str = "hunt-receipts/v1: \
     per-transaction[Pull(email/kauppatili, targeted) -> Effect::Wait(quota-backoff) -> \
     Intake(typed match candidate)] -> gate:ApproveMatches(per-item seal) -> \
     per-match[prepare-voucher -> voucher validate -> voucher seal]";

/// Effect keys for the two `close-period` effects (stable idempotency keys).
const EFFECT_TRANSITION: &str = "transition";
const EFFECT_SEAL: &str = "seal";
/// The single gate id.
const GATE_APPROVE_CLOSE: &str = "ApproveClose";
const ACTOR: &str = "process:close-period";
/// Lease TTL — a longer-held lease from a crashed driver is stealable.
const LEASE_TTL_SECS: i64 = 300;

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A test seam for the crash-injection gate (S2 invariant 4). Production callers
/// pass [`Seam::None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seam {
    None,
    /// Return immediately after the fiscal `begin_close` COMMIT, before writing
    /// the transition's `EffectSucceeded` event or opening the gate — the exact
    /// window ADR-6 §3 must survive (fiscal committed, process EffectState not).
    CrashAfterFiscalCommit,
    /// Return immediately after recording `GateAnswered(approve)`, before the
    /// `SealPeriod` effect runs — the approve-then-crash window `resume` must
    /// recover (otherwise the Run wedges: gate answered but never sealed).
    CrashAfterGateAnswer,
    /// (`file-vat`) Return immediately after the filing submit's **broker success**,
    /// before the fiscal `file_vat` COMMIT — the exact window the S3 spike-notes
    /// flag: the authority accepted the return (outbox has a terminal-success
    /// attempt) but fiscal never recorded `vat_filed`. `resume_file_vat` re-drives
    /// the idempotent submit (the outbox replays, no double filing) and commits.
    CrashAfterFilingBrokerSuccess,
    /// (`file-vat`) Return immediately after the fiscal `file_vat` COMMIT, before the
    /// filing's `EffectSucceeded` is journalled — fiscal is authoritative (`vat_filed`
    /// is set) but the process EffectState lags. `resume_file_vat` adopts fiscal
    /// (fiscal wins) without re-hitting the authority.
    CrashAfterFilingFiscalCommit,
    /// (`file-vat`) Return immediately after the filing's `EffectSucceeded` is journalled,
    /// before `finish_filing_postcondition` opens `ApprovePayment` (or records the
    /// no-payment completion) — the Run is left `Running` with no open gate, which
    /// `resume_file_vat` must un-wedge by converging the postcondition.
    CrashAfterFilingSuccess,
}

// ---------------------------------------------------------------------------
// Derived Run state (folded from the journal — never a stored status field)
// ---------------------------------------------------------------------------

/// The derived state of one effect (`candidate.md §3` EffectState machine, thin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectStatus {
    Planned,
    Executing,
    Succeeded,
    Rejected,
    /// Terminally **skipped** — a legitimate non-success (a discarded match, a seal a
    /// clarified transaction never needs). Terminal for run-completion, like `Succeeded`.
    Skipped,
    /// Parked on a **durable** `Effect::Wait` (S5): the effect hit a rate limit and
    /// recorded a `resume_after` deadline. Distinct from `Rejected` — it is not a
    /// failure, just "come back later"; `resume` re-drives it once the deadline passes.
    Waiting,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectView {
    pub key: String,
    pub effect_type: String,
    pub status: EffectStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct GateView {
    pub gate_id: String,
    pub gate_kind: String,
    pub snapshot_hash: String,
    /// `None` while open (parking the Run); `Some(answer)` once answered.
    pub answered: Option<String>,
}

/// The derived Run status. Not stored — computed from the effect/gate fold every
/// time `run show` is called (the M2 "does derived Run-state work?" question).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// Effects still to run and no gate parking us.
    Running,
    /// Parked on an open gate.
    AwaitingGate,
    /// Every planned effect Succeeded.
    Completed,
    /// A gate was answered "reject" — the close was declined.
    Rejected,
    /// An effect Rejected (partial failure) — needs a human/driver decision.
    NeedsAttention,
    /// An effect is parked on a durable `Effect::Wait` (a quota backoff, S5). Rendered
    /// distinctly from `NeedsAttention`: no human action is needed, just time.
    Waiting,
}

/// The whole derived view of a Run (`run show --json`).
#[derive(Debug, Clone, Serialize)]
pub struct RunView {
    pub run_id: String,
    pub workflow: String,
    pub workflow_version: String,
    pub period: String,
    pub policy_set: String,
    pub status: RunStatus,
    pub effects: Vec<EffectView>,
    pub gates: Vec<GateView>,
}

/// Metadata + derived state produced by folding a run's journal.
struct Fold {
    run_id: String,
    workflow: String,
    /// The version the Run currently runs under (RunStarted, unless migrated).
    effective_version: String,
    period: String,
    policy_set: String,
    effects: Vec<EffectView>,
    gates: Vec<GateView>,
    /// `effect_key → resume_after` (absolute unix seconds) for effects parked on a
    /// durable `Effect::Wait` (S5). The latest `EffectWaiting` for a key wins.
    waits: std::collections::HashMap<String, u64>,
    /// The ordered per-turn disclosure `sent_hash`es a multi-turn consult re-bound
    /// (S5/RF1) — the audit that every outbound turn re-projected + re-bound.
    disclosure_bindings: Vec<String>,
}

impl Fold {
    fn effect(&self, key: &str) -> Option<&EffectView> {
        self.effects.iter().find(|e| e.key == key)
    }
    /// The `resume_after` deadline (unix secs) an effect is waiting on, if any.
    fn wait_deadline(&self, key: &str) -> Option<u64> {
        self.effects
            .iter()
            .find(|e| e.key == key && e.status == EffectStatus::Waiting)
            .and_then(|_| self.waits.get(key).copied())
    }
    fn effect_succeeded(&self, key: &str) -> bool {
        self.effect(key)
            .map(|e| e.status == EffectStatus::Succeeded)
            .unwrap_or(false)
    }
    fn gate(&self, id: &str) -> Option<&GateView> {
        self.gates.iter().find(|g| g.gate_id == id)
    }
    /// Whether gate `id` has been answered `approve` (the crash-recovery convergence
    /// predicate the RF1/RF2 resume paths key off).
    fn gate_approved(&self, id: &str) -> bool {
        self.gate(id).and_then(|g| g.answered.as_deref()) == Some("approve")
    }
    fn status(&self) -> RunStatus {
        if self
            .gates
            .iter()
            .any(|g| g.answered.as_deref() == Some("reject"))
        {
            return RunStatus::Rejected;
        }
        if self
            .effects
            .iter()
            .any(|e| e.status == EffectStatus::Rejected)
        {
            return RunStatus::NeedsAttention;
        }
        if self
            .effects
            .iter()
            .any(|e| e.status == EffectStatus::Waiting)
        {
            return RunStatus::Waiting;
        }
        if self.gates.iter().any(|g| g.answered.is_none()) {
            return RunStatus::AwaitingGate;
        }
        if !self.effects.is_empty()
            && self
                .effects
                .iter()
                .all(|e| matches!(e.status, EffectStatus::Succeeded | EffectStatus::Skipped))
        {
            return RunStatus::Completed;
        }
        RunStatus::Running
    }
    fn to_view(&self) -> RunView {
        RunView {
            run_id: self.run_id.clone(),
            workflow: self.workflow.clone(),
            workflow_version: self.effective_version.clone(),
            period: self.period.clone(),
            policy_set: self.policy_set.clone(),
            status: self.status(),
            effects: self.effects.clone(),
            gates: self.gates.clone(),
        }
    }
}

/// Fold a journal into derived Run state. "The journal is the truth."
fn fold(run_id: &str, events: &[JournalEvent]) -> Result<Fold, ProcessError> {
    let mut f = Fold {
        run_id: run_id.to_string(),
        workflow: String::new(),
        effective_version: String::new(),
        period: String::new(),
        policy_set: String::new(),
        effects: Vec::new(),
        gates: Vec::new(),
        waits: std::collections::HashMap::new(),
        disclosure_bindings: Vec::new(),
    };
    for event in events {
        match &event.payload {
            EventPayload::RunStarted {
                workflow,
                workflow_version,
                policy_set,
                period,
                ..
            } => {
                f.workflow = workflow.clone();
                f.effective_version = workflow_version.clone();
                f.period = period.clone();
                f.policy_set = policy_set.clone();
            }
            EventPayload::EffectPlanned {
                effect_key,
                effect_type,
            } => f.effects.push(EffectView {
                key: effect_key.clone(),
                effect_type: effect_type.clone(),
                status: EffectStatus::Planned,
            }),
            EventPayload::EffectExecuting { effect_key } => {
                set_effect(&mut f, effect_key, EffectStatus::Executing)?
            }
            EventPayload::EffectSucceeded { effect_key, .. }
            | EventPayload::EffectReconciled { effect_key, .. } => {
                set_effect(&mut f, effect_key, EffectStatus::Succeeded)?
            }
            EventPayload::EffectRejected { effect_key, .. } => {
                set_effect(&mut f, effect_key, EffectStatus::Rejected)?
            }
            EventPayload::EffectSkipped { effect_key, .. } => {
                set_effect(&mut f, effect_key, EffectStatus::Skipped)?
            }
            EventPayload::EffectWaiting {
                effect_key,
                resume_after_epoch_secs,
                ..
            } => {
                set_effect(&mut f, effect_key, EffectStatus::Waiting)?;
                f.waits.insert(effect_key.clone(), *resume_after_epoch_secs);
            }
            EventPayload::DisclosureBound { sent_hash, .. } => {
                f.disclosure_bindings.push(sent_hash.clone());
            }
            EventPayload::GateOpened {
                gate_id,
                gate_kind,
                snapshot_hash,
            } => f.gates.push(GateView {
                gate_id: gate_id.clone(),
                gate_kind: gate_kind.clone(),
                snapshot_hash: snapshot_hash.clone(),
                answered: None,
            }),
            EventPayload::GateAnswered { gate_id, answer } => {
                let gate = f
                    .gates
                    .iter_mut()
                    .find(|g| &g.gate_id == gate_id)
                    .ok_or_else(|| {
                        ProcessError::JournalCorrupt(format!("answer for unknown gate {gate_id}"))
                    })?;
                gate.answered = Some(answer.clone());
            }
            EventPayload::WorkflowMigrated { to_version, .. } => {
                f.effective_version = to_version.clone();
            }
        }
    }
    Ok(f)
}

fn set_effect(f: &mut Fold, key: &str, status: EffectStatus) -> Result<(), ProcessError> {
    let e =
        f.effects.iter_mut().find(|e| e.key == key).ok_or_else(|| {
            ProcessError::JournalCorrupt(format!("state for unplanned effect {key}"))
        })?;
    e.status = status;
    Ok(())
}

// ---------------------------------------------------------------------------
// Clock (injected, so a durable Effect::Wait is tested without wall-clock sleeps)
// ---------------------------------------------------------------------------

/// The current time source for the durable `Effect::Wait` (S5). Production uses
/// [`SystemClock`]; tests inject a [`FixedClock`] and advance it past a persisted
/// `resume_after` deadline, so the wait is proven **without** ever sleeping and a
/// crash-across-the-wait resumes deterministically.
pub trait Clock {
    /// The current unix time in whole seconds.
    fn now_epoch_secs(&self) -> u64;
}

/// The wall-clock clock (production).
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_epoch_secs(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// A fixed, advanceable clock for tests.
pub struct FixedClock(std::cell::Cell<u64>);

impl FixedClock {
    pub fn new(now: u64) -> Self {
        FixedClock(std::cell::Cell::new(now))
    }
    /// Advance the clock by `secs` (simulating time passing across a wait).
    pub fn advance(&self, secs: u64) {
        self.0.set(self.0.get() + secs);
    }
}

impl Clock for FixedClock {
    fn now_epoch_secs(&self) -> u64 {
        self.0.get()
    }
}

/// The ceiling on a single quota wait (Fork C). A backoff longer than this parks the
/// Run `NeedsAttention` for the driver rather than waiting silently — an operator
/// notify threshold, not a correctness bound.
pub const MAX_WAIT_SECS: u64 = 86_400;

// ---------------------------------------------------------------------------
// The engine
// ---------------------------------------------------------------------------

/// Drives `close-period` Runs against a company home. Cheap to construct; opens a
/// fresh `Ledger` connection per fiscal step (the ADR-1 independent-connection
/// pattern).
pub struct ProcessEngine {
    home: PathBuf,
    /// The workflow version this engine's code implements. Overridable in tests to
    /// exercise `WORKFLOW_VERSION_MISMATCH` without recompiling.
    workflow_version: String,
}

impl ProcessEngine {
    pub fn open(home: &Path) -> Self {
        ProcessEngine {
            home: home.to_path_buf(),
            workflow_version: current_workflow_version(),
        }
    }

    /// Test constructor: pretend the engine's code is a different workflow version
    /// (drives the resume version check).
    pub fn open_with_version(home: &Path, workflow_version: impl Into<String>) -> Self {
        ProcessEngine {
            home: home.to_path_buf(),
            workflow_version: workflow_version.into(),
        }
    }

    fn process_root(&self) -> PathBuf {
        self.home.join("process").join("runs")
    }
    fn run_dir(&self, run_id: &str) -> PathBuf {
        self.process_root().join(run_id)
    }

    /// The F13 obligation store (git-native process namespace). Obligations live in
    /// process state and survive the Run that created them (`candidate.md §3`).
    pub fn obligations(&self) -> obligation::Obligations {
        obligation::Obligations::open(&self.home)
    }

    /// Consult Advice documents (git-native, review-gated writes). Company-local
    /// facts use the separate `company_knowledge` operations.
    pub fn knowledge(&self) -> knowledge::Knowledge {
        knowledge::Knowledge::open(&self.home)
    }

    // --- close-period -----------------------------------------------------

    /// Start a `close-period` Run: plan the effects, run `TransitionPeriod`
    /// (Open→Closing) and park on the `ApproveClose` gate. The Run's fiscal side
    /// (the CAS) is authoritative the instant `begin_close` COMMITs.
    pub fn start_close_period(
        &self,
        period: &PeriodId,
        policy_set: &PolicySet,
    ) -> Result<RunView, ProcessError> {
        self.start_close_period_seam(period, policy_set, Seam::None)
    }

    /// As [`start_close_period`], with a crash seam for the S2 crash-injection gate.
    pub fn start_close_period_seam(
        &self,
        period: &PeriodId,
        policy_set: &PolicySet,
        seam: Seam,
    ) -> Result<RunView, ProcessError> {
        // F13 gate (candidate.md §4 `OPEN_OBLIGATION`): a period with an open obligation
        // (e.g. an RF1-created "ajopäiväkirja puuttuu") cannot be closed until it is
        // resolved. The obligation lives in process state and survives the Run that
        // created it, so this bites even though a different Run opened it.
        let open = self.obligations().open_for_period(period)?;
        if !open.is_empty() {
            let ids: Vec<&str> = open.iter().map(|o| o.id.as_str()).collect();
            return Err(ProcessError::OpenObligation(format!(
                "period {period} has {} open obligation(s) [{}] — resolve them before closing",
                open.len(),
                ids.join(", ")
            )));
        }
        let run_id = new_run_id();
        let run_dir = self.run_dir(&run_id);
        fs::create_dir_all(&run_dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let _lease = self.acquire_lease(&run_id)?;
        let journal = journal::Journal::new(&run_dir);

        journal.append(EventPayload::RunStarted {
            workflow: "close-period".to_string(),
            workflow_version: self.workflow_version.clone(),
            git_commit: git_commit(),
            policy_set: policy_set.id.clone(),
            period: period.to_string(),
        })?;
        journal.append(EventPayload::EffectPlanned {
            effect_key: EFFECT_TRANSITION.to_string(),
            effect_type: "TransitionPeriod".to_string(),
        })?;
        journal.append(EventPayload::EffectPlanned {
            effect_key: EFFECT_SEAL.to_string(),
            effect_type: "SealPeriod".to_string(),
        })?;

        // Effect: TransitionPeriod(Open→Closing).
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_TRANSITION.to_string(),
        })?;
        let ledger = Ledger::open(&self.home)?;
        let auth = match ledger.begin_close(period, &run_id, ACTOR) {
            Ok(a) => a,
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_TRANSITION.to_string(),
                    error_code: e.error_code().to_string(),
                })?;
                return Err(e.into());
            }
        };

        if seam == Seam::CrashAfterFiscalCommit {
            // Simulated crash: fiscal has COMMITted (period is Closing) but the
            // process EffectState has NOT been advanced. Resume must reconcile.
            let events = journal.read_all()?;
            return Ok(fold(&run_id, &events)?.to_view());
        }

        journal.append(EventPayload::EffectSucceeded {
            effect_key: EFFECT_TRANSITION.to_string(),
            note: format!("period {period} → Closing"),
        })?;
        // The ClosingAuthority is intentionally dropped here — it is never
        // persisted; the seal step re-acquires it from authoritative fiscal state.
        drop(auth);

        self.open_close_gate(&journal, period)?;

        let events = journal.read_all()?;
        Ok(fold(&run_id, &events)?.to_view())
    }

    /// Answer the `ApproveClose` gate. `approve` runs `SealPeriod(Closing→Sealed)`;
    /// `reject` declines the close (the period stays Closing for the driver to
    /// resolve).
    pub fn answer(
        &self,
        run_id: &str,
        gate_id: &str,
        answer: &str,
    ) -> Result<RunView, ProcessError> {
        self.answer_seam(run_id, gate_id, answer, Seam::None)
    }

    /// As [`answer`], with a crash seam for the approve-then-crash resume gate.
    pub fn answer_seam(
        &self,
        run_id: &str,
        gate_id: &str,
        answer: &str,
        seam: Seam,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let events = journal.read_all()?;
        let f = fold(run_id, &events)?;
        self.check_version(&f)?;

        let gate = f.gate(gate_id).ok_or_else(|| {
            ProcessError::GateNotOpen(format!("no gate {gate_id} on run {run_id}"))
        })?;
        if gate.answered.is_some() {
            return Err(ProcessError::GateNotOpen(format!(
                "gate {gate_id} already answered"
            )));
        }
        if answer != "approve" && answer != "reject" {
            return Err(ProcessError::InvalidAnswer(format!(
                "ApproveClose accepts approve|reject, got {answer:?}"
            )));
        }

        let period = self.period_of(&f)?;
        if answer == "approve" {
            // Resume-check: recompute the snapshot hash and compare to the one the
            // gate bound to. It must match, because Closing blocked all writes
            // (`candidate.md §5` step 7) — a mismatch means the close is unsafe.
            let ledger = Ledger::open(&self.home)?;
            let recomputed = self.snapshot_hash(&ledger, &period)?;
            if recomputed != gate.snapshot_hash {
                return Err(ProcessError::JournalCorrupt(format!(
                    "period {period} snapshot changed under a Closing lock (bound {}, now {recomputed}) — aborting seal",
                    gate.snapshot_hash
                )));
            }
        }

        journal.append(EventPayload::GateAnswered {
            gate_id: gate_id.to_string(),
            answer: answer.to_string(),
        })?;

        if seam == Seam::CrashAfterGateAnswer {
            // Simulated crash: the gate is answered but SealPeriod has not run.
            return Ok(fold(run_id, &journal.read_all()?)?.to_view());
        }

        if answer == "approve" {
            self.run_seal(&journal, run_id, &period)?;
        }

        let events = journal.read_all()?;
        Ok(fold(run_id, &events)?.to_view())
    }

    /// Resume a Run: check the workflow version, reconcile any effect whose fiscal
    /// COMMIT landed but whose `EffectState` did not (ADR-6 §3 — fiscal wins), then
    /// converge to the parked-at-gate state.
    pub fn resume(&self, run_id: &str) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;

        let period = self.period_of(&f)?;
        let ledger = Ledger::open(&self.home)?;
        let fiscal = ledger.period_view(&period)?;
        let owns = fiscal.owning_run_id.as_deref() == Some(run_id);

        // Reconcile TransitionPeriod: if fiscal shows the period Closing/Sealed
        // under this run but the process never recorded success, adopt fiscal.
        let transition_done = f
            .effect(EFFECT_TRANSITION)
            .map(|e| e.status == EffectStatus::Succeeded)
            .unwrap_or(false);
        if !transition_done
            && owns
            && matches!(fiscal.state, PeriodState::Closing | PeriodState::Sealed)
        {
            journal.append(EventPayload::EffectReconciled {
                effect_key: EFFECT_TRANSITION.to_string(),
                note: format!(
                    "fiscal shows period {period} {} under this run — reconciled (fiscal wins)",
                    state_name(fiscal.state)
                ),
            })?;
        } else if !transition_done
            && fiscal.state == PeriodState::Open
            && fiscal.owning_run_id.is_none()
        {
            // The other crash window: we recorded EffectExecuting but crashed
            // *before* the fiscal CAS committed — the period is still Open and
            // unclaimed. Retry the first effect idempotently (Open→Closing).
            match ledger.begin_close(&period, run_id, ACTOR) {
                Ok(_) => journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_TRANSITION.to_string(),
                    note: format!("period {period} → Closing (retried on resume)"),
                })?,
                Err(e) => journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_TRANSITION.to_string(),
                    error_code: e.error_code().to_string(),
                })?,
            };
        }

        // Reconcile SealPeriod similarly (fiscal Sealed but process not updated).
        let seal_done = f
            .effect(EFFECT_SEAL)
            .map(|e| e.status == EffectStatus::Succeeded)
            .unwrap_or(false);
        if !seal_done && owns && fiscal.state == PeriodState::Sealed {
            journal.append(EventPayload::EffectReconciled {
                effect_key: EFFECT_SEAL.to_string(),
                note: format!("fiscal shows period {period} Sealed under this run — reconciled"),
            })?;
        }

        // Converge: if the transition is (now) done and no gate has been opened
        // and the seal is still outstanding, open the ApproveClose gate.
        let f2 = fold(run_id, &journal.read_all()?)?;
        let transition_ok = f2
            .effect(EFFECT_TRANSITION)
            .map(|e| e.status == EffectStatus::Succeeded)
            .unwrap_or(false);
        let seal_ok = f2
            .effect(EFFECT_SEAL)
            .map(|e| e.status == EffectStatus::Succeeded)
            .unwrap_or(false);
        if transition_ok && !seal_ok && f2.gate(GATE_APPROVE_CLOSE).is_none() {
            self.open_close_gate(&journal, &period)?;
        }

        // Approve-then-crash recovery: the operator approved the gate but the
        // process crashed before `SealPeriod` finished. If fiscal still shows this
        // run owns a Closing period, complete the seal now — SealPeriod is a CAS
        // the owner can safely repeat, so this is idempotent. Without this the Run
        // would be wedged (gate answered, so the converge branch above is skipped).
        let f3 = fold(run_id, &journal.read_all()?)?;
        let seal_done3 = f3
            .effect(EFFECT_SEAL)
            .map(|e| e.status == EffectStatus::Succeeded)
            .unwrap_or(false);
        let approved = f3
            .gate(GATE_APPROVE_CLOSE)
            .and_then(|g| g.answered.as_deref())
            == Some("approve");
        if approved && !seal_done3 {
            let fiscal2 = ledger.period_view(&period)?;
            if fiscal2.state == PeriodState::Closing
                && fiscal2.owning_run_id.as_deref() == Some(run_id)
            {
                self.run_seal(&journal, run_id, &period)?;
            }
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// `run show` — the derived view of the Run (no lease; a pure read).
    pub fn show(&self, run_id: &str) -> Result<RunView, ProcessError> {
        let journal = self.open_journal(run_id)?;
        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// The raw journal events (`journal show <run-id>`).
    pub fn journal(&self, run_id: &str) -> Result<Vec<JournalEvent>, ProcessError> {
        self.open_journal(run_id)?.read_all()
    }

    /// `run list` — the ids of all known runs.
    pub fn list(&self) -> Result<Vec<String>, ProcessError> {
        let root = self.process_root();
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut ids: Vec<String> = fs::read_dir(&root)
            .map_err(|e| ProcessError::Store(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        ids.sort();
        Ok(ids)
    }

    /// `run migrate` — record a workflow-version migration so the Run resumes
    /// under this engine's version (`candidate.md §9`).
    pub fn migrate(&self, run_id: &str) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        let target_version = self.expected_version(&f.workflow);
        if f.effective_version == target_version {
            return Err(ProcessError::InvalidAnswer(format!(
                "run {run_id} already at version {target_version}"
            )));
        }
        journal.append(EventPayload::WorkflowMigrated {
            from_version: f.effective_version.clone(),
            to_version: target_version,
        })?;
        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    // --- internals --------------------------------------------------------

    fn open_close_gate(
        &self,
        journal: &journal::Journal,
        period: &PeriodId,
    ) -> Result<(), ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        let snapshot_hash = self.snapshot_hash(&ledger, period)?;
        journal.append(EventPayload::GateOpened {
            gate_id: GATE_APPROVE_CLOSE.to_string(),
            gate_kind: "Approval".to_string(),
            snapshot_hash,
        })?;
        Ok(())
    }

    fn run_seal(
        &self,
        journal: &journal::Journal,
        run_id: &str,
        period: &PeriodId,
    ) -> Result<(), ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        // Re-derive authority from authoritative fiscal state (never from memory).
        let auth = ledger.acquire_authority(period, run_id)?;
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_SEAL.to_string(),
        })?;
        match ledger.seal_period(period, &auth, ACTOR) {
            Ok(_) => {
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SEAL.to_string(),
                    note: format!("period {period} → Sealed"),
                })?;
                Ok(())
            }
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SEAL.to_string(),
                    error_code: e.error_code().to_string(),
                })?;
                Err(e.into())
            }
        }
    }

    /// A thin `PeriodCloseSnapshotV1` stand-in (`candidate.md §3`): the gate binds
    /// the approval to it, and the resume-check recomputes and compares it. Because
    /// the Closing lock forbids writes, it is stable across the gate.
    fn snapshot_hash(&self, ledger: &Ledger, period: &PeriodId) -> Result<String, ProcessError> {
        let view = ledger.period_view(period)?;
        let chart_version = ledger.chart_version()?;
        let mut h = Sha256::new();
        for field in [
            view.period.as_str(),
            state_name(view.state),
            view.owning_run_id.as_deref().unwrap_or(""),
            &view.head_transition_hash,
            &chart_version,
        ] {
            h.update((field.len() as u64).to_be_bytes());
            h.update(field.as_bytes());
        }
        Ok(hex::encode(h.finalize()))
    }

    fn period_of(&self, f: &Fold) -> Result<PeriodId, ProcessError> {
        PeriodId::parse(&f.period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period {:?}: {e}", f.period)))
    }

    /// The workflow version this engine's code implements for `workflow`. Each
    /// workflow pins its own definition hash; the close-period override
    /// (`open_with_version`) still drives the close-period mismatch test.
    fn expected_version(&self, workflow: &str) -> String {
        match workflow {
            "file-vat" => file_vat_workflow_version(),
            "run-payroll" => payroll_workflow_version(),
            "capture-receipt" => capture_receipt_workflow_version(),
            "hunt-receipts" => hunt_receipts_workflow_version(),
            "close-year" => close_year_workflow_version(),
            _ => self.workflow_version.clone(),
        }
    }

    fn check_version(&self, f: &Fold) -> Result<(), ProcessError> {
        let expected = self.expected_version(&f.workflow);
        if f.effective_version != expected {
            return Err(ProcessError::WorkflowVersionMismatch(format!(
                "run started under {}, engine is {expected} — run migrate --confirm to continue",
                f.effective_version
            )));
        }
        Ok(())
    }

    fn open_journal(&self, run_id: &str) -> Result<journal::Journal, ProcessError> {
        let dir = self.run_dir(run_id);
        let journal = journal::Journal::new(&dir);
        if !journal.exists() {
            return Err(ProcessError::RunNotFound(run_id.to_string()));
        }
        Ok(journal)
    }

    /// Acquire the run's exclusive lease (`candidate.md §4`, `RUN_LEASED`). A lease
    /// whose expiry has passed (a crashed driver) is stealable. Released on drop.
    fn acquire_lease(&self, run_id: &str) -> Result<LeaseGuard, ProcessError> {
        let dir = self.run_dir(run_id);
        fs::create_dir_all(&dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = dir.join("lease");
        for _ in 0..2 {
            match OpenOptions::new().create_new(true).write(true).open(&path) {
                Ok(mut f) => {
                    let expires = chrono::Utc::now() + chrono::Duration::seconds(LEASE_TTL_SECS);
                    let _ = writeln!(f, "{}", expires.to_rfc3339());
                    let _ = f.sync_all();
                    return Ok(LeaseGuard { path });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Steal only if the existing lease has expired.
                    let stale = fs::read_to_string(&path)
                        .ok()
                        .and_then(|s| {
                            chrono::DateTime::parse_from_rfc3339(s.trim())
                                .ok()
                                .map(|t| t < chrono::Utc::now())
                        })
                        .unwrap_or(false);
                    if stale {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    return Err(ProcessError::RunLeased(format!(
                        "run {run_id} is leased by another driver"
                    )));
                }
                Err(e) => return Err(ProcessError::Store(e.to_string())),
            }
        }
        Err(ProcessError::RunLeased(format!(
            "run {run_id} lease contention did not settle"
        )))
    }
}

/// Releases the run lease when the driver's operation ends (or crashes/unwinds).
struct LeaseGuard {
    path: PathBuf,
}

impl Drop for LeaseGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// The current close-period workflow version = hash of its definition source
/// (`candidate.md §9`).
pub fn current_workflow_version() -> String {
    workflow_version_of(CLOSE_PERIOD_WORKFLOW)
}

/// The `file-vat` workflow version = hash of its definition source.
pub fn file_vat_workflow_version() -> String {
    workflow_version_of(FILE_VAT_WORKFLOW)
}

/// The `run-payroll` workflow version = hash of its definition source.
pub fn payroll_workflow_version() -> String {
    workflow_version_of(PAYROLL_WORKFLOW)
}

/// The `capture-receipt` workflow version = hash of its definition source.
pub fn capture_receipt_workflow_version() -> String {
    workflow_version_of(CAPTURE_RECEIPT_WORKFLOW)
}

/// The `hunt-receipts` workflow version = hash of its definition source.
pub fn hunt_receipts_workflow_version() -> String {
    workflow_version_of(HUNT_RECEIPTS_WORKFLOW)
}

/// The `close-year` workflow version = hash of its definition source (S6).
pub fn close_year_workflow_version() -> String {
    workflow_version_of(CLOSE_YEAR_WORKFLOW)
}

fn workflow_version_of(def: &str) -> String {
    let mut h = Sha256::new();
    h.update(def.as_bytes());
    format!("wf-{}", hex::encode(&h.finalize()[..8]))
}

fn git_commit() -> String {
    std::env::var("AGGOUNTANT_GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string())
}

fn state_name(s: PeriodState) -> &'static str {
    match s {
        PeriodState::Open => "Open",
        PeriodState::Closing => "Closing",
        PeriodState::Sealed => "Sealed",
    }
}

fn new_run_id() -> String {
    new_run_id_with("cp")
}

/// Mint a run id with a workflow-specific prefix (`cp` = close-period, `fv` =
/// file-vat). See [`new_run_id`] for the collision-avoidance rationale.
fn new_run_id_with(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = RUN_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    format!("{prefix}-{nanos:x}-{pid:x}-{n:x}")
}

mod file_wage;
pub use file_wage::WagePreview;
