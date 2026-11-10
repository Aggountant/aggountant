//! The git-native, append-only Run journal (ADR-1 process namespace: *"append-only
//! immutable event files, `fold(events)`, temp→fsync→rename, event-hash-chain"*).
//!
//! Each event is written as its own immutable, zero-padded file under
//! `<run>/events/NNNNNN.json`, created via **temp file → fsync → atomic rename** so
//! a crash mid-write never leaves a visible partial event (the half-written temp is
//! simply ignored on recovery). Events carry a hash chain
//! (`hash = SHA256(prev ‖ framed(event))`) so a tampered or reordered journal is
//! detectable on fold — the process-side analogue of the fiscal voucher chain.
//! "The journal is the truth": Run state is a `fold` of these events, never a
//! materialized status field (`candidate.md §3`).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::ProcessError;

/// Schema version stamped on every event (`candidate.md §9`, journal event
/// versioning). Bump when the payload shape changes; upcasters live here.
pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// The typed payload of a journal event. `event_type` (the CLI-visible tag) is
/// derived from the variant, so the two can never drift.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventPayload {
    /// The first event: pins the workflow version + policy set + period (the
    /// resume-time version check reads this).
    RunStarted {
        workflow: String,
        workflow_version: String,
        git_commit: String,
        policy_set: String,
        period: String,
    },
    EffectPlanned {
        effect_key: String,
        effect_type: String,
    },
    EffectExecuting {
        effect_key: String,
    },
    EffectSucceeded {
        effect_key: String,
        note: String,
    },
    EffectRejected {
        effect_key: String,
        error_code: String,
    },
    /// An effect was terminally **skipped** — a legitimate non-success (S5): a discarded
    /// match, or a seal that will never run because the transaction was clarified rather
    /// than booked. Terminal like `Succeeded` for run-completion, but distinct so the
    /// fold does not claim the effect produced its output.
    EffectSkipped {
        effect_key: String,
        reason: String,
    },
    /// An effect parked on a durable `Effect::Wait` (S5): a rate limit recorded a
    /// `resume_after` deadline (absolute unix seconds). `resume` re-drives the effect
    /// once an injected/system clock passes the deadline — the wait is durable process
    /// state, not an in-memory sleep, so it survives a crash across the wait.
    EffectWaiting {
        effect_key: String,
        resume_after_epoch_secs: u64,
        reason: String,
    },
    /// One outbound `consult-llm` turn re-projected + re-bound its disclosure (S5/RF1).
    /// Recorded per outbound message so a multi-turn consult proves each re-disclosure
    /// binds to the exact projected bytes that leave (`redaction-strategy-options.md`
    /// §4 honest-cost #3). `turn` is 1-based.
    DisclosureBound {
        turn: u32,
        sent_hash: String,
    },
    /// Resume reconciled a non-terminal effect to the authoritative fiscal state
    /// (ADR-6 §3, "fiscal wins").
    EffectReconciled {
        effect_key: String,
        note: String,
    },
    GateOpened {
        gate_id: String,
        gate_kind: String,
        snapshot_hash: String,
    },
    GateAnswered {
        gate_id: String,
        answer: String,
    },
    WorkflowMigrated {
        from_version: String,
        to_version: String,
    },
}

impl EventPayload {
    /// The snake_case discriminant, also stored as `event_type`.
    pub fn event_type(&self) -> &'static str {
        match self {
            EventPayload::RunStarted { .. } => "run_started",
            EventPayload::EffectPlanned { .. } => "effect_planned",
            EventPayload::EffectExecuting { .. } => "effect_executing",
            EventPayload::EffectSucceeded { .. } => "effect_succeeded",
            EventPayload::EffectRejected { .. } => "effect_rejected",
            EventPayload::EffectSkipped { .. } => "effect_skipped",
            EventPayload::EffectWaiting { .. } => "effect_waiting",
            EventPayload::DisclosureBound { .. } => "disclosure_bound",
            EventPayload::EffectReconciled { .. } => "effect_reconciled",
            EventPayload::GateOpened { .. } => "gate_opened",
            EventPayload::GateAnswered { .. } => "gate_answered",
            EventPayload::WorkflowMigrated { .. } => "workflow_migrated",
        }
    }
}

/// One recorded journal event with its chain glue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEvent {
    pub seq: u64,
    pub event_type: String,
    pub schema_version: u32,
    pub ts: String,
    pub prev_hash: String,
    pub hash: String,
    pub payload: EventPayload,
}

/// The append-only event store for a single run.
pub struct Journal {
    events_dir: PathBuf,
}

impl Journal {
    pub fn new(run_dir: &Path) -> Self {
        Journal {
            events_dir: run_dir.join("events"),
        }
    }

    pub fn exists(&self) -> bool {
        self.events_dir.exists()
    }

    /// Append one event, chained onto the current head. Durable before return
    /// (temp → fsync → atomic rename).
    pub fn append(&self, payload: EventPayload) -> Result<JournalEvent, ProcessError> {
        fs::create_dir_all(&self.events_dir).map_err(|e| ProcessError::Store(e.to_string()))?;
        let existing = self.read_all()?;
        let seq = existing.len() as u64 + 1;
        let prev_hash = existing
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(zero_hex);
        let ts = chrono::Utc::now().to_rfc3339();
        let event_type = payload.event_type().to_string();
        let payload_json =
            serde_json::to_vec(&payload).map_err(|e| ProcessError::Store(e.to_string()))?;
        let hash = compute_hash(
            &prev_hash,
            &event_type,
            EVENT_SCHEMA_VERSION,
            &ts,
            &payload_json,
        );
        let event = JournalEvent {
            seq,
            event_type,
            schema_version: EVENT_SCHEMA_VERSION,
            ts,
            prev_hash,
            hash,
            payload,
        };

        let final_path = self.events_dir.join(format!("{seq:06}.json"));
        let tmp_path = self.events_dir.join(format!("{seq:06}.json.tmp"));
        let bytes =
            serde_json::to_vec_pretty(&event).map_err(|e| ProcessError::Store(e.to_string()))?;
        {
            let mut f =
                fs::File::create(&tmp_path).map_err(|e| ProcessError::Store(e.to_string()))?;
            f.write_all(&bytes)
                .map_err(|e| ProcessError::Store(e.to_string()))?;
            f.sync_all()
                .map_err(|e| ProcessError::Store(e.to_string()))?;
        }
        // Exclusive publish: hard-link the temp into its final name, which FAILS if
        // that sequence already exists (a concurrent appender the lease should have
        // prevented) instead of silently replacing it the way `fs::rename` would —
        // so a lost event surfaces as a loud conflict, never silent data loss.
        fs::hard_link(&tmp_path, &final_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                ProcessError::JournalCorrupt(format!(
                    "event {seq} already exists — concurrent append without an exclusive lease"
                ))
            } else {
                ProcessError::Store(e.to_string())
            }
        })?;
        fs::remove_file(&tmp_path).ok();
        // Best-effort durability of the new directory entry (POSIX atomic publish
        // is only durable once the directory itself is synced).
        if let Ok(dir) = fs::File::open(&self.events_dir) {
            let _ = dir.sync_all();
        }
        Ok(event)
    }

    /// Read and integrity-check the whole journal in seq order (the `fold` input).
    pub fn read_all(&self) -> Result<Vec<JournalEvent>, ProcessError> {
        if !self.events_dir.exists() {
            return Ok(Vec::new());
        }
        let mut files: Vec<PathBuf> = fs::read_dir(&self.events_dir)
            .map_err(|e| ProcessError::Store(e.to_string()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();

        let mut out: Vec<JournalEvent> = Vec::new();
        let mut prev = zero_hex();
        for (i, path) in files.iter().enumerate() {
            let bytes = fs::read(path).map_err(|e| ProcessError::Store(e.to_string()))?;
            let event: JournalEvent = serde_json::from_slice(&bytes)
                .map_err(|e| ProcessError::JournalCorrupt(format!("{}: {e}", path.display())))?;
            let expected_seq = i as u64 + 1;
            if event.seq != expected_seq {
                return Err(ProcessError::JournalCorrupt(format!(
                    "seq gap: file {} holds seq {}",
                    path.display(),
                    event.seq
                )));
            }
            // The stored `event_type` must agree with the typed payload (a tamper
            // that swaps the tag is caught), and the schema version must be one we
            // understand (no silent misread of a future encoding).
            if event.event_type != event.payload.event_type() {
                return Err(ProcessError::JournalCorrupt(format!(
                    "event {} event_type {:?} disagrees with its payload ({})",
                    event.seq,
                    event.event_type,
                    event.payload.event_type()
                )));
            }
            if event.schema_version != EVENT_SCHEMA_VERSION {
                return Err(ProcessError::JournalCorrupt(format!(
                    "event {} has unsupported schema_version {} (this build speaks {EVENT_SCHEMA_VERSION})",
                    event.seq, event.schema_version
                )));
            }
            if event.prev_hash != prev {
                return Err(ProcessError::JournalCorrupt(format!(
                    "event {} prev_hash does not chain",
                    event.seq
                )));
            }
            let payload_json = serde_json::to_vec(&event.payload)
                .map_err(|e| ProcessError::Store(e.to_string()))?;
            let recomputed = compute_hash(
                &event.prev_hash,
                &event.event_type,
                event.schema_version,
                &event.ts,
                &payload_json,
            );
            if recomputed != event.hash {
                return Err(ProcessError::JournalCorrupt(format!(
                    "event {} hash mismatch (journal tampered)",
                    event.seq
                )));
            }
            prev = event.hash.clone();
            out.push(event);
        }
        Ok(out)
    }
}

fn zero_hex() -> String {
    "0".repeat(64)
}

/// The event chain hash. Length-prefixed framing (like the fiscal chains) so no
/// field-boundary ambiguity; kept identical on write and on read-verify.
fn compute_hash(
    prev_hex: &str,
    event_type: &str,
    schema_version: u32,
    ts: &str,
    payload_json: &[u8],
) -> String {
    let mut h = Sha256::new();
    h.update(prev_hex.as_bytes());
    h.update(schema_version.to_be_bytes());
    for field in [event_type.as_bytes(), ts.as_bytes(), payload_json] {
        h.update((field.len() as u64).to_be_bytes());
        h.update(field);
    }
    hex::encode(h.finalize())
}
