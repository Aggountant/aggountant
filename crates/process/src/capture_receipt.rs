//! The **`capture-receipt` Run** (RF1, scenario A2) — the S5 consult slice. A
//! non-routine receipt (a leasing invoice, partial VAT, passenger-car use) escalates
//! from the interim `capture-receipt` skill into a stateful Run that:
//!
//! 1. builds a **default-deny disclosure projection** (`ConsultQuestion`, C3-typed) and
//!    parks on an **`ApproveConsultDisclosure`** `Preview` gate bound to the projected
//!    `sent_hash` (bind-after-redact, Ba — `redaction-strategy-options.md` §4);
//! 2. on approval, drives the **`consult-llm` `Stream`**, **re-projecting + re-binding
//!    every outbound turn** (each tool-call round trip re-discloses under the same
//!    projection discipline), and folds the result into an [`Advice`];
//! 3. runs the deterministic **eligibility engine** (`assess_purchase`) — the LLM/law
//!    boundary: the user's claim is an input, the deductible is the law's output;
//! 4. parks on **`ApproveVoucher`**, then seals the voucher (through the ordinary S1
//!    path — **`Advice` never reaches the ledger**), opens an **F13 obligation** only
//!    when genuine unresolved follow-up was supplied, and parks on
//!    **`ApproveKnowledgePR`**;
//! 5. on approval, writes the review-gated **`knowledge`** candidate (no auto-cache).
//!
//! A `consult-llm` rate limit parks the consult effect on a durable **`Effect::Wait`**
//! (the same mechanism RF2 uses); `resume_capture_receipt` re-drives it once the backoff
//! elapses.

use std::fs;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use aggountant_broker::{Broker, ConsultChunk, ConsultTurn, Envelope, SVC_CONSULT_LLM};
use aggountant_domain::{
    assess_purchase, build_draft, disclosure_source_hash, vat_expand_with_role_table, Advice,
    ConsultQuestion, DraftInput, ExpenseNature, PolicySet, PurchaseAssessment,
    PurchaseAssessmentInput, ReverseChargeScope, VatExpandInput, VatTreatment,
};
use aggountant_ledger::{Ledger, SealRequest, SealedVoucher};
use aggountant_model::{AccountId, Date, Hash, Money, PeriodId, VoucherDraft};

use crate::journal::{EventPayload, Journal};
use crate::{fold, Clock, ProcessEngine, ProcessError, RunView, MAX_WAIT_SECS};

const EFFECT_CONSULT: &str = "consult";
const EFFECT_ASSESS: &str = "assess";
const EFFECT_SEAL: &str = "seal-voucher";
const EFFECT_OBLIGATION: &str = "obligation";
const EFFECT_KNOWLEDGE: &str = "knowledge";
const GATE_APPROVE_DISCLOSURE: &str = "ApproveConsultDisclosure";
const GATE_APPROVE_VOUCHER: &str = "ApproveVoucher";
const GATE_APPROVE_KNOWLEDGE: &str = "ApproveKnowledgePR";
const ACTOR: &str = "process:capture-receipt";
/// A hard ceiling on consult turns so a mis-scripted stream cannot loop forever.
const MAX_TURNS: u32 = 8;
/// Default backoff when the authority advertised no `Retry-After`.
const DEFAULT_BACKOFF_SECS: u64 = 60;

/// One allowlisted disclosure fact (CLI input shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactInput {
    pub key: String,
    pub value: String,
}

/// A scripted **local** tool result (the Broker executes tool-calls locally, `candidate
/// §6`). Consumed in order as the consult stream requests tool-calls; the result is
/// re-projected (default-deny) and re-bound on the next outbound turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolResult {
    pub tool: String,
    pub result: Value,
}

/// The F13 obligation to open when the voucher seals (survives Run close).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObligationInput {
    pub kind: String,
    pub description: String,
    /// Strongly validated deadline, serialized as `YYYY-MM-DD`. Historical state used
    /// the same string representation and remains readable.
    pub deadline: Date,
}

/// Structured input for `capture-receipt start` (stdin JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureReceiptInput {
    pub period: PeriodId,
    /// Consult topic (a coarse, non-sensitive tag).
    pub topic: String,
    pub question: String,
    /// The allowlisted facts to disclose — the *only* thing that crosses the boundary.
    pub facts: Vec<FactInput>,
    /// The raw receipt/source text. **Never disclosed**; only its lineage hash is
    /// stamped into provenance (Bb). Kept out of the persisted disclosure record.
    pub raw_source: String,
    /// The agent's interpretation of the expense nature (an input to the law).
    pub nature: ExpenseNature,
    pub receipt_vat: Money,
    /// The user's claimed business-use share (a claim, not the answer).
    pub claimed_business_share: String,
    pub document_date: Date,
    /// Genuine unresolved follow-up, when one exists. Missing and JSON `null` both mean
    /// that capture has no F13 obligation to create; malformed objects remain errors.
    #[serde(default)]
    pub obligation: Option<ObligationInput>,
    /// Slug for the knowledge candidate distilled from the consult.
    pub knowledge_slug: String,
    /// The agent-proposed voucher draft, sealed on `ApproveVoucher`.
    pub draft: DraftInput,
    /// Scripted local tool results, consumed in order for each `tool_call` turn.
    #[serde(default)]
    pub tool_results: Vec<ToolResult>,
}

/// Source-observed receipt facts. Optional raw fields let the capture boundary return
/// `CAPTURE_FACT_REQUIRED` for omissions. Unknown fields remain forbidden.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptSourceFacts {
    #[serde(default)]
    pub amount: Option<Money>,
    #[serde(default)]
    pub document_date: Option<Date>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub attachment_ref: Option<Hash>,
}

/// Explicit VAT proposal. Uncertainty is data and can never default to `NoVat`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReceiptVatProposal {
    NoVat,
    DomesticInput {
        rate_class: String,
    },
    ReverseCharge {
        rate_class: String,
        scope: ReverseChargeScope,
    },
    ClarificationRequired {
        question: String,
    },
    Unsupported {
        treatment: String,
        reason: String,
    },
}

/// Agent-proposed accounting treatment, kept separate from source facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptTreatmentProposal {
    pub posting_date: Date,
    pub net_account: AccountId,
    pub contra_account: AccountId,
    pub vat: ReceiptVatProposal,
}

/// Strict stdin shape for the ordinary, one-approval receipt path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredReceiptInput {
    #[serde(default)]
    pub natural_key: Option<String>,
    #[serde(default = "default_capture_actor")]
    pub actor: String,
    pub source: ReceiptSourceFacts,
    pub proposal: ReceiptTreatmentProposal,
}

fn default_capture_actor() -> String {
    "cli".to_string()
}

/// Exact projection shown by dry-run and bound into the one subsequent approval.
#[derive(Debug, Clone, Serialize)]
pub struct PreparedReceipt {
    pub projection_valid: bool,
    pub ledger_preflight_valid: bool,
    pub chart_version: String,
    pub policy_set: String,
    pub source_fingerprint: String,
    pub preparation_hash: String,
    pub natural_key: String,
    pub actor: String,
    pub draft: VoucherDraft,
}

/// Prepare through the one chart-owned Domain expansion path. This creates no Run,
/// approval record, fiscal write, or voucher-number prediction.
pub fn prepare_structured_receipt(
    ledger: &Ledger,
    input: &StructuredReceiptInput,
) -> Result<PreparedReceipt, ProcessError> {
    let natural_key = required_trimmed(input.natural_key.as_deref(), "natural_key")?;
    let amount = input
        .source
        .amount
        .clone()
        .ok_or_else(|| ProcessError::CaptureFactRequired("source.amount is required".into()))?;
    let document_date = input.source.document_date.ok_or_else(|| {
        ProcessError::CaptureFactRequired(
            "source.document_date is required; use the receipt/invoice date, not the bank date"
                .into(),
        )
    })?;
    let description = required_trimmed(input.source.description.as_deref(), "source.description")?;
    let actor = required_trimmed(Some(&input.actor), "actor")?;

    // Validate source facts first so the first error is deterministic and actionable.
    let treatment = match &input.proposal.vat {
        ReceiptVatProposal::NoVat => VatTreatment::NoVat,
        ReceiptVatProposal::DomesticInput { rate_class } => VatTreatment::DomesticInput {
            rate_class: required_trimmed(Some(rate_class), "proposal.vat.rate_class")?,
        },
        ReceiptVatProposal::ReverseCharge { rate_class, scope } => VatTreatment::ReverseCharge {
            rate_class: required_trimmed(Some(rate_class), "proposal.vat.rate_class")?,
            scope: *scope,
        },
        ReceiptVatProposal::ClarificationRequired { question } => {
            return Err(ProcessError::CaptureClarificationRequired(
                required_trimmed(Some(question), "proposal.vat.question")?,
            ));
        }
        ReceiptVatProposal::Unsupported { treatment, reason } => {
            let treatment = required_trimmed(Some(treatment), "proposal.vat.treatment")?;
            let reason = required_trimmed(Some(reason), "proposal.vat.reason")?;
            return Err(ProcessError::CaptureUnsupportedTreatment(format!(
                "{treatment}: {reason}"
            )));
        }
    };

    let policy_set = PolicySet::default();
    let chart_version = ledger.chart_version()?;
    let draft = vat_expand_with_role_table(
        &VatExpandInput {
            amount: amount.clone(),
            net_account: input.proposal.net_account.clone(),
            contra_account: input.proposal.contra_account.clone(),
            treatment,
            document_date,
            posting_date: input.proposal.posting_date,
            description: description.clone(),
            attachment_ref: input.source.attachment_ref,
        },
        &chart_version,
        ledger.vat_role_table(),
    )
    .map_err(map_domain)?;
    let validation = ledger.validate(&draft)?;
    if validation.chart_version != chart_version {
        return Err(ProcessError::CapturePreparationChanged(format!(
            "active chart changed during preparation (started {chart_version}, validated {})",
            validation.chart_version
        )));
    }

    let source_fingerprint = receipt_source_fingerprint(
        &amount,
        document_date,
        &description,
        input.source.attachment_ref,
    );
    let draft_request_hash = ledger.draft_request_hash(&draft)?;
    let preparation_hash = receipt_preparation_hash(
        &natural_key,
        &actor,
        &source_fingerprint,
        &chart_version,
        &policy_set.id,
        &draft_request_hash,
    );
    Ok(PreparedReceipt {
        projection_valid: true,
        ledger_preflight_valid: true,
        chart_version,
        policy_set: policy_set.id,
        source_fingerprint,
        preparation_hash,
        natural_key,
        actor,
        draft,
    })
}

/// Seal one previewed ordinary receipt. Re-preparation makes input, chart, policy,
/// and binary drift a loud refusal; Ledger checks the expected chart at its boundary.
pub fn record_structured_receipt(
    ledger: &Ledger,
    input: &StructuredReceiptInput,
    approved_preparation_hash: &str,
) -> Result<SealedVoucher, ProcessError> {
    let approved = required_trimmed(Some(approved_preparation_hash), "approved_preparation_hash")?;
    let prepared = prepare_structured_receipt(ledger, input)?;
    if approved != prepared.preparation_hash {
        return Err(ProcessError::CapturePreparationChanged(format!(
            "approved preparation {approved:?} does not match current {}",
            prepared.preparation_hash
        )));
    }
    let policy_set = PolicySet {
        id: prepared.policy_set.clone(),
    };
    Ok(ledger.seal_with_source_fingerprint_at_chart(
        &prepared.draft,
        &SealRequest {
            natural_key: prepared.natural_key,
            actor: prepared.actor,
            policy_set,
        },
        &prepared.source_fingerprint,
        &prepared.chart_version,
    )?)
}

fn required_trimmed(value: Option<&str>, field: &str) -> Result<String, ProcessError> {
    match value {
        Some(value) if !value.is_empty() && value.trim() == value => Ok(value.to_string()),
        _ => Err(ProcessError::CaptureFactRequired(format!(
            "{field} must be present, non-empty, and trimmed"
        ))),
    }
}

fn fingerprint_field(hasher: &mut Sha256, name: &str, value: Option<&str>) {
    hasher.update((name.len() as u64).to_be_bytes());
    hasher.update(name.as_bytes());
    match value {
        Some(value) => {
            hasher.update([1]);
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        None => hasher.update([0]),
    }
}

fn receipt_source_fingerprint(
    amount: &Money,
    document_date: Date,
    description: &str,
    attachment_ref: Option<Hash>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"aggountant:capture-receipt-source:v1");
    let amount_text = amount.amount_str();
    let date_text = document_date.to_string();
    let attachment_text = attachment_ref.map(|hash| hash.to_hex());
    fingerprint_field(&mut hasher, "amount", Some(&amount_text));
    fingerprint_field(&mut hasher, "currency", Some(amount.currency().as_str()));
    fingerprint_field(&mut hasher, "document_date", Some(&date_text));
    fingerprint_field(&mut hasher, "description", Some(description));
    fingerprint_field(&mut hasher, "attachment_ref", attachment_text.as_deref());
    format!("sha256:v1:{}", hex::encode(hasher.finalize()))
}

fn receipt_preparation_hash(
    natural_key: &str,
    actor: &str,
    source_fingerprint: &str,
    chart_version: &str,
    policy_set: &str,
    draft_request_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"aggountant:capture-receipt-preparation:v1");
    fingerprint_field(&mut hasher, "natural_key", Some(natural_key));
    fingerprint_field(&mut hasher, "actor", Some(actor));
    fingerprint_field(&mut hasher, "source_fingerprint", Some(source_fingerprint));
    fingerprint_field(&mut hasher, "chart_version", Some(chart_version));
    fingerprint_field(&mut hasher, "policy_set", Some(policy_set));
    fingerprint_field(&mut hasher, "draft_request_hash", Some(draft_request_hash));
    format!("sha256:v1:{}", hex::encode(hasher.finalize()))
}

/// Run-scoped state (persisted so answer/resume rebuild the approved bytes
/// deterministically). Holds the projected disclosure + its binding + the lineage
/// stamp; the **raw source is not retained** (only its hash). `Advice` and the
/// assessment are filled after the consult.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaptureState {
    period: String,
    policy_set: String,
    topic: String,
    question: String,
    facts: Vec<FactInput>,
    disclosure_body: Value,
    disclosure_sent_hash: String,
    /// Bb: `SHA-256` of the raw pre-projection source (local lineage; the secret itself
    /// is not stored).
    disclosure_source_hash: String,
    nature: ExpenseNature,
    receipt_vat: Money,
    claimed_business_share: String,
    document_date: String,
    /// Optional for new state. Serde also reads the historical object-shaped value as
    /// `Some`, so existing Runs resume without migration or loss.
    #[serde(default)]
    obligation: Option<ObligationInput>,
    knowledge_slug: String,
    draft: DraftInput,
    draft_sent_hash: String,
    tool_results: Vec<ToolResult>,
    // Filled after the consult completes:
    advice: Option<Advice>,
    assessment: Option<PurchaseAssessment>,
}

impl ProcessEngine {
    fn capture_state_path(&self, run_id: &str) -> std::path::PathBuf {
        self.run_dir(run_id).join("capture_state.json")
    }

    fn write_capture_state(&self, run_id: &str, s: &CaptureState) -> Result<(), ProcessError> {
        let bytes = serde_json::to_vec_pretty(s).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = self.capture_state_path(run_id);
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

    fn read_capture_state(&self, run_id: &str) -> Result<CaptureState, ProcessError> {
        let bytes = fs::read(self.capture_state_path(run_id))
            .map_err(|e| ProcessError::Store(format!("capture state unreadable: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("capture state corrupt: {e}")))
    }

    /// Start a `capture-receipt` Run: build the default-deny disclosure projection, stamp
    /// the source-lineage hash, persist state, and park on the `ApproveConsultDisclosure`
    /// `Preview` gate bound to the projected `sent_hash`.
    pub fn start_capture_receipt(
        &self,
        input: &CaptureReceiptInput,
        policy_set: &PolicySet,
    ) -> Result<RunView, ProcessError> {
        // Validate the draft up front so a malformed proposal fails at start, not at seal.
        build_draft(input.draft.clone()).map_err(map_domain)?;

        let run_id = crate::new_run_id_with("cr");
        let run_dir = self.run_dir(&run_id);
        fs::create_dir_all(&run_dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let _lease = self.acquire_lease(&run_id)?;
        let journal = Journal::new(&run_dir);

        journal.append(EventPayload::RunStarted {
            workflow: "capture-receipt".to_string(),
            workflow_version: crate::capture_receipt_workflow_version(),
            git_commit: crate::git_commit(),
            policy_set: policy_set.id.clone(),
            period: input.period.to_string(),
        })?;
        for (key, ty) in [
            (EFFECT_CONSULT, "Consult(consult-llm)"),
            (EFFECT_ASSESS, "AssessEligibility"),
            (EFFECT_SEAL, "Seal(voucher)"),
            (EFFECT_OBLIGATION, "OpenObligation"),
            (EFFECT_KNOWLEDGE, "WriteKnowledge"),
        ] {
            journal.append(EventPayload::EffectPlanned {
                effect_key: key.to_string(),
                effect_type: ty.to_string(),
            })?;
        }

        // Build the projection (Domain/caller concern; Broker stays schema-agnostic).
        let disclosure = build_projection(&input.topic, &input.question, &input.facts);
        let disclosure_env = disclosure_envelope(&run_id, &disclosure.to_body());
        let disclosure_sent_hash = disclosure_env.sent_hash().map_err(map_broker)?;
        let source_hash = disclosure_source_hash(input.raw_source.as_bytes());

        // Bind the ApproveVoucher gate ahead of time by computing the draft's hash.
        let draft_env = draft_envelope(&run_id, &draft_body(&input.draft)?);
        let draft_sent_hash = draft_env.sent_hash().map_err(map_broker)?;

        self.write_capture_state(
            &run_id,
            &CaptureState {
                period: input.period.to_string(),
                policy_set: policy_set.id.clone(),
                topic: input.topic.clone(),
                question: input.question.clone(),
                facts: input.facts.clone(),
                disclosure_body: disclosure.to_body(),
                disclosure_sent_hash: disclosure_sent_hash.clone(),
                disclosure_source_hash: source_hash.clone(),
                nature: input.nature,
                receipt_vat: input.receipt_vat.clone(),
                claimed_business_share: input.claimed_business_share.clone(),
                document_date: input.document_date.to_string(),
                obligation: input.obligation.clone(),
                knowledge_slug: input.knowledge_slug.clone(),
                draft: input.draft.clone(),
                draft_sent_hash,
                tool_results: input.tool_results.clone(),
                advice: None,
                assessment: None,
            },
        )?;

        // The disclosure gate binds to the *projected/redacted* bytes (Ba). The lineage
        // stamp goes into the audit note — the raw source itself is never recorded.
        journal.append(EventPayload::GateOpened {
            gate_id: GATE_APPROVE_DISCLOSURE.to_string(),
            gate_kind: "Preview".to_string(),
            snapshot_hash: disclosure_sent_hash,
        })?;

        Ok(fold(&run_id, &journal.read_all()?)?.to_view())
    }

    /// Answer one `capture-receipt` gate. Dispatches on `gate_id`:
    /// `ApproveConsultDisclosure` drives the consult stream; `ApproveVoucher` seals +
    /// opens the obligation; `ApproveKnowledgePR` writes the knowledge candidate. A
    /// `reject` at any gate declines the Run.
    pub fn answer_capture_receipt(
        &self,
        run_id: &str,
        gate_id: &str,
        answer: &str,
        broker: &Broker,
        clock: &dyn Clock,
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

        let mut state = self.read_capture_state(run_id)?;

        // Preview gates re-verify the persisted bytes against the bound hash before any
        // side effect — the approval signs the exact bytes (the S3 discipline).
        if answer == "approve" {
            match gate_id {
                GATE_APPROVE_DISCLOSURE => {
                    let env = disclosure_envelope(run_id, &state.disclosure_body);
                    assert_bound(&env, &state.disclosure_sent_hash, &gate.snapshot_hash)?;
                }
                GATE_APPROVE_VOUCHER => {
                    let env = draft_envelope(run_id, &draft_body(&state.draft)?);
                    assert_bound(&env, &state.draft_sent_hash, &gate.snapshot_hash)?;
                }
                GATE_APPROVE_KNOWLEDGE => {
                    // Bind the approval to the exact candidate bytes the human reviewed:
                    // recompute the candidate hash and refuse if the persisted advice
                    // changed after the gate opened (the same S3 tamper guard).
                    let recomputed = knowledge_candidate_hash(&state);
                    if recomputed != gate.snapshot_hash {
                        return Err(ProcessError::JournalCorrupt(format!(
                            "ApproveKnowledgePR: the knowledge candidate changed after \
                             approval was bound (bound {}, now {recomputed})",
                            gate.snapshot_hash
                        )));
                    }
                }
                other => return Err(ProcessError::GateNotOpen(format!("unknown gate {other}"))),
            }
        }

        journal.append(EventPayload::GateAnswered {
            gate_id: gate_id.to_string(),
            answer: answer.to_string(),
        })?;

        if answer == "approve" {
            match gate_id {
                GATE_APPROVE_DISCLOSURE => {
                    self.drive_consult(&journal, run_id, &mut state, broker, clock)?;
                }
                GATE_APPROVE_VOUCHER => {
                    self.seal_and_obligate(&journal, run_id, &mut state)?;
                }
                GATE_APPROVE_KNOWLEDGE => {
                    self.write_knowledge(&journal, run_id, &state)?;
                }
                _ => {}
            }
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Resume a `capture-receipt` Run — a **convergence engine**, not just a wait handler
    /// (mirrors `close-period` resume). It recovers two crash windows: (a) a consult
    /// parked on a durable `Effect::Wait` whose backoff has now elapsed, and (b) an
    /// **answered-but-unfinished gate** — an approve recorded in the journal before its
    /// side effect completed. Each re-driven step is idempotent, so resume is safe to run
    /// repeatedly.
    pub fn resume_capture_receipt(
        &self,
        run_id: &str,
        broker: &Broker,
        clock: &dyn Clock,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        let mut state = self.read_capture_state(run_id)?;

        // (a) A quota wait whose deadline passed → re-drive the consult.
        if let Some(deadline) = f.wait_deadline(EFFECT_CONSULT) {
            if clock.now_epoch_secs() < deadline {
                return Ok(f.to_view()); // still within the backoff
            }
            self.drive_consult(&journal, run_id, &mut state, broker, clock)?;
        }

        // (b) Answered-but-unfinished gates. `drive_consult`/`seal_and_obligate`/
        // `write_knowledge` are each idempotent (guarded on their effect status), so this
        // completes exactly the step the crash interrupted. Re-fold before each check so a
        // step's own appends are visible to the next.
        let f = fold(run_id, &journal.read_all()?)?;
        if f.gate_approved(GATE_APPROVE_DISCLOSURE)
            && !f.effect_succeeded(EFFECT_CONSULT)
            && f.effect(EFFECT_CONSULT).map(|e| e.status) != Some(crate::EffectStatus::Waiting)
        {
            self.drive_consult(&journal, run_id, &mut state, broker, clock)?;
        }
        let f = fold(run_id, &journal.read_all()?)?;
        if f.gate_approved(GATE_APPROVE_VOUCHER)
            && (!f.effect_succeeded(EFFECT_SEAL)
                || !f.effect_succeeded(EFFECT_OBLIGATION)
                || f.gate(GATE_APPROVE_KNOWLEDGE).is_none())
        {
            // Sealing, the obligation decision, and opening the next gate are separate
            // durable transitions. Re-drive when any is unfinished so crashes between
            // them converge without fabricating, losing, or duplicating side effects.
            self.seal_and_obligate(&journal, run_id, &mut state)?;
        }
        let f = fold(run_id, &journal.read_all()?)?;
        if f.gate_approved(GATE_APPROVE_KNOWLEDGE) && !f.effect_succeeded(EFFECT_KNOWLEDGE) {
            self.write_knowledge(&journal, run_id, &state)?;
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Drive the consult stream from turn 1: re-project + re-bind **every** outbound turn,
    /// execute tool-calls locally, and fold the `Final` body into an `Advice`. A rate
    /// limit parks the consult on a durable `Effect::Wait`; a completed consult runs the
    /// deterministic eligibility assessment and opens the `ApproveVoucher` gate.
    fn drive_consult(
        &self,
        journal: &Journal,
        run_id: &str,
        state: &mut CaptureState,
        broker: &Broker,
        clock: &dyn Clock,
    ) -> Result<(), ProcessError> {
        // Idempotent: if the consult already produced advice, skip to the assessment.
        if state.advice.is_none() {
            journal.append(EventPayload::EffectExecuting {
                effect_key: EFFECT_CONSULT.to_string(),
            })?;

            let mut turn: u32 = 1;
            // Turn 1 is the approved projection; later turns are re-projected tool results.
            let mut env = disclosure_envelope(run_id, &state.disclosure_body);
            let mut tool_idx = 0usize;
            let advice = loop {
                if turn > MAX_TURNS {
                    journal.append(EventPayload::EffectRejected {
                        effect_key: EFFECT_CONSULT.to_string(),
                        error_code: "CONSULT_TOO_MANY_TURNS".to_string(),
                    })?;
                    return Ok(());
                }
                // Re-bind: record the exact projected bytes leaving on this turn.
                let hash = env.sent_hash().map_err(map_broker)?;
                journal.append(EventPayload::DisclosureBound {
                    turn,
                    sent_hash: hash,
                })?;

                match broker.consult_turn(&env).map_err(map_broker)? {
                    ConsultTurn::RateLimited { retry_after_secs } => {
                        // A durable wait is only safe on **turn 1** — before any tool-call
                        // round trip. There, the 429 means the request was rejected (not
                        // processed), so `resume` re-sending the same approved projection is
                        // idempotent. Once the stream has progressed past turn 1 we cannot
                        // safely replay it (a stateful provider would double-charge / start
                        // a new conversation, and tool-result state is not persisted), so a
                        // mid-stream rate limit parks the effect `NeedsAttention` for the
                        // driver rather than silently re-running from scratch.
                        if turn > 1 {
                            journal.append(EventPayload::EffectRejected {
                                effect_key: EFFECT_CONSULT.to_string(),
                                error_code: "QUOTA_MID_STREAM".to_string(),
                            })?;
                            return Ok(());
                        }
                        let backoff = retry_after_secs.unwrap_or(DEFAULT_BACKOFF_SECS);
                        if backoff > MAX_WAIT_SECS {
                            // Fork C: beyond the ceiling we ask a human, not the clock.
                            journal.append(EventPayload::EffectRejected {
                                effect_key: EFFECT_CONSULT.to_string(),
                                error_code: "QUOTA_BACKOFF_EXCEEDS_MAX".to_string(),
                            })?;
                            return Ok(());
                        }
                        let deadline = clock.now_epoch_secs() + backoff;
                        journal.append(EventPayload::EffectWaiting {
                            effect_key: EFFECT_CONSULT.to_string(),
                            resume_after_epoch_secs: deadline,
                            reason: format!("consult-llm rate limited; backoff {backoff}s"),
                        })?;
                        return Ok(()); // parked Waiting; resume safely re-sends turn 1
                    }
                    ConsultTurn::Chunk(ConsultChunk::Final { body }) => {
                        let advice: Advice = serde_json::from_value(body).map_err(|e| {
                            ProcessError::Store(format!("consult Advice malformed: {e}"))
                        })?;
                        break advice;
                    }
                    ConsultTurn::Chunk(ConsultChunk::Partial { .. }) => {
                        // Advisory; ask for the next chunk with the same (approved) turn 1
                        // projection or the current tool-result projection unchanged.
                        turn += 1;
                        continue;
                    }
                    ConsultTurn::Chunk(ConsultChunk::ToolCall { tool, .. }) => {
                        // Execute locally: consume the next scripted result and re-project
                        // it (default-deny) for the next outbound turn — re-disclosure.
                        let result = state.tool_results.get(tool_idx).cloned();
                        tool_idx += 1;
                        let projected = tool_result_projection(
                            &state.topic,
                            &tool,
                            result.as_ref().map(|tr| &tr.result),
                        );
                        env = disclosure_envelope(run_id, &projected.to_body());
                        turn += 1;
                        continue;
                    }
                }
            };

            state.advice = Some(advice);
            self.write_capture_state(run_id, state)?;
            journal.append(EventPayload::EffectSucceeded {
                effect_key: EFFECT_CONSULT.to_string(),
                note: "consult complete (advice folded; Advice stays off the ledger)".to_string(),
            })?;
        }

        // AssessEligibility — the deterministic law. The consult *informs*; the Domain
        // *decides*. The user's claim is an input; `deductible_vat` is the output.
        if state.assessment.is_none() {
            journal.append(EventPayload::EffectExecuting {
                effect_key: EFFECT_ASSESS.to_string(),
            })?;
            let assessment = assess_purchase(&PurchaseAssessmentInput {
                nature: state.nature,
                receipt_vat: state.receipt_vat.clone(),
                claimed_business_share: state.claimed_business_share.clone(),
                document_date: Date::parse(&state.document_date)
                    .map_err(|e| ProcessError::JournalCorrupt(format!("bad date: {e}")))?,
            })
            .map_err(map_domain)?;
            let note = format!(
                "deductible_vat {} (claim {}, honored={})",
                assessment.deductible_vat.amount_str(),
                assessment.claimed_business_share,
                assessment.claim_honored
            );
            state.assessment = Some(assessment);
            self.write_capture_state(run_id, state)?;
            journal.append(EventPayload::EffectSucceeded {
                effect_key: EFFECT_ASSESS.to_string(),
                note,
            })?;
        }

        // Park on ApproveVoucher (bound to the draft's sent_hash).
        let f = fold(run_id, &journal.read_all()?)?;
        if f.gate(GATE_APPROVE_VOUCHER).is_none() {
            journal.append(EventPayload::GateOpened {
                gate_id: GATE_APPROVE_VOUCHER.to_string(),
                gate_kind: "Preview".to_string(),
                snapshot_hash: state.draft_sent_hash.clone(),
            })?;
        }
        Ok(())
    }

    /// On `ApproveVoucher`: seal the voucher (ordinary S1 path — Advice never touches
    /// it), durably record either the supplied F13 obligation or the explicit absence of
    /// one, and park on the knowledge gate.
    fn seal_and_obligate(
        &self,
        journal: &Journal,
        run_id: &str,
        state: &mut CaptureState,
    ) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if !f.effect_succeeded(EFFECT_SEAL) {
            let draft = build_draft(state.draft.clone()).map_err(map_domain)?;
            let ledger = Ledger::open(&self.home)?;

            // The approval is bound to the caller's exact draft, so never repair it in
            // place. Instead project the draft through the same tagged/account-role VAT
            // paths used for filing, under the exact chart version Ledger validation has
            // selected, and require equality with the deterministic law result.
            let validation = ledger.validate(&draft)?;
            let actual = ledger
                .draft_deductible_vat(&draft, &validation.chart_version)
                .map_err(|e| {
                    ProcessError::InvalidAnswer(format!(
                    "APPROVED DRAFT VAT projection failed before sealing. The nested diagnostic \
                     applies only to this draft; any voucher number it contains is a temporary \
                     projection identifier, NOT an existing ledger voucher. {}: {e}",
                    e.error_code()
                ))
                })?;
            let assessed = state.assessment.as_ref().ok_or_else(|| {
                ProcessError::JournalCorrupt(
                    "ApproveVoucher reached sealing without an eligibility assessment".into(),
                )
            })?;
            if actual != assessed.deductible_vat {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SEAL.to_string(),
                    error_code: "CAPTURE_VAT_MISMATCH".to_string(),
                })?;
                return Err(ProcessError::CaptureVatMismatch(format!(
                    "approved draft records deductible VAT {}, but eligibility assessed {} \
                     under chart_version {}. No voucher was sealed and the approved payload \
                     was not changed; start a new capture-receipt Run with a corrected draft",
                    actual.amount_str(),
                    assessed.deductible_vat.amount_str(),
                    validation.chart_version
                )));
            }

            journal.append(EventPayload::EffectExecuting {
                effect_key: EFFECT_SEAL.to_string(),
            })?;
            let natural_key = format!("capture:{}:{}", state.period, state.draft_sent_hash);
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
                Ok(sealed) => journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SEAL.to_string(),
                    note: format!("voucher #{} sealed", sealed.number),
                })?,
                Err(e) => {
                    journal.append(EventPayload::EffectRejected {
                        effect_key: EFFECT_SEAL.to_string(),
                        error_code: e.error_code().to_string(),
                    })?;
                    return Err(e.into());
                }
            };
        }

        // Resolve the planned F13 effect explicitly even when no follow-up exists. This
        // makes the journal distinguish "none supplied" from an interrupted create, so
        // resume can converge idempotently without fabricating or silently losing work.
        let f = fold(run_id, &journal.read_all()?)?;
        if !f.effect_succeeded(EFFECT_OBLIGATION) {
            journal.append(EventPayload::EffectExecuting {
                effect_key: EFFECT_OBLIGATION.to_string(),
            })?;
            let note = if let Some(input) = &state.obligation {
                let period = PeriodId::parse(&state.period)
                    .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;
                // Deterministic id keyed on the run: a resume after a crash between
                // `ensure` and `EffectSucceeded` re-uses the same id rather than minting
                // a duplicate obligation. Historical object-shaped state deserializes to
                // this `Some` branch and therefore cannot be silently dropped.
                let deadline = input.deadline.to_string();
                let obligation = self.obligations().ensure(
                    &format!("obl-{run_id}"),
                    &period,
                    &input.kind,
                    &input.description,
                    &deadline,
                    run_id,
                )?;
                format!(
                    "obligation {} opened (deadline {})",
                    obligation.id, obligation.deadline
                )
            } else {
                "no follow-up obligation supplied; no F13 obligation opened".to_string()
            };
            journal.append(EventPayload::EffectSucceeded {
                effect_key: EFFECT_OBLIGATION.to_string(),
                note,
            })?;
        }

        // Distil a knowledge candidate from the advice and park on the review gate.
        let f = fold(run_id, &journal.read_all()?)?;
        if f.gate(GATE_APPROVE_KNOWLEDGE).is_none() {
            let candidate_hash = knowledge_candidate_hash(state);
            journal.append(EventPayload::GateOpened {
                gate_id: GATE_APPROVE_KNOWLEDGE.to_string(),
                gate_kind: "Approval".to_string(),
                snapshot_hash: candidate_hash,
            })?;
        }
        // Persist any state changes (none here, but keep the write for symmetry/safety).
        self.write_capture_state(run_id, state)?;
        Ok(())
    }

    /// On `ApproveKnowledgePR`: commit the (PII-free) knowledge candidate distilled from
    /// the consult. This is the *only* path new `knowledge` enters — no auto-cache.
    fn write_knowledge(
        &self,
        journal: &Journal,
        run_id: &str,
        state: &CaptureState,
    ) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect_succeeded(EFFECT_KNOWLEDGE) {
            return Ok(());
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_KNOWLEDGE.to_string(),
        })?;
        let advice = state
            .advice
            .clone()
            .ok_or_else(|| ProcessError::InvalidAnswer("no advice to distil".into()))?;
        let candidate = advice.into_knowledge_candidate(&state.knowledge_slug);
        self.knowledge()
            .put(&candidate.slug, &candidate.to_document())?;
        journal.append(EventPayload::EffectSucceeded {
            effect_key: EFFECT_KNOWLEDGE.to_string(),
            note: format!("knowledge/{} written (review-gated)", candidate.slug),
        })?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Projection + envelope builders (deterministic; the projection IS the sent body)
// ---------------------------------------------------------------------------

fn build_projection(topic: &str, question: &str, facts: &[FactInput]) -> ConsultQuestion {
    let mut q = ConsultQuestion::new(topic, question);
    for f in facts {
        // `value` is a plain String — a DisclosureSafe type. A PII newtype could not be
        // placed here (compile error), which is the C3-typed leak barrier.
        q = q.with_fact(f.key.clone(), f.value.clone());
    }
    q
}

fn tool_result_projection(topic: &str, tool: &str, result: Option<&Value>) -> ConsultQuestion {
    let rendered = result
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string());
    ConsultQuestion::new(topic, "tool result")
        .with_fact("tool", tool.to_string())
        .with_fact("result", rendered)
}

fn disclosure_envelope(run_id: &str, body: &Value) -> Envelope {
    Envelope::new(
        SVC_CONSULT_LLM,
        "consult",
        format!("consult:{run_id}"),
        body.clone(),
    )
}

fn draft_body(draft: &DraftInput) -> Result<Value, ProcessError> {
    serde_json::to_value(draft).map_err(|e| ProcessError::Store(e.to_string()))
}

fn draft_envelope(run_id: &str, body: &Value) -> Envelope {
    Envelope::new(
        "capture-receipt",
        "voucher-draft",
        format!("draft:{run_id}"),
        body.clone(),
    )
}

/// A stable content hash of the knowledge candidate (the Approval gate's snapshot).
fn knowledge_candidate_hash(state: &CaptureState) -> String {
    let mut h = Sha256::new();
    let mut feed = |s: &str| {
        h.update((s.len() as u64).to_be_bytes());
        h.update(s.as_bytes());
    };
    feed(&state.knowledge_slug);
    if let Some(a) = &state.advice {
        feed(&a.summary);
        for c in &a.citations {
            feed(c);
        }
    }
    hex::encode(h.finalize())
}

/// Re-verify a Preview-gate's persisted bytes against both the state hash and the bound
/// gate hash before any side effect (the S3 exact-bytes discipline).
fn assert_bound(env: &Envelope, state_hash: &str, gate_hash: &str) -> Result<(), ProcessError> {
    let recomputed = env.sent_hash().map_err(map_broker)?;
    if recomputed != state_hash || recomputed != gate_hash {
        return Err(ProcessError::JournalCorrupt(format!(
            "the payload changed after approval was bound (bound {gate_hash}, now {recomputed}) \
             — refusing to send unapproved bytes"
        )));
    }
    Ok(())
}

fn map_domain(e: aggountant_domain::DomainError) -> ProcessError {
    ProcessError::InvalidAnswer(format!("{}: {e}", e.error_code()))
}

fn map_broker(e: aggountant_broker::BrokerError) -> ProcessError {
    ProcessError::Store(format!("{}: {e}", e.error_code()))
}
