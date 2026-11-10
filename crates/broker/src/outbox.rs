//! F9 substrate: the durable **outbox** — SQLite-backed and **crash-safe**. One
//! logical entry per `(service, effect_key)` records **every attempt** — its
//! `sent_hash`, its per-attempt `Provenance`, and its outcome — append-only within
//! the entry. Two properties the M3 gate leans on:
//!
//! - **Per-attempt provenance is never overwritten** (`candidate.md §3`): a retry
//!   appends a new attempt row; it does not mutate the previous one, so the trail
//!   keeps the full send history and each attempt's `sent_hash`.
//! - **Idempotency replay** (O-24 idempotency): a terminal *success* short-circuits
//!   a re-submit, but an `Unknown` does **not** — it deliberately leaves the door
//!   open for a driver-initiated retry that reuses the same `sent_hash`.
//!
//! ## Crash-safety: the pre-dispatch intent (`outbox-sqlite-crash-safe`)
//! Each attempt is a **two-phase** durable write around the network call:
//!
//! 1. [`Outbox::begin_attempt`] records the *intent* to dispatch — an `in_flight`
//!    row with its `sent_hash` and provenance but **no outcome yet** — and commits
//!    it **before** the adapter is called.
//! 2. The Broker calls the adapter.
//! 3. [`Outbox::resolve_attempt`] updates that row to its terminal outcome.
//!
//! A crash anywhere in the dispatch→resolve window leaves a durable `in_flight`
//! row. On restart [`Outbox::load`] reports that unresolved intent as
//! [`AttemptOutcome::Unknown`]: the send *may* have reached the authority, so
//! at-most-once forbids a blind retry, while a **deliberate** retry stays open (it
//! reuses the same `sent_hash` and meets the authority's idempotency `409`). This is
//! exactly the `Unknown`-then-recover path the M3 gate proved, now durable across a
//! crash *mid-send*, not only across a timeout.
//!
//! Durability rests on SQLite `synchronous=FULL` + WAL: `begin_attempt`'s commit
//! reaches the WAL before the adapter call, so the intent survives a **process
//! crash**. Power-loss durability inherits the crate-wide fsync floor — the fiscal
//! `ledger` store uses the identical pragma set (no `PRAGMA fullfsync`), so this is
//! not the place to make a stronger physical-media claim than the rest of the
//! system. Each phase runs in a `BEGIN IMMEDIATE` transaction so writers serialize
//! per database; cross-*process* contention on one entry is a multi-user concern
//! tracked in `multi-user-considerations`, not a single-user hazard here. This is a
//! fresh-install store — pre-release + single-user means there is no legacy on-disk
//! outbox state to migrate (a `PRAGMA user_version` framework is deferred with the
//! fiscal schema migrations).
//!
//! This lives in the `integration` runtime namespace (ADR-1: integration splits
//! config→git / runtime→process-store). It is not fiscal truth; the irreversible
//! *fact* of a filing is the Ledger's hash-chained `period_transition` event. The
//! outbox is the delivery ledger that feeds it.

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::envelope::Provenance;
use crate::error::BrokerError;

/// The recorded outcome of one attempt (mirrors [`crate::RemoteOutcome`] minus the
/// caller-facing payloads not needed for the audit trail).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttemptOutcome {
    Delivered {
        receipt: String,
    },
    RecoveredAlreadyProcessed {
        receipt: String,
    },
    /// No *definitive* answer — the outcome is ambiguous (a socket timeout, an
    /// ambiguous proxy `5xx`, an unverifiable `409`, or a crash-recovered in-flight
    /// intent). Recorded, but **not** a terminal success — a later deliberate submit
    /// may retry. `code`/`detail` carry the *cause*, so an operator triaging a parked
    /// `Unknown` in the outbox can tell a socket timeout (`TIMEOUT`) from a proxy
    /// `5xx` (`HTTP_503`) from an unverifiable `409` (`HTTP_409`) from a crashed
    /// send (`CRASH_UNRESOLVED`) — the whole point of
    /// `broker-unknown-cause-observability`. This preserves the string that the old
    /// unit variant discarded.
    Unknown {
        code: String,
        detail: String,
    },
    /// A definitive negative (rejection / retryable transport that exhausted).
    Failed {
        code: String,
        detail: String,
    },
}

impl AttemptOutcome {
    fn is_terminal_success(&self) -> bool {
        matches!(
            self,
            AttemptOutcome::Delivered { .. } | AttemptOutcome::RecoveredAlreadyProcessed { .. }
        )
    }
}

/// One immutable attempt record. Written once, never mutated — a crashed intent is
/// still surfaced (as [`AttemptOutcome::Unknown`]) rather than dropped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub attempt: u32,
    pub sent_hash: String,
    pub outcome: AttemptOutcome,
    pub provenance: Provenance,
}

/// The durable record for one `(service, effect_key)` submission and all its
/// attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub service: String,
    pub effect_key: String,
    pub natural_key: String,
    pub attempts: Vec<Attempt>,
}

impl OutboxEntry {
    /// The receipt of a terminal-success attempt, if the entry has one.
    pub fn terminal_success(&self) -> Option<&Attempt> {
        self.attempts
            .iter()
            .find(|a| a.outcome.is_terminal_success())
    }
    /// The next attempt number (1-based). An unresolved (crashed) intent still counts
    /// as a used attempt number, so a deliberate retry always gets a fresh index.
    /// Derived from the max recorded index (not the row count) so a non-contiguous
    /// history can never collide with an existing attempt on insert.
    pub fn next_attempt_no(&self) -> u32 {
        self.attempts.iter().map(|a| a.attempt).max().unwrap_or(0) + 1
    }
}

/// The SQLite-backed, crash-safe outbox rooted under `<home>/integration/outbox`.
pub struct Outbox {
    // `Mutex` so a single `Outbox` stays `Send + Sync` and in-process sharing
    // serializes rather than racing (mirrors the fiscal/PII stores). Cross-*process*
    // racing still uses independent connections + the `busy_timeout` writer lock.
    conn: Mutex<Connection>,
}

impl Outbox {
    /// Open (creating if absent) the outbox database under `<home>/integration/outbox`.
    pub fn open(home: &Path) -> Result<Self, BrokerError> {
        let dir = home.join("integration").join("outbox");
        fs::create_dir_all(&dir).map_err(io_err)?;
        // Sent hashes can be dictionary-verifiable PII. Keep the delivery ledger
        // and its WAL private even in homes predating this integration channel.
        // Preserve operator rules, adding the exclusion last so it takes priority.
        let ignore = dir.join(".gitignore");
        let current = match fs::read_to_string(&ignore) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(io_err(e)),
        };
        if current.lines().last() != Some("/outbox.db*") {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(ignore)
                .map_err(io_err)?;
            file.write_all(
                b"\n# Private delivery ledger (including PII-derived sent hashes).\n/outbox.db*\n",
            )
            .map_err(io_err)?;
            file.sync_all().map_err(io_err)?;
        }
        let conn = Connection::open(dir.join("outbox.db")).map_err(sql_err)?;
        // `synchronous=FULL` is what makes the pre-dispatch intent crash-safe: the
        // `begin_attempt` commit reaches disk before the adapter is ever called.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;",
        )
        .map_err(sql_err)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS outbox_entries(
                service     TEXT NOT NULL,
                effect_key  TEXT NOT NULL,
                natural_key TEXT NOT NULL,
                sent_hash   TEXT NOT NULL,
                PRIMARY KEY(service, effect_key)
             );
             CREATE TABLE IF NOT EXISTS outbox_attempts(
                service         TEXT NOT NULL,
                effect_key      TEXT NOT NULL,
                attempt         INTEGER NOT NULL CHECK(attempt >= 1),
                sent_hash       TEXT NOT NULL,
                status          TEXT NOT NULL CHECK(status IN ('in_flight','resolved')),
                outcome_json    TEXT,
                provenance_json TEXT NOT NULL,
                PRIMARY KEY(service, effect_key, attempt),
                FOREIGN KEY(service, effect_key)
                    REFERENCES outbox_entries(service, effect_key),
                -- A resolved row has an outcome; an in-flight intent never does.
                CHECK((status = 'resolved') = (outcome_json IS NOT NULL))
             );",
        )
        .map_err(sql_err)?;
        Ok(Outbox {
            conn: Mutex::new(conn),
        })
    }

    /// Load the entry for `(service, effect_key)`, or `None` if none exists yet. An
    /// unresolved `in_flight` attempt (a crash in the dispatch→resolve window) is
    /// surfaced as [`AttemptOutcome::Unknown`] — the recovery interpretation.
    pub fn load(
        &self,
        service: &str,
        effect_key: &str,
    ) -> Result<Option<OutboxEntry>, BrokerError> {
        let conn = self.conn.lock().expect("outbox mutex");
        let natural_key: Option<String> = conn
            .query_row(
                "SELECT natural_key FROM outbox_entries WHERE service = ?1 AND effect_key = ?2",
                params![service, effect_key],
                |r| r.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        let Some(natural_key) = natural_key else {
            return Ok(None);
        };

        let mut stmt = conn
            .prepare(
                "SELECT attempt, sent_hash, status, outcome_json, provenance_json
                 FROM outbox_attempts
                 WHERE service = ?1 AND effect_key = ?2
                 ORDER BY attempt",
            )
            .map_err(sql_err)?;
        let rows = stmt
            .query_map(params![service, effect_key], |r| {
                Ok((
                    r.get::<_, i64>(0)? as u32,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .map_err(sql_err)?;

        let mut attempts = Vec::new();
        for row in rows {
            let (attempt, sent_hash, status, outcome_json, provenance_json) =
                row.map_err(sql_err)?;
            let outcome = match status.as_str() {
                // Recovery: a durable intent whose outcome never landed reads back as
                // Unknown — a send may have happened; at-most-once forbids a blind
                // retry while a deliberate one stays open. Its cause is distinct from a
                // timeout/ambiguous-HTTP Unknown (the terminal outcome was never
                // persisted), so the trail labels it `CRASH_UNRESOLVED` — an operator can
                // tell an unresolved intent from a socket timeout. The label names the
                // dominant cause (a crash in the dispatch→resolve window); the detail is
                // deliberately broader, because a failed `resolve_attempt` write after a
                // real adapter answer leaves the same unresolved row — all the trail can
                // assert is that no terminal resolution was persisted, not that a crash
                // in particular occurred.
                "in_flight" => AttemptOutcome::Unknown {
                    code: "CRASH_UNRESOLVED".into(),
                    detail: "in-flight intent recorded; no terminal resolution was \
                             persisted (a crash in the dispatch→resolve window, or a \
                             failed resolution write after the adapter answered)"
                        .into(),
                },
                "resolved" => {
                    let json = outcome_json.ok_or_else(|| {
                        BrokerError::OutboxUnavailable(format!(
                            "resolved attempt {attempt} of {service}/{effect_key} has no outcome"
                        ))
                    })?;
                    serde_json::from_str(&json)
                        .map_err(|e| BrokerError::OutboxUnavailable(e.to_string()))?
                }
                // The CHECK constraint forbids any other status on a normal write;
                // reaching here means corruption or an out-of-band edit — fail loudly
                // rather than silently treat it as resolved.
                other => {
                    return Err(BrokerError::OutboxUnavailable(format!(
                        "attempt {attempt} of {service}/{effect_key} has unknown status {other:?}"
                    )))
                }
            };
            let provenance: Provenance = serde_json::from_str(&provenance_json)
                .map_err(|e| BrokerError::OutboxUnavailable(e.to_string()))?;
            attempts.push(Attempt {
                attempt,
                sent_hash,
                outcome,
                provenance,
            });
        }
        Ok(Some(OutboxEntry {
            service: service.to_string(),
            effect_key: effect_key.to_string(),
            natural_key,
            attempts,
        }))
    }

    /// **Pre-dispatch intent.** Durably record — *before* the network call — the
    /// intent to make `attempt` for this effect, as an `in_flight` row with its
    /// `sent_hash` and provenance but no outcome. Runs inside a `BEGIN IMMEDIATE`
    /// transaction under `synchronous=FULL`, so the intent is on disk before the
    /// adapter is called; a crash after this point recovers as `Unknown`.
    pub fn begin_attempt(
        &self,
        service: &str,
        effect_key: &str,
        natural_key: &str,
        attempt: u32,
        sent_hash: &str,
        provenance: &Provenance,
    ) -> Result<(), BrokerError> {
        let provenance_json = serde_json::to_string(provenance)
            .map_err(|e| BrokerError::OutboxUnavailable(e.to_string()))?;
        let mut conn = self.conn.lock().expect("outbox mutex");
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_err)?;
        // Bind the effect to its (natural_key, sent_hash) on first sight; keep both
        // stable after. Binding the sent_hash here is what makes every attempt of an
        // effect reuse the same sent bytes: a retry after an `Unknown` that changed
        // the payload is a key collision, not a replay, and the 409-recovery story
        // depends on the authority seeing identical bytes under the same key.
        tx.execute(
            "INSERT INTO outbox_entries(service, effect_key, natural_key, sent_hash)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(service, effect_key) DO NOTHING",
            params![service, effect_key, natural_key, sent_hash],
        )
        .map_err(sql_err)?;
        let (bound_nk, bound_hash): (String, String) = tx
            .query_row(
                "SELECT natural_key, sent_hash FROM outbox_entries
                 WHERE service = ?1 AND effect_key = ?2",
                params![service, effect_key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(sql_err)?;
        // The Broker already refuses these before dispatch; guard the store too so a
        // stray caller can never fork an effect's identity or its sent bytes.
        if bound_nk != natural_key {
            return Err(BrokerError::IdempotencyMismatch(format!(
                "effect {effect_key} was bound to natural_key {bound_nk:?}, incoming {natural_key:?}"
            )));
        }
        if bound_hash != sent_hash {
            return Err(BrokerError::IdempotencyMismatch(format!(
                "effect {effect_key} was bound to sent_hash {bound_hash}, incoming {sent_hash}"
            )));
        }
        tx.execute(
            "INSERT INTO outbox_attempts(
                service, effect_key, attempt, sent_hash, status, outcome_json, provenance_json)
             VALUES(?1, ?2, ?3, ?4, 'in_flight', NULL, ?5)",
            params![service, effect_key, attempt, sent_hash, provenance_json],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        Ok(())
    }

    /// Resolve the `in_flight` intent for `attempt` to its terminal `outcome`, called
    /// *after* the adapter returns. Fails loudly if there is no matching in-flight
    /// intent (already resolved, or never begun) — begin/resolve must be paired.
    pub fn resolve_attempt(
        &self,
        service: &str,
        effect_key: &str,
        attempt: u32,
        outcome: &AttemptOutcome,
    ) -> Result<(), BrokerError> {
        let outcome_json = serde_json::to_string(outcome)
            .map_err(|e| BrokerError::OutboxUnavailable(e.to_string()))?;
        let mut conn = self.conn.lock().expect("outbox mutex");
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_err)?;
        let updated = tx
            .execute(
                "UPDATE outbox_attempts
                 SET status = 'resolved', outcome_json = ?4
                 WHERE service = ?1 AND effect_key = ?2 AND attempt = ?3
                   AND status = 'in_flight'",
                params![service, effect_key, attempt, outcome_json],
            )
            .map_err(sql_err)?;
        if updated != 1 {
            return Err(BrokerError::OutboxUnavailable(format!(
                "no in-flight intent to resolve for {service}/{effect_key} attempt {attempt}"
            )));
        }
        tx.commit().map_err(sql_err)?;
        Ok(())
    }
}

fn sql_err(e: rusqlite::Error) -> BrokerError {
    BrokerError::OutboxUnavailable(e.to_string())
}

fn io_err(e: std::io::Error) -> BrokerError {
    BrokerError::OutboxUnavailable(e.to_string())
}
