//! Fiscal period state + the `ClosingAuthority` capability (ADR-6, `candidate.md
//! §3 ClosingAuthority`).
//!
//! [`PeriodState`] is the authoritative `Open | Closing | Sealed` value the Ledger
//! reads on **every** voucher write (F5) and advances by compare-and-swap inside
//! the seal's `BEGIN IMMEDIATE` writer lock (ADR-6 §2). It lives in the fiscal
//! SQLite store — this module carries only the *public* shapes the CLI/process
//! layers see; the storage rows are `pub(crate)` inside `store`.
//!
//! [`ClosingAuthority`] is the non-serialisable capability that authorises a
//! close-run's Closing-period writes, its rejection (`Closing→Open`) and the
//! `Sealed→Open/Closing` reopen (ADR-6 §5). It has **no public constructor** and
//! does not implement `Serialize`/`Deserialize`: it can only be *minted by the
//! Ledger* (`begin_close`) or *re-acquired* from the authoritative fiscal state by the run that owns the
//! period (`acquire_authority`), and it cannot be forged from a bare `--run-id`
//! string (`candidate.md §4`, `VOUCHER_NOT_IN_RUN`). Fiscal re-checks
//! `owning_run_id` on every use, so even a real token whose period ownership has
//! moved on is fenced (ADR-6 §Biggest risk).

use serde::Serialize;

use aggountant_model::{Date, PeriodId};

/// The authoritative fiscal state of a period (ADR-6 §1). `Open` is the implicit
/// state of a period that has never transitioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PeriodState {
    Open,
    Closing,
    Sealed,
}

impl PeriodState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PeriodState::Open => "Open",
            PeriodState::Closing => "Closing",
            PeriodState::Sealed => "Sealed",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "Open" => Some(PeriodState::Open),
            "Closing" => Some(PeriodState::Closing),
            "Sealed" => Some(PeriodState::Sealed),
            _ => None,
        }
    }
}

/// A derived, read-only projection of the authoritative `period` row (ADR-6 §6),
/// for `period show` and for a resuming run to read "where is this period?" with
/// a single indexed read.
#[derive(Debug, Clone, Serialize)]
pub struct PeriodView {
    pub period: String,
    pub state: PeriodState,
    /// The run that currently owns this period's close/seal, if any.
    pub owning_run_id: Option<String>,
    /// True once the period's VAT return has been filed — the guard that forbids
    /// the ADR-6 §5 reopen exception. Direct and evidence-bearing manual filing
    /// both set this through the same audited filing self-edge.
    pub vat_filed: bool,
    /// Per-period transition-log head (count + hash), for defense-in-depth
    /// `fold(period_transition) == period` verification (ADR-6 §1).
    pub head_transition_seq: u64,
    pub head_transition_hash: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VatFilingReceipt {
    pub unique_identifier: String,
    pub accepted_timestamp: String,
}

/// One recorded VAT filing of a period: the first return or a replacement.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VatFilingRecord {
    /// 1 for the first return, 2 for its first replacement, and so on.
    pub filing: u32,
    pub recorded_at: String,
    pub run_id: Option<String>,
    /// Hash of the FileVATReturn request, absent for a manual filing.
    pub sent_hash: Option<String>,
    pub receipt: Option<VatFilingReceipt>,
    pub manual: Option<ManualVatFilingEvidence>,
}

/// What proves a replacement return, as recorded in its filing event.
#[derive(Debug, Clone, Copy)]
pub enum VatFilingEvidence<'a> {
    /// Accepted through the API with a receipt.
    Receipt {
        sent_hash: &'a str,
        unique_identifier: &'a str,
        accepted_timestamp: &'a str,
    },
    /// The authority shows the approved values processed; that read returns no
    /// receipt fields and none are invented.
    Reconciled { sent_hash: &'a str },
    /// Made by the operator in the e-service.
    Manual {
        channel: &'a str,
        evidence_ref: &'a str,
        remote_status_evidence_ref: Option<&'a str>,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ManualVatFilingEvidence {
    pub channel: String,
    pub evidence_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_status_evidence_ref: Option<String>,
}

/// Non-serialisable capability authorising a close-run's Closing-period writes and
/// the reopen exception. See the module docs. **No public constructor** and no
/// `Serialize`/`Deserialize` — this is the type-level half of "`--run-id` is not
/// an authority".
#[derive(Debug, Clone)]
pub struct ClosingAuthority {
    run_id: String,
    period: PeriodId,
}

impl ClosingAuthority {
    /// Minted **only** by the Ledger, against authoritative fiscal state that has
    /// confirmed `run_id` owns `period` in `Closing` (ADR-6 §2). `pub(crate)` so
    /// no downstream crate can fabricate one.
    pub(crate) fn mint(run_id: String, period: PeriodId) -> Self {
        ClosingAuthority { run_id, period }
    }

    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Read-only view of the owning run (for logging / display). Does not make the
    /// capability forgeable — reading the id is not the same as holding the token.
    pub fn owning_run_id(&self) -> &str {
        &self.run_id
    }
}

/// The authoritative fiscal state of a **year** (RF6/S6, `SealYear`). A year is
/// `Open` until the `close-year` Run locks it; `Sealed` is irreversible (the second
/// irreversible seal after `SealPeriod`, ADR-6 / D-09). Mirrors [`PeriodState`] but
/// at fiscal-year granularity, with no `Closing` intermediate — the multi-gate
/// close is orchestrated in process; the fiscal seal itself is one atomic CAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FiscalYearState {
    Open,
    Sealed,
}

impl FiscalYearState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            FiscalYearState::Open => "Open",
            FiscalYearState::Sealed => "Sealed",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "Open" => Some(FiscalYearState::Open),
            "Sealed" => Some(FiscalYearState::Sealed),
            _ => None,
        }
    }
}

/// A derived, read-only projection of the authoritative `fiscal_year` row
/// (`fiscal-year show`). A year that never sealed reads back `Open`.
#[derive(Debug, Clone, Serialize)]
pub struct FiscalYearView {
    /// The label: the calendar year in which the fiscal year ends.
    pub year: i32,
    pub start: Date,
    pub end: Date,
    pub months: u32,
    pub state: FiscalYearState,
    /// The `statements_hash` the `ApproveStatements` gate bound and `SealYear`
    /// locked — `None` until the year is sealed (what-was-approved == what-was-sealed).
    pub statements_hash: Option<String>,
    /// The run that sealed the year, if any.
    pub sealed_by_run_id: Option<String>,
    /// Per-year transition-log head (count + hash), for the defense-in-depth
    /// `fold(fiscal_year_transition) == fiscal_year` verification.
    pub head_seq: u64,
    pub head_hash: String,
}

/// The result of `fiscal-year define` (issue `non-calendar-fiscal-year`).
#[derive(Debug, Clone, Serialize)]
pub struct FiscalYearDefinitionView {
    /// The label: the calendar year in which the defined fiscal year ends.
    pub year: i32,
    pub start: Date,
    pub end: Date,
    pub months: u32,
    /// Where the following (12-month) fiscal year starts.
    pub next_start: Date,
    /// True when the identical definition already existed.
    pub replayed: bool,
}

/// A derived, read-only projection of one `adjustment_attribution` event (issue
/// `closeyear-adjustment-attribution`): the statement year a sealed voucher's
/// postings are attributed to. Returned by `adjustment_attribution` /
/// `attribute_adjustment`. `replayed` is meaningful **only** on the value returned by
/// `attribute_adjustment` — true when a repeat call re-observed an existing (identical)
/// attribution rather than recording a new one; the read APIs (`adjustment_attribution`,
/// `adjustment_attributions_for_year`) always report `false` (a read is not a replay).
#[derive(Debug, Clone, Serialize)]
pub struct AdjustmentAttributionView {
    pub voucher_number: u64,
    pub effective_year: i32,
    /// The close-year run that recorded the attribution.
    pub owning_run_id: String,
    pub actor: String,
    pub recorded_at: String,
    pub replayed: bool,
}

/// Non-serialisable capability authorising a `file-vat` Run to record a period's
/// VAT filing (S3/RF3). Like [`ClosingAuthority`] it has **no public constructor**
/// and no `Serialize`/`Deserialize`: it can only be *minted by the Ledger*
/// (`acquire_filing_authority`) after it confirms the period is `Sealed` and
/// not-yet-filed. It is the type-level half of "a bare `--run-id` cannot file a
/// return"; the filing CAS re-checks `Sealed` + `!vat_filed` independently, so an
/// already-filed period is refused even if a token was minted concurrently.
#[derive(Debug, Clone)]
pub struct FilingAuthority {
    run_id: String,
    period: PeriodId,
}

impl FilingAuthority {
    pub(crate) fn mint(run_id: String, period: PeriodId) -> Self {
        FilingAuthority { run_id, period }
    }

    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Read-only view of the filing run (for logging / display).
    pub fn owning_run_id(&self) -> &str {
        &self.run_id
    }
}

/// Non-serialisable capability authorising one replacement return for a filed
/// period. Minted only by the Ledger (`acquire_replacement_authority`) and bound
/// to the number of filings it replaces, which the recording CAS re-checks.
#[derive(Debug, Clone)]
pub struct ReplacementAuthority {
    run_id: String,
    period: PeriodId,
    replaces: u64,
}

impl ReplacementAuthority {
    pub(crate) fn mint(run_id: String, period: PeriodId, replaces: u64) -> Self {
        ReplacementAuthority {
            run_id,
            period,
            replaces,
        }
    }

    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn period(&self) -> &PeriodId {
        &self.period
    }

    pub(crate) fn replaces(&self) -> u64 {
        self.replaces
    }
}
