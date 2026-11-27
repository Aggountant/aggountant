//! Expense-claim drafts in the company home (ADR-11 section 1).
//!
//! A draft is intake under ADR-5: it lives at
//! `intake/expense-claims/<claim_id>/draft.json`, outside git and outside the
//! fiscal store, and changes only through the `claim` commands, which call the
//! functions here. Each write reads the draft, applies one validated operation
//! from [`aggountant_domain::expense_claim`], reviews the result against the
//! claimant register, the claimant's other drafts and the sealed vouchers, and
//! replaces the file atomically. The readable claim is rendered from the
//! review on request and never stored.
//!
//! The directory gets its own `.gitignore` the first time a draft is written,
//! because a home's root `.gitignore` may predate the directory and the
//! program never rewrites that file. A company that never records a claim
//! never gets the directory: reading, listing and showing create nothing.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use aggountant_domain::expense_claim::{
    check_claim_id, code, generated_claim_id, review, ClaimDraft, DraftRefusal, DraftReview,
    LineRecord, OtherLine, OtherTrip, Others, Register, TripRecord,
};
use aggountant_ledger::{Ledger, LedgerError};
use aggountant_model::{Hash, Instant};

/// Where drafts live, relative to the company home.
pub const CLAIM_DRAFT_DIR: &str = "intake/expense-claims";
const DRAFT_FILE: &str = "draft.json";
const DISCARD_LOG: &str = "discarded.jsonl";
const IGNORE_RULE: &str = "*";

pub const CLAIM_DRAFT_NOT_FOUND: &str = "CLAIM_DRAFT_NOT_FOUND";
pub const CLAIM_DRAFT_EXISTS: &str = "CLAIM_DRAFT_EXISTS";

/// Why a claim command failed.
#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    /// The input, the target or the stored draft is refused.
    #[error("{code}: {message}")]
    Refused { code: &'static str, message: String },
    /// The draft directory could not be read or written.
    #[error("CLAIM_DRAFT_IO: {0}")]
    Io(String),
}

impl ClaimError {
    pub fn error_code(&self) -> &'static str {
        match self {
            ClaimError::Ledger(e) => e.error_code(),
            ClaimError::Refused { code, .. } => code,
            ClaimError::Io(_) => "CLAIM_DRAFT_IO",
        }
    }
}

impl From<DraftRefusal> for ClaimError {
    fn from(r: DraftRefusal) -> Self {
        ClaimError::Refused {
            code: r.code,
            message: r.message,
        }
    }
}

fn io(context: &str, path: &Path, e: std::io::Error) -> ClaimError {
    ClaimError::Io(format!("{context} {}: {e}", path.display()))
}

fn sha256(bytes: &[u8]) -> Hash {
    Hash::from_bytes(Sha256::digest(bytes).into())
}

// ---------------------------------------------------------------------------
// The draft files
// ---------------------------------------------------------------------------

/// The draft directory of one company home.
pub struct ClaimDrafts {
    root: PathBuf,
}

impl ClaimDrafts {
    pub fn open(home: &Path) -> Self {
        ClaimDrafts {
            root: home.join(CLAIM_DRAFT_DIR),
        }
    }

    fn file(&self, claim_id: &str) -> PathBuf {
        self.root.join(claim_id).join(DRAFT_FILE)
    }

    pub fn exists(&self, claim_id: &str) -> bool {
        self.file(claim_id).is_file()
    }

    fn read_bytes(&self, claim_id: &str) -> Result<Vec<u8>, ClaimError> {
        check_claim_id(claim_id)?;
        let path = self.file(claim_id);
        match fs::read(&path) {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(ClaimError::Refused {
                code: CLAIM_DRAFT_NOT_FOUND,
                message: format!(
                    "no open claim draft {claim_id}; `claim list` shows the open drafts"
                ),
            }),
            Err(e) => Err(io("could not read", &path, e)),
        }
    }

    /// Read and validate a draft, with the sha256 of its file (the revision
    /// hash). A draft changed outside aggountant that no longer validates is
    /// refused rather than used.
    pub fn read(&self, claim_id: &str) -> Result<(ClaimDraft, Hash), ClaimError> {
        let bytes = self.read_bytes(claim_id)?;
        let invalid = |why: String| ClaimError::Refused {
            code: code::CLAIM_DRAFT_INVALID,
            message: format!(
                "claim draft {claim_id} does not validate ({why}); it was changed outside \
                 aggountant. Restore the file or discard the draft with `claim discard`"
            ),
        };
        let draft: ClaimDraft =
            serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
        draft.validate().map_err(|r| invalid(r.to_string()))?;
        if draft.claim_id != claim_id {
            return Err(invalid(format!(
                "it names claim {} but lies in {claim_id}",
                draft.claim_id
            )));
        }
        Ok((draft, sha256(&bytes)))
    }

    /// The ids of the drafts in the directory, sorted; none when the company
    /// has never recorded a claim.
    pub fn ids(&self) -> Result<Vec<String>, ClaimError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io("could not list", &self.root, e)),
        };
        let mut ids = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| io("could not list", &self.root, e))?;
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if check_claim_id(&name).is_ok() && self.exists(&name) {
                ids.push(name);
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Replace the draft's file atomically; returns its revision hash.
    pub fn write(&self, draft: &ClaimDraft) -> Result<Hash, ClaimError> {
        self.ensure_ignored()?;
        let dir = self.root.join(&draft.claim_id);
        fs::create_dir_all(&dir).map_err(|e| io("could not create", &dir, e))?;
        let mut bytes = serde_json::to_vec_pretty(draft).expect("a draft serializes");
        bytes.push(b'\n');
        let path = dir.join(DRAFT_FILE);
        let tmp = dir.join(format!("{DRAFT_FILE}.tmp"));
        {
            let mut file = File::create(&tmp).map_err(|e| io("could not write", &tmp, e))?;
            file.write_all(&bytes)
                .and_then(|()| file.sync_all())
                .map_err(|e| io("could not write", &tmp, e))?;
        }
        fs::rename(&tmp, &path).map_err(|e| io("could not replace", &path, e))?;
        sync_dir(&dir)?;
        Ok(sha256(&bytes))
    }

    /// The directory's own `.gitignore`, created or extended so that its last
    /// rule ignores everything. Operator rules above it are kept.
    fn ensure_ignored(&self) -> Result<(), ClaimError> {
        fs::create_dir_all(&self.root).map_err(|e| io("could not create", &self.root, e))?;
        let path = self.root.join(".gitignore");
        let current = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(io("could not read", &path, e)),
        };
        if current.lines().rfind(|l| !l.trim().is_empty()) == Some(IGNORE_RULE) {
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| io("could not write", &path, e))?;
        let separator = if current.is_empty() || current.ends_with('\n') {
            ""
        } else {
            "\n"
        };
        write!(
            file,
            "{separator}# aggountant: expense-claim drafts hold personal data and never go to git.\n{IGNORE_RULE}\n"
        )
        .and_then(|()| file.sync_all())
        .map_err(|e| io("could not write", &path, e))?;
        sync_dir(&self.root)
    }

    /// Remove a draft, keeping its id, its last revision hash and the reason
    /// in the discard log. A draft that no longer validates can be discarded.
    fn discard(
        &self,
        claim_id: &str,
        reason: &str,
        now: Instant,
    ) -> Result<DiscardRecord, ClaimError> {
        let bytes = self.read_bytes(claim_id)?;
        let record = DiscardRecord {
            claim_id: claim_id.to_string(),
            discarded_at: now,
            last_revision_hash: sha256(&bytes),
            reason: reason.to_string(),
        };
        self.ensure_ignored()?;
        let log = self.root.join(DISCARD_LOG);
        let mut line = serde_json::to_vec(&record).expect("a discard record serializes");
        line.push(b'\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log)
            .map_err(|e| io("could not write", &log, e))?;
        file.write_all(&line)
            .and_then(|()| file.sync_all())
            .map_err(|e| io("could not write", &log, e))?;
        let path = self.file(claim_id);
        fs::remove_file(&path).map_err(|e| io("could not remove", &path, e))?;
        // Only the draft's own empty directory goes; anything else left in it
        // stays for the user to look at.
        let _ = fs::remove_dir(self.root.join(claim_id));
        Ok(record)
    }
}

fn sync_dir(dir: &Path) -> Result<(), ClaimError> {
    File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(|e| io("could not sync", dir, e))
}

/// What a discard keeps: no content, only the draft's identity, its last
/// revision hash and the user's reason.
#[derive(Debug, Clone, Serialize)]
pub struct DiscardRecord {
    pub claim_id: String,
    pub discarded_at: Instant,
    pub last_revision_hash: Hash,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Which draft a write goes to.
pub enum DraftTarget<'a> {
    /// An open draft.
    Claim(&'a str),
    /// A new draft for the claimant; the id is generated from the claimant
    /// and the first content date unless given.
    New {
        employee_id: &'a str,
        claim_id: Option<&'a str>,
    },
}

/// What one write did.
#[derive(Debug, Clone, Serialize)]
pub struct Written {
    pub action: &'static str,
    pub trip_id: Option<String>,
    pub line_id: Option<String>,
}

/// A draft with its review: what `claim show` renders and every write
/// answers with.
#[derive(Debug, Clone, Serialize)]
pub struct DraftView {
    pub draft: ClaimDraft,
    pub revision_hash: Hash,
    /// The draft file, relative to the company home.
    pub path: String,
    pub review: DraftReview,
}

pub fn add_trip(
    home: &Path,
    target: DraftTarget<'_>,
    trip_id: Option<&str>,
    record: TripRecord,
    now: Instant,
) -> Result<(DraftView, Written), ClaimError> {
    write(home, target, now, |d| {
        let id = d.add_trip(trip_id, record, now)?;
        Ok(trip_written("trip_added", id))
    })
}

pub fn revise_trip(
    home: &Path,
    claim_id: &str,
    trip_id: &str,
    record: TripRecord,
    now: Instant,
) -> Result<(DraftView, Written), ClaimError> {
    write(home, DraftTarget::Claim(claim_id), now, |d| {
        d.revise_trip(trip_id, record, now)?;
        Ok(trip_written("trip_revised", trip_id.to_string()))
    })
}

pub fn remove_trip(
    home: &Path,
    claim_id: &str,
    trip_id: &str,
    now: Instant,
) -> Result<(DraftView, Written), ClaimError> {
    write(home, DraftTarget::Claim(claim_id), now, |d| {
        d.remove_trip(trip_id)?;
        Ok(trip_written("trip_removed", trip_id.to_string()))
    })
}

pub fn add_line(
    home: &Path,
    target: DraftTarget<'_>,
    line_id: Option<&str>,
    record: LineRecord,
    now: Instant,
) -> Result<(DraftView, Written), ClaimError> {
    write(home, target, now, |d| {
        let id = d.add_line(line_id, record, now)?;
        Ok(line_written("line_added", id))
    })
}

pub fn revise_line(
    home: &Path,
    claim_id: &str,
    line_id: &str,
    record: LineRecord,
    now: Instant,
) -> Result<(DraftView, Written), ClaimError> {
    write(home, DraftTarget::Claim(claim_id), now, |d| {
        d.revise_line(line_id, record, now)?;
        Ok(line_written("line_revised", line_id.to_string()))
    })
}

pub fn remove_line(
    home: &Path,
    claim_id: &str,
    line_id: &str,
    now: Instant,
) -> Result<(DraftView, Written), ClaimError> {
    write(home, DraftTarget::Claim(claim_id), now, |d| {
        d.remove_line(line_id)?;
        Ok(line_written("line_removed", line_id.to_string()))
    })
}

/// The draft and its review; writes nothing.
pub fn show(home: &Path, claim_id: &str, now: Instant) -> Result<DraftView, ClaimError> {
    let store = ClaimDrafts::open(home);
    let (draft, revision_hash) = store.read(claim_id)?;
    let ledger = Ledger::open(home)?;
    let review = review_draft(&ledger, &store, &draft, now)?;
    Ok(view(draft, revision_hash, review))
}

/// One open draft in `claim list`.
#[derive(Debug, Clone, Serialize)]
pub struct ListedDraft {
    pub claim_id: String,
    /// `None` when the draft does not validate.
    pub employee_id: Option<String>,
    pub revision: Option<u64>,
    pub trips: usize,
    pub lines: usize,
    /// Why the draft does not validate, when it does not.
    pub invalid: Option<String>,
}

/// The open drafts; writes nothing and creates nothing.
pub fn list(home: &Path) -> Result<Vec<ListedDraft>, ClaimError> {
    let store = ClaimDrafts::open(home);
    let mut out = Vec::new();
    for id in store.ids()? {
        out.push(match store.read(&id) {
            Ok((d, _)) => ListedDraft {
                claim_id: id,
                employee_id: Some(d.employee_id),
                revision: Some(d.revision),
                trips: d.trips.len(),
                lines: d.lines.len(),
                invalid: None,
            },
            Err(e) => ListedDraft {
                claim_id: id,
                employee_id: None,
                revision: None,
                trips: 0,
                lines: 0,
                invalid: Some(e.to_string()),
            },
        });
    }
    Ok(out)
}

/// Discard a draft with the user's reason.
pub fn discard(
    home: &Path,
    claim_id: &str,
    reason: &str,
    now: Instant,
) -> Result<DiscardRecord, ClaimError> {
    if reason.trim().is_empty() || reason.chars().any(|c| c.is_control() && c != '\n') {
        return Err(ClaimError::Refused {
            code: code::CLAIM_INPUT_INVALID,
            message: "--reason must say why the draft is discarded".into(),
        });
    }
    ClaimDrafts::open(home).discard(claim_id, reason, now)
}

fn trip_written(action: &'static str, id: String) -> Written {
    Written {
        action,
        trip_id: Some(id),
        line_id: None,
    }
}

fn line_written(action: &'static str, id: String) -> Written {
    Written {
        action,
        trip_id: None,
        line_id: Some(id),
    }
}

fn view(draft: ClaimDraft, revision_hash: Hash, review: DraftReview) -> DraftView {
    DraftView {
        path: format!("{CLAIM_DRAFT_DIR}/{}/{DRAFT_FILE}", draft.claim_id),
        draft,
        revision_hash,
        review,
    }
}

/// Read (or start) the draft, apply one operation, check what the operation
/// refers to, review, and write. Nothing is written when anything refuses.
fn write(
    home: &Path,
    target: DraftTarget<'_>,
    now: Instant,
    op: impl FnOnce(&mut ClaimDraft) -> Result<Written, DraftRefusal>,
) -> Result<(DraftView, Written), ClaimError> {
    let store = ClaimDrafts::open(home);
    let ledger = Ledger::open(home)?;
    let (mut draft, generate_id) = match target {
        DraftTarget::Claim(id) => (store.read(id)?.0, false),
        DraftTarget::New {
            employee_id,
            claim_id,
        } => {
            ledger.get_employee(employee_id)?;
            if let Some(id) = claim_id {
                check_claim_id(id)?;
                if store.exists(id) {
                    return Err(ClaimError::Refused {
                        code: CLAIM_DRAFT_EXISTS,
                        message: format!(
                            "claim draft {id} already exists; add to it with --claim {id}"
                        ),
                    });
                }
            }
            (
                ClaimDraft::new(claim_id.unwrap_or("new"), employee_id, now)?,
                claim_id.is_none(),
            )
        }
    };
    let written = op(&mut draft)?;
    if generate_id {
        let first = draft
            .first_content_date()
            .expect("a new draft has the content just added");
        draft.claim_id = generated_claim_id(&draft.employee_id, first, |id| store.exists(id));
    }
    for hash in draft.artifacts() {
        if !ledger.artifact_exists(&hash)? {
            return Err(ClaimError::Refused {
                code: "ARTIFACT_NOT_FOUND",
                message: format!(
                    "artifact {hash} is not stored in this home; attach the file first with \
                     `artifact attach --file <file>` and use the hash it returns"
                ),
            });
        }
    }
    let review = review_draft(&ledger, &store, &draft, now)?;
    // A trip the computation cannot take at all is refused at the write; a
    // trip that became uncomputable later (a register change) only shows as
    // a refusal, so that the draft stays writable.
    if let Some(trip_id) = &written.trip_id {
        if let Some(rejected) = review
            .refusals
            .iter()
            .find(|r| r.code == code::TRAVEL_INPUT_INVALID && r.trip_id.as_ref() == Some(trip_id))
        {
            return Err(ClaimError::Refused {
                code: code::TRAVEL_INPUT_INVALID,
                message: rejected.message.clone(),
            });
        }
    }
    draft.validate()?;
    let revision_hash = store.write(&draft)?;
    Ok((view(draft, revision_hash, review), written))
}

/// Review the draft against the claimant's register, the claimant's other
/// open drafts, every other draft's receipts and the sealed vouchers.
fn review_draft(
    ledger: &Ledger,
    store: &ClaimDrafts,
    draft: &ClaimDraft,
    now: Instant,
) -> Result<DraftReview, ClaimError> {
    let register = ledger.claimant_history(&draft.employee_id)?;
    let mut others = Others::default();
    for id in store.ids()? {
        if id == draft.claim_id {
            continue;
        }
        let other = match store.read(&id) {
            Ok((other, _)) => other,
            Err(_) => {
                others.unreadable.push(id);
                continue;
            }
        };
        if other.employee_id == draft.employee_id {
            for (trip_id, input) in other.trip_inputs() {
                if let Ok(input) = input {
                    others.trips.push(OtherTrip {
                        claim_id: id.clone(),
                        trip_id,
                        start: input.start,
                        end: input.end,
                    });
                }
            }
        }
        for line in &other.lines {
            others.lines.push(OtherLine {
                claim_id: id.clone(),
                line_id: line.line_id.clone(),
                receipt_hash: line.record.receipt_hash,
                receipt_date: line.record.receipt_date,
                amount: line.record.amount.clone(),
            });
        }
    }
    let receipts = draft
        .lines
        .iter()
        .filter_map(|l| l.record.receipt_hash)
        .collect();
    others.booked = ledger.vouchers_attaching(&receipts)?;
    Ok(review(
        draft,
        &Register {
            places: &register.places,
            facts: &register.facts,
        },
        &others,
        now,
    ))
}
