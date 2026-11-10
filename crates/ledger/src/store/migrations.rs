//! `PRAGMA user_version` schema-migration framework for the ledger's SQLite
//! stores (the `fiscal` DB and the `pii` masterdata DB).
//!
//! **Why this exists.** Both stores previously created their schema with a batch
//! of `CREATE TABLE IF NOT EXISTS` on every open. That is fine for a *fresh*
//! home, but `IF NOT EXISTS` is a no-op against an already-existing table: it can
//! never add a *column* to a DB created by an older slice (e.g. S3's `sent_hash`
//! on a pre-existing S2 `vouchers` table). The result would be a table that is
//! silently missing a column the code now reads. This framework replaces the
//! ambient `IF NOT EXISTS` schema with an ordered, versioned list of migrations
//! stamped into the database header via `PRAGMA user_version`.
//!
//! **The model.** Each store owns a `&[Migration]` list. `version` is the
//! `user_version` the database is left at *after* the migration's `sql` runs; the
//! list is contiguous, starting at 1. On open we read the current `user_version`;
//! if the DB is already current we do no writes (a lock-free fast path that lets a
//! future read-only open avoid taking a write lock). Otherwise all pending
//! migrations run inside **one** `BEGIN IMMEDIATE` transaction that also bumps the
//! version — so the whole upgrade is atomic (a crash leaves the DB at the old
//! version, never half-migrated) and two concurrent openers cannot both apply the
//! same migration: the second blocks on the writer lock, then re-reads the version
//! (now advanced) and finds nothing to do. Re-opening an up-to-date DB is a no-op.
//!
//! **Adding a migration.** Never edit a landed migration's `sql` (existing DBs
//! have already run it and will not re-run it). Append a new entry with the next
//! version and an `ALTER TABLE`/`CREATE TABLE`/`CREATE INDEX` batch. v1 is the
//! baseline that a fresh home initializes to; from v2 onward, migrations describe
//! *changes*.
//!
//! **Baseline + legacy homes (pre-production).** v1 captures the schema *as it
//! stands today* — including columns that landed inline in earlier slices (e.g.
//! S3's `vouchers.sent_hash`) rather than through a migration. It cannot
//! retroactively add such a column to a genuinely old DB that predates that inline
//! change. Rather than *silently* stamp such a home to v1 (leaving a DB that
//! claims to be current but errors at runtime with `no such column`), [`apply`]
//! **refuses** it: a `user_version == 0` database that already contains
//! application tables predates this framework and is rejected with an actionable
//! error. This is safe for a pre-production, single-user system whose dev/test
//! data is resettable — reset such a home rather than migrate it. A truly fresh DB
//! (`user_version == 0`, no tables) initializes to v1; from there the framework
//! carries schema changes forward for any DB created at or after v1.
//!
//! **Foreign keys during migration.** Migrations run with `PRAGMA foreign_keys`
//! at its connection default (OFF): the SQLite table-rebuild ("12-step")
//! procedure a future column-drop/retype migration needs requires FK enforcement
//! off, and the pragma is a no-op inside a transaction (so it cannot be toggled
//! once [`apply`] holds its `IMMEDIATE` lock). The store enables `foreign_keys`
//! for runtime enforcement *after* [`apply`] returns. A future rebuild migration
//! should additionally run `PRAGMA foreign_key_check` before its work commits;
//! that machinery is deferred until the first such migration exists.
//!
//! Connection-level PRAGMAs (`journal_mode`, `synchronous`, `foreign_keys`,
//! `busy_timeout`) are **not** migrations — they are per-connection settings the
//! store applies on every open (see each store's `open`), around [`apply`].

use rusqlite::{Connection, TransactionBehavior};

use crate::error::LedgerError;

/// One forward-only schema migration. `version` is the `PRAGMA user_version` the
/// database is stamped with after `sql` runs; `sql` is a batch of DDL statements
/// (no connection-level PRAGMAs, no transaction-control statements — `apply`
/// wraps the batch in a transaction and relies on that framing).
pub(crate) struct Migration {
    pub version: i32,
    pub sql: &'static str,
}

/// Bring `conn`'s schema up to the latest `migrations` version. `label` names the
/// store in error messages (`"fiscal"` / `"pii"`).
///
/// Refuses: (a) a migration list that is not contiguous starting at 1 — a
/// programming error; (b) a database whose `user_version` is *newer* than this
/// binary's latest migration (a downgrade); (c) a corrupt/foreign negative
/// `user_version`; and (d) a `user_version == 0` database that already holds
/// application tables — a home that predates this framework, which is refused with
/// a reset instruction rather than silently stamped to v1 (see the module docs).
///
/// All pending migrations run in a single `IMMEDIATE` transaction so the upgrade
/// is atomic and race-free across concurrent openers; an already-current database
/// takes the lock-free fast path and performs no writes.
pub(crate) fn apply(
    conn: &mut Connection,
    label: &str,
    migrations: &[Migration],
) -> Result<(), LedgerError> {
    // The list must be contiguous starting at 1: a duplicate or gap would make the
    // "apply everything above current" fold ambiguous. This guards the constant at
    // the callsite against a future editing mistake, in release too.
    for (i, m) in migrations.iter().enumerate() {
        let expected = i as i32 + 1;
        if m.version != expected {
            return Err(LedgerError::Store(format!(
                "{label} migration list must be contiguous starting at 1: \
                 position {i} expected v{expected}, found v{}",
                m.version
            )));
        }
    }
    let latest = migrations.last().map_or(0, |m| m.version);

    // Fast path: a cheap unlocked read. If the DB is already current (the common
    // steady-state open), do nothing and take no write lock — so a read-only open
    // never has to write. Downgrade/corruption is caught here too, before we would
    // otherwise mutate the file.
    let current: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    check_version_range(label, current, latest)?;
    if current == latest {
        return Ok(());
    }

    // Work to do → take the writer lock up front (IMMEDIATE), then re-read the
    // version under the lock. A concurrent opener that raced us to here will have
    // committed its migrations before we acquire the lock, so our re-read sees the
    // advanced version and we skip whatever it already applied — no double-apply of
    // a non-idempotent `ALTER TABLE`.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current: i32 = tx.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    check_version_range(label, current, latest)?;
    if current == 0 {
        // A user_version of 0 with application tables already present is a home
        // created before this framework existed. v1's `IF NOT EXISTS` would no-op
        // over such tables and stamp it "current" while columns added inline in
        // later slices (e.g. `vouchers.sent_hash`) stay missing. Refuse loudly
        // instead — the data is resettable (see module docs).
        let table_count: i64 = tx.query_row(
            "SELECT count(*) FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if table_count > 0 {
            return Err(LedgerError::Store(format!(
                "{label} database predates the schema-migration framework \
                 (user_version 0 with existing tables) and may be missing columns \
                 added in later slices — reset this home and re-run (its data is \
                 resettable)"
            )));
        }
    }

    for m in migrations.iter().filter(|m| m.version > current) {
        tx.execute_batch(m.sql).map_err(|e| {
            LedgerError::Store(format!("{label} migration v{} failed: {e}", m.version))
        })?;
        // `user_version` lives in the DB header and is transactional with the DDL
        // above, so the schema and the version it names commit together. It cannot
        // be a bound parameter; `pragma_update` formats the i32 itself.
        tx.pragma_update(None, "user_version", m.version)
            .map_err(|e| {
                LedgerError::Store(format!(
                    "{label} migration v{} version-stamp failed: {e}",
                    m.version
                ))
            })?;
    }
    tx.commit()
        .map_err(|e| LedgerError::Store(format!("{label} migration commit failed: {e}")))?;
    Ok(())
}

/// Reject a `user_version` outside the accepted range `[0, latest]`: a negative
/// value (corrupt or foreign metadata) or one newer than this binary knows (a
/// downgrade we will not silently run an old code path against).
fn check_version_range(label: &str, current: i32, latest: i32) -> Result<(), LedgerError> {
    if current < 0 {
        return Err(LedgerError::Store(format!(
            "{label} database has a negative user_version ({current}) — corrupt or \
             not a aggountant database; refusing to open"
        )));
    }
    if current > latest {
        return Err(LedgerError::Store(format!(
            "{label} database schema is v{current} but this binary only knows up to \
             v{latest} — the database was written by a newer aggountant; upgrade the \
             binary (or reset this home, its data is resettable)"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_version(conn: &Connection) -> i32 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    fn column_names(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    const V1: Migration = Migration {
        version: 1,
        sql: "CREATE TABLE IF NOT EXISTS t(a TEXT NOT NULL);",
    };
    const V2: Migration = Migration {
        version: 2,
        sql: "ALTER TABLE t ADD COLUMN b TEXT;",
    };

    #[test]
    fn fresh_database_reaches_the_latest_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        assert_eq!(user_version(&conn), 0);
        apply(&mut conn, "test", &[V1, V2]).unwrap();
        assert_eq!(user_version(&conn), 2);
        assert_eq!(column_names(&conn, "t"), vec!["a", "b"]);
    }

    #[test]
    fn reapplying_an_up_to_date_database_is_a_noop() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn, "test", &[V1, V2]).unwrap();
        conn.execute("INSERT INTO t(a, b) VALUES('x', 'y')", [])
            .unwrap();
        // Second open must not error and must not disturb data.
        apply(&mut conn, "test", &[V1, V2]).unwrap();
        assert_eq!(user_version(&conn), 2);
        let row: (String, String) = conn
            .query_row("SELECT a, b FROM t", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(row, ("x".into(), "y".into()));
    }

    /// The core scenario from the issue: a DB created by an older slice (only v1
    /// applied) gains a *column* when opened by code that knows v2 — the thing
    /// `CREATE TABLE IF NOT EXISTS` could never do.
    #[test]
    fn an_older_database_gains_a_new_column_on_upgrade() {
        let mut conn = Connection::open_in_memory().unwrap();
        // Simulate the old home: only v1 exists at build time.
        apply(&mut conn, "test", &[V1]).unwrap();
        assert_eq!(user_version(&conn), 1);
        conn.execute("INSERT INTO t(a) VALUES('kept')", []).unwrap();
        assert_eq!(column_names(&conn, "t"), vec!["a"]);

        // New binary ships v2; opening the same DB migrates it in place.
        apply(&mut conn, "test", &[V1, V2]).unwrap();
        assert_eq!(user_version(&conn), 2);
        assert_eq!(column_names(&conn, "t"), vec!["a", "b"]);
        // Pre-existing rows survive; the new column defaults to NULL.
        let (a, b): (String, Option<String>) = conn
            .query_row("SELECT a, b FROM t", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(a, "kept");
        assert_eq!(b, None);
    }

    /// A migration whose batch fails *after* an earlier statement succeeded must
    /// roll back the whole batch — the created object AND the version bump — not
    /// leave a partially-applied schema stamped as upgraded.
    #[test]
    fn a_failing_migration_rolls_back_the_whole_batch() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn, "test", &[V1]).unwrap();
        let broken = Migration {
            version: 2,
            // First statement succeeds, second fails: proves batch atomicity, not
            // just "an all-failing migration doesn't advance the version".
            sql: "CREATE TABLE u(x INTEGER); \
                  ALTER TABLE does_not_exist ADD COLUMN b TEXT;",
        };
        let err = apply(&mut conn, "test", &[V1, broken]).unwrap_err();
        assert_eq!(err.error_code(), "STORE_ERROR");
        assert!(err.to_string().contains("migration v2 failed"), "{err}");
        // The version stayed at the last good one...
        assert_eq!(user_version(&conn), 1);
        // ...and the successful first statement of the failed batch rolled back too.
        let u_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='u')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            !u_exists,
            "partial DDL from a failed migration must roll back"
        );
    }

    #[test]
    fn a_misordered_migration_list_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        let bad = [
            V1,
            Migration {
                version: 3, // gap: should be 2
                sql: "SELECT 1;",
            },
        ];
        let err = apply(&mut conn, "test", &bad).unwrap_err();
        assert!(err.to_string().contains("contiguous"), "{err}");
    }

    #[test]
    fn a_newer_database_is_refused_as_a_downgrade() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn, "test", &[V1, V2]).unwrap();
        // Older binary that only knows v1 opening a v2 DB.
        let err = apply(&mut conn, "test", &[V1]).unwrap_err();
        assert!(err.to_string().contains("newer aggountant"), "{err}");
        // The downgrade guard must not have touched the version.
        assert_eq!(user_version(&conn), 2);
    }

    /// A `user_version == 0` DB that already holds application tables predates the
    /// framework and is refused (rather than silently stamped to v1) — the fix for
    /// the "legacy DB missing an inline-added column" timebomb.
    #[test]
    fn a_pre_framework_database_with_tables_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE legacy(a TEXT)", []).unwrap();
        assert_eq!(user_version(&conn), 0);
        let err = apply(&mut conn, "test", &[V1, V2]).unwrap_err();
        assert!(err.to_string().contains("predates"), "{err}");
        // Nothing applied: the DB is left untouched at v0 for the operator to reset.
        assert_eq!(user_version(&conn), 0);
    }

    #[test]
    fn a_negative_user_version_is_refused_as_corruption() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", -5).unwrap();
        let err = apply(&mut conn, "test", &[V1, V2]).unwrap_err();
        assert!(err.to_string().contains("negative user_version"), "{err}");
    }

    #[test]
    fn an_empty_migration_list_is_a_noop() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn, "test", &[]).unwrap();
        assert_eq!(user_version(&conn), 0);
    }
}
