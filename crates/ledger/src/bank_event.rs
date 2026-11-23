//! Bank-transaction identity for statement imports (issue `camt-event-identity`).
//!
//! One bank transaction can arrive in several statement exports, for example
//! when two yearly exports overlap. Four things are kept apart:
//!
//! - the **event key**, the bank's entry reference and account, which is also
//!   the voucher's natural key and finds the transaction in another export;
//! - the **source fingerprint** of the bank's facts before booking, which
//!   detects a transaction the bank has changed;
//! - the **projection hash** of the booking (accounts, amounts, VAT, dates,
//!   description) without the statement file, which detects a changed proposal;
//! - the **source observations**, every retained statement file the
//!   transaction was seen in.
//!
//! The statement file stays the voucher's attachment and part of its request
//! hash, but only the projection hash decides whether two sightings book the
//! same thing. A batch is resolved in one fiscal transaction: either every
//! transaction is a new voucher, a duplicate observation or a binding to a
//! voucher sealed outside the import, and all of it is written, or something
//! conflicts and nothing is.
//!
//! A voucher sealed outside the import (generic `voucher seal`) carries no event
//! key, so before a new voucher is sealed for a transaction every voucher not
//! bound to a bank event is searched for one that may already book it: one that
//! names the bank reference, or posts the same amount on the same side of the
//! same bank account within [`DATE_TOLERANCE_DAYS`] of the bank's dates. A
//! candidate stops the batch until the user binds the transaction to it or
//! states that it does not book the transaction. A false stop costs a look; a
//! missed one books the money twice.

use serde::Serialize;

use aggountant_model::{
    AccountId, Date, GeneratedVatLeg, Hash, Instant, Money, PeriodId, Posting, Side,
    VatExpansionTreatment, VoucherBody, VoucherDraft,
};

use crate::store::{
    ArtifactStore, BankBatchRequest, BankBatchResult, BankEventBackfill, BankEventRequest,
    BankEventResolution, BuiltVoucher, FiscalStore, MasterDataRefStore, PeriodGate,
    RejectionRecord, StoredProvenance, StoredVoucher,
};
use crate::{sha256_hex, Ledger, LedgerError, SealedVoucher, ValidationWarning, SOFTWARE_VERSION};

/// Why a bank transaction of an import batch cannot be accepted. Each is a
/// conflict with what the books already hold for the same event key, so the
/// whole batch stops before it writes anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankEventConflictCode {
    /// The bank's facts for the transaction (amount, dates, parties, booking
    /// status) differ from those of the sealed voucher.
    SourceFactsChanged,
    /// The bank's facts are unchanged but the proposed booking differs.
    PostingProjectionChanged,
    /// A voucher holds the event key, but the transaction it was sealed from
    /// could not be read back from its retained statement.
    SourceIdentityUnknown,
    /// A voucher sealed outside the import, not bound to any bank event, may
    /// already book the transaction.
    BankEntryPossiblyBooked,
    /// The voucher named for a binding does not book the transaction (no
    /// matching bank leg), is bound to another transaction, or the transaction
    /// is bound to another voucher.
    BankVoucherMismatch,
}

impl BankEventConflictCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceFactsChanged => "SOURCE_FACTS_CHANGED",
            Self::PostingProjectionChanged => "POSTING_PROJECTION_CHANGED",
            Self::SourceIdentityUnknown => "SOURCE_IDENTITY_UNKNOWN",
            Self::BankEntryPossiblyBooked => "BANK_ENTRY_POSSIBLY_BOOKED",
            Self::BankVoucherMismatch => "BANK_VOUCHER_MISMATCH",
        }
    }
}

/// Days either side of the bank's booking or value date within which a
/// voucher's posting or document date counts as the transaction's date, both
/// when searching for a voucher that may already book it and when binding one.
/// Generous on purpose: a hand-booked payment is often dated a few days off.
pub const DATE_TOLERANCE_DAYS: i32 = 7;

/// One bank transaction of an import batch.
pub struct BankEventEntry {
    /// The bank's entry reference and account; sealed as the natural key.
    pub event_key: String,
    pub source_fingerprint: String,
    /// What the bank leg of a voucher booking this transaction looks like.
    pub facts: BankEntryFacts,
    pub booking: BankEventBooking,
}

/// How an entry is to be booked when its event key is not yet in the books.
// One per statement line, built once and read in place: boxing the draft would
// buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum BankEventBooking {
    /// Seal this voucher, with the statement file as its attachment.
    New {
        draft: VoucherDraft,
        /// Vouchers the user has checked do not book the transaction; they are
        /// not reported as candidates for it.
        not_booked_by: Vec<u64>,
    },
    /// Bind the event key to this voucher, already sealed outside the import.
    Existing { voucher_number: u64 },
}

impl BankEventEntry {
    /// The proposed voucher, if the entry seals one.
    pub fn draft(&self) -> Option<&VoucherDraft> {
        match &self.booking {
            BankEventBooking::New { draft, .. } => Some(draft),
            BankEventBooking::Existing { .. } => None,
        }
    }
}

/// The bank's facts a voucher's bank leg is compared with.
pub struct BankEntryFacts {
    /// The bank's own reference for the entry.
    pub bank_ref: String,
    /// The ledger account the statement's account is kept on.
    pub bank_account: AccountId,
    /// The entry amount, positive.
    pub amount: Money,
    /// The side the bank account is posted on: debit for money in.
    pub bank_side: Side,
    pub booking_date: Date,
    pub value_date: Option<Date>,
}

impl BankEntryFacts {
    fn near(&self, date: Date) -> bool {
        std::iter::once(self.booking_date)
            .chain(self.value_date)
            .any(|bank_date| {
                date.inner()
                    .since(bank_date.inner())
                    .is_ok_and(|span| span.get_days().abs() <= DATE_TOLERANCE_DAYS)
            })
    }

    /// Whether `body` posts the entry's amount on the entry's side of its bank
    /// account, dated (posting or document date) near the bank's dates.
    fn booked_by(&self, body: &VoucherBody) -> bool {
        let dated = self.near(body.posting_date()) || self.near(body.document_date());
        dated
            && body.postings().iter().any(|p| {
                p.account == self.bank_account
                    && p.side == self.bank_side
                    && p.amount.currency() == self.amount.currency()
                    && p.amount.amount() == self.amount.amount()
            })
    }

    /// Whether `text` contains the bank reference as a whole token (not as a
    /// part of a longer alphanumeric run).
    fn named_in(&self, text: &str) -> bool {
        let reference = self.bank_ref.as_str();
        if reference.is_empty() {
            return false;
        }
        text.match_indices(reference).any(|(start, _)| {
            let before = text[..start].chars().next_back();
            let after = text[start + reference.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
        })
    }

    fn describe(&self) -> String {
        let side = match self.bank_side {
            Side::Debit => "debit",
            Side::Credit => "credit",
        };
        let value_date = self
            .value_date
            .map(|d| format!(", value date {d}"))
            .unwrap_or_default();
        format!(
            "{} {} on the {side} side of account {}, booked {}{value_date}",
            self.amount.amount_str(),
            self.amount.currency(),
            self.bank_account,
            self.booking_date
        )
    }
}

/// Why a voucher not bound to a bank event is a candidate for a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateEvidence {
    /// Its natural key or description names the bank reference.
    BankReference,
    /// It posts the same amount on the same side of the same bank account near
    /// the bank's dates.
    AccountAmountDate,
}

/// A transaction the statement lists with a status other than booked.
pub struct UnbookedBankEvent {
    pub event_key: String,
    pub status: String,
}

/// The source fingerprint of a voucher sealed before bank events were recorded,
/// as the caller read it from the statement the voucher retained.
pub struct LegacyBankIdentity {
    pub event_key: String,
    pub source_fingerprint: String,
}

/// A voucher sealed under a bank event key before bank events were recorded.
#[derive(Debug, Clone)]
pub struct LegacyBankVoucher {
    pub event_key: String,
    pub voucher_number: u64,
    pub attachment_ref: Option<Hash>,
}

/// Every transaction of one statement file.
pub struct BankBatch<'a> {
    /// Content address of the statement file every entry was seen in.
    pub artifact: Hash,
    pub entries: &'a [BankEventEntry],
    pub unbooked: &'a [UnbookedBankEvent],
    pub backfill: &'a [LegacyBankIdentity],
    pub actor: &'a str,
    pub policy_set: &'a str,
    /// The prefix of every event key the import writes. A voucher under such a
    /// key books its own transaction and is never a candidate for another.
    pub event_key_prefix: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BankEventOutcome {
    NewVoucher,
    DuplicateSourceObservation,
    /// Bound to a voucher sealed outside the import; nothing was sealed.
    BoundExistingVoucher,
}

#[derive(Debug, Clone, Serialize)]
pub struct BankEventResult {
    pub event_key: String,
    pub outcome: BankEventOutcome,
    /// The sealed voucher. A preview names no number for a new voucher (the number
    /// is allocated only by the seal).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voucher: Option<SealedVoucher>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BankEventConflict {
    pub event_key: String,
    pub error_code: &'static str,
    #[serde(skip)]
    pub code: BankEventConflictCode,
    /// The voucher already sealed for the event key, the candidate that may
    /// book it, or the voucher named for its binding.
    pub voucher_number: u64,
    /// Why the voucher is a candidate (`BANK_ENTRY_POSSIBLY_BOOKED` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<CandidateEvidence>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BankBatchReport {
    /// One result per entry that does not conflict, in order: every entry when
    /// `conflicts` is empty.
    pub results: Vec<BankEventResult>,
    pub conflicts: Vec<BankEventConflict>,
    /// Vouchers sealed before bank events whose identity this batch recorded
    /// (in a preview or a refused batch: would have recorded).
    pub backfilled_count: usize,
    /// Whether the batch was written; never with a conflict or in a preview.
    pub written: bool,
}

impl BankBatchReport {
    pub fn count(&self, outcome: BankEventOutcome) -> usize {
        self.results.iter().filter(|r| r.outcome == outcome).count()
    }
}

/// One recorded sighting of a bank transaction in a retained statement file.
#[derive(Debug, Clone, Serialize)]
pub struct BankSourceObservation {
    pub attachment_ref: String,
    pub event_key: String,
    pub voucher_number: u64,
    pub observed_at: String,
    /// Recorded from the statement of a voucher sealed before bank events existed.
    pub backfilled: bool,
}

/// What a booking of a bank transaction consists of: dates, description,
/// postings (accounts, amounts, sides, VAT codes) and the VAT treatment with its
/// resolved rate and generated legs. Deliberately left out: the attachment (one
/// transaction arrives in several export files) and the software state a VAT
/// expansion records (`policy_set`, `vat_rates_hash`, expansion `version`),
/// which change with a release while the booking does not.
#[derive(Serialize)]
struct Projection<'a> {
    document_date: Date,
    posting_date: Date,
    description: &'a str,
    postings: &'a [Posting],
    #[serde(skip_serializing_if = "Option::is_none")]
    buyer_vat_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    corrects_vat_period: Option<&'a PeriodId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vat: Option<VatProjection<'a>>,
}

#[derive(Serialize)]
struct VatProjection<'a> {
    treatment: &'a VatExpansionTreatment,
    source_amount: &'a Money,
    net_account: &'a AccountId,
    contra_account: &'a AccountId,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved_percent: Option<&'a str>,
    generated_vat_legs: &'a [GeneratedVatLeg],
}

/// The hash that decides whether two sightings of a transaction book the same
/// thing. Stored in `bank_event`, so what [`Projection`] covers is frozen: a
/// change re-keys every booked transaction and needs a new version prefix and a
/// decision about the stored hashes.
pub(crate) fn projection_hash(draft: &VoucherDraft) -> Result<String, LedgerError> {
    let projection = Projection {
        document_date: draft.document_date(),
        posting_date: draft.posting_date(),
        description: draft.description(),
        postings: draft.postings(),
        buyer_vat_id: draft.buyer_vat_id(),
        corrects_vat_period: draft.corrects_vat_period(),
        vat: draft.vat_expansion().map(|e| VatProjection {
            treatment: e.treatment(),
            source_amount: e.source_amount(),
            net_account: e.net_account(),
            contra_account: e.contra_account(),
            resolved_percent: e.resolved_percent(),
            generated_vat_legs: e.generated_vat_legs(),
        }),
    };
    let bytes = aggountant_wire::canonical_bytes(&projection)?;
    Ok(format!("sha256:v1:{}", sha256_hex(&bytes)))
}

impl Ledger {
    /// Vouchers sealed under a natural key starting with `key_prefix` that have no
    /// bank event recorded: imports sealed before event identity existed. Their
    /// identity is backfilled by passing [`LegacyBankIdentity`] in a batch.
    pub fn bank_vouchers_without_event(
        &self,
        key_prefix: &str,
    ) -> Result<Vec<LegacyBankVoucher>, LedgerError> {
        self.fiscal
            .vouchers_without_bank_event(key_prefix)?
            .into_iter()
            .map(|stored| {
                let body = aggountant_wire::decode_voucher_body(&stored.canonical)?;
                Ok(LegacyBankVoucher {
                    event_key: stored.provenance.natural_key,
                    voucher_number: stored.number,
                    attachment_ref: body.attachment_ref(),
                })
            })
            .collect()
    }

    /// The bytes of a retained artifact.
    pub fn read_artifact(&self, address: &Hash) -> Result<Vec<u8>, LedgerError> {
        self.artifact.get(address)
    }

    /// Resolve a batch without writing: what [`Self::seal_bank_batch`] would do
    /// now. The statement file need not be stored yet.
    pub fn preview_bank_batch(
        &self,
        batch: &BankBatch<'_>,
    ) -> Result<BankBatchReport, LedgerError> {
        self.resolve_bank_batch(batch, false)
    }

    /// Seal a batch atomically. The statement file must be stored first. With any
    /// conflict nothing is written, each conflict is logged as a refused write,
    /// and the report lists the conflicts.
    pub fn seal_bank_batch(&self, batch: &BankBatch<'_>) -> Result<BankBatchReport, LedgerError> {
        self.resolve_bank_batch(batch, true)
    }

    /// Observations recorded for one statement file, or for all of them.
    pub fn bank_source_observations(
        &self,
        attachment_ref: Option<&Hash>,
    ) -> Result<Vec<BankSourceObservation>, LedgerError> {
        let filter = attachment_ref.map(Hash::to_hex);
        Ok(self
            .fiscal
            .bank_source_observations(filter.as_deref())?
            .into_iter()
            .map(|o| BankSourceObservation {
                attachment_ref: o.artifact_ref,
                event_key: o.event_key,
                voucher_number: o.voucher_number,
                observed_at: o.observed_at,
                backfilled: o.backfilled,
            })
            .collect())
    }

    fn resolve_bank_batch(
        &self,
        batch: &BankBatch<'_>,
        write: bool,
    ) -> Result<BankBatchReport, LedgerError> {
        let chart_version = self.masterdata.chart_version()?;
        let pending = (!write).then_some(batch.artifact);
        let refusal = |err: LedgerError, request_hash: &str| {
            if write {
                self.refuse_as(err, request_hash, batch.actor)
            } else {
                err
            }
        };

        // Conflicts found before the store resolves the batch, by entry index:
        // a voucher outside the import that may already book a new entry, and a
        // binding whose voucher does not book its entry.
        let mut early_conflicts: Vec<(usize, BankEventConflict)> = Vec::new();
        let unbound = if batch
            .entries
            .iter()
            .any(|e| matches!(e.booking, BankEventBooking::New { .. }))
        {
            self.unbound_vouchers(batch)?
        } else {
            Vec::new()
        };
        let mut requests = Vec::with_capacity(batch.entries.len());
        let mut warnings = Vec::with_capacity(batch.entries.len());
        let mut request_hashes = Vec::with_capacity(batch.entries.len());
        for (index, entry) in batch.entries.iter().enumerate() {
            let booked = self
                .fiscal
                .read_voucher_by_natural_key(&entry.event_key)?
                .is_some();
            let (request_hash, projection, bind, gate) = match &entry.booking {
                BankEventBooking::New { draft, .. } => {
                    let request_hash = self.draft_request_hash(draft)?;
                    if draft.attachment_ref() != Some(batch.artifact) {
                        let err = LedgerError::Model(
                            aggountant_model::ModelError::InvalidVoucherBody(format!(
                                "bank event {:?} must carry the batch statement {} as its \
                                 attachment",
                                entry.event_key,
                                batch.artifact.to_hex()
                            )),
                        );
                        return Err(refusal(err, &request_hash));
                    }
                    // A transaction already booked seals nothing, so today's
                    // chart, VAT roles and policy need not accept its booking
                    // again: the stored projection decides whether it is the same.
                    if !booked {
                        if let Err(err) = self.check_seal_content(
                            draft,
                            batch.policy_set,
                            &chart_version,
                            pending,
                        ) {
                            return Err(refusal(err, &request_hash));
                        }
                    }
                    warnings.push(self.untagged_vat_warnings(draft, &chart_version)?);
                    let gate = PeriodGate {
                        posting_date: draft.posting_date(),
                        authority: None,
                        corrects_vat_period: draft.corrects_vat_period(),
                    };
                    (request_hash, projection_hash(draft)?, None, gate)
                }
                BankEventBooking::Existing { voucher_number } => {
                    let Some(existing) = self.fiscal.read_voucher(*voucher_number)? else {
                        return Err(LedgerError::VoucherNotFound(format!(
                            "voucher #{voucher_number} named for bank transaction {:?} is not \
                             sealed",
                            entry.event_key
                        )));
                    };
                    let body = aggountant_wire::decode_voucher_body(&existing.canonical)?;
                    // A voucher the import sealed books the transaction its own
                    // key names, never another one.
                    let imported = existing
                        .provenance
                        .natural_key
                        .starts_with(batch.event_key_prefix)
                        && existing.provenance.natural_key != entry.event_key;
                    if imported || !entry.facts.booked_by(&body) {
                        early_conflicts.push((
                            index,
                            BankEventConflict {
                                event_key: entry.event_key.clone(),
                                error_code: BankEventConflictCode::BankVoucherMismatch.as_str(),
                                code: BankEventConflictCode::BankVoucherMismatch,
                                voucher_number: *voucher_number,
                                evidence: None,
                                message: if imported {
                                    format!(
                                        "voucher #{voucher_number} was sealed by the bank import \
                                         for bank transaction {:?}, so it cannot book {:?}",
                                        existing.provenance.natural_key, entry.event_key
                                    )
                                } else {
                                    format!(
                                        "voucher #{voucher_number} does not book bank \
                                         transaction {:?}: it has no posting of {} dated within \
                                         {DATE_TOLERANCE_DAYS} days. Name the voucher that books \
                                         it, or assign the transaction for a new voucher",
                                        entry.event_key,
                                        entry.facts.describe()
                                    )
                                },
                            },
                        ));
                    }
                    warnings.push(Vec::new());
                    // A binding seals nothing, so no period gate applies.
                    let gate = PeriodGate {
                        posting_date: body.posting_date(),
                        authority: None,
                        corrects_vat_period: None,
                    };
                    (
                        existing.provenance.request_hash.clone(),
                        projection_hash(&body.draft())?,
                        Some(*voucher_number),
                        gate,
                    )
                }
            };
            requests.push(BankEventRequest {
                provenance: StoredProvenance {
                    actor: batch.actor.to_string(),
                    natural_key: entry.event_key.clone(),
                    request_hash: request_hash.clone(),
                    source_fingerprint: Some(entry.source_fingerprint.clone()),
                    policy_set: batch.policy_set.to_string(),
                    software_version: SOFTWARE_VERSION.to_string(),
                    chart_version: chart_version.clone(),
                    sealed_at: Instant::now().to_string(),
                    sent_hash: None,
                },
                projection_hash: projection,
                gate,
                bind,
            });
            request_hashes.push(request_hash);
            if !booked {
                if let BankEventBooking::New { not_booked_by, .. } = &entry.booking {
                    early_conflicts.extend(
                        possible_bookings(batch, entry, not_booked_by, &unbound)
                            .into_iter()
                            .map(|conflict| (index, conflict)),
                    );
                }
            }
        }

        let mut backfill = Vec::with_capacity(batch.backfill.len());
        for identity in batch.backfill {
            let Some(stored) = self
                .fiscal
                .read_voucher_by_natural_key(&identity.event_key)?
            else {
                return Err(LedgerError::VoucherNotFound(format!(
                    "no voucher holds bank event {:?} to backfill",
                    identity.event_key
                )));
            };
            let body = aggountant_wire::decode_voucher_body(&stored.canonical)?;
            let Some(artifact) = body.attachment_ref() else {
                return Err(LedgerError::ArtifactNotFound(format!(
                    "voucher #{} for bank event {:?} retains no statement to backfill from",
                    stored.number, identity.event_key
                )));
            };
            backfill.push(BankEventBackfill {
                event_key: identity.event_key.clone(),
                source_fingerprint: identity.source_fingerprint.clone(),
                projection_hash: projection_hash(&body.draft())?,
                artifact_ref: artifact.to_hex(),
            });
        }

        let request = BankBatchRequest {
            artifact_ref: batch.artifact.to_hex(),
            observed_at: Instant::now().to_string(),
            entries: requests,
            backfill,
            unbooked_event_keys: batch.unbooked.iter().map(|u| u.event_key.clone()).collect(),
            // With an early conflict the store still resolves every entry, so
            // one run lists every conflict, but in a transaction it rolls back.
            write: write && early_conflicts.is_empty(),
        };
        let build = |index: usize, number: u64, prev: &Hash| -> Result<BuiltVoucher, LedgerError> {
            let draft = batch.entries[index].draft().ok_or_else(|| {
                LedgerError::Store(format!(
                    "bank event {:?} is a binding and seals no voucher",
                    batch.entries[index].event_key
                ))
            })?;
            let body =
                aggountant_model::VoucherBody::from_draft(draft, number, chart_version.clone())?;
            let (canonical, hash) = aggountant_wire::seal_voucher(prev, &body)?;
            Ok(BuiltVoucher {
                body,
                canonical,
                hash,
            })
        };

        let (resolutions, unbooked_conflicts, backfilled, committed) =
            match self.fiscal.commit_bank_batch(&request, &build)? {
                BankBatchResult::Refused { error, rejection } => {
                    if write {
                        return Err(self.record_refusal(error, &rejection));
                    }
                    return Err(error);
                }
                BankBatchResult::Resolved {
                    entries,
                    unbooked_conflicts,
                    backfilled,
                    committed,
                } => (entries, unbooked_conflicts, backfilled, committed),
            };

        let mut conflicts = Vec::new();
        // The request hash each conflict's rejection record names: the incoming
        // draft's, or for an unbooked entry or a binding the booked voucher's.
        let mut conflict_request_hashes = Vec::new();
        let mut results = Vec::with_capacity(resolutions.len());
        for (index, (((resolution, entry), warnings), request_hash)) in resolutions
            .into_iter()
            .zip(batch.entries)
            .zip(warnings)
            .zip(&request_hashes)
            .enumerate()
        {
            let early: Vec<BankEventConflict> = early_conflicts
                .iter()
                .filter(|(i, _)| *i == index)
                .map(|(_, conflict)| conflict.clone())
                .collect();
            let conflicted = !early.is_empty();
            conflict_request_hashes.extend(early.iter().map(|_| request_hash.clone()));
            conflicts.extend(early);
            let (outcome, voucher) = match resolution {
                BankEventResolution::Conflict { code, existing } => {
                    conflicts.push(entry_conflict(entry, code, &existing));
                    conflict_request_hashes.push(request_hash.clone());
                    continue;
                }
                _ if conflicted => continue,
                BankEventResolution::New(voucher) => (
                    BankEventOutcome::NewVoucher,
                    committed.then(|| SealedVoucher::from_stored(voucher, false)),
                ),
                BankEventResolution::Duplicate(voucher) => (
                    BankEventOutcome::DuplicateSourceObservation,
                    Some(SealedVoucher::from_stored(voucher, true)),
                ),
                BankEventResolution::Bound(voucher) => (
                    BankEventOutcome::BoundExistingVoucher,
                    Some(SealedVoucher::from_stored(voucher, true)),
                ),
            };
            results.push(BankEventResult {
                event_key: entry.event_key.clone(),
                outcome,
                voucher,
                warnings,
            });
        }
        for (index, existing) in unbooked_conflicts {
            let unbooked = &batch.unbooked[index];
            conflict_request_hashes.push(existing.provenance.request_hash.clone());
            conflicts.push(BankEventConflict {
                event_key: unbooked.event_key.clone(),
                error_code: BankEventConflictCode::SourceFactsChanged.as_str(),
                code: BankEventConflictCode::SourceFactsChanged,
                voucher_number: existing.number,
                evidence: None,
                message: format!(
                    "the statement lists bank transaction {:?} with status {:?}, but voucher \
                     #{} booked it as a booked transaction; the bank has changed its status",
                    unbooked.event_key, unbooked.status, existing.number
                ),
            });
        }

        // Each conflict of a batch meant to write is an audited refused write.
        // (The CLI previews before it seals, so in practice a conflict is
        // refused there, without a record, as any other import refusal is.)
        if write {
            for (conflict, request_hash) in conflicts.iter().zip(&conflict_request_hashes) {
                let rejection = RejectionRecord {
                    reason_code: conflict.error_code.to_string(),
                    request_hash: request_hash.clone(),
                    actor: batch.actor.to_string(),
                    target: "fiscal:main".to_string(),
                    note: conflict.message.clone(),
                };
                self.fiscal
                    .record_rejection(&rejection)
                    .map_err(|audit_err| {
                        LedgerError::AuditUnavailable(format!(
                            "failed to persist rejection for {}: {audit_err}",
                            conflict.error_code
                        ))
                    })?;
            }
        }
        Ok(BankBatchReport {
            results,
            conflicts,
            backfilled_count: backfilled,
            written: committed,
        })
    }

    /// Every voucher not bound to a bank event and not sealed under the
    /// import's own key prefix, decoded once per batch.
    fn unbound_vouchers(
        &self,
        batch: &BankBatch<'_>,
    ) -> Result<Vec<(StoredVoucher, VoucherBody)>, LedgerError> {
        self.fiscal
            .vouchers_not_bound_to_bank_event()?
            .into_iter()
            .filter(|stored| {
                !stored
                    .provenance
                    .natural_key
                    .starts_with(batch.event_key_prefix)
            })
            .map(|stored| {
                let body = aggountant_wire::decode_voucher_body(&stored.canonical)?;
                Ok((stored, body))
            })
            .collect()
    }
}

/// Vouchers sealed outside the import, bound to no bank event, that may
/// already book `entry`, as one conflict each. A voucher another entry of
/// this batch binds, and those the user has ruled out, are not candidates.
fn possible_bookings(
    batch: &BankBatch<'_>,
    entry: &BankEventEntry,
    not_booked_by: &[u64],
    unbound: &[(StoredVoucher, VoucherBody)],
) -> Vec<BankEventConflict> {
    let bound_in_batch: Vec<u64> = batch
        .entries
        .iter()
        .filter_map(|e| match e.booking {
            BankEventBooking::Existing { voucher_number } => Some(voucher_number),
            BankEventBooking::New { .. } => None,
        })
        .collect();
    let facts = &entry.facts;
    let mut conflicts = Vec::new();
    for (stored, body) in unbound {
        let number = stored.number;
        if bound_in_batch.contains(&number) || not_booked_by.contains(&number) {
            continue;
        }
        let natural_key = &stored.provenance.natural_key;
        let evidence = if facts.named_in(natural_key) || facts.named_in(body.description()) {
            CandidateEvidence::BankReference
        } else if facts.booked_by(body) {
            CandidateEvidence::AccountAmountDate
        } else {
            continue;
        };
        let why = match evidence {
            CandidateEvidence::BankReference => format!(
                "its natural key {natural_key:?} or its description names bank reference {:?}",
                facts.bank_ref
            ),
            CandidateEvidence::AccountAmountDate => format!(
                "it posts {} (dated within {DATE_TOLERANCE_DAYS} days)",
                facts.describe()
            ),
        };
        conflicts.push(BankEventConflict {
            event_key: entry.event_key.clone(),
            error_code: BankEventConflictCode::BankEntryPossiblyBooked.as_str(),
            code: BankEventConflictCode::BankEntryPossiblyBooked,
            voucher_number: number,
            evidence: Some(evidence),
            message: format!(
                "bank transaction {:?} may already be booked by voucher #{number}, sealed outside \
                 the bank import: {why}. If #{number} books it, bind the transaction to it \
                 (existing_vouchers) instead of sealing a new voucher; if it does not, list \
                 {number} in the assignment's not_booked_by",
                entry.event_key
            ),
        });
    }
    conflicts
}

fn entry_conflict(
    entry: &BankEventEntry,
    code: BankEventConflictCode,
    existing: &StoredVoucher,
) -> BankEventConflict {
    let event_key = entry.event_key.as_str();
    let number = existing.number;
    let message = match code {
        BankEventConflictCode::SourceFactsChanged => format!(
            "bank transaction {event_key:?} was booked as voucher #{number} from different bank \
             facts (amount, dates, direction, parties or remittance); the bank has changed the \
             transaction. Find out what changed before booking anything for it"
        ),
        BankEventConflictCode::PostingProjectionChanged => format!(
            "bank transaction {event_key:?} is already booked as voucher #{number} with the same \
             bank facts but a different booking (accounts, amounts, VAT, dates or description). \
             Give it the assignment voucher #{number} was sealed with, or, if the transaction \
             was bound to #{number} as an existing voucher, the same existing_vouchers binding; \
             a change to the booking is a correction voucher that refers to #{number}"
        ),
        BankEventConflictCode::SourceIdentityUnknown => format!(
            "voucher #{number} already holds bank transaction {event_key:?}, but the transaction \
             could not be read back from the statement it retained, so it cannot be compared; \
             it is not booked again"
        ),
        BankEventConflictCode::BankVoucherMismatch => match entry.booking {
            BankEventBooking::Existing { voucher_number } if voucher_number == number => format!(
                "voucher #{number} is already bound to another bank transaction, so it cannot \
                 also book {event_key:?}"
            ),
            _ => format!(
                "bank transaction {event_key:?} is already bound to voucher #{number}; name \
                 #{number} for it, or leave the binding out"
            ),
        },
        // Raised before the store resolves the batch, never from a stored voucher.
        BankEventConflictCode::BankEntryPossiblyBooked => {
            format!("bank transaction {event_key:?} may already be booked by voucher #{number}")
        }
    };
    BankEventConflict {
        event_key: event_key.to_string(),
        error_code: code.as_str(),
        code,
        voucher_number: number,
        evidence: None,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aggountant_domain::{vat_expand, VatExpandInput, VatTreatment};
    use aggountant_model::{Currency, VatExpansion};

    fn expanded(attachment: Option<Hash>, net_account: &str) -> VoucherDraft {
        let date = Date::parse("2026-12-30").unwrap();
        vat_expand(&VatExpandInput {
            amount: Money::parse("124.00", Currency::new("EUR").unwrap()).unwrap(),
            net_account: AccountId::new(net_account).unwrap(),
            contra_account: AccountId::new("1910").unwrap(),
            treatment: VatTreatment::DomesticInput {
                rate_class: "domestic-standard".into(),
            },
            document_date: date,
            posting_date: date,
            description: "Bank payment".into(),
            attachment_ref: attachment,
        })
        .unwrap()
    }

    /// The same draft with the software state of its VAT expansion changed, as
    /// a later release with another rates file or policy set would project it.
    fn with_other_release(draft: &VoucherDraft) -> VoucherDraft {
        let e = draft.vat_expansion().unwrap();
        let other = VatExpansion::new(
            e.version(),
            e.treatment().clone(),
            e.source_amount().clone(),
            e.net_account().clone(),
            e.contra_account().clone(),
            e.resolved_percent().map(str::to_owned),
            "fi.core.2027-01".into(),
            Hash::from_bytes([7; 32]),
            e.generated_vat_legs().to_vec(),
        )
        .unwrap();
        VoucherDraft::new(
            draft.document_date(),
            draft.posting_date(),
            draft.description().to_owned(),
            draft.postings().to_vec(),
            draft.attachment_ref(),
        )
        .unwrap()
        .with_vat_expansion(other)
        .unwrap()
    }

    #[test]
    fn the_projection_ignores_the_statement_file_and_the_release() {
        let a = expanded(Some(Hash::from_bytes([1; 32])), "7690");
        let b = expanded(Some(Hash::from_bytes([2; 32])), "7690");
        let hash = projection_hash(&a).unwrap();
        assert_eq!(hash, projection_hash(&b).unwrap());
        assert_eq!(hash, projection_hash(&with_other_release(&a)).unwrap());
        assert_ne!(hash, projection_hash(&expanded(None, "7600")).unwrap());
    }
}
