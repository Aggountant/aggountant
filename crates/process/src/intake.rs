//! A **minimal typed intake home** for RF2 match candidates (S5, Fork A option b). The
//! full ADR-5 intake stage (F17: `intake list|show|revise|discard|prepare-voucher`) is
//! its own later slice; this slice builds just enough of it that a hunted match lands in
//! a **typed** record — *not* an under-defined scratch namespace (`s5-spec-ahead.md`
//! done-criteria / Fork A).
//!
//! The load-bearing privacy property: a [`MatchCandidate`] holds only the **derived**
//! match fields + a pointer to the stored **attachment** (a `sha256` artifact ref) and
//! provenance. The **raw** email/marketplace body never lands here — it is used
//! transiently to derive the match and produce the artifact, then discarded when the Run
//! ends (`s5-spec-ahead.md` §2.4 privacy boundary).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ProcessError;

/// Where a match candidate stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    /// Awaiting the human's `ApproveMatch` decision.
    Pending,
    /// Approved and sealed into a voucher.
    Matched,
    /// Discarded (a false positive / unexplained).
    Discarded,
}

/// A typed match candidate: the evidence a hunt found for one bank transaction. No raw
/// email/marketplace body — only the derived fields + the artifact pointer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchCandidate {
    pub id: String,
    pub run_id: String,
    pub bank_account: String,
    pub txn_id: String,
    pub counterparty: String,
    pub amount: String,
    pub date: String,
    /// Which pull surfaced it (`email` / `kauppatili`).
    pub source: String,
    /// The `sha256` of the stored attachment blob (the only thing kept from the raw pull).
    pub artifact_ref: String,
    pub status: MatchStatus,
}

/// The git-native intake store (`<home>/intake`).
pub struct Intake {
    root: PathBuf,
}

impl Intake {
    pub fn open(home: &Path) -> Self {
        Intake {
            root: home.join("intake"),
        }
    }

    /// The on-disk filename for a candidate id: the SHA-256 of the (caller-controlled)
    /// semantic id, so `bank_account`/`txn_id` values containing `/`, `..`, or platform
    /// separators can never write outside `<home>/intake`. The semantic id is kept
    /// inside the JSON, and `list`/`get` round-trip through it.
    fn path(&self, id: &str) -> PathBuf {
        let mut h = Sha256::new();
        h.update(id.as_bytes());
        self.root
            .join(format!("{}.json", hex::encode(h.finalize())))
    }

    /// The candidate id for a bank transaction — the idempotency key
    /// `voucher-match:<bank-account>:<txn-id>` (dedupes a re-hunted transaction).
    pub fn candidate_id(bank_account: &str, txn_id: &str) -> String {
        format!("voucher-match:{bank_account}:{txn_id}")
    }

    fn write(&self, c: &MatchCandidate) -> Result<(), ProcessError> {
        fs::create_dir_all(&self.root).map_err(|e| ProcessError::Store(e.to_string()))?;
        let bytes = serde_json::to_vec_pretty(c).map_err(|e| ProcessError::Store(e.to_string()))?;
        let path = self.path(&c.id);
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

    /// Upsert a pending match candidate (idempotent on its `voucher-match:<acct>:<txn>` id).
    #[allow(clippy::too_many_arguments)]
    pub fn upsert(
        &self,
        run_id: &str,
        bank_account: &str,
        txn_id: &str,
        counterparty: &str,
        amount: &str,
        date: &str,
        source: &str,
        artifact_ref: &str,
    ) -> Result<MatchCandidate, ProcessError> {
        let id = Self::candidate_id(bank_account, txn_id);
        let candidate = MatchCandidate {
            id: id.clone(),
            run_id: run_id.to_string(),
            bank_account: bank_account.to_string(),
            txn_id: txn_id.to_string(),
            counterparty: counterparty.to_string(),
            amount: amount.to_string(),
            date: date.to_string(),
            source: source.to_string(),
            artifact_ref: artifact_ref.to_string(),
            status: MatchStatus::Pending,
        };
        self.write(&candidate)?;
        Ok(candidate)
    }

    pub fn get(&self, id: &str) -> Result<MatchCandidate, ProcessError> {
        let bytes = fs::read(self.path(id))
            .map_err(|_| ProcessError::RunNotFound(format!("intake candidate {id}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| ProcessError::JournalCorrupt(format!("intake {id} corrupt: {e}")))
    }

    pub fn list(&self) -> Result<Vec<MatchCandidate>, ProcessError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        // Filenames are opaque hashes, so read each record and recover its semantic id
        // from the JSON; sort by that for a stable order.
        let mut out: Vec<MatchCandidate> = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|e| ProcessError::Store(e.to_string()))? {
            let path = entry
                .map_err(|e| ProcessError::Store(e.to_string()))?
                .path();
            if path.extension().is_some_and(|x| x == "json") {
                let bytes = fs::read(&path).map_err(|e| ProcessError::Store(e.to_string()))?;
                let c: MatchCandidate = serde_json::from_slice(&bytes).map_err(|e| {
                    ProcessError::JournalCorrupt(format!("intake {} corrupt: {e}", path.display()))
                })?;
                out.push(c);
            }
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// Mark a candidate matched (sealed into a voucher). Idempotent.
    pub fn mark_matched(&self, id: &str) -> Result<MatchCandidate, ProcessError> {
        let mut c = self.get(id)?;
        c.status = MatchStatus::Matched;
        self.write(&c)?;
        Ok(c)
    }

    /// Mark a candidate discarded (a false positive / declined match). Idempotent.
    pub fn mark_discarded(&self, id: &str) -> Result<MatchCandidate, ProcessError> {
        let mut c = self.get(id)?;
        c.status = MatchStatus::Discarded;
        self.write(&c)?;
        Ok(c)
    }
}
