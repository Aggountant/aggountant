//! Git-native reference / non-PII masterdata store (ADR-1 §Masterdata, revised
//! 2026-08-03). Reference masterdata — the chart of accounts, account kinds and
//! the content-hashed `chart_version` — is **read-mostly, reviewable,
//! non-personal** data: the sweet spot for the file-as-database ethos. It lives
//! as **plaintext files under the company home** so a human can `git diff`,
//! `git blame` and PR-review the chart, and `chart_version` is a hash over
//! inspectable bytes.
//!
//! This is the write path that replaces S1's seeded read-only SQLite masterdata
//! store: `init` writes the git-native tree once; `open` loads it for reading.
//!
//! ## On-disk layout (under `<home>/masterdata/`)
//!
//! ```text
//! masterdata/
//!   reference/                 <- git-native plaintext (this store)
//!     chart-of-accounts.json   <- one account per line, diffable
//!     chart_version            <- the active content-hashed version string
//!     vat-account-roles.json   <- exact-version VAT roles; activation gate
//!     chart-snapshots/         <- immutable documents addressed by full hash
//!       <sha256>.json
//!     README.md                <- "derived reference masterdata; PII lives elsewhere"
//! ```
//!
//! ## PII boundary (ADR-1 §Masterdata, F7 — deferred, NOT built here)
//!
//! PII masterdata (HETU, IBAN, tax cards, employee identity, `EmployeeState`)
//! must **not** land in git — git history is not erasable, so a GDPR erasure
//! cannot be satisfied at proportionate cost. It belongs in a **separate
//! encrypted SQLite database** (own file, own encryption boundary, coherent
//! `DELETE`/crypto-shred). That store is future work; the seam is kept clean by
//! confining this store to `reference/` and reserving the rest of `masterdata/`
//! (e.g. `masterdata/pii.sqlite`) for the erasable boundary.
//!
//! ## Fail-closed classification (ADR-1 binding constraint)
//!
//! What is plaintext must be a *positive* whitelist of known-safe fields, never
//! "everything except HETU/IBAN". Each account carries only `id`, `name` and
//! `kind`; the document also carries a `chart` id and a `chart_date` edition
//! marker (a validated ISO date — non-PII by construction). Names here come from
//! the bundled standard Finnish chart (no personal data). A user-added custom
//! account with a personal name would need an
//! explicit classification gate before it is safe as plaintext — that gate is
//! future work; the `reference_schema_is_a_positive_non_pii_whitelist` contract
//! test (`tests/masterdata.rs`) fails closed if the schema grows an unclassified
//! field, and a canary scan proves no HETU/IBAN reaches the git bytes.

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use aggountant_domain::{VatAccountRoleTable, VatRoleTag};
use aggountant_model::{Account, AccountId, AccountKind};

use crate::error::LedgerError;
use crate::store::MasterDataRefStore;

/// The bundled standard Finnish general chart of accounts (the single
/// maintainable source — edit the data file, not scattered code).
const DEFAULT_CHART_JSON: &str = include_str!("../../data/fi-general-2026.json");

/// The on-disk / hashed chart document. `deny_unknown_fields` so a hand-edited
/// file that adds an unclassified field fails the read path loudly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChartFile {
    chart: String,
    /// Human-readable edition marker (ISO `YYYY-MM-DD` effective date of this
    /// chart edition) so a person can tell *which* edition a home carries
    /// without hashing bytes. Non-PII reference metadata; part of the content
    /// hash (see `compute_chart_version`), so an edition bump re-versions.
    chart_date: String,
    accounts: Vec<Account>,
}

/// A borrowed view serialized (via the wire encoder) to compute the content
/// hash without inventing a second on-disk shape for the hashed document.
#[derive(Serialize)]
struct ChartHashView<'a> {
    chart: &'a str,
    chart_date: &'a str,
    accounts: &'a [Account],
}

#[derive(Debug)]
pub(crate) struct GitMasterDataRefStore {
    accounts: Vec<Account>,
    ids: HashSet<AccountId>,
    chart_version: String,
    /// Exact, immutable account semantics by content-addressed chart version.
    snapshots: BTreeMap<String, Vec<Account>>,
    /// Imported VAT roles travel with the company chart and resolve by its full version.
    vat_roles: VatAccountRoleTable,
}

impl GitMasterDataRefStore {
    /// Bootstrap the git-native reference masterdata tree under `dir`
    /// (`<home>/masterdata`). Writes the bundled standard chart + a
    /// content-hashed `chart_version` as plaintext. **Fails clearly rather than
    /// clobbering** if the tree already exists (re-init safety).
    ///
    /// The whole `reference/` tree is published as a **unit**: all files are
    /// written into a private staging directory, fsynced, then the directory is
    /// atomically renamed into place. A crash mid-write leaves only an orphan
    /// staging dir (never a half-initialized `reference/`), and the rename is the
    /// single ownership gate — two racing initializers cannot both win, and the
    /// loser cannot clobber the winner (renaming onto a non-empty directory
    /// fails). This replaces the earlier file-by-file publish, which could wedge
    /// a home if it crashed between files.
    pub(crate) fn init(dir: &Path) -> Result<Self, LedgerError> {
        let reference = dir.join("reference");
        if reference.join("chart-of-accounts.json").exists() {
            return Err(already_initialized(&reference));
        }

        let chart = parse_chart(DEFAULT_CHART_JSON)?;
        let chart_version =
            compute_chart_version(&chart.chart, &chart.chart_date, &chart.accounts)?;
        let chart_bytes = render_chart_file(&chart.chart, &chart.chart_date, &chart.accounts);

        // Fail closed: never write a suspected HETU/IBAN token to plaintext
        // reference masterdata (ADR-1 binding constraint — enforced at write
        // time, not only in CI). The bundled chart is clean; this guards edits.
        if let Some(hit) = first_pii_token(&chart_bytes) {
            return Err(LedgerError::MasterDataCorrupt(format!(
                "refusing to write a suspected PII token ({hit}) to plaintext \
                 reference masterdata — PII belongs in the erasable encrypted store"
            )));
        }

        fs::create_dir_all(dir).map_err(|e| LedgerError::Store(e.to_string()))?;
        let staging = dir.join(format!(
            ".reference.staging.{}.{}",
            std::process::id(),
            next_unique()
        ));
        // A leftover staging dir from a prior crash must not poison this run.
        let _ = fs::remove_dir_all(&staging);
        fs::create_dir_all(&staging).map_err(|e| LedgerError::Store(e.to_string()))?;

        let publish = || -> Result<(), LedgerError> {
            write_file_synced(
                &staging.join("chart-of-accounts.json"),
                chart_bytes.as_bytes(),
            )?;
            write_file_synced(
                &staging.join("chart_version"),
                format!("{chart_version}\n").as_bytes(),
            )?;
            let mut role_bytes = serde_json::to_string_pretty(VatAccountRoleTable::bundled())
                .map_err(|e| LedgerError::Store(format!("VAT role render: {e}")))?;
            role_bytes.push('\n');
            write_file_synced(
                &staging.join("vat-account-roles.json"),
                role_bytes.as_bytes(),
            )?;
            let snapshots = staging.join("chart-snapshots");
            fs::create_dir_all(&snapshots).map_err(|e| LedgerError::Store(e.to_string()))?;
            let digest = chart_version
                .rsplit_once('@')
                .map(|(_, digest)| digest)
                .ok_or_else(|| {
                    LedgerError::MasterDataCorrupt("chart_version lacks @digest".into())
                })?;
            write_file_synced(
                &snapshots.join(format!("{digest}.json")),
                chart_bytes.as_bytes(),
            )?;
            fsync_dir(&snapshots);
            write_file_synced(&staging.join("README.md"), README.as_bytes())?;
            fsync_dir(&staging);
            Ok(())
        };
        if let Err(e) = publish() {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }

        // The atomic ownership gate. Renaming onto an existing non-empty
        // `reference/` fails (first-writer-wins); map that to a clean refusal.
        if let Err(e) = fs::rename(&staging, &reference) {
            let _ = fs::remove_dir_all(&staging);
            if reference.join("chart-of-accounts.json").exists() {
                return Err(already_initialized(&reference));
            }
            return Err(LedgerError::Store(format!(
                "failed to publish reference masterdata: {e}"
            )));
        }
        fsync_dir(dir);

        let vat_roles = VatAccountRoleTable::bundled().clone();
        validate_chart_vat_roles(
            &chart_version,
            &BTreeMap::from([(chart_version.clone(), chart.accounts.clone())]),
            &vat_roles,
        )?;
        Ok(Self::from_chart(chart, chart_version, vat_roles))
    }

    /// Load the git-native reference masterdata for reading. Errors clearly if
    /// the company home has not been initialized (`aggountant init`). Re-derives
    /// `chart_version` from the chart bytes and cross-checks it against the
    /// stored `chart_version` file, so a hand-edit that changes chart content
    /// without updating the version (or vice versa) is caught, not trusted.
    pub(crate) fn open(dir: &Path) -> Result<Self, LedgerError> {
        let reference = dir.join("reference");
        let chart_path = reference.join("chart-of-accounts.json");
        if !chart_path.exists() {
            return Err(LedgerError::MasterDataNotInitialized(format!(
                "no reference masterdata at {} — run `aggountant init` first",
                chart_path.display()
            )));
        }

        let raw = fs::read_to_string(&chart_path).map_err(|e| LedgerError::Store(e.to_string()))?;
        let chart = parse_chart(&raw)?;
        let recomputed = compute_chart_version(&chart.chart, &chart.chart_date, &chart.accounts)?;

        // A present chart with a missing/unreadable version file is corrupt
        // masterdata (a user deletion or a foreign partial write), not a generic
        // backend I/O failure — surface it with the specific code.
        let version_path = reference.join("chart_version");
        let stored = fs::read_to_string(&version_path)
            .map_err(|e| {
                LedgerError::MasterDataCorrupt(format!(
                    "chart present but {} is missing or unreadable: {e}",
                    version_path.display()
                ))
            })?
            .trim()
            .to_string();
        if stored != recomputed {
            return Err(LedgerError::MasterDataCorrupt(format!(
                "chart_version file {stored:?} disagrees with the chart content \
                 (recomputed {recomputed:?}) — the chart was edited without \
                 re-deriving its content hash"
            )));
        }

        let role_path = reference.join("vat-account-roles.json");
        let role_raw = fs::read_to_string(&role_path).map_err(|e| {
            LedgerError::MasterDataCorrupt(format!(
                "active chart has no readable VAT role metadata at {}: {e}",
                role_path.display()
            ))
        })?;
        let vat_roles = VatAccountRoleTable::from_json(&role_raw).map_err(|e| {
            LedgerError::MasterDataCorrupt(format!("invalid company-chart VAT roles: {e}"))
        })?;

        let mut store = Self::from_chart(chart, recomputed, vat_roles);
        let snapshots_dir = reference.join("chart-snapshots");
        if snapshots_dir.exists() {
            for entry in
                fs::read_dir(&snapshots_dir).map_err(|e| LedgerError::Store(e.to_string()))?
            {
                let path = entry.map_err(|e| LedgerError::Store(e.to_string()))?.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let raw =
                    fs::read_to_string(&path).map_err(|e| LedgerError::Store(e.to_string()))?;
                let snapshot = parse_chart(&raw)?;
                let version = compute_chart_version(
                    &snapshot.chart,
                    &snapshot.chart_date,
                    &snapshot.accounts,
                )?;
                let expected = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if version.rsplit_once('@').map(|(_, d)| d) != Some(expected) {
                    return Err(LedgerError::MasterDataCorrupt(format!(
                        "chart snapshot {} does not match its content version {version}",
                        path.display()
                    )));
                }
                store.snapshots.insert(version, snapshot.accounts);
            }
        }
        validate_chart_vat_roles(&store.chart_version, &store.snapshots, &store.vat_roles)?;
        Ok(store)
    }

    fn from_chart(
        mut chart: ChartFile,
        chart_version: String,
        vat_roles: VatAccountRoleTable,
    ) -> Self {
        // Canonical read order: by account id.
        chart.accounts.sort_by(|a, b| a.id.cmp(&b.id));
        let ids = chart.accounts.iter().map(|a| a.id.clone()).collect();
        let accounts = chart.accounts;
        let snapshots = BTreeMap::from([(chart_version.clone(), accounts.clone())]);
        GitMasterDataRefStore {
            accounts,
            ids,
            chart_version,
            snapshots,
            vat_roles,
        }
    }

    pub(crate) fn vat_roles(&self) -> &VatAccountRoleTable {
        &self.vat_roles
    }
}

/// Activation gate joining the imported role document to the actual immutable chart
/// snapshots. Domain validation owns role-table semantics; this boundary owns the facts
/// only masterdata can prove: every tagged account exists with the right balance-sheet
/// kind, and the active chart has both input- and sales-VAT coverage.
fn validate_chart_vat_roles(
    active_chart_version: &str,
    snapshots: &BTreeMap<String, Vec<Account>>,
    roles: &VatAccountRoleTable,
) -> Result<(), LedgerError> {
    let active = roles.resolve(active_chart_version).map_err(|e| {
        LedgerError::MasterDataCorrupt(format!("active chart VAT role activation failed: {e}"))
    })?;
    let mut has_input = false;
    let mut has_sales = false;

    for role_snapshot in roles.snapshots() {
        let chart = snapshots
            .get(role_snapshot.chart_version())
            .ok_or_else(|| {
                LedgerError::MasterDataCorrupt(format!(
                    "VAT role snapshot {:?} has no matching immutable chart snapshot",
                    role_snapshot.chart_version()
                ))
            })?;
        for role in role_snapshot.accounts() {
            let account = chart
                .iter()
                .find(|account| account.id.as_str() == role.account())
                .ok_or_else(|| {
                    LedgerError::MasterDataCorrupt(format!(
                        "chart {:?} VAT role account {} is absent from the full chart snapshot",
                        role_snapshot.chart_version(),
                        role.account()
                    ))
                })?;
            let expected_kind = match role.role() {
                VatRoleTag::InputVat => AccountKind::Asset,
                VatRoleTag::ReverseChargeOutputVat | VatRoleTag::TaggedOnly => {
                    AccountKind::Liability
                }
            };
            if account.kind != expected_kind {
                return Err(LedgerError::MasterDataCorrupt(format!(
                    "chart {:?} VAT role account {} must be {:?}, got {:?}",
                    role_snapshot.chart_version(),
                    role.account(),
                    expected_kind,
                    account.kind
                )));
            }
        }
        for guard in role_snapshot.mistag_guards() {
            if !chart
                .iter()
                .any(|account| account.id.as_str() == guard.account())
            {
                return Err(LedgerError::MasterDataCorrupt(format!(
                    "chart {:?} VAT mistag account {} is absent from the full chart snapshot",
                    role_snapshot.chart_version(),
                    guard.account()
                )));
            }
        }
    }

    for role in active.accounts() {
        has_input |= role.role() == VatRoleTag::InputVat;
        has_sales |= matches!(
            role.role(),
            VatRoleTag::ReverseChargeOutputVat | VatRoleTag::TaggedOnly
        );
    }
    if !has_input || !has_sales {
        return Err(LedgerError::MasterDataCorrupt(format!(
            "active chart {active_chart_version:?} VAT roles are incomplete: input-vat and sales/output-vat coverage are both required"
        )));
    }
    Ok(())
}

impl MasterDataRefStore for GitMasterDataRefStore {
    fn chart_version(&self) -> Result<String, LedgerError> {
        Ok(self.chart_version.clone())
    }

    fn list_accounts(&self) -> Result<Vec<Account>, LedgerError> {
        Ok(self.accounts.clone())
    }

    fn list_accounts_at(&self, chart_version: &str) -> Result<Vec<Account>, LedgerError> {
        self.snapshots.get(chart_version).cloned().ok_or_else(|| {
            LedgerError::MasterDataCorrupt(format!(
                "immutable chart snapshot {chart_version:?} is unavailable"
            ))
        })
    }

    fn account_exists(&self, id: &AccountId) -> Result<bool, LedgerError> {
        Ok(self.ids.contains(id))
    }
}

/// Parse the chart document with the strict (deny-unknown-fields) shape and
/// sort its accounts by id, so both the hash and the written file are
/// deterministic regardless of source ordering.
fn parse_chart(json: &str) -> Result<ChartFile, LedgerError> {
    let mut chart: ChartFile =
        serde_json::from_str(json).map_err(|e| LedgerError::Store(format!("chart parse: {e}")))?;
    if chart.accounts.is_empty() {
        return Err(LedgerError::Store("chart has no accounts".to_string()));
    }
    // `chart_date` feeds the permanent content hash and is the field a human reads
    // to identify the edition — so a free-text value would let a typo (or a
    // personal note) become a legally-pinned edition marker, and would let two
    // spellings of the same date ("2026-01-01" vs "2026-1-1") mint different
    // `chart_version`s. Enforce the documented ISO shape at the single input funnel.
    validate_chart_date(&chart.chart_date)?;
    chart.accounts.sort_by(|a, b| a.id.cmp(&b.id));
    // Reject duplicate account ids (a hand-edit hazard the model can't catch).
    let mut seen = HashSet::new();
    for a in &chart.accounts {
        if !seen.insert(a.id.clone()) {
            return Err(LedgerError::Store(format!(
                "duplicate account id {} in chart",
                a.id
            )));
        }
    }
    Ok(chart)
}

/// Enforce that `chart_date` is a real ISO `YYYY-MM-DD` calendar date. Rejects
/// wrong length, misplaced separators, non-digits, and impossible month/day
/// values (leap years included) — so the hashed, human-read edition marker is
/// always a canonical, meaningful date rather than arbitrary text.
fn validate_chart_date(s: &str) -> Result<(), LedgerError> {
    let bad = |why: &str| {
        LedgerError::Store(format!(
            "chart_date {s:?} is not a valid ISO YYYY-MM-DD date ({why})"
        ))
    };
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return Err(bad("expected YYYY-MM-DD"));
    }
    let digits = |lo: usize, hi: usize| b[lo..hi].iter().all(u8::is_ascii_digit);
    if !digits(0, 4) || !digits(5, 7) || !digits(8, 10) {
        return Err(bad("non-digit in a date position"));
    }
    let num = |lo: usize, hi: usize| s[lo..hi].parse::<u32>().expect("digits parse");
    let (year, month, day) = (num(0, 4), num(5, 7), num(8, 10));
    if !(1..=12).contains(&month) {
        return Err(bad("month out of range"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=max_day).contains(&day) {
        return Err(bad("day out of range for the month"));
    }
    Ok(())
}

/// `chart_version = "<chart>@<64 hex>"` — a stable content hash over the
/// canonical (ADR-2 JCS) bytes of the chart *document* (its id, edition
/// `chart_date`, and accounts). Reuses the fiscal wire encoder, so there is no
/// second hashing convention; the version is independent of the on-disk file's
/// human formatting and identifies exactly one chart edition + content. Bumping
/// `chart_date` (a new edition) therefore re-versions. Because the date is part
/// of the hashed preimage, an *accidental or partial* edit — the human
/// `chart_date` changed without re-deriving `chart_version`, or vice versa —
/// trips the `open` drift check (an internal-consistency guard, not
/// authentication: it does not stop someone who re-derives both in lockstep).
///
/// The **full** SHA-256 is retained: this string is pinned into every sealed
/// voucher as the chart it was recorded against, so it is a permanent content
/// address. Truncating would trade a legally-critical tamper-evidence property
/// for a few saved bytes.
fn compute_chart_version(
    chart: &str,
    chart_date: &str,
    accounts: &[Account],
) -> Result<String, LedgerError> {
    // Callers pass id-sorted accounts; sort defensively so the hash never
    // depends on input order even if a future caller forgets.
    let mut sorted = accounts.to_vec();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    let bytes = aggountant_wire::canonical_bytes(&ChartHashView {
        chart,
        chart_date,
        accounts: &sorted,
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(format!("{chart}@{}", hex::encode(digest)))
}

/// Render the chart file as human-diffable plaintext: one account per line, so a
/// single account added / removed / renamed is a single-line `git diff`. Valid
/// JSON (re-parsed by `open`); the content hash does not depend on this layout.
fn render_chart_file(chart: &str, chart_date: &str, accounts: &[Account]) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"chart\": {},\n",
        serde_json::to_string(chart).expect("chart id serializes")
    ));
    out.push_str(&format!(
        "  \"chart_date\": {},\n",
        serde_json::to_string(chart_date).expect("chart_date serializes")
    ));
    out.push_str("  \"accounts\": [\n");
    for (i, acc) in accounts.iter().enumerate() {
        let line = serde_json::to_string(acc).expect("account serializes");
        out.push_str("    ");
        out.push_str(&line);
        if i + 1 < accounts.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

/// The `ALREADY_INITIALIZED` refusal — shared by the pre-check and the
/// rename-lost-the-race path so the two report identically.
fn already_initialized(reference: &Path) -> LedgerError {
    LedgerError::AlreadyInitialized(format!(
        "reference masterdata already exists at {}",
        reference.join("chart-of-accounts.json").display()
    ))
}

/// A process-local monotonic counter for collision-free temp names, so two
/// threads in one process never stage into the same directory (PID alone is not
/// unique within a process — mirrors the artifact store's approach).
fn next_unique() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Write one file inside a private staging directory and fsync it. No temp
/// dance is needed here (the whole staging dir is published atomically by the
/// caller's `rename`); durability of the bytes still wants the fsync.
fn write_file_synced(path: &Path, contents: &[u8]) -> Result<(), LedgerError> {
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| LedgerError::Store(e.to_string()))?;
    f.write_all(contents)
        .map_err(|e| LedgerError::Store(e.to_string()))?;
    f.sync_all()
        .map_err(|e| LedgerError::Store(e.to_string()))?;
    Ok(())
}

/// Best-effort directory durability (so a rename/creation survives a crash).
/// Directory fsync is not meaningful on every platform; failure to open the
/// directory as a file is not fatal.
fn fsync_dir(dir: &Path) {
    if let Ok(dirf) = File::open(dir) {
        let _ = dirf.sync_all();
    }
}

/// Return the first suspected HETU/IBAN token in `s`, if any (fail-closed PII
/// canary — see the module docs). Kept deliberately conservative: it recognizes
/// Finnish IBAN (`FI` + 16 digits) and HETU (`ddmmyy` + century mark + `nnnc`)
/// shapes; a real classification gate for custom account authoring is future
/// work.
fn first_pii_token(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    if let Some(i) = (0..b.len().saturating_sub(17)).find(|&i| {
        b[i] == b'F' && b[i + 1] == b'I' && b[i + 2..i + 18].iter().all(u8::is_ascii_digit)
    }) {
        return Some(&s[i..i + 18]);
    }
    if let Some(i) = (0..b.len().saturating_sub(10)).find(|&i| {
        b[i..i + 6].iter().all(u8::is_ascii_digit)
            && matches!(b[i + 6], b'+' | b'-' | b'A' | b'B' | b'X' | b'Y')
            && b[i + 7..i + 10].iter().all(u8::is_ascii_digit)
            && (b[i + 10].is_ascii_digit() || b[i + 10].is_ascii_uppercase())
    }) {
        return Some(&s[i..i + 11]);
    }
    None
}

const README: &str = "\
# Reference masterdata (git-native, non-PII)

These files are **aggountant's git-native reference masterdata** (ADR-1 §Masterdata).
They are plaintext on purpose: commit them, `git diff` them, `git blame` them,
PR-review changes to the chart of accounts.

- `chart-of-accounts.json` — the chart of accounts (account number, name, kind),
  one account per line for clean diffs, plus a `chart_date` edition marker (ISO
  date) so you can tell which edition this is at a glance.
- `chart_version` — a content hash over the active chart; each sealed voucher pins the
  exact `chart_version` it was recorded against. It is derived from chart content.
- `vat-account-roles.json` — reviewed VAT account roles keyed by the exact chart version.
  Adoption fails closed unless the active chart has complete, chart-valid role metadata.
- `chart-snapshots/<sha256>.json` — immutable copies used to resolve each voucher under
  the account kinds it actually sealed. Never overwrite a snapshot; chart evolution
  creates another content-addressed file.

**No PII here.** Personal data (HETU, IBAN, tax cards, employee identity) never
lives in git — it goes in a separate, *erasable* encrypted store (GDPR). Do not
add personal names or identifiers to this tree.
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_then_open_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("masterdata");

        let written = GitMasterDataRefStore::init(&md).unwrap();
        let reopened = GitMasterDataRefStore::open(&md).unwrap();

        assert_eq!(
            written.chart_version().unwrap(),
            reopened.chart_version().unwrap()
        );
        assert_eq!(
            written.list_accounts().unwrap(),
            reopened.list_accounts().unwrap()
        );
        // The real chart is substantial, not the S1 5-account stub.
        assert!(reopened.list_accounts().unwrap().len() > 50);
        assert!(reopened
            .account_exists(&AccountId::new("7690").unwrap())
            .unwrap());
        assert!(!reopened
            .account_exists(&AccountId::new("9999").unwrap())
            .unwrap());
    }

    #[test]
    fn chart_version_is_deterministic_across_homes() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let va = GitMasterDataRefStore::init(&a.path().join("masterdata"))
            .unwrap()
            .chart_version()
            .unwrap();
        let vb = GitMasterDataRefStore::init(&b.path().join("masterdata"))
            .unwrap()
            .chart_version()
            .unwrap();
        assert_eq!(va, vb, "same chart content → same content hash");
        assert!(va.starts_with("fi-general-2026@"));
        // "<chart>@" + full 64 hex chars (32-byte SHA-256).
        assert_eq!(va.rsplit('@').next().unwrap().len(), 64);
    }

    #[test]
    fn init_refuses_to_clobber() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("masterdata");
        GitMasterDataRefStore::init(&md).unwrap();
        let err = GitMasterDataRefStore::init(&md).unwrap_err();
        assert_eq!(err.error_code(), "ALREADY_INITIALIZED");
    }

    #[test]
    fn open_before_init_is_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = GitMasterDataRefStore::open(&dir.path().join("masterdata")).unwrap_err();
        assert_eq!(err.error_code(), "MASTERDATA_NOT_INITIALIZED");
    }

    #[test]
    fn open_detects_chart_edited_without_reversioning() {
        let dir = tempfile::tempdir().unwrap();
        let md = dir.path().join("masterdata");
        GitMasterDataRefStore::init(&md).unwrap();

        // Hand-edit an account name but leave chart_version untouched.
        let chart_path = md.join("reference").join("chart-of-accounts.json");
        let raw = fs::read_to_string(&chart_path).unwrap();
        let tampered = raw.replace("Pankkitili", "Käyttötili");
        assert_ne!(raw, tampered, "the replacement must actually change bytes");
        fs::write(&chart_path, tampered).unwrap();

        let err = GitMasterDataRefStore::open(&md).unwrap_err();
        assert_eq!(err.error_code(), "MASTERDATA_CORRUPT");
    }

    #[test]
    fn pii_canary_flags_hetu_and_iban_but_not_the_chart() {
        // The write-path guard that keeps HETU/IBAN out of plaintext masterdata.
        assert!(first_pii_token("maksaja 131052-308T tili").is_some());
        assert!(first_pii_token("IBAN FI2112345600000785 loppu").is_some());
        assert!(first_pii_token("A1 010101A123B end").is_some());
        // The real bundled chart is clean.
        let chart = parse_chart(DEFAULT_CHART_JSON).unwrap();
        let rendered = render_chart_file(&chart.chart, &chart.chart_date, &chart.accounts);
        assert_eq!(first_pii_token(&rendered), None);
        // A 4-digit account id must not trip the HETU/IBAN heuristic.
        assert_eq!(first_pii_token(r#"{"id":"1910","kind":"asset"}"#), None);
    }

    #[test]
    fn validate_chart_date_accepts_iso_and_rejects_junk() {
        assert!(validate_chart_date("2026-01-01").is_ok());
        assert!(validate_chart_date("2024-02-29").is_ok()); // leap day
        for bad in [
            "",
            "2026-1-1",             // unpadded
            "2026-01-01 ",          // trailing space
            "2026-01-01T00:00:00Z", // datetime
            "2026-13-01",           // month out of range
            "2026-00-01",           // month zero
            "2026-02-30",           // impossible day
            "2025-02-29",           // non-leap
            "2026-04-31",           // 30-day month
            "not-a-date",
            "20260-1-01",
            "note about a person",
        ] {
            assert!(validate_chart_date(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn parse_chart_rejects_a_malformed_chart_date() {
        let json = r#"{"chart":"c","chart_date":"2026-13-40","accounts":[{"id":"1000","name":"X","kind":"asset"}]}"#;
        let err = parse_chart(json).unwrap_err();
        assert_eq!(err.error_code(), "STORE_ERROR");
        assert!(format!("{err}").contains("chart_date"));
    }

    #[test]
    fn rendered_chart_file_is_reparseable_and_one_line_per_account() {
        let chart = parse_chart(DEFAULT_CHART_JSON).unwrap();
        let rendered = render_chart_file(&chart.chart, &chart.chart_date, &chart.accounts);
        // Re-parses (valid JSON, strict shape).
        let reparsed = parse_chart(&rendered).unwrap();
        assert_eq!(reparsed.accounts, chart.accounts);
        // One `{"id":...}` object line per account (diffability). The top-level
        // `{` opens with a bare brace, so account objects are `{"`.
        let account_lines = rendered
            .lines()
            .filter(|l| l.trim_start().starts_with("{\""))
            .count();
        assert_eq!(account_lines, chart.accounts.len());
    }
}
