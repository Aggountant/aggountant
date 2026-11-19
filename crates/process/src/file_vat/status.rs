//! `file-vat status`: what happened to a period's VAT return, without sending
//! anything.
//!
//! The question is answered from local records first: the fiscal filing record
//! (receipt or manual evidence) and the period's `file-vat` Run journals. Two
//! cases then ask the authority's GetFiledVATReturn/v2 view: a direct filing with
//! a stored receipt, and a Run interrupted while dispatching its filing, whose
//! outbox and remote view tell whether `file-vat reconcile` will adopt the filing
//! or close the Run as not filed. The read never submits, never writes the outbox,
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

use super::recovery::{judge, Dispatch, Finding, Reach, Unresolved};
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
    /// A filing dispatch was interrupted before its outcome reached the books, and
    /// what the Tax Administration shows does not settle it yet.
    FilingInterrupted,
    /// An interrupted filing the Tax Administration accepted (its receipt is in
    /// the outbox, or it shows the approved return processed); `file-vat
    /// reconcile` records it without sending anything.
    InterruptedFiled,
    /// An interrupted filing that never reached the Tax Administration, which has
    /// no return for the period; `file-vat reconcile` closes the Run as not filed.
    InterruptedNotFiled,
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

/// A filing to compare with the authority's view by
/// [`ProcessEngine::query_file_vat_status`]: a direct filing with a stored
/// receipt, or a Run interrupted mid-dispatch.
#[derive(Debug)]
pub struct FiledVatQuery {
    period: String,
    run_id: String,
    expected: FileVatReturn,
    kind: QueryKind,
}

#[derive(Debug)]
enum QueryKind {
    /// `run_lags`: the Run holding the request was interrupted after the receipt
    /// reached the books, so `file-vat reconcile` still has to catch it up.
    Receipted {
        receipt: VatFilingReceipt,
        run_lags: bool,
    },
    Interrupted,
}

impl FiledVatQuery {
    /// Why nothing may be filed while the authority's view cannot be read.
    pub fn unread_meaning(&self) -> &'static str {
        match self.kind {
            QueryKind::Receipted { .. } => "the stored receipt still proves the filing",
            QueryKind::Interrupted => {
                "the interrupted filing may have been accepted and nothing was changed"
            }
        }
    }
}

/// What a period's `file-vat` Run journal says about its filing effect.
pub(super) struct FilingRun {
    pub(super) run_id: String,
    pub(super) status: EffectStatus,
    /// The error code of the latest `EffectRejected` for the filing effect.
    pub(super) rejected_code: Option<String>,
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
                let run_lags = runs
                    .iter()
                    .any(|run| run.run_id == run_id && run.status == EffectStatus::Executing);
                return Ok(FileVatStatusPlan::Query(FiledVatQuery {
                    period: state.period,
                    run_id,
                    expected,
                    kind: QueryKind::Receipted { receipt, run_lags },
                }));
            }
            if let Some(manual) = ledger.manual_vat_filing_evidence(period)? {
                let message = format!(
                    "Filed manually through {} (evidence {}). There is no API receipt, so \
                     aggountant has nothing to ask the Tax Administration about; check OmaVero. \
                     Do not file this period again.",
                    manual.channel, manual.evidence_ref
                );
                // A Run interrupted before the manual filing is closed by reconcile.
                let interrupted = runs
                    .iter()
                    .find(|run| run.status == EffectStatus::Executing)
                    .map(|run| run.run_id.clone());
                let next_command = interrupted.as_deref().map(reconcile_command);
                let mut status = local(
                    FileVatFilingStatus::FiledWithoutReceipt,
                    interrupted,
                    next_command,
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

        match self.unresolved_filing(period, None)? {
            Some((run_id, Unresolved::OutcomeUnknown)) => {
                return Ok(answer(
                    FileVatFilingStatus::OutcomeUnknown,
                    Some(run_id.clone()),
                    Some(format!("aggountant file-vat reconcile --run-id {run_id}")),
                    "A direct filing was sent but its outcome is unknown (no timely answer, or \
                     an answer that was neither a receipt nor a rejection). It may have been \
                     received. Resolve it with `file-vat reconcile`: it adopts the filing if \
                     the Tax Administration has it, and two answers that it has none, at \
                     least one bank day apart, close the Run so the period can be filed again. \
                     Do not file again before that."
                        .into(),
                ));
            }
            Some((run_id, Unresolved::Interrupted)) => {
                let state = self.read_file_vat_state(&run_id)?;
                let expected = approved_filing_request(&state)?;
                return Ok(FileVatStatusPlan::Query(FiledVatQuery {
                    period: state.period,
                    run_id,
                    expected,
                    kind: QueryKind::Interrupted,
                }));
            }
            None => {}
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
        let QueryKind::Receipted { receipt, run_lags } = &query.kind else {
            return self.query_interrupted_filing(query, broker);
        };
        let unknown = |detail: String| {
            ProcessError::VatStatusUnknown(format!(
                "could not read the Tax Administration's view of {}: {detail}. Receipt {} \
                 (accepted {}) still proves the filing; do not file again.",
                query.period, receipt.unique_identifier, receipt.accepted_timestamp
            ))
        };
        let filed = read_filed_return(query, broker).map_err(unknown)?;
        let submitted_date = submitted_date(&filed);
        let (status, detail) = classify(&filed, submitted_date.as_deref(), query, receipt);
        let mut message = format!(
            "Accepted by the Tax Administration with receipt {} at {}. {detail} {NO_REFILE}",
            receipt.unique_identifier, receipt.accepted_timestamp
        );
        let next_command = run_lags.then(|| {
            message.push_str(
                " The Run was interrupted before it recorded the filing; `file-vat reconcile` \
                 catches it up without sending anything.",
            );
            reconcile_command(&query.run_id)
        });
        Ok(FileVatStatus {
            period: query.period.clone(),
            status,
            authority_checked: true,
            run_id: Some(query.run_id.clone()),
            receipt: Some(receipt.clone()),
            remote_status: filed.status.clone(),
            submitted_date,
            manual_filing: None,
            next_command,
            message,
        })
    }

    /// What `file-vat reconcile` would conclude about an interrupted Run, from its
    /// outbox and the authority's view, without recording anything.
    fn query_interrupted_filing(
        &self,
        query: &FiledVatQuery,
        broker: &Broker,
    ) -> Result<FileVatStatus, ProcessError> {
        let (period, run_id) = (&query.period, &query.run_id);
        let unknown = |detail: String| {
            ProcessError::VatStatusUnknown(format!(
                "could not tell what happened to the filing Run {run_id} was sending for \
                 {period}: {detail}. Nothing was changed; check again later. The return may \
                 have been accepted, so do not file again."
            ))
        };
        let period_id = PeriodId::parse(period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;
        let state = self.read_file_vat_state(run_id)?;
        let events = self.open_journal(run_id)?.read_all()?;
        let dispatch = self
            .interrupted_dispatch(broker, &events, &period_id, &state)
            .map_err(|e| unknown(e.to_string()))?;
        let reconcile = Some(reconcile_command(run_id));
        let status =
            |status, filed: Option<&FiledVatReturn>, next_command, message| FileVatStatus {
                period: period.clone(),
                status,
                authority_checked: filed.is_some(),
                run_id: Some(run_id.clone()),
                receipt: None,
                remote_status: filed.and_then(|f| f.status.clone()),
                submitted_date: filed.and_then(submitted_date),
                manual_filing: None,
                next_command,
                message,
            };
        let reach = match dispatch {
            Dispatch::Receipt(receipt) => {
                let message = format!(
                    "The Run was interrupted after the Tax Administration accepted the return \
                     with receipt {} at {}, before the filing reached the books. Record it with \
                     `file-vat reconcile`; nothing will be sent. {NO_REFILE}",
                    receipt.unique_identifier, receipt.accepted_timestamp
                );
                let mut answered = status(
                    FileVatFilingStatus::InterruptedFiled,
                    None,
                    reconcile,
                    message,
                );
                answered.receipt = Some(VatFilingReceipt {
                    unique_identifier: receipt.unique_identifier,
                    accepted_timestamp: receipt.accepted_timestamp,
                });
                return Ok(answered);
            }
            Dispatch::MayHaveArrived { attempted } => Reach::MayHaveArrived { attempted },
            Dispatch::NotAccepted { since } => Reach::Refused { since },
        };
        let filed = read_filed_return(query, broker).map_err(unknown)?;
        let remote = filed.status.as_deref().unwrap_or("(none)");
        Ok(match (judge(&filed, &query.expected, &reach), &reach) {
            (Finding::Matches, _) => status(
                FileVatFilingStatus::InterruptedFiled,
                Some(&filed),
                reconcile,
                "The Run was interrupted while sending the return, and the Tax Administration \
                 shows the approved return processed. Record it with `file-vat reconcile`; \
                 nothing will be sent. Do not file this period again."
                    .into(),
            ),
            (Finding::NotFound, Reach::Refused { .. }) => status(
                FileVatFilingStatus::InterruptedNotFiled,
                Some(&filed),
                reconcile,
                format!(
                    "The Run was interrupted before the Tax Administration accepted its \
                     request, and it has no return for {period}. `file-vat reconcile` records \
                     this answer and closes the Run as not filed; then start a new file-vat \
                     Run to file the period."
                ),
            ),
            (Finding::NotFound, Reach::MayHaveArrived { .. }) => status(
                FileVatFilingStatus::FilingInterrupted,
                Some(&filed),
                reconcile,
                format!(
                    "The Run was interrupted after its request may have reached the Tax \
                     Administration, which shows no return for {period} yet; a return can take \
                     days to show. `file-vat reconcile` records the answer, and two such \
                     answers at least one bank day apart close the Run so the period can be \
                     filed again. Do not file before that."
                ),
            ),
            (Finding::Other, _) => status(
                FileVatFilingStatus::FilingInterrupted,
                Some(&filed),
                None,
                format!(
                    "The Run was interrupted while sending the return, and the Tax \
                     Administration answers with status {remote}, which neither matches the \
                     approved return nor says it has none. Compare the return in OmaVero and \
                     check again later; do not file again."
                ),
            ),
        })
    }

    /// The period's VAT receipt, when it belongs to `run_id`'s approved request:
    /// the Run is the one [`Self::filing_run_with_receipt`] picks, or bound the
    /// same request bytes. `None` when the period has no receipt, or when a resume
    /// adopted a filing another Run sent with different bytes, so that the receipt
    /// is never shown on a Run whose request it does not acknowledge.
    pub(crate) fn vat_receipt_of_run(
        &self,
        run_id: &str,
        period: &PeriodId,
    ) -> Result<Option<VatFilingReceipt>, ProcessError> {
        let Some(receipt) = Ledger::open(&self.home)?.vat_filing_receipt(period)? else {
            return Ok(None);
        };
        let runs = self.file_vat_filing_runs(period)?;
        let Some((holder, state)) = self.filing_run_with_receipt(&runs)? else {
            return Ok(None);
        };
        let ours = holder == run_id
            || self.read_file_vat_state(run_id)?.filing_sent_hash == state.filing_sent_hash;
        Ok(ours.then_some(receipt))
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
    pub(super) fn file_vat_filing_runs(
        &self,
        period: &PeriodId,
    ) -> Result<Vec<FilingRun>, ProcessError> {
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

fn reconcile_command(run_id: &str) -> String {
    format!("aggountant file-vat reconcile --run-id {run_id}")
}

/// GetFiledVATReturn/v2 for the query's period; a read only.
fn read_filed_return(query: &FiledVatQuery, broker: &Broker) -> Result<FiledVatReturn, String> {
    let envelope = filed_vat_return_envelope(&query.expected, format!("status:{}", query.period))
        .map_err(|e| e.to_string())?;
    let body = broker.fetch(&envelope).map_err(|e| e.to_string())?;
    serde_json::from_value(body).map_err(|e| format!("GetFiledVATReturn malformed: {e}"))
}

fn submitted_date(filed: &FiledVatReturn) -> Option<String> {
    filed
        .values
        .get("SubmittedDate")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn classify(
    filed: &FiledVatReturn,
    submitted_date: Option<&str>,
    query: &FiledVatQuery,
    receipt: &VatFilingReceipt,
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
            let receipt_date = receipt_date(&receipt.accepted_timestamp);
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
