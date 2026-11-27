//! Deterministic camt.053 (ISO 20022 `BankToCustomerStatement`) and camt.052
//! (`BankToCustomerAccountReport`) reader — the
//! Stage-1 parser of the manual tosite-loading pipeline. Each `Ntry`
//! becomes one canonical [`BankTransaction`] row: amount + direction + dates +
//! counterparty + the bank's own reference, all preserved verbatim.
//!
//! It is a **pure, bounded, golden-testable** transform. It carries only the
//! decisions that are *facts about the format* (which date is the accounting
//! date; which party is the counterparty for a given direction) — it does **not**
//! assign ledger accounts or VAT codes (that is the skill's judgment, Stage 3;
//! `phase4-imports` must-spec #2).
//!
//! Element lookup is by **local name**, ignoring the XML namespace, so the same
//! reader handles camt.053.001.02 (Holvi's export) and later minor revisions
//! without a schema-version branch.
//!
//! It also accepts **camt.052** (`BankToCustomerAccountReport`) — Holvi's
//! self-service "Electronic account statement" export produces .052, not .053.
//! For the field *subset this parser reads*, the two formats share the same
//! element paths and differ only in the container element (camt.053
//! `BkToCstmrStmt → Stmt`; camt.052 `BkToCstmrAcctRpt → Rpt`), so the same
//! per-`Ntry` reduction runs for both. This is a structural claim about the read
//! subset, not full message equivalence: camt.052 is an interim/intraday report
//! and can carry semantics .053 does not.
//!
//! Beyond those fields it passes through, as typed optional fields, the source
//! data that helps the skill judge an entry: the bank transaction code
//! (`BkTxCd`), reversal and return information (`RvslInd`, `RtrInf`), charges
//! (`Chrgs`) and the instructed amount of a foreign-currency payment
//! (`AmtDtls/InstdAmt`). These fields never refuse a file and never enter the
//! entry's event key, source fingerprint or default description: a statement
//! the parser read before they existed reads to the same identity now, so an
//! overlapping re-import of it is still a duplicate observation. An element
//! that cannot be read into its typed form is left out rather than refused,
//! because the build before these fields accepted the file and its retained
//! copy must still read back.

use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use aggountant_model::{Currency, Date, Money};

use crate::error::ImportError;

/// Direction of a bank entry, from ISO 20022 `CdtDbtInd`. `Credit` = money into
/// the account (`CRDT`); `Debit` = money out (`DBIT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Credit,
    Debit,
}

/// One `Ntry` reduced to canonical, account-free voucher-intent fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BankTransaction {
    /// The transaction's event key and the voucher's natural key
    /// (`bank-import:holvi:<iban>:<ref>`). Stable across re-exported / overlapping
    /// statement ranges, because the uniqueness-bearing part is the bank's own
    /// per-entry reference — so the Ledger's bank batch finds the transaction
    /// already booked from another export.
    pub natural_key: String,
    /// The bank's own reference for this entry (`AcctSvcrRef`, else `EndToEndId`),
    /// preserved verbatim. The uniqueness-bearing part of `natural_key`, which is
    /// the transaction's event key: the same transaction in another export of the
    /// same account has the same key.
    pub bank_ref: String,
    /// `BookgDt` — the date the bank booked the entry (kirjauspäivä).
    pub booking_date: Date,
    /// `ValDt` — the value date (arvopäivä), when present. Retained but not the
    /// accounting date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_date: Option<Date>,
    /// The KPL-correct accounting/posting date. For a bank statement this is the
    /// **booking date** (`BookgDt`): kirjauspäivä is when the entry hits the
    /// account, whereas `ValDt` (arvopäivä) governs only interest accrual.
    /// VERIFY(KPL 2:4 / KILA) — the choice misfiles the fiscal year / VAT period
    /// if wrong, so it is pinned here rather than left to the caller.
    pub accounting_date: Date,
    /// Credit (money in) or Debit (money out), from `CdtDbtInd`.
    pub direction: Direction,
    /// The entry amount, always positive; the sign lives in `direction`. Currency
    /// is read from the `Amt@Ccy` attribute.
    pub amount: Money,
    /// The counterparty name: the creditor (`Cdtr`) for a Debit, the debtor
    /// (`Dbtr`) for a Credit — i.e. the *other* party, whichever direction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty: Option<String>,
    /// Unstructured remittance information (`RmtInf/Ustrd`), joined with spaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The bank transaction code (`BkTxCd`) of the entry, else of its
    /// transaction details: what kind of transaction the bank says it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_code: Option<BankTransactionCode>,
    /// Present when the entry reverses an earlier one (`RvslInd` true) or
    /// returns a payment (`RtrInf`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reversal: Option<Reversal>,
    /// The charges the bank reports for the entry (`Chrgs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charges: Option<Charges>,
    /// The amount the payment was instructed in (`AmtDtls/InstdAmt`), for
    /// example the original currency of a foreign-currency card purchase. The
    /// entry `amount` is what moved on the account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructed_amount: Option<Money>,
}

/// An ISO 20022 bank transaction code (`BkTxCd`): the structured domain code,
/// the bank's proprietary code, or both, verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BankTransactionCode {
    /// `Domn`: domain, family and sub-family, e.g. `PMNT` / `CCRD` / `POSD`
    /// for a card purchase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<DomainCode>,
    /// `Prtry`: the bank's or a national scheme's own code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<ProprietaryCode>,
}

/// The structured part of a bank transaction code (`BkTxCd/Domn`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DomainCode {
    /// `Domn/Cd`, e.g. `PMNT`.
    pub domain: String,
    /// `Domn/Fmly/Cd`, e.g. `CCRD`, `ICDT`, `RCDT`.
    pub family: String,
    /// `Domn/Fmly/SubFmlyCd`, e.g. `POSD`, `ESCT`, `RRTN`.
    pub sub_family: String,
}

/// A proprietary code (`Prtry/Cd`) and who issued it (`Prtry/Issr`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProprietaryCode {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
}

/// Whether a bank entry undoes an earlier one: a bank correction (`RvslInd`)
/// or a returned payment (`RtrInf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReversalKind {
    /// `RvslInd` true: the bank reverses an earlier entry of the account.
    Reversal,
    /// `RtrInf` without `RvslInd`: a payment came back as a new entry.
    Return,
}

/// Reversal and return information of an entry.
///
/// A reversal or return is booked like any entry: money moves in the entry's
/// own `direction` (`CdtDbtInd`; ISO 20022 reports a reversed debit as a
/// credit), its event key is its own reference, and the skill assigns its
/// accounts, normally those the original was booked to, so the projection's
/// sides undo it. A reversal that reuses the original's reference is refused
/// in the same file as a duplicate key and, against an original already in the
/// books, stops the import as changed source facts; it is never dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reversal {
    pub kind: ReversalKind,
    /// `RtrInf/Rsn`: why the payment was returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonCode>,
    /// `RtrInf/AddtlInf`, every element in document order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub additional_info: Vec<String>,
    /// `RtrInf/OrgnlBkTxCd`: the transaction code of the original payment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_transaction_code: Option<BankTransactionCode>,
    /// The end-to-end id of the transaction details (`Refs/EndToEndId`). A
    /// return or reversal normally carries the original payment's id, which
    /// links it to the original.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_to_end_id: Option<String>,
}

/// A reason given as an ISO external code (`Cd`, e.g. `AC04` closed account)
/// or as proprietary text (`Prtry`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasonCode {
    Code(String),
    Proprietary(String),
}

/// The charges of an entry (`Chrgs`). camt.053.001.02 lists each charge as its
/// own `Chrgs`; later versions put one `Chrgs` with records (`Rcrd`). Both read
/// into the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Charges {
    /// `TtlChrgsAndTaxAmt`: the total of charges and their taxes, when given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<Money>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<ChargeRecord>,
}

/// One charge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChargeRecord {
    /// The charge amount, positive.
    pub amount: Money,
    /// Whether the charge was taken from (`debit`) or credited to the account
    /// (`CdtDbtInd`), when given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
    /// `ChrgInclInd`: whether the charge is included in the entry amount, when
    /// the bank says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub included_in_amount: Option<bool>,
    /// `Tp/Cd`, or `Tp/Prtry/Id`, verbatim (e.g. `COMM`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub charge_type: Option<String>,
    /// `Br`: who bears the charge (e.g. `DEBT`, `CRED`, `SHAR`, `SLEV`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer: Option<String>,
}

impl BankTransaction {
    /// Versioned SHA-256 fingerprint of the normalized source fields retained by
    /// this parser before account/VAT assignment projects them into a voucher.
    ///
    /// Version 1 covers the natural key, bank reference, booking/value dates,
    /// direction, scale-sensitive amount, currency, counterparty, and remittance.
    /// Fields are name- and length-framed; dates, money, and currency use Model's
    /// documented canonical text forms. Derived `accounting_date` is deliberately
    /// excluded, and so are the passthrough fields (`transaction_code`,
    /// `reversal`, `charges`, `instructed_amount`): they were added after
    /// statements were imported, and covering them would change the
    /// fingerprint of every such entry that carries them. Changing what it covers changes the fingerprint of every booked
    /// transaction, and every overlapping re-import would then stop with
    /// `SOURCE_FACTS_CHANGED`: a new version needs a decision about stored ones.
    pub fn source_fingerprint(&self) -> String {
        fn field(hasher: &mut Sha256, name: &str, value: Option<&str>) {
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

        let mut hasher = Sha256::new();
        hasher.update(b"aggountant:bank-source-fingerprint:v1");
        let booking_date = self.booking_date.to_string();
        let value_date = self.value_date.map(|date| date.to_string());
        let amount = self.amount.amount_str();
        let direction = match self.direction {
            Direction::Credit => "credit",
            Direction::Debit => "debit",
        };
        field(&mut hasher, "natural_key", Some(&self.natural_key));
        field(&mut hasher, "bank_ref", Some(&self.bank_ref));
        field(&mut hasher, "booking_date", Some(&booking_date));
        field(&mut hasher, "value_date", value_date.as_deref());
        field(&mut hasher, "direction", Some(direction));
        field(&mut hasher, "amount", Some(&amount));
        field(
            &mut hasher,
            "currency",
            Some(self.amount.currency().as_str()),
        );
        field(&mut hasher, "counterparty", self.counterparty.as_deref());
        field(&mut hasher, "description", self.description.as_deref());
        format!("sha256:v1:{}", hex::encode(hasher.finalize()))
    }
}

/// A parsed statement: the account it is for, its currency, its entries, and
/// the period and balances of each report block in the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParsedStatement {
    /// The statement account's IBAN (`Acct/Id/IBAN`). The account-scoping
    /// part of every entry's `natural_key`.
    pub account_iban: String,
    /// The statement account currency (`Acct/Ccy`), when declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    pub transactions: Vec<BankTransaction>,
    /// One summary per `<Stmt>` (camt.053) or `<Rpt>` (camt.052) block, in
    /// document order: what the bank says the period and balances were, so the
    /// month can be reconciled against the bank's own closing figure.
    pub statements: Vec<StatementSummary>,
    /// Entries with a status other than booked (for example pending `PDNG`).
    /// They are not booked, but an import refuses one whose transaction is
    /// already in the books: the bank has changed its status.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unbooked: Vec<UnbookedEntry>,
    /// Present only when the document has no booked entries at all. A wrong
    /// account, a wrong period or a truncated export would otherwise look like
    /// a quiet month: zero transactions and nothing else said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_report: Option<EmptyReport>,
}

/// The explicit result for a report with balances but no booked entries. It is
/// not an error: a quiet month is a real statement and imports with no
/// vouchers. It names the account and the period so whoever reads it can check
/// that this is the statement they meant before treating the month as quiet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmptyReport {
    /// The report's account (`Acct/Id/IBAN`).
    pub account_iban: String,
    /// The `FrToDt` of every report block that declares one, in document order.
    pub periods: Vec<StatementPeriod>,
    /// Entries listed with a status other than booked (for example pending),
    /// including those without a reference that `unbooked` leaves out.
    pub unbooked_count: usize,
    /// One sentence that says all of the above, for a person or an agent.
    pub message: String,
}

impl EmptyReport {
    fn new(account_iban: &str, statements: &[StatementSummary], unbooked_count: usize) -> Self {
        let periods: Vec<StatementPeriod> =
            statements.iter().filter_map(|s| s.period.clone()).collect();
        let period_text = if periods.is_empty() {
            "an undeclared period".to_string()
        } else {
            let listed: Vec<String> = periods
                .iter()
                .map(|p| format!("{}..{}", p.from, p.to))
                .collect();
            format!("period {}", listed.join(", "))
        };
        let pending = match unbooked_count {
            0 => String::new(),
            n => format!(" ({n} not yet booked by the bank)"),
        };
        let balances = if statements
            .iter()
            .any(|s| s.balances_reconcile == Some(false))
        {
            " Its opening and closing balances differ, so the file is missing entries; \
             do not import it as a quiet month."
        } else {
            " Check that this is the right account and period and that the bank shows no \
             transactions for it before importing it as a quiet month; a wrong or truncated \
             export looks the same."
        };
        EmptyReport {
            account_iban: account_iban.to_string(),
            message: format!(
                "the report for account {account_iban}, {period_text}, has no booked \
                 entries{pending}.{balances}"
            ),
            periods,
            unbooked_count,
        }
    }
}

/// An entry the statement lists with a status other than booked. Only entries
/// with a usable bank reference are kept, since only they can be matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnbookedEntry {
    pub natural_key: String,
    pub bank_ref: String,
    /// The `Sts` code verbatim.
    pub status: String,
}

/// The period (`FrToDt`) and balances (`Bal`) of one report block, and whether
/// its booked entries carry the opening balance to the closing balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatementSummary {
    /// The bank's identifier for the block (`Id`), when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// `FrToDt`, as calendar dates; `None` when the block does not declare it.
    pub period: Option<StatementPeriod>,
    /// The opening booked balance: `OPBD`, or `PRCD` (previously closed booked)
    /// when the block has no `OPBD`.
    pub opening_balance: Option<StatementBalance>,
    /// The closing booked balance (`CLBD`). An interim camt.052 report may
    /// carry only an interim balance (`ITBD`), in which case this is `None`.
    pub closing_balance: Option<StatementBalance>,
    /// Every `Bal` of the block in document order, including the two above.
    pub balances: Vec<StatementBalance>,
    /// How many booked entries of this block are in `transactions`.
    pub entry_count: usize,
    /// The signed sum of those entries (money in positive, money out
    /// negative), in the account currency. `None` when no currency is known
    /// (no `Acct/Ccy` and no entries) or the entries are in several
    /// currencies.
    pub entries_net: Option<Money>,
    /// `opening_balance + entries_net == closing_balance`, in one currency.
    /// `None` when the opening or closing balance is missing. `false` means
    /// the entries read from the file do not explain the bank's own balances,
    /// including when they are not all in the balances' currency.
    pub balances_reconcile: Option<bool>,
}

/// A statement period (`FrToDt`): the calendar dates of `FrDtTm` and `ToDtTm`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatementPeriod {
    pub from: Date,
    pub to: Date,
}

/// One `Bal` element: its ISO type code, signed amount and date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatementBalance {
    /// The ISO 20022 balance type (`Tp/CdOrPrtry/Cd`, e.g. `OPBD`, `CLBD`,
    /// `ITBD`, `CLAV`), or the proprietary code (`Prtry`) verbatim.
    #[serde(rename = "type")]
    pub balance_type: String,
    /// Signed: negative when the balance is a debit (`CdtDbtInd` `DBIT`,
    /// an overdrawn account).
    pub amount: Money,
    pub date: Date,
}

/// First direct child element with the given local (namespace-stripped) name.
fn child<'a, 'input>(n: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

/// Every direct child element with the given local name.
fn children<'a, 'input: 'a>(
    n: Node<'a, 'input>,
    name: &'a str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

/// First descendant element (any depth) with the given local name.
fn descendant<'a, 'input>(n: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    n.descendants()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

/// Trimmed text content of an element (`None` if empty/whitespace-only).
fn text(n: Node) -> Option<String> {
    let t = n.text().unwrap_or("").trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Read an ISO date from a `BookgDt`/`ValDt` wrapper: a `Dt` (`YYYY-MM-DD`) or a
/// `DtTm` (take the calendar-date component before the `T`).
fn read_date(wrapper: Node) -> Result<Date, ImportError> {
    if let Some(dt) = child(wrapper, "Dt").and_then(text) {
        return Ok(Date::parse(&dt)?);
    }
    if let Some(dttm) = child(wrapper, "DtTm").and_then(text) {
        // A camt DtTm is an ISO 8601 datetime; the accounting date is the calendar
        // part before the time separator. Split on 'T' (not a byte slice) so a
        // date-only value or a multi-byte tail can never mis-slice.
        let date = dttm.split('T').next().unwrap_or(&dttm);
        return Ok(Date::parse(date)?);
    }
    Err(ImportError::Camt(format!(
        "<{}> has neither <Dt> nor <DtTm>",
        wrapper.tag_name().name()
    )))
}

/// Drop the well-known "no reference" placeholder sentinels banks emit, so they
/// never become a dedup key. Applied per candidate before the fallback chain.
fn usable_ref(value: Option<String>) -> Option<String> {
    value.filter(|r| !matches!(r.as_str(), "NOTPROVIDED" | "NONREF" | "NA"))
}

/// The report account's IBAN (`Acct/Id/IBAN`) — the same path under a camt.053
/// `<Stmt>` and a camt.052 `<Rpt>`.
fn statement_iban(stmt: Node) -> Option<String> {
    // `<Id>` is an IBAN-XOR-Othr choice, so the IBAN is a direct child — use
    // `child` (not `descendant`, which could pick an IBAN nested inside an
    // `<Othr>` sub-block).
    child(stmt, "Acct")
        .and_then(|a| child(a, "Id"))
        .and_then(|id| child(id, "IBAN"))
        .and_then(text)
}

/// Parse a camt.053 or camt.052 bank statement into canonical bank-transaction
/// rows.
///
/// Accepts either ISO 20022 container: camt.053 `BankToCustomerStatement`
/// (`BkToCstmrStmt → Stmt`) or camt.052 `BankToCustomerAccountReport`
/// (`BkToCstmrAcctRpt → Rpt`). For the fields this parser reads, each report's
/// `Acct/Id/IBAN` and `<Ntry>` sit at the same paths, so the same per-entry
/// reduction runs for both (see the module docs for the limits of that claim).
///
/// The document may carry several report blocks (Holvi exports one per period
/// file, but the standard allows more) — their entries are concatenated in
/// document order. Every report must be for the **same** account IBAN: a
/// document mixing accounts is refused rather than silently scoping the second
/// account's entries under the first account's dedup key. Entries with a
/// non-`BOOK` status (e.g. still-pending `PDNG`) are not transactions — only
/// booked entries are bookable vouchers; they are listed in `unbooked`.
pub fn parse_camt(xml: &str) -> Result<ParsedStatement, ImportError> {
    let doc = Document::parse(xml).map_err(|e| ImportError::Camt(format!("not valid XML: {e}")))?;
    let root = doc.root_element();

    // Accept either container: camt.053 (BkToCstmrStmt → Stmt) or camt.052
    // (BkToCstmrAcctRpt → Rpt). Both reduce to a list of report nodes whose
    // Acct/Id/IBAN and <Ntry> children are read identically below. The container
    // is a direct child of <Document> per the ISO schema, so match on `child`
    // (not `descendant`, which would match a same-named element at any depth) and
    // refuse a document carrying both containers rather than silently picking one.
    let (container, report_name) = match (
        child(root, "BkToCstmrStmt"),
        child(root, "BkToCstmrAcctRpt"),
    ) {
        (Some(bk), None) => (bk, "Stmt"),
        (None, Some(rpt)) => (rpt, "Rpt"),
        (Some(_), Some(_)) => {
            return Err(ImportError::Camt(
                "document has both <BkToCstmrStmt> and <BkToCstmrAcctRpt> — refusing rather \
                     than silently picking one"
                    .into(),
            ));
        }
        (None, None) => {
            return Err(ImportError::Camt(
                "no <BkToCstmrStmt> or <BkToCstmrAcctRpt> — not a camt.053 or camt.052 bank \
                     statement document"
                    .into(),
            ));
        }
    };

    let reports: Vec<Node> = children(container, report_name).collect();
    if reports.is_empty() {
        return Err(ImportError::Camt(format!(
            "bank statement document has no <{report_name}> report block"
        )));
    }

    // The first report labels the result (account it is for).
    let account_iban = statement_iban(reports[0]).ok_or_else(|| {
        ImportError::Camt("<Acct/Id/IBAN> missing — cannot scope the dedup key".into())
    })?;
    let currency = child(reports[0], "Acct")
        .and_then(|a| child(a, "Ccy"))
        .and_then(text);

    let mut transactions = Vec::new();
    let mut unbooked = Vec::new();
    // Every non-booked entry, including those without a reference to match
    // by, which `unbooked` leaves out.
    let mut unbooked_count = 0;
    let mut statements = Vec::new();
    for report in &reports {
        // Refuse a multi-account document: every entry's natural_key is scoped by
        // the account IBAN, so mixing accounts under one label would corrupt dedup.
        // Every report must declare its IBAN — a missing one must not silently
        // inherit the first report's account (that is the same mis-scoping this
        // check exists to prevent).
        let iban = statement_iban(*report).ok_or_else(|| {
            ImportError::Camt(format!(
                "a <{report_name}> report has no <Acct/Id/IBAN> — cannot scope its dedup keys"
            ))
        })?;
        if iban != account_iban {
            return Err(ImportError::Camt(format!(
                "document mixes accounts ({account_iban} and {iban}) — import one account at a time"
            )));
        }
        let first_entry = transactions.len();
        for ntry in children(*report, "Ntry") {
            match parse_entry(ntry, &account_iban, currency.as_deref())? {
                ParsedEntry::Booked(txn) => transactions.push(*txn),
                ParsedEntry::Unbooked(Some(entry)) => {
                    unbooked_count += 1;
                    unbooked.push(entry);
                }
                ParsedEntry::Unbooked(None) => unbooked_count += 1,
            }
        }
        statements.push(summarize_report(
            *report,
            currency.as_deref(),
            &transactions[first_entry..],
        )?);
    }

    // A stable dedup key must be unique within the batch. Two booked entries that
    // collapse to the same natural_key (e.g. a re-used payer `EndToEndId`) would
    // otherwise be silently deduped by `Ledger::seal` — dropping a real entry.
    let mut seen = std::collections::HashSet::new();
    for txn in &transactions {
        if !seen.insert(txn.natural_key.as_str()) {
            return Err(ImportError::Camt(format!(
                "two entries share the dedup key {} (reference {}) — the reference is not unique, \
                 so importing would silently drop one entry",
                txn.natural_key, txn.bank_ref
            )));
        }
    }

    // A reference the document also lists as booked (an interim report may show
    // it pending before a later one books it) is booked, not unbooked.
    unbooked.retain(|u| !seen.contains(u.natural_key.as_str()));

    let empty_report = transactions
        .is_empty()
        .then(|| EmptyReport::new(&account_iban, &statements, unbooked_count));
    Ok(ParsedStatement {
        account_iban,
        currency,
        transactions,
        statements,
        unbooked,
        empty_report,
    })
}

/// Read one report block's id, period and balances, and check its booked
/// entries against them.
fn summarize_report(
    report: Node,
    account_currency: Option<&str>,
    entries: &[BankTransaction],
) -> Result<StatementSummary, ImportError> {
    let id = child(report, "Id").and_then(text);
    let period = match child(report, "FrToDt") {
        Some(fr_to) => {
            let bound = |name: &str| -> Result<Date, ImportError> {
                let raw = child(fr_to, name)
                    .and_then(text)
                    .ok_or_else(|| ImportError::Camt(format!("<FrToDt> has no <{name}>")))?;
                Ok(Date::parse(raw.split('T').next().unwrap_or(&raw))?)
            };
            Some(StatementPeriod {
                from: bound("FrDtTm")?,
                to: bound("ToDtTm")?,
            })
        }
        None => None,
    };
    let balances = children(report, "Bal")
        .map(parse_balance)
        .collect::<Result<Vec<_>, _>>()?;
    let find = |code: &str| balances.iter().find(|b| b.balance_type == code).cloned();
    let opening_balance = find("OPBD").or_else(|| find("PRCD"));
    let closing_balance = find("CLBD");

    let currency = match account_currency {
        Some(code) => Some(Currency::new(code)?),
        None => entries.first().map(|e| e.amount.currency().clone()),
    };
    // Without `Acct/Ccy` the parser does not stop entries in several
    // currencies; such entries have no net in one currency.
    let entries_net = match currency {
        Some(currency) if entries.iter().all(|e| e.amount.currency() == &currency) => {
            let mut net = rust_decimal::Decimal::ZERO;
            for entry in entries {
                let amount = entry.amount.amount();
                net = match entry.direction {
                    Direction::Credit => net.checked_add(amount),
                    Direction::Debit => net.checked_sub(amount),
                }
                .ok_or_else(|| ImportError::Camt("entry amounts overflow Decimal".into()))?;
            }
            Some(Money::new(net, currency))
        }
        _ => None,
    };
    let balances_reconcile = match (&opening_balance, &closing_balance) {
        (Some(opening), Some(closing)) => {
            let currency = opening.amount.currency();
            let net = match &entries_net {
                Some(net) if net.currency() == currency => Some(net.amount()),
                // No entries and no declared currency: nothing moved.
                None if entries.is_empty() => Some(rust_decimal::Decimal::ZERO),
                _ => None,
            };
            Some(
                closing.amount.currency() == currency
                    && net.and_then(|net| opening.amount.amount().checked_add(net))
                        == Some(closing.amount.amount()),
            )
        }
        _ => None,
    };

    Ok(StatementSummary {
        id,
        period,
        opening_balance,
        closing_balance,
        balances,
        entry_count: entries.len(),
        entries_net,
        balances_reconcile,
    })
}

/// Parse one `Bal`: type code, signed amount and date. A balance that lacks
/// any of them is refused, because a reconciliation figure the parser cannot
/// read correctly must not be shown at all.
fn parse_balance(bal: Node) -> Result<StatementBalance, ImportError> {
    let code_or_prtry = child(bal, "Tp").and_then(|tp| child(tp, "CdOrPrtry"));
    let balance_type = code_or_prtry
        .and_then(|c| {
            child(c, "Cd")
                .and_then(text)
                .or_else(|| child(c, "Prtry").and_then(text))
        })
        .ok_or_else(|| ImportError::Camt("<Bal> has no <Tp/CdOrPrtry> code".into()))?;
    let amt_node = child(bal, "Amt")
        .ok_or_else(|| ImportError::Camt(format!("<Bal> {balance_type} has no <Amt>")))?;
    let amt_str = text(amt_node)
        .ok_or_else(|| ImportError::Camt(format!("<Bal> {balance_type} has an empty <Amt>")))?;
    let ccy = amt_node.attribute("Ccy").ok_or_else(|| {
        ImportError::Camt(format!("<Bal> {balance_type} <Amt> has no Ccy attribute"))
    })?;
    let magnitude = Money::parse(&amt_str, Currency::new(ccy)?)?;
    let amount = match child(bal, "CdtDbtInd").and_then(text).as_deref() {
        Some("CRDT") => magnitude,
        // Negate only a non-zero amount: a debit zero stays `0.00`, never `-0.00`.
        Some("DBIT") if !magnitude.amount().is_zero() => {
            Money::new(-magnitude.amount(), magnitude.currency().clone())
        }
        Some("DBIT") => magnitude,
        other => {
            return Err(ImportError::Camt(format!(
                "<Bal> {balance_type} <CdtDbtInd> must be CRDT or DBIT, got {other:?}"
            )))
        }
    };
    let date = read_date(
        child(bal, "Dt")
            .ok_or_else(|| ImportError::Camt(format!("<Bal> {balance_type} has no <Dt>")))?,
    )?;
    Ok(StatementBalance {
        balance_type,
        amount,
        date,
    })
}

enum ParsedEntry {
    Booked(Box<BankTransaction>),
    /// A non-booked entry; `None` when it has no usable reference to match by.
    Unbooked(Option<UnbookedEntry>),
}

/// The per-entry reference: prefer the account-servicer reference (stable
/// across re-exports), then the end-to-end id. Placeholder sentinels are
/// filtered **per candidate**, so a placeholder `AcctSvcrRef` does not shadow a
/// usable `EndToEndId`.
fn entry_bank_ref(ntry: Node, tx_dtls: Option<Node>) -> Option<String> {
    let refs = tx_dtls.and_then(|t| descendant(t, "Refs"));
    usable_ref(child(ntry, "AcctSvcrRef").and_then(text))
        .or_else(|| usable_ref(refs.and_then(|r| child(r, "AcctSvcrRef")).and_then(text)))
        .or_else(|| usable_ref(refs.and_then(|r| child(r, "EndToEndId")).and_then(text)))
}

/// Parse one `Ntry`.
fn parse_entry(
    ntry: Node,
    account_iban: &str,
    account_currency: Option<&str>,
) -> Result<ParsedEntry, ImportError> {
    // Only booked entries are bookable. A missing <Sts> is treated as booked
    // (some issuers omit it on a fully-booked statement).
    if let Some(status) = child(ntry, "Sts").and_then(|s| {
        // camt.053.001.08 wraps the status in <Cd>; earlier revisions inline it.
        child(s, "Cd").and_then(text).or_else(|| text(s))
    }) {
        if status != "BOOK" {
            let first_tx = children(ntry, "NtryDtls")
                .flat_map(|nd| children(nd, "TxDtls"))
                .next();
            return Ok(ParsedEntry::Unbooked(entry_bank_ref(ntry, first_tx).map(
                |bank_ref| UnbookedEntry {
                    natural_key: natural_key(account_iban, &bank_ref),
                    bank_ref,
                    status,
                },
            )));
        }
    }

    // `RvslInd` true marks a reversal, which is booked in its own direction
    // like any entry (see `Reversal`). A value that is neither true nor false
    // was refused before and still is.
    let is_reversal = match child(ntry, "RvslInd").and_then(text).as_deref() {
        None | Some("false") => false,
        Some("true") => true,
        Some(other) => {
            return Err(ImportError::Camt(format!(
                "<RvslInd> must be true or false, got {other:?}"
            )));
        }
    };

    // A batched entry (several `TxDtls`, whether under one `NtryDtls` or spread
    // across several `NtryDtls` blocks) aggregates many transactions under one
    // `Ntry/Amt`. Reducing it to a single voucher would misattribute the
    // counterparty/reference/description of all but the first sub-transaction, so
    // refuse it rather than silently collapse it (per-TxDtls splitting is a later
    // slice — see the import spin-offs).
    let tx_dtls_all: Vec<Node> = children(ntry, "NtryDtls")
        .flat_map(|nd| children(nd, "TxDtls"))
        .collect();
    if tx_dtls_all.len() > 1 {
        return Err(ImportError::Camt(format!(
            "batched <Ntry> with {} <TxDtls> is not supported — one bank entry must map to one \
             voucher (the aggregate amount would misattribute the sub-transactions)",
            tx_dtls_all.len()
        )));
    }
    let tx_dtls = tx_dtls_all.first().copied();

    let amt_node =
        child(ntry, "Amt").ok_or_else(|| ImportError::Camt("<Ntry> has no <Amt>".into()))?;
    let amt_str = text(amt_node).ok_or_else(|| ImportError::Camt("empty <Amt>".into()))?;
    let ccy = amt_node
        .attribute("Ccy")
        .ok_or_else(|| ImportError::Camt("<Amt> has no Ccy attribute".into()))?;
    // Fail fast on a foreign-currency entry: the account is single-currency and
    // this MVP has no FX treatment, so posting a non-account-currency amount to the
    // bank leg would misfile it. (Multi-currency handling is deferred.)
    if let Some(acct_ccy) = account_currency {
        if ccy != acct_ccy {
            return Err(ImportError::Camt(format!(
                "entry currency {ccy} differs from account currency {acct_ccy} — \
                 multi-currency import is not supported"
            )));
        }
    }
    let amount = Money::parse(&amt_str, Currency::new(ccy)?)?;

    let direction = match child(ntry, "CdtDbtInd").and_then(text).as_deref() {
        Some("CRDT") => Direction::Credit,
        Some("DBIT") => Direction::Debit,
        other => {
            return Err(ImportError::Camt(format!(
                "<CdtDbtInd> must be CRDT or DBIT, got {other:?}"
            )))
        }
    };

    let booking_date = read_date(
        child(ntry, "BookgDt")
            .ok_or_else(|| ImportError::Camt("<Ntry> has no <BookgDt>".into()))?,
    )?;
    let value_date = match child(ntry, "ValDt") {
        Some(v) => Some(read_date(v)?),
        None => None,
    };

    // Without a usable reference there is no stable dedup key, so the entry is
    // refused rather than double-postable.
    let bank_ref = entry_bank_ref(ntry, tx_dtls).ok_or_else(|| {
        ImportError::Camt(
            "<Ntry> has no usable AcctSvcrRef / EndToEndId — cannot build a stable dedup key"
                .into(),
        )
    })?;

    // The counterparty is the *other* party: creditor for money out, debtor for
    // money in. Read from the entry-details related-parties block. A reversal
    // keeps the original transaction's roles (the ISO 20022 camt.05x usage), so
    // its other party is in the role opposite to its own direction; the
    // direction's role is the fallback for a bank that swaps them. Ordinary
    // entries read exactly as they always have.
    let counterparty = tx_dtls
        .and_then(|t| descendant(t, "RltdPties"))
        .and_then(|rp| {
            let (party, original_party) = match direction {
                Direction::Debit => ("Cdtr", "Dbtr"),
                Direction::Credit => ("Dbtr", "Cdtr"),
            };
            let name =
                |role: &str| child(rp, role).and_then(|p| descendant(p, "Nm").and_then(text));
            if is_reversal {
                name(original_party).or_else(|| name(party))
            } else {
                name(party)
            }
        });

    // Unstructured remittance info, joined (an entry may carry several <Ustrd>).
    let description = tx_dtls
        .and_then(|t| descendant(t, "RmtInf"))
        .map(|rmt| {
            children(rmt, "Ustrd")
                .filter_map(text)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|s| !s.is_empty());

    Ok(ParsedEntry::Booked(Box::new(BankTransaction {
        natural_key: natural_key(account_iban, &bank_ref),
        bank_ref,
        booking_date,
        value_date,
        // KPL 2:4: the accounting date is the booking date (see field doc).
        accounting_date: booking_date,
        direction,
        amount,
        counterparty,
        description,
        transaction_code: child(ntry, "BkTxCd")
            .and_then(read_transaction_code)
            .or_else(|| {
                tx_dtls
                    .and_then(|t| child(t, "BkTxCd"))
                    .and_then(read_transaction_code)
            }),
        reversal: read_reversal(is_reversal, tx_dtls),
        // Charges and the instructed amount are read from the entry, else from
        // its one transaction details block.
        charges: [Some(ntry), tx_dtls]
            .into_iter()
            .flatten()
            .find(|n| child(*n, "Chrgs").is_some())
            .and_then(read_charges),
        instructed_amount: [Some(ntry), tx_dtls].into_iter().flatten().find_map(|n| {
            let amt = child(child(child(n, "AmtDtls")?, "InstdAmt")?, "Amt")?;
            read_money(amt)
        }),
    })))
}

/// Read an `Amt`-like element (text amount, `Ccy` attribute) into money, or
/// `None` when either is missing or malformed.
fn read_money(amt: Node) -> Option<Money> {
    let currency = Currency::new(amt.attribute("Ccy")?).ok()?;
    Money::parse(&text(amt)?, currency).ok()
}

/// Read a `BkTxCd` (or `OrgnlBkTxCd`). A `Domn` lacking any of its three codes
/// is left out; `None` when neither part is readable.
fn read_transaction_code(code: Node) -> Option<BankTransactionCode> {
    let domain = child(code, "Domn").and_then(|domn| {
        let family = child(domn, "Fmly")?;
        Some(DomainCode {
            domain: child(domn, "Cd").and_then(text)?,
            family: child(family, "Cd").and_then(text)?,
            sub_family: child(family, "SubFmlyCd").and_then(text)?,
        })
    });
    let proprietary = child(code, "Prtry").and_then(|prtry| {
        Some(ProprietaryCode {
            code: child(prtry, "Cd").and_then(text)?,
            issuer: child(prtry, "Issr").and_then(text),
        })
    });
    (domain.is_some() || proprietary.is_some()).then_some(BankTransactionCode {
        domain,
        proprietary,
    })
}

/// Read reversal and return information: present when the entry is a reversal
/// or its transaction details carry `RtrInf`.
fn read_reversal(is_reversal: bool, tx_dtls: Option<Node>) -> Option<Reversal> {
    let rtr_inf = tx_dtls.and_then(|t| child(t, "RtrInf"));
    let kind = match (is_reversal, rtr_inf) {
        (true, _) => ReversalKind::Reversal,
        (false, Some(_)) => ReversalKind::Return,
        (false, None) => return None,
    };
    let reason = rtr_inf.and_then(|r| child(r, "Rsn")).and_then(|rsn| {
        child(rsn, "Cd")
            .and_then(text)
            .map(ReasonCode::Code)
            .or_else(|| {
                child(rsn, "Prtry")
                    .and_then(text)
                    .map(ReasonCode::Proprietary)
            })
    });
    Some(Reversal {
        kind,
        reason,
        additional_info: rtr_inf
            .map(|r| children(r, "AddtlInf").filter_map(text).collect())
            .unwrap_or_default(),
        original_transaction_code: rtr_inf
            .and_then(|r| child(r, "OrgnlBkTxCd"))
            .and_then(read_transaction_code),
        end_to_end_id: tx_dtls
            .and_then(|t| child(t, "Refs"))
            .and_then(|r| child(r, "EndToEndId"))
            .and_then(text),
    })
}

/// Read the `Chrgs` children of `parent` (the entry or its transaction
/// details). A `Chrgs` with `Rcrd` children (camt.053.001.03 and later) gives
/// one record per `Rcrd`; one without (camt.053.001.02) is itself a record. A
/// record whose amount cannot be read is left out. `None` when nothing is
/// readable.
fn read_charges(parent: Node) -> Option<Charges> {
    let mut total = None;
    let mut records = Vec::new();
    for chrgs in children(parent, "Chrgs") {
        if total.is_none() {
            total = child(chrgs, "TtlChrgsAndTaxAmt").and_then(read_money);
        }
        let rcrds: Vec<Node> = children(chrgs, "Rcrd").collect();
        if rcrds.is_empty() {
            records.extend(read_charge_record(chrgs));
        } else {
            records.extend(rcrds.into_iter().filter_map(read_charge_record));
        }
    }
    (total.is_some() || !records.is_empty()).then_some(Charges { total, records })
}

fn read_charge_record(record: Node) -> Option<ChargeRecord> {
    let amount = child(record, "Amt").and_then(read_money)?;
    let charge_type = child(record, "Tp").and_then(|tp| {
        child(tp, "Cd").and_then(text).or_else(|| {
            child(tp, "Prtry")
                .and_then(|p| child(p, "Id"))
                .and_then(text)
        })
    });
    Some(ChargeRecord {
        amount,
        direction: match child(record, "CdtDbtInd").and_then(text).as_deref() {
            Some("CRDT") => Some(Direction::Credit),
            Some("DBIT") => Some(Direction::Debit),
            _ => None,
        },
        included_in_amount: match child(record, "ChrgInclInd").and_then(text).as_deref() {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => None,
        },
        charge_type,
        bearer: child(record, "Br").and_then(text),
    })
}

/// The prefix of every bank-transaction event key.
pub const EVENT_KEY_PREFIX: &str = "bank-import:";

/// The event key of an entry: the account and the bank's own reference.
fn natural_key(account_iban: &str, bank_ref: &str) -> String {
    format!("{EVENT_KEY_PREFIX}holvi:{account_iban}:{bank_ref}")
}
