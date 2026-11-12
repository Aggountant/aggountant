//! SQLite-primary `fiscal` store (ADR-1). A single `BEGIN IMMEDIATE`
//! transaction performs: idempotency check (by `natural_key`, bound to
//! `request_hash`) → allocate the next gapless number → build canonical wire +
//! chain hash → insert voucher → CAS the counter → CAS the chain head → record
//! idempotency → enqueue the (stubbed) git-projection outbox. Racing writers
//! serialize on the reserved-writer lock (`busy_timeout`).
//!
//! Gaplessness is structural: the number appears only in the same transaction
//! that inserts the voucher and advances the counter, so a rolled-back or
//! crashed attempt consumes no number (ADR-1 §Decision). `UNIQUE(number)`/
//! `UNIQUE(hash)`, the CHECK constraints, and the two CAS predicates (each on
//! *both* old values) turn any logic bug or corruption into a loud failure
//! instead of a silent gap or fork.

use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::types::ToSql;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

use aggountant_model::{Date, FiscalCalendar, FiscalYear, Hash, ModelError, PeriodId};

use crate::error::LedgerError;
use crate::period::{FiscalYearState, PeriodState};
use crate::store::migrations::{self, Migration};
use crate::store::{
    AttributionRequest, AttributionResult, BuiltVoucher, CommitResult, DefinitionResult,
    FiscalSnapshot, FiscalStore, FiscalYearDefinitionRequest, FiscalYearRow,
    FiscalYearTransitionRequest, IdempotencyConflict, PeriodGate, PeriodRow, RejectionRecord,
    StoredAttribution, StoredFiscalYearTransition, StoredPeriodTransition, StoredProvenance,
    StoredRejection, StoredVoucher, TransitionRequest,
};

const SERIES: &str = "main";

/// The `fiscal` DB schema, as an ordered `PRAGMA user_version` migration list
/// (see [`migrations`]). **v1 is the baseline**: the full schema as it stood when
/// migrations were introduced, plus the single-series seed rows (`counter`,
/// `chain_head`). A truly fresh home runs this once and is stamped v1; a home that
/// predates the framework (tables present, `user_version` 0) is *refused* by
/// [`migrations::apply`], not adopted (see the migrations module docs). The seeds
/// live here — not in `open()` — so schema + initial state commit atomically and
/// an already-initialized store performs no writes on open (the `'main'` literal
/// duplicates the [`SERIES`] constant, which a unit test cross-checks). From v2
/// onward, append a new `Migration` describing the *change* (e.g.
/// `ALTER TABLE ... ADD COLUMN`) — never edit a landed migration's SQL.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: "CREATE TABLE IF NOT EXISTS counter(
             series TEXT PRIMARY KEY,
             next_number INTEGER NOT NULL CHECK(next_number >= 1)
          );
          CREATE TABLE IF NOT EXISTS chain_head(
             series TEXT PRIMARY KEY,
             head_hash BLOB NOT NULL,
             head_num INTEGER NOT NULL CHECK(head_num >= 0)
          );
          CREATE TABLE IF NOT EXISTS vouchers(
             number INTEGER PRIMARY KEY CHECK(number >= 1),
             series TEXT NOT NULL,
             prev_hash BLOB NOT NULL,
             hash BLOB NOT NULL UNIQUE,
             canonical_bytes BLOB NOT NULL,
             chart_version TEXT NOT NULL,
             sealed_at TEXT NOT NULL,
             actor TEXT NOT NULL,
             natural_key TEXT NOT NULL,
             request_hash TEXT NOT NULL,
             policy_set TEXT NOT NULL,
             sent_hash TEXT
          );
          CREATE TABLE IF NOT EXISTS idempotency(
             natural_key TEXT PRIMARY KEY,
             number INTEGER NOT NULL REFERENCES vouchers(number)
          );
          CREATE TABLE IF NOT EXISTS rejections(
             seq INTEGER PRIMARY KEY AUTOINCREMENT,
             prev_reject_hash BLOB NOT NULL,
             reason_code TEXT NOT NULL,
             request_hash TEXT NOT NULL,
             actor TEXT NOT NULL,
             target TEXT NOT NULL,
             note TEXT NOT NULL,
             ts TEXT NOT NULL,
             hash BLOB NOT NULL UNIQUE
          );
          CREATE TABLE IF NOT EXISTS git_projection_outbox(
             number INTEGER PRIMARY KEY REFERENCES vouchers(number),
             mirrored_at TEXT
          );
          -- ADR-6: the authoritative materialized period-state row (the O(1)
          -- indexed lookup the seal reads on every voucher write), advanced by
          -- CAS in the same BEGIN IMMEDIATE writer lock as the seal.
          CREATE TABLE IF NOT EXISTS period(
             period_id TEXT PRIMARY KEY,
             state TEXT NOT NULL CHECK(state IN ('Open','Closing','Sealed')),
             owning_run_id TEXT,
             vat_filed INTEGER NOT NULL DEFAULT 0 CHECK(vat_filed IN (0,1)),
             head_seq INTEGER NOT NULL DEFAULT 0 CHECK(head_seq >= 0),
             head_hash BLOB NOT NULL
          );
          -- ADR-6: the append-only, hash-chained transition log. The current
          -- period row is a fold of this; `fiscal verify` checks fold == row.
          CREATE TABLE IF NOT EXISTS period_transition(
             seq INTEGER PRIMARY KEY AUTOINCREMENT,
             period_id TEXT NOT NULL,
             period_seq INTEGER NOT NULL CHECK(period_seq >= 1),
             prev_hash BLOB NOT NULL,
             from_state TEXT NOT NULL,
             to_state TEXT NOT NULL,
             owning_run_id TEXT,
             -- S3: the sent_hash bound to a VAT-filing self-edge (Sealed→Sealed);
             -- NULL for a plain state transition. Part of the event hash preimage.
             sent_hash TEXT,
             actor TEXT NOT NULL,
             ts TEXT NOT NULL,
             hash BLOB NOT NULL UNIQUE,
             UNIQUE(period_id, period_seq)
          );
          -- S6: the authoritative materialized fiscal-year-state row (SealYear),
          -- advanced by CAS in the same BEGIN IMMEDIATE writer lock as the seal.
          CREATE TABLE IF NOT EXISTS fiscal_year(
             year INTEGER PRIMARY KEY,
             state TEXT NOT NULL CHECK(state IN ('Open','Sealed')),
             statements_hash TEXT,
             owning_run_id TEXT,
             head_seq INTEGER NOT NULL DEFAULT 0 CHECK(head_seq >= 0),
             head_hash BLOB NOT NULL
          );
          -- S6: the append-only, hash-chained fiscal-year transition log. The
          -- current fiscal_year row is a fold of this; `verify` checks fold == row.
          CREATE TABLE IF NOT EXISTS fiscal_year_transition(
             seq INTEGER PRIMARY KEY AUTOINCREMENT,
             year INTEGER NOT NULL,
             year_seq INTEGER NOT NULL CHECK(year_seq >= 1),
             prev_hash BLOB NOT NULL,
             from_state TEXT NOT NULL,
             to_state TEXT NOT NULL,
             statements_hash TEXT,
             owning_run_id TEXT,
             actor TEXT NOT NULL,
             ts TEXT NOT NULL,
             hash BLOB NOT NULL UNIQUE,
             UNIQUE(year, year_seq)
          );
          -- Seed the single 'main' series: the counter starts at voucher #1 and
          -- the chain head is the 32-byte zero hash (Hash::zero()) at height 0.
          -- 'main' duplicates the SERIES constant (cross-checked by a unit test);
          -- INSERT OR IGNORE keeps this safe even if v1 were ever re-run.
          INSERT OR IGNORE INTO counter(series, next_number) VALUES('main', 1);
          INSERT OR IGNORE INTO chain_head(series, head_hash, head_num)
             VALUES('main',
                    X'0000000000000000000000000000000000000000000000000000000000000000',
                    0);",
    },
    // v2 (issue `closeyear-adjustment-attribution`): the append-only, hash-chained
    // effective-year attribution log. Binds a sealed voucher to the statement year
    // its postings belong to, which may differ from the calendar year its
    // posting_date falls in (a year-end adjustment posts to the next open period
    // under A10 but must affect the *closing* year's result). At most one row per
    // voucher (attribution is immutable); the current mapping is a fold of this log,
    // and `verify` walks it. `seq` is the logical chain height (1,2,3,…, assigned as
    // count+1 in the writer lock — NOT AUTOINCREMENT), part of the event hash preimage
    // so a gap or reorder is caught, mirroring period_transition.period_seq.
    // `effective_year` is unconstrained here — the write path enforces the semantic
    // guards (no affected year Sealed, effective_year <= posting_year).
    Migration {
        version: 2,
        sql: "CREATE TABLE IF NOT EXISTS adjustment_attribution(
                 seq INTEGER PRIMARY KEY CHECK(seq >= 1),
                 voucher_number INTEGER NOT NULL UNIQUE REFERENCES vouchers(number),
                 effective_year INTEGER NOT NULL,
                 prev_hash BLOB NOT NULL,
                 owning_run_id TEXT NOT NULL,
                 actor TEXT NOT NULL,
                 ts TEXT NOT NULL,
                 hash BLOB NOT NULL UNIQUE
              );",
    },
    // v3 (issue `provenance-software-version`): build provenance is metadata,
    // outside canonical_bytes and therefore outside the voucher hash preimage.
    // Existing rows receive an explicit legacy value rather than a fabricated SHA.
    Migration {
        version: 3,
        sql: "ALTER TABLE vouchers ADD COLUMN software_version TEXT NOT NULL
                 DEFAULT 'unknown-legacy';",
    },
    // v4 (issue `import-seal-source-fingerprint`): source-event provenance is
    // compared before idempotent replay but remains outside canonical_bytes and
    // the voucher hash preimage. NULL denotes a legacy or non-source seal.
    Migration {
        version: 4,
        sql: "ALTER TABLE vouchers ADD COLUMN source_fingerprint TEXT;",
    },
    // v5: immutable Vero FileVATReturn/v2 receipt fields.
    Migration {
        version: 5,
        sql: "ALTER TABLE period_transition ADD COLUMN receipt_unique_identifier TEXT;
              ALTER TABLE period_transition ADD COLUMN receipt_accepted_timestamp TEXT;",
    },
    Migration {
        version: 6,
        sql: "ALTER TABLE period_transition ADD COLUMN manual_channel TEXT;
              ALTER TABLE period_transition ADD COLUMN manual_evidence_ref TEXT;
              ALTER TABLE period_transition ADD COLUMN remote_status_evidence_ref TEXT;",
    },
    // v7 (issue `non-calendar-fiscal-year`): explicit fiscal-year definitions,
    // append-only. No rows means calendar years, so an existing home reads exactly as
    // before. The calendar derived from these rows (`FiscalCalendar`) decides which
    // fiscal year every voucher belongs to, which is why it lives here, written in the
    // same writer lock that seals vouchers and years.
    Migration {
        version: 7,
        sql: "CREATE TABLE IF NOT EXISTS fiscal_year_definition(
                 start_date TEXT PRIMARY KEY,
                 end_date TEXT NOT NULL,
                 actor TEXT NOT NULL,
                 recorded_at TEXT NOT NULL
              );",
    },
];

pub(crate) struct SqliteFiscalStore {
    // `Mutex` (not `RefCell`) so a single `Ledger` is `Send + Sync` and sharing
    // it across threads serializes rather than panicking. Cross-*process*
    // racing (the ADR-1 concurrency bet) still uses independent connections.
    conn: Mutex<Connection>,
}

fn hash_from_blob(blob: &[u8]) -> Result<Hash, LedgerError> {
    let arr: [u8; 32] = blob.try_into().map_err(|_| {
        LedgerError::Store(format!("hash blob is {} bytes, expected 32", blob.len()))
    })?;
    Ok(Hash::from_bytes(arr))
}

/// Load one voucher row into a `StoredVoucher` (used for idempotent replay).
fn load_voucher(conn: &Connection, number: u64) -> Result<StoredVoucher, LedgerError> {
    conn.query_row(
        "SELECT prev_hash, hash, canonical_bytes, chart_version, sealed_at,
                actor, natural_key, request_hash, source_fingerprint, policy_set, software_version, sent_hash
         FROM vouchers WHERE number = ?1",
        params![number],
        row_to_stored_voucher(number),
    )
    .map_err(LedgerError::from)?
}

/// Build a rusqlite row mapper that yields `Result<StoredVoucher, LedgerError>`
/// (the outer `query_row` gives `Result<_, rusqlite::Error>`; we flatten).
#[allow(clippy::type_complexity)]
fn row_to_stored_voucher(
    number: u64,
) -> impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Result<StoredVoucher, LedgerError>> {
    move |r| {
        let prev: Vec<u8> = r.get(0)?;
        let hash: Vec<u8> = r.get(1)?;
        let canonical: Vec<u8> = r.get(2)?;
        let chart_version: String = r.get(3)?;
        let sealed_at: String = r.get(4)?;
        let actor: String = r.get(5)?;
        let natural_key: String = r.get(6)?;
        let request_hash: String = r.get(7)?;
        let source_fingerprint: Option<String> = r.get(8)?;
        let policy_set: String = r.get(9)?;
        let software_version: String = r.get(10)?;
        let sent_hash: Option<String> = r.get(11)?;
        Ok((|| {
            Ok(StoredVoucher {
                number,
                prev_hash: hash_from_blob(&prev)?,
                hash: hash_from_blob(&hash)?,
                canonical,
                chart_version: chart_version.clone(),
                sealed_at: sealed_at.clone(),
                provenance: StoredProvenance {
                    actor,
                    natural_key,
                    request_hash,
                    source_fingerprint,
                    policy_set,
                    software_version,
                    chart_version,
                    sealed_at,
                    sent_hash,
                },
            })
        })())
    }
}

impl SqliteFiscalStore {
    pub(crate) fn open(path: &Path) -> Result<Self, LedgerError> {
        let mut conn = Connection::open(path)?;
        // Install the busy handler FIRST, before any pragma that can contend for a
        // lock (WAL setup can), so a concurrent open waits instead of failing with
        // an immediate SQLITE_BUSY.
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        // `foreign_keys` is deliberately left OFF (the connection default) across
        // `apply`: FK enforcement must be off for the SQLite table-rebuild a future
        // migration may need, and the pragma is a no-op inside `apply`'s
        // transaction. We enable it below for runtime enforcement.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;",
        )?;
        migrations::apply(&mut conn, "fiscal", MIGRATIONS)?;
        // Enforce foreign keys for all runtime writes (the idempotency/outbox rows
        // reference vouchers). Must be set outside a transaction — `apply` has
        // committed and returned by now.
        conn.pragma_update(None, "foreign_keys", true)?;
        Ok(SqliteFiscalStore {
            conn: Mutex::new(conn),
        })
    }
}

impl FiscalStore for SqliteFiscalStore {
    fn commit(
        &self,
        provenance: &StoredProvenance,
        gate: &PeriodGate<'_>,
        build: &dyn Fn(u64, &Hash) -> Result<BuiltVoucher, LedgerError>,
    ) -> Result<CommitResult, LedgerError> {
        let mut guard = self.conn.lock().unwrap();
        let tx = guard.transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Idempotency (natural_key), independently bound to both projected
        // request content and the pre-projection source event when one is supplied.
        if let Some((number, existing_rh)) = tx
            .query_row(
                "SELECT v.number, v.request_hash
                 FROM idempotency i JOIN vouchers v ON v.number = i.number
                 WHERE i.natural_key = ?1",
                params![provenance.natural_key],
                |r| Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
        {
            let existing = load_voucher(&tx, number)?;
            // A stored NULL means the original seal carried no authenticated source
            // fingerprint. A later source-aware retry may replay the unchanged fiscal
            // request, but must not rewrite history by adopting an unverifiable value.
            // Conversely, once a key is source-bound, omitting its fingerprint is a
            // conflict rather than a way to bypass source-change detection.
            let source_conflict = match (
                existing.provenance.source_fingerprint.as_deref(),
                provenance.source_fingerprint.as_deref(),
            ) {
                (None, _) => false,
                (Some(stored), Some(incoming)) => stored != incoming,
                (Some(_), None) => true,
            };
            if existing_rh != provenance.request_hash || source_conflict {
                tx.commit()?;
                return Ok(CommitResult::Conflict(IdempotencyConflict {
                    existing,
                    existing_request_hash: existing_rh,
                    incoming_source_fingerprint: provenance.source_fingerprint.clone(),
                    source_conflict,
                }));
            }

            tx.commit()?;
            return Ok(CommitResult::Replayed(existing));
        }

        // ADR-6 period gate (F5), evaluated INSIDE the writer lock so it shares
        // the seal's transaction domain: a concurrent TransitionPeriod either
        // committed before us (we see `Closing`/`Sealed` and refuse) or blocks
        // behind us (it will see our voucher committed into a still-`Open`
        // period). There is no window where a voucher seals into a period a
        // close-run believes is `Closing`.
        // A posting date can be governed by either a new calendar-month row or its
        // retained legacy-quarter row. At most those two canonical ids can contain
        // the date, so use indexed lookups rather than scanning period history.
        // Every voucher must belong to a fiscal year, or no statement projection could
        // place it. Read in the writer lock, so a concurrent definition cannot slip in.
        if let Err(error) = load_calendar(&tx)?.year_of(gate.posting_date) {
            let refusal = LedgerError::Model(error);
            insert_rejection(
                &tx,
                &RejectionRecord {
                    reason_code: refusal.error_code().to_string(),
                    request_hash: provenance.request_hash.clone(),
                    actor: provenance.actor.clone(),
                    target: format!("fiscal:date:{}", gate.posting_date),
                    note: refusal.to_string(),
                },
            )?;
            tx.commit()?;
            return Ok(CommitResult::Refused(refusal));
        }

        let containing = [
            PeriodId::month_containing(gate.posting_date),
            PeriodId::containing(gate.posting_date),
        ];
        for period in containing {
            let Some(prow) = read_period_row_opt(&tx, &period.to_string())? else {
                continue;
            };
            if let Some(refusal) = evaluate_period_gate(&prow, gate) {
                insert_rejection(
                    &tx,
                    &RejectionRecord {
                        reason_code: refusal.error_code().to_string(),
                        request_hash: provenance.request_hash.clone(),
                        actor: provenance.actor.clone(),
                        target: format!("fiscal:period:{}", prow.period_id),
                        note: refusal.to_string(),
                    },
                )?;
                tx.commit()?;
                return Ok(CommitResult::Refused(refusal));
            }
        }

        let next_number: u64 = tx.query_row(
            "SELECT next_number FROM counter WHERE series = ?1",
            params![SERIES],
            |r| r.get(0),
        )?;
        let (head_blob, head_num): (Vec<u8>, u64) = tx.query_row(
            "SELECT head_hash, head_num FROM chain_head WHERE series = ?1",
            params![SERIES],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        // Counter and chain head must agree, or the store is internally
        // inconsistent — fail loud rather than extend a corrupt state.
        if head_num.checked_add(1) != Some(next_number) {
            return Err(LedgerError::Store(format!(
                "counter/head divergence: next_number={next_number}, head_num={head_num}"
            )));
        }
        let following = next_number
            .checked_add(1)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or_else(|| LedgerError::Store("voucher number space exhausted".into()))?;
        let prev_hash = hash_from_blob(&head_blob)?;

        // Pure, deterministic build (wire crate) with the allocated number and
        // previous hash — the only place the chain hash is computed.
        let built = build(next_number, &prev_hash)?;
        if built.body.number() != next_number {
            return Err(LedgerError::Store(format!(
                "built voucher number {} != allocated {}",
                built.body.number(),
                next_number
            )));
        }

        tx.execute(
            "INSERT INTO vouchers(number, series, prev_hash, hash, canonical_bytes,
                                  chart_version, sealed_at, actor, natural_key, request_hash,
                                  source_fingerprint, policy_set, software_version, sent_hash)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                next_number,
                SERIES,
                prev_hash.as_bytes().as_slice(),
                built.hash.as_bytes().as_slice(),
                built.canonical,
                provenance.chart_version,
                provenance.sealed_at,
                provenance.actor,
                provenance.natural_key,
                provenance.request_hash,
                provenance.source_fingerprint,
                provenance.policy_set,
                provenance.software_version,
                provenance.sent_hash,
            ],
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(f, _)
                if f.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                LedgerError::VoucherNumberTaken(format!(
                    "number {next_number} or its hash already present"
                ))
            }
            other => LedgerError::from(other),
        })?;

        // CAS the counter and the chain head on *both* prior values.
        let counter_updated = tx.execute(
            "UPDATE counter SET next_number = ?1 WHERE series = ?2 AND next_number = ?3",
            params![following, SERIES, next_number],
        )?;
        if counter_updated != 1 {
            return Err(LedgerError::CasConflict(format!(
                "counter advanced under us at number {next_number}"
            )));
        }
        let head_updated = tx.execute(
            "UPDATE chain_head SET head_hash = ?1, head_num = ?2
             WHERE series = ?3 AND head_hash = ?4 AND head_num = ?5",
            params![
                built.hash.as_bytes().as_slice(),
                next_number,
                SERIES,
                prev_hash.as_bytes().as_slice(),
                head_num,
            ],
        )?;
        if head_updated != 1 {
            return Err(LedgerError::CasConflict(format!(
                "chain head advanced under us at number {next_number}"
            )));
        }

        tx.execute(
            "INSERT INTO idempotency(natural_key, number) VALUES(?1, ?2)",
            params![provenance.natural_key, next_number],
        )?;
        tx.execute(
            "INSERT INTO git_projection_outbox(number, mirrored_at) VALUES(?1, NULL)",
            params![next_number],
        )?;

        let voucher = StoredVoucher {
            number: next_number,
            prev_hash,
            hash: built.hash,
            canonical: built.canonical,
            chart_version: provenance.chart_version.clone(),
            sealed_at: provenance.sealed_at.clone(),
            provenance: provenance.clone(),
        };
        tx.commit()?;
        Ok(CommitResult::Sealed(voucher))
    }

    fn record_rejection(&self, rej: &RejectionRecord) -> Result<(), LedgerError> {
        let mut guard = self.conn.lock().unwrap();
        let tx = guard.transaction_with_behavior(TransactionBehavior::Immediate)?;
        insert_rejection(&tx, rej)?;
        tx.commit()?;
        Ok(())
    }

    fn read_period(&self, period_id: &str) -> Result<PeriodRow, LedgerError> {
        let guard = self.conn.lock().unwrap();
        read_period_row(&guard, period_id)
    }

    fn transition_period(&self, req: &TransitionRequest) -> Result<PeriodRow, LedgerError> {
        let mut guard = self.conn.lock().unwrap();
        // Same BEGIN IMMEDIATE writer lock as the seal — seals and transitions
        // serialize here (ADR-6 §2).
        let tx = guard.transaction_with_behavior(TransactionBehavior::Immediate)?;

        // S6: once the containing fiscal year is Sealed, no subperiod may transition
        // (the reopen edge would otherwise let a sealed year diverge from its bound
        // `statements_hash`). Checked inside the same writer lock as the year seal, so
        // there is no window. The VAT-filing self-edge (`Sealed→Sealed`) is exempt —
        // filing does not change the ledger figures the year seal bound.
        if !(req.from == PeriodState::Sealed && req.to == PeriodState::Sealed) {
            let calendar = load_calendar(&tx)?;
            for year in fiscal_years_of_period(&calendar, &req.period_id)? {
                let fy = read_fiscal_year_row(&tx, year)?;
                if fy.state == FiscalYearState::Sealed {
                    let err = LedgerError::PeriodTransitionConflict(format!(
                        "period {} cannot transition {}→{}: fiscal year {year} is Sealed",
                        req.period_id,
                        req.from.as_str(),
                        req.to.as_str()
                    ));
                    insert_rejection(
                        &tx,
                        &RejectionRecord {
                            reason_code: err.error_code().to_string(),
                            request_hash: String::new(),
                            actor: req.actor.clone(),
                            target: format!("fiscal:period:{}", req.period_id),
                            note: err.to_string(),
                        },
                    )?;
                    tx.commit()?;
                    return Err(err);
                }
            }
        }

        let requested = PeriodId::parse(&req.period_id).map_err(|error| {
            LedgerError::Store(format!(
                "invalid requested period_id {:?}: {error}",
                req.period_id
            ))
        })?;
        let existing = read_period_row_opt(&tx, &req.period_id)?;
        let cur = existing
            .clone()
            .unwrap_or_else(|| implicit_open(&req.period_id));

        // CAS guards: current state must equal `from`; Seal/Reopen additionally
        // require the presented run to still own the period (fencing re-check);
        // Reopen requires the VAT return not yet filed (ADR-6 §5).
        let state_ok = cur.state == req.from;
        let owner_ok =
            !req.match_owner || cur.owning_run_id.as_deref() == Some(req.run_id.as_str());
        let filed_ok = !req.require_unfiled || !cur.vat_filed;

        if !(state_ok && owner_ok && filed_ok) {
            let err = if state_ok && filed_ok && req.match_owner && !owner_ok {
                // Right state, but the presented authority does not own the period:
                // a stale/foreign token (ADR-6 §Biggest risk). Fenced.
                LedgerError::VoucherNotInRun(format!(
                    "period {} is owned by {:?}, not run {} — {}→{} refused",
                    req.period_id,
                    cur.owning_run_id,
                    req.run_id,
                    req.from.as_str(),
                    req.to.as_str()
                ))
            } else {
                LedgerError::PeriodTransitionConflict(format!(
                    "period {} is {} (expected {}), filed={}: {}→{} CAS matched no row",
                    req.period_id,
                    cur.state.as_str(),
                    req.from.as_str(),
                    cur.vat_filed,
                    req.from.as_str(),
                    req.to.as_str()
                ))
            };
            // Log the refusal in the rejection namespace INSIDE the same
            // transaction, COMMIT, and return Refused (ADR-6 §2 failure path).
            insert_rejection(
                &tx,
                &RejectionRecord {
                    reason_code: err.error_code().to_string(),
                    request_hash: String::new(),
                    actor: req.actor.clone(),
                    target: format!("fiscal:period:{}", req.period_id),
                    note: err.to_string(),
                },
            )?;
            tx.commit()?;
            return Err(err);
        }

        // A calendar year uses one active coverage granularity at a time. Ignore
        // reopened Open remnants so a deliberate month↔quarter switch is possible,
        // but reject activating any month alongside any active quarter in that year,
        // even when they do not overlap (Q1 + October would make year close impossible).
        // Parsing every stored id here also fails visibly on malformed fiscal state.
        if req.to != PeriodState::Open {
            for other in read_all_period_rows(&tx)? {
                if other.period_id == req.period_id {
                    continue;
                }
                let other_id = PeriodId::parse(&other.period_id).map_err(|error| {
                    LedgerError::Store(format!(
                        "invalid period_id {:?} in fiscal store: {error}",
                        other.period_id
                    ))
                })?;
                let mixed_active = requested.year() == other_id.year()
                    && requested.is_month() != other_id.is_month()
                    && (other.state != PeriodState::Open || other.vat_filed);
                if mixed_active {
                    let err = LedgerError::PeriodTransitionConflict(format!(
                        "period {} cannot transition {}→{} while {} is {}: calendar year {} cannot mix active month and quarter coverage",
                        req.period_id,
                        req.from.as_str(),
                        req.to.as_str(),
                        other.period_id,
                        other.state.as_str(),
                        requested.year()
                    ));
                    insert_rejection(
                        &tx,
                        &RejectionRecord {
                            reason_code: err.error_code().to_string(),
                            request_hash: String::new(),
                            actor: req.actor.clone(),
                            target: format!("fiscal:period:{}", req.period_id),
                            note: err.to_string(),
                        },
                    )?;
                    tx.commit()?;
                    return Err(err);
                }
            }
        }

        // Receipt fields are an all-or-nothing consequence of a complete HTTP 200.
        if req.receipt_unique_identifier.is_some() != req.receipt_accepted_timestamp.is_some() {
            return Err(LedgerError::Store(
                "VAT filing receipt fields must be stored together".into(),
            ));
        }
        if req.manual_channel.is_some() != req.manual_evidence_ref.is_some()
            || (req.manual_channel.is_none() && req.remote_status_evidence_ref.is_some())
            || (req.manual_channel.is_some() && req.sent_hash.is_some())
            || (req.manual_channel.is_some() && req.receipt_unique_identifier.is_some())
        {
            return Err(LedgerError::Store(
                "manual VAT filing requires channel + evidence only, with optional remote-status evidence".into(),
            ));
        }

        // Success: append one immutable transition event, advance the row by CAS.
        let new_seq = cur
            .head_seq
            .checked_add(1)
            .ok_or_else(|| LedgerError::Store("period transition sequence overflow".into()))?;
        let ts = Utc::now().to_rfc3339();
        let hash = period_transition_hash(
            &cur.head_hash,
            &req.period_id,
            new_seq,
            req.from.as_str(),
            req.to.as_str(),
            Some(req.run_id.as_str()),
            req.sent_hash.as_deref(),
            req.receipt_unique_identifier.as_deref(),
            req.receipt_accepted_timestamp.as_deref(),
            req.manual_channel.as_deref(),
            req.manual_evidence_ref.as_deref(),
            req.remote_status_evidence_ref.as_deref(),
            &req.actor,
            &ts,
        );
        tx.execute(
            "INSERT INTO period_transition(period_id, period_seq, prev_hash, from_state,
                                           to_state, owning_run_id, sent_hash,
                                           receipt_unique_identifier, receipt_accepted_timestamp,
                                           manual_channel, manual_evidence_ref,
                                           remote_status_evidence_ref, actor, ts, hash)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                req.period_id,
                new_seq,
                cur.head_hash.as_bytes().as_slice(),
                req.from.as_str(),
                req.to.as_str(),
                req.run_id,
                req.sent_hash,
                req.receipt_unique_identifier,
                req.receipt_accepted_timestamp,
                req.manual_channel,
                req.manual_evidence_ref,
                req.remote_status_evidence_ref,
                req.actor,
                ts,
                hash.as_bytes().as_slice(),
            ],
        )?;

        let new_row = PeriodRow {
            period_id: req.period_id.clone(),
            state: req.to,
            owning_run_id: Some(req.run_id.clone()),
            vat_filed: cur.vat_filed || req.set_filed,
            head_seq: new_seq,
            head_hash: hash,
        };

        if existing.is_some() {
            // A real CAS on the row: pin state=from (+ owner/unfiled guards) so a
            // logic error surfaces as rowcount 0 rather than a silent overwrite.
            let new_state_str = req.to.as_str();
            let from_str = req.from.as_str();
            let head_hash_blob = new_row.head_hash.as_bytes().to_vec();
            // The VAT-filing self-edge additionally flips vat_filed 0→1 in the same
            // CAS, so the flag and its audit event are one atomic write.
            let set_filed_clause = if req.set_filed { ", vat_filed=1" } else { "" };
            let mut sql = format!(
                "UPDATE period SET state=?1, owning_run_id=?2, head_seq=?3, head_hash=?4{set_filed_clause} \
                 WHERE period_id=?5 AND state=?6",
            );
            let mut sql_params: Vec<&dyn ToSql> = vec![
                &new_state_str,
                &new_row.owning_run_id,
                &new_row.head_seq,
                &head_hash_blob,
                &req.period_id,
                &from_str,
            ];
            if req.match_owner {
                sql.push_str(" AND owning_run_id=?7");
                sql_params.push(&req.run_id);
            }
            if req.require_unfiled {
                sql.push_str(" AND vat_filed=0");
            }
            let n = tx.execute(&sql, sql_params.as_slice())?;
            if n != 1 {
                return Err(LedgerError::CasConflict(format!(
                    "period {} row moved under the writer lock during {}→{}",
                    req.period_id,
                    req.from.as_str(),
                    req.to.as_str()
                )));
            }
        } else {
            // No physical row yet (implicit Open) → this is the first BeginClose.
            tx.execute(
                "INSERT INTO period(period_id, state, owning_run_id, vat_filed, head_seq, head_hash)
                 VALUES(?1, ?2, ?3, 0, ?4, ?5)",
                params![
                    req.period_id,
                    req.to.as_str(),
                    req.run_id,
                    new_seq,
                    new_row.head_hash.as_bytes().as_slice(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(new_row)
    }

    fn last_filing_sent_hash(
        &self,
        period_id: &str,
    ) -> Result<Option<Option<String>>, LedgerError> {
        let guard = self.conn.lock().unwrap();
        // The latest Sealed→Sealed self-edge is the (single) filing event.
        let row: Option<Option<String>> = guard
            .query_row(
                "SELECT sent_hash FROM period_transition
                 WHERE period_id = ?1 AND from_state = 'Sealed' AND to_state = 'Sealed'
                 ORDER BY period_seq DESC LIMIT 1",
                params![period_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?;
        Ok(row)
    }

    fn last_filing_receipt(
        &self,
        period_id: &str,
    ) -> Result<Option<(String, String)>, LedgerError> {
        let guard = self.conn.lock().unwrap();
        guard
            .query_row(
                "SELECT receipt_unique_identifier, receipt_accepted_timestamp
                 FROM period_transition
                 WHERE period_id = ?1 AND from_state = 'Sealed' AND to_state = 'Sealed'
                 ORDER BY period_seq DESC LIMIT 1",
                params![period_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()?
            .and_then(|(identifier, timestamp)| identifier.zip(timestamp))
            .map_or(Ok(None), |pair| Ok(Some(pair)))
    }

    fn last_manual_filing_evidence(
        &self,
        period_id: &str,
    ) -> Result<Option<(String, String, Option<String>)>, LedgerError> {
        let guard = self.conn.lock().unwrap();
        guard
            .query_row(
                "SELECT manual_channel, manual_evidence_ref, remote_status_evidence_ref
                 FROM period_transition
                 WHERE period_id = ?1 AND from_state = 'Sealed' AND to_state = 'Sealed'
                 ORDER BY period_seq DESC LIMIT 1",
                params![period_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?
            .and_then(|(channel, evidence, remote)| {
                channel
                    .zip(evidence)
                    .map(|(channel, evidence)| (channel, evidence, remote))
            })
            .map_or(Ok(None), |value| Ok(Some(value)))
    }

    fn read_fiscal_year(&self, year: i32) -> Result<FiscalYearRow, LedgerError> {
        let guard = self.conn.lock().unwrap();
        read_fiscal_year_row(&guard, year)
    }

    fn transition_fiscal_year(
        &self,
        req: &FiscalYearTransitionRequest,
    ) -> Result<FiscalYearRow, LedgerError> {
        let mut guard = self.conn.lock().unwrap();
        // Same BEGIN IMMEDIATE writer lock as the seal — the subperiod precondition
        // check and the year CAS share one transaction domain, so a concurrent
        // period transition or voucher seal cannot open a window where the year
        // locks over an unsealed subperiod (S6 atomicity).
        let tx = guard.transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Precondition: every required subperiod must be Sealed (read inside the
        // lock). A refusal is logged and the transaction committed (audited failure).
        for pid in &req.required_sealed_periods {
            let prow = read_period_row(&tx, pid)?;
            if prow.state != PeriodState::Sealed {
                let err = LedgerError::PeriodNotSealed(format!(
                    "cannot seal year {}: subperiod {pid} is {} (all subperiods must be Sealed)",
                    req.year,
                    prow.state.as_str()
                ));
                insert_rejection(
                    &tx,
                    &RejectionRecord {
                        reason_code: err.error_code().to_string(),
                        request_hash: String::new(),
                        actor: req.actor.clone(),
                        target: format!("fiscal:year:{}", req.year),
                        note: err.to_string(),
                    },
                )?;
                tx.commit()?;
                return Err(err);
            }
        }

        let existing = read_fiscal_year_row_opt(&tx, req.year)?;
        let cur = existing
            .clone()
            .unwrap_or_else(|| implicit_open_year(req.year));

        if cur.state != req.from {
            let err = LedgerError::PeriodTransitionConflict(format!(
                "fiscal year {} is {} (expected {}): {}→{} CAS matched no row \
                 (a year seal is irreversible)",
                req.year,
                cur.state.as_str(),
                req.from.as_str(),
                req.from.as_str(),
                req.to.as_str()
            ));
            insert_rejection(
                &tx,
                &RejectionRecord {
                    reason_code: err.error_code().to_string(),
                    request_hash: String::new(),
                    actor: req.actor.clone(),
                    target: format!("fiscal:year:{}", req.year),
                    note: err.to_string(),
                },
            )?;
            tx.commit()?;
            return Err(err);
        }

        let new_seq = cur
            .head_seq
            .checked_add(1)
            .ok_or_else(|| LedgerError::Store("fiscal-year transition sequence overflow".into()))?;
        let ts = Utc::now().to_rfc3339();
        let hash = fiscal_year_transition_hash(
            &cur.head_hash,
            req.year,
            new_seq,
            req.from.as_str(),
            req.to.as_str(),
            req.statements_hash.as_deref(),
            Some(req.run_id.as_str()),
            &req.actor,
            &ts,
        );
        tx.execute(
            "INSERT INTO fiscal_year_transition(year, year_seq, prev_hash, from_state,
                                                to_state, statements_hash, owning_run_id, actor, ts, hash)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                req.year,
                new_seq,
                cur.head_hash.as_bytes().as_slice(),
                req.from.as_str(),
                req.to.as_str(),
                req.statements_hash,
                req.run_id,
                req.actor,
                ts,
                hash.as_bytes().as_slice(),
            ],
        )?;

        let new_row = FiscalYearRow {
            year: req.year,
            state: req.to,
            statements_hash: req.statements_hash.clone(),
            owning_run_id: Some(req.run_id.clone()),
            head_seq: new_seq,
            head_hash: hash,
        };

        if existing.is_some() {
            let from_str = req.from.as_str();
            let to_str = req.to.as_str();
            let head_blob = new_row.head_hash.as_bytes().to_vec();
            let n = tx.execute(
                "UPDATE fiscal_year SET state=?1, statements_hash=?2, owning_run_id=?3,
                        head_seq=?4, head_hash=?5
                 WHERE year=?6 AND state=?7",
                params![
                    to_str,
                    new_row.statements_hash,
                    new_row.owning_run_id,
                    new_row.head_seq,
                    head_blob,
                    req.year,
                    from_str,
                ],
            )?;
            if n != 1 {
                return Err(LedgerError::CasConflict(format!(
                    "fiscal year {} row moved under the writer lock during {}→{}",
                    req.year, from_str, to_str
                )));
            }
        } else {
            tx.execute(
                "INSERT INTO fiscal_year(year, state, statements_hash, owning_run_id, head_seq, head_hash)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    req.year,
                    req.to.as_str(),
                    new_row.statements_hash,
                    req.run_id,
                    new_seq,
                    new_row.head_hash.as_bytes().as_slice(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(new_row)
    }

    fn fiscal_calendar(&self) -> Result<FiscalCalendar, LedgerError> {
        let guard = self.conn.lock().unwrap();
        load_calendar(&guard)
    }

    fn define_fiscal_year(
        &self,
        req: &FiscalYearDefinitionRequest,
    ) -> Result<DefinitionResult, LedgerError> {
        let mut guard = self.conn.lock().unwrap();
        // The seal's writer lock: no voucher or year seal can land between the check
        // against recorded state and the append.
        let tx = guard.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let old = load_calendar(&tx)?;
        if old.defined().contains(&req.year) {
            tx.commit()?;
            return Ok(DefinitionResult::Replayed(old));
        }
        let new = old.with_definition(req.year)?;

        let refuse =
            |tx: rusqlite::Transaction<'_>, note: String| -> Result<LedgerError, LedgerError> {
                let err = LedgerError::FiscalYearConflict(note);
                insert_rejection(
                    &tx,
                    &RejectionRecord {
                        reason_code: err.error_code().to_string(),
                        request_hash: String::new(),
                        actor: req.actor.clone(),
                        target: format!("fiscal:year-definition:{}", req.year.start()),
                        note: err.to_string(),
                    },
                )?;
                tx.commit()?;
                Ok(err)
            };

        // A sealed voucher keeps the fiscal year it was sealed into.
        let mut moved = None;
        {
            let mut stmt =
                tx.prepare("SELECT number, canonical_bytes FROM vouchers ORDER BY number")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, u64>(0)?, r.get::<_, Vec<u8>>(1)?)))?;
            for row in rows {
                let (number, canonical) = row?;
                let date = aggountant_wire::decode_voucher_body(&canonical)?.posting_date();
                let before = old.year_of(date).ok().map(|y| y.year());
                let after = new.year_of(date).ok().map(|y| y.year());
                if before != after {
                    moved = Some((number, date, before, after));
                    break;
                }
            }
        }
        if let Some((number, date, before, after)) = moved {
            let label =
                |year: Option<i32>| year.map_or("no fiscal year".to_string(), |y| y.to_string());
            return Err(refuse(
                tx,
                format!(
                    "fiscal year {}..{} would move sealed voucher {number} (dated {date}) from \
                     fiscal year {} to {}; a definition may not change the year of anything \
                     already sealed",
                    req.year.start(),
                    req.year.end(),
                    label(before),
                    label(after)
                ),
            )?);
        }

        // A year with recorded state keeps its exact dates.
        let mut changed = None;
        {
            let mut stmt = tx.prepare("SELECT year FROM fiscal_year ORDER BY year")?;
            let years = stmt.query_map([], |r| r.get::<_, i32>(0))?;
            for year in years {
                let year = year?;
                if old.year(year).ok() != new.year(year).ok() {
                    changed = Some(year);
                    break;
                }
            }
        }
        if let Some(year) = changed {
            return Err(refuse(
                tx,
                format!(
                    "fiscal year {}..{} would change the dates of fiscal year {year}, which \
                     already has recorded state",
                    req.year.start(),
                    req.year.end()
                ),
            )?);
        }

        tx.execute(
            "INSERT INTO fiscal_year_definition(start_date, end_date, actor, recorded_at)
             VALUES(?1, ?2, ?3, ?4)",
            params![
                req.year.start().to_string(),
                req.year.end().to_string(),
                req.actor,
                Utc::now().to_rfc3339(),
            ],
        )?;
        tx.commit()?;
        Ok(DefinitionResult::Recorded(new))
    }

    fn read_voucher(&self, number: u64) -> Result<Option<StoredVoucher>, LedgerError> {
        let guard = self.conn.lock().unwrap();
        let raw = guard
            .query_row(
                "SELECT prev_hash, hash, canonical_bytes, chart_version, sealed_at,
                        actor, natural_key, request_hash, source_fingerprint, policy_set, software_version, sent_hash
                 FROM vouchers WHERE number = ?1",
                params![number],
                row_to_stored_voucher(number),
            )
            .optional()?;
        raw.transpose()
    }

    fn attribute_adjustment(
        &self,
        req: &AttributionRequest,
    ) -> Result<AttributionResult, LedgerError> {
        let mut guard = self.conn.lock().unwrap();
        // Same BEGIN IMMEDIATE writer lock as SealYear, so the "target year not yet
        // Sealed" guard and the append share one transaction domain: a concurrent
        // SealYear either committed first (we see Sealed and refuse) or blocks behind
        // us (it seals a year whose attribution set is already final).
        let tx = guard.transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Idempotency / immutability: a voucher is attributed at most once. A repeat to
        // the same year replays; a different year is a refused reuse (an attribution can
        // never be silently rewritten once it may have entered a sealed year's fold).
        if let Some(existing) = read_attribution(&tx, req.voucher_number)? {
            if existing.effective_year == req.effective_year {
                tx.commit()?;
                return Ok(AttributionResult::Replayed(existing));
            }
            let err = LedgerError::IdempotencyKeyReuse(format!(
                "voucher {} is already attributed to statement year {}, cannot re-attribute to {}",
                req.voucher_number, existing.effective_year, req.effective_year
            ));
            return Err(refuse_attribution(tx, err, req)?);
        }

        // No fiscal year in the affected interval [effective_year, posting_year] may be
        // Sealed. Attributing a voucher across that span changes: the effective year's
        // current activity, every intervening year's opening balances, and the posting
        // year's current activity — so a Sealed year anywhere in it would have its bound
        // statements_hash silently diverge. Checked in-lock, so a concurrent SealYear on
        // any of those years either committed first (we see Sealed and refuse) or blocks
        // behind us. (A year *after* posting_year folds this voucher into its opening
        // identically whether it is classed to E or P, so it is unaffected and excluded.)
        if let Some(sealed_year) = first_sealed_year_in(&tx, req.effective_year, req.posting_year)?
        {
            let err = LedgerError::PeriodTransitionConflict(format!(
                "cannot attribute voucher {} to statement year {}: fiscal year {sealed_year} in the \
                 affected interval [{}, {}] is Sealed (its statements are bound)",
                req.voucher_number, req.effective_year, req.effective_year, req.posting_year
            ));
            return Err(refuse_attribution(tx, err, req)?);
        }

        // Logical chain height = count+1 (contiguous 1,2,3,…), bound into the hash and
        // cross-checked by verify — an append-only, gapless sequence like the period
        // transition chains, not the opaque AUTOINCREMENT rowid.
        let seq: i64 = tx.query_row("SELECT COUNT(*) FROM adjustment_attribution", [], |r| {
            r.get::<_, i64>(0)
        })? + 1;
        let prev = latest_attribution_hash(&tx)?;
        let ts = Utc::now().to_rfc3339();
        let hash = attribution_hash(
            &prev,
            seq as u64,
            req.voucher_number,
            req.effective_year,
            &req.run_id,
            &req.actor,
            &ts,
        );
        // The FK on voucher_number turns "attribute a number that was never sealed"
        // into a constraint violation → a loud STORE error, not a dangling row.
        tx.execute(
            "INSERT INTO adjustment_attribution(seq, voucher_number, effective_year, prev_hash,
                                                owning_run_id, actor, ts, hash)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                seq,
                req.voucher_number,
                req.effective_year,
                prev.as_bytes().as_slice(),
                req.run_id,
                req.actor,
                ts,
                hash.as_bytes().as_slice(),
            ],
        )?;
        tx.commit()?;
        Ok(AttributionResult::Recorded(StoredAttribution {
            seq,
            voucher_number: req.voucher_number,
            effective_year: req.effective_year,
            prev_hash: prev,
            run_id: req.run_id.clone(),
            actor: req.actor.clone(),
            ts,
            hash,
        }))
    }

    fn verification_snapshot(&self) -> Result<FiscalSnapshot, LedgerError> {
        let mut guard = self.conn.lock().unwrap();
        // One deferred read transaction → all reads share a single WAL snapshot,
        // so a concurrent seal cannot make vouchers and head disagree.
        let tx = guard.transaction_with_behavior(TransactionBehavior::Deferred)?;

        let vouchers = {
            let mut stmt = tx.prepare(
                "SELECT number, prev_hash, hash, canonical_bytes, chart_version, sealed_at,
                        actor, natural_key, request_hash, source_fingerprint, policy_set, software_version, sent_hash
                 FROM vouchers ORDER BY number ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                let number: u64 = r.get(0)?;
                let prev: Vec<u8> = r.get(1)?;
                let hash: Vec<u8> = r.get(2)?;
                Ok((
                    number,
                    prev,
                    hash,
                    r.get::<_, Vec<u8>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, String>(10)?,
                    r.get::<_, String>(11)?,
                    r.get::<_, Option<String>>(12)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (
                    number,
                    prev,
                    hash,
                    canonical,
                    chart_version,
                    sealed_at,
                    actor,
                    natural_key,
                    request_hash,
                    source_fingerprint,
                    policy_set,
                    software_version,
                    sent_hash,
                ) = row?;
                out.push(StoredVoucher {
                    number,
                    prev_hash: hash_from_blob(&prev)?,
                    hash: hash_from_blob(&hash)?,
                    canonical,
                    chart_version: chart_version.clone(),
                    sealed_at: sealed_at.clone(),
                    provenance: StoredProvenance {
                        actor,
                        natural_key,
                        request_hash,
                        source_fingerprint,
                        policy_set,
                        software_version,
                        chart_version,
                        sealed_at,
                        sent_hash,
                    },
                });
            }
            out
        };

        let (head_blob, head_num): (Vec<u8>, u64) = tx.query_row(
            "SELECT head_hash, head_num FROM chain_head WHERE series = ?1",
            params![SERIES],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        let rejections = {
            let mut stmt = tx.prepare(
                "SELECT seq, prev_reject_hash, reason_code, request_hash, actor, target, note, ts, hash
                 FROM rejections ORDER BY seq ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, Vec<u8>>(8)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (seq, prev, reason_code, request_hash, actor, target, note, ts, hash) = row?;
                out.push(StoredRejection {
                    seq,
                    prev_reject_hash: hash_from_blob(&prev)?,
                    reason_code,
                    request_hash,
                    actor,
                    target,
                    note,
                    ts,
                    hash: hash_from_blob(&hash)?,
                });
            }
            out
        };

        let periods = {
            let mut stmt = tx.prepare(
                "SELECT period_id, state, owning_run_id, vat_filed, head_seq, head_hash
                 FROM period ORDER BY period_id ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Vec<u8>>(5)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (period_id, state, owner, filed, seq, head) = row?;
                let state = PeriodState::parse(&state)
                    .ok_or_else(|| LedgerError::Store(format!("unknown period state {state:?}")))?;
                out.push(PeriodRow {
                    period_id,
                    state,
                    owning_run_id: owner,
                    vat_filed: filed != 0,
                    head_seq: seq as u64,
                    head_hash: hash_from_blob(&head)?,
                });
            }
            out
        };

        let period_transitions = {
            let mut stmt = tx.prepare(
                "SELECT period_id, period_seq, prev_hash, from_state, to_state,
                        owning_run_id, sent_hash, receipt_unique_identifier,
                        receipt_accepted_timestamp, manual_channel, manual_evidence_ref,
                        remote_status_evidence_ref, actor, ts, hash
                 FROM period_transition ORDER BY seq ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, Option<String>>(10)?,
                    r.get::<_, Option<String>>(11)?,
                    r.get::<_, String>(12)?,
                    r.get::<_, String>(13)?,
                    r.get::<_, Vec<u8>>(14)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (
                    period_id,
                    period_seq,
                    prev,
                    from_state,
                    to_state,
                    owner,
                    sent_hash,
                    receipt_unique_identifier,
                    receipt_accepted_timestamp,
                    manual_channel,
                    manual_evidence_ref,
                    remote_status_evidence_ref,
                    actor,
                    ts,
                    hash,
                ) = row?;
                out.push(StoredPeriodTransition {
                    period_id,
                    period_seq: period_seq as u64,
                    prev_hash: hash_from_blob(&prev)?,
                    from_state,
                    to_state,
                    owning_run_id: owner,
                    sent_hash,
                    receipt_unique_identifier,
                    receipt_accepted_timestamp,
                    manual_channel,
                    manual_evidence_ref,
                    remote_status_evidence_ref,
                    actor,
                    ts,
                    hash: hash_from_blob(&hash)?,
                });
            }
            out
        };

        let fiscal_years = {
            let mut stmt = tx.prepare(
                "SELECT year, state, statements_hash, owning_run_id, head_seq, head_hash
                 FROM fiscal_year ORDER BY year ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Vec<u8>>(5)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (year, state, statements_hash, owner, seq, head) = row?;
                let state = FiscalYearState::parse(&state).ok_or_else(|| {
                    LedgerError::Store(format!("unknown fiscal-year state {state:?}"))
                })?;
                out.push(FiscalYearRow {
                    year: year as i32,
                    state,
                    statements_hash,
                    owning_run_id: owner,
                    head_seq: seq as u64,
                    head_hash: hash_from_blob(&head)?,
                });
            }
            out
        };

        let fiscal_year_transitions = {
            let mut stmt = tx.prepare(
                "SELECT year, year_seq, prev_hash, from_state, to_state, statements_hash,
                        owning_run_id, actor, ts, hash
                 FROM fiscal_year_transition ORDER BY seq ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, Vec<u8>>(9)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (year, year_seq, prev, from_state, to_state, sh, owner, actor, ts, hash) = row?;
                out.push(StoredFiscalYearTransition {
                    year: year as i32,
                    year_seq: year_seq as u64,
                    prev_hash: hash_from_blob(&prev)?,
                    from_state,
                    to_state,
                    statements_hash: sh,
                    owning_run_id: owner,
                    actor,
                    ts,
                    hash: hash_from_blob(&hash)?,
                });
            }
            out
        };

        let attributions = {
            let mut stmt = tx.prepare(
                "SELECT seq, voucher_number, effective_year, prev_hash, owning_run_id, actor, ts, hash
                 FROM adjustment_attribution ORDER BY seq ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, u64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, Vec<u8>>(7)?,
                ))
            })?;
            let mut out = Vec::new();
            for row in rows {
                let (seq, voucher_number, effective_year, prev, run_id, actor, ts, hash) = row?;
                out.push(StoredAttribution {
                    seq,
                    voucher_number,
                    effective_year: effective_year as i32,
                    prev_hash: hash_from_blob(&prev)?,
                    run_id,
                    actor,
                    ts,
                    hash: hash_from_blob(&hash)?,
                });
            }
            out
        };

        let calendar = load_calendar(&tx)?;

        Ok(FiscalSnapshot {
            vouchers,
            calendar,
            head_number: head_num,
            head_hash: hash_from_blob(&head_blob)?,
            rejections,
            periods,
            period_transitions,
            fiscal_years,
            fiscal_year_transitions,
            attributions,
        })
    }

    fn rejection_count(&self) -> Result<u64, LedgerError> {
        let guard = self.conn.lock().unwrap();
        Ok(guard.query_row("SELECT COUNT(*) FROM rejections", [], |r| r.get(0))?)
    }
}

/// Read the authoritative `period` row, synthesizing the implicit `Open` state
/// for a period that has never transitioned (ADR-6 §1).
fn read_period_row(conn: &Connection, period_id: &str) -> Result<PeriodRow, LedgerError> {
    Ok(read_period_row_opt(conn, period_id)?.unwrap_or_else(|| implicit_open(period_id)))
}

fn read_all_period_rows(conn: &Connection) -> Result<Vec<PeriodRow>, LedgerError> {
    let mut stmt = conn.prepare(
        "SELECT period_id, state, owning_run_id, vat_filed, head_seq, head_hash \
         FROM period ORDER BY period_id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, Vec<u8>>(5)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (period_id, state, owner, filed, seq, head) = row?;
        let state = PeriodState::parse(&state)
            .ok_or_else(|| LedgerError::Store(format!("unknown period state {state:?}")))?;
        out.push(PeriodRow {
            period_id,
            state,
            owning_run_id: owner,
            vat_filed: filed != 0,
            head_seq: seq as u64,
            head_hash: hash_from_blob(&head)?,
        });
    }
    Ok(out)
}

/// Like [`read_period_row`] but distinguishes "no physical row" (`None`) from a
/// materialized row — the transition path needs that to choose INSERT vs. CAS.
fn read_period_row_opt(
    conn: &Connection,
    period_id: &str,
) -> Result<Option<PeriodRow>, LedgerError> {
    let raw = conn
        .query_row(
            "SELECT state, owning_run_id, vat_filed, head_seq, head_hash
             FROM period WHERE period_id = ?1",
            params![period_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                ))
            },
        )
        .optional()?;
    match raw {
        None => Ok(None),
        Some((state, owner, filed, seq, head)) => {
            let state = PeriodState::parse(&state)
                .ok_or_else(|| LedgerError::Store(format!("unknown period state {state:?}")))?;
            Ok(Some(PeriodRow {
                period_id: period_id.to_string(),
                state,
                owning_run_id: owner,
                vat_filed: filed != 0,
                head_seq: seq as u64,
                head_hash: hash_from_blob(&head)?,
            }))
        }
    }
}

/// Read the authoritative `fiscal_year` row, synthesizing the implicit `Open` state
/// for a year that has never sealed.
fn read_fiscal_year_row(conn: &Connection, year: i32) -> Result<FiscalYearRow, LedgerError> {
    Ok(read_fiscal_year_row_opt(conn, year)?.unwrap_or_else(|| implicit_open_year(year)))
}

/// Like [`read_fiscal_year_row`] but distinguishes "no physical row" (`None`), which
/// the transition path needs to choose INSERT vs. CAS.
fn read_fiscal_year_row_opt(
    conn: &Connection,
    year: i32,
) -> Result<Option<FiscalYearRow>, LedgerError> {
    let raw = conn
        .query_row(
            "SELECT state, statements_hash, owning_run_id, head_seq, head_hash
             FROM fiscal_year WHERE year = ?1",
            params![year],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                ))
            },
        )
        .optional()?;
    match raw {
        None => Ok(None),
        Some((state, sh, owner, seq, head)) => {
            let state = FiscalYearState::parse(&state).ok_or_else(|| {
                LedgerError::Store(format!("unknown fiscal-year state {state:?}"))
            })?;
            Ok(Some(FiscalYearRow {
                year,
                state,
                statements_hash: sh,
                owning_run_id: owner,
                head_seq: seq as u64,
                head_hash: hash_from_blob(&head)?,
            }))
        }
    }
}

/// Read a voucher's current attribution row (issue `closeyear-adjustment-attribution`),
/// or `None` if the voucher has never been attributed.
fn read_attribution(
    conn: &Connection,
    voucher_number: u64,
) -> Result<Option<StoredAttribution>, LedgerError> {
    let raw = conn
        .query_row(
            "SELECT seq, effective_year, prev_hash, owning_run_id, actor, ts, hash
             FROM adjustment_attribution WHERE voucher_number = ?1",
            params![voucher_number],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Vec<u8>>(6)?,
                ))
            },
        )
        .optional()?;
    match raw {
        None => Ok(None),
        Some((seq, effective_year, prev, run_id, actor, ts, hash)) => Ok(Some(StoredAttribution {
            seq,
            voucher_number,
            effective_year: effective_year as i32,
            prev_hash: hash_from_blob(&prev)?,
            run_id,
            actor,
            ts,
            hash: hash_from_blob(&hash)?,
        })),
    }
}

/// The first `Sealed` fiscal year in the inclusive interval `[lo, hi]`, or `None`
/// (issue `closeyear-adjustment-attribution`). Drives the cascade seal-guard: an
/// attribution across `[effective_year, posting_year]` may not straddle any sealed
/// year. Assumes `lo <= hi` (the Ledger enforces `effective_year <= posting_year`).
fn first_sealed_year_in(conn: &Connection, lo: i32, hi: i32) -> Result<Option<i32>, LedgerError> {
    let y: Option<i64> = conn
        .query_row(
            "SELECT year FROM fiscal_year
             WHERE year BETWEEN ?1 AND ?2 AND state = 'Sealed'
             ORDER BY year ASC LIMIT 1",
            params![lo, hi],
            |r| r.get(0),
        )
        .optional()?;
    Ok(y.map(|v| v as i32))
}

/// Record an attribution refusal in the append-only rejection log (bound to
/// `req.request_hash` so duplicate refusals are distinguishable), commit, and return
/// the original error — the attribution analogue of the period-transition failure
/// path. Consumes the transaction (the refusal is its terminal write).
fn refuse_attribution(
    tx: rusqlite::Transaction<'_>,
    err: LedgerError,
    req: &AttributionRequest,
) -> Result<LedgerError, LedgerError> {
    insert_rejection(
        &tx,
        &RejectionRecord {
            reason_code: err.error_code().to_string(),
            request_hash: req.request_hash.clone(),
            actor: req.actor.clone(),
            target: format!("fiscal:attribution:{}", req.voucher_number),
            note: err.to_string(),
        },
    )?;
    tx.commit()?;
    Ok(err)
}

/// The head hash of the `adjustment_attribution` chain (`Hash::zero()` when empty).
fn latest_attribution_hash(conn: &Connection) -> Result<Hash, LedgerError> {
    let prev: Option<Vec<u8>> = conn
        .query_row(
            "SELECT hash FROM adjustment_attribution ORDER BY seq DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    match prev {
        Some(b) => hash_from_blob(&b),
        None => Ok(Hash::zero()),
    }
}

/// Parse the calendar year from a canonical month or quarter period id (for the S6
/// fiscal-year-sealed transition guard). `None` if the shape is unexpected.
/// The labels of every fiscal year a period overlaps (a calendar quarter can straddle
/// two non-calendar years). Days outside any fiscal year contribute nothing.
fn fiscal_years_of_period(
    calendar: &FiscalCalendar,
    period_id: &str,
) -> Result<Vec<i32>, LedgerError> {
    let period = PeriodId::parse(period_id)
        .map_err(|error| LedgerError::Store(format!("invalid period_id {period_id:?}: {error}")))?;
    let (first, last) = period.month_bounds();
    let mut years = Vec::new();
    for month in first..=last {
        let day = Date::from_ymd(period.year(), month.into(), 1)?;
        match calendar.year_of(day) {
            Ok(year) if !years.contains(&year.year()) => years.push(year.year()),
            Ok(_) | Err(ModelError::FiscalYearUndefined(_)) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(years)
}

/// The fiscal calendar from the append-only definitions. A row that does not form a
/// valid calendar fails closed: year attribution must never be guessed.
fn load_calendar(conn: &Connection) -> Result<FiscalCalendar, LedgerError> {
    let mut stmt = conn
        .prepare("SELECT start_date, end_date FROM fiscal_year_definition ORDER BY start_date")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut defined = Vec::new();
    for row in rows {
        let (start, end) = row?;
        let year = Date::parse(&start)
            .and_then(|start| FiscalYear::new(start, Date::parse(&end)?))
            .map_err(|error| {
                LedgerError::Store(format!(
                    "invalid fiscal_year_definition {start}..{end} in fiscal store: {error}"
                ))
            })?;
        defined.push(year);
    }
    FiscalCalendar::new(defined).map_err(|error| {
        LedgerError::Store(format!(
            "fiscal_year_definition rows do not form a calendar: {error}"
        ))
    })
}

fn implicit_open_year(year: i32) -> FiscalYearRow {
    FiscalYearRow {
        year,
        state: FiscalYearState::Open,
        statements_hash: None,
        owning_run_id: None,
        head_seq: 0,
        head_hash: Hash::zero(),
    }
}

fn implicit_open(period_id: &str) -> PeriodRow {
    PeriodRow {
        period_id: period_id.to_string(),
        state: PeriodState::Open,
        owning_run_id: None,
        vat_filed: false,
        head_seq: 0,
        head_hash: Hash::zero(),
    }
}

/// The F5 per-voucher period check (ADR-6 §2), evaluated inside the seal txn.
/// `None` = the write may proceed; `Some(err)` = a refusal to be logged.
fn evaluate_period_gate(prow: &PeriodRow, gate: &PeriodGate<'_>) -> Option<LedgerError> {
    match prow.state {
        PeriodState::Open => None,
        PeriodState::Closing => match gate.authority {
            Some(a)
                if prow.owning_run_id.as_deref() == Some(a.run_id())
                    && a.period().to_string() == prow.period_id
                    && a.period().contains(gate.posting_date) =>
            {
                None
            }
            Some(_) => Some(LedgerError::VoucherNotInRun(format!(
                "period {} is Closing; presented ClosingAuthority does not own it",
                prow.period_id
            ))),
            None => Some(LedgerError::PeriodClosing(format!(
                "period {} is Closing; a bare voucher seal is refused (no ClosingAuthority)",
                prow.period_id
            ))),
        },
        PeriodState::Sealed => Some(LedgerError::PeriodSealed(format!(
            "period {} is Sealed; corrections go to the next open period",
            prow.period_id
        ))),
    }
}

/// Append one entry to the hash-chained rejection log using the caller's
/// transaction (so a period-transition refusal is logged atomically with the
/// COMMIT that returns Refused — ADR-6 §2). Does not commit.
fn insert_rejection(conn: &Connection, rej: &RejectionRecord) -> Result<(), LedgerError> {
    let prev: Option<Vec<u8>> = conn
        .query_row(
            "SELECT hash FROM rejections ORDER BY seq DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let prev = match prev {
        Some(b) => hash_from_blob(&b)?,
        None => Hash::zero(),
    };
    let ts = Utc::now().to_rfc3339();
    let hash = rejection_hash(&prev, rej, &ts);
    conn.execute(
        "INSERT INTO rejections(prev_reject_hash, reason_code, request_hash, actor, target, note, ts, hash)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            prev.as_bytes().as_slice(),
            rej.reason_code,
            rej.request_hash,
            rej.actor,
            rej.target,
            rej.note,
            ts,
            hash.as_bytes().as_slice(),
        ],
    )?;
    Ok(())
}

/// The `period_transition` chain hash (ADR-6 §1). Length-prefixed fields; kept
/// identical here and in `verify` so the per-period chain is recomputable and
/// `fold(period_transition) == period` is checkable.
#[allow(clippy::too_many_arguments)]
pub(crate) fn period_transition_hash(
    prev: &Hash,
    period_id: &str,
    period_seq: u64,
    from_state: &str,
    to_state: &str,
    owning_run_id: Option<&str>,
    sent_hash: Option<&str>,
    receipt_unique_identifier: Option<&str>,
    receipt_accepted_timestamp: Option<&str>,
    manual_channel: Option<&str>,
    manual_evidence_ref: Option<&str>,
    remote_status_evidence_ref: Option<&str>,
    actor: &str,
    ts: &str,
) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(prev.as_bytes());
    hasher.update(period_seq.to_be_bytes());
    // Leading discriminator bytes distinguish "no value" from an empty one for each
    // optional field, so `None` and `Some("")` can never collide.
    hasher.update([owning_run_id.is_some() as u8, sent_hash.is_some() as u8]);
    for field in [
        period_id,
        from_state,
        to_state,
        owning_run_id.unwrap_or(""),
        sent_hash.unwrap_or(""),
        actor,
        ts,
    ] {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    // Preserve the landed hash exactly for legacy/non-receipt events. New complete
    // receipt pairs extend the preimage with explicit presence bits and values.
    if receipt_unique_identifier.is_some() || receipt_accepted_timestamp.is_some() {
        hasher.update([
            receipt_unique_identifier.is_some() as u8,
            receipt_accepted_timestamp.is_some() as u8,
        ]);
        for field in [
            receipt_unique_identifier.unwrap_or(""),
            receipt_accepted_timestamp.unwrap_or(""),
        ] {
            hasher.update((field.len() as u64).to_be_bytes());
            hasher.update(field.as_bytes());
        }
    }
    // Preserve old hashes unless this is a new evidence-bearing manual filing.
    if manual_channel.is_some()
        || manual_evidence_ref.is_some()
        || remote_status_evidence_ref.is_some()
    {
        hasher.update([
            manual_channel.is_some() as u8,
            manual_evidence_ref.is_some() as u8,
            remote_status_evidence_ref.is_some() as u8,
        ]);
        for field in [
            manual_channel.unwrap_or(""),
            manual_evidence_ref.unwrap_or(""),
            remote_status_evidence_ref.unwrap_or(""),
        ] {
            hasher.update((field.len() as u64).to_be_bytes());
            hasher.update(field.as_bytes());
        }
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    Hash::from_bytes(out)
}

/// The `fiscal_year_transition` chain hash (S6). Length-prefixed fields; kept
/// identical here and in `verify` so the per-year chain is recomputable and
/// `fold(fiscal_year_transition) == fiscal_year` is checkable.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fiscal_year_transition_hash(
    prev: &Hash,
    year: i32,
    year_seq: u64,
    from_state: &str,
    to_state: &str,
    statements_hash: Option<&str>,
    owning_run_id: Option<&str>,
    actor: &str,
    ts: &str,
) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(prev.as_bytes());
    hasher.update(year.to_be_bytes());
    hasher.update(year_seq.to_be_bytes());
    // Leading discriminator bytes so `None` and `Some("")` cannot collide.
    hasher.update([
        statements_hash.is_some() as u8,
        owning_run_id.is_some() as u8,
    ]);
    for field in [
        from_state,
        to_state,
        statements_hash.unwrap_or(""),
        owning_run_id.unwrap_or(""),
        actor,
        ts,
    ] {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    Hash::from_bytes(out)
}

/// The `adjustment_attribution` chain hash (issue `closeyear-adjustment-attribution`).
/// Length-prefixed string fields + fixed-width integers; kept identical here and in
/// `verify` so the chain is recomputable and `fold(adjustment_attribution)` is
/// checkable. `seq` (the logical chain height) is bound in so a gap or reorder breaks
/// the chain, mirroring `period_transition.period_seq`. `effective_year` is a signed
/// 32-bit big-endian value so a future negative/BCE year cannot alias a positive one.
pub(crate) fn attribution_hash(
    prev: &Hash,
    seq: u64,
    voucher_number: u64,
    effective_year: i32,
    run_id: &str,
    actor: &str,
    ts: &str,
) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(prev.as_bytes());
    hasher.update(seq.to_be_bytes());
    hasher.update(voucher_number.to_be_bytes());
    hasher.update(effective_year.to_be_bytes());
    for field in [run_id, actor, ts] {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    Hash::from_bytes(out)
}

/// The rejection-log chain hash. Length-prefixed fields prevent ambiguity;
/// kept identical here and in `verify` so the chain is recomputable.
pub(crate) fn rejection_hash(prev: &Hash, rej: &RejectionRecord, ts: &str) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(prev.as_bytes());
    for field in [
        &rej.reason_code,
        &rej.request_hash,
        &rej.actor,
        &rej.target,
        &rej.note,
        &ts.to_string(),
    ] {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field.as_bytes());
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    Hash::from_bytes(out)
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    fn user_version(conn: &Connection) -> i32 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    /// Opening a fresh fiscal DB runs the migrations and stamps it to the latest
    /// version (v6 today), and the seed rows are present.
    #[test]
    fn fresh_fiscal_store_is_stamped_and_seeded() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteFiscalStore::open(&dir.path().join("fiscal.sqlite")).unwrap();
        let conn = store.conn.lock().unwrap();
        assert_eq!(user_version(&conn), 7);
        let next: i64 = conn
            .query_row(
                "SELECT next_number FROM counter WHERE series='main'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(next, 1);
        let head_num: i64 = conn
            .query_row(
                "SELECT head_num FROM chain_head WHERE series='main'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(head_num, 0);
    }

    /// A home that predates the migration framework (tables present, `user_version`
    /// still 0) is *refused* on open — not silently stamped to v1 while missing
    /// columns added inline in later slices. The operator resets it (data is
    /// resettable).
    #[test]
    fn a_pre_framework_home_is_refused_with_a_reset_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fiscal.sqlite");

        // Build the DB the "old way": tables present but user_version left at 0.
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(MIGRATIONS[0].sql).unwrap();
            conn.pragma_update(None, "user_version", 0).unwrap();
            assert_eq!(user_version(&conn), 0);
        }

        let err = match SqliteFiscalStore::open(&path) {
            Ok(_) => panic!("a pre-framework home must be refused, not opened"),
            Err(e) => e,
        };
        assert_eq!(err.error_code(), "STORE_ERROR");
        assert!(err.to_string().contains("predates"), "{err}");
    }

    /// Re-opening an already-migrated fiscal DB is a no-op (no error, version
    /// unchanged) — the common steady-state path.
    #[test]
    fn reopening_a_fiscal_store_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fiscal.sqlite");
        drop(SqliteFiscalStore::open(&path).unwrap());
        let store = SqliteFiscalStore::open(&path).unwrap();
        assert_eq!(user_version(&store.conn.lock().unwrap()), 7);
    }

    #[test]
    fn v3_marks_pre_version_stamp_vouchers_as_unknown_legacy() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply(&mut conn, "fiscal", &MIGRATIONS[..2]).unwrap();
        conn.execute(
            "INSERT INTO vouchers(number, series, prev_hash, hash, canonical_bytes,
                                  chart_version, sealed_at, actor, natural_key, request_hash,
                                  policy_set, sent_hash)
             VALUES(1, 'main', ?1, ?2, X'7b7d', 'chart', '2026-01-01T00:00:00Z',
                    'actor', 'key', 'request', 'policy', NULL)",
            params![vec![0u8; 32], vec![1u8; 32]],
        )
        .unwrap();

        migrations::apply(&mut conn, "fiscal", MIGRATIONS).unwrap();

        assert_eq!(user_version(&conn), 7);
        let version: String = conn
            .query_row(
                "SELECT software_version FROM vouchers WHERE number = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(version, "unknown-legacy");
        let source_fingerprint: Option<String> = conn
            .query_row(
                "SELECT source_fingerprint FROM vouchers WHERE number = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(source_fingerprint, None);
    }

    /// Runtime foreign-key enforcement is ON after open (set after migrations),
    /// even though migrations themselves run with it off.
    #[test]
    fn foreign_keys_are_enforced_after_open() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteFiscalStore::open(&dir.path().join("fiscal.sqlite")).unwrap();
        let conn = store.conn.lock().unwrap();
        let fk_on: bool = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert!(fk_on, "foreign_keys must be ON for runtime writes");
    }

    /// The v1 seed hardcodes the series name `'main'`; guard the coupling to the
    /// `SERIES` constant used by the write path so the two can't silently diverge.
    #[test]
    fn series_constant_matches_the_v1_seed_literal() {
        assert_eq!(SERIES, "main");
    }
}
