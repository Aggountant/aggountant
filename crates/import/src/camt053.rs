//! Deterministic camt.053 (ISO 20022 `BankToCustomerStatement`) reader — the
//! Stage-1 parser of the manual tosite-loading pipeline
//! (`briox-holvi-import-strategy/architecture-options.md` §1.2). Each `Ntry`
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
//! and can carry semantics .053 does not (e.g. reversals — see `parse_entry`,
//! which refuses `<RvslInd>true</RvslInd>`).

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
    /// Source-based idempotency key (`candidate.md §3`;
    /// `bank-import:holvi:<iban>:<ref>`). Stable across re-exported / overlapping
    /// statement ranges, because the uniqueness-bearing part is the bank's own
    /// per-entry reference — so `Ledger::seal` collapses a duplicate to one voucher.
    pub natural_key: String,
    /// The bank's own reference for this entry (`AcctSvcrRef`, else `EndToEndId`),
    /// preserved verbatim. The uniqueness-bearing part of `natural_key`.
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
}

impl BankTransaction {
    /// Versioned SHA-256 fingerprint of the normalized source fields retained by
    /// this parser before account/VAT assignment projects them into a voucher.
    ///
    /// Version 1 covers the natural key, bank reference, booking/value dates,
    /// direction, scale-sensitive amount, currency, counterparty, and remittance.
    /// Fields are name- and length-framed; dates, money, and currency use Model's
    /// documented canonical text forms. Derived `accounting_date` is deliberately
    /// excluded. Exact replay still requires equal projected draft bytes as well as
    /// this fingerprint; a caller that attaches a different whole-export artifact
    /// can therefore conflict on the draft `request_hash` first.
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

/// A parsed statement: the account it is for, its currency, and its entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParsedStatement {
    /// The statement account's IBAN (`Acct/Id/IBAN`). The account-scoping
    /// part of every entry's `natural_key`.
    pub account_iban: String,
    /// The statement account currency (`Acct/Ccy`), when declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    pub transactions: Vec<BankTransaction>,
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
    Err(ImportError::Camt053(format!(
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
/// non-`BOOK` status (e.g. still-pending `PDNG`) are skipped — only booked
/// entries are bookable vouchers.
pub fn parse_camt053(xml: &str) -> Result<ParsedStatement, ImportError> {
    let doc =
        Document::parse(xml).map_err(|e| ImportError::Camt053(format!("not valid XML: {e}")))?;
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
            return Err(ImportError::Camt053(
                "document has both <BkToCstmrStmt> and <BkToCstmrAcctRpt> — refusing rather \
                     than silently picking one"
                    .into(),
            ));
        }
        (None, None) => {
            return Err(ImportError::Camt053(
                "no <BkToCstmrStmt> or <BkToCstmrAcctRpt> — not a camt.053 or camt.052 bank \
                     statement document"
                    .into(),
            ));
        }
    };

    let reports: Vec<Node> = children(container, report_name).collect();
    if reports.is_empty() {
        return Err(ImportError::Camt053(format!(
            "bank statement document has no <{report_name}> report block"
        )));
    }

    // The first report labels the result (account it is for).
    let account_iban = statement_iban(reports[0]).ok_or_else(|| {
        ImportError::Camt053("<Acct/Id/IBAN> missing — cannot scope the dedup key".into())
    })?;
    let currency = child(reports[0], "Acct")
        .and_then(|a| child(a, "Ccy"))
        .and_then(text);

    let mut transactions = Vec::new();
    for report in &reports {
        // Refuse a multi-account document: every entry's natural_key is scoped by
        // the account IBAN, so mixing accounts under one label would corrupt dedup.
        // Every report must declare its IBAN — a missing one must not silently
        // inherit the first report's account (that is the same mis-scoping this
        // check exists to prevent).
        let iban = statement_iban(*report).ok_or_else(|| {
            ImportError::Camt053(format!(
                "a <{report_name}> report has no <Acct/Id/IBAN> — cannot scope its dedup keys"
            ))
        })?;
        if iban != account_iban {
            return Err(ImportError::Camt053(format!(
                "document mixes accounts ({account_iban} and {iban}) — import one account at a time"
            )));
        }
        for ntry in children(*report, "Ntry") {
            if let Some(txn) = parse_entry(ntry, &account_iban, currency.as_deref())? {
                transactions.push(txn);
            }
        }
    }

    // A stable dedup key must be unique within the batch. Two booked entries that
    // collapse to the same natural_key (e.g. a re-used payer `EndToEndId`) would
    // otherwise be silently deduped by `Ledger::seal` — dropping a real entry.
    let mut seen = std::collections::HashSet::new();
    for txn in &transactions {
        if !seen.insert(txn.natural_key.as_str()) {
            return Err(ImportError::Camt053(format!(
                "two entries share the dedup key {} (reference {}) — the reference is not unique, \
                 so importing would silently drop one entry",
                txn.natural_key, txn.bank_ref
            )));
        }
    }

    Ok(ParsedStatement {
        account_iban,
        currency,
        transactions,
    })
}

/// Parse one `Ntry`. Returns `Ok(None)` for a non-booked entry (skipped).
fn parse_entry(
    ntry: Node,
    account_iban: &str,
    account_currency: Option<&str>,
) -> Result<Option<BankTransaction>, ImportError> {
    // Only booked entries are bookable. A missing <Sts> is treated as booked
    // (some issuers omit it on a fully-booked statement).
    if let Some(status) = child(ntry, "Sts").and_then(|s| {
        // camt.053.001.08 wraps the status in <Cd>; earlier revisions inline it.
        child(s, "Cd").and_then(text).or_else(|| text(s))
    }) {
        if status != "BOOK" {
            return Ok(None);
        }
    }

    // A reversal (`<RvslInd>true</RvslInd>`, common in camt.052 interim reports)
    // is not a plain booking: posting it as an independent entry mis-books, and if
    // it reuses the original entry's reference the `natural_key` collides and the
    // reversal is silently dropped by `Ledger::seal`. Refuse it rather than
    // mis-handle it (reversal-aware projection is a later slice — see the import
    // spin-offs). `false`/absent are the normal case.
    match child(ntry, "RvslInd").and_then(text).as_deref() {
        None | Some("false") => {}
        Some("true") => {
            return Err(ImportError::Camt053(
                "<Ntry> has <RvslInd>true</RvslInd> — reversal entries are not supported (a \
                 reversal is not an independent booking)"
                    .into(),
            ));
        }
        Some(other) => {
            return Err(ImportError::Camt053(format!(
                "<RvslInd> must be true or false, got {other:?}"
            )));
        }
    }

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
        return Err(ImportError::Camt053(format!(
            "batched <Ntry> with {} <TxDtls> is not supported — one bank entry must map to one \
             voucher (the aggregate amount would misattribute the sub-transactions)",
            tx_dtls_all.len()
        )));
    }
    let tx_dtls = tx_dtls_all.first().copied();

    let amt_node =
        child(ntry, "Amt").ok_or_else(|| ImportError::Camt053("<Ntry> has no <Amt>".into()))?;
    let amt_str = text(amt_node).ok_or_else(|| ImportError::Camt053("empty <Amt>".into()))?;
    let ccy = amt_node
        .attribute("Ccy")
        .ok_or_else(|| ImportError::Camt053("<Amt> has no Ccy attribute".into()))?;
    // Fail fast on a foreign-currency entry: the account is single-currency and
    // this MVP has no FX treatment, so posting a non-account-currency amount to the
    // bank leg would misfile it. (Multi-currency handling is deferred.)
    if let Some(acct_ccy) = account_currency {
        if ccy != acct_ccy {
            return Err(ImportError::Camt053(format!(
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
            return Err(ImportError::Camt053(format!(
                "<CdtDbtInd> must be CRDT or DBIT, got {other:?}"
            )))
        }
    };

    let booking_date = read_date(
        child(ntry, "BookgDt")
            .ok_or_else(|| ImportError::Camt053("<Ntry> has no <BookgDt>".into()))?,
    )?;
    let value_date = match child(ntry, "ValDt") {
        Some(v) => Some(read_date(v)?),
        None => None,
    };

    // The per-entry reference: prefer the account-servicer reference (stable
    // across re-exports), then the end-to-end id. Placeholder sentinels are
    // filtered **per candidate**, so a placeholder `AcctSvcrRef` does not shadow a
    // usable `EndToEndId`. Without a usable reference there is no stable dedup key,
    // so the entry is refused rather than double-postable.
    let refs = tx_dtls.and_then(|t| descendant(t, "Refs"));
    let bank_ref = usable_ref(child(ntry, "AcctSvcrRef").and_then(text))
        .or_else(|| usable_ref(refs.and_then(|r| child(r, "AcctSvcrRef")).and_then(text)))
        .or_else(|| usable_ref(refs.and_then(|r| child(r, "EndToEndId")).and_then(text)))
        .ok_or_else(|| {
            ImportError::Camt053(
                "<Ntry> has no usable AcctSvcrRef / EndToEndId — cannot build a stable dedup key"
                    .into(),
            )
        })?;

    // The counterparty is the *other* party: creditor for money out, debtor for
    // money in. Read from the entry-details related-parties block.
    let counterparty = tx_dtls
        .and_then(|t| descendant(t, "RltdPties"))
        .and_then(|rp| {
            let party = match direction {
                Direction::Debit => "Cdtr",
                Direction::Credit => "Dbtr",
            };
            child(rp, party).and_then(|p| descendant(p, "Nm").and_then(text))
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

    let natural_key = format!("bank-import:holvi:{account_iban}:{bank_ref}");

    Ok(Some(BankTransaction {
        natural_key,
        bank_ref,
        booking_date,
        value_date,
        // KPL 2:4: the accounting date is the booking date (see field doc).
        accounting_date: booking_date,
        direction,
        amount,
        counterparty,
        description,
    }))
}
