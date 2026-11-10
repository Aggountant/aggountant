//! **F13 obligations** (S5). An obligation is an open commitment with a deadline (e.g.
//! *"ajopäiväkirja puuttuu"*, a Tulorekisteri deadline) that a Run **creates** and that
//! **survives the Run's close**. It lives in the git-native process namespace, not in
//! the fiscal ledger (`candidate.md §3`: "Obligation elää process-tilassa; Run luo/sulkee
//! sen"), and it **blocks `close-period`** for its period until resolved (the
//! `OPEN_OBLIGATION` invariant, `candidate.md §4`).
//!
//! Each obligation is a single JSON file under `<home>/process/obligations/<id>.json`,
//! written atomically (temp → fsync → rename). Status is a mutable field — this is
//! process state, not the fiscal hash chain, and there is exactly one local user (v1),
//! so a create/resolve rewrite is safe. `close-period` reads the *current* status.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use aggountant_model::PeriodId;

use crate::error::ProcessError;

static OBL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Whether an obligation is still outstanding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationStatus {
    Open,
    Resolved,
}

/// One open commitment created by a Run, blocking `close-period` for its period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    pub id: String,
    /// The fiscal period this obligation blocks (its `close-period` cannot proceed).
    pub period: String,
    /// A short machine tag, e.g. `"ajopaivakirja-missing"`.
    pub kind: String,
    /// Human-readable description shown to the driver.
    pub description: String,
    /// The deadline (`YYYY-MM-DD`) by which it must be resolved.
    pub deadline: String,
    /// The Run that created it (provenance).
    pub created_by_run: String,
    pub status: ObligationStatus,
}

/// The git-native obligation store (`<home>/process/obligations`).
pub struct Obligations {
    root: PathBuf,
}

impl Obligations {
    pub fn open(home: &Path) -> Self {
        Obligations {
            root: home.join("process").join("obligations"),
        }
    }

    fn path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    /// Create a new open obligation (minting a unique id) and persist it durably.
    pub fn create(
        &self,
        period: &PeriodId,
        kind: &str,
        description: &str,
        deadline: &str,
        created_by_run: &str,
    ) -> Result<Obligation, ProcessError> {
        self.ensure(
            &new_obligation_id(),
            period,
            kind,
            description,
            deadline,
            created_by_run,
        )
    }

    /// **Idempotent** create with a caller-supplied deterministic id. A retried workflow
    /// effect (a resume after a crash between `create` and `EffectSucceeded`) re-uses the
    /// same id and does not mint a duplicate obligation that would permanently block
    /// `close-period`. If a record already exists for `id`, it is returned unchanged.
    #[allow(clippy::too_many_arguments)]
    pub fn ensure(
        &self,
        id: &str,
        period: &PeriodId,
        kind: &str,
        description: &str,
        deadline: &str,
        created_by_run: &str,
    ) -> Result<Obligation, ProcessError> {
        if self.path(id).exists() {
            return self.get(id);
        }
        let obligation = Obligation {
            id: id.to_string(),
            period: period.to_string(),
            kind: kind.to_string(),
            description: description.to_string(),
            deadline: deadline.to_string(),
            created_by_run: created_by_run.to_string(),
            status: ObligationStatus::Open,
        };
        self.write(&obligation)?;
        Ok(obligation)
    }

    fn write(&self, o: &Obligation) -> Result<(), ProcessError> {
        fs::create_dir_all(&self.root).map_err(|e| ProcessError::Store(e.to_string()))?;
        let bytes = serde_json::to_vec_pretty(o).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = self.path(&o.id);
        let tmp = path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| ProcessError::Store(e.to_string()))?;
            f.write_all(&bytes)
                .map_err(|e| ProcessError::Store(e.to_string()))?;
            f.sync_all()
                .map_err(|e| ProcessError::Store(e.to_string()))?;
        }
        fs::rename(&tmp, &path).map_err(|e| ProcessError::Store(e.to_string()))?;
        Ok(())
    }

    /// Load one obligation by id.
    pub fn get(&self, id: &str) -> Result<Obligation, ProcessError> {
        let bytes = fs::read(self.path(id))
            .map_err(|_| ProcessError::RunNotFound(format!("obligation {id}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("obligation {id} corrupt: {e}")))
    }

    /// All obligations, id-sorted.
    pub fn list(&self) -> Result<Vec<Obligation>, ProcessError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut ids: Vec<String> = fs::read_dir(&self.root)
            .map_err(|e| ProcessError::Store(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
            .collect();
        ids.sort();
        ids.iter().map(|id| self.get(id)).collect()
    }

    /// Open obligations whose calendar span overlaps `period`. Stored ids are
    /// strictly parsed; malformed process state fails visibly rather than silently
    /// dropping a bookkeeping blocker.
    pub fn open_for_period(&self, period: &PeriodId) -> Result<Vec<Obligation>, ProcessError> {
        let mut open = Vec::new();
        for obligation in self.list()? {
            if obligation.status != ObligationStatus::Open {
                continue;
            }
            let obligation_period = PeriodId::parse(&obligation.period).map_err(|error| {
                ProcessError::JournalCorrupt(format!(
                    "obligation {} has invalid period {:?}: {error}",
                    obligation.id, obligation.period
                ))
            })?;
            if period.overlaps(&obligation_period) {
                open.push(obligation);
            }
        }
        Ok(open)
    }

    /// Resolve an open obligation (the missing item was supplied). Idempotent.
    pub fn resolve(&self, id: &str) -> Result<Obligation, ProcessError> {
        let mut o = self.get(id)?;
        if o.status == ObligationStatus::Resolved {
            return Ok(o);
        }
        o.status = ObligationStatus::Resolved;
        self.write(&o)?;
        Ok(o)
    }
}

fn new_obligation_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = OBL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    format!("obl-{nanos:x}-{pid:x}-{n:x}")
}
