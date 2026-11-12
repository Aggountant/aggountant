//! The **`close-year` Run** (RF6/S6, scenario D2) — the longest, branchiest firm
//! Run. It orchestrates the year-end close: precondition + must-seal gating,
//! deterministic poistot/jaksotukset (Domain), the statements-from-policy projection
//! over the sealed chain, four sequential human check-gates, the D7 pre-close
//! reclassifications, and the irreversible **`SealYear`** — with crash-resume at RF6
//! length and version-pin workflow migration (`s6-spec-ahead.md`,
//! `workflow-migration-options.md §4`).
//!
//! Shape (the `CLOSE_YEAR_WORKFLOW` definition):
//! `Precheck(subperiods Sealed + must-seal + D7 keys)` → `Compute(poistot/jaksotukset)`
//! → **`ApproveAdjustments`** (Approval) → `Seal(adjustment vouchers)` →
//! `Compute(statements)` → **`ApproveTaseErittelyt`** (Approval) →
//! **`ApproveLiitetiedot`** (Approval) → **`ApproveStatements`** (Preview, binds the
//! exact statement bytes) → `SealYear`.
//!
//! **Model decision (documented as the headline M6 scope gap in `spike-notes.md`):**
//! the year's selected twelve months (or four retained legacy quarters) are `Sealed`
//! before close-year starts, so year-end adjustments + D7 corrections are booked
//! with the A10/O-21 rule —
//! **sealed-period corrections post to the next open period** — via the ordinary seal
//! path. The statements are the deterministic projection over the sealed year-N chain.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use aggountant_domain::{
    accrual_voucher, compute_depreciation, compute_statements, depreciation_policy_version,
    depreciation_rate_table, depreciation_voucher, kpa_policy, kpa_policy_version, tase_erittelyt,
    AccrualInput, AssetInput, DepreciationLine, Liitetiedot, PolicySet, StatementProvenance,
    Statements,
};
use aggountant_ledger::{FiscalYearState, Ledger, PeriodState};
use aggountant_model::{AccountId, Date, Money, PeriodId};

use crate::journal::{EventPayload, Journal};
use crate::{fold, EffectStatus, Fold, ProcessEngine, ProcessError, RunView};

const EFFECT_COMPUTE_ADJ: &str = "compute-adjustments";
const EFFECT_SEAL_ADJ: &str = "seal-adjustments";
const EFFECT_COMPUTE_STMT: &str = "compute-statements";
const EFFECT_SEAL_YEAR: &str = "seal-year";
const GATE_APPROVE_ADJ: &str = "ApproveAdjustments";
const GATE_APPROVE_TE: &str = "ApproveTaseErittelyt";
const GATE_APPROVE_LT: &str = "ApproveLiitetiedot";
const GATE_APPROVE_STMT: &str = "ApproveStatements";
const ACTOR: &str = "process:close-year";

/// The effect key for one D7 correction (`correction:<id>`).
fn correction_effect_key(id: &str) -> String {
    format!("correction:{id}")
}

/// A D7 pre-close reclassification (`voucher correct`): a balanced adjustment sealed
/// on its **own** approval (thin commit boundary), before `ApproveStatements`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionInput {
    pub correction_id: String,
    pub description: String,
    pub debit_account: AccountId,
    pub credit_account: AccountId,
    pub amount: Money,
}

/// Structured input for `close-year` (CLI stdin JSON).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CloseYearInput {
    pub year: i32,
    /// Assets to depreciate (poistot). Amounts are Domain-computed, never here.
    #[serde(default)]
    pub assets: Vec<AssetInput>,
    /// Accruals to book (jaksotukset).
    #[serde(default)]
    pub accruals: Vec<AccrualInput>,
    /// D7 pre-close reclassifications, each sealed on its own approval.
    #[serde(default)]
    pub corrections: Vec<CorrectionInput>,
}

/// Run-scoped state persisted so answer/resume re-derive the exact drafts + the
/// bound statement bytes (the Preview binding depends on this being durable).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloseYearState {
    year: i32,
    policy_set: String,
    subperiods: Vec<String>,
    next_open_period: String,
    /// `YYYY-MM-DD` document date for the adjustments (fiscal-year end).
    doc_date: String,
    /// `YYYY-MM-DD` posting date for the adjustments (next open period).
    posting_date: String,
    input: CloseYearInput,
    /// The computed statements, once `compute-statements` has run.
    statements: Option<Statements>,
}

/// The start of a `close-year` Run (the `RunView` plus the computed adjustment
/// preview a CLI echoes for the `ApproveAdjustments` gate).
#[derive(Debug, Clone, Serialize)]
pub struct CloseYearStart {
    #[serde(flatten)]
    pub run: RunView,
    pub depreciation: Vec<DepreciationLine>,
}

impl ProcessEngine {
    fn close_year_state_path(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("close_year_state.json")
    }

    fn write_close_year_state(
        &self,
        run_id: &str,
        state: &CloseYearState,
    ) -> Result<(), ProcessError> {
        let bytes =
            serde_json::to_vec_pretty(state).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = self.close_year_state_path(run_id);
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

    fn read_close_year_state(&self, run_id: &str) -> Result<CloseYearState, ProcessError> {
        let bytes = fs::read(self.close_year_state_path(run_id))
            .map_err(|e| ProcessError::Store(format!("close-year state unreadable: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("close-year state corrupt: {e}")))
    }

    /// Start a `close-year` Run: check preconditions (all subperiods `Sealed`,
    /// must-seal, D7 keys), compute the poistot/jaksotukset, persist the plan, and
    /// park on the `ApproveAdjustments` gate.
    pub fn start_close_year(
        &self,
        input: &CloseYearInput,
        policy_set: &PolicySet,
    ) -> Result<CloseYearStart, ProcessError> {
        let year = input.year;
        let ledger = Ledger::open(&self.home)?;

        // The year's dates come from the company's fiscal calendar; `year` is the
        // calendar year the fiscal year ends in.
        let fiscal_year = ledger.fiscal_year(year)?;

        // Precondition A: the year must not already be sealed (idempotent refusal).
        let fy = ledger.fiscal_year_view(year)?;
        if fy.state == FiscalYearState::Sealed {
            return Err(ProcessError::InvalidAnswer(format!(
                "fiscal year {year} is already sealed — SealYear is irreversible"
            )));
        }

        // Precondition B: every selected subperiod must be Sealed. Active monthly
        // years require all twelve months; untouched legacy years retain four quarters.
        let subperiods = ledger.year_close_periods(year)?;
        let mut blockers = Vec::new();
        for p in &subperiods {
            if ledger.period_view(p)?.state != PeriodState::Sealed {
                blockers.push(format!("subperiod-not-sealed:{p}"));
            }
        }
        // Precondition C (must-seal, ADR-5): open obligations for any subperiod, and
        // unresolved (Pending) intake candidates dated within the year, block the close.
        let mut obligation_ids = std::collections::BTreeSet::new();
        for p in &subperiods {
            for obligation in self.obligations().open_for_period(p)? {
                obligation_ids.insert(obligation.id);
            }
        }
        blockers.extend(
            obligation_ids
                .into_iter()
                .map(|id| format!("open-obligation:{id}")),
        );
        for c in crate::intake::Intake::open(&self.home).list()? {
            if c.status == crate::intake::MatchStatus::Pending
                && Date::parse(&c.date).is_ok_and(|date| fiscal_year.contains(date))
            {
                blockers.push(format!("unresolved-intake:{}", c.id));
            }
        }
        if !blockers.is_empty() {
            return Err(ProcessError::CloseYearBlocked(format!(
                "close-year for {year} is blocked by {} item(s): [{}]",
                blockers.len(),
                blockers.join(", ")
            )));
        }

        // Ids become natural keys / effect keys — a duplicate would silently
        // idempotent-replay (dropping the second entry) or collide two effects. Reject
        // empty or duplicate ids up front.
        assert_unique_ids("asset_id", input.assets.iter().map(|a| a.asset_id.as_str()))?;
        assert_unique_ids(
            "accrual_id",
            input.accruals.iter().map(|a| a.accrual_id.as_str()),
        )?;
        assert_unique_ids(
            "correction_id",
            input.corrections.iter().map(|c| c.correction_id.as_str()),
        )?;

        // Compute the actual first open period in the next fiscal year for adjustments.
        // A calendar target year owns its granularity once active; otherwise it
        // continues the closing year's mode (monthly books continue at January, legacy
        // books at Q1). A non-calendar fiscal year is closed by its months only.
        let next_year = ledger
            .fiscal_calendar()?
            .next(&fiscal_year)
            .map_err(aggountant_ledger::LedgerError::from)?;
        let target_year = next_year.year();
        let target_periods = if next_year.is_calendar_year() {
            let target_months = (1..=12)
                .map(|month| PeriodId::new_month(target_year, month))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| ProcessError::Store(format!("bad next-open year: {e}")))?;
            let target_quarters = (1..=4)
                .map(|quarter| PeriodId::new(target_year, quarter))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| ProcessError::Store(format!("bad next-open year: {e}")))?;
            let month_active = target_months.iter().try_fold(false, |active, period| {
                Ok::<_, ProcessError>(
                    active || ledger.period_view(period)?.state != PeriodState::Open,
                )
            })?;
            let quarter_active = target_quarters.iter().try_fold(false, |active, period| {
                Ok::<_, ProcessError>(
                    active || ledger.period_view(period)?.state != PeriodState::Open,
                )
            })?;
            if month_active && quarter_active {
                return Err(ProcessError::CloseYearBlocked(format!(
                    "close-year for {year} is blocked: target year {target_year} has mixed active month and quarter coverage"
                )));
            }
            if month_active
                || (!quarter_active && subperiods.first().is_some_and(PeriodId::is_month))
            {
                target_months
            } else {
                target_quarters
            }
        } else {
            next_year
                .month_periods()
                .map_err(|e| ProcessError::Store(format!("bad next-open year: {e}")))?
        };
        let next_open = target_periods
            .into_iter()
            .find_map(|period| match ledger.period_view(&period) {
                Ok(view) if view.state == PeriodState::Open => Some(Ok(period)),
                Ok(_) => None,
                Err(error) => Some(Err(ProcessError::from(error))),
            })
            .transpose()?
            .ok_or_else(|| {
                ProcessError::CloseYearBlocked(format!(
                    "close-year for {year} is blocked: target year {target_year} has no open adjustment period"
                ))
            })?;

        // Compute poistot (Domain) — a bad asset class / unresolvable rate fails here.
        let doc_date = fiscal_year.end();
        let posting_date =
            Date::from_ymd(next_open.year(), u32::from(next_open.month_bounds().0), 1)
                .map_err(|e| ProcessError::Store(format!("bad next-open date: {e}")))?;
        // A depreciation refusal (unknown class, a fiscal year that is not 12 months)
        // is something the caller must resolve before the close can start.
        let depreciation =
            compute_depreciation(&fiscal_year, &input.assets, depreciation_rate_table()).map_err(
                |e| {
                    ProcessError::CloseYearBlocked(format!(
                        "close-year for {year} is blocked: {}: {e}",
                        e.error_code()
                    ))
                },
            )?;
        // Validate every adjustment + correction draft assembles now (fail fast).
        for line in &depreciation {
            depreciation_voucher(line, doc_date, posting_date).map_err(map_domain)?;
        }
        for a in &input.accruals {
            accrual_voucher(a, doc_date, posting_date).map_err(map_domain)?;
        }
        for c in &input.corrections {
            build_correction_draft(c, doc_date, posting_date)?;
        }

        let run_id = crate::new_run_id_with("cy");
        let run_dir = self.run_dir(&run_id);
        fs::create_dir_all(&run_dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let _lease = self.acquire_lease(&run_id)?;
        let journal = Journal::new(&run_dir);

        self.write_close_year_state(
            &run_id,
            &CloseYearState {
                year,
                policy_set: policy_set.id.clone(),
                subperiods: subperiods.iter().map(|p| p.to_string()).collect(),
                next_open_period: next_open.to_string(),
                doc_date: doc_date.to_string(),
                posting_date: posting_date.to_string(),
                input: input.clone(),
                statements: None,
            },
        )?;

        journal.append(EventPayload::RunStarted {
            workflow: "close-year".to_string(),
            workflow_version: crate::close_year_workflow_version(),
            git_commit: crate::git_commit(),
            policy_set: policy_set.id.clone(),
            period: subperiods
                .last()
                .expect("a fiscal year has selected subperiods")
                .to_string(),
        })?;
        for (key, ty) in [
            (EFFECT_COMPUTE_ADJ.to_string(), "ComputeAdjustments"),
            (EFFECT_SEAL_ADJ.to_string(), "SealAdjustments"),
            (EFFECT_COMPUTE_STMT.to_string(), "ComputeStatements"),
            (EFFECT_SEAL_YEAR.to_string(), "SealYear"),
        ] {
            journal.append(EventPayload::EffectPlanned {
                effect_key: key,
                effect_type: ty.to_string(),
            })?;
        }
        // One planned effect per D7 correction, sealed on its own approval.
        for c in &input.corrections {
            journal.append(EventPayload::EffectPlanned {
                effect_key: correction_effect_key(&c.correction_id),
                effect_type: "VoucherCorrect".to_string(),
            })?;
        }

        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_COMPUTE_ADJ.to_string(),
        })?;
        journal.append(EventPayload::EffectSucceeded {
            effect_key: EFFECT_COMPUTE_ADJ.to_string(),
            note: format!(
                "{} depreciation line(s), {} accrual(s), {} D7 correction(s)",
                depreciation.len(),
                input.accruals.len(),
                input.corrections.len()
            ),
        })?;
        journal.append(EventPayload::GateOpened {
            gate_id: GATE_APPROVE_ADJ.to_string(),
            gate_kind: "Approval".to_string(),
            snapshot_hash: adjustments_hash(&depreciation, &input.accruals),
        })?;

        let run = fold(&run_id, &journal.read_all()?)?.to_view();
        Ok(CloseYearStart { run, depreciation })
    }

    /// Seal one D7 correction on its own approval (the thin commit boundary). The
    /// correction voucher seals into the next open period; the effect flips to
    /// `Succeeded`. Idempotent — a repeat is a no-op (the ledger natural_key replays).
    pub fn seal_close_year_correction(
        &self,
        run_id: &str,
        correction_id: &str,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        let state = self.read_close_year_state(run_id)?;

        let key = correction_effect_key(correction_id);
        if f.effect(&key).is_none() {
            return Err(ProcessError::InvalidAnswer(format!(
                "run {run_id} has no D7 correction {correction_id:?}"
            )));
        }
        if f.effect_succeeded(&key) {
            return Ok(f.to_view()); // already sealed
        }
        let correction = state
            .input
            .corrections
            .iter()
            .find(|c| c.correction_id == correction_id)
            .ok_or_else(|| {
                ProcessError::JournalCorrupt(format!("correction {correction_id} not in state"))
            })?;
        let doc_date = parse_date(&state.doc_date)?;
        let posting_date = parse_date(&state.posting_date)?;
        let draft = build_correction_draft(correction, doc_date, posting_date)?;

        journal.append(EventPayload::EffectExecuting {
            effect_key: key.clone(),
        })?;
        let ledger = Ledger::open(&self.home)?;
        let natural_key = format!("closeyear:{}:corr:{}", state.year, correction_id);
        match ledger.seal(
            &draft,
            &aggountant_ledger::SealRequest {
                natural_key,
                actor: ACTOR.to_string(),
                policy_set: PolicySet {
                    id: state.policy_set.clone(),
                },
            },
        ) {
            Ok(_) => {
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: key,
                    note: format!(
                        "D7 correction {correction_id} sealed to {}",
                        state.next_open_period
                    ),
                })?;
            }
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: key,
                    error_code: e.error_code().to_string(),
                })?;
                return Err(e.into());
            }
        }
        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Answer a `close-year` gate. `approve` advances to the next stage (sealing
    /// adjustments / computing statements / opening the next gate / `SealYear`);
    /// `reject` declines the close (the Run is Rejected).
    pub fn answer_close_year(
        &self,
        run_id: &str,
        gate_id: &str,
        answer: &str,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
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
                "{gate_id} accepts approve|reject, got {answer:?}"
            )));
        }

        // Preconditions that must hold BEFORE the answer is recorded (so a refusal
        // leaves the gate open for the operator to resolve, never a wedged Run).
        if answer == "approve" {
            match gate_id {
                GATE_APPROVE_LT => self.assert_corrections_sealed(&f)?,
                GATE_APPROVE_STMT => self.assert_statements_binding(run_id, &f, gate_id)?,
                _ => {}
            }
        }

        journal.append(EventPayload::GateAnswered {
            gate_id: gate_id.to_string(),
            answer: answer.to_string(),
        })?;

        if answer == "approve" {
            match gate_id {
                GATE_APPROVE_ADJ => {
                    self.run_seal_adjustments(&journal, run_id)?;
                    self.run_compute_statements(&journal, run_id)?;
                    self.open_gate(&journal, run_id, GATE_APPROVE_TE, "Approval")?;
                }
                GATE_APPROVE_TE => {
                    self.open_gate(&journal, run_id, GATE_APPROVE_LT, "Approval")?;
                }
                GATE_APPROVE_LT => {
                    self.open_gate(&journal, run_id, GATE_APPROVE_STMT, "Preview")?;
                }
                GATE_APPROVE_STMT => {
                    self.run_seal_year(&journal, run_id)?;
                }
                other => return Err(ProcessError::GateNotOpen(format!("unknown gate {other}"))),
            }
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Resume an interrupted `close-year` Run: forward-drive any trailing automation
    /// left incomplete behind an already-answered gate (so a crash mid-stage cannot
    /// wedge the Run), reconcile `SealYear` to fiscal (fiscal wins) **only** when the
    /// sealed year matches this run's own statements, and otherwise return the
    /// parked-at-gate view (lands at the correct gate).
    pub fn resume_close_year(&self, run_id: &str) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        let state = self.read_close_year_state(run_id)?;
        let ledger = Ledger::open(&self.home)?;
        let fy = ledger.fiscal_year_view(state.year)?;

        // Forward-drive the automation that trails each *approved* gate. Every step is
        // idempotent (effect-succeeded / gate-open guards + ledger natural-key replay),
        // so re-running after a crash mid-stage repairs a missing postcondition rather
        // than duplicating work — and un-wedges a Run whose gate was answered but whose
        // downstream effects never completed. Skip entirely once the year is sealed.
        if fy.state == FiscalYearState::Open {
            if f.gate_approved(GATE_APPROVE_ADJ)
                && (!f.effect_succeeded(EFFECT_SEAL_ADJ)
                    || !f.effect_succeeded(EFFECT_COMPUTE_STMT)
                    || f.gate(GATE_APPROVE_TE).is_none())
            {
                self.run_seal_adjustments(&journal, run_id)?;
                self.run_compute_statements(&journal, run_id)?;
                self.open_gate(&journal, run_id, GATE_APPROVE_TE, "Approval")?;
            }
            if f.gate_approved(GATE_APPROVE_TE) && f.gate(GATE_APPROVE_LT).is_none() {
                self.open_gate(&journal, run_id, GATE_APPROVE_LT, "Approval")?;
            }
            let f2 = fold(run_id, &journal.read_all()?)?;
            if f2.gate_approved(GATE_APPROVE_LT) && f2.gate(GATE_APPROVE_STMT).is_none() {
                self.assert_corrections_sealed(&f2)?;
                self.open_gate(&journal, run_id, GATE_APPROVE_STMT, "Preview")?;
            }
        }

        let f3 = fold(run_id, &journal.read_all()?)?;
        let seal_done = f3.effect_succeeded(EFFECT_SEAL_YEAR);
        if !seal_done && fy.state == FiscalYearState::Sealed {
            // The year is sealed but this run did not record it. Adopt fiscal ONLY if
            // the sealed statements are this run's own — otherwise a *different* run
            // sealed the year (single-user sequential: an abandoned run then a fresh
            // one), and this run must not claim it or overwrite the authoritative
            // archive with different figures.
            let own_hash = state.statements.as_ref().map(|s| s.statements_hash());
            if own_hash.is_some() && fy.statements_hash == own_hash {
                journal.append(EventPayload::EffectReconciled {
                    effect_key: EFFECT_SEAL_YEAR.to_string(),
                    note: format!(
                        "fiscal shows year {} Sealed by this run — reconciled",
                        state.year
                    ),
                })?;
                let _ = self.write_archive_stub(run_id, &state);
            } else {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SEAL_YEAR.to_string(),
                    error_code: "SUPERSEDED_BY_ANOTHER_CLOSE".to_string(),
                })?;
            }
        } else if !seal_done
            && f3.gate_approved(GATE_APPROVE_STMT)
            && fy.state == FiscalYearState::Open
        {
            // Approve-then-crash: the statements were approved but SealYear did not
            // run. Complete it now (the Preview binding was already checked at answer).
            self.run_seal_year(&journal, run_id)?;
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    // --- internals --------------------------------------------------------

    fn open_gate(
        &self,
        journal: &Journal,
        run_id: &str,
        gate_id: &str,
        kind: &str,
    ) -> Result<(), ProcessError> {
        // Idempotent (crash-repair): do not re-open an already-open gate.
        let f = fold(run_id, &journal.read_all()?)?;
        if f.gate(gate_id).is_some() {
            return Ok(());
        }
        let state = self.read_close_year_state(run_id)?;
        let snapshot_hash = match gate_id {
            GATE_APPROVE_TE => self.tase_erittelyt_hash(&state)?,
            GATE_APPROVE_LT => self.liitetiedot_hash(&state)?,
            GATE_APPROVE_STMT => state
                .statements
                .as_ref()
                .map(|s| s.statements_hash())
                .ok_or_else(|| {
                    ProcessError::JournalCorrupt(
                        "statements not computed before ApproveStatements".into(),
                    )
                })?,
            _ => String::new(),
        };
        journal.append(EventPayload::GateOpened {
            gate_id: gate_id.to_string(),
            gate_kind: kind.to_string(),
            snapshot_hash,
        })?;
        Ok(())
    }

    fn run_seal_adjustments(&self, journal: &Journal, run_id: &str) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect_succeeded(EFFECT_SEAL_ADJ) {
            return Ok(());
        }
        let state = self.read_close_year_state(run_id)?;
        let doc_date = parse_date(&state.doc_date)?;
        let posting_date = parse_date(&state.posting_date)?;
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_SEAL_ADJ.to_string(),
        })?;
        let ledger = Ledger::open(&self.home)?;
        let depreciation = compute_depreciation(
            &ledger.fiscal_year(state.year)?,
            &state.input.assets,
            depreciation_rate_table(),
        )
        .map_err(map_domain)?;
        let seal =
            |draft: &aggountant_model::VoucherDraft, nk: String| -> Result<(), ProcessError> {
                ledger
                    .seal(
                        draft,
                        &aggountant_ledger::SealRequest {
                            natural_key: nk,
                            actor: ACTOR.to_string(),
                            policy_set: PolicySet {
                                id: state.policy_set.clone(),
                            },
                        },
                    )
                    .map(|_| ())
                    .map_err(ProcessError::from)
            };
        for line in &depreciation {
            let draft = depreciation_voucher(line, doc_date, posting_date).map_err(map_domain)?;
            seal(
                &draft,
                format!("closeyear:{}:dep:{}", state.year, line.asset_id),
            )?;
        }
        for a in &state.input.accruals {
            let draft = accrual_voucher(a, doc_date, posting_date).map_err(map_domain)?;
            seal(
                &draft,
                format!("closeyear:{}:acc:{}", state.year, a.accrual_id),
            )?;
        }
        journal.append(EventPayload::EffectSucceeded {
            effect_key: EFFECT_SEAL_ADJ.to_string(),
            note: format!(
                "{} adjustment voucher(s) sealed to {}",
                depreciation.len() + state.input.accruals.len(),
                state.next_open_period
            ),
        })?;
        Ok(())
    }

    fn run_compute_statements(&self, journal: &Journal, run_id: &str) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect_succeeded(EFFECT_COMPUTE_STMT) {
            return Ok(());
        }
        let mut state = self.read_close_year_state(run_id)?;
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_COMPUTE_STMT.to_string(),
        })?;
        let statements = self.compute_year_statements(&state)?;
        // A statement set whose balance-sheet identity fails is a corrupt/torn read —
        // never let it be approved and sealed. (Holds for any balanced double-entry
        // ledger, so `false` signals real corruption, not ordinary business data.)
        if !statements.balanced {
            journal.append(EventPayload::EffectRejected {
                effect_key: EFFECT_COMPUTE_STMT.to_string(),
                error_code: "STATEMENTS_UNBALANCED".to_string(),
            })?;
            return Err(ProcessError::JournalCorrupt(format!(
                "year {} statements do not balance (assets {} != equity+liabilities {}) — \
                 refusing to proceed",
                state.year, statements.assets_total, statements.equity_liabilities_total
            )));
        }
        state.statements = Some(statements.clone());
        self.write_close_year_state(run_id, &state)?;
        journal.append(EventPayload::EffectSucceeded {
            effect_key: EFFECT_COMPUTE_STMT.to_string(),
            note: format!(
                "statements computed (result {}, statements_hash {})",
                statements.result,
                &statements.statements_hash()[..16]
            ),
        })?;
        Ok(())
    }

    fn run_seal_year(&self, journal: &Journal, run_id: &str) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect_succeeded(EFFECT_SEAL_YEAR) {
            return Ok(());
        }
        let state = self.read_close_year_state(run_id)?;
        let statements = state
            .statements
            .as_ref()
            .ok_or_else(|| ProcessError::JournalCorrupt("no statements to seal".into()))?;
        let statements_hash = statements.statements_hash();
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_SEAL_YEAR.to_string(),
        })?;
        let ledger = Ledger::open(&self.home)?;
        match ledger.seal_year(state.year, &statements_hash, run_id, ACTOR) {
            Ok(_) => {
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SEAL_YEAR.to_string(),
                    note: format!("fiscal year {} sealed", state.year),
                })?;
                let _ = self.write_archive_stub(run_id, &state);
                Ok(())
            }
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SEAL_YEAR.to_string(),
                    error_code: e.error_code().to_string(),
                })?;
                Err(e.into())
            }
        }
    }

    /// All D7 corrections must be sealed before `ApproveStatements` can be reached
    /// (must-seal, ADR-5). Blocks with stable ids while any is unsealed.
    fn assert_corrections_sealed(&self, f: &Fold) -> Result<(), ProcessError> {
        let unsealed: Vec<String> = f
            .effects
            .iter()
            .filter(|e| e.effect_type == "VoucherCorrect" && e.status != EffectStatus::Succeeded)
            .map(|e| e.key.clone())
            .collect();
        if !unsealed.is_empty() {
            return Err(ProcessError::CloseYearBlocked(format!(
                "ApproveStatements is blocked by {} unsealed D7 correction(s): [{}]",
                unsealed.len(),
                unsealed.join(", ")
            )));
        }
        Ok(())
    }

    /// The `ApproveStatements` Preview binding: recompute the statements from the
    /// sealed chain and from the persisted state, and require both hashes to equal
    /// the hash the gate bound — the approval signs the exact statement bytes. A
    /// post-approval input change (or a tampered state file) recomputes differently
    /// and is refused (`s6-spec-ahead.md §3.3`).
    fn assert_statements_binding(
        &self,
        run_id: &str,
        f: &Fold,
        gate_id: &str,
    ) -> Result<(), ProcessError> {
        let gate = f
            .gate(gate_id)
            .ok_or_else(|| ProcessError::GateNotOpen(format!("gate {gate_id} not open")))?;
        let state = self.read_close_year_state(run_id)?;
        let persisted = state
            .statements
            .as_ref()
            .ok_or_else(|| ProcessError::JournalCorrupt("no persisted statements".into()))?;
        let persisted_hash = persisted.statements_hash();
        let fresh = self.compute_year_statements(&state)?;
        let fresh_hash = fresh.statements_hash();
        if persisted_hash != gate.snapshot_hash || fresh_hash != gate.snapshot_hash {
            return Err(ProcessError::JournalCorrupt(format!(
                "ApproveStatements: the statements changed after approval was bound \
                 (bound {}, persisted {persisted_hash}, recomputed {fresh_hash}) — \
                 refusing to seal unapproved figures",
                gate.snapshot_hash
            )));
        }
        Ok(())
    }

    /// Compute the statements deterministically from the sealed year chain + pinned
    /// KPA policy (the authoritative sealed-chain fold — Fork B).
    fn compute_year_statements(&self, state: &CloseYearState) -> Result<Statements, ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        let data = ledger.fiscal_year_data(state.year)?;
        let chart = ledger.chart_list(None)?;
        let provenance = StatementProvenance {
            chart_version: ledger.chart_version()?,
            policy_set: state.policy_set.clone(),
            kpa_policy_version: kpa_policy_version().to_string(),
        };
        compute_statements(
            &data.fiscal_year,
            data.opening_balance.as_ref(),
            &data.vouchers,
            &chart,
            kpa_policy(),
            provenance,
        )
        .map_err(map_domain)
    }

    fn tase_erittelyt_hash(&self, state: &CloseYearState) -> Result<String, ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        let data = ledger.fiscal_year_data(state.year)?;
        let chart = ledger.chart_list(None)?;
        let provenance = StatementProvenance {
            chart_version: ledger.chart_version()?,
            policy_set: state.policy_set.clone(),
            kpa_policy_version: kpa_policy_version().to_string(),
        };
        let te = tase_erittelyt(
            &data.fiscal_year,
            data.opening_balance.as_ref(),
            &data.vouchers,
            &chart,
            provenance,
        )
        .map_err(map_domain)?;
        Ok(te.statements_hash())
    }

    fn liitetiedot_hash(&self, state: &CloseYearState) -> Result<String, ProcessError> {
        Ok(self.build_liitetiedot(state)?.statements_hash())
    }

    /// The determined parts of `liitetiedot`: one depreciation note per asset class
    /// actually depreciated, derived from the pinned policy (Fork F).
    fn build_liitetiedot(&self, state: &CloseYearState) -> Result<Liitetiedot, ProcessError> {
        use std::collections::BTreeMap;
        let ledger = Ledger::open(&self.home)?;
        let depreciation = compute_depreciation(
            &ledger.fiscal_year(state.year)?,
            &state.input.assets,
            depreciation_rate_table(),
        )
        .map_err(map_domain)?;
        let mut notes: BTreeMap<String, aggountant_domain::statements::DepreciationNote> =
            BTreeMap::new();
        for line in &depreciation {
            notes.insert(
                line.asset_class.clone(),
                aggountant_domain::statements::DepreciationNote {
                    asset_class: line.asset_class.clone(),
                    // Stable string form (never the Debug derive — it would drift the
                    // liitetiedot hash on a variant rename / rustc upgrade).
                    method: line.method.as_str().to_string(),
                    rate_percent: line.rate_percent.clone(),
                },
            );
        }
        Ok(Liitetiedot {
            fiscal_year: state.year,
            depreciation_notes: notes.into_values().collect(),
            depreciation_policy_version: depreciation_policy_version().to_string(),
            narrative_pending: true,
            provenance: StatementProvenance {
                chart_version: ledger.chart_version()?,
                policy_set: state.policy_set.clone(),
                kpa_policy_version: kpa_policy_version().to_string(),
            },
        })
    }

    /// Write the sealed statements + tase-erittelyt + liitetiedot to the minimal
    /// **fiscal archive stub** (`<home>/archive/<year>/`, Fork C option b) on
    /// `SealYear`. The full portable `fiscal archive` command (F18/F19) is deferred;
    /// the rendition's hash is already covered by the `SealYear` transition.
    fn write_archive_stub(&self, run_id: &str, state: &CloseYearState) -> Result<(), ProcessError> {
        let statements = match &state.statements {
            Some(s) => s,
            None => return Ok(()),
        };
        let dir = self.home.join("archive").join(state.year.to_string());
        fs::create_dir_all(&dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let te = {
            let ledger = Ledger::open(&self.home)?;
            let data = ledger.fiscal_year_data(state.year)?;
            let chart = ledger.chart_list(None)?;
            tase_erittelyt(
                &data.fiscal_year,
                data.opening_balance.as_ref(),
                &data.vouchers,
                &chart,
                statements.provenance.clone(),
            )
            .map_err(map_domain)?
        };
        let lt = self.build_liitetiedot(state)?;
        let bundle = serde_json::json!({
            "sealed_by_run": run_id,
            "statements": statements,
            "tase_erittelyt": te,
            "liitetiedot": lt,
        });
        let bytes =
            serde_json::to_vec_pretty(&bundle).map_err(|e| ProcessError::Store(e.to_string()))?;
        // Atomic durable write (temp → fsync → rename), matching the journal/state so
        // a crash mid-write never leaves a truncated archive.
        let path = dir.join("tilinpaatos.json");
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
}

/// Build a balanced D7 correction voucher (`voucher correct`): debit/credit the
/// named accounts, into the next open period.
fn build_correction_draft(
    c: &CorrectionInput,
    document_date: Date,
    posting_date: Date,
) -> Result<aggountant_model::VoucherDraft, ProcessError> {
    use aggountant_model::{Currency, Posting, Side};
    if c.debit_account == c.credit_account {
        return Err(ProcessError::InvalidAnswer(format!(
            "D7 correction {} debits and credits the same account {}",
            c.correction_id, c.debit_account
        )));
    }
    // EUR-only in this slice (mirrors the accrual/depreciation constraint); a foreign
    // correction would not fold into the EUR statements.
    let eur = Currency::new("EUR").expect("EUR is a valid currency code");
    if c.amount.currency() != &eur {
        return Err(ProcessError::InvalidAnswer(format!(
            "D7 correction {} is {}, but close-year corrections are EUR-only",
            c.correction_id,
            c.amount.currency()
        )));
    }
    let postings = vec![
        Posting {
            account: c.debit_account.clone(),
            amount: c.amount.clone(),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: c.credit_account.clone(),
            amount: c.amount.clone(),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    aggountant_model::VoucherDraft::new(
        document_date,
        posting_date,
        format!("Oikaisu (D7): {}", c.description),
        postings,
        None,
    )
    .map_err(|e| ProcessError::from(aggountant_ledger::LedgerError::from(e)))
}

/// The `ApproveAdjustments` snapshot hash — binds the review to the exact computed
/// poistot/jaksotukset that are shown (an Approval gate, informational binding).
fn adjustments_hash(depreciation: &[DepreciationLine], accruals: &[AccrualInput]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(serde_json::to_vec(depreciation).unwrap_or_default());
    h.update(serde_json::to_vec(accruals).unwrap_or_default());
    hex::encode(h.finalize())
}

/// Reject empty or duplicate ids in an input list (they become natural/effect keys).
fn assert_unique_ids<'a>(
    what: &str,
    ids: impl Iterator<Item = &'a str>,
) -> Result<(), ProcessError> {
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(ProcessError::InvalidAnswer(format!(
                "{what} must not be empty"
            )));
        }
        if !seen.insert(id) {
            return Err(ProcessError::InvalidAnswer(format!(
                "duplicate {what} {id:?} — ids must be unique within a close-year run"
            )));
        }
    }
    Ok(())
}

fn parse_date(s: &str) -> Result<Date, ProcessError> {
    Date::parse(s).map_err(|e| ProcessError::JournalCorrupt(format!("bad date {s:?}: {e}")))
}

fn map_domain(e: aggountant_domain::DomainError) -> ProcessError {
    match e {
        aggountant_domain::DomainError::Model(m) => {
            ProcessError::Ledger(aggountant_ledger::LedgerError::Model(m))
        }
        other => ProcessError::Store(format!("{}: {other}", other.error_code())),
    }
}
