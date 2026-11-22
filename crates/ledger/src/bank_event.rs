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
//! transaction is a new voucher or a duplicate observation and all of it is
//! written, or something conflicts and nothing is.

use serde::Serialize;

use aggountant_model::{
    AccountId, Date, GeneratedVatLeg, Hash, Instant, Money, PeriodId, Posting,
    VatExpansionTreatment, VoucherDraft,
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
}

impl BankEventConflictCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SourceFactsChanged => "SOURCE_FACTS_CHANGED",
            Self::PostingProjectionChanged => "POSTING_PROJECTION_CHANGED",
            Self::SourceIdentityUnknown => "SOURCE_IDENTITY_UNKNOWN",
        }
    }
}

/// One bank transaction of an import batch.
pub struct BankEventEntry {
    /// The bank's entry reference and account; sealed as the natural key.
    pub event_key: String,
    pub source_fingerprint: String,
    /// The proposed voucher, with the statement file as its attachment.
    pub draft: VoucherDraft,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BankEventOutcome {
    NewVoucher,
    DuplicateSourceObservation,
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
    /// The voucher already sealed for the event key.
    pub voucher_number: u64,
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

        let mut requests = Vec::with_capacity(batch.entries.len());
        let mut warnings = Vec::with_capacity(batch.entries.len());
        let mut request_hashes = Vec::with_capacity(batch.entries.len());
        for entry in batch.entries {
            let request_hash = self.draft_request_hash(&entry.draft)?;
            if entry.draft.attachment_ref() != Some(batch.artifact) {
                let err =
                    LedgerError::Model(aggountant_model::ModelError::InvalidVoucherBody(format!(
                        "bank event {:?} must carry the batch statement {} as its attachment",
                        entry.event_key,
                        batch.artifact.to_hex()
                    )));
                return Err(refusal(err, &request_hash));
            }
            // A transaction already booked seals nothing, so today's chart, VAT
            // roles and policy need not accept its booking again: the stored
            // projection decides whether it is the same.
            let booked = self
                .fiscal
                .read_voucher_by_natural_key(&entry.event_key)?
                .is_some();
            if !booked {
                if let Err(err) =
                    self.check_seal_content(&entry.draft, batch.policy_set, &chart_version, pending)
                {
                    return Err(refusal(err, &request_hash));
                }
            }
            warnings.push(self.untagged_vat_warnings(&entry.draft, &chart_version)?);
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
                projection_hash: projection_hash(&entry.draft)?,
                gate: PeriodGate {
                    posting_date: entry.draft.posting_date(),
                    authority: None,
                    corrects_vat_period: entry.draft.corrects_vat_period(),
                },
            });
            request_hashes.push(request_hash);
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
            write,
        };
        let build = |index: usize, number: u64, prev: &Hash| -> Result<BuiltVoucher, LedgerError> {
            let body = aggountant_model::VoucherBody::from_draft(
                &batch.entries[index].draft,
                number,
                chart_version.clone(),
            )?;
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
        // draft's, or for an unbooked entry the booked voucher's.
        let mut conflict_request_hashes = Vec::new();
        let mut results = Vec::with_capacity(resolutions.len());
        for (((resolution, entry), warnings), request_hash) in resolutions
            .into_iter()
            .zip(batch.entries)
            .zip(warnings)
            .zip(&request_hashes)
        {
            let (outcome, voucher) = match resolution {
                BankEventResolution::New(voucher) => (
                    BankEventOutcome::NewVoucher,
                    committed.then(|| SealedVoucher::from_stored(voucher, false)),
                ),
                BankEventResolution::Duplicate(voucher) => (
                    BankEventOutcome::DuplicateSourceObservation,
                    Some(SealedVoucher::from_stored(voucher, true)),
                ),
                BankEventResolution::Conflict { code, existing } => {
                    conflicts.push(entry_conflict(&entry.event_key, code, &existing));
                    conflict_request_hashes.push(request_hash.clone());
                    continue;
                }
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
}

fn entry_conflict(
    event_key: &str,
    code: BankEventConflictCode,
    existing: &StoredVoucher,
) -> BankEventConflict {
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
             Give it the assignment voucher #{number} was sealed with; a change to the booking \
             is a correction voucher that refers to #{number}"
        ),
        BankEventConflictCode::SourceIdentityUnknown => format!(
            "voucher #{number} already holds bank transaction {event_key:?}, but the transaction \
             could not be read back from the statement it retained, so it cannot be compared; \
             it is not booked again"
        ),
    };
    BankEventConflict {
        event_key: event_key.to_string(),
        error_code: code.as_str(),
        code,
        voucher_number: number,
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
