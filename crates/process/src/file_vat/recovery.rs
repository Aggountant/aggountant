//! A VAT filing whose outcome is unknown, and the way out of it.
//!
//! Once a FileVATReturn may have been accepted (a timeout, a 5xx, a 2xx without a
//! receipt), nothing may send the period's return again until the Tax
//! Administration has said what happened. [`ProcessEngine::refuse_while_filing_unresolved`]
//! is that rule; every route to a filing request passes it.
//!
//! `file-vat reconcile` asks with the read-only GetFiledVATReturn/v2 and records
//! the answer on the unresolved Run:
//! - the approved return, processed: the filing is adopted and nothing is sent;
//! - `Missing`: one "not found". A second one at least one Finnish bank day after
//!   the first (the same clock time on the next bank day or later) closes the Run
//!   as not filed, and a new Run may file the period;
//! - anything else: recorded, and the "not found" count starts over, because the
//!   authority shows something for the period;
//! - a failed or unreadable read: nothing is recorded.
//!
//! Each answer is an `Observe(GetFiledVATReturn)` effect on the Run, planned and
//! succeeded with the remote status and the check time in its note, so the
//! evidence sits in the Run's hash-chained journal with the attempt it resolves.
//! Closing the Run skips its filing and payment effects.
//!
//! An interrupted Run (the filing effect left executing, `vat_filed` not set) is
//! reconciled the same way, after what the Run left locally is used first:
//! - the fiscal filing already landed: the Run records it, nothing is read; a
//!   filing `complete-manual` recorded meanwhile is not the Run's, which is
//!   closed without claiming it;
//! - the outbox holds the authority's receipt: the receipt is recorded, nothing
//!   is read;
//! - a request may have reached the authority (an `Unknown` attempt, including a
//!   crash between the outbox intent and its answer): exactly as an unknown
//!   outcome above, because `Missing` right after sending does not prove the
//!   return was lost;
//! - no request of the Run left, or the authority refused each outright (by the
//!   same codes rule as a journalled rejection): one
//!   `Missing` closes the Run as not filed, because no return of it can surface.
//!
//! `file-vat status` reads the same evidence without recording it and names
//! `file-vat reconcile`; there is no resume command.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use aggountant_broker::{
    Attempt, AttemptOutcome, Broker, FileVatReceipt, FileVatReturn, FiledVatReturn,
    SVC_VERO_FILE_VAT_RETURN,
};
use aggountant_domain::bank_days::{finnish_date, finnish_local_time, one_bank_day_after};
use aggountant_ledger::{Ledger, ManualVatFilingEvidence};
use aggountant_model::PeriodId;

use super::{
    approved_filing_request, assert_filing_gate_approved_and_bound, filed_vat_return_envelope,
    filing_outcome_may_be_accepted, map_adapter, map_broker, rebuild_filing_env, FileVatState,
    ACTOR, EFFECT_SUBMIT_FILING, EFFECT_SUBMIT_PAYMENT,
};
use crate::journal::{EventPayload, Journal, JournalEvent};
use crate::{fold, Clock, EffectStatus, Fold, ProcessEngine, ProcessError, RunView, SystemClock};

const CHECK_EFFECT_PREFIX: &str = "authority-check:";
const CHECK_EFFECT_TYPE: &str = "Observe(GetFiledVATReturn)";
const CHECKED_AT: &str = " checked_at=";
/// The GetFiledVATReturn/v2 status for "no return for this period".
const NOT_FOUND: &str = "Missing";

/// What `file-vat reconcile` concluded from the authority's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileVatReconcileOutcome {
    /// The authority has processed the approved return; the filing is recorded.
    Adopted,
    /// The authority has no return for the period. Recorded; one more such answer
    /// a bank day later closes the Run.
    NotFound,
    /// The second qualifying "not found": the Run is closed as not filed.
    ClosedNotFiled,
    /// An interrupted Run whose period was meanwhile filed manually: closed
    /// without claiming the manual filing.
    Superseded,
    /// Anything else. Nothing counts toward re-filing.
    Unresolved,
}

/// One answer recorded on the Run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorityCheck {
    pub checked_at: DateTime<Utc>,
    /// The Finnish calendar date of the check, which bank days are counted in.
    pub finnish_date: NaiveDate,
    /// GetFiledVATReturn's `Status`, or `(none)`.
    pub remote_status: String,
}

/// Result of `file-vat reconcile`.
#[derive(Debug, Clone, Serialize)]
pub struct FileVatReconcile {
    #[serde(flatten)]
    pub run: RunView,
    pub outcome: FileVatReconcileOutcome,
    /// The status this read returned.
    pub remote_status: Option<String>,
    /// Every answer recorded on the Run, oldest first.
    pub authority_checks: Vec<AuthorityCheck>,
    /// After a "not found": the earliest instant a second one counts.
    pub next_check_from: Option<DateTime<Utc>>,
    pub message: String,
}

/// Why a period may not be filed now.
pub(super) enum Unresolved {
    OutcomeUnknown,
    Interrupted,
}

impl ProcessEngine {
    /// Refuse to file `period` while an earlier filing for it may have been
    /// accepted: a Run whose filing outcome is unknown (this Run included), or
    /// another Run whose dispatch was interrupted before its outcome reached the
    /// books. `own_run` is the Run about to file, `None` before a Run exists.
    pub(super) fn refuse_while_filing_unresolved(
        &self,
        period: &PeriodId,
        own_run: Option<&str>,
    ) -> Result<(), ProcessError> {
        let Some((run_id, why)) = self.unresolved_filing(period, own_run)? else {
            return Ok(());
        };
        let message = match why {
            Unresolved::OutcomeUnknown => format!(
                "Run {run_id} sent a VAT return for {period} whose outcome is unknown; it may \
                 have been accepted. Run `aggountant file-vat reconcile --run-id {run_id}`: it \
                 adopts the filing if the Tax Administration has it, and two answers that it \
                 has none, at least one bank day apart, close that Run so the period can be \
                 filed again"
            ),
            Unresolved::Interrupted => format!(
                "Run {run_id} was interrupted while sending a VAT return for {period}; it may \
                 have been accepted. Run `aggountant file-vat status --period {period}`: it asks \
                 the Tax Administration what happened and names the command that records the \
                 answer. Do not file the period again until that Run is resolved"
            ),
        };
        Err(ProcessError::VatFilingUnresolved(message))
    }

    /// Whether a filing for `period` is unresolved, and in which Run. One rule for
    /// the dispatch guard, `complete-manual` and `file-vat status`.
    pub(super) fn unresolved_filing(
        &self,
        period: &PeriodId,
        own_run: Option<&str>,
    ) -> Result<Option<(String, Unresolved)>, ProcessError> {
        let vat_filed = Ledger::open(&self.home)?.period_view(period)?.vat_filed;
        for run in self.file_vat_filing_runs(period)? {
            if run
                .rejected_code
                .as_deref()
                .is_some_and(filing_outcome_may_be_accepted)
            {
                return Ok(Some((run.run_id, Unresolved::OutcomeUnknown)));
            }
            // A Run resuming its own interrupted dispatch is the one allowed through:
            // it replays a stored outcome and never sends again.
            let interrupted = !vat_filed
                && Some(run.run_id.as_str()) != own_run
                && matches!(
                    run.status,
                    EffectStatus::Executing | EffectStatus::Succeeded
                );
            if interrupted {
                return Ok(Some((run.run_id, Unresolved::Interrupted)));
            }
        }
        Ok(None)
    }

    pub(super) fn period_has_unresolved_filing(
        &self,
        period: &PeriodId,
    ) -> Result<bool, ProcessError> {
        Ok(self.unresolved_filing(period, None)?.is_some())
    }

    /// Ask the Tax Administration about a Vero filing parked after a rejection or
    /// interrupted mid-dispatch, with the read-only GetFiledVATReturn/v2. A processed return matching the
    /// approved request is adopted without a re-submit. For a filing whose outcome
    /// is unknown the answer is recorded on the Run, and two "not found" answers at
    /// least one bank day apart close it as not filed (see the module docs).
    pub fn reconcile_file_vat(
        &self,
        run_id: &str,
        broker: &Broker,
    ) -> Result<FileVatReconcile, ProcessError> {
        self.reconcile_file_vat_at(run_id, broker, &SystemClock)
    }

    /// As [`Self::reconcile_file_vat`], with the clock that dates the check.
    pub fn reconcile_file_vat_at(
        &self,
        run_id: &str,
        broker: &Broker,
        clock: &dyn Clock,
    ) -> Result<FileVatReconcile, ProcessError> {
        let _lease = self.acquire_lease(run_id)?;
        let journal = self.open_journal(run_id)?;
        let events = journal.read_all()?;
        let f = fold(run_id, &events)?;
        self.check_version(&f)?;
        let filing = f.effect(EFFECT_SUBMIT_FILING).map(|effect| effect.status);
        if !matches!(
            filing,
            Some(EffectStatus::Rejected | EffectStatus::Executing)
        ) {
            return Err(ProcessError::InvalidAnswer(
                "submit-filing is neither parked for reconciliation nor interrupted".into(),
            ));
        }
        let state = self.read_file_vat_state(run_id)?;
        assert_filing_gate_approved_and_bound(
            &f,
            &state.filing_body,
            &state.vero_period,
            &state.filing_sent_hash,
        )?;
        let period = PeriodId::parse(&state.period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;
        let vat_filed = Ledger::open(&self.home)?.period_view(&period)?.vat_filed;

        let route = if filing == Some(EffectStatus::Rejected) {
            Route::Parked {
                outcome_unknown: latest_filing_rejection(&events)
                    .is_some_and(|code| filing_outcome_may_be_accepted(&code)),
            }
        } else if vat_filed {
            // `complete-manual` may file the period while this Run is unresolved; that
            // filing is not this Run's to claim.
            match Ledger::open(&self.home)?.manual_vat_filing_evidence(&period)? {
                Some(manual) => Route::FiledManually(manual),
                None => Route::FiscalLanded,
            }
        } else {
            match self.interrupted_dispatch(broker, &events, &period, &state)? {
                Dispatch::Receipt(receipt) => Route::OutboxReceipt(receipt),
                Dispatch::MayHaveArrived { attempted } => {
                    Route::Interrupted(Reach::MayHaveArrived { attempted })
                }
                Dispatch::NotAccepted { since } => Route::Interrupted(Reach::Refused { since }),
            }
        };
        let adopted = |message: String| -> Result<_, ProcessError> {
            let f2 = fold(run_id, &journal.read_all()?)?;
            self.finish_filing_postcondition(&journal, &f2, &state)?;
            Ok((FileVatReconcileOutcome::Adopted, None, None, message))
        };
        let (outcome, remote_status, next_check_from, message) = match route {
            // The fiscal filing landed and the Run stopped before journalling it: the
            // books already hold the filing, so only the Run catches up.
            Route::FiscalLanded => {
                journal.append(EventPayload::EffectReconciled {
                    effect_key: EFFECT_SUBMIT_FILING.into(),
                    note: format!("fiscal shows period {period} vat_filed (fiscal wins)"),
                })?;
                adopted(
                    "The filing had already been recorded in the books before the Run was \
                     interrupted; the Run now records it too. Nothing was sent."
                        .into(),
                )?
            }
            Route::FiledManually(manual) => {
                close_as_not_filed(
                    &journal,
                    &format!(
                        "SUPERSEDED: period {period} was filed manually through {} (evidence \
                         {}); this Run's filing is not recorded as filed",
                        manual.channel, manual.evidence_ref
                    ),
                )?;
                (
                    FileVatReconcileOutcome::Superseded,
                    None,
                    None,
                    format!(
                        "{period} was filed manually through {} (evidence {}) while this Run \
                         was interrupted. The Run is closed without claiming that filing; \
                         nothing was sent.",
                        manual.channel, manual.evidence_ref
                    ),
                )
            }
            // The authority's receipt is proof; no read is needed.
            Route::OutboxReceipt(receipt) => {
                self.record_vat_receipt(&period, run_id, &state, &receipt)?;
                journal.append(EventPayload::EffectSucceeded {
                    effect_key: EFFECT_SUBMIT_FILING.into(),
                    note: super::receipt_note(&receipt),
                })?;
                adopted(format!(
                    "The Tax Administration accepted the return with receipt {} at {} before \
                     the Run was interrupted. The receipt is recorded and nothing was sent.",
                    receipt.unique_identifier, receipt.accepted_timestamp
                ))?
            }
            Route::Parked { outcome_unknown } => {
                let attempted = self.last_attempt_date(broker, &period)?.ok_or_else(|| {
                    ProcessError::Store("Vero filing attempt timestamp is missing".into())
                })?;
                let reach = if outcome_unknown {
                    Reach::MayHaveArrived { attempted }
                } else {
                    Reach::Refused { since: attempted }
                };
                self.reconcile_with_authority(
                    &journal, &f, run_id, &state, &period, broker, clock, &reach, false,
                )?
            }
            Route::Interrupted(reach) => self.reconcile_with_authority(
                &journal, &f, run_id, &state, &period, broker, clock, &reach, true,
            )?,
        };
        let events = journal.read_all()?;
        Ok(FileVatReconcile {
            run: self.show(run_id)?,
            outcome,
            remote_status,
            authority_checks: authority_checks(&events)?,
            next_check_from,
            message,
        })
    }

    /// Ask GetFiledVATReturn about the period and act on the answer (see the module
    /// docs). `interrupted` says the Run stopped mid-dispatch rather than parking on
    /// a rejection; such a Run whose request never reached the authority is closed
    /// on one "not found", because nothing of it can surface later.
    #[allow(clippy::too_many_arguments)]
    fn reconcile_with_authority(
        &self,
        journal: &Journal,
        f: &Fold,
        run_id: &str,
        state: &FileVatState,
        period: &PeriodId,
        broker: &Broker,
        clock: &dyn Clock,
        reach: &Reach,
        interrupted: bool,
    ) -> Result<ReconcileStep, ProcessError> {
        let expected = approved_filing_request(state)?;
        let envelope = filed_vat_return_envelope(&expected, format!("reconcile:{}", state.period))?;
        // A failed or unreadable read returns here, before anything is recorded.
        let body = broker.fetch(&envelope).map_err(map_broker)?;
        let filed: FiledVatReturn = serde_json::from_value(body)
            .map_err(|e| ProcessError::Store(format!("GetFiledVATReturn malformed: {e}")))?;
        let checked_at = clock_now(clock)?;
        let remote_status = filed.status.clone();
        let status = remote_status.as_deref().unwrap_or("(none)").to_string();

        let step = match judge(&filed, &expected, reach) {
            Finding::Matches => {
                let ledger = Ledger::open(&self.home)?;
                if !ledger.period_view(period)?.vat_filed {
                    let auth = ledger.acquire_filing_authority(period, run_id)?;
                    // A reconciliation response carries no FileVATReturn receipt fields;
                    // never invent them. The approved payload hash is still audited.
                    ledger.file_vat(period, &auth, &state.filing_sent_hash, ACTOR)?;
                }
                journal.append(EventPayload::EffectReconciled {
                    effect_key: EFFECT_SUBMIT_FILING.into(),
                    note: "GetFiledVATReturn semantically matches approved FileVATReturn bytes"
                        .into(),
                })?;
                let f2 = fold(run_id, &journal.read_all()?)?;
                self.finish_filing_postcondition(journal, &f2, state)?;
                (
                    FileVatReconcileOutcome::Adopted,
                    "The Tax Administration has processed the approved return; the filing is \
                     recorded and nothing was sent."
                        .to_string(),
                    None,
                )
            }
            // A rejected filing has nothing outstanding to record answers for.
            _ if !interrupted && matches!(reach, Reach::Refused { .. }) => (
                FileVatReconcileOutcome::Unresolved,
                "The Tax Administration does not show the approved return. This filing was \
                 rejected, so nothing is outstanding."
                    .to_string(),
                None,
            ),
            Finding::NotFound if matches!(reach, Reach::Refused { .. }) => {
                record_authority_check(journal, f, &status, checked_at)?;
                let at = finnish_minute(checked_at);
                close_as_not_filed(
                    journal,
                    &format!(
                        "NOT_FILED: the Run was interrupted before the Tax Administration \
                         accepted a FileVATReturn request, and GetFiledVATReturn answered \
                         {NOT_FOUND} on {at} (Finnish time)"
                    ),
                )?;
                (
                    FileVatReconcileOutcome::ClosedNotFiled,
                    format!(
                        "The Run was interrupted before the Tax Administration accepted its \
                         request, and on {at} (Finnish time) it answered that it has no return \
                         for {period}. This Run is closed as not filed; start a new file-vat \
                         Run to file the period."
                    ),
                    None,
                )
            }
            Finding::Other if matches!(reach, Reach::Refused { .. }) => {
                record_authority_check(journal, f, &status, checked_at)?;
                (
                    FileVatReconcileOutcome::Unresolved,
                    format!(
                        "The Tax Administration answered with status {status} for {period}, \
                         which neither matches the approved return nor says it has none, \
                         although this Run's request was not accepted. The answer is recorded. \
                         Compare the return in OmaVero; do not file again."
                    ),
                    None,
                )
            }
            _ => {
                record_authority_check(journal, f, &status, checked_at)?;
                let checks = authority_checks(&journal.read_all()?)?;
                match not_found_streak(&checks) {
                    NotFoundStreak::Conclusive { first, last } => {
                        let (first, last) = (finnish_minute(first), finnish_minute(last));
                        close_as_not_filed(
                            journal,
                            &format!(
                                "NOT_FILED: GetFiledVATReturn answered {NOT_FOUND} on {first} \
                                 and on {last} (Finnish time)"
                            ),
                        )?;
                        (
                            FileVatReconcileOutcome::ClosedNotFiled,
                            format!(
                                "The Tax Administration answered on {first} and again on {last} \
                                 (Finnish time) that it has no return for {period}. This Run is \
                                 closed as not filed; start a new file-vat Run to file the \
                                 period."
                            ),
                            None,
                        )
                    }
                    NotFoundStreak::Waiting { first } => {
                        let next = one_bank_day_after(first);
                        (
                            FileVatReconcileOutcome::NotFound,
                            format!(
                                "The Tax Administration has no return for {period} yet; a return \
                                 can take days to show. Ask again at {} (Finnish time) or later: \
                                 a second \"not found\" then closes this Run so the period can \
                                 be filed again. Do not file before that.",
                                finnish_minute(next)
                            ),
                            Some(next),
                        )
                    }
                    NotFoundStreak::None => (
                        FileVatReconcileOutcome::Unresolved,
                        format!(
                            "The Tax Administration answered with status {status}, which \
                             neither matches the approved return nor says it has none. The \
                             answer is recorded and the \"not found\" count starts over. \
                             Compare the return in OmaVero; do not file again."
                        ),
                        None,
                    ),
                }
            }
        };
        let (outcome, message, next_check_from) = step;
        Ok((outcome, remote_status, next_check_from, message))
    }

    /// What the outbox says about the dispatch of an interrupted Run: the requests
    /// made under the period's key since the Run first began dispatching.
    pub(super) fn interrupted_dispatch(
        &self,
        broker: &Broker,
        events: &[JournalEvent],
        period: &PeriodId,
        state: &FileVatState,
    ) -> Result<Dispatch, ProcessError> {
        let began = events
            .iter()
            .find(|event| {
                matches!(&event.payload, EventPayload::EffectExecuting { effect_key }
                    if effect_key == EFFECT_SUBMIT_FILING)
            })
            .map(|event| parse_instant(&event.ts))
            .ok_or_else(|| {
                ProcessError::JournalCorrupt("submit-filing never began executing".into())
            })??;
        let entry = broker
            .outbox_entry(SVC_VERO_FILE_VAT_RETURN, &filing_outbox_key(period))
            .map_err(map_broker)?;
        let Some(entry) = entry else {
            return Ok(Dispatch::NotAccepted { since: began });
        };
        // A receipt under the period's key is the period's filing whichever attempt
        // earned it: the Broker replays it instead of sending again.
        if let Some(success) = entry.terminal_success() {
            let sent_hash = rebuild_filing_env(period, &state.filing_body)
                .sent_hash()
                .map_err(map_broker)?;
            if success.sent_hash != sent_hash {
                return Err(ProcessError::JournalCorrupt(format!(
                    "the outbox receipt for {period} acknowledges a different FileVATReturn \
                     request than this Run approved; compare the filing in OmaVero"
                )));
            }
            let (AttemptOutcome::Delivered { receipt }
            | AttemptOutcome::RecoveredAlreadyProcessed { receipt }) = &success.outcome
            else {
                unreachable!("terminal_success returns a delivered attempt");
            };
            let receipt = FileVatReceipt::decode(receipt).map_err(map_adapter)?;
            return Ok(Dispatch::Receipt(receipt));
        }
        // An attempt whose time cannot be read is counted as this Run's.
        let ours: Vec<&Attempt> = entry
            .attempts
            .iter()
            .filter(|attempt| parse_instant(&attempt.provenance.ts).map_or(true, |ts| ts >= began))
            .collect();
        let arrived = ours.iter().rev().find(|attempt| match &attempt.outcome {
            AttemptOutcome::Unknown { .. } => true,
            // The same rule as a journalled rejection: older binaries recorded a
            // receiptless 2xx under these codes.
            AttemptOutcome::Failed { code, .. } => filing_outcome_may_be_accepted(code),
            _ => false,
        });
        Ok(match arrived {
            Some(attempt) => Dispatch::MayHaveArrived {
                attempted: parse_instant(&attempt.provenance.ts).unwrap_or(began),
            },
            None => Dispatch::NotAccepted { since: began },
        })
    }

    /// When the period's latest filing attempt was made.
    fn last_attempt_date(
        &self,
        broker: &Broker,
        period: &PeriodId,
    ) -> Result<Option<DateTime<Utc>>, ProcessError> {
        let entry = broker
            .outbox_entry(SVC_VERO_FILE_VAT_RETURN, &filing_outbox_key(period))
            .map_err(map_broker)?
            .ok_or_else(|| ProcessError::Store("Vero filing outbox entry is missing".into()))?;
        entry
            .attempts
            .last()
            .map(|attempt| parse_instant(&attempt.provenance.ts))
            .transpose()
    }
}

/// The period's FileVATReturn outbox key.
fn filing_outbox_key(period: &PeriodId) -> String {
    format!("{EFFECT_SUBMIT_FILING}:{period}")
}

/// What one reconciliation concluded: outcome, the status read, when to ask
/// again, and the message.
type ReconcileStep = (
    FileVatReconcileOutcome,
    Option<String>,
    Option<DateTime<Utc>>,
    String,
);

enum Route {
    /// The filing effect was rejected; `outcome_unknown` when it may still have
    /// been accepted.
    Parked { outcome_unknown: bool },
    /// Interrupted after the fiscal filing was written.
    FiscalLanded,
    /// Interrupted, and the period has since been filed manually.
    FiledManually(ManualVatFilingEvidence),
    /// Interrupted after the authority's receipt reached the outbox.
    OutboxReceipt(FileVatReceipt),
    /// Interrupted with nothing recorded locally.
    Interrupted(Reach),
}

/// What the outbox shows about an interrupted Run's FileVATReturn request.
#[derive(Debug)]
pub(super) enum Dispatch {
    /// The authority accepted the request; the outbox holds its receipt.
    Receipt(FileVatReceipt),
    /// A request may have reached the authority without an answer recorded.
    MayHaveArrived { attempted: DateTime<Utc> },
    /// No request of the Run left, or every one was refused outright, so no
    /// return of the Run can surface later. `since` is when the Run began.
    NotAccepted { since: DateTime<Utc> },
}

/// Whether the authority may hold a return from this filing.
pub(super) enum Reach {
    MayHaveArrived { attempted: DateTime<Utc> },
    Refused { since: DateTime<Utc> },
}

/// What GetFiledVATReturn says about the approved return.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Finding {
    /// The approved return, processed on or after the attempt.
    Matches,
    /// No return for the period.
    NotFound,
    Other,
}

/// One rule for `file-vat status` and `file-vat reconcile`.
pub(super) fn judge(filed: &FiledVatReturn, expected: &FileVatReturn, reach: &Reach) -> Finding {
    let since = match reach {
        Reach::MayHaveArrived { attempted } => attempted,
        Reach::Refused { since } => since,
    };
    // SubmittedDate is a Finnish calendar date, compared with the attempt's.
    let earliest = finnish_date(*since).to_string();
    if filed.semantically_matches_attempt(expected, Some(&earliest)) {
        Finding::Matches
    } else if filed.status.as_deref() == Some(NOT_FOUND) {
        Finding::NotFound
    } else {
        Finding::Other
    }
}

fn parse_instant(value: &str) -> Result<DateTime<Utc>, ProcessError> {
    DateTime::parse_from_rfc3339(value)
        .map(|instant| instant.to_utc())
        .map_err(|_| ProcessError::JournalCorrupt(format!("bad timestamp {value:?}")))
}

fn clock_now(clock: &dyn Clock) -> Result<DateTime<Utc>, ProcessError> {
    i64::try_from(clock.now_epoch_secs())
        .ok()
        .and_then(|secs| DateTime::from_timestamp(secs, 0))
        .ok_or_else(|| ProcessError::Store("the clock is out of range".into()))
}

/// An instant as Finnish local time to the minute, for messages and notes.
fn finnish_minute(instant: DateTime<Utc>) -> String {
    finnish_local_time(instant)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

fn latest_filing_rejection(events: &[crate::journal::JournalEvent]) -> Option<String> {
    events.iter().rev().find_map(|event| match &event.payload {
        EventPayload::EffectRejected {
            effect_key,
            error_code,
        } if effect_key == EFFECT_SUBMIT_FILING => Some(error_code.clone()),
        _ => None,
    })
}

/// Record one GetFiledVATReturn answer on the Run.
fn record_authority_check(
    journal: &Journal,
    f: &Fold,
    remote_status: &str,
    checked_at: DateTime<Utc>,
) -> Result<(), ProcessError> {
    let n = f
        .effects
        .iter()
        .filter(|effect| effect.key.starts_with(CHECK_EFFECT_PREFIX))
        .count()
        + 1;
    let effect_key = format!("{CHECK_EFFECT_PREFIX}{n}");
    journal.append(EventPayload::EffectPlanned {
        effect_key: effect_key.clone(),
        effect_type: CHECK_EFFECT_TYPE.into(),
    })?;
    journal.append(EventPayload::EffectSucceeded {
        effect_key,
        note: format!(
            "{remote_status}{CHECKED_AT}{}",
            checked_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ),
    })?;
    Ok(())
}

/// The answers recorded on a Run, oldest first.
fn authority_checks(
    events: &[crate::journal::JournalEvent],
) -> Result<Vec<AuthorityCheck>, ProcessError> {
    events
        .iter()
        .filter_map(|event| match &event.payload {
            EventPayload::EffectSucceeded { effect_key, note }
                if effect_key.starts_with(CHECK_EFFECT_PREFIX) =>
            {
                Some(note)
            }
            _ => None,
        })
        .map(|note| {
            let corrupt = || ProcessError::JournalCorrupt(format!("bad authority check {note:?}"));
            let (remote_status, checked_at) = note.rsplit_once(CHECKED_AT).ok_or_else(corrupt)?;
            let instant = DateTime::parse_from_rfc3339(checked_at)
                .map_err(|_| corrupt())?
                .to_utc();
            Ok(AuthorityCheck {
                checked_at: instant,
                finnish_date: finnish_date(instant),
                remote_status: remote_status.to_string(),
            })
        })
        .collect()
}

enum NotFoundStreak {
    None,
    Waiting {
        first: DateTime<Utc>,
    },
    Conclusive {
        first: DateTime<Utc>,
        last: DateTime<Utc>,
    },
}

/// The "not found" answers since the last other answer. Conclusive once one of
/// them came at least one bank day after the first ([`one_bank_day_after`]).
fn not_found_streak(checks: &[AuthorityCheck]) -> NotFoundStreak {
    let since = checks
        .iter()
        .rposition(|check| check.remote_status != NOT_FOUND)
        .map_or(0, |index| index + 1);
    let streak = &checks[since..];
    let Some(first) = streak.first().map(|check| check.checked_at) else {
        return NotFoundStreak::None;
    };
    let due = one_bank_day_after(first);
    match streak
        .iter()
        .map(|check| check.checked_at)
        .find(|checked_at| *checked_at >= due)
    {
        Some(last) => NotFoundStreak::Conclusive { first, last },
        None => NotFoundStreak::Waiting { first },
    }
}

/// Close a Run as not filed: its filing and payment will never run. `reason`
/// cites the authority's answers.
fn close_as_not_filed(journal: &Journal, reason: &str) -> Result<(), ProcessError> {
    journal.append(EventPayload::EffectSkipped {
        effect_key: EFFECT_SUBMIT_FILING.into(),
        reason: reason.to_string(),
    })?;
    journal.append(EventPayload::EffectSkipped {
        effect_key: EFFECT_SUBMIT_PAYMENT.into(),
        reason: "the filing was closed as not filed".into(),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(status: &str, date: &str) -> AuthorityCheck {
        let checked_at = DateTime::parse_from_rfc3339(&format!("{date}T09:00:00Z"))
            .unwrap()
            .to_utc();
        AuthorityCheck {
            checked_at,
            finnish_date: finnish_date(checked_at),
            remote_status: status.into(),
        }
    }

    #[test]
    fn another_answer_restarts_the_not_found_count() {
        let streak = not_found_streak(&[
            check("Missing", "2026-11-16"),
            check("Being Processed", "2026-11-17"),
            check("Missing", "2026-11-18"),
        ]);
        assert!(matches!(streak, NotFoundStreak::Waiting { first }
            if finnish_date(first).to_string() == "2026-11-18"));
        assert!(matches!(
            not_found_streak(&[
                check("Missing", "2026-11-16"),
                check("Processed", "2026-11-17")
            ]),
            NotFoundStreak::None
        ));
        assert!(matches!(
            not_found_streak(&[
                check("Missing", "2026-11-16"),
                check("Missing", "2026-11-16"),
                check("Missing", "2026-11-17"),
            ]),
            NotFoundStreak::Conclusive { .. }
        ));
    }

    #[test]
    fn two_answers_across_midnight_are_not_a_bank_day_apart() {
        let at = |value: &str| AuthorityCheck {
            checked_at: DateTime::parse_from_rfc3339(value).unwrap().to_utc(),
            finnish_date: finnish_date(DateTime::parse_from_rfc3339(value).unwrap().to_utc()),
            remote_status: NOT_FOUND.into(),
        };
        // Monday 23:59 and Tuesday 00:00 in Finland (UTC+2).
        let streak = not_found_streak(&[at("2026-11-16T21:59:00Z"), at("2026-11-16T22:00:00Z")]);
        assert!(matches!(streak, NotFoundStreak::Waiting { .. }));
        // Tuesday 23:59 counts.
        let streak = not_found_streak(&[at("2026-11-16T21:59:00Z"), at("2026-11-17T21:59:00Z")]);
        assert!(matches!(streak, NotFoundStreak::Conclusive { .. }));
    }
}
