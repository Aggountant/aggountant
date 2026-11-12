//! Read-only monthly statement projection and presentation.
//!
//! All amounts are computed here from the same native report payload. Renderers only
//! format those values; they never perform bookkeeping arithmetic.

use std::collections::{BTreeMap, BTreeSet};

use aggountant_domain::{
    compute_statements, kpa_policy, kpa_policy_version, tase_erittelyt, RowNature, StatementKind,
    StatementProvenance, Statements,
};
use aggountant_model::{
    Account, AccountKind, Date, OpeningBalance, OpeningBalanceSource, PeriodId, VoucherBody,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{FiscalYearData, Ledger, LedgerError, PeriodState};

pub const MONTHLY_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReportCompleteness {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_declared_history_start: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_declared_limitation: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReportStatus {
    pub presentation: String,
    pub ledger_period_state: String,
    pub vat_filed: bool,
    pub user_approval: String,
    pub statutory_financial_statements: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReportProvenance {
    pub chart_version: String,
    pub policy_set: String,
    pub kpa_policy_id: String,
    pub kpa_policy_version: String,
    pub source_set_hash: String,
    pub tase_erittelyt_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_balance_source: Option<OpeningBalanceSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReportVoucherSource {
    pub voucher_number: u64,
    pub voucher_hash: String,
    pub original_document_date: Date,
    pub original_posting_date: Date,
    pub effective_posting_date: Date,
    pub chart_version: String,
    pub policy_set: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReportSourceRef {
    OpeningBalance {
        source_head_hash: String,
    },
    SealedVoucher {
        voucher_number: u64,
        voucher_hash: String,
        original_document_date: Date,
        original_posting_date: Date,
        effective_posting_date: Date,
        chart_version: String,
        policy_set: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        attachment_ref: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source_fingerprint: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReportAccount {
    pub account: String,
    pub account_name: String,
    /// Normal-positive amount for the row nature. On the synthetic current-result
    /// row, expenses are negative contributions.
    pub amount: String,
    pub sources: Vec<ReportSourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReportRow {
    pub row_id: String,
    pub label: String,
    pub nature: RowNature,
    pub amount: String,
    pub accounts: Vec<ReportAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IncomeSection {
    pub title: String,
    pub rows: Vec<ReportRow>,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BalanceSection {
    pub title: String,
    pub rows: Vec<ReportRow>,
    pub assets_total: String,
    pub equity_liabilities_total: String,
    pub balanced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonthlyReportSections {
    pub month_income: IncomeSection,
    pub year_to_date_income: IncomeSection,
    pub balance: BalanceSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonthlyStatementsReport {
    pub schema_version: u32,
    pub report_type: String,
    pub company: String,
    pub period: PeriodId,
    pub statement_date: Date,
    pub currency: String,
    pub status: ReportStatus,
    pub completeness: ReportCompleteness,
    pub provenance: ReportProvenance,
    /// Exact sealed identities in the pinned report source set. Projection-only date
    /// attribution is separate and never replaces the original voucher identity.
    pub sources: Vec<ReportVoucherSource>,
    pub sections: MonthlyReportSections,
}

#[derive(Debug, Clone, Default)]
pub struct MonthlyReportOptions {
    pub history_start: Option<Date>,
    pub limitation: Option<String>,
}

fn decimal_string(mut value: Decimal) -> String {
    value.rescale(2);
    value.to_string()
}

fn account_normal(kind: AccountKind, signed: Decimal) -> Decimal {
    match kind {
        AccountKind::Asset | AccountKind::Expense => signed,
        AccountKind::Income | AccountKind::Liability | AccountKind::Equity => -signed,
    }
}

fn row_for(
    kind: AccountKind,
    account: &str,
) -> Result<&'static aggountant_domain::KpaRow, LedgerError> {
    let statement = match kind {
        AccountKind::Income | AccountKind::Expense => StatementKind::Income,
        _ => StatementKind::Balance,
    };
    kpa_policy()
        .rows
        .iter()
        .filter(|row| row.statement == statement)
        .filter_map(|row| {
            row.accounts
                .iter()
                .filter(|prefix| account.starts_with(prefix.as_str()))
                .map(String::len)
                .max()
                .map(|length| (row, length))
        })
        .max_by_key(|(_, length)| *length)
        .map(|(row, _)| row)
        .ok_or_else(|| LedgerError::Store(format!("account {account} maps to no KPA report row")))
}

fn month_end(period: &PeriodId) -> Result<Date, LedgerError> {
    let month = period.month().ok_or_else(|| {
        LedgerError::Model(aggountant_model::ModelError::InvalidPeriodId(format!(
            "monthly statements require YYYY-MM, got {period}"
        )))
    })?;
    let day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if period.year() % 400 == 0 || (period.year() % 4 == 0 && period.year() % 100 != 0) => 29,
        2 => 28,
        _ => unreachable!(),
    };
    Date::from_ymd(period.year(), month.into(), day).map_err(Into::into)
}

fn source_ref(source: &ReportVoucherSource) -> ReportSourceRef {
    ReportSourceRef::SealedVoucher {
        voucher_number: source.voucher_number,
        voucher_hash: source.voucher_hash.clone(),
        original_document_date: source.original_document_date,
        original_posting_date: source.original_posting_date,
        effective_posting_date: source.effective_posting_date,
        chart_version: source.chart_version.clone(),
        policy_set: source.policy_set.clone(),
        attachment_ref: source.attachment_ref.clone(),
        source_fingerprint: source.source_fingerprint.clone(),
    }
}

fn traced_rows(
    statement: StatementKind,
    statements: &Statements,
    opening: Option<&OpeningBalance>,
    vouchers: &[VoucherBody],
    chart: &[Account],
    sources: &BTreeMap<u64, ReportVoucherSource>,
) -> Result<Vec<ReportRow>, LedgerError> {
    let account_info: BTreeMap<&str, (&str, AccountKind)> = chart
        .iter()
        .map(|account| (account.id.as_str(), (account.name.as_str(), account.kind)))
        .collect();
    let mut signed: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut refs: BTreeMap<String, BTreeSet<ReportSourceRef>> = BTreeMap::new();
    if let Some(opening) = opening {
        let opening_ref = ReportSourceRef::OpeningBalance {
            source_head_hash: opening.source().source_head_hash().to_hex(),
        };
        for posting in opening.postings() {
            *signed
                .entry(posting.account.as_str().to_string())
                .or_default() += posting.signed_amount();
            refs.entry(posting.account.as_str().to_string())
                .or_default()
                .insert(opening_ref.clone());
        }
    }
    for voucher in vouchers {
        let source = sources.get(&voucher.number()).ok_or_else(|| {
            LedgerError::Store(format!(
                "report source identity missing for voucher {}",
                voucher.number()
            ))
        })?;
        for posting in voucher.postings() {
            *signed
                .entry(posting.account.as_str().to_string())
                .or_default() += posting.signed_amount();
            refs.entry(posting.account.as_str().to_string())
                .or_default()
                .insert(source_ref(source));
        }
    }

    let statement_amounts: BTreeMap<&str, (&str, RowNature)> = match statement {
        StatementKind::Income => statements.income.iter(),
        StatementKind::Balance => statements.balance.iter(),
    }
    .map(|row| (row.row_id.as_str(), (row.amount.as_str(), row.nature)))
    .collect();

    let mut accounts_by_row: BTreeMap<String, Vec<ReportAccount>> = BTreeMap::new();
    for (account, sum) in signed {
        let (name, kind) = account_info.get(account.as_str()).copied().ok_or_else(|| {
            LedgerError::Store(format!("account {account} is absent from active chart"))
        })?;
        let account_statement = if matches!(kind, AccountKind::Income | AccountKind::Expense) {
            StatementKind::Income
        } else {
            StatementKind::Balance
        };
        if account_statement != statement {
            if statement == StatementKind::Balance && account_statement == StatementKind::Income {
                let mut contribution = account_normal(kind, sum);
                if kind == AccountKind::Expense {
                    contribution = -contribution;
                }
                accounts_by_row
                    .entry("BS-TILIKAUDEN-TULOS".into())
                    .or_default()
                    .push(ReportAccount {
                        account: account.clone(),
                        account_name: name.to_string(),
                        amount: decimal_string(contribution),
                        sources: refs
                            .remove(&account)
                            .unwrap_or_default()
                            .into_iter()
                            .collect(),
                    });
            }
            continue;
        }
        let row = row_for(kind, &account)?;
        accounts_by_row
            .entry(row.row_id.clone())
            .or_default()
            .push(ReportAccount {
                account: account.clone(),
                account_name: name.to_string(),
                amount: decimal_string(account_normal(kind, sum)),
                sources: refs
                    .remove(&account)
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
            });
    }

    // Every policy row is emitted, including zero rows. This keeps empty and sparse
    // months explicit rather than silently changing the report shape.
    let mut policy_rows: Vec<_> = kpa_policy()
        .rows
        .iter()
        .filter(|row| row.statement == statement)
        .collect();
    policy_rows.sort_by_key(|row| (row.order, row.row_id.as_str()));
    let mut rows = Vec::new();
    for row in policy_rows {
        let amount = statement_amounts
            .get(row.row_id.as_str())
            .map(|(a, _)| *a)
            .unwrap_or("0.00");
        rows.push(ReportRow {
            row_id: row.row_id.clone(),
            label: row.label.clone(),
            nature: row.nature,
            amount: amount.to_string(),
            accounts: accounts_by_row.remove(&row.row_id).unwrap_or_default(),
        });
    }
    if statement == StatementKind::Balance {
        let current = statement_amounts
            .get("BS-TILIKAUDEN-TULOS")
            .map(|(a, _)| *a)
            .unwrap_or("0.00");
        let mut result_accounts = accounts_by_row
            .remove("BS-TILIKAUDEN-TULOS")
            .unwrap_or_default();
        result_accounts.sort_by(|a, b| a.account.cmp(&b.account));
        rows.push(ReportRow {
            row_id: "BS-TILIKAUDEN-TULOS".into(),
            label: "Tilikauden tulos".into(),
            nature: RowNature::EquityLiabilities,
            amount: current.to_string(),
            accounts: result_accounts,
        });
    }
    Ok(rows)
}

impl Ledger {
    /// Produce month income, YTD income and month-end balance from sealed vouchers.
    /// This method is read-only with respect to every ledger store.
    pub fn monthly_statements_report(
        &self,
        period: &PeriodId,
        company: &str,
        options: MonthlyReportOptions,
    ) -> Result<MonthlyStatementsReport, LedgerError> {
        if !period.is_month() {
            return Err(LedgerError::Model(
                aggountant_model::ModelError::InvalidPeriodId(format!(
                    "monthly statements require YYYY-MM, got {period}"
                )),
            ));
        }
        if company.trim().is_empty() {
            return Err(LedgerError::Store(
                "company display name must not be empty".into(),
            ));
        }
        if options
            .limitation
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(LedgerError::Store(
                "history limitation must not be empty".into(),
            ));
        }
        let end = month_end(period)?;
        if options.history_start.is_some_and(|start| start > end) {
            return Err(LedgerError::Model(
                aggountant_model::ModelError::InvalidDate(format!(
                    "history start must not be after statement date {end}"
                )),
            ));
        }
        // Reports are evidence over the sealed ledger, so local row consistency is not
        // enough: refuse output unless the complete fiscal hash chain verifies.
        self.verify()?;
        // The year-to-date runs from the start of the fiscal year containing the month,
        // which for a non-calendar fiscal year is not 1 January.
        let year = self.fiscal_year_of(end)?.year();
        let FiscalYearData {
            fiscal_year,
            opening_balance,
            vouchers,
        } = self.fiscal_year_data(year)?;
        let chart = self.chart_list(None)?;
        let chart_version = self.chart_version()?;
        let policy_set = aggountant_domain::PolicySet::default().id;
        let provenance = StatementProvenance {
            chart_version: chart_version.clone(),
            policy_set: policy_set.clone(),
            kpa_policy_version: kpa_policy_version().to_string(),
        };
        let ytd_vouchers: Vec<_> = vouchers
            .iter()
            .filter(|voucher| voucher.posting_date() <= end)
            .cloned()
            .collect();
        let month_vouchers: Vec<_> = ytd_vouchers
            .iter()
            .filter(|voucher| period.contains(voucher.posting_date()))
            .cloned()
            .collect();

        let month = compute_statements(
            &fiscal_year,
            None,
            &month_vouchers,
            &chart,
            kpa_policy(),
            provenance.clone(),
        )?;
        let ytd = compute_statements(
            &fiscal_year,
            opening_balance.as_ref(),
            &ytd_vouchers,
            &chart,
            kpa_policy(),
            provenance.clone(),
        )?;
        let balance_specification = tase_erittelyt(
            &fiscal_year,
            opening_balance.as_ref(),
            &ytd_vouchers,
            &chart,
            provenance.clone(),
        )?;

        let mut source_map = BTreeMap::new();
        for projected in &ytd_vouchers {
            let original = self.voucher(projected.number())?;
            source_map.insert(
                projected.number(),
                ReportVoucherSource {
                    voucher_number: original.number,
                    voucher_hash: original.voucher_hash,
                    original_document_date: original.body.document_date(),
                    original_posting_date: original.body.posting_date(),
                    effective_posting_date: projected.posting_date(),
                    chart_version: original.body.chart_version().to_string(),
                    policy_set: original.provenance.policy_set,
                    attachment_ref: original.body.attachment_ref().map(|hash| hash.to_hex()),
                    source_fingerprint: original.provenance.source_fingerprint,
                },
            );
        }
        let sources: Vec<_> = source_map.values().cloned().collect();
        let source_preimage = serde_json::to_vec(&(
            opening_balance.as_ref().map(OpeningBalance::source),
            &sources,
        ))
        .map_err(|error| LedgerError::Store(error.to_string()))?;
        let source_set_hash = hex::encode(Sha256::digest(source_preimage));
        let period_view = self.period_view(period)?;
        let presentation = if period_view.state == PeriodState::Sealed {
            "closed"
        } else {
            "draft"
        };

        Ok(MonthlyStatementsReport {
            schema_version: MONTHLY_REPORT_SCHEMA_VERSION,
            report_type: "monthly_statements".into(),
            company: company.to_string(),
            period: period.clone(),
            statement_date: end,
            currency: "EUR".into(),
            status: ReportStatus {
                presentation: presentation.into(),
                ledger_period_state: period_view.state.as_str().to_lowercase(),
                vat_filed: period_view.vat_filed,
                user_approval: "not_asserted".into(),
                statutory_financial_statements: false,
            },
            completeness: ReportCompleteness {
                state: "unverified".into(),
                caller_declared_history_start: options.history_start,
                caller_declared_limitation: options.limitation,
                notes: vec![
                    "Raportti ei itsessään todista, että koko aiempi kirjanpitohistoria on mukana.".into(),
                    "Kauden sulkeminen ei ole näyttö käyttäjän hyväksynnästä tai viranomaisilmoituksesta.".into(),
                ],
            },
            provenance: ReportProvenance {
                chart_version,
                policy_set,
                kpa_policy_id: kpa_policy().policy.clone(),
                kpa_policy_version: kpa_policy_version().to_string(),
                source_set_hash,
                tase_erittelyt_hash: balance_specification.statements_hash(),
                opening_balance_source: opening_balance.as_ref().map(|opening| opening.source().clone()),
            },
            sources,
            sections: MonthlyReportSections {
                month_income: IncomeSection {
                    title: "Tuloslaskelma, kuukausi".into(),
                    rows: traced_rows(StatementKind::Income, &month, None, &month_vouchers, &chart, &source_map)?,
                    result: month.result,
                },
                year_to_date_income: IncomeSection {
                    title: "Tuloslaskelma, tilikauden alusta".into(),
                    rows: traced_rows(StatementKind::Income, &ytd, None, &ytd_vouchers, &chart, &source_map)?,
                    result: ytd.result.clone(),
                },
                balance: BalanceSection {
                    title: "Tase kauden lopussa".into(),
                    rows: traced_rows(StatementKind::Balance, &ytd, opening_balance.as_ref(), &ytd_vouchers, &chart, &source_map)?,
                    assets_total: ytd.assets_total,
                    equity_liabilities_total: ytd.equity_liabilities_total,
                    balanced: ytd.balanced,
                },
            },
        })
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn fi_amount(value: &str) -> String {
    let (negative, raw) = value
        .strip_prefix('-')
        .map_or((false, value), |rest| (true, rest));
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, "00"));
    let mut grouped = String::new();
    for (index, ch) in whole.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(' ');
        }
        grouped.push(ch);
    }
    let whole: String = grouped.chars().rev().collect();
    format!("{}{},{}", if negative { "−" } else { "" }, whole, fraction)
}

fn source_html(source: &ReportSourceRef) -> String {
    match source {
        ReportSourceRef::OpeningBalance { source_head_hash } => format!(
            "<a href=\"#source-opening\">Alkusaldo</a>, lähdehash <code>{}</code>",
            escape_html(source_head_hash)
        ),
        ReportSourceRef::SealedVoucher {
            voucher_number,
            voucher_hash,
            original_document_date,
            original_posting_date,
            effective_posting_date,
            chart_version,
            policy_set,
            attachment_ref,
            source_fingerprint,
        } => {
            let mut text = format!("<a href=\"#source-voucher-{voucher_number}\">Tosite #{voucher_number}</a>, asiakirjapäivä {original_document_date}, alkuperäinen kirjauspäivä {original_posting_date}, raporttipäivä {effective_posting_date}, hash <code>{}</code>, kartta <code>{}</code>, policy <code>{}</code>", escape_html(voucher_hash), escape_html(chart_version), escape_html(policy_set));
            if let Some(reference) = attachment_ref {
                text.push_str(&format!(", lähde <code>{}</code>", escape_html(reference)));
            }
            if let Some(fingerprint) = source_fingerprint {
                text.push_str(&format!(
                    ", lähdesormenjälki <code>{}</code>",
                    escape_html(fingerprint)
                ));
            }
            text
        }
    }
}

fn rows_html(rows: &[ReportRow], currency: &str) -> String {
    let mut out = String::new();
    for row in rows {
        let id = format!("row-{}", row.row_id.to_ascii_lowercase());
        out.push_str(&format!(
            "<tr id=\"{}\"><th scope=\"row\"><details><summary>{}</summary>",
            escape_html(&id),
            escape_html(&row.label)
        ));
        if row.accounts.is_empty() {
            out.push_str("<p>Ei tilisaldoa tällä raporttijaksolla.</p>");
        }
        for account in &row.accounts {
            out.push_str(&format!("<details class=\"account\" id=\"account-{}-{}\"><summary><span class=\"account-name\">{} {}</span> <span class=\"amount\">{} {}</span></summary><ul>", escape_html(&row.row_id.to_ascii_lowercase()), escape_html(&account.account), escape_html(&account.account), escape_html(&account.account_name), fi_amount(&account.amount), escape_html(currency)));
            for source in &account.sources {
                out.push_str(&format!("<li>{}</li>", source_html(source)));
            }
            out.push_str("</ul></details>");
        }
        out.push_str(&format!(
            "</details></th><td class=\"amount\">{} {}</td></tr>",
            fi_amount(&row.amount),
            escape_html(currency)
        ));
    }
    out
}

/// Render standalone Finnish HTML from the native payload without recomputation.
pub fn render_monthly_report_html(report: &MonthlyStatementsReport) -> String {
    let mut limitation = String::new();
    if let Some(start) = report.completeness.caller_declared_history_start {
        limitation.push_str(&format!("<p>Kutsujan ilmoittama historian saatavuuden alkupäivä (varmistamaton, ei laskennan rajaus): {start}</p>"));
    }
    if let Some(note) = &report.completeness.caller_declared_limitation {
        limitation.push_str(&format!("<p>Ilmoitettu rajaus: {}</p>", escape_html(note)));
    }
    let notes = report
        .completeness
        .notes
        .iter()
        .map(|note| format!("<li>{}</li>", escape_html(note)))
        .collect::<String>();
    let section = |title: &str, rows: &[ReportRow], totals: &[(&str, &str)]| {
        let totals = totals
            .iter()
            .map(|(label, total)| {
                format!(
                    "<tr class=\"total\"><th>{}</th><td class=\"amount\">{} {}</td></tr>",
                    escape_html(label),
                    fi_amount(total),
                    escape_html(&report.currency)
                )
            })
            .collect::<String>();
        format!("<section><h2>{}</h2><table><thead><tr><th>Rivi ja tarkastusjälki</th><th>Summa</th></tr></thead><tbody>{}{}</tbody></table></section>", escape_html(title), rows_html(rows, &report.currency), totals)
    };
    let balance = &report.sections.balance;
    let mut source_register = String::new();
    if let Some(opening) = &report.provenance.opening_balance_source {
        source_register.push_str(&format!(
            "<li id=\"source-opening\"><strong>Alkusaldo</strong>: <code>{}</code></li>",
            escape_html(&serde_json::to_string(opening).expect("opening source serializes"))
        ));
    }
    for source in &report.sources {
        let attachment = source
            .attachment_ref
            .as_ref()
            .map(|value| format!(", lähde <code>{}</code>", escape_html(value)))
            .unwrap_or_default();
        let fingerprint = source
            .source_fingerprint
            .as_ref()
            .map(|value| format!(", lähdesormenjälki <code>{}</code>", escape_html(value)))
            .unwrap_or_default();
        source_register.push_str(&format!(
            "<li id=\"source-voucher-{}\"><strong>Tosite #{}</strong>: hash <code>{}</code>, asiakirjapäivä {}, alkuperäinen kirjauspäivä {}, raporttipäivä {}, kartta <code>{}</code>, policy <code>{}</code>{}{}</li>",
            source.voucher_number,
            source.voucher_number,
            escape_html(&source.voucher_hash),
            source.original_document_date,
            source.original_posting_date,
            source.effective_posting_date,
            escape_html(&source.chart_version),
            escape_html(&source.policy_set),
            attachment,
            fingerprint
        ));
    }
    if source_register.is_empty() {
        source_register.push_str("<li>Ei sinetöityjä tositteita raportin rajauksessa.</li>");
    }
    format!(
        r#"<!doctype html><html lang="fi"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{company} – kuukausiraportti {period}</title><style>:root{{font-family:system-ui,-apple-system,sans-serif;color:#18212b}}body{{max-width:1100px;margin:2rem auto;padding:0 1rem}}h1{{overflow-wrap:anywhere}}.meta{{display:grid;grid-template-columns:repeat(auto-fit,minmax(16rem,1fr));gap:.5rem;background:#f2f5f7;padding:1rem}}.notice{{border-left:.35rem solid #a45b00;padding:.7rem 1rem;background:#fff7e8}}table{{border-collapse:collapse;width:100%;table-layout:fixed;margin-bottom:2rem}}th,td{{border-bottom:1px solid #ccd4da;padding:.55rem;text-align:left;vertical-align:top;overflow-wrap:anywhere}}th:first-child{{width:72%}}.amount{{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}}summary{{cursor:pointer}}.account{{margin:.5rem 0 .5rem 1rem}}code{{overflow-wrap:anywhere}}.total{{font-weight:700;border-top:2px solid #566}}@media(max-width:600px){{th:first-child{{width:58%}}.account-name,.account .amount{{display:block;text-align:left;white-space:normal}}}}@media print{{body{{max-width:none;margin:0}}details{{display:block}}details>summary{{list-style:none}}details>*{{display:block!important}}section{{break-before:auto;break-inside:avoid-page}}a{{color:inherit}}}}</style></head><body><header><h1>{company}</h1><p>Kuukausiraportti {period}</p></header><div class="meta"><div><strong>Tilannepäivä</strong><br>{date}</div><div><strong>Tila</strong><br>{presentation} (ledger: {ledger_state})</div><div><strong>Valuutta</strong><br>{currency}</div><div><strong>Lähdejoukon hash</strong><br><code>{source_hash}</code></div><div><strong>Tase-erittelyjen hash</strong><br><code>{tase_hash}</code></div><div><strong>Kartta</strong><br><code>{chart}</code></div><div><strong>Policy</strong><br><code>{policy} / {kpa}@{kpa_version}</code></div></div><aside class="notice"><h2>Kattavuus: varmistamaton</h2>{limitation}<ul>{notes}</ul><p>Tämä ei ole väite lakisääteisestä tilinpäätöksestä. Käyttäjän hyväksyntää ei ole väitetty. ALV-ilmoitus kirjattu: {vat_filed}.</p></aside>{month}{ytd}{balance_section}<section><h2>Lähdejoukko</h2><ul>{source_register}</ul></section><footer><p>Raporttiskeema {schema}. Tase täsmää: {balanced}. Varat yhteensä {assets} {currency}; oma pääoma ja velat yhteensä {equity} {currency}.</p></footer></body></html>"#,
        company = escape_html(&report.company),
        period = report.period,
        date = report.statement_date,
        presentation = escape_html(&report.status.presentation),
        ledger_state = escape_html(&report.status.ledger_period_state),
        currency = escape_html(&report.currency),
        source_hash = escape_html(&report.provenance.source_set_hash),
        tase_hash = escape_html(&report.provenance.tase_erittelyt_hash),
        chart = escape_html(&report.provenance.chart_version),
        policy = escape_html(&report.provenance.policy_set),
        kpa = escape_html(&report.provenance.kpa_policy_id),
        kpa_version = escape_html(&report.provenance.kpa_policy_version),
        limitation = limitation,
        notes = notes,
        source_register = source_register,
        vat_filed = if report.status.vat_filed {
            "kyllä"
        } else {
            "ei"
        },
        month = section(
            &report.sections.month_income.title,
            &report.sections.month_income.rows,
            &[("Kuukauden tulos", &report.sections.month_income.result)]
        ),
        ytd = section(
            &report.sections.year_to_date_income.title,
            &report.sections.year_to_date_income.rows,
            &[(
                "Tilikauden tulos",
                &report.sections.year_to_date_income.result
            )]
        ),
        balance_section = section(
            &balance.title,
            &balance.rows,
            &[
                ("Vastaavaa yhteensä", &balance.assets_total),
                ("Vastattavaa yhteensä", &balance.equity_liabilities_total),
            ]
        ),
        schema = report.schema_version,
        balanced = if balance.balanced { "kyllä" } else { "ei" },
        assets = fi_amount(&balance.assets_total),
        equity = fi_amount(&balance.equity_liabilities_total)
    )
}
