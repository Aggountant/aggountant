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
//!   the first closes the Run as not filed, and a new Run may file the period;
//! - anything else: recorded, and the "not found" count starts over, because the
//!   authority shows something for the period;
//! - a failed or unreadable read: nothing is recorded.
//!
//! Each answer is an `Observe(GetFiledVATReturn)` effect on the Run, planned and
//! succeeded with the remote status and the check time in its note, so the
//! evidence sits in the Run's hash-chained journal with the attempt it resolves.
//! Closing the Run skips its filing and payment effects.
//!
//! The recording and closing helpers do not care why the filing is unresolved, so
//! an interrupted dispatch can be checked the same way.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

use aggountant_broker::{Broker, FiledVatReturn, SVC_VERO_FILE_VAT_RETURN};
use aggountant_domain::bank_days::{a_bank_day_has_passed, finnish_date, next_finnish_bank_day};
use aggountant_ledger::Ledger;
use aggountant_model::PeriodId;

use super::{
    approved_filing_request, assert_filing_gate_approved_and_bound, filed_vat_return_envelope,
    filing_outcome_may_be_accepted, map_broker, ACTOR, EFFECT_SUBMIT_FILING, EFFECT_SUBMIT_PAYMENT,
};
use crate::journal::{EventPayload, Journal};
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
    /// on a later bank day closes the Run.
    NotFound,
    /// The second qualifying "not found": the Run is closed as not filed.
    ClosedNotFiled,
    /// Anything else. Nothing counts toward re-filing.
    Unresolved,
}

/// One answer recorded on the Run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuthorityCheck {
    /// RFC 3339, UTC.
    pub checked_at: String,
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
    /// After one "not found": the first Finnish date on which a second one counts.
    pub next_check_from: Option<NaiveDate>,
    pub message: String,
}

/// Why a period may not be filed now.
enum Unresolved {
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
                 have been accepted. Do not file the period again until that Run is resolved"
            ),
        };
        Err(ProcessError::VatFilingUnresolved(message))
    }

    /// Whether a filing for `period` is unresolved. One rule for the dispatch
    /// guard and for `complete-manual`.
    fn unresolved_filing(
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

    /// Ask the Tax Administration about a Vero filing parked after a rejection,
    /// with the read-only GetFiledVATReturn/v2. A processed return matching the
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
        if f.effect(EFFECT_SUBMIT_FILING).map(|effect| effect.status)
            != Some(EffectStatus::Rejected)
        {
            return Err(ProcessError::InvalidAnswer(
                "submit-filing is not parked for reconciliation".into(),
            ));
        }
        let outcome_unknown = latest_filing_rejection(&events)
            .is_some_and(|code| filing_outcome_may_be_accepted(&code));
        let state = self.read_file_vat_state(run_id)?;
        assert_filing_gate_approved_and_bound(
            &f,
            &state.filing_body,
            &state.vero_period,
            &state.filing_sent_hash,
        )?;
        let expected = approved_filing_request(&state)?;
        let envelope = filed_vat_return_envelope(&expected, format!("reconcile:{}", state.period))?;
        // A failed or unreadable read returns here, before anything is recorded.
        let body = broker.fetch(&envelope).map_err(map_broker)?;
        let filed: FiledVatReturn = serde_json::from_value(body)
            .map_err(|e| ProcessError::Store(format!("GetFiledVATReturn malformed: {e}")))?;
        let checked_at = clock_now(clock)?;
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
        let period = PeriodId::parse(&state.period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period: {e}")))?;
        let remote_status = filed.status.clone();

        let (outcome, next_check_from, message) = if filed
            .semantically_matches_attempt(&expected, Some(attempted_date))
        {
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
            (
                FileVatReconcileOutcome::Adopted,
                None,
                "The Tax Administration has processed the approved return; the filing is \
                 recorded and nothing was sent."
                    .to_string(),
            )
        } else if outcome_unknown {
            let status = remote_status.as_deref().unwrap_or("(none)");
            record_authority_check(&journal, &f, status, checked_at)?;
            let checks = authority_checks(&journal.read_all()?)?;
            match not_found_streak(&checks) {
                NotFoundStreak::Conclusive { first, last } => {
                    close_as_not_filed(&journal, first, last)?;
                    (
                        FileVatReconcileOutcome::ClosedNotFiled,
                        None,
                        format!(
                            "The Tax Administration answered on {first} and again on {last} that \
                             it has no return for {period}. This Run is closed as not filed; \
                             start a new file-vat Run to file the period."
                        ),
                    )
                }
                NotFoundStreak::Waiting { first } => {
                    let next = next_finnish_bank_day(first);
                    (
                        FileVatReconcileOutcome::NotFound,
                        Some(next),
                        format!(
                            "The Tax Administration has no return for {period} yet; a return can \
                             take days to show. Ask again on {next} or later: a second \"not \
                             found\" then closes this Run so the period can be filed again. Do \
                             not file before that."
                        ),
                    )
                }
                NotFoundStreak::None => (
                    FileVatReconcileOutcome::Unresolved,
                    None,
                    format!(
                        "The Tax Administration answered with status {status}, which neither \
                         matches the approved return nor says it has none. The answer is \
                         recorded and the \"not found\" count starts over. Compare the return \
                         in OmaVero; do not file again."
                    ),
                ),
            }
        } else {
            (
                FileVatReconcileOutcome::Unresolved,
                None,
                "The Tax Administration does not show the approved return. This filing was \
                 rejected, so nothing is outstanding."
                    .to_string(),
            )
        };
        let events = journal.read_all()?;
        Ok(FileVatReconcile {
            run: fold(run_id, &events)?.to_view(),
            outcome,
            remote_status,
            authority_checks: authority_checks(&events)?,
            next_check_from,
            message,
        })
    }
}

fn clock_now(clock: &dyn Clock) -> Result<DateTime<Utc>, ProcessError> {
    i64::try_from(clock.now_epoch_secs())
        .ok()
        .and_then(|secs| DateTime::from_timestamp(secs, 0))
        .ok_or_else(|| ProcessError::Store("the clock is out of range".into()))
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
                checked_at: checked_at.to_string(),
                finnish_date: finnish_date(instant),
                remote_status: remote_status.to_string(),
            })
        })
        .collect()
}

enum NotFoundStreak {
    None,
    Waiting { first: NaiveDate },
    Conclusive { first: NaiveDate, last: NaiveDate },
}

/// The "not found" answers since the last other answer. Conclusive once one of
/// them came at least one bank day after the first.
fn not_found_streak(checks: &[AuthorityCheck]) -> NotFoundStreak {
    let since = checks
        .iter()
        .rposition(|check| check.remote_status != NOT_FOUND)
        .map_or(0, |index| index + 1);
    let streak = &checks[since..];
    let Some(first) = streak.first().map(|check| check.finnish_date) else {
        return NotFoundStreak::None;
    };
    match streak
        .iter()
        .map(|check| check.finnish_date)
        .find(|date| a_bank_day_has_passed(first, *date))
    {
        Some(last) => NotFoundStreak::Conclusive { first, last },
        None => NotFoundStreak::Waiting { first },
    }
}

/// Close a Run whose filing the authority has twice said it does not have: the
/// filing and the payment will never run.
fn close_as_not_filed(
    journal: &Journal,
    first: NaiveDate,
    last: NaiveDate,
) -> Result<(), ProcessError> {
    journal.append(EventPayload::EffectSkipped {
        effect_key: EFFECT_SUBMIT_FILING.into(),
        reason: format!(
            "NOT_FILED: GetFiledVATReturn answered {NOT_FOUND} on {first} and on {last}"
        ),
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
        AuthorityCheck {
            checked_at: format!("{date}T09:00:00Z"),
            finnish_date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
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
            if first.to_string() == "2026-11-18"));
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
}
