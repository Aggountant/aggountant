//! SQLite **PII masterdata** store (ADR-1 §Masterdata, F7) — the erasable boundary
//! for personal data (employee identity, HETU, IBAN, tax cards, YTD) that must not
//! land in git. Each record is stored as a single blob through the at-rest-encryption
//! [`Cipher`] **seam**.
//!
//! **S4 placeholder: the blob is PLAINTEXT.** The cipher is [`IdentityCipher`], the
//! same honest placeholder the artifact/masterdata stores use — real envelope
//! encryption (installation KEK, versioned DEKs, AEAD) lands in a later slice and
//! changes only the `Cipher` impl, not this store. The design intent is that once the
//! real cipher is in place a GDPR erasure is a coherent `DELETE` of one row +
//! crypto-shred of its DEK; today it is a plaintext `DELETE`. Do NOT put real PII here
//! until the cipher is real (see `spike-notes.md`).
//!
//! This is a **distinct port** from the git-native reference masterdata: it never
//! serves the chart, and the reference store never serves PII. It lives at
//! `masterdata/pii.sqlite`, which the home `.gitignore` excludes.
//!
//! YTD is a **derived, year-scoped** total, not an embedded scalar. Each applied pay
//! run appends one idempotent row to `ytd_earnings`, keyed
//! `(employee_id, tax_year, natural_key)`; a year's YTD is the sum of its rows. A row
//! is inserted inside a `BEGIN IMMEDIATE` writer lock so a pay run's `accumulate_ytd`
//! is atomic against a concurrent update, and the identity blob (`employee`) and the
//! earnings ledger are distinct tables — an `employee add` upsert rewrites the blob
//! but never the earnings, so it can no longer reset the accumulator.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use rust_decimal::Decimal;

use aggountant_model::{Currency, Money};

use aggountant_domain::workplace::{ClaimantFact, FactChange, FactEntry, NewFact, Place};
use aggountant_model::{Hash, Instant};

use crate::employee::EmployeeState;
use crate::error::LedgerError;
use crate::store::cipher::{Cipher, IdentityCipher};
use crate::store::migrations::{self, Migration};
use crate::store::{ClaimantRows, ClaimantStore, EmployeeStore};

/// The `pii` masterdata DB schema, as an ordered `PRAGMA user_version` migration
/// list (see [`super::migrations`]). v1 is the baseline; a fresh home initializes
/// to it, while a home predating the framework is refused (not adopted) by
/// [`migrations::apply`]. Append later changes as new `Migration`s — never edit a
/// landed one.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: "CREATE TABLE IF NOT EXISTS employee(
             employee_id TEXT PRIMARY KEY,
             -- at-rest-encrypted serialized EmployeeState (identity + PII)
             sealed BLOB NOT NULL
          );
          -- Year-scoped, idempotent earnings ledger: one row per applied pay run,
          -- keyed by (employee_id, tax_year, natural_key). YTD for a year is the sum
          -- of that year's rows — so a later tax year never adds onto the prior year,
          -- and the PK makes a crash/retry (same natural_key) a no-op insert (no
          -- double- or under-count). The gross is stored as its decimal string +
          -- currency (the same currency the pay run advanced with).
          CREATE TABLE IF NOT EXISTS ytd_earnings(
             employee_id    TEXT    NOT NULL,
             tax_year       INTEGER NOT NULL,
             natural_key    TEXT    NOT NULL,
             gross_amount   TEXT    NOT NULL,
             gross_currency TEXT    NOT NULL,
             PRIMARY KEY (employee_id, tax_year, natural_key),
             -- Earnings are part of the employee's erasable PII record: a future
             -- GDPR erasure that DELETEs the employee row cascades them away too,
             -- leaving no orphaned salary history keyed to the (pseudonymous) id.
             FOREIGN KEY (employee_id) REFERENCES employee(employee_id)
                 ON DELETE CASCADE
          );",
    },
    // v2: the claimant register (ADR-11 sections 1 and 3). Additive: two empty
    // tables, nothing existing changes. Both are append-only (an UPDATE aborts);
    // DELETE stays possible only through the employee's erasure cascade.
    // Codes (place kind, role, basis, relationship) are validated by the program,
    // not by CHECK, so that adding a code later needs no table rebuild.
    Migration {
        version: 2,
        sql: "CREATE TABLE claimant_place(
             employee_id  TEXT NOT NULL
                          REFERENCES employee(employee_id) ON DELETE CASCADE,
             place_id     TEXT NOT NULL,
             kind         TEXT NOT NULL,
             label        TEXT NOT NULL,
             location     TEXT NOT NULL,
             municipality TEXT NOT NULL,
             -- the place this row corrects; a place is replaced at most once
             replaces     TEXT,
             recorded_at  TEXT NOT NULL,
             confirmed_by TEXT NOT NULL,
             PRIMARY KEY (employee_id, place_id),
             UNIQUE (employee_id, replaces),
             FOREIGN KEY (employee_id, replaces)
                 REFERENCES claimant_place(employee_id, place_id)
          );
          -- One row per assertion or retraction; a correction supersedes exactly
          -- one earlier row (UNIQUE), so the history never forks.
          CREATE TABLE claimant_fact(
             fact_id      INTEGER PRIMARY KEY AUTOINCREMENT,
             employee_id  TEXT NOT NULL
                          REFERENCES employee(employee_id) ON DELETE CASCADE,
             kind         TEXT NOT NULL
                          CHECK (kind IN ('workplace', 'relationship', 'part_owner')),
             action       TEXT NOT NULL CHECK (action IN ('assert', 'retract')),
             -- the asserted fact as JSON ({valid_from, valid_to, fact}); NULL
             -- for a retraction
             entry        TEXT,
             recorded_at  TEXT NOT NULL,
             confirmed_by TEXT NOT NULL,
             evidence_hash TEXT,
             supersedes   INTEGER UNIQUE REFERENCES claimant_fact(fact_id),
             reason       TEXT,
             CHECK ((action = 'assert') = (entry IS NOT NULL)),
             CHECK (action = 'assert' OR supersedes IS NOT NULL),
             CHECK ((supersedes IS NULL) = (reason IS NULL))
          );
          CREATE INDEX claimant_fact_by_employee ON claimant_fact(employee_id);
          CREATE TRIGGER claimant_place_append_only BEFORE UPDATE ON claimant_place
          BEGIN SELECT RAISE(ABORT, 'claimant_place is append-only'); END;
          CREATE TRIGGER claimant_fact_append_only BEFORE UPDATE ON claimant_fact
          BEGIN SELECT RAISE(ABORT, 'claimant_fact is append-only'); END;",
    },
];

pub(crate) struct SqlitePiiStore {
    // `Mutex` so a single `Ledger` stays `Send + Sync` (mirrors the fiscal store).
    pub(crate) conn: Mutex<Connection>,
    cipher: IdentityCipher,
}

impl SqlitePiiStore {
    /// Open (creating if absent) the encrypted PII store. Unlike the fiscal DB, this
    /// is created lazily on first `open` of a home — employees are added after
    /// `init`, via `employee add`.
    pub(crate) fn open(path: &Path) -> Result<Self, LedgerError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| LedgerError::Store(e.to_string()))?;
        }
        let mut conn = Connection::open(path)?;
        // Install the busy handler FIRST, before any lock-contending pragma (WAL
        // setup can contend), so concurrent opens wait rather than fail with an
        // immediate SQLITE_BUSY.
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        // `foreign_keys` stays OFF (the default) across `apply`: FK enforcement must
        // be off for the SQLite table-rebuild a future migration may need, and the
        // pragma is a no-op inside `apply`'s transaction. Enabled below for runtime.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;",
        )?;
        migrations::apply(&mut conn, "pii", MIGRATIONS)?;
        // Turn FK enforcement on so the earnings→employee reference (and its
        // erasure cascade) is honoured at runtime. Set outside a transaction —
        // `apply` has committed and returned.
        conn.pragma_update(None, "foreign_keys", true)?;
        Ok(SqlitePiiStore {
            conn: Mutex::new(conn),
            cipher: IdentityCipher,
        })
    }

    fn encode(&self, state: &EmployeeState) -> Result<Vec<u8>, LedgerError> {
        let plaintext = serde_json::to_vec(state)
            .map_err(|e| LedgerError::Store(format!("employee encode: {e}")))?;
        Ok(self.cipher.seal(&plaintext))
    }

    fn decode(&self, sealed: &[u8]) -> Result<EmployeeState, LedgerError> {
        let plaintext = self.cipher.open(sealed);
        serde_json::from_slice(&plaintext)
            .map_err(|e| LedgerError::Store(format!("employee decode (PII store corrupt): {e}")))
    }
}

impl EmployeeStore for SqlitePiiStore {
    fn upsert(&self, state: &EmployeeState) -> Result<(), LedgerError> {
        let sealed = self.encode(state)?;
        let conn = self.conn.lock().expect("pii store mutex");
        conn.execute(
            "INSERT INTO employee(employee_id, sealed) VALUES(?1, ?2)
             ON CONFLICT(employee_id) DO UPDATE SET sealed = excluded.sealed",
            params![state.employee_id, sealed],
        )?;
        Ok(())
    }

    fn get(&self, employee_id: &str) -> Result<Option<EmployeeState>, LedgerError> {
        let conn = self.conn.lock().expect("pii store mutex");
        let sealed: Option<Vec<u8>> = conn
            .query_row(
                "SELECT sealed FROM employee WHERE employee_id = ?1",
                params![employee_id],
                |r| r.get(0),
            )
            .optional()?;
        match sealed {
            Some(bytes) => Ok(Some(self.decode(&bytes)?)),
            None => Ok(None),
        }
    }

    fn list_ids(&self) -> Result<Vec<String>, LedgerError> {
        let conn = self.conn.lock().expect("pii store mutex");
        let mut stmt = conn.prepare("SELECT employee_id FROM employee ORDER BY employee_id")?;
        let ids = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids)
    }

    fn accumulate_ytd(
        &self,
        employee_id: &str,
        tax_year: i32,
        natural_key: &str,
        add: &Money,
    ) -> Result<Money, LedgerError> {
        let mut conn = self.conn.lock().expect("pii store mutex");
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // The employee must exist — accumulating YTD for an unknown id is a bug, not a
        // silent no-op. (Same durable-master-data contract as `get`.)
        let exists: bool = tx
            .query_row(
                "SELECT 1 FROM employee WHERE employee_id = ?1",
                params![employee_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(LedgerError::EmployeeNotFound(format!(
                "no employee {employee_id} in the PII store"
            )));
        }
        // Currency guard: a year's rows must all share one currency (they are summed).
        // Compare against any existing row for this (employee, tax_year).
        if let Some(existing_currency) = tx
            .query_row(
                "SELECT gross_currency FROM ytd_earnings
                 WHERE employee_id = ?1 AND tax_year = ?2 LIMIT 1",
                params![employee_id, tax_year],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            if existing_currency != add.currency().as_str() {
                return Err(LedgerError::Store(format!(
                    "YTD accumulation currency mismatch for {employee_id} {tax_year}: {existing_currency} vs {}",
                    add.currency()
                )));
            }
        }
        // Idempotent append: `INSERT OR IGNORE` the earnings row inside the writer lock.
        // A repeat with the same `(employee_id, tax_year, natural_key)` touches 0 rows —
        // we fall through and return the current year total (no double-count); a crash
        // before COMMIT leaves no row at all (no under-count).
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO ytd_earnings
                 (employee_id, tax_year, natural_key, gross_amount, gross_currency)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                employee_id,
                tax_year,
                natural_key,
                add.amount_str(),
                add.currency().as_str()
            ],
        )?;
        // A no-op insert is only a legitimate replay if the stored row is byte-for-byte
        // the same request. A reused key carrying a *different* gross (a caller bug — a
        // recompute that changed the amount, an accidental key collision) must be a loud
        // conflict, not a silent replay of stale data — mirroring the fiscal seal's
        // `request_hash` binding (`IdempotencyConflict`).
        if inserted == 0 {
            let (stored_amount, stored_currency): (String, String) = tx.query_row(
                "SELECT gross_amount, gross_currency FROM ytd_earnings
                 WHERE employee_id = ?1 AND tax_year = ?2 AND natural_key = ?3",
                params![employee_id, tax_year, natural_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            if stored_amount != add.amount_str() || stored_currency != add.currency().as_str() {
                return Err(LedgerError::IdempotencyKeyReuse(format!(
                    "YTD key {natural_key} for {employee_id} {tax_year} was recorded as \
                     {stored_amount} {stored_currency}, now presented as {} {}",
                    add.amount_str(),
                    add.currency()
                )));
            }
        }
        let total = sum_year(&tx, employee_id, tax_year)?;
        tx.commit()?;
        Ok(total)
    }

    fn ytd_gross(&self, employee_id: &str, tax_year: i32) -> Result<Money, LedgerError> {
        let conn = self.conn.lock().expect("pii store mutex");
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM employee WHERE employee_id = ?1",
                params![employee_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(LedgerError::EmployeeNotFound(format!(
                "no employee {employee_id} in the PII store"
            )));
        }
        sum_year(&conn, employee_id, tax_year)
    }
}

fn employee_exists(conn: &Connection, employee_id: &str) -> Result<bool, LedgerError> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM employee WHERE employee_id = ?1",
            params![employee_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn corrupt(what: &str, detail: impl std::fmt::Display) -> LedgerError {
    LedgerError::Store(format!("claimant register row corrupt ({what}): {detail}"))
}

/// Read one person's register rows. Every stored code and date is parsed back
/// through the Domain's strict types, so a hand-edited or damaged row fails
/// loudly instead of being read as something else.
fn read_claimant_rows(conn: &Connection, employee_id: &str) -> Result<ClaimantRows, LedgerError> {
    if !employee_exists(conn, employee_id)? {
        return Err(LedgerError::EmployeeNotFound(format!(
            "no person {employee_id} in the personal-data register; add them with \
             `employee add` first"
        )));
    }
    let mut stmt = conn.prepare(
        "SELECT place_id, kind, label, location, municipality, replaces, recorded_at,
                confirmed_by
         FROM claimant_place WHERE employee_id = ?1 ORDER BY rowid",
    )?;
    let places = stmt
        .query_map(params![employee_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?
        .map(|row| {
            let (
                place_id,
                kind,
                label,
                location,
                municipality,
                replaces,
                recorded_at,
                confirmed_by,
            ) = row?;
            Ok(Place {
                employee_id: employee_id.to_string(),
                kind: serde_json::from_value(serde_json::Value::String(kind))
                    .map_err(|e| corrupt("place kind", e))?,
                place_id,
                label,
                location,
                municipality,
                replaces,
                recorded_at: Instant::parse(&recorded_at).map_err(|e| corrupt("recorded_at", e))?,
                confirmed_by,
            })
        })
        .collect::<Result<Vec<_>, LedgerError>>()?;

    let mut stmt = conn.prepare(
        "SELECT fact_id, action, entry, recorded_at, confirmed_by, evidence_hash,
                supersedes, reason
         FROM claimant_fact WHERE employee_id = ?1 ORDER BY fact_id",
    )?;
    let facts = stmt
        .query_map(params![employee_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<i64>>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })?
        .map(|row| {
            let (fact_id, action, entry, recorded_at, confirmed_by, evidence, supersedes, reason) =
                row?;
            let change = match (action.as_str(), entry) {
                ("assert", Some(json)) => FactChange::Assert(
                    serde_json::from_str::<FactEntry>(&json).map_err(|e| corrupt("entry", e))?,
                ),
                ("retract", None) => FactChange::Retract,
                (other, _) => return Err(corrupt("action", other)),
            };
            Ok(ClaimantFact {
                fact_id,
                employee_id: employee_id.to_string(),
                change,
                recorded_at: Instant::parse(&recorded_at).map_err(|e| corrupt("recorded_at", e))?,
                confirmed_by,
                evidence_hash: evidence
                    .map(|h| Hash::parse_hex(&h))
                    .transpose()
                    .map_err(|e| corrupt("evidence_hash", e))?,
                supersedes,
                reason,
            })
        })
        .collect::<Result<Vec<_>, LedgerError>>()?;
    Ok(ClaimantRows { places, facts })
}

impl ClaimantStore for SqlitePiiStore {
    fn claimant_rows(&self, employee_id: &str) -> Result<ClaimantRows, LedgerError> {
        let conn = self.conn.lock().expect("pii store mutex");
        read_claimant_rows(&conn, employee_id)
    }

    fn append_claimant_place(
        &self,
        place: &Place,
        validate: &dyn Fn(&ClaimantRows) -> Result<(), LedgerError>,
    ) -> Result<(), LedgerError> {
        let mut conn = self.conn.lock().expect("pii store mutex");
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        validate(&read_claimant_rows(&tx, &place.employee_id)?)?;
        let kind = serde_json::to_value(place.kind).map_err(|e| corrupt("place kind", e))?;
        tx.execute(
            "INSERT INTO claimant_place
                 (employee_id, place_id, kind, label, location, municipality, replaces,
                  recorded_at, confirmed_by)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                place.employee_id,
                place.place_id,
                kind.as_str(),
                place.label,
                place.location,
                place.municipality,
                place.replaces,
                place.recorded_at.to_string(),
                place.confirmed_by
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn append_claimant_fact(
        &self,
        fact: &NewFact,
        recorded_at: Instant,
        validate: &dyn Fn(&ClaimantRows) -> Result<(), LedgerError>,
    ) -> Result<ClaimantFact, LedgerError> {
        let mut conn = self.conn.lock().expect("pii store mutex");
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let rows = read_claimant_rows(&tx, &fact.employee_id)?;
        validate(&rows)?;
        let (action, entry, kind) = match &fact.change {
            FactChange::Assert(entry) => (
                "assert",
                Some(serde_json::to_string(entry).map_err(|e| corrupt("entry", e))?),
                entry.body.kind(),
            ),
            // A retraction has its target's kind; validation found the target.
            FactChange::Retract => {
                let target = fact.supersedes.as_ref().map(|s| s.fact_id);
                let kind = rows
                    .facts
                    .iter()
                    .find(|f| Some(f.fact_id) == target)
                    .and_then(|f| f.change.entry())
                    .map(|e| e.body.kind())
                    .ok_or_else(|| corrupt("retraction target", "not an assertion"))?;
                ("retract", None, kind)
            }
        };
        tx.execute(
            "INSERT INTO claimant_fact
                 (employee_id, kind, action, entry, recorded_at, confirmed_by,
                  evidence_hash, supersedes, reason)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                fact.employee_id,
                kind.as_str(),
                action,
                entry,
                recorded_at.to_string(),
                fact.confirmed_by,
                fact.evidence_hash.map(|h| h.to_hex()),
                fact.supersedes.as_ref().map(|s| s.fact_id),
                fact.supersedes.as_ref().map(|s| s.reason.as_str()),
            ],
        )?;
        let fact_id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(ClaimantFact {
            fact_id,
            employee_id: fact.employee_id.clone(),
            change: fact.change.clone(),
            recorded_at,
            confirmed_by: fact.confirmed_by.clone(),
            evidence_hash: fact.evidence_hash,
            supersedes: fact.supersedes.as_ref().map(|s| s.fact_id),
            reason: fact.supersedes.as_ref().map(|s| s.reason.clone()),
        })
    }
}

/// Sum one `(employee, tax_year)`'s earnings rows into a `Money`. The amounts are
/// stored as decimal strings and summed with `Decimal` (never SQL `SUM`, which would
/// coerce text) so the total is exact. The currency is derived from the rows and
/// verified consistent across them — `accumulate_ytd` guards the year to a single
/// currency at write time, but summing re-checks rather than trusting one arbitrary
/// row, so a bypassed guard (migration, manual repair) surfaces as a loud corruption
/// error instead of a silently mislabelled total. An empty year is `0.00 EUR` (the
/// system is single-currency, so a zero YTD is a zero-euro YTD).
fn sum_year(conn: &Connection, employee_id: &str, tax_year: i32) -> Result<Money, LedgerError> {
    let mut stmt = conn.prepare(
        "SELECT gross_amount, gross_currency FROM ytd_earnings
         WHERE employee_id = ?1 AND tax_year = ?2",
    )?;
    let mut total = Decimal::ZERO;
    let mut currency: Option<String> = None;
    let rows = stmt.query_map(params![employee_id, tax_year], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (amount, code) = row?;
        match &currency {
            None => currency = Some(code),
            Some(seen) if *seen != code => {
                return Err(LedgerError::Store(format!(
                    "ytd_earnings currency mix for {employee_id} {tax_year}: {seen} vs {code}"
                )));
            }
            Some(_) => {}
        }
        let d: Decimal = amount
            .parse()
            .map_err(|e| LedgerError::Store(format!("ytd_earnings amount corrupt: {e}")))?;
        total += d;
    }
    total.rescale(2);
    let currency = match currency {
        Some(code) => Currency::new(code).map_err(LedgerError::Model)?,
        None => Currency::new("EUR").map_err(LedgerError::Model)?,
    };
    Ok(Money::new(total, currency))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aggountant_domain::TaxCard;
    use aggountant_model::Currency;

    fn eur(s: &str) -> Money {
        Money::parse(s, Currency::new("EUR").unwrap()).unwrap()
    }

    fn sample(id: &str) -> EmployeeState {
        EmployeeState {
            employee_id: id.into(),
            name: "Maija Meikäläinen".into(),
            hetu: "131052-308T".into(),
            iban: "FI2112345600000785".into(),
            tax_card: TaxCard {
                withholding_pct: "20.00".into(),
            },
        }
    }

    #[test]
    fn upsert_get_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        let got = store.get("emp-1").unwrap().unwrap();
        assert_eq!(got, sample("emp-1"));
        assert!(store.get("nope").unwrap().is_none());
    }

    #[test]
    fn accumulate_ytd_is_additive_per_natural_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        let after = store
            .accumulate_ytd("emp-1", 2026, "run:2", &eur("3000.00"))
            .unwrap();
        assert_eq!(after.amount_str(), "6000.00");
        assert_eq!(
            store.ytd_gross("emp-1", 2026).unwrap().amount_str(),
            "6000.00"
        );
    }

    #[test]
    fn accumulate_ytd_is_idempotent_on_the_natural_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        // Re-applying the SAME natural key (a crash/retry) does not double-count.
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        let again = store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        assert_eq!(again.amount_str(), "3000.00");
    }

    #[test]
    fn ytd_is_scoped_per_tax_year() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        // The SAME natural_key in two different years is two independent earnings rows
        // (the PK includes tax_year), and neither year bleeds into the other.
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        let y2027 = store
            .accumulate_ytd("emp-1", 2027, "run:1", &eur("3500.00"))
            .unwrap();
        assert_eq!(y2027.amount_str(), "3500.00");
        assert_eq!(
            store.ytd_gross("emp-1", 2026).unwrap().amount_str(),
            "3000.00"
        );
        assert_eq!(
            store.ytd_gross("emp-1", 2027).unwrap().amount_str(),
            "3500.00"
        );
    }

    #[test]
    fn upsert_does_not_reset_accumulated_ytd() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        // Re-adding the employee with a *changed* identity field (a corrected IBAN)
        // must persist the identity change yet leave the YTD untouched — the earnings
        // live in a separate table the identity upsert never rewrites.
        let mut corrected = sample("emp-1");
        corrected.iban = "FI9080002627761478".into();
        store.upsert(&corrected).unwrap();
        assert_eq!(
            store.get("emp-1").unwrap().unwrap().iban,
            "FI9080002627761478"
        );
        assert_eq!(
            store.ytd_gross("emp-1", 2026).unwrap().amount_str(),
            "3000.00"
        );
    }

    #[test]
    fn reusing_a_key_with_a_different_amount_is_a_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        // Same key, same amount → benign replay.
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        // Same key, DIFFERENT amount → loud conflict, not a silent stale replay.
        let err = store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3500.00"))
            .unwrap_err();
        assert_eq!(err.error_code(), "IDEMPOTENCY_KEY_REUSE");
        // The stored total is unchanged by the rejected call.
        assert_eq!(
            store.ytd_gross("emp-1", 2026).unwrap().amount_str(),
            "3000.00"
        );
    }

    #[test]
    fn sum_year_rejects_a_currency_mix() {
        // Defence-in-depth: if the write-time currency guard is ever bypassed and two
        // currencies land in one year, summing them must be a loud corruption error,
        // not a silently mislabelled total.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pii.sqlite");
        let store = SqlitePiiStore::open(&path).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO ytd_earnings
                     (employee_id, tax_year, natural_key, gross_amount, gross_currency)
                 VALUES('emp-1', 2026, 'run:usd', '10.00', 'USD')",
                [],
            )
            .unwrap();
        }
        let err = store.ytd_gross("emp-1", 2026).unwrap_err();
        assert_eq!(err.error_code(), "STORE_ERROR");
    }

    #[test]
    fn deleting_an_employee_cascades_their_earnings() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        store
            .accumulate_ytd("emp-1", 2026, "run:1", &eur("3000.00"))
            .unwrap();
        // The store's raison d'être is erasability: a DELETE of the identity row must
        // leave no orphaned salary history behind (ON DELETE CASCADE + foreign_keys=ON).
        {
            let conn = store.conn.lock().unwrap();
            conn.execute("DELETE FROM employee WHERE employee_id = 'emp-1'", [])
                .unwrap();
            let remaining: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM ytd_earnings WHERE employee_id = 'emp-1'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                remaining, 0,
                "earnings must cascade-delete with the employee"
            );
        }
    }

    #[test]
    fn ytd_gross_is_zero_for_a_year_with_no_earnings() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        assert_eq!(store.ytd_gross("emp-1", 2026).unwrap().amount_str(), "0.00");
    }

    #[test]
    fn accumulate_missing_employee_is_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        let err = store
            .accumulate_ytd("ghost", 2026, "run:1", &eur("1.00"))
            .unwrap_err();
        assert_eq!(err.error_code(), "EMPLOYEE_NOT_FOUND");
    }

    #[test]
    fn ytd_gross_missing_employee_is_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        let err = store.ytd_gross("ghost", 2026).unwrap_err();
        assert_eq!(err.error_code(), "EMPLOYEE_NOT_FOUND");
    }

    fn user_version(conn: &rusqlite::Connection) -> i32 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    /// A fresh PII store is migrated and stamped to the latest version.
    #[test]
    fn fresh_pii_store_is_stamped() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        assert_eq!(user_version(&store.conn.lock().unwrap()), 2);
    }

    /// Re-opening an already-migrated PII store is a no-op and preserves data.
    #[test]
    fn reopening_a_pii_store_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pii.sqlite");
        {
            let store = SqlitePiiStore::open(&path).unwrap();
            store.upsert(&sample("emp-1")).unwrap();
        }
        let store = SqlitePiiStore::open(&path).unwrap();
        assert_eq!(user_version(&store.conn.lock().unwrap()), 2);
        assert!(store.get("emp-1").unwrap().is_some());
    }

    /// A PII home predating the framework (tables present, `user_version` 0) is
    /// refused on open rather than silently adopted.
    #[test]
    fn a_pre_framework_pii_home_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pii.sqlite");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(MIGRATIONS[0].sql).unwrap();
            conn.pragma_update(None, "user_version", 0).unwrap();
        }
        let err = match SqlitePiiStore::open(&path) {
            Ok(_) => panic!("a pre-framework PII home must be refused, not opened"),
            Err(e) => e,
        };
        assert_eq!(err.error_code(), "STORE_ERROR");
        assert!(err.to_string().contains("predates"), "{err}");
        crate::store::migrations::tests::assert_refusal_gives_no_reset_advice(&err.to_string());
    }

    /// A PII store written by a newer aggountant (the case met when a binary is
    /// rolled back after a pii migration) is refused without reset advice.
    #[test]
    fn a_newer_pii_store_is_refused_without_reset_advice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pii.sqlite");
        drop(SqlitePiiStore::open(&path).unwrap());
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", MIGRATIONS.len() as i32 + 1)
                .unwrap();
        }
        let err = match SqlitePiiStore::open(&path) {
            Ok(_) => panic!("a newer PII store must be refused"),
            Err(e) => e,
        };
        assert_eq!(err.error_code(), "STORE_ERROR");
        assert!(err.to_string().contains("newer aggountant"), "{err}");
        crate::store::migrations::tests::assert_refusal_gives_no_reset_advice(&err.to_string());
    }

    /// Every row of the v1 tables, as text, in key order.
    fn v1_rows(conn: &rusqlite::Connection) -> Vec<String> {
        let mut out = Vec::new();
        for sql in [
            "SELECT employee_id || '|' || hex(sealed) FROM employee ORDER BY employee_id",
            "SELECT employee_id || '|' || tax_year || '|' || natural_key || '|' || \
             gross_amount || '|' || gross_currency FROM ytd_earnings \
             ORDER BY employee_id, tax_year, natural_key",
            "SELECT type || '|' || name || '|' || sql FROM sqlite_master \
             WHERE name IN ('employee', 'ytd_earnings') ORDER BY name",
        ] {
            let mut stmt = conn.prepare(sql).unwrap();
            out.extend(
                stmt.query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .map(Result::unwrap),
            );
        }
        out
    }

    /// The v2 migration is additive: a v1 store (a home from before the claimant
    /// register) keeps every row and table definition byte for byte, and gains
    /// two empty tables.
    #[test]
    fn the_claimant_migration_is_additive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pii.sqlite");
        {
            // A v1 store with data, as a binary from before this migration leaves it.
            let mut conn = rusqlite::Connection::open(&path).unwrap();
            migrations::apply(&mut conn, "pii", &MIGRATIONS[..1]).unwrap();
            assert_eq!(user_version(&conn), 1);
            conn.execute(
                "INSERT INTO employee(employee_id, sealed) VALUES('emp-1', x'7b7d')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO ytd_earnings VALUES('emp-1', 2026, 'run:1', '3000.00', 'EUR')",
                [],
            )
            .unwrap();
        }
        let before = v1_rows(&rusqlite::Connection::open(&path).unwrap());
        let store = SqlitePiiStore::open(&path).unwrap();
        let conn = store.conn.lock().unwrap();
        assert_eq!(user_version(&conn), 2);
        assert_eq!(v1_rows(&conn), before);
        for table in ["claimant_place", "claimant_fact"] {
            let n: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} starts empty");
        }
    }

    fn claimant_place(employee_id: &str) -> Place {
        Place {
            employee_id: employee_id.into(),
            place_id: "home".into(),
            kind: aggountant_domain::workplace::PlaceKind::Home,
            label: "Koti".into(),
            location: "Esimerkkikatu 1, Tampere".into(),
            municipality: "Tampere".into(),
            replaces: None,
            recorded_at: Instant::from_unix(1_800_000_000, 0).unwrap(),
            confirmed_by: "Maija Meikäläinen".into(),
        }
    }

    fn claimant_fact(employee_id: &str) -> NewFact {
        use aggountant_domain::workplace::{BasisCode, FactBody, WorkplaceRole};
        NewFact {
            employee_id: employee_id.into(),
            change: FactChange::Assert(FactEntry {
                valid_from: aggountant_model::Date::parse("2025-01-01").unwrap(),
                valid_to: None,
                body: FactBody::Workplace {
                    place_id: Some("home".into()),
                    role: WorkplaceRole::RegularPrimary,
                    basis: BasisCode::CompanyPremisesInHome,
                    basis_explanation: None,
                },
            }),
            confirmed_by: "Maija Meikäläinen".into(),
            evidence_hash: None,
            supersedes: None,
        }
    }

    /// Register rows are never edited in place, and the person's erasure takes
    /// them with it (places, facts and the corrections that reference them).
    #[test]
    fn claimant_rows_are_append_only_and_erased_with_the_person() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        let at = Instant::from_unix(1_800_000_001, 0).unwrap();
        store
            .append_claimant_place(&claimant_place("emp-1"), &|_| Ok(()))
            .unwrap();
        let first = store
            .append_claimant_fact(&claimant_fact("emp-1"), at, &|_| Ok(()))
            .unwrap();
        let mut correction = claimant_fact("emp-1");
        correction.supersedes = Some(aggountant_domain::workplace::Supersede {
            fact_id: first.fact_id,
            reason: "typo".into(),
        });
        store
            .append_claimant_fact(&correction, at, &|_| Ok(()))
            .unwrap();
        // A second correction of the same row is refused by the schema too.
        assert!(store
            .append_claimant_fact(&correction, at, &|_| Ok(()))
            .is_err());
        let rows = store.claimant_rows("emp-1").unwrap();
        assert_eq!(rows.places, vec![claimant_place("emp-1")]);
        assert_eq!(rows.facts.len(), 2);
        assert_eq!(rows.facts[0], first);

        let conn = store.conn.lock().unwrap();
        for sql in [
            "UPDATE claimant_place SET label = 'x'",
            "UPDATE claimant_fact SET confirmed_by = 'x'",
        ] {
            let err = conn.execute(sql, []).unwrap_err();
            assert!(err.to_string().contains("append-only"), "{err}");
        }
        conn.execute("DELETE FROM employee WHERE employee_id = 'emp-1'", [])
            .unwrap();
        for table in ["claimant_place", "claimant_fact"] {
            let n: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} must cascade-delete with the employee");
        }
    }

    /// A failed validation leaves no row behind.
    #[test]
    fn a_refused_claimant_write_appends_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        store.upsert(&sample("emp-1")).unwrap();
        let refuse = |_: &ClaimantRows| Err(LedgerError::Store("refused".into()));
        assert!(store
            .append_claimant_place(&claimant_place("emp-1"), &refuse)
            .is_err());
        let at = Instant::from_unix(1_800_000_001, 0).unwrap();
        assert!(store
            .append_claimant_fact(&claimant_fact("emp-1"), at, &refuse)
            .is_err());
        let rows = store.claimant_rows("emp-1").unwrap();
        assert!(rows.places.is_empty() && rows.facts.is_empty());
    }

    /// Foreign keys are enforced at runtime (set after migrations), so the
    /// earnings→employee cascade works.
    #[test]
    fn foreign_keys_are_enforced_after_open() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqlitePiiStore::open(&dir.path().join("pii.sqlite")).unwrap();
        let conn = store.conn.lock().unwrap();
        let fk_on: bool = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert!(fk_on, "foreign_keys must be ON for the erasure cascade");
    }
}
