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
    GetVatPeriodsResponse, RemoteOutcome, VatReplacementReason, VatValueDifference,
    SVC_PANKKI_MAKSU, SVC_VERO_FILED_VAT_RETURN, SVC_VERO_FILE_VAT_RETURN, SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::{compute_derived_vat_filing, PolicySet, VatFiling};
use aggountant_ledger::{
    Ledger, ManualVatFilingEvidence, PeriodState, PeriodView, VatFilingEvidence, VatFilingRecord,
};
use aggountant_model::{BusinessId, FilingContactName, FilingContactPhone, Money, PeriodId};
use sha2::{Digest, Sha256};

use crate::journal::{EventPayload, Journal};
use crate::vat_reconciliation::{
    attention_items, reconcile_period, require_reconciled, VatAttentionItem,
    VatReconciliationAcknowledgement,
};
use crate::{fold, EffectStatus, Fold, GateView, ProcessEngine, ProcessError, RunView, Seam};

mod recovery;
mod status;
#[cfg(test)]
mod tests;
pub use recovery::{AuthorityCheck, FileVatReconcile, FileVatReconcileOutcome};
pub use status::{FileVatFilingStatus, FileVatStatus, FileVatStatusPlan, FiledVatQuery};

/// Effect keys for the `file-vat` Run (stable idempotency keys).
const EFFECT_RECONCILE: &str = "reconcile";
pub(crate) const EFFECT_SUBMIT_FILING: &str = "submit-filing";
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
    /// Why the filed return is corrected. Only for `file-vat replace`, which
    /// requires it; `file-vat start` refuses an input that carries it.
    #[serde(default)]
    pub replacement_reasons: Option<Vec<VatReplacementReason>>,
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
    /// Present when the Run files a replacement return for a filed period.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replacement: Option<ReplacementState>,
}

/// What a replacement Run was prepared against.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplacementState {
    /// The number of filings recorded for the period when the Run started (the
    /// first return and every replacement so far). The Run's own filing is the
    /// next one; the Ledger refuses it if another was recorded in between.
    replaces_filings: usize,
    reasons: Vec<VatReplacementReason>,
    /// GetFiledVATReturn's answer before anything was sent: the processed return
    /// this Run replaces. An identical answer after sending means the authority
    /// does not show the replacement.
    prior_filed_return: FiledVatReturn,
}

impl FileVatState {
    /// How many filings the period had when this Run started: 0 for a first
    /// return. The Run's filing has reached the books once there are more.
    fn replaces_filings(&self) -> usize {
        self.replacement
            .as_ref()
            .map_or(0, |replacement| replacement.replaces_filings)
    }

    /// The Run's FileVATReturn outbox key. A first return is keyed by the period,
    /// so two Runs filing it dedupe. A replacement is a different payload and gets
    /// its own key (the filing it replaces and the request it sends), so the
    /// first return's recorded outcome is never replayed as the replacement's and
    /// a rejected replacement can be followed by a corrected one. What keeps two
    /// replacements from both being sent is not the key: an open outcome blocks
    /// every new dispatch, and the Ledger refuses a replacement of a filing that
    /// has already been replaced.
    fn filing_outbox_key(&self) -> String {
        match self.replaces_filings() {
            0 => format!("{EFFECT_SUBMIT_FILING}:{}", self.period),
            n => format!(
                "{EFFECT_SUBMIT_FILING}:{}:replaces-{n}:{}",
                self.period,
                self.filing_sent_hash.get(..16).unwrap_or_default()
            ),
        }
    }
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
    /// Present for `file-vat replace`: what the approval replaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<ReplacementPreview>,
}

/// What a replacement return replaces, shown before the approval.
#[derive(Debug, Clone, Serialize)]
pub struct ReplacementPreview {
    /// The filing being replaced: 1 is the first return.
    pub replaces_filing: usize,
    pub reasons: Vec<VatReplacementReason>,
    /// The return the Tax Administration shows now (GetFiledVATReturn/v2).
    pub authority_return: FiledVatReturn,
    /// Every amount the replacement changes: `approved` is the replacement's
    /// value, `authority` the value in force now.
    pub changes: Vec<VatValueDifference>,
    pub payment_note: &'static str,
}

const REPLACEMENT_PAYMENT_NOTE: &str =
    "A replacement return changes the tax of the period by the difference to the return in \
     force. This Run does not pay: check the tax account in OmaVero for the amount due or \
     refunded, and pay a difference separately.";

/// Read-only offline filing view. The numbered fields are the same Domain projection
/// used by direct Vero filing; this type adds only operator-facing instructions, which
/// name both filing routes because the preview does not evaluate Vero.fi readiness.
#[derive(Debug, Clone, Serialize)]
pub struct OfflineVatPreview {
    pub period: String,
    /// The period's state when the preview was derived. Before `sealed` the
    /// figures and the reconciliation are a draft of the coming return.
    pub period_state: PeriodState,
    /// Whether the period is in a fileable state: only a sealed period is filed,
    /// so an open or closing period's preview is for checking, not for filing.
    /// Whether the figures themselves need action first is in `attention`.
    pub fileable: bool,
    pub filing_fields: aggountant_domain::VatReturnFields,
    pub sales_vat: String,
    pub purchase_vat: String,
    pub net_vat: String,
    pub payment_amount: String,
    pub vat_reconciliation: aggountant_domain::VatAccountReconciliation,
    pub attention: Vec<VatAttentionItem>,
    /// Every filing recorded for the period, oldest first; the last is in force.
    pub filings: Vec<VatFilingRecord>,
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
        let period_state = ledger.period_view(period)?.state;
        let verified = derive_verified_filing(&ledger, period, policy_set)?;
        let filings = ledger.vat_filings(period)?;
        // A filed period is fileable again only as a replacement return.
        let fileable = period_state == PeriodState::Sealed;
        let filing = verified.filing;
        let mut attention = attention_items(&verified.reconciliation, period_state);
        let unfiled_corrections = self.unfiled_vat_corrections(&ledger, period)?;
        if !unfiled_corrections.is_empty() {
            attention.push(VatAttentionItem {
                code: "VAT_REPLACEMENT_DUE",
                severity: "info",
                message: format!(
                    "These figures include correction voucher(s) sealed for {period} after \
                     its latest filing, so the return in force is not these figures. File a \
                     replacement return with `file-vat replace`."
                ),
                vouchers: unfiled_corrections,
            });
        }
        let corrects_elsewhere: Vec<u64> = ledger
            .period_vouchers(period)?
            .iter()
            .filter(|voucher| voucher.corrects_vat_period().is_some())
            .map(|voucher| voucher.number())
            .collect();
        if !corrects_elsewhere.is_empty() {
            attention.push(VatAttentionItem {
                code: "VAT_CORRECTIONS_OF_OTHER_PERIODS",
                severity: "info",
                message: format!(
                    "Voucher(s) posted in {period} correct the filed VAT return of an earlier \
                     period (`corrects_vat_period`). Their VAT is left out of these figures \
                     and belongs to that period's replacement return."
                ),
                vouchers: corrects_elsewhere,
            });
        }
        let payment_amount = if filing.net_vat.amount().is_sign_negative() {
            "0.00".to_string()
        } else {
            filing.net_vat.amount_str()
        };
        const NOT_YET_PAYABLE: &str =
            "Nothing to pay yet: the amount is known once the closed period is previewed.";
        let (filing_instruction, payment_instruction) = match period_state {
            PeriodState::Sealed if !filings.is_empty() => (
                "The return of this period is filed. Changed figures are filed as a replacement return with `file-vat replace` (all values again, with a reason); a replacement made in OmaVero is recorded with `file-vat complete-manual --replacement`.",
                "A replacement changes the tax by the difference to the return in force; check the tax account in OmaVero for what is due or refunded.",
            ),
            PeriodState::Sealed => (
                "Two routes; this preview is offline and did not check which one is available. Direct filing through the Vero.fi API: when `aggountant vero-fi status` reports ready, start the filing with `file-vat start` using these figures and approve it at the `ApproveFiling` gate (`file-vat approve-filing`). Manual filing otherwise: enter these numbered fields in OmaVero (or send an Ilmoitin.fi file you prepared yourself; aggountant does not create one) and record it with `file-vat complete-manual`.",
                "Pay separately using the current payment details and reference shown in OmaVero; aggountant does not initiate the payment.",
            ),
            PeriodState::Open => (
                "Not fileable: the period is not closed, so these are draft figures. Correct any finding with a voucher in this period, close it (`run start close-period`), and preview again before filing.",
                NOT_YET_PAYABLE,
            ),
            PeriodState::Closing => (
                "Not fileable: the period is closing, so these are draft figures. If a finding needs a correcting voucher, reject the close (`run answer --gate ApproveClose --answer reject`) to reopen the period first; otherwise approve the close, and preview again before filing.",
                NOT_YET_PAYABLE,
            ),
        };
        Ok(OfflineVatPreview {
            period: period.to_string(),
            period_state,
            fileable,
            filing_fields: filing.fields,
            sales_vat: filing.sales_vat.amount_str(),
            purchase_vat: filing.purchase_vat.amount_str(),
            net_vat: filing.net_vat.amount_str(),
            payment_amount,
            attention,
            vat_reconciliation: verified.reconciliation,
            filings,
            filing_instruction,
            payment_instruction,
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
        self.record_manual_vat(
            period,
            channel,
            evidence_ref,
            remote_status_evidence_ref,
            false,
        )
    }

    /// As [`Self::complete_manual_vat`], for a replacement the operator made of
    /// an already filed return: recorded as a further filing of the period, and
    /// refused for a period that is not filed, as a first filing is for one that
    /// is.
    pub fn complete_manual_vat_replacement(
        &self,
        period: &PeriodId,
        channel: &str,
        evidence_ref: &str,
        remote_status_evidence_ref: Option<&str>,
    ) -> Result<ManualVatCompletion, ProcessError> {
        self.record_manual_vat(
            period,
            channel,
            evidence_ref,
            remote_status_evidence_ref,
            true,
        )
    }

    fn record_manual_vat(
        &self,
        period: &PeriodId,
        channel: &str,
        evidence_ref: &str,
        remote_status_evidence_ref: Option<&str>,
        replacement: bool,
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
        if self.period_has_unresolved_filing(period)? && remote_status_evidence_ref.is_none() {
            return Err(ProcessError::InvalidAnswer(
                "OUTCOME_UNKNOWN: manual filing is refused until remote status resolution evidence is supplied".into(),
            ));
        }
        let ledger = Ledger::open(&self.home)?;
        const MANUAL_ACTOR: &str = "process:file-vat-manual";
        let period_view = if replacement {
            let replaces = ledger.vat_filings(period)?.len();
            let authority =
                ledger.acquire_replacement_authority(period, "manual-file-vat", replaces)?;
            ledger.replace_vat_filing(
                period,
                &authority,
                VatFilingEvidence::Manual {
                    channel,
                    evidence_ref,
                    remote_status_evidence_ref,
                },
                MANUAL_ACTOR,
            )?
        } else {
            let authority = ledger.acquire_filing_authority(period, "manual-file-vat")?;
            ledger.file_vat_manually(
                period,
                &authority,
                channel,
                evidence_ref,
                remote_status_evidence_ref,
                MANUAL_ACTOR,
            )?
        };
        let manual_filing = ledger.manual_vat_filing_evidence(period)?.ok_or_else(|| {
            ProcessError::Store("manual VAT filing evidence missing after commit".into())
        })?;
        Ok(ManualVatCompletion {
            period: period_view,
            manual_filing,
        })
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
        if input.replacement_reasons.is_some() {
            return Err(ProcessError::InvalidAnswer(
                "VAT_REPLACEMENT_NOT_A_FIRST_FILING: `replacement_reasons` belongs to a \
                 replacement return; start it with `file-vat replace`"
                    .into(),
            ));
        }
        // A filed period is refused before a Run exists and before its figures are
        // compared: whatever the input says, the answer is a replacement return.
        if Ledger::open(&self.home)?
            .period_view(&input.period)?
            .vat_filed
        {
            return Err(ProcessError::Ledger(
                aggountant_ledger::LedgerError::PeriodTransitionConflict(format!(
                    "period {} is already filed — filing is irreversible; a filed return is \
                     corrected with a replacement return (`file-vat replace`)",
                    input.period
                )),
            ));
        }
        self.start_filing(input, None, broker, policy_set)
    }

    /// Start a `file-vat` Run that replaces the filed return of a period: the
    /// whole return again from the sealed vouchers (the period's own and the later
    /// correction vouchers sealed for it), marked as a replacement with its
    /// reasons. Same gates, outbox and recovery as a first return. Refused for a
    /// period that is not filed, while the Tax Administration has not processed
    /// the return in force, and when it already shows these values.
    pub fn start_file_vat_replacement(
        &self,
        input: &FileVatInput,
        broker: &Broker,
        policy_set: &PolicySet,
    ) -> Result<FileVatStart, ProcessError> {
        let Some(reasons) = input.replacement_reasons.as_deref() else {
            return Err(ProcessError::InvalidAnswer(
                "VAT_REPLACEMENT_REASON_REQUIRED: a replacement return needs \
                 `replacement_reasons` (calculation-error, legal-practice, \
                 tax-audit-guidance, law-interpretation)"
                    .into(),
            ));
        };
        // Refused before a Run exists: a period without a filed return has nothing
        // to replace, and a replacement Run for it could never be approved.
        let replaces_filings = Ledger::open(&self.home)?.vat_filings(&input.period)?.len();
        if replaces_filings == 0 {
            return Err(ProcessError::Ledger(
                aggountant_ledger::LedgerError::VatPeriodNotFiled(format!(
                    "period {} has no VAT return filed, so there is nothing to replace; file \
                     the first return with `file-vat start`",
                    input.period
                )),
            ));
        }
        self.start_filing(input, Some((replaces_filings, reasons)), broker, policy_set)
    }

    fn start_filing(
        &self,
        input: &FileVatInput,
        replacement: Option<(usize, &[VatReplacementReason])>,
        broker: &Broker,
        policy_set: &PolicySet,
    ) -> Result<FileVatStart, ProcessError> {
        // Refused before a Run exists, so an unresolved earlier filing leaves no
        // second Run waiting for an approval it could never use.
        self.refuse_while_filing_unresolved(&input.period, None)?;
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
        // A failed or refused read ends the reconcile effect terminally (the Run
        // shows NeedsAttention in `run show`); nothing retries it automatically.
        let vero_period = match read_fileable_vero_period(input, broker, replacement.is_some()) {
            Ok(vero_period) => vero_period,
            Err(PreflightFailure { code, detail }) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_RECONCILE.to_string(),
                    error_code: code.to_string(),
                })?;
                return Err(ProcessError::Store(format!(
                    "{code}: {detail}; Run {run_id} needs attention, nothing was sent \
                     — start a new file-vat Run when the cause is fixed"
                )));
            }
        };

        // The sole projection builds the final API request. Persist its exact bytes;
        // approval and dispatch both bind to this same string. A projection the
        // adapter refuses (e.g. `VERO_VAT_CROSS_FIELD`) ends the reconcile effect
        // like a failed preflight, so the Run does not stay `executing`.
        let projection = (|| {
            let request = project_file_vat_return(
                &input.period,
                &filing.fields,
                &input.business_id,
                &input.contact_name,
                &input.contact_phone,
            )
            .map_err(map_adapter)?;
            let request = match replacement {
                Some((_, reasons)) => request.into_replacement(reasons).map_err(map_adapter)?,
                None => request,
            };
            let request_bytes = request.bytes().map_err(map_adapter)?;
            let request_string = String::from_utf8(request_bytes).map_err(|e| {
                ProcessError::Store(format!("FileVATReturn bytes are not UTF-8: {e}"))
            })?;
            let filing_sent_hash = request.sent_hash().map_err(map_adapter)?;
            let filing_body = json!({ "request_bytes": request_string });
            let filing_approval_hash = filing_approval_hash(&filing_body, &vero_period)?;
            Ok::<_, ProcessError>((request, filing_body, filing_sent_hash, filing_approval_hash))
        })();
        let (request, filing_body, filing_sent_hash, filing_approval_hash) = match projection {
            Ok(projection) => projection,
            Err(error) => {
                journal.append(EventPayload::EffectRejected {
                    effect_key: EFFECT_RECONCILE.to_string(),
                    error_code: error.error_code().to_string(),
                })?;
                return Err(error);
            }
        };

        // A replacement is prepared against the return the authority shows now.
        let replacement = match replacement {
            None => None,
            Some((replaces_filings, reasons)) => {
                match read_replaced_return(&request, &input.period, broker) {
                    Ok((prior_filed_return, changes)) => Some((
                        ReplacementState {
                            replaces_filings,
                            reasons: reasons.to_vec(),
                            prior_filed_return,
                        },
                        changes,
                    )),
                    Err(PreflightFailure { code, detail }) => {
                        journal.append(EventPayload::EffectRejected {
                            effect_key: EFFECT_RECONCILE.to_string(),
                            error_code: code.to_string(),
                        })?;
                        return Err(ProcessError::Store(format!(
                            "{code}: {detail}; Run {run_id} needs attention, nothing was sent"
                        )));
                    }
                }
            }
        };

        // Payment remains a separate approval/effect. GetVATPeriods does not expose
        // tax-account credit, so this slice displays the filing net without pretending
        // it has reconciled a bank balance. A replacement changes the tax by its
        // difference to the return in force, which this Run does not know as a
        // payable amount (the earlier return may be paid, or not), so it pays nothing.
        let bank_payment_required =
            if replacement.is_some() || filing.net_vat.amount().is_sign_negative() {
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
        let authority = match &replacement {
            None => ledger
                .acquire_filing_authority(&input.period, &run_id)
                .map(drop),
            Some((state, _)) => ledger
                .acquire_replacement_authority(&input.period, &run_id, state.replaces_filings)
                .map(drop),
        };
        if let Err(error) = authority {
            journal.append(EventPayload::EffectRejected {
                effect_key: EFFECT_RECONCILE.to_string(),
                error_code: error.error_code().to_string(),
            })?;
            return Err(error.into());
        }
        let replacement_preview = replacement
            .as_ref()
            .map(|(state, changes)| ReplacementPreview {
                replaces_filing: state.replaces_filings,
                reasons: state.reasons.clone(),
                authority_return: state.prior_filed_return.clone(),
                changes: changes.clone(),
                payment_note: REPLACEMENT_PAYMENT_NOTE,
            });

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
                replacement: replacement.map(|(state, _)| state),
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
            replacement: replacement_preview,
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

        // Checked before the approval is recorded, so a refused Run can still be
        // approved once the earlier filing is resolved. The dispatch checks again.
        if answer == "approve" && gate_id == GATE_APPROVE_FILING {
            self.refuse_while_filing_unresolved(&period, Some(run_id))?;
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

        self.show(run_id)
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
        self.show(run_id)
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
        let landed = filing_landed(&ledger, &period, &state)?;

        // --- Reconcile the filing ---
        // A filing closed as not filed (`file-vat reconcile`) is finished: the Run
        // neither sends it nor adopts a filing a later Run made.
        let closed = f.effect(EFFECT_SUBMIT_FILING).map(|effect| effect.status)
            == Some(EffectStatus::Skipped);
        if !closed && !f.effect_succeeded(EFFECT_SUBMIT_FILING) {
            if landed && f.gate_approved(GATE_APPROVE_FILING) {
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
            } else if !landed
                && f.gate_approved(GATE_APPROVE_FILING)
                && filing_resume_safe(broker, &f, &state)?
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

        self.show(run_id)
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
        if f.effect(EFFECT_SUBMIT_FILING).map(|effect| effect.status) == Some(EffectStatus::Skipped)
        {
            return Err(ProcessError::VatFilingUnresolved(format!(
                "Run {run_id} was closed as not filed; start a new file-vat Run for {period}"
            )));
        }
        self.assert_current_filing_proposal(&f, period, state)?;
        self.refuse_while_filing_unresolved(period, Some(run_id))?;
        let env = rebuild_filing_env(period, &state.filing_body);
        // The outbox key is period-scoped so two Runs filing the *same* period
        // dedupe (never a double filing), while different periods stay independent.
        let outbox_key = state.filing_outbox_key();
        if f.effect(EFFECT_SUBMIT_FILING)
            .is_none_or(|effect| effect.status == EffectStatus::Planned)
        {
            // A first dispatch needs a period that is still unfiled: a manual filing
            // or another Run may have filed it since this Run started.
            let ledger = Ledger::open(&self.home)?;
            match &state.replacement {
                None => drop(ledger.acquire_filing_authority(period, run_id)?),
                // A replacement needs the period exactly as it was previewed: the
                // filing it replaces must still be the latest.
                Some(replacement) => drop(ledger.acquire_replacement_authority(
                    period,
                    run_id,
                    replacement.replaces_filings,
                )?),
            }
            // The Broker binds every attempt under the period's key to the bytes of
            // the first. A Run re-filing after an earlier attempt was closed as not
            // filed can only send the same bytes; refuse here rather than leave the
            // effect executing behind a refused submit.
            let earlier = broker
                .outbox_entry(SVC_VERO_FILE_VAT_RETURN, &outbox_key)
                .map_err(map_broker)?
                .and_then(|entry| entry.attempts.first().map(|a| a.sent_hash.clone()));
            let sent_hash = env.sent_hash().map_err(map_broker)?;
            if earlier.is_some_and(|hash| hash != sent_hash) {
                return Err(ProcessError::VatFilingUnresolved(format!(
                    "an earlier filing attempt for {period} sent a different FileVATReturn \
                     request, and aggountant can re-send a period only with the same request. \
                     File this return manually (`file-vat complete-manual`, citing the \
                     recorded \"not found\" answers as the remote status evidence)"
                )));
            }
        }
        journal.append(EventPayload::EffectExecuting {
            effect_key: EFFECT_SUBMIT_FILING.to_string(),
        })?;
        if seam == Seam::CrashBeforeFilingDispatch {
            return Ok(());
        }
        let outcome = broker
            .submit(ACTOR, &outbox_key, &env)
            .map_err(map_broker)?;
        if seam == Seam::CrashAfterFilingDispatch {
            return Ok(());
        }
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
                self.record_filing(period, run_id, state, Some(&receipt))?;
                // Crash seam: the fiscal COMMIT landed (`vat_filed` is now set) but we
                // die before journalling `EffectSucceeded`. Fiscal is authoritative;
                // `resume_file_vat` adopts it without re-hitting the authority.
                if seam == Seam::CrashAfterFilingFiscalCommit {
                    return Ok(());
                }
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SUBMIT_FILING.to_string(),
                    note: receipt_note(&receipt),
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

    /// Record the Run's filing as the audited, irreversible Ledger event bound to
    /// the approved bytes: with the authority's receipt, or without one when
    /// `file-vat reconcile` matched the processed return (that read returns no
    /// receipt fields and none are invented). Idempotent: a filing that already
    /// reached the books (a crashed earlier attempt of this Run) is left as it is.
    fn record_filing(
        &self,
        period: &PeriodId,
        run_id: &str,
        state: &FileVatState,
        receipt: Option<&FileVatReceipt>,
    ) -> Result<(), ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        if filing_landed(&ledger, period, state)? {
            return Ok(());
        }
        let sent_hash = &state.filing_sent_hash;
        match &state.replacement {
            None => {
                let auth = ledger.acquire_filing_authority(period, run_id)?;
                match receipt {
                    Some(receipt) => ledger.file_vat_with_receipt(
                        period,
                        &auth,
                        sent_hash,
                        &receipt.unique_identifier,
                        &receipt.accepted_timestamp,
                        ACTOR,
                    )?,
                    None => ledger.file_vat(period, &auth, sent_hash, ACTOR)?,
                };
            }
            Some(replacement) => {
                let auth = ledger.acquire_replacement_authority(
                    period,
                    run_id,
                    replacement.replaces_filings,
                )?;
                let evidence = match receipt {
                    Some(receipt) => VatFilingEvidence::Receipt {
                        sent_hash,
                        unique_identifier: &receipt.unique_identifier,
                        accepted_timestamp: &receipt.accepted_timestamp,
                    },
                    None => VatFilingEvidence::Reconciled { sent_hash },
                };
                ledger.replace_vat_filing(period, &auth, evidence, ACTOR)?;
            }
        }
        Ok(())
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
                    note: match state.replacement {
                        Some(_) => {
                            "replacement return: this Run pays nothing; the difference \
                                    to the replaced return is settled on the tax account"
                        }
                        None => {
                            "no bank payment required (refund or fully covered by \
                                 tax-account credit)"
                        }
                    }
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

/// Whether the Run's filing has reached the books: the period has more filings
/// than when the Run started. For a first return that is `vat_filed`.
fn filing_landed(
    ledger: &Ledger,
    period: &PeriodId,
    state: &FileVatState,
) -> Result<bool, ProcessError> {
    Ok(ledger.vat_filings(period)?.len() > state.replaces_filings())
}

/// Refuse a period that is not sealed, then derive its filing: only a sealed
/// period's figures are filed.
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
    derive_verified_filing(ledger, period, policy_set)
}

/// Read and fully verify the ledger before deriving a period's filing from its
/// sealed vouchers, then bind the proposal to both each period voucher's fiscal
/// hash and its source fingerprint. The explicit length framing keeps the digest
/// unambiguous even for legacy source identifiers containing separators. The
/// period state is the caller's concern: the preview reads an open period too.
fn derive_verified_filing(
    ledger: &Ledger,
    period: &PeriodId,
    policy_set: &PolicySet,
) -> Result<VerifiedFilingSnapshot, ProcessError> {
    ledger.verify()?;
    let sealed_vouchers = ledger.vat_period_vouchers(period)?;
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

/// Whether a journalled filing rejection leaves open that the authority accepted
/// the return. `OUTCOME_UNKNOWN` is the code a Run journals today for a timeout,
/// a 5xx or a 2xx without a readable receipt. Runs journalled before a 2xx was
/// classified as unknown carry `MALFORMED: …` or `HTTP_2xx: …` for that answer,
/// so those count too. `MALFORMED` is still journalled today for a 400 whose body
/// is not a structured error and for a request that could not be built; the
/// prefix cannot tell those from an older receiptless 200, so they are treated as
/// possibly accepted as well, which only asks for remote-status evidence. Every
/// other code is a rejection the authority stated. Both `file-vat status` and the
/// `complete-manual` guard read a filing through this.
pub(crate) fn filing_outcome_may_be_accepted(code: &str) -> bool {
    code == "OUTCOME_UNKNOWN" || code.starts_with("MALFORMED") || code.starts_with("HTTP_2")
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

/// The journal note of a filing accepted with a receipt.
fn receipt_note(receipt: &FileVatReceipt) -> String {
    format!(
        "filed (UniqueIdentifier {}, AcceptedTimestamp {}); period audited-filed",
        receipt.unique_identifier, receipt.accepted_timestamp
    )
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
fn filing_resume_safe(
    broker: &Broker,
    f: &Fold,
    state: &FileVatState,
) -> Result<bool, ProcessError> {
    match f.effect(EFFECT_SUBMIT_FILING).map(|effect| effect.status) {
        Some(EffectStatus::Planned) => Ok(true),
        Some(EffectStatus::Executing) => Ok(broker
            .outbox_entry(SVC_VERO_FILE_VAT_RETURN, &state.filing_outbox_key())
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

/// Why the ADR-9 preflight could not offer the period for filing. `code` is
/// journalled on the rejected reconcile effect; `detail` goes to the caller.
struct PreflightFailure {
    code: &'static str,
    detail: String,
}

/// Read the authority's period list and return the exact period of `input`, only
/// if the Tax Administration still treats it as fileable: `Unfiled`/`Missing`
/// for a first return, and for a replacement any status but `Expired` (outside
/// the adjustment period), since the return in force is read separately.
fn read_fileable_vero_period(
    input: &FileVatInput,
    broker: &Broker,
    replacement: bool,
) -> Result<serde_json::Value, PreflightFailure> {
    let local = |detail: String| PreflightFailure {
        code: "PROCESS_STORE",
        detail,
    };
    let periods_request = serde_json::to_string(&json!({
        "BusinessId": input.business_id.as_str(),
        "FilingYear": input.period.year(),
    }))
    .map_err(|e| local(e.to_string()))?;
    let periods_env = Envelope::new(
        SVC_VERO_VAT_PERIODS,
        "GetVATPeriods/v1",
        format!("periods:{}", input.period),
        json!({ "request_bytes": periods_request }),
    );
    let periods_body = broker.fetch(&periods_env).map_err(|e| PreflightFailure {
        code: e.error_code(),
        detail: format!("GetVATPeriods read failed: {e}"),
    })?;
    let periods: GetVatPeriodsResponse =
        serde_json::from_value(periods_body).map_err(|e| PreflightFailure {
            code: "VERO_VAT_PERIODS_MALFORMED",
            detail: format!("GetVATPeriods response malformed: {e}"),
        })?;
    let (period_start, period_end) = period_dates(&input.period);
    let vero_period = periods
        .vat_periods
        .into_iter()
        .find(|period| period.start_date == period_start && period.end_date == period_end)
        .ok_or_else(|| PreflightFailure {
            code: "VERO_VAT_PERIOD_MISSING",
            detail: format!("authority did not offer period {period_start}..{period_end}"),
        })?;
    let fileable = match (replacement, vero_period.status.as_str()) {
        (false, status) => matches!(status, "Unfiled" | "Missing"),
        (true, status) => status != "Expired",
    };
    if !fileable {
        return Err(PreflightFailure {
            code: "VERO_VAT_PERIOD_NOT_FILEABLE",
            detail: format!(
                "period {} has remote status {}",
                vero_period.period, vero_period.status
            ),
        });
    }
    serde_json::to_value(vero_period).map_err(|e| local(e.to_string()))
}

/// Read the return the Tax Administration holds for the period a replacement is
/// about to replace, and how the replacement changes it. A replacement is offered
/// only against a processed return: while the return in force is still being
/// processed, a later answer could not be told from it. A replacement that changes
/// no amount is refused, because the authority then already shows these values and
/// a later match would prove nothing about this filing.
fn read_replaced_return(
    request: &FileVatReturn,
    period: &PeriodId,
    broker: &Broker,
) -> Result<(FiledVatReturn, Vec<VatValueDifference>), PreflightFailure> {
    let envelope = filed_vat_return_envelope(request, format!("replace-preflight:{period}"))
        .map_err(|e| PreflightFailure {
            code: "PROCESS_STORE",
            detail: e.to_string(),
        })?;
    let body = broker.fetch(&envelope).map_err(|e| PreflightFailure {
        code: e.error_code(),
        detail: format!("GetFiledVATReturn read failed: {e}"),
    })?;
    let filed: FiledVatReturn = serde_json::from_value(body).map_err(|e| PreflightFailure {
        code: "VERO_FILED_VAT_RETURN_MALFORMED",
        detail: format!("GetFiledVATReturn response malformed: {e}"),
    })?;
    if filed.status.as_deref() != Some("Processed") {
        return Err(PreflightFailure {
            code: "VERO_VAT_RETURN_NOT_PROCESSED",
            detail: format!(
                "the Tax Administration answers with status {} for {period}; a replacement is \
                 sent only once the return in force is processed, so try again later",
                filed.status.as_deref().unwrap_or("(none)")
            ),
        });
    }
    let changes = filed.value_differences(request);
    if changes.is_empty() {
        return Err(PreflightFailure {
            code: "VAT_REPLACEMENT_CHANGES_NOTHING",
            detail: format!(
                "the Tax Administration already shows these values for {period}, so there is \
                 nothing to replace. If the return was replaced in OmaVero, record that with \
                 `file-vat complete-manual --replacement`"
            ),
        });
    }
    Ok((filed, changes))
}

fn period_dates(period: &PeriodId) -> (String, String) {
    let (first_month, _) = period.month_bounds();
    let start = format!("{:04}-{first_month:02}-01", period.year());
    (start, period.last_date().to_string())
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
