//! The authority's answer to a filing effect, read back from local records for
//! `run show` and the approve commands. Offline and read-only: the receipt is the
//! one the Run or the Ledger already recorded, never a fresh query.
//!
//! An agent recording a filing needs the authority's receipt identifier, and
//! without this it went digging in the Run's journal files. The journal note of an
//! accepted Incomes Register effect is the broker's receipt string; a VAT filing's
//! receipt is read from the Ledger, which holds it even when a resume adopted the
//! filing from fiscal state and the note does not carry it.

use serde::Serialize;

use aggountant_ledger::Ledger;
use aggountant_model::PeriodId;

use crate::{
    file_employer_report, file_vat, EffectStatus, EffectView, Fold, ProcessEngine, ProcessError,
};

use crate::file_employer_report::EmployerReportReceipt;

/// What the authority answered to a filing effect, as far as local records know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AuthorityOutcome {
    /// The authority accepted the filing (for the Incomes Register: the delivery
    /// was processed as valid). `receipt` is what it returned; `None` when the
    /// filing was adopted without one, and `next_command` then says where to look.
    Accepted {
        receipt: Option<AuthorityReceipt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        next_command: Option<String>,
    },
    /// Sent, but no definitive answer is recorded. It may have been received;
    /// nothing may be sent again before `next_command` settles it.
    OutcomeUnknown {
        next_command: Option<String>,
        message: String,
    },
    /// The authority refused the filing.
    Rejected { error_code: String },
}

/// The receipt identifiers an authority returned for an accepted filing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "authority", rename_all = "snake_case")]
pub enum AuthorityReceipt {
    /// Vero API `FileVATReturn`: `UniqueIdentifier` and its receipt time.
    Vero {
        unique_identifier: String,
        accepted_timestamp: String,
    },
    /// Incomes Register: `IRDeliveryId`, the report's `IRItemId` (`ir_report_id`)
    /// and its version, from processing feedback (`source: api`) or from e-service
    /// evidence a person gave (`source: e_service`).
    Tulorekisteri(EmployerReportReceipt),
}

impl ProcessEngine {
    /// Fill in the authority outcome of every filing effect of a folded Run.
    pub(crate) fn authority_outcomes(
        &self,
        f: &Fold,
        effects: &mut [EffectView],
    ) -> Result<(), ProcessError> {
        for effect in effects {
            effect.outcome = self.authority_outcome(f, effect)?;
        }
        Ok(())
    }

    fn authority_outcome(
        &self,
        f: &Fold,
        effect: &EffectView,
    ) -> Result<Option<AuthorityOutcome>, ProcessError> {
        let run_id = &f.run_id;
        let (reconcile, unknown): (String, fn(&str) -> bool) =
            match (f.workflow.as_str(), effect.key.as_str()) {
                ("file-vat", file_vat::EFFECT_SUBMIT_FILING) => (
                    format!("aggountant file-vat reconcile --run-id {run_id}"),
                    file_vat::filing_outcome_may_be_accepted,
                ),
                ("file-wage", crate::file_wage::EFFECT) => {
                    (format!("aggountant file-wage reconcile {run_id}"), |code| {
                        code == crate::file_wage::UNKNOWN_CODE
                    })
                }
                (file_employer_report::WORKFLOW_NAME, file_employer_report::EFFECT) => (
                    format!(
                        "aggountant file-wage employer-report status --month {}",
                        f.period
                    ),
                    |code| code == file_employer_report::UNKNOWN_CODE,
                ),
                _ => return Ok(None),
            };
        let vat = f.workflow == "file-vat";
        let unsettled = |next_command: Option<String>, message: &str| {
            Some(AuthorityOutcome::OutcomeUnknown {
                next_command,
                message: message.to_string(),
            })
        };
        Ok(match effect.status {
            EffectStatus::Succeeded if vat => Some(self.vat_accepted(f)?),
            EffectStatus::Succeeded => {
                let note = f.notes.get(&effect.key).map(String::as_str).unwrap_or("");
                match incomes_register_receipt(note) {
                    Some(receipt) => Some(AuthorityOutcome::Accepted {
                        receipt: Some(AuthorityReceipt::Tulorekisteri(receipt)),
                        next_command: None,
                    }),
                    // An acceptance without readable references settles nothing; the
                    // employer's report reads such a Run the same way.
                    None => unsettled(
                        Some(reconcile),
                        "recorded as accepted, but without readable Incomes Register \
                         references; do not send again before the status query settles it",
                    ),
                }
            }
            EffectStatus::Rejected => {
                let code = f.error_codes.get(&effect.key).cloned().unwrap_or_default();
                if unknown(&code) {
                    unsettled(
                        Some(reconcile),
                        "sent without a definitive answer; it may have been received. Do not \
                         send again before the reconcile command settles it",
                    )
                } else {
                    Some(AuthorityOutcome::Rejected { error_code: code })
                }
            }
            // Dispatched, and the Run stopped before an answer reached the journal.
            EffectStatus::Executing if vat => unsettled(
                Some(format!("aggountant file-vat status --period {}", f.period)),
                "the filing was dispatched but the Run was interrupted before its outcome \
                 was recorded; it may have been accepted. Do not file again",
            ),
            EffectStatus::Executing => unsettled(
                Some(reconcile),
                "dispatched, but the Run stopped before an answer was recorded; it may have \
                 been received. Do not send again before the reconcile command settles it",
            ),
            EffectStatus::Planned | EffectStatus::Skipped | EffectStatus::Waiting => None,
        })
    }

    /// A succeeded VAT filing: the receipt the Ledger recorded for the period. The
    /// Ledger files a period once, so its receipt belongs to this filing.
    fn vat_accepted(&self, f: &Fold) -> Result<AuthorityOutcome, ProcessError> {
        let period = PeriodId::parse(&f.period)
            .map_err(|e| ProcessError::JournalCorrupt(format!("bad period {:?}: {e}", f.period)))?;
        let receipt = Ledger::open(&self.home)?.vat_filing_receipt(&period)?;
        Ok(match receipt {
            Some(r) => AuthorityOutcome::Accepted {
                receipt: Some(AuthorityReceipt::Vero {
                    unique_identifier: r.unique_identifier,
                    accepted_timestamp: r.accepted_timestamp,
                }),
                next_command: None,
            },
            // `file-vat reconcile` adopted a processed return: there is no receipt.
            None => AuthorityOutcome::Accepted {
                receipt: None,
                next_command: Some(format!("aggountant file-vat status --period {period}")),
            },
        })
    }
}

/// The Incomes Register references in an accepted effect's journal note. A
/// travel-expense report completed from e-service evidence records only the
/// delivery and report ids (`manual-e-service:<delivery>/<report>`), without a
/// version.
fn incomes_register_receipt(note: &str) -> Option<EmployerReportReceipt> {
    EmployerReportReceipt::parse(note).or_else(|| {
        let rest = note.strip_prefix("manual-e-service:")?;
        (rest.split('/').count() == 2)
            .then(|| EmployerReportReceipt::parse(&format!("{note}/-")))
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &str = "11111111222233334444555555555555";
    const R: &str = "66666666777788889999000000000000";

    #[test]
    fn reads_api_and_manual_incomes_register_notes() {
        let api = incomes_register_receipt(&format!("{D}/{R}/1")).expect("api note");
        assert_eq!(api.ir_delivery_id.as_deref(), Some(D));
        assert_eq!(api.ir_report_id, R);
        assert_eq!(api.version, Some(1));
        assert_eq!(api.source, "api");

        let wage_manual =
            incomes_register_receipt(&format!("manual-e-service:{D}/{R}")).expect("wage manual");
        assert_eq!(wage_manual.ir_delivery_id.as_deref(), Some(D));
        assert_eq!(wage_manual.version, None);
        assert_eq!(wage_manual.source, "e_service");

        assert!(incomes_register_receipt("filed").is_none());
        assert!(incomes_register_receipt(&format!("{D}/{R}")).is_none());
    }
}
