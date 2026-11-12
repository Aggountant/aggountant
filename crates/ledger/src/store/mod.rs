//! Internal storage ports (F2), refined per **ADR-1 §Implications for the Store
//! trait**: instead of one generic key-value `Store`, capability-specific ports
//! whose contract tests assert *semantic* guarantees (gapless numbering,
//! exactly-once chain advance, idempotency, durable refusals, artifact
//! completeness).
//!
//! These traits are `pub(crate)`: no crate outside `aggountant-ledger` references
//! them. The F16 structural check enforces this — the `Store` boundary lives
//! only inside the Ledger crate (`candidate.md §2/§4`).

pub(crate) mod cipher;
pub(crate) mod fs_artifact;
pub(crate) mod git_masterdata;
pub(crate) mod migrations;
pub(crate) mod sqlite_fiscal;
pub(crate) mod sqlite_pii;

use aggountant_model::{
    Account, AccountId, Date, FiscalCalendar, FiscalYear, Hash, Money, VoucherBody,
};

use crate::employee::EmployeeState;
use crate::error::LedgerError;
use crate::period::{ClosingAuthority, FiscalYearState, PeriodState};

/// The pure, deterministic output of building one voucher for a given allocated
/// number and previous hash (produced by the Ledger via the `wire` crate,
/// inside the fiscal transaction — `ADR-1 §Decision`).
pub(crate) struct BuiltVoucher {
    pub body: VoucherBody,
    pub canonical: Vec<u8>,
    pub hash: Hash,
}

/// Provenance persisted alongside a sealed voucher (F9 basic). Deliberately
/// **not** part of the hash preimage, so the chain hash stays reproducible
/// regardless of wall-clock time or actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredProvenance {
    pub actor: String,
    pub natural_key: String,
    /// Hash of the canonical draft bytes; bound to the idempotency key so a
    /// reused `natural_key` with different content is a conflict, not a silent
    /// replay of unrelated data.
    pub request_hash: String,
    /// Fingerprint of the source event before projection into a voucher. Import
    /// callers set this so a changed re-export cannot replay merely because its
    /// projected voucher happens to be unchanged. Legacy/direct callers may omit it.
    /// Like the rest of provenance, this stays outside the fiscal hash preimage.
    pub source_fingerprint: Option<String>,
    pub policy_set: String,
    /// Commit description embedded into the sealing binary at build time. This is
    /// environmental provenance and deliberately remains outside the hash preimage.
    pub software_version: String,
    pub chart_version: String,
    pub sealed_at: String,
    pub sent_hash: Option<String>,
}

/// A sealed fiscal record as read back from the store.
#[derive(Debug, Clone)]
pub(crate) struct StoredVoucher {
    pub number: u64,
    pub prev_hash: Hash,
    pub hash: Hash,
    pub canonical: Vec<u8>,
    pub chart_version: String,
    pub sealed_at: String,
    pub provenance: StoredProvenance,
}

/// Outcome of a commit attempt inside the fiscal transaction.
pub(crate) enum CommitResult {
    /// A fresh voucher was sealed.
    Sealed(StoredVoucher),
    /// The `natural_key` already sealed this exact request → idempotent replay.
    Replayed(StoredVoucher),
    /// The `natural_key` exists but the request or its source fingerprint differs.
    Conflict(IdempotencyConflict),
    /// The write was refused by the in-transaction period gate (ADR-6 §2/F5):
    /// the target period is `Closing`/`Sealed` or the presented authority did not
    /// own it. The transaction is rolled back; the caller records the refusal in
    /// the rejection log (same discipline as the account/attachment refusals).
    Refused(LedgerError),
}

/// The period a voucher is being written into, plus any `ClosingAuthority` the
/// caller presents (ADR-6 §2). Evaluated **inside** the seal transaction so the
/// state guarding the write and the write itself share one transaction domain —
/// there is no window in which a voucher seals into a period a close-run believes
/// is `Closing`.
pub(crate) struct PeriodGate<'a> {
    /// Posting date whose containing month and legacy quarter are both checked.
    pub posting_date: Date,
    /// A close-run's Closing-write capability, or `None` for an ordinary seal.
    pub authority: Option<&'a ClosingAuthority>,
}

/// The authoritative `period` state row (ADR-6 §1): the O(1) indexed lookup the
/// Ledger reads on every voucher write. A period with no row is implicitly `Open`.
#[derive(Debug, Clone)]
pub(crate) struct PeriodRow {
    pub period_id: String,
    pub state: PeriodState,
    pub owning_run_id: Option<String>,
    pub vat_filed: bool,
    pub head_seq: u64,
    pub head_hash: Hash,
}

/// One immutable event on the append-only, hash-chained `period_transition` log
/// (ADR-6 §1). The current-state row is a fold of these; `fiscal verify` checks
/// `fold(period_transition) == period`.
#[derive(Debug, Clone)]
pub(crate) struct StoredPeriodTransition {
    pub period_id: String,
    /// 1-based sequence within the period (the period's transition-chain height).
    pub period_seq: u64,
    pub prev_hash: Hash,
    pub from_state: String,
    pub to_state: String,
    pub owning_run_id: Option<String>,
    /// The `sent_hash` bound to a VAT-filing event (the `Sealed→Sealed` filing
    /// self-edge, S3/RF3). `None` for a plain state transition. Part of the event
    /// hash pre-image, so a tamper is caught by `voucher verify`.
    pub sent_hash: Option<String>,
    pub receipt_unique_identifier: Option<String>,
    pub receipt_accepted_timestamp: Option<String>,
    pub manual_channel: Option<String>,
    pub manual_evidence_ref: Option<String>,
    pub remote_status_evidence_ref: Option<String>,
    pub actor: String,
    pub ts: String,
    pub hash: Hash,
}

/// A requested period CAS transition (ADR-6 §2/§5). Every successful transition
/// sets `owning_run_id := run_id`; the CAS predicate always pins the current
/// `state = from`, and optionally the owner and the not-yet-filed guard.
pub(crate) struct TransitionRequest {
    pub period_id: String,
    pub from: PeriodState,
    pub to: PeriodState,
    pub run_id: String,
    /// Require `owning_run_id == run_id` in the CAS (Seal/Reopen — the fencing
    /// re-check). `false` for `BeginClose`, which claims an unowned Open period.
    pub match_owner: bool,
    /// Require `vat_filed == 0` in the CAS (the Reopen pre-filing guard, ADR-6 §5,
    /// and the "file once" guard for the filing self-edge — filing is irreversible).
    pub require_unfiled: bool,
    /// Set `vat_filed := 1` in the CAS (the VAT-filing self-edge `Sealed→Sealed`,
    /// S3/RF3). The event is co-transacted with the flag in one writer lock, so the
    /// filing fact is tamper-evident (`fold(period_transition).vat_filed == row`).
    pub set_filed: bool,
    /// The `sent_hash` of the approved+submitted envelope, bound into the filing
    /// event's hash pre-image (`None` for a plain state transition).
    pub sent_hash: Option<String>,
    pub receipt_unique_identifier: Option<String>,
    pub receipt_accepted_timestamp: Option<String>,
    pub manual_channel: Option<String>,
    pub manual_evidence_ref: Option<String>,
    pub remote_status_evidence_ref: Option<String>,
    pub actor: String,
}

/// The authoritative `fiscal_year` state row (RF6/S6). A year with no row is
/// implicitly `Open`. Mirrors [`PeriodRow`] at fiscal-year granularity.
#[derive(Debug, Clone)]
pub(crate) struct FiscalYearRow {
    pub year: i32,
    pub state: FiscalYearState,
    pub statements_hash: Option<String>,
    pub owning_run_id: Option<String>,
    pub head_seq: u64,
    pub head_hash: Hash,
}

/// One immutable event on the append-only, hash-chained `fiscal_year_transition`
/// log. The current-state row is a fold of these; `verify` checks the fold.
#[derive(Debug, Clone)]
pub(crate) struct StoredFiscalYearTransition {
    pub year: i32,
    pub year_seq: u64,
    pub prev_hash: Hash,
    pub from_state: String,
    pub to_state: String,
    /// The `statements_hash` bound into the `SealYear` event's hash pre-image (the
    /// approved-statements binding). `None` would be an illegal seal.
    pub statements_hash: Option<String>,
    pub owning_run_id: Option<String>,
    pub actor: String,
    pub ts: String,
    pub hash: Hash,
}

/// A requested `fiscal_year` CAS transition (`SealYear`). The store additionally
/// verifies — inside the same writer lock — that every `required_sealed_period` is
/// `Sealed`, so the year cannot lock over an unsealed subperiod (the atomicity that
/// makes the precondition race-free against a concurrent period transition).
pub(crate) struct FiscalYearTransitionRequest {
    pub year: i32,
    pub from: FiscalYearState,
    pub to: FiscalYearState,
    pub required_sealed_periods: Vec<String>,
    pub statements_hash: Option<String>,
    pub run_id: String,
    pub actor: String,
}

/// One immutable event on the append-only, hash-chained `adjustment_attribution`
/// log (issue `closeyear-adjustment-attribution`). Each event binds a sealed
/// voucher to the **statement year** whose tilinpäätös it belongs to, which may
/// differ from the calendar year its `posting_date` falls in (a year-end
/// poistot/jaksotukset/D7 adjustment posts to the *next open* period under A10 but
/// must affect the *closing* year's result). There is at most one attribution per
/// voucher (`UNIQUE(voucher_number)`), so the log doubles as the authoritative
/// current state — like the rejection log, and unlike the mutable period/year rows.
/// `verify` walks the chain and checks `fold` continuity.
#[derive(Debug, Clone)]
pub(crate) struct StoredAttribution {
    pub seq: i64,
    pub voucher_number: u64,
    /// The statement year this voucher's postings are attributed to.
    pub effective_year: i32,
    pub prev_hash: Hash,
    /// The close-year run that recorded the attribution.
    pub run_id: String,
    pub actor: String,
    pub ts: String,
    pub hash: Hash,
}

/// A requested effective-year attribution (issue `closeyear-adjustment-attribution`).
/// The store binds `voucher_number → effective_year` as one immutable, hash-chained
/// event inside the writer lock, refusing (each with an audited rejection): any
/// fiscal year in the affected interval `[effective_year, posting_year]` that is
/// already `Sealed` (attributing across it would silently change a sealed year's
/// current activity or opening balances, diverging its bound `statements_hash`), and
/// a re-attribution to a *different* year (attribution is immutable; a repeat to the
/// *same* year is an idempotent replay). The Ledger validates voucher existence and
/// derives `posting_year` before calling (a sealed voucher is immutable, so that read
/// cannot race); the interval seal-guard must run in-lock, hence `posting_year` here.
pub(crate) struct AttributionRequest {
    pub voucher_number: u64,
    pub effective_year: i32,
    /// The calendar year the voucher's `posting_date` falls in — the far end of the
    /// affected interval the in-lock seal-guard scans.
    pub posting_year: i32,
    pub run_id: String,
    pub actor: String,
    /// Deterministic audit key (`sha256(attribution\0number\0year\0run_id)`), bound
    /// into any rejection this request produces so duplicate refusals are
    /// distinguishable in the append-only rejection log.
    pub request_hash: String,
}

/// Outcome of an [`AttributionRequest`] commit.
pub(crate) enum AttributionResult {
    /// A fresh attribution was recorded.
    Recorded(StoredAttribution),
    /// The voucher was already attributed to the *same* year — idempotent replay.
    Replayed(StoredAttribution),
}

/// One entry of the append-only, hash-chained rejection log (O-25).
#[derive(Debug, Clone)]
pub(crate) struct StoredRejection {
    pub seq: i64,
    pub prev_reject_hash: Hash,
    pub reason_code: String,
    pub request_hash: String,
    pub actor: String,
    pub target: String,
    pub note: String,
    pub ts: String,
    pub hash: Hash,
}

/// A requested fiscal-year definition (issue `non-calendar-fiscal-year`).
pub(crate) struct FiscalYearDefinitionRequest {
    pub year: FiscalYear,
    pub actor: String,
}

/// Outcome of a definition: the resulting calendar, and whether it was already there.
pub(crate) enum DefinitionResult {
    Recorded(FiscalCalendar),
    Replayed(FiscalCalendar),
}

/// A single-transaction snapshot for `verify` (ADR-1: reads must be consistent,
/// not interleaved with concurrent seals).
pub(crate) struct FiscalSnapshot {
    pub vouchers: Vec<StoredVoucher>,
    /// The fiscal calendar, read in the same transaction as the vouchers it places.
    pub calendar: FiscalCalendar,
    pub head_number: u64,
    pub head_hash: Hash,
    pub rejections: Vec<StoredRejection>,
    /// Authoritative period rows + their transition log, for the ADR-6
    /// `fold(period_transition) == period` verify check (F18).
    pub periods: Vec<PeriodRow>,
    pub period_transitions: Vec<StoredPeriodTransition>,
    /// Authoritative fiscal-year rows + their transition log, for the S6
    /// `fold(fiscal_year_transition) == fiscal_year` verify check.
    pub fiscal_years: Vec<FiscalYearRow>,
    pub fiscal_year_transitions: Vec<StoredFiscalYearTransition>,
    /// The append-only effective-year attribution log (issue
    /// `closeyear-adjustment-attribution`), ordered by `seq`. Drives both the
    /// statement-year override in `year_vouchers` and its own `verify` chain walk.
    pub attributions: Vec<StoredAttribution>,
}

/// Raised when a `natural_key` is reused for changed projected or source facts.
pub(crate) struct IdempotencyConflict {
    pub existing: StoredVoucher,
    pub existing_request_hash: String,
    pub incoming_source_fingerprint: Option<String>,
    pub source_conflict: bool,
}

/// A refused write, recorded append-only (O-25). Carries no raw PII or secrets
/// (ADR-1 §Rejection log, guardrail 2) — only a `request_hash`, a reason code,
/// and safe context.
#[derive(Debug, Clone)]
pub(crate) struct RejectionRecord {
    pub reason_code: String,
    pub request_hash: String,
    pub actor: String,
    pub target: String,
    pub note: String,
}

/// Fiscal namespace port (ADR-1: SQLite-primary, single ACID transaction for
/// gapless-CAS numbering + hash-chain advance + idempotency + rejection log).
pub(crate) trait FiscalStore {
    /// Atomically allocate the next gapless number, advance the hash chain and
    /// persist the voucher — or, if `natural_key` was already sealed, replay the
    /// existing result. The `build` closure produces the canonical bytes and
    /// hash for `(number, prev_hash)`; it is invoked **inside** the transaction.
    fn commit(
        &self,
        provenance: &StoredProvenance,
        gate: &PeriodGate<'_>,
        build: &dyn Fn(u64, &Hash) -> Result<BuiltVoucher, LedgerError>,
    ) -> Result<CommitResult, LedgerError>;

    /// Append a refusal to the hash-chained rejection log. Must be durable
    /// before the caller returns a `Refused` (guardrail 1).
    fn record_rejection(&self, rej: &RejectionRecord) -> Result<(), LedgerError>;

    /// Read the authoritative `period` row (ADR-6 §1). A period with no row is
    /// implicitly `Open` and unowned; the returned [`PeriodRow`] reflects that.
    fn read_period(&self, period_id: &str) -> Result<PeriodRow, LedgerError>;

    /// Perform a period CAS transition inside the `BEGIN IMMEDIATE` writer lock
    /// (ADR-6 §2/§5): pin `state = from` (+ optional owner / unfiled guards), CAS
    /// to `to`, append one immutable `period_transition` event, advance the row.
    /// On a lost CAS the refusal is logged in the rejection namespace inside the
    /// same transaction, then `PERIOD_TRANSITION_CONFLICT`/`VOUCHER_NOT_IN_RUN` is
    /// returned — no partial state.
    /// A period CAS transition **or** the VAT-filing self-edge, depending on
    /// `req.set_filed` (S3 folds filing into the same audited hash-chained log).
    fn transition_period(&self, req: &TransitionRequest) -> Result<PeriodRow, LedgerError>;

    /// The `sent_hash` of a period's latest VAT-filing self-edge (`Sealed→Sealed`),
    /// for the idempotent-refile check. `Ok(None)` = no filing edge; `Ok(Some(h))` =
    /// filed, with `h` the stored `sent_hash` (itself `None` for a manual mark).
    fn last_filing_sent_hash(&self, period_id: &str)
        -> Result<Option<Option<String>>, LedgerError>;

    fn last_filing_receipt(&self, period_id: &str)
        -> Result<Option<(String, String)>, LedgerError>;

    fn last_manual_filing_evidence(
        &self,
        period_id: &str,
    ) -> Result<Option<(String, String, Option<String>)>, LedgerError>;

    /// Read the authoritative `fiscal_year` row (S6). A year with no row is
    /// implicitly `Open`.
    fn read_fiscal_year(&self, year: i32) -> Result<FiscalYearRow, LedgerError>;

    /// `SealYear`: inside the `BEGIN IMMEDIATE` writer lock, verify every
    /// `required_sealed_period` is `Sealed`, then CAS the `fiscal_year` row
    /// `from → to`, append one immutable `fiscal_year_transition` event and advance
    /// the row. On a lost CAS or an unsealed subperiod the refusal is logged in the
    /// rejection namespace inside the same transaction, then the error is returned —
    /// no partial state, atomic against a concurrent voucher/period write.
    fn transition_fiscal_year(
        &self,
        req: &FiscalYearTransitionRequest,
    ) -> Result<FiscalYearRow, LedgerError>;

    /// The fiscal calendar derived from the append-only definitions (calendar years
    /// when there are none).
    fn fiscal_calendar(&self) -> Result<FiscalCalendar, LedgerError>;

    /// Append a fiscal-year definition inside the `BEGIN IMMEDIATE` writer lock. An
    /// identical existing definition replays. Refuses (audited, `FISCAL_YEAR_CONFLICT`)
    /// a definition that would move a sealed voucher to another fiscal year or change
    /// the dates of a year with recorded state; an invalid calendar is a model error.
    fn define_fiscal_year(
        &self,
        req: &FiscalYearDefinitionRequest,
    ) -> Result<DefinitionResult, LedgerError>;

    /// Read one sealed voucher by number, or `None` if no such number is sealed.
    /// The attribution write path uses it (outside no extra lock: a sealed
    /// voucher is immutable) to read the voucher's `posting_date` year before
    /// binding an effective year to it.
    fn read_voucher(&self, number: u64) -> Result<Option<StoredVoucher>, LedgerError>;

    /// Bind a voucher to a statement year (issue `closeyear-adjustment-attribution`)
    /// as one immutable, hash-chained event, inside the `BEGIN IMMEDIATE` writer
    /// lock so the "no affected year Sealed" guard and the append are atomic against a
    /// concurrent `SealYear`. Refuses (with an audited rejection bound to
    /// `req.request_hash`): any `Sealed` fiscal year in `[effective_year, posting_year]`
    /// (`PERIOD_TRANSITION_CONFLICT`), or a re-attribution to a *different* year
    /// (`IDEMPOTENCY_KEY_REUSE`). A repeat to the same year replays. Voucher existence
    /// is the Ledger's precondition (audited there); the FK on `voucher_number` is the
    /// last-resort backstop, surfacing as a loud `STORE_ERROR` if that path is bypassed.
    fn attribute_adjustment(
        &self,
        req: &AttributionRequest,
    ) -> Result<AttributionResult, LedgerError>;

    /// A single-transaction snapshot of vouchers + head + rejections, for a
    /// `verify` that cannot be tricked into a spurious break by a concurrent seal.
    fn verification_snapshot(&self) -> Result<FiscalSnapshot, LedgerError>;

    /// Count of rejection-log entries (for tests/observability).
    fn rejection_count(&self) -> Result<u64, LedgerError>;
}

/// Artifact namespace port (F8): content-addressed (`sha256`) blob store.
pub(crate) trait ArtifactStore {
    /// Store `plaintext`, returning its content address (`sha256` of plaintext).
    fn put(&self, plaintext: &[u8]) -> Result<Hash, LedgerError>;
    fn exists(&self, addr: &Hash) -> Result<bool, LedgerError>;
    /// Read a blob back. Part of the port contract (exercised by the artifact
    /// contract test); the S1 CLI never reads blobs, so the non-test lib build
    /// sees no caller. Later slices (e.g. serving attachments) use it.
    #[allow(dead_code)]
    fn get(&self, addr: &Hash) -> Result<Vec<u8>, LedgerError>;
}

/// PII masterdata namespace port (ADR-1 §Masterdata, F7). The erasable, encrypted
/// employee store — a **distinct** port from the git-native reference masterdata: it
/// serves personal data (identity, HETU, IBAN, tax cards, YTD) that never lands in
/// git. Implemented by `sqlite_pii` over the `Cipher` seam.
pub(crate) trait EmployeeStore {
    /// Insert or replace an employee record (F7 master-data mutation).
    fn upsert(&self, state: &EmployeeState) -> Result<(), LedgerError>;
    /// Read one employee record, or `None` if unknown.
    fn get(&self, employee_id: &str) -> Result<Option<EmployeeState>, LedgerError>;
    /// List known employee ids (non-PII).
    fn list_ids(&self) -> Result<Vec<String>, LedgerError>;
    /// Record `add` as one **year-scoped** earnings entry for `tax_year`, idempotent
    /// on `(employee_id, tax_year, natural_key)`, and return the employee's new
    /// year-to-date gross **for that tax year**. Idempotent: a repeat with the same
    /// key is a no-op (returns the current year total unchanged), so a crash/retry
    /// between a voucher seal and this call can never double- or under-count; and a
    /// later tax year accumulates on its own row rather than onto the prior year.
    /// A reused key carrying a *different* gross is `IDEMPOTENCY_KEY_REUSE` (a caller
    /// bug — not a silent replay). `EMPLOYEE_NOT_FOUND` if the employee is unknown.
    /// Additive-only in this slice: corrections/reversals (signed deltas) are the
    /// issue's later reconcile work.
    fn accumulate_ytd(
        &self,
        employee_id: &str,
        tax_year: i32,
        natural_key: &str,
        add: &Money,
    ) -> Result<Money, LedgerError>;
    /// The employee's year-to-date gross for `tax_year`, derived from the earnings
    /// ledger (zero if nothing was recorded that year). `EMPLOYEE_NOT_FOUND` if the
    /// employee is unknown.
    fn ytd_gross(&self, employee_id: &str, tax_year: i32) -> Result<Money, LedgerError>;
}

/// Reference / non-PII masterdata namespace port (ADR-1 §Masterdata, revised
/// git-native): chart of accounts + account kinds + content-hashed
/// `chart_version`. Read-mostly, reviewable, non-personal — persisted as
/// plaintext git files (`git_masterdata`), not SQLite. The PII masterdata
/// boundary (HETU/IBAN/employees → separate erasable encrypted SQLite, F7) is a
/// distinct port and is not built in this slice.
pub(crate) trait MasterDataRefStore {
    fn chart_version(&self) -> Result<String, LedgerError>;
    fn list_accounts(&self) -> Result<Vec<Account>, LedgerError>;
    /// Resolve an immutable chart snapshot by the exact content version pinned in a
    /// sealed voucher. Never substitutes the active chart for another version.
    fn list_accounts_at(&self, chart_version: &str) -> Result<Vec<Account>, LedgerError>;
    fn account_exists(&self, id: &AccountId) -> Result<bool, LedgerError>;
}
