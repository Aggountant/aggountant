//! The **`file-vat` Run** (RF3, scenario B2) — the process orchestration of the S3
//! edge slice. It composes the deterministic Domain (VAT figures), the Ledger (the
//! audited, irreversible filing event) and the Broker (mTLS, durable outbox and
//! honest `OutcomeUnknown`) into one interruptible, stateful Run.
//!
//! Shape (the `FILE_VAT_WORKFLOW` definition):
//! `GetVATPeriods` → **`ApproveFiling`** (a `Preview` gate that binds the final
//! FileVATReturn/v2 bytes) → `Submit(vero-api)` →
//! `file_vat` (audited) → **`ApprovePayment`** (binds the payment `sent_hash`) →
//! `Submit(pankki-maksu)`.
//!
//! The M3 invariants live here at the Run level:
//! - a submit `Unknown` (timeout/5xx) parks the Run in `NeedsAttention` — **no blind
//!   retry**; only GetFiledVATReturn semantic reconciliation may recover it;
//! - each `Preview` gate recomputes the persisted envelope's `sent_hash` and
//!   requires it to equal the hash it bound at open time — **the approval signs the
//!   exact bytes** (a tampered payload is refused);
//! - the payment amount must equal `reconciliation.bank_payment_required` (the
//!   net-invariant, `PAYMENT_EXCEEDS_RECONCILED_AMOUNT`);
//! - the filing is recorded through the Ledger's irreversible audited path.

use std::fs;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_json::json;

use aggountant_broker::{
    project_file_vat_return, Broker, Envelope, FileVatReceipt, FileVatReturn, FiledVatReturn,
    GetVatPeriodsResponse, RemoteOutcome, SVC_PANKKI_MAKSU, SVC_VERO_FILED_VAT_RETURN,
    SVC_VERO_FILE_VAT_RETURN, SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::{compute_derived_vat_filing, PolicySet, VatFiling};
use aggountant_ledger::{Ledger, ManualVatFilingEvidence, PeriodState, PeriodView};
use aggountant_model::{BusinessId, FilingContactName, FilingContactPhone, Money, PeriodId};
use sha2::{Digest, Sha256};

use crate::journal::{EventPayload, Journal};
use crate::vat_reconciliation::{
    attention_items, reconcile_period, require_reconciled, VatAttentionItem,
    VatReconciliationAcknowledgement,
};
use crate::{fold, EffectStatus, Fold, GateView, ProcessEngine, ProcessError, RunView, Seam};

mod status;
pub use status::{FileVatFilingStatus, FileVatStatus, FileVatStatusPlan, FiledVatQuery};

/// Effect keys for the `file-vat` Run (stable idempotency keys).
const EFFECT_RECONCILE: &str = "reconcile";
const EFFECT_SUBMIT_FILING: &str = "submit-filing";
const EFFECT_SUBMIT_PAYMENT: &str = "submit-payment";
const GATE_APPROVE_FILING: &str = "ApproveFiling";
const GATE_APPROVE_PAYMENT: &str = "ApprovePayment";
const ACTOR: &str = "process:file-vat";

/// Structured input for `file-vat` (CLI stdin JSON): the period and its aggregated
/// VAT figures, plus where the bank payment goes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileVatInput {
    pub period: PeriodId,
    pub sales_vat: Money,
    pub purchase_vat: Money,
    /// Filing identity is intentionally scoped here rather than in a company registry.
    pub business_id: BusinessId,
    pub contact_name: FilingContactName,
    pub contact_phone: FilingContactPhone,
    /// The IBAN for the separate payment approval.
    pub payee_iban: String,
    /// Required only when the period has untagged VAT-account postings.
    #[serde(default)]
    pub vat_reconciliation_acknowledgement: Option<VatReconciliationAcknowledgement>,
}

/// Run-scoped state persisted at reconcile time so `answer`/`retry`/`resume` can
/// re-derive the exact approved envelopes (the `Preview`-gate binding depends on
/// this being deterministic and durable).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileVatState {
    period: String,
    policy_set: String,
    voucher_source_fingerprint: String,
    filing_fields: serde_json::Value,
    sales_vat: String,
    purchase_vat: String,
    filing_body: serde_json::Value,
    filing_sent_hash: String,
    payment_body: serde_json::Value,
    payment_sent_hash: String,
    bank_payment_required: String,
    net_vat: String,
    vero_period: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vat_reconciliation_acknowledgement: Option<VatReconciliationAcknowledgement>,
}

struct VerifiedFilingSnapshot {
    filing: VatFiling,
    voucher_source_fingerprint: String,
    reconciliation: aggountant_domain::VatAccountReconciliation,
}

/// The start of a `file-vat` Run (the `RunView` plus a couple of figures a CLI
/// wants to echo).
#[derive(Debug, Clone, Serialize)]
pub struct FileVatStart {
    #[serde(flatten)]
    pub run: RunView,
    pub net_vat: String,
    pub bank_payment_required: String,
    /// Exact UTF-8 FileVATReturn/v2 bytes bound by the approval gate.
    pub filing_request: String,
    pub vero_period: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_reconciliation_acknowledgement: Option<VatReconciliationAcknowledgement>,
}

/// Read-only manual filing view. The numbered fields are the same Domain projection
/// used by direct Vero filing; this type adds only operator-facing instructions.
#[derive(Debug, Clone, Serialize)]
pub struct OfflineVatPreview {
    pub period: String,
    pub filing_fields: aggountant_domain::VatReturnFields,
    pub sales_vat: String,
    pub purchase_vat: String,
    pub net_vat: String,
    pub payment_amount: String,
    pub vat_reconciliation: aggountant_domain::VatAccountReconciliation,
    pub attention: Vec<VatAttentionItem>,
    pub filing_instruction: &'static str,
    pub payment_instruction: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManualVatCompletion {
    #[serde(flatten)]
    pub period: PeriodView,
    pub manual_filing: ManualVatFilingEvidence,
}

impl ProcessEngine {
    fn file_vat_state_path(&self, run_id: &str) -> std::path::PathBuf {
        self.run_dir(run_id).join("file_vat_state.json")
    }

    fn write_file_vat_state(&self, run_id: &str, state: &FileVatState) -> Result<(), ProcessError> {
        let bytes =
            serde_json::to_vec_pretty(state).map_err(|e| ProcessError::Store(e.to_string()))?;
        // Atomic durable write (temp → fsync → rename), matching the journal/outbox:
        // a crash mid-write never leaves a truncated state file that would break the
        // gate re-binding on resume.
        let path = self.file_vat_state_path(run_id);
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

    fn read_file_vat_state(&self, run_id: &str) -> Result<FileVatState, ProcessError> {
        let bytes = fs::read(self.file_vat_state_path(run_id))
            .map_err(|e| ProcessError::Store(format!("file-vat state unreadable: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("file-vat state corrupt: {e}")))
    }

    /// Require the current local profile to identify the exact filing target and
    /// contact already snapshotted into the approved request. Readiness itself is
    /// evaluated by the shared Broker evaluator; this check prevents a ready but
    /// changed profile from redirecting an awaiting Run.
    pub fn ensure_file_vat_profile_matches(
        &self,
        run_id: &str,
        profile: &aggountant_broker::VeroFilingProfile,
    ) -> Result<(), ProcessError> {
        let state = self.read_file_vat_state(run_id)?;
        let request = state
            .filing_body
            .get("request_bytes")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ProcessError::JournalCorrupt("file-vat request bytes missing".into()))?;
        let request: FileVatReturn = serde_json::from_str(request).map_err(|_| {
            ProcessError::JournalCorrupt("file-vat request bytes are malformed".into())
        })?;
        let matches = profile.business_id.as_ref().map(BusinessId::as_str)
            == Some(request.business_id.as_str())
            && profile.contact_name.as_ref().map(FilingContactName::as_str)
                == Some(request.contact_details.full_name.as_str())
            && profile
                .contact_phone
                .as_ref()
                .map(FilingContactPhone::as_str)
                == Some(request.contact_details.phone_number.as_str());
        if !matches {
            return Err(ProcessError::Store(
                "VERO_FILING_PROFILE_CHANGED: create a new filing preview".into(),
            ));
        }
        Ok(())
    }

    /// Derive the manual filing view without creating a Run or touching Broker,
    /// profile, credentials, or network state.
    pub fn preview_offline_vat(
        &self,
        period: &PeriodId,
        policy_set: &PolicySet,
    ) -> Result<OfflineVatPreview, ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        let verified = derive_verified_sealed_filing(&ledger, period, policy_set)?;
        let filing = verified.filing;
        let payment_amount = if filing.net_vat.amount().is_sign_negative() {
            "0.00".to_string()
        } else {
            filing.net_vat.amount_str()
        };
        Ok(OfflineVatPreview {
            period: period.to_string(),
            filing_fields: filing.fields,
            sales_vat: filing.sales_vat.amount_str(),
            purchase_vat: filing.purchase_vat.amount_str(),
            net_vat: filing.net_vat.amount_str(),
            payment_amount,
            attention: attention_items(&verified.reconciliation),
            vat_reconciliation: verified.reconciliation,
            filing_instruction: "Enter these numbered VAT fields manually in OmaVero, or use an independently prepared Ilmoitin.fi file; aggountant does not create or upload that file.",
            payment_instruction: "Pay separately using the current payment details and reference shown in OmaVero; aggountant does not initiate the payment.",
        })
    }

    /// Record evidence that the operator completed a manual filing. This creates no
    /// Run and performs no remote call. A prior uncertain API submission requires a
    /// separate reference proving that its remote status was resolved first.
    pub fn complete_manual_vat(
        &self,
        period: &PeriodId,
        channel: &str,
        evidence_ref: &str,
        remote_status_evidence_ref: Option<&str>,
    ) -> Result<ManualVatCompletion, ProcessError> {
        if !matches!(channel, "omavero" | "ilmoitin") {
            return Err(ProcessError::InvalidAnswer(
                "INVALID_MANUAL_VAT_CHANNEL: expected omavero|ilmoitin".into(),
            ));
        }
        validate_evidence_ref(evidence_ref)?;
        if let Some(reference) = remote_status_evidence_ref {
            validate_evidence_ref(reference)?;
        }
        if self.period_has_unresolved_unknown_filing(period)?
            && remote_status_evidence_ref.is_none()
        {
            return Err(ProcessError::InvalidAnswer(
                "OUTCOME_UNKNOWN: manual filing is refused until remote status resolution evidence is supplied".into(),
            ));
        }
        let ledger = Ledger::open(&self.home)?;
        let authority = ledger.acquire_filing_authority(period, "manual-file-vat")?;
        let period_view = ledger.file_vat_manually(
            period,
            &authority,
            channel,
            evidence_ref,
            remote_status_evidence_ref,
            "process:file-vat-manual",
        )?;
        let manual_filing = ledger.manual_vat_filing_evidence(period)?.ok_or_else(|| {
            ProcessError::Store("manual VAT filing evidence missing after commit".into())
        })?;
        Ok(ManualVatCompletion {
            period: period_view,
            manual_filing,
        })
    }

    fn period_has_unresolved_unknown_filing(
        &self,
        period: &PeriodId,
    ) -> Result<bool, ProcessError> {
        for run_id in self.list()? {
            let state_path = self.file_vat_state_path(&run_id);
            if !state_path.exists() {
                continue;
            }
            let state = self.read_file_vat_state(&run_id)?;
            if state.period != period.to_string() {
                continue;
            }
            let events = self.open_journal(&run_id)?.read_all()?;
            let unresolved_unknown = events.iter().rev().find_map(|event| match &event.payload {
                EventPayload::EffectSucceeded { effect_key, .. }
                | EventPayload::EffectReconciled { effect_key, .. }
                    if effect_key == EFFECT_SUBMIT_FILING =>
                {
                    Some(false)
                }
                EventPayload::EffectRejected {
                    effect_key,
                    error_code,
                } if effect_key == EFFECT_SUBMIT_FILING => Some(error_code == "OUTCOME_UNKNOWN"),
                _ => None,
            });
            if unresolved_unknown == Some(true) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Start a `file-vat` Run: precondition-check (period `Sealed`), pull the
    /// verotili snapshot, compute + reconcile the filing, persist the envelopes, and
    /// park on the `ApproveFiling` `Preview` gate bound to the filing `sent_hash`.
    pub fn start_file_vat(
        &self,
        input: &FileVatInput,
        broker: &Broker,
        policy_set: &PolicySet,
    ) -> Result<FileVatStart, ProcessError> {
        let run_id = crate::new_run_id_with("fv");
        let run_dir = self.run_dir(&run_id);
        fs::create_dir_all(&run_dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let _lease = self.acquire_lease(&run_id)?;
        let journal = Journal::new(&run_dir);

        journal.append(EventPayload::RunStarted {
            workflow: "file-vat".to_string(),
            workflow_version: crate::file_vat_workflow_version(),
            git_commit: crate::git_commit(),
            policy_set: policy_set.id.clone(),
            period: input.period.to_string(),
        })?;
        for (key, ty) in [
            (EFFECT_RECONCILE, "Reconcile"),
            (
                EFFECT_SUBMIT_FILING,
                "ExchangeAdapter(vero-file-vat-return)",
            ),
            (EFFECT_SUBMIT_PAYMENT, "ExchangeAdapter(pankki-maksu)"),
        ] {
            journal.append(EventPayload::EffectPlanned {
                effect_key: key.to_string(),
                effect_type: ty.to_string(),
            })?;
        }

        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_RECONCILE.to_string(),
        })?;

        let ledger = Ledger::open(&self.home)?;
        // Accounting-integrity cross-check (issue `vat-figures-from-sealed-ledger`):
        // derive the period's VAT aggregates from the sealed vouchers and require the
        // CLI-supplied figures to match them exactly. The `sent_hash`/Preview gate
        // binds the approval to whatever figures were computed — but without this the
        // figures were merely trusted from input. Deriving them from the sealed ledger
        // means the approval binds to figures the ledger itself can reproduce. A
        // mismatch rejects the reconcile effect and parks the Run NeedsAttention (never
        // a silent file of unverified figures).
        //
        // ADR-7 §3: `derive_filing_figures` is the authoritative projection — the
        // legacy tagged `base × rate` figures **plus** the account-sum over the sealed
        // VAT-A legs (reverse charge + domestic input VAT). A VAT-A period that the
        // tagged path alone reports as 0,00 now files its real self-assessed /
        // deductible amounts.
        let verified = match derive_verified_sealed_filing(&ledger, &input.period, policy_set) {
            Ok(verified) => verified,
            Err(error) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_RECONCILE.to_string(),
                    error_code: error.error_code().to_string(),
                })?;
                return Err(error);
            }
        };
        let filing = verified.filing;
        // Currency-aware value equality (not string): a EUR figure must not match a
        // same-numeral non-EUR input, and `Money`/`Decimal` compares across scale.
        if filing.sales_vat != input.sales_vat || filing.purchase_vat != input.purchase_vat {
            journal.append(EventPayload::EffectRejected {
                effect_key: EFFECT_RECONCILE.to_string(),
                error_code: "VAT_FIGURES_MISMATCH".to_string(),
            })?;
            return Err(ProcessError::VatFiguresMismatch(format!(
                "period {}: input sales_vat {} / purchase_vat {} disagree with the sealed \
                 ledger's derived sales_vat {} / purchase_vat {}",
                input.period,
                input.sales_vat.amount_str(),
                input.purchase_vat.amount_str(),
                filing.sales_vat.amount_str(),
                filing.purchase_vat.amount_str(),
            )));
        }
        let acknowledgement = input.vat_reconciliation_acknowledgement.as_ref();
        if let Err(error) = require_reconciled(&verified.reconciliation, acknowledgement) {
            journal.append(EventPayload::EffectRejected {
                effect_key: EFFECT_RECONCILE.to_string(),
                error_code: error.error_code().to_string(),
            })?;
            return Err(error);
        }

        // ADR-9 preflight: read the authority's exact period before opening approval.
        let periods_request = serde_json::to_string(&json!({
            "BusinessId": input.business_id.as_str(),
            "FilingYear": input.period.year(),
        }))
        .map_err(|e| ProcessError::Store(e.to_string()))?;
        let periods_env = Envelope::new(
            SVC_VERO_VAT_PERIODS,
            "GetVATPeriods/v1",
            format!("periods:{}", input.period),
            json!({ "request_bytes": periods_request }),
        );
        let periods_body = broker.fetch(&periods_env).map_err(map_broker)?;
        let periods: GetVatPeriodsResponse = serde_json::from_value(periods_body)
            .map_err(|e| ProcessError::Store(format!("GetVATPeriods response malformed: {e}")))?;
        let (period_start, period_end) = period_dates(&input.period)?;
        let vero_period = periods
            .vat_periods
            .into_iter()
            .find(|period| period.start_date == period_start && period.end_date == period_end)
            .ok_or_else(|| {
                ProcessError::Store(format!(
                    "VERO_VAT_PERIOD_MISSING: authority did not offer period {period_start}..{period_end}"
                ))
            })?;
        if !matches!(vero_period.status.as_str(), "Unfiled" | "Missing") {
            return Err(ProcessError::Store(format!(
                "VERO_VAT_PERIOD_NOT_FILEABLE: period {} has remote status {}",
                vero_period.period, vero_period.status
            )));
        }
        let vero_period =
            serde_json::to_value(vero_period).map_err(|e| ProcessError::Store(e.to_string()))?;

        // The sole projection builds the final API request. Persist its exact bytes;
        // approval and dispatch both bind to this same string.
        let request = project_file_vat_return(
            &input.period,
            &filing.fields,
            &input.business_id,
            &input.contact_name,
            &input.contact_phone,
        )
        .map_err(map_adapter)?;
        let request_bytes = request.bytes().map_err(map_adapter)?;
        let request_string = String::from_utf8(request_bytes)
            .map_err(|e| ProcessError::Store(format!("FileVATReturn bytes are not UTF-8: {e}")))?;
        let filing_sent_hash = request.sent_hash().map_err(map_adapter)?;
        let filing_body = json!({ "request_bytes": request_string });
        let filing_approval_hash = filing_approval_hash(&filing_body, &vero_period)?;

        // Payment remains a separate approval/effect. GetVATPeriods does not expose
        // tax-account credit, so this slice displays the filing net without pretending
        // it has reconciled a bank balance.
        let bank_payment_required = if filing.net_vat.amount().is_sign_negative() {
            "0.00".to_string()
        } else {
            filing.net_vat.amount_str()
        };
        let payment_body = json!({
            "period": input.period.to_string(),
            "amount": bank_payment_required,
            "payee_iban": input.payee_iban,
        });
        let payment_env = Envelope::new(
            SVC_PANKKI_MAKSU,
            "pay-vat",
            format!("pay:{}", input.period),
            payment_body.clone(),
        );
        let payment_sent_hash = payment_env.sent_hash().map_err(map_broker)?;

        // Acquire the local filing authority only after the read-only remote preflight
        // and all projection validation have succeeded. An authority outage or invalid
        // response must not strand an undiscoverable Run as the period's filing owner.
        if let Err(error) = ledger.acquire_filing_authority(&input.period, &run_id) {
            journal.append(EventPayload::EffectRejected {
                effect_key: EFFECT_RECONCILE.to_string(),
                error_code: error.error_code().to_string(),
            })?;
            return Err(error.into());
        }

        self.write_file_vat_state(
            &run_id,
            &FileVatState {
                period: input.period.to_string(),
                policy_set: policy_set.id.clone(),
                voucher_source_fingerprint: verified.voucher_source_fingerprint,
                filing_fields: serde_json::to_value(&filing.fields)
                    .map_err(|e| ProcessError::Store(e.to_string()))?,
                sales_vat: filing.sales_vat.amount_str(),
                purchase_vat: filing.purchase_vat.amount_str(),
                filing_body: filing_body.clone(),
                filing_sent_hash: filing_sent_hash.clone(),
                payment_body,
                payment_sent_hash,
                bank_payment_required: bank_payment_required.clone(),
                net_vat: filing.net_vat.amount_str(),
                vero_period: vero_period.clone(),
                vat_reconciliation_acknowledgement: acknowledgement.cloned(),
            },
        )?;

        journal.append(EventPayload::EffectSucceeded {
            effect_key: EFFECT_RECONCILE.to_string(),
            note: match acknowledgement {
                None => format!(
                    "net_vat {}, bank_payment_required {}",
                    filing.net_vat.amount_str(),
                    bank_payment_required
                ),
                Some(ack) => format!(
                    "net_vat {}, bank_payment_required {}, untagged VAT on vouchers {:?} acknowledged: {}",
                    filing.net_vat.amount_str(),
                    bank_payment_required,
                    ack.vouchers,
                    ack.reason
                ),
            },
        })?;
        // The ApproveFiling Preview gate binds to the filing envelope's sent_hash.
        journal.append(EventPayload::GateOpened {
            gate_id: GATE_APPROVE_FILING.to_string(),
            gate_kind: "Preview".to_string(),
            snapshot_hash: filing_approval_hash,
        })?;

        let run = fold(&run_id, &journal.read_all()?)?.to_view();
        Ok(FileVatStart {
            net_vat: filing.net_vat.amount_str(),
            bank_payment_required,
            filing_request: filing_body["request_bytes"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            vero_period,
            vat_reconciliation_acknowledgement: acknowledgement.cloned(),
            run,
        })
    }

    /// Answer a `file-vat` gate (`ApproveFiling` or `ApprovePayment`). `approve`
    /// runs the corresponding submit; `reject` declines (the Run is Rejected). Every
    /// approve re-verifies the persisted envelope's `sent_hash` against the hash the
    /// gate bound — the approval signs the exact sent bytes.
    pub fn answer_file_vat(
        &self,
        run_id: &str,
        gate_id: &str,
        answer: &str,
        broker: &Broker,
    ) -> Result<RunView, ProcessError> {
        self.answer_file_vat_seam(run_id, gate_id, answer, broker, Seam::None)
    }

    /// As [`answer_file_vat`], with a crash seam for the `file-vat` resume tests
    /// (the broker-success / fiscal-COMMIT windows the spike-notes flag, plus the
    /// approve-then-crash-before-submit window). Production callers pass [`Seam::None`].
    pub fn answer_file_vat_seam(
        &self,
        run_id: &str,
        gate_id: &str,
        answer: &str,
        broker: &Broker,
        seam: Seam,
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

        let state = self.read_file_vat_state(run_id)?;
        let period = PeriodId::parse(&state.period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;

        // Preview-gate binding: recompute the persisted envelope's sent_hash and
        // require it to equal the bound hash — the approval signs the exact bytes.
        if answer == "approve" {
            let recomputed = match gate_id {
                GATE_APPROVE_FILING => {
                    if filing_payload_hash(&state.filing_body)? != state.filing_sent_hash {
                        return Err(ProcessError::JournalCorrupt(
                            "ApproveFiling: final request bytes changed after projection".into(),
                        ));
                    }
                    filing_approval_hash(&state.filing_body, &state.vero_period)?
                }
                GATE_APPROVE_PAYMENT => {
                    let hash = rebuild_payment_env(&period, &state.payment_body)
                        .sent_hash()
                        .map_err(map_broker)?;
                    if hash != state.payment_sent_hash {
                        return Err(ProcessError::JournalCorrupt(
                            "ApprovePayment: payload changed after projection".into(),
                        ));
                    }
                    hash
                }
                other => return Err(ProcessError::GateNotOpen(format!("unknown gate {other}"))),
            };
            if recomputed != gate.snapshot_hash {
                return Err(ProcessError::JournalCorrupt(format!(
                    "{gate_id}: the approval snapshot changed (bound {}, now {recomputed})",
                    gate.snapshot_hash
                )));
            }
        }

        journal.append(EventPayload::GateAnswered {
            gate_id: gate_id.to_string(),
            answer: answer.to_string(),
        })?;

        // Crash seam: the operator approved the gate but the process died before the
        // downstream submit ran (gate answered, effect still Planned). `resume_file_vat`
        // must forward-drive it — otherwise the Run wedges behind an answered gate.
        if seam == Seam::CrashAfterGateAnswer {
            return Ok(fold(run_id, &journal.read_all()?)?.to_view());
        }

        if answer == "approve" {
            match gate_id {
                GATE_APPROVE_FILING => {
                    self.run_submit_filing(&journal, run_id, &period, &state, broker, seam)?
                }
                GATE_APPROVE_PAYMENT => {
                    self.run_submit_payment(&journal, run_id, &period, &state, broker)?
                }
                _ => {}
            }
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Reconcile a Vero filing parked on `OutcomeUnknown` using the documented
    /// GetFiledVATReturn/v2 read. Missing, stale, and mismatching values remain
    /// NeedsAttention; matching semantic values complete locally without a re-submit.
    pub fn reconcile_file_vat(
        &self,
        run_id: &str,
        broker: &Broker,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        if f.effect(EFFECT_SUBMIT_FILING).map(|effect| effect.status)
            != Some(EffectStatus::Rejected)
        {
            return Err(ProcessError::InvalidAnswer(
                "submit-filing is not parked for reconciliation".into(),
            ));
        }
        let state = self.read_file_vat_state(run_id)?;
        assert_filing_gate_approved_and_bound(
            &f,
            &state.filing_body,
            &state.vero_period,
            &state.filing_sent_hash,
        )?;
        let expected = approved_filing_request(&state)?;
        let envelope = filed_vat_return_envelope(&expected, format!("reconcile:{}", state.period))?;
        let body = broker.fetch(&envelope).map_err(map_broker)?;
        let filed: FiledVatReturn = serde_json::from_value(body)
            .map_err(|e| ProcessError::Store(format!("GetFiledVATReturn malformed: {e}")))?;
        let effect_key = format!("{EFFECT_SUBMIT_FILING}:{}", state.period);
        let outbox = broker
            .outbox_entry(SVC_VERO_FILE_VAT_RETURN, &effect_key)
            .map_err(map_broker)?
            .ok_or_else(|| ProcessError::Store("Vero filing outbox entry is missing".into()))?;
        let attempted_date = outbox
            .attempts
            .last()
            .and_then(|attempt| attempt.provenance.ts.get(..10))
            .ok_or_else(|| {
                ProcessError::Store("Vero filing attempt timestamp is missing".into())
            })?;
        if filed.semantically_matches_attempt(&expected, Some(attempted_date)) {
            let period = PeriodId::parse(&state.period)
                .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;
            let ledger = Ledger::open(&self.home)?;
            if !ledger.period_view(&period)?.vat_filed {
                let auth = ledger.acquire_filing_authority(&period, run_id)?;
                // A reconciliation response carries no FileVATReturn receipt fields;
                // never invent them. The approved payload hash is still audited.
                ledger.file_vat(&period, &auth, &state.filing_sent_hash, ACTOR)?;
            }
            journal.append(EventPayload::EffectReconciled {
                effect_key: EFFECT_SUBMIT_FILING.into(),
                note: "GetFiledVATReturn semantically matches approved FileVATReturn bytes".into(),
            })?;
            let f2 = fold(run_id, &journal.read_all()?)?;
            self.finish_filing_postcondition(&journal, &f2, &state)?;
        }
        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Deliberately re-drive a parked payment submit. Vero filing must use
    /// `reconcile_file_vat` and is never blindly re-submitted.
    pub fn retry_file_vat_submit(
        &self,
        run_id: &str,
        effect_key: &str,
        broker: &Broker,
    ) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        let state = self.read_file_vat_state(run_id)?;
        let period = PeriodId::parse(&state.period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;

        // Preconditions (a deliberate retry is not a way around the workflow): the
        // effect must be *parked* (its last state Rejected — an OUTCOME_UNKNOWN /
        // failed attempt), its gate must be approved, its predecessor complete, and
        // — crucially — the persisted envelope must still bind to the approved bytes.
        // Without this, a retry could send unapproved/tampered bytes (the same
        // binding the gate-answer path enforces).
        let (gate_id, env, bound) = match effect_key {
            EFFECT_SUBMIT_FILING => {
                return Err(ProcessError::InvalidAnswer(
                    "a Vero filing with OutcomeUnknown must be reconciled with GetFiledVATReturn; blind re-submit is forbidden".into(),
                ));
            }
            EFFECT_SUBMIT_PAYMENT => {
                if !f.effect_succeeded(EFFECT_SUBMIT_FILING) {
                    return Err(ProcessError::InvalidAnswer(
                        "cannot retry submit-payment before the filing succeeded".into(),
                    ));
                }
                (
                    GATE_APPROVE_PAYMENT,
                    rebuild_payment_env(&period, &state.payment_body),
                    &state.payment_sent_hash,
                )
            }
            other => {
                return Err(ProcessError::InvalidAnswer(format!(
                    "cannot retry unknown effect {other}"
                )))
            }
        };
        if f.effect(effect_key).map(|e| e.status) != Some(EffectStatus::Rejected) {
            return Err(ProcessError::InvalidAnswer(format!(
                "effect {effect_key} is not parked for retry (only a failed/unknown effect may be retried)"
            )));
        }
        assert_gate_approved_and_bound(&f, gate_id, &env, bound)?;

        match effect_key {
            EFFECT_SUBMIT_PAYMENT => {
                self.run_submit_payment(&journal, run_id, &period, &state, broker)?
            }
            _ => unreachable!("effect key validated above"),
        }
        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// Resume an interrupted `file-vat` Run (crash recovery). Reconciles the Run to
    /// authoritative fiscal state the way `close-period`'s `resume` does — closing the
    /// S3 spike-notes gap where a crash *between the broker success and the fiscal
    /// `file_vat` COMMIT* left the Run wedged (the filing effect stuck `Executing`, so
    /// even the deliberate `retry` — which requires a *parked* `Rejected` effect —
    /// could not advance it).
    ///
    /// Resume is safe to run repeatedly because a filing is dispatched only when it
    /// was still Planned, or replayed locally after the outbox has a terminal receipt:
    /// - **fiscal committed, process lagging** (`vat_filed` set but no `EffectSucceeded`):
    ///   adopt fiscal (fiscal wins) *without* re-hitting the authority, then repair the
    ///   payment-phase postcondition (open `ApprovePayment` or record the no-payment
    ///   completion) exactly as the happy path does;
    /// - **broker receipt persisted, fiscal not committed**: replay the terminal
    ///   outbox result locally and complete the fiscal COMMIT without another HTTP call;
    /// - **payment approve-then-crash** (filing done, `ApprovePayment` approved, submit
    ///   not run): re-drive the payment submit (idempotent on its outbox key).
    ///
    /// A crash re-drive re-verifies the approved-bytes binding first (windows 2/3), so a
    /// crash+tamper never lets resume send bytes the gate did not sign. It does **not**
    /// re-drive a *parked* effect: an effect left `Rejected` by a genuine `Unknown` is
    /// the deliberate-`retry` pathway's job, so resume only ever re-drives an effect
    /// still `Planned`/`Executing` — that keeps the M3 "no blind retry" invariant intact.
    /// Window 1 (adopt-fiscal) does **not** verify the fiscal filing's stored `sent_hash`
    /// equals this run's approved bytes (`period_view` does not expose it, and the ledger
    /// is out of scope this round); it instead requires that this run *approved* its own
    /// `ApproveFiling` gate, so an unapproved/rejected run never claims a filing it did
    /// not authorise. Full sent-hash provenance is a tracked follow-up.
    pub fn resume_file_vat(&self, run_id: &str, broker: &Broker) -> Result<RunView, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let f = fold(run_id, &journal.read_all()?)?;
        self.check_version(&f)?;
        let state = self.read_file_vat_state(run_id)?;
        let period = PeriodId::parse(&state.period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;
        let ledger = Ledger::open(&self.home)?;
        let fiscal = ledger.period_view(&period)?;

        // --- Reconcile the filing ---
        if !f.effect_succeeded(EFFECT_SUBMIT_FILING) {
            if fiscal.vat_filed && f.gate_approved(GATE_APPROVE_FILING) {
                // Fiscal COMMIT landed but the process never recorded the success. Adopt
                // fiscal (fiscal wins) — no authority round-trip. Gate-approval is required
                // so a run that never authorised *its* filing cannot claim the fiscal one
                // (an unapproved/rejected run stays put). Single-user (v1): the only run
                // filing this period is this one; a divergent-figures sequential-run case
                // needs the filed sent_hash the ledger does not yet expose (follow-up).
                journal.append(EventPayload::EffectReconciled {
                    effect_key: EFFECT_SUBMIT_FILING.to_string(),
                    note: format!(
                        "fiscal shows period {period} vat_filed — reconciled (fiscal wins)"
                    ),
                })?;
            } else if !fiscal.vat_filed
                && f.gate_approved(GATE_APPROVE_FILING)
                && filing_resume_safe(broker, &f, &period)?
            {
                // A Planned effect has not dispatched; an Executing effect is resumed
                // only when the outbox already has a terminal receipt and will replay
                // locally. An unresolved in-flight attempt is never re-sent. Never fires
                // on a parked `Rejected` — that is `retry`'s job). The shared
                // dispatch guard re-checks both approved bytes and current fiscal
                // source before this path can touch Broker.
                self.run_submit_filing(&journal, run_id, &period, &state, broker, Seam::None)?;
            }
        }

        // Converge the payment-phase postcondition. Covers the crash window *between* the
        // filing `EffectSucceeded`/`EffectReconciled` and `finish_filing_postcondition`:
        // the filing is done but no `ApprovePayment` gate (nor a no-payment success) exists
        // yet, which would otherwise wedge the Run `Running` forever. Idempotent.
        let f2 = fold(run_id, &journal.read_all()?)?;
        if f2.effect_succeeded(EFFECT_SUBMIT_FILING)
            && !f2.effect_succeeded(EFFECT_SUBMIT_PAYMENT)
            && f2.gate(GATE_APPROVE_PAYMENT).is_none()
        {
            self.finish_filing_postcondition(&journal, &f2, &state)?;
        }

        // --- Reconcile the payment (approve-then-crash before the submit ran) ---
        let f3 = fold(run_id, &journal.read_all()?)?;
        if f3.effect_succeeded(EFFECT_SUBMIT_FILING)
            && !f3.effect_succeeded(EFFECT_SUBMIT_PAYMENT)
            && f3.gate_approved(GATE_APPROVE_PAYMENT)
            && crash_resumable(&f3, EFFECT_SUBMIT_PAYMENT)
        {
            let env = rebuild_payment_env(&period, &state.payment_body);
            assert_gate_approved_and_bound(
                &f3,
                GATE_APPROVE_PAYMENT,
                &env,
                &state.payment_sent_hash,
            )?;
            self.run_submit_payment(&journal, run_id, &period, &state, broker)?;
        }

        Ok(fold(run_id, &journal.read_all()?)?.to_view())
    }

    /// The filing submit: broker.submit → on a terminal success, record the audited,
    /// irreversible Ledger filing bound to the `sent_hash`, then open the
    /// `ApprovePayment` gate; on `Unknown`, park `NeedsAttention` (no blind retry).
    fn run_submit_filing(
        &self,
        journal: &Journal,
        run_id: &str,
        period: &PeriodId,
        state: &FileVatState,
        broker: &Broker,
        seam: Seam,
    ) -> Result<(), ProcessError> {
        // Idempotent within the Run: if the effect already succeeded, do not re-file —
        // but still *repair* the postcondition (open the payment gate / record the
        // no-payment completion) in case a crash landed between the two appends.
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect_succeeded(EFFECT_SUBMIT_FILING) {
            return self.finish_filing_postcondition(journal, &f, state);
        }
        // One guard owns every path that can dispatch the filing: initial approval,
        // any deliberate re-drive, and crash resume. It runs before EffectExecuting
        // and, most importantly, before the Broker can create an outbox attempt.
        self.assert_current_filing_proposal(&f, period, state)?;
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_SUBMIT_FILING.to_string(),
        })?;
        let env = rebuild_filing_env(period, &state.filing_body);
        // The outbox key is period-scoped so two Runs filing the *same* period
        // dedupe (never a double filing), while different periods stay independent.
        let outcome = broker
            .submit(ACTOR, &format!("{EFFECT_SUBMIT_FILING}:{period}"), &env)
            .map_err(map_broker)?;
        match outcome {
            RemoteOutcome::Delivered { receipt } => {
                // Crash seam: the authority accepted the return (the outbox holds a
                // terminal-success attempt) but we die before the fiscal COMMIT. The
                // effect is left Executing and `vat_filed` is still false — the exact
                // window `resume_file_vat` re-drives (the outbox replay meets no
                // double filing).
                if seam == Seam::CrashAfterFilingBrokerSuccess {
                    return Ok(());
                }
                // Record the filing as an audited, irreversible Ledger event bound to
                // the exact sent_hash that was approved and submitted. If a prior
                // (crashed) attempt already filed it, `file_vat` is idempotent for our
                // sent_hash — but we must not re-`acquire_filing_authority` (which
                // refuses an already-filed period), so guard on `vat_filed`.
                let receipt = FileVatReceipt::decode(&receipt).map_err(map_adapter)?;
                let ledger = Ledger::open(&self.home)?;
                if !ledger.period_view(period)?.vat_filed {
                    let auth = ledger.acquire_filing_authority(period, run_id)?;
                    ledger.file_vat_with_receipt(
                        period,
                        &auth,
                        &state.filing_sent_hash,
                        &receipt.unique_identifier,
                        &receipt.accepted_timestamp,
                        ACTOR,
                    )?;
                }
                // Crash seam: the fiscal COMMIT landed (`vat_filed` is now set) but we
                // die before journalling `EffectSucceeded`. Fiscal is authoritative;
                // `resume_file_vat` adopts it without re-hitting the authority.
                if seam == Seam::CrashAfterFilingFiscalCommit {
                    return Ok(());
                }
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SUBMIT_FILING.to_string(),
                    note: format!(
                        "filed (UniqueIdentifier {}, AcceptedTimestamp {}); period audited-filed",
                        receipt.unique_identifier, receipt.accepted_timestamp
                    ),
                })?;
                // Crash seam: the filing success is journalled but we die before the
                // payment-phase postcondition runs — the Run wedges `Running` with no
                // open gate until `resume_file_vat` converges the postcondition.
                if seam == Seam::CrashAfterFilingSuccess {
                    return Ok(());
                }
                let f2 = fold(run_id, &journal.read_all()?)?;
                self.finish_filing_postcondition(journal, &f2, state)
            }
            RemoteOutcome::RecoveredAlreadyProcessed { .. } => Err(ProcessError::Store(
                "VERO_UNSUPPORTED_RECOVERY: Vero filing has no 409/idempotency recovery contract"
                    .into(),
            )),
            RemoteOutcome::Unknown => {
                // The load-bearing M3 case: DO NOT retry blindly. Park NeedsAttention;
                // The outbox holds the attempt for status reconciliation. Never retry
                // the Vero filing merely because no definitive response arrived.
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SUBMIT_FILING.to_string(),
                    error_code: "OUTCOME_UNKNOWN".to_string(),
                })?;
                Ok(())
            }
            RemoteOutcome::Rejected { code, message } => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SUBMIT_FILING.to_string(),
                    error_code: format!("{code}: {message}"),
                })?;
                Ok(())
            }
        }
    }

    fn assert_current_filing_proposal(
        &self,
        f: &Fold,
        period: &PeriodId,
        state: &FileVatState,
    ) -> Result<(), ProcessError> {
        assert_filing_gate_approved_and_bound(
            f,
            &state.filing_body,
            &state.vero_period,
            &state.filing_sent_hash,
        )?;

        let ledger = Ledger::open(&self.home)?;
        let policy_set = PolicySet {
            id: state.policy_set.clone(),
        };
        let current = derive_verified_sealed_filing(&ledger, period, &policy_set)?;
        let current_fields = serde_json::to_value(&current.filing.fields)
            .map_err(|error| ProcessError::Store(error.to_string()))?;
        let unchanged = current.voucher_source_fingerprint == state.voucher_source_fingerprint
            && current_fields == state.filing_fields
            && current.filing.sales_vat.amount_str() == state.sales_vat
            && current.filing.purchase_vat.amount_str() == state.purchase_vat
            && current.filing.net_vat.amount_str() == state.net_vat;
        if !unchanged {
            return Err(ProcessError::Store(format!(
                "VAT_FILING_SOURCE_CHANGED: sealed vouchers or derived VAT fields for period \
                 {period} differ from the approved filing proposal; create a new preview"
            )));
        }
        // A Run started before the reconciliation existed was never checked. Guard a
        // first dispatch only, so an attempt already made still recovers.
        let first_dispatch = f
            .effect(EFFECT_SUBMIT_FILING)
            .is_none_or(|effect| effect.status == EffectStatus::Planned);
        if first_dispatch {
            require_reconciled(
                &current.reconciliation,
                state.vat_reconciliation_acknowledgement.as_ref(),
            )?;
        }
        Ok(())
    }

    /// The payment submit: enforce the net-invariant, then broker.submit; success
    /// completes the Run, `Unknown` parks `NeedsAttention`.
    fn run_submit_payment(
        &self,
        journal: &Journal,
        run_id: &str,
        period: &PeriodId,
        state: &FileVatState,
        broker: &Broker,
    ) -> Result<(), ProcessError> {
        let f = fold(run_id, &journal.read_all()?)?;
        if f.effect_succeeded(EFFECT_SUBMIT_PAYMENT) {
            return Ok(());
        }
        // Net-invariant: the payment amount must equal the reconciled bank payment
        // required — never a blind gross payment (PAYMENT_EXCEEDS_RECONCILED_AMOUNT).
        let amount = state
            .payment_body
            .get("amount")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if amount != state.bank_payment_required {
            journal.append(EventPayload::EffectRejected {
                effect_key: EFFECT_SUBMIT_PAYMENT.to_string(),
                error_code: "PAYMENT_EXCEEDS_RECONCILED_AMOUNT".to_string(),
            })?;
            return Err(ProcessError::InvalidAnswer(format!(
                "payment amount {amount} != reconciled bank_payment_required {} (period {period})",
                state.bank_payment_required
            )));
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_SUBMIT_PAYMENT.to_string(),
        })?;
        let env = rebuild_payment_env(period, &state.payment_body);
        let outcome = broker
            .submit(ACTOR, &format!("{EFFECT_SUBMIT_PAYMENT}:{period}"), &env)
            .map_err(map_broker)?;
        match outcome {
            RemoteOutcome::Delivered { receipt }
            | RemoteOutcome::RecoveredAlreadyProcessed { receipt } => {
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SUBMIT_PAYMENT.to_string(),
                    note: format!("paid (receipt {receipt})"),
                })?;
                Ok(())
            }
            RemoteOutcome::Unknown => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SUBMIT_PAYMENT.to_string(),
                    error_code: "OUTCOME_UNKNOWN".to_string(),
                })?;
                Ok(())
            }
            RemoteOutcome::Rejected { code, message } => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_SUBMIT_PAYMENT.to_string(),
                    error_code: format!("{code}: {message}"),
                })?;
                Ok(())
            }
        }
    }

    /// After the filing succeeds, make the payment-phase postcondition hold: if no
    /// bank payment is required (a refund or a fully-covered filing), record the
    /// payment effect as a no-op success so the Run completes without a spurious
    /// zero-value payment; otherwise open the `ApprovePayment` preview gate. Both
    /// branches are idempotent, so a crash-recovery re-entry repairs a missing
    /// postcondition rather than duplicating it.
    fn finish_filing_postcondition(
        &self,
        journal: &Journal,
        f: &Fold,
        state: &FileVatState,
    ) -> Result<(), ProcessError> {
        if is_zero_eur(&state.bank_payment_required) {
            if !f.effect_succeeded(EFFECT_SUBMIT_PAYMENT) {
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SUBMIT_PAYMENT.to_string(),
                    note:
                        "no bank payment required (refund or fully covered by tax-account credit)"
                            .to_string(),
                })?;
            }
        } else if f.gate(GATE_APPROVE_PAYMENT).is_none() {
            journal.append(EventPayload::GateOpened {
                gate_id: GATE_APPROVE_PAYMENT.to_string(),
                gate_kind: "Preview".to_string(),
                snapshot_hash: state.payment_sent_hash.clone(),
            })?;
        }
        Ok(())
    }
}

/// Read and fully verify the sealed ledger before deriving a filing, then bind the
/// proposal to both each period voucher's fiscal hash and its source fingerprint.
/// The explicit length framing keeps the digest unambiguous even for legacy source
/// identifiers containing separators.
fn derive_verified_sealed_filing(
    ledger: &Ledger,
    period: &PeriodId,
    policy_set: &PolicySet,
) -> Result<VerifiedFilingSnapshot, ProcessError> {
    let view = ledger.period_view(period)?;
    if view.state != PeriodState::Sealed {
        return Err(ProcessError::Ledger(
            aggountant_ledger::LedgerError::PeriodNotSealed(format!(
                "period {period} is {:?} — file-vat requires a Sealed period",
                view.state
            )),
        ));
    }
    ledger.verify()?;
    let sealed_vouchers = ledger.period_vouchers(period)?;
    let mut source_hash = Sha256::new();
    source_hash.update(b"aggountant:file-vat:voucher-source:v1");
    source_hash.update((sealed_vouchers.len() as u64).to_be_bytes());
    for voucher in &sealed_vouchers {
        if let Some(expansion) = voucher.vat_expansion() {
            if expansion.policy_set() != policy_set.id {
                return Err(map_domain(aggountant_domain::DomainError::VatStructure(
                    format!(
                        "voucher {} VAT expansion policy_set {:?} differs from filing policy_set {:?}",
                        voucher.number(), expansion.policy_set(), policy_set.id
                    ),
                )));
            }
        }
        let stored = ledger.voucher(voucher.number())?;
        hash_framed(&mut source_hash, stored.voucher_hash.as_bytes());
        match stored.provenance.source_fingerprint {
            Some(source) => {
                source_hash.update([1]);
                hash_framed(&mut source_hash, source.as_bytes());
            }
            None => source_hash.update([0]),
        }
    }
    let figures = ledger.derive_vat_filing_figures(period, &sealed_vouchers)?;
    let filing = compute_derived_vat_filing(&figures).map_err(map_domain)?;
    let reconciliation = reconcile_period(ledger, period, &sealed_vouchers, &figures)?;
    Ok(VerifiedFilingSnapshot {
        filing,
        voucher_source_fingerprint: hex::encode(source_hash.finalize()),
        reconciliation,
    })
}

fn hash_framed(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn validate_evidence_ref(reference: &str) -> Result<(), ProcessError> {
    let valid = !reference.is_empty()
        && reference.len() <= 256
        && reference.trim() == reference
        && reference
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'=' | b'&' | b'?' | b'#'));
    if !valid {
        return Err(ProcessError::InvalidAnswer(
            "INVALID_EVIDENCE_REF: use a 1..256 character non-secret reference without whitespace or query/fragment data".into(),
        ));
    }
    Ok(())
}

/// A canonical EUR zero (`amount_str` rescales to 2 dp, so zero is exactly "0.00").
fn is_zero_eur(amount: &str) -> bool {
    amount == "0.00"
}

/// Whether an effect may be re-driven on `resume` — i.e. it is mid-flight from a crash
/// (`Planned` = approved-then-crashed before dispatch; `Executing` = crashed during the
/// submit), not a *parked* terminal state. A `Rejected` effect (a genuine `Unknown` or a
/// remote rejection) is deliberately excluded: re-driving it would be the blind retry the
/// M3 invariant forbids — only the explicit `retry_file_vat_submit` may re-drive that.
fn filing_resume_safe(broker: &Broker, f: &Fold, period: &PeriodId) -> Result<bool, ProcessError> {
    match f.effect(EFFECT_SUBMIT_FILING).map(|effect| effect.status) {
        Some(EffectStatus::Planned) => Ok(true),
        Some(EffectStatus::Executing) => Ok(broker
            .outbox_entry(
                SVC_VERO_FILE_VAT_RETURN,
                &format!("{EFFECT_SUBMIT_FILING}:{period}"),
            )
            .map_err(map_broker)?
            .is_some_and(|entry| entry.terminal_success().is_some())),
        _ => Ok(false),
    }
}

fn crash_resumable(f: &Fold, effect_key: &str) -> bool {
    matches!(
        f.effect(effect_key).map(|e| e.status),
        Some(EffectStatus::Planned | EffectStatus::Executing)
    )
}

/// A shared dispatch precondition: the effect's gate must be *approved* and the
/// rebuilt envelope must still hash to the exact bytes the gate bound. Enforced
/// before every dispatch — the initial gate-answer and every deliberate retry — so
/// the approval always signs the bytes that actually go out.
fn assert_filing_gate_approved_and_bound(
    f: &Fold,
    body: &serde_json::Value,
    vero_period: &serde_json::Value,
    state_hash: &str,
) -> Result<(), ProcessError> {
    let gate = f
        .gate(GATE_APPROVE_FILING)
        .ok_or_else(|| ProcessError::GateNotOpen("gate ApproveFiling is not open".into()))?;
    if gate.answered.as_deref() != Some("approve") {
        return Err(ProcessError::GateNotOpen(
            "gate ApproveFiling is not approved — cannot dispatch its effect".into(),
        ));
    }
    let payload_hash = filing_payload_hash(body)?;
    let approval_hash = filing_approval_hash(body, vero_period)?;
    if payload_hash != state_hash || approval_hash != gate.snapshot_hash {
        return Err(ProcessError::JournalCorrupt(format!(
            "ApproveFiling: request bytes or Vero period snapshot changed after approval (bound {}, now {approval_hash})",
            gate.snapshot_hash
        )));
    }
    Ok(())
}

fn assert_gate_approved_and_bound(
    f: &Fold,
    gate_id: &str,
    env: &Envelope,
    state_hash: &str,
) -> Result<(), ProcessError> {
    let gate: &GateView = f
        .gate(gate_id)
        .ok_or_else(|| ProcessError::GateNotOpen(format!("gate {gate_id} is not open")))?;
    if gate.answered.as_deref() != Some("approve") {
        return Err(ProcessError::GateNotOpen(format!(
            "gate {gate_id} is not approved — cannot dispatch its effect"
        )));
    }
    let recomputed = env.sent_hash().map_err(map_broker)?;
    if recomputed != state_hash || recomputed != gate.snapshot_hash {
        return Err(ProcessError::JournalCorrupt(format!(
            "{gate_id}: the payload changed after approval was bound (bound {}, now {recomputed}) \
             — refusing to send unapproved bytes",
            gate.snapshot_hash
        )));
    }
    Ok(())
}

/// The approved FileVATReturn/v2 request, parsed back from the bytes the
/// approval gate bound.
fn approved_filing_request(state: &FileVatState) -> Result<FileVatReturn, ProcessError> {
    serde_json::from_str(
        state.filing_body["request_bytes"]
            .as_str()
            .unwrap_or_default(),
    )
    .map_err(|e| ProcessError::JournalCorrupt(format!("bad FileVATReturn state: {e}")))
}

/// The GetFiledVATReturn/v2 read for the period and organisation of an approved
/// request. A read only: it is fetched, never submitted through the outbox.
fn filed_vat_return_envelope(
    expected: &FileVatReturn,
    natural_key: String,
) -> Result<Envelope, ProcessError> {
    let request = serde_json::to_string(&json!({
        "BusinessId": expected.business_id,
        "FilingPeriod": expected.filing_period,
    }))
    .map_err(|e| ProcessError::Store(e.to_string()))?;
    Ok(Envelope::new(
        SVC_VERO_FILED_VAT_RETURN,
        "GetFiledVATReturn/v2",
        natural_key,
        json!({ "request_bytes": request }),
    ))
}

fn rebuild_filing_env(period: &PeriodId, body: &serde_json::Value) -> Envelope {
    Envelope::new(
        SVC_VERO_FILE_VAT_RETURN,
        "FileVATReturn/v2",
        format!("file:{period}"),
        body.clone(),
    )
}

fn period_dates(period: &PeriodId) -> Result<(String, String), ProcessError> {
    let (first_month, last_month) = period.month_bounds();
    let start = format!("{:04}-{first_month:02}-01", period.year());
    let (next_year, next_month) = if last_month == 12 {
        (period.year() + 1, 1)
    } else {
        (period.year(), u32::from(last_month) + 1)
    };
    let next = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .ok_or_else(|| ProcessError::Store("invalid VAT period date".into()))?;
    let end = next
        .pred_opt()
        .ok_or_else(|| ProcessError::Store("invalid VAT period end date".into()))?;
    Ok((start, end.format("%Y-%m-%d").to_string()))
}

fn filing_approval_hash(
    body: &serde_json::Value,
    vero_period: &serde_json::Value,
) -> Result<String, ProcessError> {
    let snapshot = serde_json::to_vec(&json!({
        "file_vat_return": body.get("request_bytes"),
        "vero_period": vero_period,
    }))
    .map_err(|error| ProcessError::Store(error.to_string()))?;
    let mut hash = Sha256::new();
    hash.update(snapshot);
    Ok(hex::encode(hash.finalize()))
}

fn filing_payload_hash(body: &serde_json::Value) -> Result<String, ProcessError> {
    let bytes = body
        .get("request_bytes")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ProcessError::JournalCorrupt("file-vat state lacks request_bytes".into()))?;
    let mut hash = Sha256::new();
    hash.update(bytes.as_bytes());
    Ok(hex::encode(hash.finalize()))
}

fn rebuild_payment_env(period: &PeriodId, body: &serde_json::Value) -> Envelope {
    Envelope::new(
        SVC_PANKKI_MAKSU,
        "pay-vat",
        format!("pay:{period}"),
        body.clone(),
    )
}

fn map_domain(e: aggountant_domain::DomainError) -> ProcessError {
    ProcessError::Ledger(aggountant_ledger::LedgerError::Model(match e {
        aggountant_domain::DomainError::Model(m) => m,
        other => {
            // A non-model domain error (e.g. VAT_CURRENCY) has no LedgerError peer;
            // surface it as a JournalCorrupt-adjacent Store error with its code.
            return ProcessError::Store(format!("{}: {other}", other.error_code()));
        }
    }))
}

fn map_broker(e: aggountant_broker::BrokerError) -> ProcessError {
    ProcessError::Store(format!("{}: {e}", e.error_code()))
}

fn map_adapter(e: aggountant_broker::AdapterError) -> ProcessError {
    ProcessError::Store(e.to_string())
}
