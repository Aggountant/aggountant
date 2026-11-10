//! Stable, read-only views over sealed vouchers.
//!
//! These types deliberately project the Ledger's private store rows. Store types and
//! SQL never cross the crate boundary, and provenance remains visibly separate from
//! the hash-bound canonical voucher body.

use serde::Serialize;

use aggountant_model::{Date, Hash, VoucherBody};

use crate::store::StoredVoucher;
use crate::LedgerError;

/// Filters for the bounded sealed-voucher list surface.
#[derive(Debug, Clone)]
pub struct VoucherListQuery {
    pub attachment_ref: Option<Hash>,
    pub posting_from: Option<Date>,
    pub posting_to: Option<Date>,
    pub limit: usize,
}

/// Stored metadata accompanying a voucher but excluded from its fiscal hash preimage.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VoucherProvenanceView {
    pub actor: String,
    pub natural_key: String,
    pub request_hash: String,
    pub source_fingerprint: Option<String>,
    pub policy_set: String,
    pub software_version: String,
    pub chart_version: String,
    pub sealed_at: String,
    pub sent_hash: Option<String>,
}

/// One exact sealed voucher, with its canonical body and separate stored provenance.
#[derive(Debug, Clone, Serialize)]
pub struct VoucherView {
    pub number: u64,
    pub voucher_hash: String,
    pub body: VoucherBody,
    pub provenance: VoucherProvenanceView,
}

/// Bounded list-row projection. A row is only a possible source match; it is not a
/// duplicate decision or a statement about economic-event identity.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VoucherSummary {
    pub number: u64,
    pub voucher_hash: String,
    pub document_date: Date,
    pub posting_date: Date,
    pub attachment_ref: Option<Hash>,
    pub match_status: &'static str,
}

/// Deterministically ordered, bounded result from [`crate::Ledger::list_vouchers`].
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VoucherListView {
    /// Number of rows returned in `vouchers` (at most the requested limit).
    /// `truncated` says whether at least one additional match exists.
    pub count: usize,
    pub truncated: bool,
    pub order: &'static str,
    pub vouchers: Vec<VoucherSummary>,
}

pub(crate) fn decode_view(stored: &StoredVoucher) -> Result<VoucherView, LedgerError> {
    let body = aggountant_wire::decode_voucher_body(&stored.canonical).map_err(|error| {
        LedgerError::VoucherReadCorrupt(format!(
            "voucher {} canonical body failed strict decoding: {error}",
            stored.number
        ))
    })?;
    let recomputed = aggountant_wire::chain_hash(&stored.prev_hash, &stored.canonical);
    if recomputed != stored.hash {
        return Err(LedgerError::VoucherReadCorrupt(format!(
            "voucher {} hash does not bind its stored prev_hash and canonical body",
            stored.number
        )));
    }
    if body.number() != stored.number {
        return Err(LedgerError::VoucherReadCorrupt(format!(
            "row number {} holds a canonical body numbered {}",
            stored.number,
            body.number()
        )));
    }
    if body.chart_version() != stored.chart_version {
        return Err(LedgerError::VoucherReadCorrupt(format!(
            "voucher {} chart_version column disagrees with its canonical body",
            stored.number
        )));
    }
    if stored.provenance.chart_version != stored.chart_version {
        return Err(LedgerError::VoucherReadCorrupt(format!(
            "voucher {} stored provenance chart_version disagrees with its row",
            stored.number
        )));
    }
    if stored.provenance.sealed_at != stored.sealed_at {
        return Err(LedgerError::VoucherReadCorrupt(format!(
            "voucher {} stored provenance sealed_at disagrees with its row",
            stored.number
        )));
    }

    Ok(VoucherView {
        number: stored.number,
        voucher_hash: stored.hash.to_hex(),
        body,
        provenance: VoucherProvenanceView {
            actor: stored.provenance.actor.clone(),
            natural_key: stored.provenance.natural_key.clone(),
            request_hash: stored.provenance.request_hash.clone(),
            source_fingerprint: stored.provenance.source_fingerprint.clone(),
            policy_set: stored.provenance.policy_set.clone(),
            software_version: stored.provenance.software_version.clone(),
            chart_version: stored.provenance.chart_version.clone(),
            sealed_at: stored.provenance.sealed_at.clone(),
            sent_hash: stored.provenance.sent_hash.clone(),
        },
    })
}

pub(crate) fn validate_query(query: &VoucherListQuery) -> Result<(), LedgerError> {
    if query.limit == 0 || query.limit > 100 {
        return Err(LedgerError::InvalidVoucherRead(format!(
            "limit must be in 1..=100, got {}",
            query.limit
        )));
    }
    match (query.posting_from, query.posting_to) {
        (Some(from), Some(to)) if from > to => Err(LedgerError::InvalidVoucherRead(format!(
            "posting date range is reversed: {from} is after {to}"
        ))),
        (Some(_), None) | (None, Some(_)) => Err(LedgerError::InvalidVoucherRead(
            "posting date filtering requires both posting_from and posting_to".into(),
        )),
        (None, None) if query.attachment_ref.is_none() => Err(LedgerError::InvalidVoucherRead(
            "voucher list requires attachment_ref or both posting date bounds".into(),
        )),
        _ => Ok(()),
    }
}
