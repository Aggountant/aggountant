//! `file-vat status`: what happened to a period's VAT return, without sending
//! anything.
//!
//! The question is answered from local records first: the fiscal filing record
//! (receipt or manual evidence) and the period's `file-vat` Run journals. Only a
//! direct filing with a stored receipt is then compared with the authority's
//! GetFiledVATReturn/v2 view. The read never submits, never writes the outbox,
//! the journal or the fiscal store, and a failed read is `VAT_STATUS_UNKNOWN`,
//! never "not filed": the receipt, not OmaVero visibility, proves the filing.
//!
//! The fetched status is deliberately not recorded as an observation. A Run's
//! journal records what the Run did, and a completed Run has nothing left to do;
//! a status read that wrote would also stop being safe to repeat at will.

use serde::Serialize;
use serde_json::Value;

use aggountant_broker::{Broker, FileVatReturn, FiledVatReturn};
use aggountant_ledger::{Ledger, ManualVatFilingEvidence, VatFilingReceipt};
use aggountant_model::PeriodId;

use super::{
    approved_filing_request, filed_vat_return_envelope, FileVatState, EFFECT_SUBMIT_FILING,
};
use crate::journal::EventPayload;
use crate::{fold, EffectStatus, ProcessEngine, ProcessError};

const NO_REFILE: &str = "The receipt proves the filing; do not file this period again.";

/// The answer `file-vat status` gives for a period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileVatFilingStatus {
    /// Accepted with a receipt; the authority does not show the return yet.
    Received,
    /// Accepted with a receipt; the authority is still processing it, or still
    /// shows an earlier return.
    PendingAuthorityProcessing,
    /// The authority shows the return processed with the approved values.
    Processed,
    /// The authority shows a return processed on or after the receipt whose
    /// values differ from the approved request.
    ProcessedValuesDiffer,
    /// The authority answered with a status this version cannot interpret.
    UnrecognizedRemoteStatus,
    /// Filed, but with no API receipt to ask about (manual filing, or a filing
    /// recorded by `file-vat reconcile`).
    FiledWithoutReceipt,
    /// A direct filing whose outcome is unknown; `file-vat reconcile` resolves it.
    OutcomeUnknown,
    /// A filing dispatch was interrupted before its outcome reached the books.
    FilingInterrupted,
    /// No filing and no filing attempt is recorded for the period.
    NotFiled,
}

/// Result of `file-vat status`. Every key is always present so a caller can
/// branch on `status` and read the rest without probing.
#[derive(Debug, Clone, Serialize)]
pub struct FileVatStatus {
    pub period: String,
    pub status: FileVatFilingStatus,
    /// True only when GetFiledVATReturn/v2 was actually read for this answer.
    pub authority_checked: bool,
    pub run_id: Option<String>,
    pub receipt: Option<VatFilingReceipt>,
    pub remote_status: Option<String>,
    pub submitted_date: Option<String>,
    pub manual_filing: Option<ManualVatFilingEvidence>,
    pub next_command: Option<String>,
    pub message: String,
}

/// The local half of `file-vat status`. A caller builds a Broker (which needs
/// credentials) only for [`FileVatStatusPlan::Query`].
#[derive(Debug)]
pub enum FileVatStatusPlan {
    Answered(FileVatStatus),
    Query(FiledVatQuery),
}

/// A direct filing with a stored receipt, ready to be compared with the
/// authority's view by [`ProcessEngine::query_file_vat_status`].
#[derive(Debug)]
pub struct FiledVatQuery {
    period: String,
    run_id: String,
    receipt: VatFilingReceipt,
    expected: FileVatReturn,
}

/// What a period's `file-vat` Run journal says about its filing effect.
struct FilingRun {
    run_id: String,
    status: EffectStatus,
    /// The error code of the latest `EffectRejected` for the filing effect.
    rejected_code: Option<String>,
}

impl ProcessEngine {
    /// Decide from local records alone what can be said about the period's VAT
    /// filing. Writes nothing.
    pub fn file_vat_status_plan(
        &self,
        period: &PeriodId,
    ) -> Result<FileVatStatusPlan, ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        let view = ledger.period_view(period)?;
        let runs = self.file_vat_filing_runs(period)?;
        let local = |status, run_id: Option<String>, next_command, message| FileVatStatus {
            period: period.to_string(),
            status,
            authority_checked: false,
            run_id,
            receipt: None,
            remote_status: None,
            submitted_date: None,
            manual_filing: None,
            next_command,
            message,
        };
        let answer = |status, run_id, next_command, message| {
            FileVatStatusPlan::Answered(local(status, run_id, next_command, message))
        };

        if view.vat_filed {
            if let Some(receipt) = ledger.vat_filing_receipt(period)? {
                let (run_id, state) = self.filing_run_with_receipt(&runs)?.ok_or_else(|| {
                    ProcessError::JournalCorrupt(format!(
                        "period {period} has VAT receipt {} but no single file-vat Run holds \
                         the approved request; {NO_REFILE}",
                        receipt.unique_identifier
                    ))
                })?;
                let expected = approved_filing_request(&state)?;
                return Ok(FileVatStatusPlan::Query(FiledVatQuery {
                    period: state.period,
                    run_id,
                    receipt,
                    expected,
                }));
            }
            if let Some(manual) = ledger.manual_vat_filing_evidence(period)? {
                let message = format!(
                    "Filed manually through {} (evidence {}). There is no API receipt, so \
                     aggountant has nothing to ask the Tax Administration about; check OmaVero. \
                     Do not file this period again.",
                    manual.channel, manual.evidence_ref
                );
                let mut status = local(
                    FileVatFilingStatus::FiledWithoutReceipt,
                    None,
                    None,
                    message,
                );
                status.manual_filing = Some(manual);
                return Ok(FileVatStatusPlan::Answered(status));
            }
            let run_id = runs
                .iter()
                .find(|run| run.status == EffectStatus::Succeeded)
                .map(|run| run.run_id.clone());
            return Ok(answer(
                FileVatFilingStatus::FiledWithoutReceipt,
                run_id,
                None,
                "Filed: `file-vat reconcile` matched the Tax Administration's processed return \
                 to the approved request. That path returns no receipt to ask about; check \
                 OmaVero. Do not file this period again."
                    .into(),
            ));
        }

        if let Some(run) = runs.iter().find(|run| {
            run.rejected_code
                .as_deref()
                .is_some_and(outcome_may_be_accepted)
        }) {
            return Ok(answer(
                FileVatFilingStatus::OutcomeUnknown,
                Some(run.run_id.clone()),
                Some(format!(
                    "aggountant file-vat reconcile --run-id {}",
                    run.run_id
                )),
                "A direct filing was sent but its outcome is unknown (no timely answer, or \
                 an answer that was neither a receipt nor a rejection). It may have been \
                 received. Resolve it with `file-vat reconcile`; do not file again."
                    .into(),
            ));
        }
        // Executing, or Succeeded without the fiscal record: the dispatch began and
        // its outcome never reached the books. It may have been accepted.
        if let Some(run) = runs.iter().find(|run| {
            matches!(
                run.status,
                EffectStatus::Executing | EffectStatus::Succeeded
            )
        }) {
            return Ok(answer(
                FileVatFilingStatus::FilingInterrupted,
                Some(run.run_id.clone()),
                None,
                "A direct filing was dispatched but the Run was interrupted before its outcome \
                 reached the books. It may have been accepted; do not file again until the Run \
                 has been resumed."
                    .into(),
            ));
        }
        let rejected = runs.iter().rev().find(|run| run.rejected_code.is_some());
        let message = match rejected {
            Some(run) => format!(
                "No VAT filing is recorded for {period}. The direct filing in Run {} was \
                 rejected ({}), so nothing was accepted. If the return may have been filed \
                 outside aggountant, check OmaVero before filing.",
                run.run_id,
                run.rejected_code.as_deref().unwrap_or_default()
            ),
            None => format!(
                "No VAT filing is recorded for {period}. If the return may have been filed \
                 outside aggountant, check OmaVero before filing."
            ),
        };
        Ok(answer(
            FileVatFilingStatus::NotFiled,
            rejected.map(|run| run.run_id.clone()),
            None,
            message,
        ))
    }

    /// Compare a receipted filing with the authority's GetFiledVATReturn/v2 view.
    /// A read only: nothing is submitted and nothing local is written.
    pub fn query_file_vat_status(
        &self,
        query: &FiledVatQuery,
        broker: &Broker,
    ) -> Result<FileVatStatus, ProcessError> {
        let unknown = |detail: String| {
            ProcessError::VatStatusUnknown(format!(
                "could not read the Tax Administration's view of {}: {detail}. Receipt {} \
                 (accepted {}) still proves the filing; do not file again.",
                query.period, query.receipt.unique_identifier, query.receipt.accepted_timestamp
            ))
        };
        let envelope =
            filed_vat_return_envelope(&query.expected, format!("status:{}", query.period))?;
        let body = broker
            .fetch(&envelope)
            .map_err(|e| unknown(e.to_string()))?;
        let filed: FiledVatReturn = serde_json::from_value(body)
            .map_err(|e| unknown(format!("GetFiledVATReturn malformed: {e}")))?;
        let submitted_date = filed
            .values
            .get("SubmittedDate")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let (status, detail) = classify(&filed, submitted_date.as_deref(), query);
        let receipt = &query.receipt;
        let message = format!(
            "Accepted by the Tax Administration with receipt {} at {}. {detail} {NO_REFILE}",
            receipt.unique_identifier, receipt.accepted_timestamp
        );
        Ok(FileVatStatus {
            period: query.period.clone(),
            status,
            authority_checked: true,
            run_id: Some(query.run_id.clone()),
            receipt: Some(query.receipt.clone()),
            remote_status: filed.status.clone(),
            submitted_date,
            manual_filing: None,
            next_command: None,
            message,
        })
    }

    /// The Run that holds the approved request of a receipted filing: one whose
    /// filing effect succeeded (a resume can also adopt a filing another Run made),
    /// or, after a crash between the fiscal record and the journal, the one still
    /// executing it. Several candidates are accepted only when they bound identical
    /// request bytes; otherwise the request is ambiguous and `None` is returned.
    fn filing_run_with_receipt(
        &self,
        runs: &[FilingRun],
    ) -> Result<Option<(String, FileVatState)>, ProcessError> {
        for wanted in [EffectStatus::Succeeded, EffectStatus::Executing] {
            let mut found: Option<(String, FileVatState)> = None;
            for run in runs.iter().filter(|run| run.status == wanted) {
                let state = self.read_file_vat_state(&run.run_id)?;
                match &found {
                    None => found = Some((run.run_id.clone(), state)),
                    Some((_, first)) if first.filing_sent_hash == state.filing_sent_hash => {}
                    Some(_) => return Ok(None),
                }
            }
            if found.is_some() {
                return Ok(found);
            }
        }
        Ok(None)
    }

    /// The filing effect of every `file-vat` Run for `period`, in run-id order.
    fn file_vat_filing_runs(&self, period: &PeriodId) -> Result<Vec<FilingRun>, ProcessError> {
        let period = period.to_string();
        let mut runs = Vec::new();
        for run_id in self.list()? {
            let journal = match self.open_journal(&run_id) {
                Ok(journal) => journal,
                // A directory without a journal is not a Run.
                Err(ProcessError::RunNotFound(_)) => continue,
                Err(error) => return Err(error),
            };
            let events = journal.read_all()?;
            let f = fold(&run_id, &events)?;
            if f.workflow != "file-vat" || f.period != period {
                continue;
            }
            let Some(effect) = f.effect(EFFECT_SUBMIT_FILING) else {
                continue;
            };
            let rejected_code = events.iter().rev().find_map(|event| match &event.payload {
                EventPayload::EffectRejected {
                    effect_key,
                    error_code,
                } if effect_key == EFFECT_SUBMIT_FILING => Some(error_code.clone()),
                _ => None,
            });
            runs.push(FilingRun {
                status: effect.status,
                // A later success or retry supersedes an earlier rejection.
                rejected_code: rejected_code.filter(|_| effect.status == EffectStatus::Rejected),
                run_id,
            });
        }
        Ok(runs)
    }
}

/// Whether a journalled filing rejection leaves open that the authority accepted
/// the return: an unknown outcome, or a success status whose body could not be
/// read as a receipt (`MALFORMED`, any other 2xx). Only these can be reconciled
/// into a filing; every other code is a rejection the authority stated.
fn outcome_may_be_accepted(code: &str) -> bool {
    code == "OUTCOME_UNKNOWN" || code.starts_with("MALFORMED") || code.starts_with("HTTP_2")
}

fn classify(
    filed: &FiledVatReturn,
    submitted_date: Option<&str>,
    query: &FiledVatQuery,
) -> (FileVatFilingStatus, String) {
    match filed.status.as_deref() {
        Some("Missing") => (
            FileVatFilingStatus::Received,
            "The return is not visible in GetFiledVATReturn yet; processing and OmaVero \
             visibility can take days."
                .into(),
        ),
        Some("Being Processed") => (
            FileVatFilingStatus::PendingAuthorityProcessing,
            "The Tax Administration is processing the return.".into(),
        ),
        Some("Processed") => {
            let receipt_date = receipt_date(&query.receipt.accepted_timestamp);
            let (Some(receipt_date), Some(submitted)) =
                (receipt_date, submitted_date.filter(|d| is_date(d)))
            else {
                return (
                    FileVatFilingStatus::UnrecognizedRemoteStatus,
                    "The authority shows a processed return, but its submission date cannot be \
                     compared with the receipt, so it cannot be identified as this filing."
                        .into(),
                );
            };
            // The receipt timestamp is UTC (or carries its offset) and the authority's
            // date is Finnish local time, which is never earlier, so a submission date
            // before the receipt's date belongs to an earlier return.
            if submitted < receipt_date {
                (
                    FileVatFilingStatus::PendingAuthorityProcessing,
                    format!(
                        "The authority still shows an earlier return submitted {submitted}; this \
                         filing is awaiting processing."
                    ),
                )
            } else if filed.semantically_matches(&query.expected) {
                (
                    FileVatFilingStatus::Processed,
                    format!(
                        "The Tax Administration has processed the return (submitted \
                         {submitted}) with the approved values."
                    ),
                )
            } else {
                (
                    FileVatFilingStatus::ProcessedValuesDiffer,
                    format!(
                        "The authority shows a processed return submitted {submitted} whose \
                         values differ from the approved request. Compare it in OmaVero; a \
                         correction is a replacement return with its own approval."
                    ),
                )
            }
        }
        other => (
            FileVatFilingStatus::UnrecognizedRemoteStatus,
            format!(
                "The authority answered with status {} that aggountant cannot interpret.",
                other.unwrap_or("(none)")
            ),
        ),
    }
}

fn receipt_date(accepted_timestamp: &str) -> Option<&str> {
    accepted_timestamp.get(..10).filter(|date| is_date(date))
}

fn is_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}
