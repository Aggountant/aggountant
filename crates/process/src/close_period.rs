//! What a `close-period` approval binds: the period's content as the user sees it.
//!
//! Closing a period locks its entries, and the user approves that from what they
//! see. [`ClosePeriodContent`] is that content: the period's vouchers, its income
//! statement, the balance sheet at its last day with per-account closing balances,
//! and the obligations still open for it. It is built from the sealed books the
//! same way the monthly report is (`Ledger::fiscal_year_data` projection,
//! `compute_statements`, `tase_erittelyt`), so its figures agree with
//! `report statements` for a month.
//!
//! `close-period preview` shows it without changing anything; the `ApproveClose`
//! gate's `snapshot_hash` covers its hash ([`ClosePeriodContent::content_hash`]),
//! so `run answer --answer approve` refuses when the books no longer say what the
//! gate showed (`CLOSE_CONTENT_CHANGED`).

use serde::Serialize;
use sha2::{Digest, Sha256};

use aggountant_domain::{
    compute_statements, kpa_policy, kpa_policy_version, tase_erittelyt, PolicySet,
    StatementProvenance, StatementRow,
};
use aggountant_ledger::{Ledger, PeriodState};
use aggountant_model::{Date, PeriodId};

use crate::close_year::map_domain;
use crate::{Obligation, ProcessEngine, ProcessError, RunView, GATE_APPROVE_CLOSE};

/// The vouchers whose posting date falls in the period.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PeriodVouchers {
    pub count: usize,
    pub first_number: Option<u64>,
    pub last_number: Option<u64>,
}

/// The period's income statement (non-zero rows) and the result since the start
/// of the fiscal year.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PeriodIncome {
    pub rows: Vec<StatementRow>,
    pub result: String,
    pub year_to_date_result: String,
}

/// One balance-sheet account's closing balance at the period's last day
/// (normal-positive).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccountBalance {
    pub account: String,
    pub account_name: String,
    pub balance: String,
}

/// The balance sheet at the period's last day: non-zero rows and the accounts
/// behind them (bank, payables, VAT accounts…).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PeriodBalance {
    pub date: Date,
    pub rows: Vec<StatementRow>,
    pub accounts: Vec<AccountBalance>,
    pub assets_total: String,
    pub equity_liabilities_total: String,
    pub balanced: bool,
    /// Present when the balances leave out the opening position because the
    /// previous fiscal year is not closed yet; says so and why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_note: Option<String>,
}

/// What closing the period means, as the user approves it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClosePeriodContent {
    pub period: String,
    pub first_date: Date,
    pub last_date: Date,
    pub vouchers: PeriodVouchers,
    pub income_statement: PeriodIncome,
    pub balance_sheet: PeriodBalance,
    /// Obligations open for the period. One blocks starting a close; at the gate
    /// the list is normally empty, and one opened meanwhile changes the hash.
    pub open_obligations: Vec<Obligation>,
    /// What the approval locks, in words.
    pub locks: String,
    /// The monthly report with the same figures and their vouchers (months only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_command: Option<String>,
}

impl ClosePeriodContent {
    /// The hash of the exact content bytes; the gate's `snapshot_hash` covers it.
    pub fn content_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("close-period content serializes");
        hex::encode(Sha256::digest(bytes))
    }
}

/// `close-period preview`: the content a close would show, and whether one can
/// start now. Changes nothing.
#[derive(Debug, Clone, Serialize)]
pub struct ClosePeriodPreview {
    pub period: String,
    pub state: PeriodState,
    pub owning_run_id: Option<String>,
    /// True when the period is Open and no obligation is open for it. The
    /// ledger's coverage rules (no months and quarters in one year, quarters only
    /// in a calendar fiscal year) are checked when the close starts.
    pub ready: bool,
    /// Why a close cannot start now (empty when `ready`).
    pub blockers: Vec<String>,
    pub content: ClosePeriodContent,
    /// Equal to the `ApproveClose` gate's `content_hash` when nothing changes
    /// between this preview and the close.
    pub content_hash: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_command: Option<String>,
}

/// The open `ApproveClose` gate with the content its hash binds.
#[derive(Debug, Clone, Serialize)]
pub struct ClosePeriodGate {
    pub gate_id: String,
    pub gate_kind: String,
    pub snapshot_hash: String,
    pub content: ClosePeriodContent,
    pub content_hash: String,
    /// True when `snapshot_hash` covers exactly this content. False for a gate an
    /// older version opened (its hash does not cover content) or when the books
    /// changed after the gate opened; `message` says which.
    pub bound: bool,
}

/// A `close-period` Run view: the ordinary [`RunView`] plus, while `ApproveClose`
/// is open, what it binds and how to answer it.
#[derive(Debug, Clone, Serialize)]
pub struct ClosePeriodRunView {
    #[serde(flatten)]
    pub run: RunView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_gate: Option<ClosePeriodGate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_command: Option<String>,
}

fn non_zero(rows: Vec<StatementRow>) -> Vec<StatementRow> {
    rows.into_iter()
        .filter(|row| row.amount != "0.00")
        .collect()
}

impl ProcessEngine {
    /// Build the period's close content from the sealed books (read-only).
    pub(crate) fn close_period_content(
        &self,
        ledger: &Ledger,
        period: &PeriodId,
    ) -> Result<ClosePeriodContent, ProcessError> {
        let (first_month, _) = period.month_bounds();
        let first_date = Date::from_ymd(period.year(), first_month.into(), 1)
            .map_err(|e| ProcessError::Ledger(e.into()))?;
        let last_date = period.last_date();
        let fiscal_year = ledger.fiscal_year_of(last_date)?;
        if !fiscal_year.contains(first_date) {
            // The ledger refuses to close such a quarter (`begin_close`); refuse it
            // here too, with the same code, because one fiscal year's statements
            // would silently leave out the quarter's part in the previous year.
            return Err(ProcessError::Ledger(
                aggountant_ledger::LedgerError::PeriodTransitionConflict(format!(
                    "period {period} ({first_date}..{last_date}) spans two fiscal years \
                     (fiscal year {} starts {}); close it by month",
                    fiscal_year.year(),
                    fiscal_year.start()
                )),
            ));
        }
        // The opening position comes from the previous fiscal year, which is
        // usually still open when its successor's first months are closed. The
        // close does not wait for the year-end close: the balances then show this
        // fiscal year's movements only, and the content says so.
        let (opening_balance, vouchers, opening_note) =
            match ledger.fiscal_year_data(fiscal_year.year()) {
                Ok(data) => (data.opening_balance, data.vouchers, None),
                Err(aggountant_ledger::LedgerError::PeriodNotSealed(reason)) => (
                    None,
                    ledger.year_vouchers(fiscal_year.year())?,
                    Some(format!(
                        "Balances exclude the opening position (movements of fiscal year {} \
                         only): {reason}",
                        fiscal_year.year()
                    )),
                ),
                Err(e) => return Err(e.into()),
            };
        let chart = ledger.chart_list(None)?;
        let provenance = StatementProvenance {
            chart_version: ledger.chart_version()?,
            policy_set: PolicySet::default().id,
            kpa_policy_version: kpa_policy_version().to_string(),
        };
        let to_date: Vec<_> = vouchers
            .iter()
            .filter(|v| v.posting_date() <= last_date)
            .cloned()
            .collect();
        let in_period: Vec<_> = to_date
            .iter()
            .filter(|v| period.contains(v.posting_date()))
            .cloned()
            .collect();

        let period_statements = compute_statements(
            &fiscal_year,
            None,
            &in_period,
            &chart,
            kpa_policy(),
            provenance.clone(),
        )
        .map_err(map_domain)?;
        let to_date_statements = compute_statements(
            &fiscal_year,
            opening_balance.as_ref(),
            &to_date,
            &chart,
            kpa_policy(),
            provenance.clone(),
        )
        .map_err(map_domain)?;
        let balances = tase_erittelyt(
            &fiscal_year,
            opening_balance.as_ref(),
            &to_date,
            &chart,
            provenance,
        )
        .map_err(map_domain)?;

        let numbers = in_period.iter().map(|v| v.number());
        let open_obligations = self.obligations().open_for_period(period)?;
        Ok(ClosePeriodContent {
            period: period.to_string(),
            first_date,
            last_date,
            vouchers: PeriodVouchers {
                count: in_period.len(),
                first_number: numbers.clone().min(),
                last_number: numbers.max(),
            },
            income_statement: PeriodIncome {
                rows: non_zero(period_statements.income),
                result: period_statements.result,
                year_to_date_result: to_date_statements.result.clone(),
            },
            balance_sheet: PeriodBalance {
                date: last_date,
                rows: non_zero(to_date_statements.balance),
                accounts: balances
                    .lines
                    .into_iter()
                    .map(|line| AccountBalance {
                        account: line.account,
                        account_name: line.account_name,
                        balance: line.closing_balance,
                    })
                    .collect(),
                assets_total: to_date_statements.assets_total,
                equity_liabilities_total: to_date_statements.equity_liabilities_total,
                balanced: to_date_statements.balanced,
                opening_note,
            },
            open_obligations,
            locks: format!(
                "Approving seals period {period}: nothing dated {first_date}..{last_date} \
                 can be recorded afterwards, a later correction goes to the next open \
                 period, and the period's VAT return is made from these figures."
            ),
            report_command: period.is_month().then(|| {
                format!("aggountant report statements --period {period} --company \"<company>\"")
            }),
        })
    }

    /// `run preview close-period`: what closing `period` would show the user, and
    /// whether a close can start now. A pure read.
    pub fn preview_close_period(
        &self,
        period: &PeriodId,
    ) -> Result<ClosePeriodPreview, ProcessError> {
        let ledger = Ledger::open(&self.home)?;
        let view = ledger.period_view(period)?;
        let content = self.close_period_content(&ledger, period)?;
        let content_hash = content.content_hash();
        let mut blockers = Vec::new();
        let mut next_command = None;
        match (view.state, view.owning_run_id.as_deref()) {
            (PeriodState::Open, _) => {}
            (PeriodState::Closing, owner) => {
                let owner = owner.unwrap_or("?");
                blockers.push(format!(
                    "period {period} is already closing under run {owner}; answer that run"
                ));
                next_command = Some(format!("aggountant run show --run-id {owner}"));
            }
            (PeriodState::Sealed, _) => {
                blockers.push(format!("period {period} is already sealed"));
            }
        }
        if view.state == PeriodState::Open && !content.open_obligations.is_empty() {
            let ids: Vec<&str> = content
                .open_obligations
                .iter()
                .map(|o| o.id.as_str())
                .collect();
            blockers.push(format!(
                "OPEN_OBLIGATION: period {period} has {} open obligation(s) [{}]; resolve them first",
                ids.len(),
                ids.join(", ")
            ));
            next_command = Some("aggountant obligation list".to_string());
        }
        let ready = blockers.is_empty();
        let message = if ready {
            next_command = Some(format!(
                "aggountant run start close-period --period {period}"
            ));
            format!(
                "Nothing changed. Starting the close moves {period} to Closing at once (no more \
                 entries) and opens ApproveClose with this content; the user approves or \
                 rejects there."
            )
        } else {
            format!(
                "Nothing changed. A close of {period} cannot start now: {}",
                blockers.join("; ")
            )
        };
        Ok(ClosePeriodPreview {
            period: period.to_string(),
            state: view.state,
            owning_run_id: view.owning_run_id,
            ready,
            blockers,
            content,
            content_hash,
            message,
            next_command,
        })
    }

    /// The Run view of a `close-period` Run with the open `ApproveClose` gate's
    /// content. Any other view passes through unchanged.
    pub fn close_period_view(&self, run: RunView) -> Result<ClosePeriodRunView, ProcessError> {
        let gate = run
            .gates
            .iter()
            .find(|g| g.gate_id == GATE_APPROVE_CLOSE && g.answered.is_none())
            .filter(|_| run.workflow == "close-period")
            .cloned();
        let Some(gate) = gate else {
            return Ok(ClosePeriodRunView {
                run,
                open_gate: None,
                message: None,
                next_command: None,
            });
        };
        let period = PeriodId::parse(&run.period).map_err(|e| {
            ProcessError::JournalCorrupt(format!("bad period {:?}: {e}", run.period))
        })?;
        let ledger = Ledger::open(&self.home)?;
        let content = self.close_period_content(&ledger, &period)?;
        let content_hash = content.content_hash();
        let bound =
            self.snapshot_hash(&ledger, &period, Some(&content_hash))? == gate.snapshot_hash;
        // A quoted placeholder: run as it is, the command is refused (INVALID_ANSWER)
        // instead of a shell reading `approve|reject` as a pipe and approving.
        let answer = format!(
            "aggountant run answer --run-id {} --gate {GATE_APPROVE_CLOSE} --answer '<approve|reject>'",
            run.run_id
        );
        let (message, next_command) = if bound {
            (
                format!(
                    "Show the user this content of {period} and answer after their decision, \
                     putting approve or reject in place of '<approve|reject>': approve seals \
                     the period permanently; reject returns it to Open and removes nothing."
                ),
                answer,
            )
        } else if self.snapshot_hash(&ledger, &period, None)? == gate.snapshot_hash {
            (
                "This gate was opened by an older version and its hash does not cover the \
                 content; the content is rebuilt from the books now. Show it to the user and \
                 answer after their decision, putting approve or reject in place of \
                 '<approve|reject>'."
                    .to_string(),
                answer,
            )
        } else {
            (
                "The books changed after the gate opened, so approval is refused \
                 (CLOSE_CONTENT_CHANGED). Reject this close and start a new one to approve the \
                 content shown here."
                    .to_string(),
                format!(
                    "aggountant run answer --run-id {} --gate {GATE_APPROVE_CLOSE} --answer reject",
                    run.run_id
                ),
            )
        };
        Ok(ClosePeriodRunView {
            open_gate: Some(ClosePeriodGate {
                gate_id: gate.gate_id,
                gate_kind: gate.gate_kind,
                snapshot_hash: gate.snapshot_hash,
                content,
                content_hash,
                bound,
            }),
            run,
            message: Some(message),
            next_command: Some(next_command),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{EventPayload, Journal};
    use crate::Seam;

    /// A gate an older version opened bound only the period lock. Such a Run
    /// still shows the rebuilt content (unbound) and can be approved.
    #[test]
    fn a_gate_opened_before_content_existed_still_closes() {
        let home = tempfile::tempdir().unwrap();
        Ledger::init(home.path()).unwrap();
        let engine = ProcessEngine::open(home.path());
        let july = PeriodId::parse("2026-07").unwrap();
        let started = engine
            .start_close_period_seam(&july, &PolicySet::default(), Seam::CrashAfterFiscalCommit)
            .unwrap();
        let run_id = started.run_id;
        let ledger = Ledger::open(home.path()).unwrap();
        let journal = Journal::new(&engine.run_dir(&run_id));
        journal
            .append(EventPayload::EffectSucceeded {
                effect_key: crate::EFFECT_TRANSITION.to_string(),
                note: "older version".into(),
            })
            .unwrap();
        journal
            .append(EventPayload::GateOpened {
                gate_id: GATE_APPROVE_CLOSE.to_string(),
                gate_kind: "Approval".to_string(),
                snapshot_hash: engine.snapshot_hash(&ledger, &july, None).unwrap(),
            })
            .unwrap();

        let view = engine
            .close_period_view(engine.show(&run_id).unwrap())
            .unwrap();
        let gate = view.open_gate.unwrap();
        assert!(!gate.bound);
        assert!(view.message.unwrap().contains("older version"));
        assert_eq!(
            view.next_command.as_deref(),
            Some(
                format!(
                    "aggountant run answer --run-id {run_id} --gate ApproveClose \
                     --answer '<approve|reject>'"
                )
                .as_str()
            )
        );

        let done = engine
            .answer(&run_id, GATE_APPROVE_CLOSE, "approve")
            .unwrap();
        assert_eq!(done.status, crate::RunStatus::Completed);
    }
}
