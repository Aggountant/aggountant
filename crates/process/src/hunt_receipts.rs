//! The **`hunt-receipts` Run** (RF2, scenario A5) — the S5 receipt-hunt slice. It
//! enumerates bank transactions with **missing evidence** and, per transaction, issues a
//! **targeted** pull (counterparty + amount + date window — never the whole inbox) over
//! the injected email / kauppatili adapters. A found document is stored as an
//! **artifact** and lands as a **typed intake match candidate** (`intake.rs`); the raw
//! email/marketplace body is used transiently and **discarded** — only the artifact +
//! provenance survive the Run.
//!
//! The two S5 substrates it exercises:
//! - **`Effect::Wait`:** a `429` on a pull parks that transaction's pull effect on a
//!   *durable* wait keyed off the advertised backoff. `resume_hunt_receipts` re-drives it
//!   once the deadline elapses (per an injected clock) — proven across a process restart,
//!   because the deadline lives in the journal, not in memory.
//! - **Per-item review:** the batch review pauses, but **each match seals on its own
//!   approval** (a thin commit boundary — no approved-but-unsealed queue); an unexplained
//!   transfer raises an actionable **`Clarify`** gate rather than a guess.

use std::fs;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use aggountant_broker::{Broker, Envelope, PullOutcome, SVC_EMAIL_PULL, SVC_KAUPPATILI_PULL};
use aggountant_domain::{build_draft, DraftInput, PolicySet};
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{Date, Money, PeriodId};

use crate::intake::Intake;
use crate::journal::{EventPayload, Journal};
use crate::{fold, Clock, EffectStatus, ProcessEngine, ProcessError, RunView, MAX_WAIT_SECS};

const ACTOR: &str = "process:hunt-receipts";
const DEFAULT_BACKOFF_SECS: u64 = 60;

fn k_pull(txn: &str) -> String {
    format!("pull:{txn}")
}
fn k_seal(txn: &str) -> String {
    format!("seal:{txn}")
}
fn g_match(txn: &str) -> String {
    format!("ApproveMatch:{txn}")
}
fn g_clarify(txn: &str) -> String {
    format!("Clarify:{txn}")
}

/// One bank transaction to hunt evidence for.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BankTransaction {
    pub bank_account: String,
    pub txn_id: String,
    pub counterparty: String,
    pub amount: Money,
    pub date: Date,
    /// `missing` transactions are hunted; anything else is skipped
    /// (`reconciliation transactions list --evidence-status missing`).
    pub evidence_status: String,
    /// The agent's proposed booking for a found match; its `attachment_ref` is filled in
    /// from the hunted artifact before sealing (the "prepare-voucher" step).
    pub proposed_draft: DraftInput,
}

/// Structured input for `hunt-receipts start` (stdin JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HuntReceiptsInput {
    pub transactions: Vec<BankTransaction>,
}

/// Per-transaction plan (persisted, non-raw). After a match, holds the artifact pointer +
/// the prepared draft binding — never the raw pulled body.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TxnPlan {
    /// The transaction **identity** used for all effect/gate keys — `<bank>:<txn>`, not
    /// `txn_id` alone. Two banks may legitimately share a `txn_id`; keying on the pair
    /// keeps their effects, gates, and match drafts from colliding (which would attach
    /// evidence to, or seal, the wrong transaction).
    key: String,
    bank_account: String,
    txn_id: String,
    counterparty: String,
    amount: String,
    date: String,
    proposed_draft: DraftInput,
    artifact_ref: Option<String>,
    draft_body: Option<Value>,
    draft_sent_hash: Option<String>,
    intake_id: Option<String>,
    matched_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HuntState {
    policy_set: String,
    txns: Vec<TxnPlan>,
}

impl ProcessEngine {
    fn hunt_state_path(&self, run_id: &str) -> std::path::PathBuf {
        self.run_dir(run_id).join("hunt_state.json")
    }

    fn write_hunt_state(&self, run_id: &str, s: &HuntState) -> Result<(), ProcessError> {
        let bytes = serde_json::to_vec_pretty(s).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = self.hunt_state_path(run_id);
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

    fn read_hunt_state(&self, run_id: &str) -> Result<HuntState, ProcessError> {
        let bytes = fs::read(self.hunt_state_path(run_id))
            .map_err(|e| ProcessError::Store(format!("hunt state unreadable: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("hunt state corrupt: {e}")))
    }

    /// Start a `hunt-receipts` Run: enumerate the missing-evidence transactions, plan a
    /// pull + seal effect per transaction, and drive each targeted pull (which either
    /// opens an `ApproveMatch`/`Clarify` gate or parks on a durable `Effect::Wait`).
    pub fn start_hunt_receipts(
        &self,
        input: &HuntReceiptsInput,
        broker: &Broker,
        policy_set: &PolicySet,
        clock: &dyn Clock,
    ) -> Result<RunView, ProcessError> {
        let missing: Vec<&BankTransaction> = input
            .transactions
            .iter()
            .filter(|t| t.evidence_status == "missing")
            .collect();
        if missing.is_empty() {
            return Err(ProcessError::InvalidAnswer(
                "no transactions with missing evidence to hunt".into(),
            ));
        }
        // Reject duplicate (bank_account, txn_id) — they collide on effect keys + the
        // intake idempotency key.
        let mut seen = std::collections::HashSet::new();
        for t in &missing {
            if !seen.insert((t.bank_account.as_str(), t.txn_id.as_str())) {
                return Err(ProcessError::InvalidAnswer(format!(
                    "duplicate transaction {}:{}",
                    t.bank_account, t.txn_id
                )));
            }
        }

        let run_id = crate::new_run_id_with("hr");
        let run_dir = self.run_dir(&run_id);
        fs::create_dir_all(&run_dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let _lease = self.acquire_lease(&run_id)?;
        let journal = Journal::new(&run_dir);

        // Use the first transaction's period for the RunStarted marker (the run may span
        // several, but the period field is a coarse locator here).
        let period = PeriodId::containing(missing[0].date);
        journal.append(EventPayload::RunStarted {
            workflow: "hunt-receipts".to_string(),
            workflow_version: crate::hunt_receipts_workflow_version(),
            git_commit: crate::git_commit(),
            policy_set: policy_set.id.clone(),
            period: period.to_string(),
        })?;

        let mut state = HuntState {
            policy_set: policy_set.id.clone(),
            txns: Vec::new(),
        };
        for t in &missing {
            let key = txn_key(&t.bank_account, &t.txn_id);
            for (ekey, ty) in [
                (k_pull(&key), "ExchangeAdapter(email/kauppatili pull)"),
                (k_seal(&key), "Seal(match voucher)"),
            ] {
                journal.append(EventPayload::EffectPlanned {
                    effect_key: ekey,
                    effect_type: ty.to_string(),
                })?;
            }
            state.txns.push(TxnPlan {
                key,
                bank_account: t.bank_account.clone(),
                txn_id: t.txn_id.clone(),
                counterparty: t.counterparty.clone(),
                amount: t.amount.amount_str(),
                date: t.date.to_string(),
                proposed_draft: t.proposed_draft.clone(),
                artifact_ref: None,
                draft_body: None,
                draft_sent_hash: None,
                intake_id: None,
                matched_source: None,
            });
        }
        self.write_hunt_state(&run_id, &state)?;

        let keys: Vec<String> = state.txns.iter().map(|t| t.key.clone()).collect();
        for key in keys {
            self.drive_pull(&journal, &run_id, &key, &mut state, broker, clock)?;
        }

        Ok(fold(&run_id, &journal.read_all()?)?.to_view())
    }

    /// Resume a `hunt-receipts` Run — a convergence engine (mirrors `close-period`). Per
    /// transaction it recovers two crash windows: (a) a pull parked on a durable
    /// `Effect::Wait` whose deadline has now passed (the deadline is journal state, so this
    /// survives a process restart across the wait), and (b) an **approved-but-unsealed
    /// match** — an `ApproveMatch` recorded before `seal_match` finished. Both re-driven
    /// steps are idempotent.
    pub fn resume_hunt_receipts(
        &self,
        run_id: &str,
        broker: &Broker,
        clock: &dyn Clock,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        let mut state = self.read_hunt_state(run_id)?;

        let keys: Vec<String> = state.txns.iter().map(|t| t.key.clone()).collect();
        for key in keys {
            // (a) A quota wait whose deadline passed → re-drive the pull.
            let ff = fold(run_id, &journal.read_all()?)?;
            if let Some(deadline) = ff.wait_deadline(&k_pull(&key)) {
                if clock.now_epoch_secs() >= deadline {
                    self.drive_pull(&journal, run_id, &key, &mut state, broker, clock)?;
                }
            }
            // (b) An approved match whose seal never completed → seal it now.
            let ff = fold(run_id, &journal.read_all()?)?;
            if ff.gate_approved(&g_match(&key)) && !ff.effect_succeeded(&k_seal(&key)) {
                self.seal_match(&journal, run_id, &key, &mut state)?;
            }
        }
        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// One transaction's targeted pull: email first, then kauppatili. A found document is
    /// stored as an artifact and opens the transaction's `ApproveMatch` gate; nothing
    /// found opens a `Clarify` gate (no guess); a rate limit parks a durable `Effect::Wait`.
    fn drive_pull(
        &self,
        journal: &Journal,
        run_id: &str,
        key: &str,
        state: &mut HuntState,
        broker: &Broker,
        clock: &dyn Clock,
    ) -> Result<(), ProcessError> {
        // Idempotent: a transaction already matched or clarified is not re-pulled.
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect(&k_pull(key)).map(|e| e.status) == Some(EffectStatus::Succeeded) {
            return Ok(());
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: k_pull(key),
        })?;

        // A targeted pull carries only the counterparty + amount + date window — never the
        // whole inbox (candidate §5 / A5).
        let (counterparty, amount, date) = {
            let t = self.txn(state, key)?;
            (t.counterparty.clone(), t.amount.clone(), t.date.clone())
        };
        for (service, source) in [
            (SVC_EMAIL_PULL, "email"),
            (SVC_KAUPPATILI_PULL, "kauppatili"),
        ] {
            let env = pull_envelope(service, &counterparty, &amount, &date, key);
            match broker.pull(&env).map_err(map_broker)? {
                PullOutcome::RateLimited { retry_after_secs } => {
                    let backoff = retry_after_secs.unwrap_or(DEFAULT_BACKOFF_SECS);
                    if backoff > MAX_WAIT_SECS {
                        journal.append(EventPayload::EffectRejected {
                            effect_key: k_pull(key),
                            error_code: "QUOTA_BACKOFF_EXCEEDS_MAX".to_string(),
                        })?;
                        return Ok(());
                    }
                    let deadline = clock.now_epoch_secs() + backoff;
                    journal.append(EventPayload::EffectWaiting {
                        effect_key: k_pull(key),
                        resume_after_epoch_secs: deadline,
                        reason: format!("{source} pull rate limited; backoff {backoff}s"),
                    })?;
                    // Record the adapter cursor into the integration namespace and stop.
                    self.write_cursor(service, key)?;
                    return Ok(());
                }
                PullOutcome::Found(body) => {
                    self.record_match(journal, run_id, key, source, &body, state)?;
                    self.write_cursor(service, key)?;
                    return Ok(());
                }
                PullOutcome::NotFound => {
                    // Try the next source; the raw (absent) result is not persisted.
                    self.write_cursor(service, key)?;
                    continue;
                }
            }
        }

        // No evidence from any source: the pull ran but found nothing. Raise an actionable
        // Clarify question rather than guessing a booking. The seal effect will never run,
        // so mark it Skipped (a terminal non-success) — otherwise the Run could never reach
        // Completed once the Clarify is acknowledged.
        journal.append(EventPayload::EffectSucceeded {
            effect_key: k_pull(key),
            note: "no evidence found in any source".to_string(),
        })?;
        journal.append(EventPayload::EffectSkipped {
            effect_key: k_seal(key),
            reason: "no evidence found — clarified, not booked".to_string(),
        })?;
        if fold(run_id, &journal.read_all()?)?
            .gate(&g_clarify(key))
            .is_none()
        {
            journal.append(EventPayload::GateOpened {
                gate_id: g_clarify(key),
                gate_kind: "Clarify".to_string(),
                snapshot_hash: String::new(),
            })?;
        }
        Ok(())
    }

    /// A found document: store its bytes as an artifact, land a typed intake candidate
    /// (no raw body), bind the prepared draft to the artifact, and open `ApproveMatch`.
    fn record_match(
        &self,
        journal: &Journal,
        run_id: &str,
        key: &str,
        source: &str,
        body: &Value,
        state: &mut HuntState,
    ) -> Result<(), ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        // The attachment is ONLY the typed `document`. If the adapter returns a
        // differently-shaped body we must NOT fall back to persisting the whole raw
        // response — that would attach the raw email/marketplace body (with its PII) as a
        // permanent artifact. Reject instead; the raw body stays transient and is dropped.
        let doc = body.get("document").cloned().ok_or_else(|| {
            ProcessError::Store(
                "pull response missing a typed `document` — refusing to persist the raw body"
                    .into(),
            )
        })?;
        let doc_bytes = serde_json::to_vec(&doc).map_err(|e| ProcessError::Store(e.to_string()))?;
        let artifact = ledger.attach_artifact(&doc_bytes)?;
        let artifact_hex = artifact.to_hex();

        // Prepare the voucher: bind the agent's proposed draft to the hunted artifact.
        let (bank_account, txn_id, counterparty, amount, date, mut draft) = {
            let t = self.txn(state, key)?;
            (
                t.bank_account.clone(),
                t.txn_id.clone(),
                t.counterparty.clone(),
                t.amount.clone(),
                t.date.clone(),
                t.proposed_draft.clone(),
            )
        };
        draft.attachment_ref = Some(artifact);
        // Validate the prepared draft (Σ=0 etc.) + attachment presence before binding.
        build_draft(draft.clone()).map_err(map_domain)?;
        let draft_body =
            serde_json::to_value(&draft).map_err(|e| ProcessError::Store(e.to_string()))?;
        let draft_env = draft_envelope(key, &draft_body);
        let draft_sent_hash = draft_env.sent_hash().map_err(map_broker)?;

        // Land the typed intake candidate (idempotent on voucher-match:<acct>:<txn>). Only
        // derived fields + the artifact pointer — the raw body never reaches disk here.
        let candidate = Intake::open(&self.home).upsert(
            run_id,
            &bank_account,
            &txn_id,
            &counterparty,
            &amount,
            &date,
            source,
            &artifact_hex,
        )?;

        {
            let t = self.txn_mut(state, key)?;
            t.artifact_ref = Some(artifact_hex.clone());
            t.draft_body = Some(draft_body);
            t.draft_sent_hash = Some(draft_sent_hash.clone());
            t.intake_id = Some(candidate.id.clone());
            t.matched_source = Some(source.to_string());
        }
        self.write_hunt_state(run_id, state)?;

        journal.append(EventPayload::EffectSucceeded {
            effect_key: k_pull(key),
            note: format!("matched via {source}; artifact {artifact_hex}"),
        })?;
        if fold(run_id, &journal.read_all()?)?
            .gate(&g_match(key))
            .is_none()
        {
            journal.append(EventPayload::GateOpened {
                gate_id: g_match(key),
                gate_kind: "Preview".to_string(),
                snapshot_hash: draft_sent_hash,
            })?;
        }
        Ok(())
    }

    /// Answer one hunt gate. `ApproveMatch:<bank>:<txn>` accepts `approve` (seal the match
    /// now — a thin per-item commit) or `discard` (mark the candidate discarded).
    /// `Clarify:<bank>:<txn>` accepts `acknowledge` (the human took note; no booking guessed).
    pub fn answer_hunt(
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

        let mut state = self.read_hunt_state(run_id)?;

        if let Some(key) = gate_id.strip_prefix("ApproveMatch:") {
            if answer != "approve" && answer != "discard" {
                return Err(ProcessError::InvalidAnswer(format!(
                    "{gate_id} accepts approve|discard, got {answer:?}"
                )));
            }
            let key = key.to_string();
            // Verify the prepared draft still binds to the approved bytes.
            let intake_id = self.txn(&state, &key)?.intake_id.clone();
            if answer == "approve" {
                let t = self.txn(&state, &key)?;
                let body = t
                    .draft_body
                    .clone()
                    .ok_or_else(|| ProcessError::InvalidAnswer("no prepared draft".into()))?;
                let bound = t
                    .draft_sent_hash
                    .clone()
                    .ok_or_else(|| ProcessError::InvalidAnswer("no bound draft hash".into()))?;
                let env = draft_envelope(&key, &body);
                let recomputed = env.sent_hash().map_err(map_broker)?;
                if recomputed != bound || recomputed != gate.snapshot_hash {
                    return Err(ProcessError::JournalCorrupt(format!(
                        "{gate_id}: the match draft changed after approval was bound \
                         (bound {}, now {recomputed})",
                        gate.snapshot_hash
                    )));
                }
            }
            journal.append(EventPayload::GateAnswered {
                gate_id: gate_id.to_string(),
                answer: answer.to_string(),
            })?;
            if answer == "approve" {
                self.seal_match(&journal, run_id, &key, &mut state)?;
            } else {
                // Discard: the match is declined. Mark the intake candidate discarded and
                // skip the seal effect (terminal non-success) so the Run can still complete.
                if let Some(id) = intake_id {
                    Intake::open(&self.home).mark_discarded(&id)?;
                }
                journal.append(EventPayload::EffectSkipped {
                    effect_key: k_seal(&key),
                    reason: "match discarded by reviewer".to_string(),
                })?;
            }
        } else if gate_id.strip_prefix("Clarify:").is_some() {
            if answer != "acknowledge" && answer != "discard" {
                return Err(ProcessError::InvalidAnswer(format!(
                    "{gate_id} accepts acknowledge|discard, got {answer:?}"
                )));
            }
            journal.append(EventPayload::GateAnswered {
                gate_id: gate_id.to_string(),
                answer: answer.to_string(),
            })?;
        } else {
            return Err(ProcessError::GateNotOpen(format!("unknown gate {gate_id}")));
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Seal one matched transaction's voucher (idempotent on `voucher-match:<acct>:<txn>`)
    /// and mark its intake candidate matched.
    fn seal_match(
        &self,
        journal: &Journal,
        run_id: &str,
        key: &str,
        state: &mut HuntState,
    ) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect_succeeded(&k_seal(key)) {
            return Ok(());
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: k_seal(key),
        })?;
        let (bank_account, txn_id, body, intake_id) = {
            let t = self.txn(state, key)?;
            (
                t.bank_account.clone(),
                t.txn_id.clone(),
                t.draft_body
                    .clone()
                    .ok_or_else(|| ProcessError::InvalidAnswer("no prepared draft".into()))?,
                t.intake_id.clone(),
            )
        };
        let draft_input: DraftInput = serde_json::from_value(body)
            .map_err(|e| ProcessError::JournalCorrupt(format!("prepared draft corrupt: {e}")))?;
        let draft = build_draft(draft_input).map_err(map_domain)?;
        let ledger = Ledger::open(&self.home)?;
        let natural_key = Intake::candidate_id(&bank_account, &txn_id);
        match ledger.seal(
            &draft,
            &SealRequest {
                natural_key,
                actor: ACTOR.to_string(),
                policy_set: PolicySet {
                    id: state.policy_set.clone(),
                },
            },
        ) {
            Ok(sealed) => {
                if let Some(id) = intake_id {
                    Intake::open(&self.home).mark_matched(&id)?;
                }
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: k_seal(key),
                    note: format!("match voucher #{} sealed", sealed.number),
                })?;
                Ok(())
            }
            Err(e) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: k_seal(key),
                    error_code: e.error_code().to_string(),
                })?;
                Err(e.into())
            }
        }
    }

    /// Record the adapter cursor into the `integration` namespace (not the run store):
    /// cursors are operational, not fiscal, and outlive a single Run (candidate §6).
    fn write_cursor(&self, service: &str, txn_id: &str) -> Result<(), ProcessError> {
        let dir = self.home.join("integration");
        fs::create_dir_all(&dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = dir.join(format!("{service}.json"));
        let tmp = path.with_extension("json.tmp");
        let body = json!({ "service": service, "last_txn": txn_id });
        let bytes =
            serde_json::to_vec_pretty(&body).map_err(|e| ProcessError::Store(e.to_string()))?;
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

    fn txn<'a>(&self, state: &'a HuntState, key: &str) -> Result<&'a TxnPlan, ProcessError> {
        state
            .txns
            .iter()
            .find(|t| t.key == key)
            .ok_or_else(|| ProcessError::InvalidAnswer(format!("no plan for txn {key}")))
    }
    fn txn_mut<'a>(
        &self,
        state: &'a mut HuntState,
        key: &str,
    ) -> Result<&'a mut TxnPlan, ProcessError> {
        state
            .txns
            .iter_mut()
            .find(|t| t.key == key)
            .ok_or_else(|| ProcessError::InvalidAnswer(format!("no plan for txn {key}")))
    }
}

/// The composite transaction identity for effect/gate keys — `<bank>:<txn>`.
fn txn_key(bank_account: &str, txn_id: &str) -> String {
    format!("{bank_account}:{txn_id}")
}

fn pull_envelope(
    service: &str,
    counterparty: &str,
    amount: &str,
    date: &str,
    key: &str,
) -> Envelope {
    Envelope::new(
        service,
        "pull-evidence",
        format!("hunt:{service}:{key}"),
        json!({
            "counterparty": counterparty,
            "amount": amount,
            "date_window": date,
        }),
    )
}

fn draft_envelope(key: &str, body: &Value) -> Envelope {
    Envelope::new(
        "hunt-receipts",
        "match-voucher-draft",
        format!("match:{key}"),
        body.clone(),
    )
}

fn map_domain(e: aggountant_domain::DomainError) -> ProcessError {
    ProcessError::InvalidAnswer(format!("{}: {e}", e.error_code()))
}

fn map_broker(e: aggountant_broker::BrokerError) -> ProcessError {
    ProcessError::Store(format!("{}: {e}", e.error_code()))
}
