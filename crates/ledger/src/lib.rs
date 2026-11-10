//! # aggountant-ledger (F5)
//!
//! The **single** ledger write-port (`candidate.md §2/§4`). Both the CLI
//! primitive (`voucher seal`) and — in later slices — the workflow effect
//! (`Effect::RecordVoucher`) call this one service. There is no second write
//! path to the store: the storage ports (`store` module) are `pub(crate)` and
//! never leak outside this crate (F16).
//!
//! S1 responsibilities: masterdata reads (chart, F7), artifact blob writes (F8),
//! a validating dry-run, and the atomic seal (gapless CAS numbering + hash-chain
//! advance + idempotency + provenance + rejection log, on SQLite per ADR-1).
//! Out of scope: periods/period-state, Runs, gates, VAT calculation.

#![forbid(unsafe_code)]

mod employee;
mod error;
mod period;
mod reports;
mod store;
mod voucher_read;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};

use aggountant_domain::{
    compute_statements, kpa_policy, kpa_policy_version, tase_erittelyt, PolicySet,
    StatementProvenance,
};
use aggountant_model::{
    Account, AccountId, AccountKind, Currency, Date, Hash, Money, OpeningBalance,
    OpeningBalanceSource, PeriodId, Posting, Side, VoucherBody, VoucherDraft,
};

pub use employee::EmployeeState;
pub use error::LedgerError;
pub use period::{
    AdjustmentAttributionView, ClosingAuthority, FilingAuthority, FiscalYearState, FiscalYearView,
    ManualVatFilingEvidence, PeriodState, PeriodView, VatFilingReceipt,
};
pub use reports::{
    render_monthly_report_html, BalanceSection, IncomeSection, MonthlyReportOptions,
    MonthlyReportSections, MonthlyStatementsReport, ReportAccount, ReportCompleteness,
    ReportProvenance, ReportRow, ReportSourceRef, ReportStatus, ReportVoucherSource,
    MONTHLY_REPORT_SCHEMA_VERSION,
};
pub use voucher_read::{
    VoucherListQuery, VoucherListView, VoucherProvenanceView, VoucherSummary, VoucherView,
};

use store::fs_artifact::FsArtifactStore;
use store::git_masterdata::GitMasterDataRefStore;
use store::sqlite_fiscal::SqliteFiscalStore;
use store::sqlite_fiscal::{
    attribution_hash, fiscal_year_transition_hash, period_transition_hash, rejection_hash,
};
use store::sqlite_pii::SqlitePiiStore;
use store::{
    ArtifactStore, AttributionRequest, AttributionResult, BuiltVoucher, CommitResult,
    EmployeeStore, FiscalStore, FiscalYearTransitionRequest, MasterDataRefStore, PeriodGate,
    PeriodRow, RejectionRecord, StoredAttribution, StoredFiscalYearTransition,
    StoredPeriodTransition, StoredProvenance, StoredVoucher, TransitionRequest,
};

fn eur() -> Currency {
    Currency::new("EUR").expect("EUR is a valid currency code")
}

const STATEMENT_ACCOUNT_ROLES_JSON: &str = include_str!("../data/statement-account-roles.json");

fn statement_account_roles_hash() -> Hash {
    Hash::parse_hex(&sha256_hex(STATEMENT_ACCOUNT_ROLES_JSON.as_bytes()))
        .expect("sha256_hex produces a model hash")
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StatementAccountRolesFile {
    schema_version: u32,
    charts: Vec<StatementAccountRoles>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StatementAccountRoles {
    chart_version: String,
    retained_earnings_account: AccountId,
}

/// Resolve the retained-earnings semantic role from versioned policy data. The
/// account number is never a Ledger constant, and an unknown chart fails closed.
fn retained_earnings_account(
    chart_version: &str,
    chart: &[Account],
) -> Result<AccountId, LedgerError> {
    static ROLES: OnceLock<StatementAccountRolesFile> = OnceLock::new();
    let roles = ROLES.get_or_init(|| {
        serde_json::from_str(STATEMENT_ACCOUNT_ROLES_JSON)
            .expect("bundled statement account roles parse")
    });
    if roles.schema_version != 1 {
        return Err(LedgerError::MasterDataCorrupt(format!(
            "unsupported statement account-role schema {}",
            roles.schema_version
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in &roles.charts {
        if !seen.insert(entry.chart_version.as_str()) {
            return Err(LedgerError::MasterDataCorrupt(format!(
                "duplicate retained-earnings role snapshot for {:?}",
                entry.chart_version
            )));
        }
    }
    let role = roles
        .charts
        .iter()
        .find(|entry| entry.chart_version == chart_version)
        .ok_or_else(|| {
            LedgerError::MasterDataCorrupt(format!(
                "chart_version {chart_version:?} has no retained-earnings role snapshot"
            ))
        })?;
    match chart
        .iter()
        .find(|account| account.id == role.retained_earnings_account)
    {
        Some(account) if account.kind == AccountKind::Equity => {
            Ok(role.retained_earnings_account.clone())
        }
        Some(account) => Err(LedgerError::MasterDataCorrupt(format!(
            "retained-earnings role account {} must be equity, got {:?}",
            account.id, account.kind
        ))),
        None => Err(LedgerError::MasterDataCorrupt(format!(
            "retained-earnings role account {} is absent from chart_version {chart_version}",
            role.retained_earnings_account
        ))),
    }
}

/// Exact domain-separation prefix bytes for the close-package hash pre-image (issue
/// `closeyear-statement-completeness`): the label, a NUL, the format version, a NUL.
/// Specified as literal bytes (not split between a `&str` and a separate update) so any
/// re-implementation can reproduce the digest. Bumping `v1` deliberately re-keys every
/// package hash so a format change can never silently alias an old one.
const CLOSE_PACKAGE_HASH_PREFIX: &[u8] = b"aggountant-close-package\x00v1\x00";

/// The load-bearing component labels, in fixed pre-image order. These bytes enter the
/// hash, so renaming one (e.g. `tase_erittelyt` → `tase-erittelyt`) silently re-keys
/// every package hash — treat a change here as deliberately as bumping the version.
const CLOSE_PACKAGE_COMPONENTS: [&str; 3] = ["statements", "tase_erittelyt", "liitetiedot"];

/// True iff `hash` is a 64-char lowercase-hex sha256 digest — the shape every bound
/// `statements_hash` and every close-package component hash must have.
fn is_sha256_hex(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Compose the three deterministic close-package component hashes — the financial
/// statements (`tuloslaskelma` + `tase`), the `tase-erittelyt`, and the determined
/// `liitetiedot` — into the single **close-package hash** a `SealYear` should bind
/// (issue `closeyear-statement-completeness`, part 3).
///
/// `seal_year` today binds only the statements hash, so a sealed year does not attest
/// the erittelyt or the notes it was closed with. This composition is the canonical
/// pre-image that folds all three under one sha256, owned by the write-port that binds
/// it into the hash-chained year-lock event. Properties:
///
/// - **Domain-separated:** the fixed prefix means a package hash is not a pre-image of
///   any bare component hash (structural aliasing is impossible; equal *outputs* remain
///   possible only under a sha256 collision, assumed infeasible).
/// - **Unambiguous:** the component count (3) and labels are fixed and each digest is a
///   fixed 64 bytes, so the mapping from the three inputs to the pre-image is injective;
///   the `name=hash\0` framing keeps it injective if a component is ever made
///   variable-length (NUL never appears in a validated hex digest).
/// - **Order-fixed & tamper-evident:** the order is (statements, erittelyt,
///   liitetiedot); changing any component is expected to change the package hash.
///
/// Each component must be a 64-char lowercase-hex sha256 digest (e.g.
/// [`Statements::statements_hash`](aggountant_domain::Statements) and the sibling
/// component hashes); a malformed component is refused (`INVALID_HASH`) rather than
/// folded, so the package hash is meaningful.
pub fn close_package_hash(
    statements_hash: &str,
    tase_erittelyt_hash: &str,
    liitetiedot_hash: &str,
) -> Result<String, LedgerError> {
    let hashes = [statements_hash, tase_erittelyt_hash, liitetiedot_hash];
    let mut hasher = Sha256::new();
    hasher.update(CLOSE_PACKAGE_HASH_PREFIX);
    for (name, hash) in CLOSE_PACKAGE_COMPONENTS.iter().zip(hashes) {
        if !is_sha256_hex(hash) {
            return Err(LedgerError::InvalidHash(format!(
                "close_package_hash: {name} component must be 64 lowercase hex chars, got {hash:?}"
            )));
        }
        hasher.update(name.as_bytes());
        hasher.update(b"=");
        hasher.update(hash.as_bytes());
        hasher.update([0u8]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Build-time commit description used for sealing provenance. The workspace build
/// script obtains this with `git describe --always --dirty --long`; source archives without
/// git metadata degrade explicitly to `unknown-no-git`.
pub const SOFTWARE_VERSION: &str = env!("AGGOUNTANT_SOFTWARE_VERSION");

/// Inputs a caller supplies to `seal` beyond the draft itself.
pub struct SealRequest {
    /// Source-based idempotency key (`candidate.md §3`), e.g.
    /// `<company>:<artifact_hash>`. Must not change across a safe retry.
    pub natural_key: String,
    /// Who is sealing (recorded in provenance).
    pub actor: String,
    /// Policy set stamped into provenance (F6 stub).
    pub policy_set: PolicySet,
}

/// A sealed fiscal record, as returned to the CLI (`--json`).
#[derive(Debug, Clone, Serialize)]
pub struct SealedVoucher {
    pub number: u64,
    pub hash: String,
    pub prev_hash: String,
    pub chart_version: String,
    pub policy_set: String,
    /// Commit description of the binary that originally sealed this voucher.
    pub software_version: String,
    /// Fingerprint of source-event facts supplied before voucher projection.
    /// `None` identifies legacy and direct seals that did not carry a source event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_fingerprint: Option<String>,
    pub natural_key: String,
    pub actor: String,
    pub sealed_at: String,
    /// True when this was an idempotent replay of a prior seal (same `natural_key`).
    pub replayed: bool,
}

impl SealedVoucher {
    fn from_stored(v: StoredVoucher, replayed: bool) -> Self {
        SealedVoucher {
            number: v.number,
            hash: v.hash.to_hex(),
            prev_hash: v.prev_hash.to_hex(),
            chart_version: v.chart_version,
            policy_set: v.provenance.policy_set,
            software_version: v.provenance.software_version,
            source_fingerprint: v.provenance.source_fingerprint,
            natural_key: v.provenance.natural_key,
            actor: v.provenance.actor,
            sealed_at: v.sealed_at,
            replayed,
        }
    }
}

/// Result of the `validate` dry-run. Deliberately carries **no** number
/// prediction — `would_allocate` is a TOCTOU trap (`candidate.md §5`).
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub ok: bool,
    pub chart_version: String,
    pub posting_count: usize,
    pub currency: String,
    pub attachment_present: bool,
}

/// One contiguous voucher-number span sealed by the same software build.
///
/// `verify --json` reports ranges rather than repeating the same revision for
/// every voucher, so output scales with build transitions instead of ledger size.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SoftwareVersionSpan {
    pub first_number: u64,
    pub last_number: u64,
    pub software_version: String,
}

/// One consistent set of first-class inputs to a fiscal-year projection.
#[derive(Debug, Clone)]
pub struct FiscalYearData {
    pub opening_balance: Option<OpeningBalance>,
    /// Projection-only voucher views. Attributed adjustments may be re-dated into
    /// their statement year; use [`Ledger::year_vouchers`] for original sealed bodies.
    pub vouchers: Vec<VoucherBody>,
}

/// Result of `verify` — a full chain walk.
#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    pub ok: bool,
    pub voucher_count: u64,
    pub head_number: u64,
    pub head_hash: String,
    /// Build-provenance ranges read from outside the fiscal hash preimage.
    pub software_version_spans: Vec<SoftwareVersionSpan>,
    /// Number of periods whose `fold(period_transition) == period` check passed
    /// (ADR-6 §1, F18).
    pub period_count: u64,
    /// Number of fiscal years whose `fold(fiscal_year_transition) == fiscal_year`
    /// check passed (S6).
    pub fiscal_year_count: u64,
    /// Number of effective-year attribution events whose hash chain verified (issue
    /// `closeyear-adjustment-attribution`).
    pub attribution_count: u64,
}

/// Project an authoritative `fiscal_year` row into the public read view (S6).
fn row_to_year_view(row: store::FiscalYearRow) -> FiscalYearView {
    FiscalYearView {
        year: row.year,
        state: row.state,
        statements_hash: row.statements_hash,
        sealed_by_run_id: row.owning_run_id,
        head_seq: row.head_seq,
        head_hash: row.head_hash.to_hex(),
    }
}

/// Project a stored attribution event into the public read view (issue
/// `closeyear-adjustment-attribution`).
fn attribution_to_view(row: StoredAttribution, replayed: bool) -> AdjustmentAttributionView {
    AdjustmentAttributionView {
        voucher_number: row.voucher_number,
        effective_year: row.effective_year,
        owning_run_id: row.run_id,
        actor: row.actor,
        recorded_at: row.ts,
        replayed,
    }
}

/// Project an authoritative `period` row into the public read view (ADR-6 §6).
fn row_to_view(row: PeriodRow) -> PeriodView {
    PeriodView {
        period: row.period_id,
        state: row.state,
        owning_run_id: row.owning_run_id,
        vat_filed: row.vat_filed,
        head_transition_seq: row.head_seq,
        head_transition_hash: row.head_hash.to_hex(),
    }
}

pub struct Ledger {
    fiscal: SqliteFiscalStore,
    artifact: FsArtifactStore,
    masterdata: GitMasterDataRefStore,
    /// The erasable encrypted PII store (F7 employees). Distinct boundary from the
    /// git-native reference masterdata.
    pii: SqlitePiiStore,
}

/// The path of the encrypted PII store within a company home.
fn pii_path(home: &Path) -> std::path::PathBuf {
    home.join("masterdata").join("pii.sqlite")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// The git-native reference masterdata is meant to be committed, but it lives in
/// the same home as the fiscal SQLite DB, the artifact blobs and (future) the
/// erasable PII store — none of which may ever land in git (GDPR / large blobs).
/// Write a `.gitignore` at the home root so `git add .` in the home commits the
/// reviewable chart but not the sensitive stores. Best-effort and idempotent: we
/// never overwrite an existing `.gitignore` (the operator may have curated it).
fn write_home_gitignore(home: &Path) {
    let path = home.join(".gitignore");
    if path.exists() {
        return;
    }
    let body = "\
# aggountant company home — commit `masterdata/reference/` (git-native chart),
# never the fiscal DB, artifact blobs, or the erasable PII store.
/fiscal.sqlite
/fiscal.sqlite-*
/artifacts/
/masterdata/pii.sqlite
/masterdata/pii.sqlite-*
";
    let _ = std::fs::write(path, body);
}

impl Ledger {
    /// Bootstrap a company home: create the fiscal DB + artifact store, and
    /// **write** the git-native reference masterdata tree (chart of accounts +
    /// account kinds + content-hashed `chart_version`) as diffable plaintext.
    ///
    /// This is the masterdata WRITE path (`aggountant init`). It is safe to re-run:
    /// if the home already holds reference masterdata it fails with
    /// `ALREADY_INITIALIZED` rather than clobbering the (possibly hand-curated,
    /// git-tracked) chart.
    pub fn init(home: &Path) -> Result<Ledger, LedgerError> {
        std::fs::create_dir_all(home).map_err(|e| LedgerError::Store(e.to_string()))?;
        // Masterdata init is the refuse-to-clobber gate — do it FIRST, so a
        // re-init against an existing home fails before we create any fiscal /
        // artifact state (no partial-home litter on refusal).
        let masterdata = GitMasterDataRefStore::init(&home.join("masterdata"))?;
        write_home_gitignore(home);
        let fiscal = SqliteFiscalStore::open(&home.join("fiscal.sqlite"))?;
        let artifact = FsArtifactStore::open(&home.join("artifacts"))?;
        let pii = SqlitePiiStore::open(&pii_path(home))?;
        Ok(Ledger {
            fiscal,
            artifact,
            masterdata,
            pii,
        })
    }

    /// Open an already-initialized company home for reading + sealing. The
    /// git-native reference masterdata is loaded read-only (no seeding side
    /// effect — the WRITE path is `init`); a home that was never initialized
    /// fails with `MASTERDATA_NOT_INITIALIZED`. Cheap to open many times against
    /// the same `home` — the concurrency test relies on independent connections
    /// racing on the fiscal file.
    pub fn open(home: &Path) -> Result<Ledger, LedgerError> {
        std::fs::create_dir_all(home).map_err(|e| LedgerError::Store(e.to_string()))?;
        let fiscal = SqliteFiscalStore::open(&home.join("fiscal.sqlite"))?;
        let artifact = FsArtifactStore::open(&home.join("artifacts"))?;
        let masterdata = GitMasterDataRefStore::open(&home.join("masterdata"))?;
        let pii = SqlitePiiStore::open(&pii_path(home))?;
        Ok(Ledger {
            fiscal,
            artifact,
            masterdata,
            pii,
        })
    }

    // --- F7 employee master-data (PII-isolated, encrypted store) ----------

    /// Insert or replace an employee's master data (F7 mutation). PII lands only in
    /// the encrypted store, never in git.
    pub fn upsert_employee(&self, state: &EmployeeState) -> Result<(), LedgerError> {
        self.pii.upsert(state)
    }

    /// Read an employee's master data. `EMPLOYEE_NOT_FOUND` if unknown — the
    /// durable master-data read the RF4 Run performs at every effect (M4).
    pub fn get_employee(&self, employee_id: &str) -> Result<EmployeeState, LedgerError> {
        self.pii
            .get(employee_id)?
            .ok_or_else(|| LedgerError::EmployeeNotFound(employee_id.to_string()))
    }

    /// List known employee ids (non-PII).
    pub fn list_employees(&self) -> Result<Vec<String>, LedgerError> {
        self.pii.list_ids()
    }

    /// Record an employee's earnings for `tax_year` **once per `natural_key`** and
    /// return the new year-to-date gross **for that tax year** (the RF4 record effect
    /// calls this with the pay run's tax year + natural key once the voucher seals).
    /// Year-scoped: a 2027 pay run accumulates on its own year, never onto 2026.
    /// Idempotent: a retry with the same key is a no-op, so a crash between the seal
    /// and the advance can neither double- nor under-count.
    pub fn accumulate_ytd(
        &self,
        employee_id: &str,
        tax_year: i32,
        natural_key: &str,
        add: &Money,
    ) -> Result<Money, LedgerError> {
        self.pii
            .accumulate_ytd(employee_id, tax_year, natural_key, add)
    }

    /// An employee's year-to-date gross for `tax_year`, derived from the earnings
    /// ledger (zero for a year with no recorded earnings). `EMPLOYEE_NOT_FOUND` if the
    /// employee is unknown.
    pub fn ytd_gross(&self, employee_id: &str, tax_year: i32) -> Result<Money, LedgerError> {
        self.pii.ytd_gross(employee_id, tax_year)
    }

    /// List the chart of accounts (F7 read). `_at` is accepted for the
    /// `chart list --active-at` surface; S1 has a single chart version.
    pub fn chart_list(
        &self,
        _at: Option<aggountant_model::Date>,
    ) -> Result<Vec<Account>, LedgerError> {
        self.masterdata.list_accounts()
    }

    pub fn chart_version(&self) -> Result<String, LedgerError> {
        self.masterdata.chart_version()
    }

    /// The exact company-chart VAT role table loaded from git-native masterdata.
    pub fn vat_role_table(&self) -> &aggountant_domain::VatAccountRoleTable {
        self.masterdata.vat_roles()
    }

    /// Derive period VAT figures under the exact company-chart role snapshots loaded
    /// from git-native masterdata. Unknown, inactive, or incomplete roles fail closed.
    pub fn derive_vat_filing_figures(
        &self,
        period: &aggountant_model::PeriodId,
        vouchers: &[VoucherBody],
    ) -> Result<aggountant_domain::DerivedVatFigures, LedgerError> {
        Ok(aggountant_domain::derive_filing_figures_with_role_table(
            period,
            vouchers,
            self.masterdata.vat_roles(),
        )?)
    }

    /// Project one approved draft exactly as filing will read it after this ledger seals
    /// it under the active company chart.
    pub fn draft_deductible_vat(
        &self,
        draft: &VoucherDraft,
        chart_version: &str,
    ) -> Result<Money, LedgerError> {
        Ok(aggountant_domain::draft_deductible_vat_with_role_table(
            draft,
            chart_version,
            self.masterdata.vat_roles(),
        )?)
    }

    /// Store an artifact blob (F8), returning its content address (`sha256`).
    pub fn attach_artifact(&self, plaintext: &[u8]) -> Result<Hash, LedgerError> {
        self.artifact.put(plaintext)
    }

    /// Dry-run validation (Ledger, no write): every posting account exists in the
    /// resolved chart version, and any referenced attachment is present.
    pub fn validate(&self, draft: &VoucherDraft) -> Result<ValidationReport, LedgerError> {
        let chart_version = self.masterdata.chart_version()?;
        for p in draft.postings() {
            if !self.masterdata.account_exists(&p.account)? {
                return Err(LedgerError::AccountNotFound(format!(
                    "account {} not in chart_version {}",
                    p.account, chart_version
                )));
            }
        }
        // F25 record-time mistag guard: refuse a generic band VAT code on an account
        // whose supply has a dedicated rate-class treatment that diverges on this date.
        aggountant_domain::check_vat_rate_class_mistag_with_role_table(
            self.masterdata.vat_roles(),
            &chart_version,
            draft.document_date(),
            draft.postings(),
        )?;
        if let Some(attachment) = draft.attachment_ref() {
            if !self.artifact.exists(&attachment)? {
                return Err(LedgerError::ArtifactNotFound(attachment.to_hex()));
            }
        }
        Ok(ValidationReport {
            ok: true,
            chart_version,
            posting_count: draft.postings().len(),
            currency: draft.currency().as_str().to_string(),
            attachment_present: draft.attachment_ref().is_some(),
        })
    }

    /// The canonical request identity Ledger uses for idempotency comparisons.
    /// A stateless approval binds this value instead of inventing a second draft codec.
    pub fn draft_request_hash(&self, draft: &VoucherDraft) -> Result<String, LedgerError> {
        Ok(sha256_hex(&aggountant_wire::canonical_bytes(draft)?))
    }

    /// Seal a voucher: the atomic Ledger commit (F5). Refused writes are logged
    /// (O-25) before the error is returned (ADR-1 guardrail 1). Idempotent on
    /// `natural_key` — a repeat seal replays the existing voucher.
    pub fn seal(
        &self,
        draft: &VoucherDraft,
        req: &SealRequest,
    ) -> Result<SealedVoucher, LedgerError> {
        self.seal_with_provenance(draft, req, None, None, None)
    }

    /// Seal a voucher originating from an external source event.
    ///
    /// The source fingerprint is compared independently from the projected draft's
    /// `request_hash`. Reusing a `natural_key` with changed source facts is refused
    /// even when assignment overrides make the resulting voucher byte-identical.
    /// Exact retries replay normally. The fingerprint is provenance metadata and is
    /// deliberately outside the fiscal hash preimage.
    pub fn seal_with_source_fingerprint(
        &self,
        draft: &VoucherDraft,
        req: &SealRequest,
        source_fingerprint: &str,
    ) -> Result<SealedVoucher, LedgerError> {
        self.seal_with_provenance(draft, req, None, Some(source_fingerprint), None)
    }

    /// Seal source-bound content only if the active chart still equals the version an
    /// external stateless approval covered. The check happens inside the Ledger write
    /// boundary; changed masterdata cannot silently produce a different approved body.
    pub fn seal_with_source_fingerprint_at_chart(
        &self,
        draft: &VoucherDraft,
        req: &SealRequest,
        source_fingerprint: &str,
        expected_chart_version: &str,
    ) -> Result<SealedVoucher, LedgerError> {
        self.seal_with_provenance(
            draft,
            req,
            None,
            Some(source_fingerprint),
            Some(expected_chart_version),
        )
    }

    /// Seal a voucher on behalf of a close-run, presenting its `ClosingAuthority`
    /// (ADR-6 §2). The period gate accepts a Closing-period write **only** when
    /// the token owns the period; a bare `--run-id` cannot reach this path (the
    /// token is non-serialisable and un-forgeable). Ordinary seals call
    /// [`Ledger::seal`], which passes `None`.
    pub fn seal_with_authority(
        &self,
        draft: &VoucherDraft,
        req: &SealRequest,
        authority: Option<&ClosingAuthority>,
    ) -> Result<SealedVoucher, LedgerError> {
        self.seal_with_provenance(draft, req, authority, None, None)
    }

    fn seal_with_provenance(
        &self,
        draft: &VoucherDraft,
        req: &SealRequest,
        authority: Option<&ClosingAuthority>,
        source_fingerprint: Option<&str>,
        expected_chart_version: Option<&str>,
    ) -> Result<SealedVoucher, LedgerError> {
        let chart_version = self.masterdata.chart_version()?;
        let request_hash = self.draft_request_hash(draft)?;
        if expected_chart_version.is_some_and(|expected| expected != chart_version) {
            let err = LedgerError::ChartVersionChanged(format!(
                "approved chart_version {:?}, active chart_version {:?}",
                expected_chart_version.expect("checked Some"),
                chart_version
            ));
            return Err(self.refuse(err, &request_hash, req));
        }

        // Pre-commit validation. A failure here is a *refused write* → rejection
        // log, then surface the error (never a silent refusal).
        for p in draft.postings() {
            if !self.masterdata.account_exists(&p.account)? {
                let err = LedgerError::AccountNotFound(format!(
                    "account {} not in chart_version {}",
                    p.account, chart_version
                ));
                return Err(self.refuse(err, &request_hash, req));
            }
        }
        if let Some(expansion) = draft.vat_expansion() {
            if expansion.policy_set() != req.policy_set.id {
                let err =
                    LedgerError::Model(aggountant_model::ModelError::InvalidVatExpansion(format!(
                        "VAT expansion resolved policy_set {:?} differs from seal policy_set {:?}",
                        expansion.policy_set(),
                        req.policy_set.id
                    )));
                return Err(self.refuse(err, &request_hash, req));
            }
            let candidate = VoucherBody::from_draft(draft, 1, chart_version.clone())?;
            if let Err(error) = aggountant_domain::verify_vat_expansion_for_seal_with_role_table(
                &candidate,
                self.masterdata.vat_roles(),
            ) {
                return Err(self.refuse(LedgerError::Statements(error), &request_hash, req));
            }
        }
        // F25 record-time mistag guard: a generic band VAT code on an account whose
        // supply has a dedicated rate-class treatment that diverges on this date is a
        // refused write (logged, then surfaced — never a silent wrong-rate seal).
        if let Err(e) = aggountant_domain::check_vat_rate_class_mistag_with_role_table(
            self.masterdata.vat_roles(),
            &chart_version,
            draft.document_date(),
            draft.postings(),
        ) {
            return Err(self.refuse(LedgerError::from(e), &request_hash, req));
        }
        if let Some(attachment) = draft.attachment_ref() {
            if !self.artifact.exists(&attachment)? {
                let err = LedgerError::ArtifactNotFound(attachment.to_hex());
                return Err(self.refuse(err, &request_hash, req));
            }
        }

        let provenance = StoredProvenance {
            actor: req.actor.clone(),
            natural_key: req.natural_key.clone(),
            request_hash: request_hash.clone(),
            source_fingerprint: source_fingerprint.map(str::to_owned),
            policy_set: req.policy_set.id.clone(),
            software_version: SOFTWARE_VERSION.to_string(),
            chart_version: chart_version.clone(),
            // Attempt time (not commit-order): captured before the writer lock.
            sealed_at: chrono::Utc::now().to_rfc3339(),
            sent_hash: None,
        };

        // The pure build step, invoked inside the fiscal transaction.
        let build = |number: u64, prev: &Hash| -> Result<BuiltVoucher, LedgerError> {
            let body = VoucherBody::from_draft(draft, number, chart_version.clone())?;
            let (canonical, hash) = aggountant_wire::seal_voucher(prev, &body)?;
            Ok(BuiltVoucher {
                body,
                canonical,
                hash,
            })
        };

        // A date may be governed by either its canonical month or a retained legacy
        // quarter. The store checks both rows in the seal transaction (ADR-6 §2).
        let gate = PeriodGate {
            posting_date: draft.posting_date(),
            authority,
        };

        match self.fiscal.commit(&provenance, &gate, &build)? {
            CommitResult::Sealed(v) => Ok(SealedVoucher::from_stored(v, false)),
            CommitResult::Replayed(v) => Ok(SealedVoucher::from_stored(v, true)),
            // The period-gate refusal was already recorded inside the fiscal
            // transaction (ADR-6 §2); surface it without a second rejection write.
            CommitResult::Refused(err) => Err(err),
            CommitResult::Conflict(c) => {
                // Same natural_key, changed projected or source content: refuse and
                // retain an audit record rather than silently replaying stale facts.
                let err = if c.source_conflict && c.existing_request_hash == request_hash {
                    LedgerError::IdempotencyKeyReuse(format!(
                        "natural_key {:?} already sealed voucher #{} with source fingerprint \
                         {:?}, incoming {:?}; projected voucher content is unchanged",
                        req.natural_key,
                        c.existing.number,
                        c.existing.provenance.source_fingerprint,
                        c.incoming_source_fingerprint
                    ))
                } else {
                    LedgerError::IdempotencyKeyReuse(format!(
                        "natural_key {:?} already sealed voucher #{} with a different \
                         request_hash (existing {}, incoming {request_hash})",
                        req.natural_key, c.existing.number, c.existing_request_hash
                    ))
                };
                Err(self.refuse(err, &request_hash, req))
            }
        }
    }

    /// Record a refused write, then return the original error — or
    /// `AUDIT_UNAVAILABLE` if the rejection could not be persisted.
    fn refuse(&self, err: LedgerError, request_hash: &str, req: &SealRequest) -> LedgerError {
        let rec = RejectionRecord {
            reason_code: err.error_code().to_string(),
            request_hash: request_hash.to_string(),
            actor: req.actor.clone(),
            target: "fiscal:main".to_string(),
            note: err.to_string(),
        };
        match self.fiscal.record_rejection(&rec) {
            Ok(()) => err,
            Err(audit_err) => LedgerError::AuditUnavailable(format!(
                "failed to persist rejection for {}: {audit_err}",
                err.error_code()
            )),
        }
    }

    /// Record an attribution refusal caught in the Ledger layer (unknown voucher,
    /// or an effective year after the posting year) in the same append-only rejection
    /// log the store's in-lock refusals use, then return the original error — so
    /// *every* documented attribution guard is audited, not only the in-lock ones.
    fn refuse_attribution(
        &self,
        err: LedgerError,
        request_hash: &str,
        voucher_number: u64,
        actor: &str,
    ) -> LedgerError {
        let rec = RejectionRecord {
            reason_code: err.error_code().to_string(),
            request_hash: request_hash.to_string(),
            actor: actor.to_string(),
            target: format!("fiscal:attribution:{voucher_number}"),
            note: err.to_string(),
        };
        match self.fiscal.record_rejection(&rec) {
            Ok(()) => err,
            Err(audit_err) => LedgerError::AuditUnavailable(format!(
                "failed to persist rejection for {}: {audit_err}",
                err.error_code()
            )),
        }
    }

    pub fn rejection_count(&self) -> Result<u64, LedgerError> {
        self.fiscal.rejection_count()
    }

    /// The authoritative state of a period (`period show`; ADR-6 §6 derived view).
    /// A period that never transitioned reads back `Open` and unowned.
    pub fn period_view(&self, period: &PeriodId) -> Result<PeriodView, LedgerError> {
        Ok(row_to_view(self.fiscal.read_period(&period.to_string())?))
    }

    /// The sealed vouchers whose `posting_date` maps to `period` (ADR-6 §2 period
    /// mapping), strict-decoded from their canonical bytes. The read path for
    /// deriving a period's VAT aggregates (issue `vat-figures-from-sealed-ledger`):
    /// the figures a filing reports must be reproducible from the sealed ledger,
    /// not merely trusted from CLI input. Uses the same consistent snapshot as
    /// `verify`, so a concurrent seal cannot yield a torn read.
    pub fn period_vouchers(&self, period: &PeriodId) -> Result<Vec<VoucherBody>, LedgerError> {
        let snapshot = self.fiscal.verification_snapshot()?;
        let mut vouchers = Vec::new();
        for v in &snapshot.vouchers {
            // Strict decode (re-encode byte-equal) — a non-canonical stored body
            // is a tamper/corruption signal, not silently skipped.
            let body = aggountant_wire::decode_voucher_body(&v.canonical)?;
            if period.contains(body.posting_date()) {
                vouchers.push(body);
            }
        }
        Ok(vouchers)
    }

    /// Read one sealed voucher from one consistent fiscal snapshot. The canonical
    /// body is strict-decoded and checked against the row number, duplicated chart
    /// version, stored hash and separate provenance metadata. This is a local record
    /// integrity check, not a claim that the full chain was verified.
    pub fn voucher(&self, number: u64) -> Result<VoucherView, LedgerError> {
        if number == 0 {
            return Err(LedgerError::InvalidVoucherRead(
                "voucher number must be a positive integer, got 0".into(),
            ));
        }
        let snapshot = self.fiscal.verification_snapshot()?;
        let stored = snapshot
            .vouchers
            .iter()
            .find(|voucher| voucher.number == number)
            .ok_or_else(|| {
                LedgerError::VoucherNotFound(format!(
                    "no sealed voucher exists with number {number}"
                ))
            })?;
        voucher_read::decode_view(stored)
    }

    /// List sealed vouchers from one consistent fiscal snapshot, ordered by voucher
    /// number and bounded to at most 100 returned rows. Attachment and inclusive
    /// posting-date filters use AND semantics. Every stored voucher is strict-decoded
    /// before a result is returned, so corruption cannot become partial success.
    pub fn list_vouchers(&self, query: &VoucherListQuery) -> Result<VoucherListView, LedgerError> {
        voucher_read::validate_query(query)?;
        let snapshot = self.fiscal.verification_snapshot()?;
        // `verification_snapshot` returns voucher rows in number ASC order.
        let mut matches = Vec::with_capacity(query.limit.saturating_add(1));
        for stored in &snapshot.vouchers {
            let view = voucher_read::decode_view(stored)?;
            let attachment_matches = query
                .attachment_ref
                .is_none_or(|wanted| view.body.attachment_ref() == Some(wanted));
            let date_matches = match (query.posting_from, query.posting_to) {
                (Some(from), Some(to)) => {
                    let date = view.body.posting_date();
                    from <= date && date <= to
                }
                (None, None) => true,
                _ => unreachable!("query validation requires paired date bounds"),
            };
            if attachment_matches && date_matches && matches.len() <= query.limit {
                // Retain only limit + 1 rows: the extra row proves truthful
                // truncation while the loop continues strict-decoding every stored
                // body so later corruption cannot become partial success.
                matches.push(VoucherSummary {
                    number: view.number,
                    voucher_hash: view.voucher_hash,
                    document_date: view.body.document_date(),
                    posting_date: view.body.posting_date(),
                    attachment_ref: view.body.attachment_ref(),
                    match_status: "possible_source_match",
                });
            }
        }
        let truncated = matches.len() > query.limit;
        matches.truncate(query.limit);
        Ok(VoucherListView {
            count: matches.len(),
            truncated,
            order: "number_asc",
            vouchers: matches,
        })
    }

    /// First-class statement inputs for `year`, read from one consistent fiscal
    /// snapshot. `vouchers` contains sealed vouchers only; opening balances are
    /// separate domain data and never consume an in-band voucher number.
    pub fn fiscal_year_data(&self, year: i32) -> Result<FiscalYearData, LedgerError> {
        let snapshot = self.fiscal.verification_snapshot()?;
        let effective_of: BTreeMap<u64, i32> = snapshot
            .attributions
            .iter()
            .map(|a| (a.voucher_number, a.effective_year))
            .collect();
        let chart = self.chart_list(None)?;
        let active_chart_version = self.chart_version()?;
        let active_kinds: BTreeMap<String, AccountKind> = chart
            .iter()
            .map(|account| (account.id.as_str().to_string(), account.kind))
            .collect();
        let mut kinds_by_version: BTreeMap<String, BTreeMap<String, AccountKind>> = BTreeMap::new();
        let mut retained_earnings: Option<AccountId> = None;

        let mut vouchers = Vec::new();
        let mut prior_signed: BTreeMap<String, Decimal> = BTreeMap::new();
        let mut has_prior = false;
        for stored in &snapshot.vouchers {
            let body = aggountant_wire::decode_voucher_body(&stored.canonical)?;
            let posting_year = PeriodId::containing(body.posting_date()).year();
            let statement_year = effective_of
                .get(&stored.number)
                .copied()
                .unwrap_or(posting_year);
            // Resolve every statement-relevant voucher under its pinned immutable
            // snapshot. Additive/rename-only chart evolution is safe; a retired or
            // re-kinded touched account needs ADR-8's explicit mapping boundary.
            let mut source_kinds = None;
            if statement_year <= year {
                if !kinds_by_version.contains_key(body.chart_version()) {
                    let snapshot = self.masterdata.list_accounts_at(body.chart_version())?;
                    kinds_by_version.insert(
                        body.chart_version().to_string(),
                        snapshot
                            .into_iter()
                            .map(|account| (account.id.as_str().to_string(), account.kind))
                            .collect(),
                    );
                }
                let pinned = &kinds_by_version[body.chart_version()];
                for posting in body.postings() {
                    let source = pinned.get(posting.account.as_str());
                    let target = active_kinds.get(posting.account.as_str());
                    if source != target || source.is_none() {
                        return Err(LedgerError::MasterDataCorrupt(format!(
                            "account {} used by voucher {} is {:?} under pinned chart {} and \
                             {:?} under active chart {}; an explicit ExternalCutover opening \
                             balance with account_mapping_hash and live_period is required",
                            posting.account,
                            body.number(),
                            source,
                            body.chart_version(),
                            target,
                            active_chart_version
                        )));
                    }
                }
                source_kinds = Some(pinned);
            }
            if statement_year == year {
                if posting_year == year {
                    vouchers.push(body);
                } else {
                    vouchers.push(redate_into_year(&body, year)?);
                }
            } else if statement_year < year {
                has_prior = true;
                // Resolve semantics from the exact immutable snapshot pinned by the
                // voucher. A chart boundary additionally requires ADR-8's explicit
                // account mapping / live-period OpeningBalance bridge; equal-looking
                // account numbers are never mapped implicitly.
                let source_kinds = source_kinds.expect("prior voucher resolved above");
                for posting in body.postings() {
                    if posting.amount.currency() != &eur() {
                        return Err(LedgerError::Store(format!(
                            "opening balances are EUR-only, but voucher {} posts {}",
                            body.number(),
                            posting.amount.currency()
                        )));
                    }
                    let account = posting.account.as_str();
                    let kind = source_kinds.get(account).ok_or_else(|| {
                        LedgerError::AccountNotFound(format!(
                            "account {account} from chart_version {} cannot be resolved",
                            body.chart_version()
                        ))
                    })?;
                    // Classification is intentionally made under the voucher's pinned
                    // chart version (the exact snapshot checked above), not inferred
                    // from account-number ranges.
                    match kind {
                        AccountKind::Asset | AccountKind::Liability | AccountKind::Equity => {
                            *prior_signed.entry(account.to_string()).or_default() +=
                                posting.signed_amount();
                        }
                        AccountKind::Income | AccountKind::Expense => {
                            let retained = match &retained_earnings {
                                Some(account) => account,
                                None => retained_earnings.insert(retained_earnings_account(
                                    &active_chart_version,
                                    &chart,
                                )?),
                            };
                            *prior_signed
                                .entry(retained.as_str().to_string())
                                .or_default() += posting.signed_amount();
                        }
                    }
                }
            }
        }

        // State safety depends on prior authoritative activity, not its net amount:
        // an offsetting/unbalanced-later history must not bypass the Y-1 seal gate.
        let sealed_prior = if has_prior {
            let prior_year = year.checked_sub(1).ok_or_else(|| {
                LedgerError::Store(format!("cannot derive prior fiscal year for {year}"))
            })?;
            let prior = snapshot
                .fiscal_years
                .iter()
                .find(|row| row.year == prior_year)
                .ok_or_else(|| {
                    LedgerError::PeriodNotSealed(format!(
                        "fiscal year {prior_year} must be Sealed before its closing balances \
                         can open {year}"
                    ))
                })?;
            if prior.state != FiscalYearState::Sealed {
                return Err(LedgerError::PeriodNotSealed(format!(
                    "fiscal year {prior_year} is {} — SealYear is required before opening {year}",
                    prior.state.as_str()
                )));
            }
            Some((prior_year, prior))
        } else {
            None
        };

        let opening_balance = if prior_signed.values().all(Decimal::is_zero) {
            None
        } else {
            let (prior_year, prior) = sealed_prior.expect("nonzero prior fold has prior activity");
            let mut postings = Vec::new();
            for (account, signed) in prior_signed {
                if signed.is_zero() {
                    continue;
                }
                let (side, amount) = if signed > Decimal::ZERO {
                    (Side::Debit, signed)
                } else {
                    (Side::Credit, -signed)
                };
                postings.push(Posting {
                    account: AccountId::new(&account)?,
                    amount: Money::new(amount, eur()),
                    side,
                    vat_code: None,
                });
            }
            Some(OpeningBalance::new(
                year,
                postings,
                OpeningBalanceSource::PriorFiscalYear {
                    prior_year,
                    source_head_hash: prior.head_hash,
                    role_policy_hash: statement_account_roles_hash(),
                },
            )?)
        };

        Ok(FiscalYearData {
            opening_balance,
            vouchers,
        })
    }

    /// Sealed vouchers whose statement year is `year`. Opening balances are no
    /// longer spoofed into this collection; call [`Ledger::fiscal_year_data`] when
    /// building statements.
    pub fn year_vouchers(&self, year: i32) -> Result<Vec<VoucherBody>, LedgerError> {
        let snapshot = self.fiscal.verification_snapshot()?;
        let effective_of: BTreeMap<u64, i32> = snapshot
            .attributions
            .iter()
            .map(|a| (a.voucher_number, a.effective_year))
            .collect();
        let mut vouchers = Vec::new();
        for stored in &snapshot.vouchers {
            let body = aggountant_wire::decode_voucher_body(&stored.canonical)?;
            let posting_year = PeriodId::containing(body.posting_date()).year();
            let statement_year = effective_of
                .get(&stored.number)
                .copied()
                .unwrap_or(posting_year);
            if statement_year == year {
                vouchers.push(body);
            }
        }
        Ok(vouchers)
    }

    /// The authoritative fiscal-year state (`fiscal-year show`; S6). A year that
    /// never sealed reads back `Open`.
    pub fn fiscal_year_view(&self, year: i32) -> Result<FiscalYearView, LedgerError> {
        Ok(row_to_year_view(self.fiscal.read_fiscal_year(year)?))
    }

    /// The [`close_package_hash`] for `year`, with the two ledger-derivable components
    /// recomputed from the year's **sealed vouchers** — derived from the ledger, not
    /// trusted from caller input (the `vat-figures-from-sealed-ledger` principle) — and
    /// the non-derivable `liitetiedot` component supplied by the caller.
    ///
    /// `compute_statements` and `tase_erittelyt` fold the same `fiscal_year_data` the
    /// close-year Run projects (sealed vouchers + separate opening data); the determined
    /// `liitetiedot`, by contrast, fold process-held asset/depreciation inputs that are
    /// **not** in the sealed ledger, so their hash must be passed in. This is meant to be
    /// called **during the close-year Run** — after the year's vouchers are sealed but
    /// while the fiscal year is still `Open` — to produce the value `SealYear` binds, so
    /// a sealed year attests the full close package (statements + `tase-erittelyt` +
    /// `liitetiedot`), not the statements alone.
    ///
    /// **Reproducibility is conditional, not intrinsic.** The result is stable only for
    /// the *same sealed vouchers under the same context*: `policy_set` is caller-passed,
    /// `chart_version` is read live from the current masterdata, and `kpa_policy_version`
    /// is a compile-time constant. A chart edit, a different policy id, or a binary
    /// upgrade all change the hash. Pinning that context at seal time and re-deriving it
    /// on the `verify` path (which also needs the process-held `liitetiedot_hash`
    /// persisted) is the process-layer follow-up recorded on the issue — do **not** read
    /// this as "any future recomputation of a sealed year reproduces the bound bytes".
    ///
    /// `liitetiedot_hash` must be a 64-char lowercase-hex sha256 digest.
    pub fn close_package_hash_for_year(
        &self,
        year: i32,
        policy_set: &str,
        liitetiedot_hash: &str,
    ) -> Result<String, LedgerError> {
        // Reject a malformed notes hash up front, before the (comparatively expensive)
        // voucher fold — `close_package_hash` re-checks it as the last line of defense.
        if !is_sha256_hex(liitetiedot_hash) {
            return Err(LedgerError::InvalidHash(format!(
                "close_package_hash_for_year: liitetiedot_hash must be 64 lowercase hex chars, \
                 got {liitetiedot_hash:?}"
            )));
        }
        let data = self.fiscal_year_data(year)?;
        let chart = self.chart_list(None)?;
        let provenance = StatementProvenance {
            chart_version: self.chart_version()?,
            policy_set: policy_set.to_string(),
            kpa_policy_version: kpa_policy_version().to_string(),
        };
        // Domain projection failures propagate with their own code preserved (the
        // `LedgerError::Statements` variant wraps `DomainError`, not a flattened string).
        let statements = compute_statements(
            year,
            data.opening_balance.as_ref(),
            &data.vouchers,
            &chart,
            kpa_policy(),
            provenance.clone(),
        )?;
        let erittelyt = tase_erittelyt(
            year,
            data.opening_balance.as_ref(),
            &data.vouchers,
            &chart,
            provenance,
        )?;
        close_package_hash(
            &statements.statements_hash(),
            &erittelyt.statements_hash(),
            liitetiedot_hash,
        )
    }

    /// Canonical close coverage for a calendar year. An active monthly row selects
    /// all twelve months; otherwise retained legacy years use four quarters. Reopened
    /// `Open` remnants do not select a mode, so a year can deliberately switch only
    /// after all old coverage has been reopened.
    pub fn year_close_periods(&self, year: i32) -> Result<Vec<PeriodId>, LedgerError> {
        let snapshot = self.fiscal.verification_snapshot()?;
        let mut uses_months = false;
        for row in &snapshot.periods {
            let period = PeriodId::parse(&row.period_id).map_err(|error| {
                LedgerError::Store(format!(
                    "invalid period_id {:?} in fiscal store: {error}",
                    row.period_id
                ))
            })?;
            if period.year() == year
                && period.is_month()
                && (row.state != PeriodState::Open || row.vat_filed)
            {
                uses_months = true;
                break;
            }
        }
        if uses_months {
            (1..=12)
                .map(|month| PeriodId::new_month(year, month).map_err(LedgerError::from))
                .collect()
        } else {
            (1..=4)
                .map(|quarter| PeriodId::new(year, quarter).map_err(LedgerError::from))
                .collect()
        }
    }

    /// `Effect::SealYear` — lock the fiscal `year` (S6). Verifies (inside the writer
    /// lock) that every selected subperiod is `Sealed`, CAS the year `Open → Sealed`,
    /// binds the approved `statements_hash` into the hash-chained year-lock event, and
    /// advances the authoritative row. Irreversible: a second seal fails the CAS
    /// (`PERIOD_TRANSITION_CONFLICT`); an unsealed subperiod fails with
    /// `PERIOD_NOT_SEALED`. Atomic against a concurrent voucher/period write.
    pub fn seal_year(
        &self,
        year: i32,
        statements_hash: &str,
        run_id: &str,
        actor: &str,
    ) -> Result<FiscalYearView, LedgerError> {
        // Derive the exact first-class inputs before taking the irreversible year lock.
        // If prior authoritative activity exists, this enforces that Y-1 is already
        // Sealed; a later close can never bind an opening position that a prior reopen
        // or late attribution could still change.
        let _ = self.fiscal_year_data(year)?;

        // The bound hash must be a well-formed sha256 hex digest — a sealed year's
        // `statements_hash` is the "what was approved == what was sealed" anchor, so
        // an empty/malformed value would make the binding meaningless.
        if !is_sha256_hex(statements_hash) {
            return Err(LedgerError::InvalidHash(format!(
                "seal_year: statements_hash must be 64 lowercase hex chars, got {statements_hash:?}"
            )));
        }
        let subperiods: Vec<String> = self
            .year_close_periods(year)?
            .into_iter()
            .map(|period| period.to_string())
            .collect();
        let row = self
            .fiscal
            .transition_fiscal_year(&FiscalYearTransitionRequest {
                year,
                from: FiscalYearState::Open,
                to: FiscalYearState::Sealed,
                required_sealed_periods: subperiods,
                statements_hash: Some(statements_hash.to_string()),
                run_id: run_id.to_string(),
                actor: actor.to_string(),
            })?;
        Ok(row_to_year_view(row))
    }

    /// Attribute a sealed voucher to the **statement year** its postings belong to
    /// (issue `closeyear-adjustment-attribution`). A year-end adjustment
    /// (poistot/jaksotukset/D7) posts to the *next open* period under A10 — the
    /// closing year's quarters are already Sealed — yet must affect the *closing*
    /// year's tuloslaskelma/tase. This binding is what makes `fiscal_year_data` (and
    /// therefore `compute_statements` / `close_package_hash_for_year`) fold the
    /// adjustment into the closing year rather than the calendar year it posts in.
    ///
    /// It is meant to be called **during the close-year Run**, after the adjustment
    /// voucher is sealed and *before* `seal_year` binds the statements — the target
    /// year must still be `Open`. Guards, each refused with an audited rejection:
    ///
    /// - the voucher must be sealed (`VOUCHER_NUMBER_TAKEN` if unknown — reusing the
    ///   "this number is not a live sealed voucher" code);
    /// - `effective_year` must not be *after* the voucher's posting year
    ///   (`PERIOD_TRANSITION_CONFLICT`) — an adjustment attributes an earlier or the
    ///   same statement year, never pulls a voucher into a future year's result;
    /// - the target year must be `Open`, not `Sealed` (`PERIOD_TRANSITION_CONFLICT`) —
    ///   a sealed year's `statements_hash` is bound and must not gain a late voucher;
    /// - attribution is immutable: re-attributing a voucher to a *different* year is
    ///   `IDEMPOTENCY_KEY_REUSE`, while a repeat to the *same* year replays.
    ///
    /// Under the single-user trusted model this takes a bare `run_id`/`actor` like
    /// [`Ledger::seal_year`] (no capability token); the reviewer-identity / signing
    /// question is a multi-user concern recorded in `multi-user-considerations`.
    pub fn attribute_adjustment(
        &self,
        voucher_number: u64,
        effective_year: i32,
        run_id: &str,
        actor: &str,
    ) -> Result<AdjustmentAttributionView, LedgerError> {
        // Deterministic audit key so a repeated refusal is distinguishable in the
        // append-only rejection log (bound into every rejection this call produces).
        let request_hash = sha256_hex(
            format!("attribution\0{voucher_number}\0{effective_year}\0{run_id}").as_bytes(),
        );
        // The voucher must be a live sealed record; read it to bound the effective
        // year by its posting year (a sealed voucher is immutable, so reading it
        // outside the attribute writer lock cannot race). A missing voucher is a
        // refused, audited write — not a bare error.
        let stored = match self.fiscal.read_voucher(voucher_number)? {
            Some(v) => v,
            None => {
                let err = LedgerError::VoucherNotFound(format!(
                    "cannot attribute voucher {voucher_number}: no such sealed voucher"
                ));
                return Err(self.refuse_attribution(err, &request_hash, voucher_number, actor));
            }
        };
        let body = aggountant_wire::decode_voucher_body(&stored.canonical)?;
        let posting_year = PeriodId::containing(body.posting_date()).year();
        if effective_year > posting_year {
            let err = LedgerError::PeriodTransitionConflict(format!(
                "cannot attribute voucher {voucher_number} (posted in {posting_year}) to a later \
                 statement year {effective_year}: an adjustment belongs to an earlier or the same \
                 year, never a future one"
            ));
            return Err(self.refuse_attribution(err, &request_hash, voucher_number, actor));
        }
        let result = self.fiscal.attribute_adjustment(&AttributionRequest {
            voucher_number,
            effective_year,
            posting_year,
            run_id: run_id.to_string(),
            actor: actor.to_string(),
            request_hash,
        })?;
        let (row, replayed) = match result {
            AttributionResult::Recorded(a) => (a, false),
            AttributionResult::Replayed(a) => (a, true),
        };
        Ok(attribution_to_view(row, replayed))
    }

    /// The statement year a voucher is attributed to, or `None` if it carries no
    /// attribution (its statement year is then its `posting_date` year). Read side of
    /// [`Ledger::attribute_adjustment`].
    pub fn adjustment_attribution(
        &self,
        voucher_number: u64,
    ) -> Result<Option<AdjustmentAttributionView>, LedgerError> {
        Ok(self
            .fiscal
            .verification_snapshot()?
            .attributions
            .into_iter()
            .find(|a| a.voucher_number == voucher_number)
            .map(|a| attribution_to_view(a, false)))
    }

    /// Every voucher attributed to statement `year`, in attribution order — the
    /// adjustments `fiscal_year_data(year)` folds in over and above the year's own
    /// posting-dated vouchers.
    pub fn adjustment_attributions_for_year(
        &self,
        year: i32,
    ) -> Result<Vec<AdjustmentAttributionView>, LedgerError> {
        Ok(self
            .fiscal
            .verification_snapshot()?
            .attributions
            .into_iter()
            .filter(|a| a.effective_year == year)
            .map(|a| attribution_to_view(a, false))
            .collect())
    }

    /// `Effect::TransitionPeriod(Open → Closing)` — the first effect of a
    /// `close-period` Run (ADR-6 §2). Atomic CAS into the authoritative fiscal
    /// row in the seal's writer lock; binds the period to `run_id` and mints the
    /// non-serialisable [`ClosingAuthority`]. From this instant the Ledger refuses
    /// new vouchers into the period except against this token.
    pub fn begin_close(
        &self,
        period: &PeriodId,
        run_id: &str,
        actor: &str,
    ) -> Result<ClosingAuthority, LedgerError> {
        self.fiscal.transition_period(&TransitionRequest {
            period_id: period.to_string(),
            from: PeriodState::Open,
            to: PeriodState::Closing,
            run_id: run_id.to_string(),
            match_owner: false,
            require_unfiled: false,
            set_filed: false,
            sent_hash: None,
            receipt_unique_identifier: None,
            receipt_accepted_timestamp: None,
            manual_channel: None,
            manual_evidence_ref: None,
            remote_status_evidence_ref: None,
            actor: actor.to_string(),
        })?;
        Ok(ClosingAuthority::mint(run_id.to_string(), period.clone()))
    }

    /// `Effect::SealPeriod(Closing → Sealed)` — locks the chain (ADR-6 §2). The
    /// presented `authority` must own the period; fiscal re-checks `owning_run_id`
    /// in the CAS, so a stale/foreign token is fenced (`VOUCHER_NOT_IN_RUN`).
    pub fn seal_period(
        &self,
        period: &PeriodId,
        authority: &ClosingAuthority,
        actor: &str,
    ) -> Result<PeriodView, LedgerError> {
        self.check_authority_period(period, authority)?;
        let row = self.fiscal.transition_period(&TransitionRequest {
            period_id: period.to_string(),
            from: PeriodState::Closing,
            to: PeriodState::Sealed,
            run_id: authority.run_id().to_string(),
            match_owner: true,
            require_unfiled: false,
            set_filed: false,
            sent_hash: None,
            receipt_unique_identifier: None,
            receipt_accepted_timestamp: None,
            manual_channel: None,
            manual_evidence_ref: None,
            remote_status_evidence_ref: None,
            actor: actor.to_string(),
        })?;
        Ok(row_to_view(row))
    }

    /// The ADR-6 §5 reopen exception: `Sealed → Open|Closing`, gated by
    /// `ClosingAuthority` and permitted **only before the VAT return is filed**.
    /// Serializes on the same writer lock; a stale token, a foreign run, or a
    /// filed period all fail the CAS and are refused with a rejection event.
    pub fn reopen_period(
        &self,
        period: &PeriodId,
        authority: &ClosingAuthority,
        target: PeriodState,
        actor: &str,
    ) -> Result<PeriodView, LedgerError> {
        if !matches!(target, PeriodState::Open | PeriodState::Closing) {
            return Err(LedgerError::PeriodTransitionConflict(format!(
                "reopen target must be Open or Closing, got {}",
                target.as_str()
            )));
        }
        self.check_authority_period(period, authority)?;
        let row = self.fiscal.transition_period(&TransitionRequest {
            period_id: period.to_string(),
            from: PeriodState::Sealed,
            to: target,
            run_id: authority.run_id().to_string(),
            match_owner: true,
            require_unfiled: true,
            set_filed: false,
            sent_hash: None,
            receipt_unique_identifier: None,
            receipt_accepted_timestamp: None,
            manual_channel: None,
            manual_evidence_ref: None,
            remote_status_evidence_ref: None,
            actor: actor.to_string(),
        })?;
        Ok(row_to_view(row))
    }

    /// Re-acquire a period's `ClosingAuthority` from the authoritative fiscal
    /// state — the resume/gate-answer path (ADR-6 §3). Succeeds **only** for the
    /// run that fiscally owns the period while it is `Closing` or `Sealed`; the
    /// token is never reloaded from process memory (it is non-serialisable), it is
    /// re-derived from fiscal truth. A foreign run is fenced.
    ///
    /// **M2 caveat (surfaced, not yet decided):** the fence is string equality on
    /// `owning_run_id`, which `PeriodView` exposes and the process journal stores
    /// on disk — so at the *library* level a caller who reads the owning `run_id`
    /// can re-mint a token. At the *CLI* boundary the capability still holds (there
    /// is no `--run-id`→seal path). Whether the Ledger needs a fencing generation
    /// (`close_epoch`) or a persisted unguessable credential before the other five
    /// Runs are built is an explicit M2 question for the product owner.
    pub fn acquire_authority(
        &self,
        period: &PeriodId,
        run_id: &str,
    ) -> Result<ClosingAuthority, LedgerError> {
        let row = self.fiscal.read_period(&period.to_string())?;
        let owns = row.owning_run_id.as_deref() == Some(run_id);
        match row.state {
            PeriodState::Closing | PeriodState::Sealed if owns => {
                Ok(ClosingAuthority::mint(run_id.to_string(), period.clone()))
            }
            PeriodState::Closing | PeriodState::Sealed => {
                Err(LedgerError::VoucherNotInRun(format!(
                    "period {period} is owned by {:?}, not run {run_id}",
                    row.owning_run_id
                )))
            }
            PeriodState::Open => Err(LedgerError::PeriodTransitionConflict(format!(
                "period {period} is Open — no ClosingAuthority to acquire"
            ))),
        }
    }

    /// Mint a [`FilingAuthority`] for a `Sealed`, not-yet-filed period (S3/RF3).
    /// The `file-vat` Run acquires this before it records a filing — the capability
    /// half of "a bare `--run-id` is not an authority to file". Under the accepted
    /// trusted-local threat model (`closing-authority-fencing` (a)), any run may
    /// file a Sealed unfiled period (the filing run is legitimately *not* the close
    /// run that sealed it), so this does not pin `owning_run_id`; the guards that
    /// matter — `Sealed` and `!vat_filed` — are enforced here and re-checked in the
    /// filing CAS. A period that is Open/Closing, or already filed, is refused.
    pub fn acquire_filing_authority(
        &self,
        period: &PeriodId,
        run_id: &str,
    ) -> Result<FilingAuthority, LedgerError> {
        let snapshot = self.fiscal.verification_snapshot()?;
        for other in &snapshot.periods {
            if other.period_id == period.to_string() {
                continue;
            }
            let other_id = PeriodId::parse(&other.period_id).map_err(|error| {
                LedgerError::Store(format!(
                    "invalid period_id {:?} in fiscal store: {error}",
                    other.period_id
                ))
            })?;
            if period.overlaps(&other_id) && (other.state != PeriodState::Open || other.vat_filed) {
                return Err(LedgerError::PeriodTransitionConflict(format!(
                    "period {period} cannot be filed while overlapping period {} is {} (filed={})",
                    other.period_id,
                    other.state.as_str(),
                    other.vat_filed
                )));
            }
        }
        let row = self.fiscal.read_period(&period.to_string())?;
        match row.state {
            PeriodState::Sealed if !row.vat_filed => {
                Ok(FilingAuthority::mint(run_id.to_string(), period.clone()))
            }
            PeriodState::Sealed => Err(LedgerError::PeriodTransitionConflict(format!(
                "period {period} is already filed — filing is irreversible"
            ))),
            other => Err(LedgerError::PeriodNotSealed(format!(
                "period {period} is {} — a VAT return can only be filed for a Sealed period",
                other.as_str()
            ))),
        }
    }

    /// Record a period's VAT filing as an **audited, hash-chained** event bound to
    /// the `sent_hash` of the submitted envelope (S3/RF3 — replaces S2's untracked
    /// `mark_vat_filed` stub). The filing self-edge (`Sealed→Sealed`, `vat_filed
    /// 0→1`) is appended to the same `period_transition` chain the seal uses and
    /// flips the flag in one writer lock, so the filing becomes untamperable history
    /// (`voucher verify` walks it; `fold(period_transition).vat_filed == row`). A
    /// second filing fails the `!vat_filed` CAS — **filing is irreversible**.
    pub fn file_vat(
        &self,
        period: &PeriodId,
        authority: &FilingAuthority,
        sent_hash: &str,
        actor: &str,
    ) -> Result<PeriodView, LedgerError> {
        if authority.period() != period {
            return Err(LedgerError::VoucherNotInRun(format!(
                "FilingAuthority is for period {}, not {period}",
                authority.period()
            )));
        }
        self.file_vat_inner(
            period,
            authority.run_id(),
            Some(sent_hash),
            None,
            None,
            actor,
        )
    }

    /// Record a complete HTTP 200 FileVATReturn receipt in the same immutable
    /// filing event as the approved payload hash.
    pub fn file_vat_with_receipt(
        &self,
        period: &PeriodId,
        authority: &FilingAuthority,
        sent_hash: &str,
        unique_identifier: &str,
        accepted_timestamp: &str,
        actor: &str,
    ) -> Result<PeriodView, LedgerError> {
        if authority.period() != period {
            return Err(LedgerError::VoucherNotInRun(format!(
                "FilingAuthority is for period {}, not {period}",
                authority.period()
            )));
        }
        if unique_identifier.trim().is_empty() || accepted_timestamp.trim().is_empty() {
            return Err(LedgerError::Store(
                "FileVATReturn receipt requires UniqueIdentifier and AcceptedTimestamp".into(),
            ));
        }
        self.file_vat_inner(
            period,
            authority.run_id(),
            Some(sent_hash),
            Some((unique_identifier, accepted_timestamp)),
            None,
            actor,
        )
    }

    /// Record an operator-completed electronic filing with explicit channel and
    /// non-secret evidence in the same irreversible filing event.
    pub fn file_vat_manually(
        &self,
        period: &PeriodId,
        authority: &FilingAuthority,
        channel: &str,
        evidence_ref: &str,
        remote_status_evidence_ref: Option<&str>,
        actor: &str,
    ) -> Result<PeriodView, LedgerError> {
        if authority.period() != period {
            return Err(LedgerError::VoucherNotInRun(format!(
                "FilingAuthority is for period {}, not {period}",
                authority.period()
            )));
        }
        if !matches!(channel, "omavero" | "ilmoitin") || evidence_ref.is_empty() {
            return Err(LedgerError::Store(
                "manual VAT filing requires channel omavero|ilmoitin and evidence_ref".into(),
            ));
        }
        self.file_vat_inner(
            period,
            authority.run_id(),
            None,
            None,
            Some((channel, evidence_ref, remote_status_evidence_ref)),
            actor,
        )
    }

    /// Shared filing CAS: `Sealed→Sealed`, `vat_filed 0→1`, one audited event. Maps
    /// a non-Sealed period to `PERIOD_NOT_SEALED` and an already-filed period to
    /// `PERIOD_TRANSITION_CONFLICT` (the irreversibility guard).
    fn file_vat_inner(
        &self,
        period: &PeriodId,
        run_id: &str,
        sent_hash: Option<&str>,
        receipt: Option<(&str, &str)>,
        manual: Option<(&str, &str, Option<&str>)>,
        actor: &str,
    ) -> Result<PeriodView, LedgerError> {
        let cur = self.fiscal.read_period(&period.to_string())?;
        if cur.state != PeriodState::Sealed {
            return Err(LedgerError::PeriodNotSealed(format!(
                "period {period} is {} — a VAT return can only be filed for a Sealed period",
                cur.state.as_str()
            )));
        }
        // Idempotent re-file (crash recovery): if the period is already filed with
        // the *same* `sent_hash`, replay the existing filing instead of a conflict —
        // so a crash between the broker success and this COMMIT can be retried
        // cleanly. A *different* sent_hash is still refused (filing is irreversible).
        if cur.vat_filed {
            let existing = self.fiscal.last_filing_sent_hash(&period.to_string())?;
            return match existing {
                Some(stored) if stored.as_deref() == sent_hash => Ok(row_to_view(cur)),
                _ => Err(LedgerError::PeriodTransitionConflict(format!(
                    "period {period} is already filed with different bytes — filing is irreversible"
                ))),
            };
        }
        let row = self.fiscal.transition_period(&TransitionRequest {
            period_id: period.to_string(),
            from: PeriodState::Sealed,
            to: PeriodState::Sealed,
            run_id: run_id.to_string(),
            match_owner: false,
            require_unfiled: true,
            set_filed: true,
            sent_hash: sent_hash.map(|s| s.to_string()),
            receipt_unique_identifier: receipt.map(|value| value.0.to_string()),
            receipt_accepted_timestamp: receipt.map(|value| value.1.to_string()),
            manual_channel: manual.map(|value| value.0.to_string()),
            manual_evidence_ref: manual.map(|value| value.1.to_string()),
            remote_status_evidence_ref: manual.and_then(|value| value.2.map(str::to_string)),
            actor: actor.to_string(),
        })?;
        Ok(row_to_view(row))
    }

    pub fn manual_vat_filing_evidence(
        &self,
        period: &PeriodId,
    ) -> Result<Option<ManualVatFilingEvidence>, LedgerError> {
        Ok(self
            .fiscal
            .last_manual_filing_evidence(&period.to_string())?
            .map(
                |(channel, evidence_ref, remote_status_evidence_ref)| ManualVatFilingEvidence {
                    channel,
                    evidence_ref,
                    remote_status_evidence_ref,
                },
            ))
    }

    pub fn vat_filing_receipt(
        &self,
        period: &PeriodId,
    ) -> Result<Option<VatFilingReceipt>, LedgerError> {
        Ok(self.fiscal.last_filing_receipt(&period.to_string())?.map(
            |(unique_identifier, accepted_timestamp)| VatFilingReceipt {
                unique_identifier,
                accepted_timestamp,
            },
        ))
    }

    /// A `ClosingAuthority` is only valid for the period it was minted against.
    fn check_authority_period(
        &self,
        period: &PeriodId,
        authority: &ClosingAuthority,
    ) -> Result<(), LedgerError> {
        if authority.period() != period {
            return Err(LedgerError::VoucherNotInRun(format!(
                "ClosingAuthority is for period {}, not {period}",
                authority.period()
            )));
        }
        Ok(())
    }

    /// Full integrity walk (`voucher verify`). Reads a single consistent
    /// snapshot (so a concurrent seal cannot cause a spurious break), then
    /// checks: numbers gapless from 1; each voucher chains onto the previous
    /// hash; every record passes the strict byte-equality read path + chain-hash
    /// recompute (`ADR-2`); the decoded body's `number`/`chart_version` agree
    /// with the relational row; the stored `chain_head` matches the final
    /// voucher; and the append-only rejection log's own hash chain is intact.
    pub fn verify(&self) -> Result<VerifyReport, LedgerError> {
        let snapshot = self.fiscal.verification_snapshot()?;

        let mut prev = Hash::zero();
        for (i, v) in snapshot.vouchers.iter().enumerate() {
            let expected_number = (i as u64) + 1;
            if v.number != expected_number {
                return Err(LedgerError::HashChainBroken(format!(
                    "gap or out-of-order: expected number {expected_number}, got {}",
                    v.number
                )));
            }
            if v.prev_hash != prev {
                return Err(LedgerError::HashChainBroken(format!(
                    "voucher {} prev_hash does not match voucher {}'s hash",
                    v.number,
                    v.number.saturating_sub(1)
                )));
            }
            // Strict read path: re-encode byte-equal + recompute chain hash. The
            // returned body is authoritative — cross-check the mutable SQL
            // columns that duplicate hashed fields against it.
            let body = aggountant_wire::verify_voucher(&v.canonical, &prev, &v.hash)?;
            if body.number() != v.number {
                return Err(LedgerError::HashChainBroken(format!(
                    "row number {} holds a canonical body numbered {}",
                    v.number,
                    body.number()
                )));
            }
            if body.chart_version() != v.chart_version {
                return Err(LedgerError::HashChainBroken(format!(
                    "voucher {} chart_version column disagrees with the hashed body",
                    v.number
                )));
            }
            prev = v.hash;
        }

        // The materialized chain head must match the walked chain exactly.
        let count = snapshot.vouchers.len() as u64;
        if snapshot.head_number != count {
            return Err(LedgerError::HashChainBroken(format!(
                "chain head number {} != voucher count {count}",
                snapshot.head_number
            )));
        }
        if snapshot.head_hash != prev {
            return Err(LedgerError::HashChainBroken(format!(
                "chain head hash {} != final voucher hash {}",
                snapshot.head_hash, prev
            )));
        }

        // Walk the append-only rejection log's own hash chain.
        let mut rej_prev = Hash::zero();
        for r in &snapshot.rejections {
            if r.prev_reject_hash != rej_prev {
                return Err(LedgerError::HashChainBroken(format!(
                    "rejection seq {} prev_reject_hash does not chain",
                    r.seq
                )));
            }
            let rec = RejectionRecord {
                reason_code: r.reason_code.clone(),
                request_hash: r.request_hash.clone(),
                actor: r.actor.clone(),
                target: r.target.clone(),
                note: r.note.clone(),
            };
            let recomputed = rejection_hash(&rej_prev, &rec, &r.ts);
            if recomputed != r.hash {
                return Err(LedgerError::HashChainBroken(format!(
                    "rejection seq {} hash mismatch (log tampered)",
                    r.seq
                )));
            }
            rej_prev = r.hash;
        }

        // ADR-6 §1 defense-in-depth: fold(period_transition) == period, per
        // period. Walk each period's own hash chain (genesis zero, prev ← last),
        // recompute every event hash, check from/to continuity, and compare the
        // folded terminal state to the materialized current-state row.
        let period_count = verify_periods(&snapshot)?;

        // S6 defense-in-depth: fold(fiscal_year_transition) == fiscal_year, per year.
        let fiscal_year_count = verify_fiscal_years(&snapshot)?;

        // Issue `closeyear-adjustment-attribution`: walk the attribution chain so a
        // tampered/reordered attribution (which would silently move a voucher between
        // statement years, changing a projected — and possibly sealed — result) fails
        // verification exactly as a tampered voucher or period transition does.
        let attribution_count = verify_attributions(&snapshot)?;

        let mut software_version_spans: Vec<SoftwareVersionSpan> = Vec::new();
        for voucher in &snapshot.vouchers {
            if let Some(last) = software_version_spans
                .last_mut()
                .filter(|span| span.software_version == voucher.provenance.software_version)
            {
                last.last_number = voucher.number;
            } else {
                software_version_spans.push(SoftwareVersionSpan {
                    first_number: voucher.number,
                    last_number: voucher.number,
                    software_version: voucher.provenance.software_version.clone(),
                });
            }
        }

        Ok(VerifyReport {
            ok: true,
            voucher_count: count,
            head_number: snapshot.head_number,
            head_hash: snapshot.head_hash.to_hex(),
            software_version_spans,
            period_count,
            fiscal_year_count,
            attribution_count,
        })
    }
}

/// Produce a projection-only copy of `body` whose **posting** date is moved to
/// `year-12-31` (issue `closeyear-adjustment-attribution`). Used by `fiscal_year_data` to
/// present an attributed adjustment — sealed into a later period — so the Domain's
/// posting-year filter (`compute_statements` / `tase_erittelyt` key off `posting_date`
/// only) folds it into its statement year.
///
/// Only `posting_date` is rewritten; the real `document_date` is preserved (the domain
/// filter never reads it, and it is a legally meaningful field — the earlier version
/// rewrote both, needlessly falsifying it). The rest of the fold input (number,
/// postings, description, attachment, buyer VAT ID, chart_version) is carried through unchanged.
///
/// **Projection-only, load-bearing assumption:** the returned body is never stored or
/// re-sealed, and its recomputed canonical bytes / hash do **not** match the sealed
/// original — callers must treat `FiscalYearData::vouchers` as statement-projection input,
/// not a chain-authoritative record. The narrower rewrite depends on the Domain
/// projection filtering solely by `posting_date`; a future Domain change that inspected
/// dates otherwise would need the proper projection-wrapper design recorded on the issue.
fn redate_into_year(body: &VoucherBody, year: i32) -> Result<VoucherBody, LedgerError> {
    let posting = Date::from_ymd(year, 12, 31)?;
    let draft = VoucherDraft::new(
        body.document_date(),
        posting,
        body.description().to_string(),
        body.postings().to_vec(),
        body.attachment_ref(),
    )?;
    let draft = match body.buyer_vat_id() {
        Some(buyer_vat_id) => draft.with_buyer_vat_id(buyer_vat_id.to_string())?,
        None => draft,
    };
    Ok(VoucherBody::from_draft(
        &draft,
        body.number(),
        body.chart_version().to_string(),
    )?)
}

/// The issue `closeyear-adjustment-attribution` chain walk: the append-only
/// `adjustment_attribution` log is a single hash chain (genesis zero, `prev ← last`).
/// Recompute every event hash and check `prev_hash` continuity + strictly increasing
/// `seq`, and that each event references a sealed voucher exactly once. A tamper to
/// `effective_year` (silently reattributing a voucher, changing a projected result) or
/// any reordering breaks the chain. Returns the number of verified events.
fn verify_attributions(snapshot: &store::FiscalSnapshot) -> Result<u64, LedgerError> {
    use std::collections::BTreeSet;

    let mut prev = Hash::zero();
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    // Vouchers were already proven gapless `1..=head_number` earlier in `verify`, so a
    // valid `voucher_number` is exactly one in that range — no per-number set needed.
    let head_number = snapshot.head_number;
    for (i, a) in snapshot.attributions.iter().enumerate() {
        // Contiguous logical height 1,2,3,… (a gap or reorder — including a deleted
        // interior/leading row — breaks this; matches the period/year transition chains).
        let expected_seq = (i as i64) + 1;
        if a.seq != expected_seq {
            return Err(LedgerError::HashChainBroken(format!(
                "attribution seq gap: expected {expected_seq}, got {}",
                a.seq
            )));
        }
        if a.prev_hash != prev {
            return Err(LedgerError::HashChainBroken(format!(
                "attribution seq {} prev_hash does not chain",
                a.seq
            )));
        }
        if !seen.insert(a.voucher_number) {
            return Err(LedgerError::HashChainBroken(format!(
                "attribution seq {} re-attributes voucher {} (attribution is immutable)",
                a.seq, a.voucher_number
            )));
        }
        if a.voucher_number < 1 || a.voucher_number > head_number {
            return Err(LedgerError::HashChainBroken(format!(
                "attribution seq {} references voucher {} which is not a sealed voucher",
                a.seq, a.voucher_number
            )));
        }
        let recomputed = attribution_hash(
            &prev,
            a.seq as u64,
            a.voucher_number,
            a.effective_year,
            &a.run_id,
            &a.actor,
            &a.ts,
        );
        if recomputed != a.hash {
            return Err(LedgerError::HashChainBroken(format!(
                "attribution seq {} hash mismatch (log tampered)",
                a.seq
            )));
        }
        prev = a.hash;
    }
    Ok(snapshot.attributions.len() as u64)
}

/// The legal edges of the fiscal-year state machine: `Open→Sealed` only (a year
/// seal is irreversible — nothing follows `Sealed`).
fn is_legal_year_edge(from: &str, to: &str) -> bool {
    matches!((from, to), ("Open", "Sealed"))
}

/// The S6 `fold(fiscal_year_transition) == fiscal_year` check. Returns the number of
/// verified years, or `HASH_CHAIN_BROKEN` on any divergence.
fn verify_fiscal_years(snapshot: &store::FiscalSnapshot) -> Result<u64, LedgerError> {
    use std::collections::BTreeMap;

    let mut by_year: BTreeMap<i32, Vec<&StoredFiscalYearTransition>> = BTreeMap::new();
    for t in &snapshot.fiscal_year_transitions {
        by_year.entry(t.year).or_default().push(t);
    }

    for (year, transitions) in &by_year {
        let mut prev = Hash::zero();
        let mut last_to = "Open";
        let mut folded_statements_hash: Option<String> = None;
        let mut folded_owner: Option<String> = None;
        for (i, t) in transitions.iter().enumerate() {
            let expected_seq = (i as u64) + 1;
            if t.year_seq != expected_seq {
                return Err(LedgerError::HashChainBroken(format!(
                    "fiscal year {year} transition seq gap: expected {expected_seq}, got {}",
                    t.year_seq
                )));
            }
            if t.from_state != last_to {
                return Err(LedgerError::HashChainBroken(format!(
                    "fiscal year {year} transition {} from_state {:?} does not follow {:?}",
                    t.year_seq, t.from_state, last_to
                )));
            }
            if !is_legal_year_edge(&t.from_state, &t.to_state) {
                return Err(LedgerError::HashChainBroken(format!(
                    "fiscal year {year} transition {} is an illegal edge {}→{}",
                    t.year_seq, t.from_state, t.to_state
                )));
            }
            if t.prev_hash != prev {
                return Err(LedgerError::HashChainBroken(format!(
                    "fiscal year {year} transition {} prev_hash does not chain",
                    t.year_seq
                )));
            }
            let recomputed = fiscal_year_transition_hash(
                &prev,
                t.year,
                t.year_seq,
                &t.from_state,
                &t.to_state,
                t.statements_hash.as_deref(),
                t.owning_run_id.as_deref(),
                &t.actor,
                &t.ts,
            );
            if recomputed != t.hash {
                return Err(LedgerError::HashChainBroken(format!(
                    "fiscal year {year} transition {} hash mismatch (log tampered)",
                    t.year_seq
                )));
            }
            folded_statements_hash = t.statements_hash.clone();
            folded_owner = t.owning_run_id.clone();
            prev = t.hash;
            last_to = &t.to_state;
        }

        let row = snapshot
            .fiscal_years
            .iter()
            .find(|y| y.year == *year)
            .ok_or_else(|| {
                LedgerError::HashChainBroken(format!(
                    "fiscal year {year} has a transition log but no current-state row"
                ))
            })?;
        if row.state.as_str() != last_to
            || row.head_seq != transitions.len() as u64
            || row.head_hash != prev
            || row.owning_run_id != folded_owner
            || row.statements_hash != folded_statements_hash
        {
            return Err(LedgerError::HashChainBroken(format!(
                "fiscal year {year} current-state row disagrees with fold(fiscal_year_transition)"
            )));
        }
    }

    for y in &snapshot.fiscal_years {
        if !by_year.contains_key(&y.year) {
            return Err(LedgerError::HashChainBroken(format!(
                "fiscal year {} has a current-state row but no transition log",
                y.year
            )));
        }
    }

    Ok(snapshot.fiscal_years.len() as u64)
}

/// The legal edges of the ADR-6 period state machine: the forward path
/// `Open→Closing→Sealed`, the reopen return edge `Sealed→Open|Closing`, and the S3
/// VAT-filing self-edge `Sealed→Sealed` (which flips `vat_filed`, not `state`).
fn is_legal_period_edge(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("Open", "Closing")
            | ("Closing", "Sealed")
            | ("Sealed", "Open")
            | ("Sealed", "Closing")
            | ("Sealed", "Sealed")
    )
}

/// The ADR-6 `fold(period_transition) == period` check (F18). Returns the number
/// of verified periods, or `HASH_CHAIN_BROKEN` on any divergence.
fn verify_periods(snapshot: &store::FiscalSnapshot) -> Result<u64, LedgerError> {
    use std::collections::BTreeMap;

    let mut by_period: BTreeMap<&str, Vec<&StoredPeriodTransition>> = BTreeMap::new();
    for t in &snapshot.period_transitions {
        by_period.entry(t.period_id.as_str()).or_default().push(t);
    }

    for (pid, transitions) in &by_period {
        let mut prev = Hash::zero();
        let mut last_to = "Open"; // the implicit pre-history state
        let mut folded_filed = false;
        for (i, t) in transitions.iter().enumerate() {
            let expected_seq = (i as u64) + 1;
            if t.period_seq != expected_seq {
                return Err(LedgerError::HashChainBroken(format!(
                    "period {pid} transition seq gap: expected {expected_seq}, got {}",
                    t.period_seq
                )));
            }
            if t.from_state != last_to {
                return Err(LedgerError::HashChainBroken(format!(
                    "period {pid} transition {} from_state {:?} does not follow {:?}",
                    t.period_seq, t.from_state, last_to
                )));
            }
            // The chain must walk only legal edges of the ADR-6 state machine — a
            // well-chained but illegal edge (e.g. a forged Open→Sealed) must fail.
            if !is_legal_period_edge(&t.from_state, &t.to_state) {
                return Err(LedgerError::HashChainBroken(format!(
                    "period {pid} transition {} is an illegal edge {}→{}",
                    t.period_seq, t.from_state, t.to_state
                )));
            }
            // Filing is irreversible: nothing may follow the filing self-edge, and
            // there is at most one filing per period. A forged second filing edge or
            // any post-filing transition (e.g. a reopen) is a broken chain.
            if folded_filed {
                return Err(LedgerError::HashChainBroken(format!(
                    "period {pid} transition {} follows an irreversible VAT filing",
                    t.period_seq
                )));
            }
            if t.prev_hash != prev {
                return Err(LedgerError::HashChainBroken(format!(
                    "period {pid} transition {} prev_hash does not chain",
                    t.period_seq
                )));
            }
            let recomputed = period_transition_hash(
                &prev,
                &t.period_id,
                t.period_seq,
                &t.from_state,
                &t.to_state,
                t.owning_run_id.as_deref(),
                t.sent_hash.as_deref(),
                t.receipt_unique_identifier.as_deref(),
                t.receipt_accepted_timestamp.as_deref(),
                t.manual_channel.as_deref(),
                t.manual_evidence_ref.as_deref(),
                t.remote_status_evidence_ref.as_deref(),
                &t.actor,
                &t.ts,
            );
            if recomputed != t.hash {
                return Err(LedgerError::HashChainBroken(format!(
                    "period {pid} transition {} hash mismatch (log tampered)",
                    t.period_seq
                )));
            }
            // A `Sealed→Sealed` self-edge is the VAT-filing event: it flips
            // `vat_filed`, not `state`. Folding it lets the row's `vat_filed` be
            // cross-checked, so a tamper that clears the flag to reopen a filed
            // period (the S2 gap) is now caught.
            if t.from_state == "Sealed" && t.to_state == "Sealed" {
                folded_filed = true;
            }
            prev = t.hash;
            last_to = &t.to_state;
        }

        // The folded terminal must equal the materialized row exactly.
        let row = snapshot
            .periods
            .iter()
            .find(|p| p.period_id == **pid)
            .ok_or_else(|| {
                LedgerError::HashChainBroken(format!(
                    "period {pid} has a transition log but no current-state row"
                ))
            })?;
        let folded_owner = transitions.last().and_then(|t| t.owning_run_id.clone());
        if row.state.as_str() != last_to
            || row.head_seq != transitions.len() as u64
            || row.head_hash != prev
            || row.owning_run_id != folded_owner
            || row.vat_filed != folded_filed
        {
            return Err(LedgerError::HashChainBroken(format!(
                "period {pid} current-state row disagrees with fold(period_transition)"
            )));
        }
    }

    // A materialized row must be backed by at least one transition.
    for p in &snapshot.periods {
        if !by_period.contains_key(p.period_id.as_str()) {
            return Err(LedgerError::HashChainBroken(format!(
                "period {} has a current-state row but no transition log",
                p.period_id
            )));
        }
    }

    Ok(snapshot.periods.len() as u64)
}
