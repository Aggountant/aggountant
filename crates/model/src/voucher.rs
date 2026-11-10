//! Voucher / Posting types and the **local** invariants they enforce
//! (`candidate.md §3/§4`): balance `Σ = 0`, `>= 2` postings, single currency.
//! These are constructor invariants that `serde` deserialize must not bypass,
//! so `VoucherDraft` and `VoucherBody` hand-write `Deserialize` to route
//! through the validating constructor.

use rust_decimal::Decimal;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ModelError;
use crate::newtype::{AccountId, Date, Hash, Money, VatCode};
use crate::vat_expansion::VatExpansion;

/// Current wire-format profile revision (`ADR-2 §Decision.2`).
pub const WIRE_VERSION: u32 = 1;
/// Schema version of the hashed voucher body.
pub const SCHEMA_VERSION_VOUCHER: u32 = 1;
/// Record-type tag bound into the hash preimage for domain separation
/// (`ADR-2 §Consequences`, "unambiguous framing").
pub const RECORD_TYPE_VOUCHER: &str = "voucher";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Debit,
    Credit,
}

/// One debit or credit line. Never stands alone — always part of a voucher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Posting {
    pub account: AccountId,
    pub amount: Money,
    pub side: Side,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vat_code: Option<VatCode>,
}

impl Posting {
    /// Signed contribution to the balance: debit positive, credit negative.
    pub fn signed_amount(&self) -> Decimal {
        match self.side {
            Side::Debit => self.amount.amount(),
            Side::Credit => -self.amount.amount(),
        }
    }
}

/// Shared local-invariant check over a set of postings.
pub(crate) fn check_postings(postings: &[Posting]) -> Result<(), ModelError> {
    if postings.len() < 2 {
        return Err(ModelError::InsufficientPostings(format!(
            "a voucher needs >= 2 postings, got {}",
            postings.len()
        )));
    }
    let currency = postings[0].amount.currency().clone();
    for (i, p) in postings.iter().enumerate() {
        if p.amount.currency() != &currency {
            return Err(ModelError::MixedCurrency(format!(
                "posting {i} currency {} != voucher currency {}",
                p.amount.currency(),
                currency
            )));
        }
        if p.amount.amount() <= Decimal::ZERO {
            return Err(ModelError::InvalidMoney(format!(
                "posting {i} amount must be > 0 (use the side for direction), got {}",
                p.amount.amount_str()
            )));
        }
    }
    let sum: Decimal = postings.iter().map(Posting::signed_amount).sum();
    if !sum.is_zero() {
        return Err(ModelError::UnbalancedVoucher(format!(
            "debits - credits must be 0, got {sum}"
        )));
    }
    Ok(())
}

/// Model-level wire invariant for a buyer VAT ID that the Domain has already
/// validated: the stored value is its normalized ASCII representation. Country
/// membership and EU-sale applicability remain Domain rules.
fn check_normalized_buyer_vat_id(buyer_vat_id: &str) -> Result<(), ModelError> {
    let canonical = buyer_vat_id.len() >= 4
        && buyer_vat_id.len() <= 14
        && buyer_vat_id.as_bytes()[..2]
            .iter()
            .all(u8::is_ascii_uppercase)
        && buyer_vat_id.as_bytes()[2..]
            .iter()
            .all(u8::is_ascii_alphanumeric)
        && !buyer_vat_id.bytes().any(|byte| byte.is_ascii_lowercase());
    if !canonical {
        return Err(ModelError::InvalidVoucherBody(
            "buyer_vat_id must be a normalized uppercase VAT ID".into(),
        ));
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// VoucherDraft — pre-seal, no number / hash. Enforces local invariants.
// ----------------------------------------------------------------------------

/// Invariant-bearing fields are **private**: a `VoucherDraft` can only be
/// obtained through `new` or the validating `Deserialize`, so `Σ=0`, `>=2`
/// postings and single-currency always hold. Accessors are read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoucherDraft {
    document_date: Date,
    posting_date: Date,
    description: String,
    postings: Vec<Posting>,
    attachment_ref: Option<Hash>,
    buyer_vat_id: Option<String>,
    vat_expansion: Option<VatExpansion>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoucherDraftWire {
    document_date: Date,
    posting_date: Date,
    description: String,
    postings: Vec<Posting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attachment_ref: Option<Hash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    buyer_vat_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vat_expansion: Option<VatExpansion>,
}

impl VoucherDraft {
    pub fn new(
        document_date: Date,
        posting_date: Date,
        description: String,
        postings: Vec<Posting>,
        attachment_ref: Option<Hash>,
    ) -> Result<Self, ModelError> {
        if description.trim().is_empty() {
            return Err(ModelError::EmptyDescription(
                "description must not be empty".into(),
            ));
        }
        check_postings(&postings)?;
        Ok(VoucherDraft {
            document_date,
            posting_date,
            description,
            postings,
            attachment_ref,
            buyer_vat_id: None,
            vat_expansion: None,
        })
    }

    pub fn document_date(&self) -> Date {
        self.document_date
    }
    pub fn posting_date(&self) -> Date {
        self.posting_date
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }
    pub fn attachment_ref(&self) -> Option<Hash> {
        self.attachment_ref
    }
    pub fn buyer_vat_id(&self) -> Option<&str> {
        self.buyer_vat_id.as_deref()
    }
    pub fn vat_expansion(&self) -> Option<&VatExpansion> {
        self.vat_expansion.as_ref()
    }

    /// Attach a domain-validated, normalized buyer VAT ID while retaining `new`'s
    /// source-compatible signature for vouchers that do not carry one.
    pub fn with_buyer_vat_id(mut self, buyer_vat_id: String) -> Result<Self, ModelError> {
        check_normalized_buyer_vat_id(&buyer_vat_id)?;
        if self.buyer_vat_id.is_some() {
            return Err(ModelError::InvalidVoucherBody(
                "buyer VAT ID is already attached".into(),
            ));
        }
        self.buyer_vat_id = Some(buyer_vat_id);
        Ok(self)
    }

    /// Attach validated VAT-A expansion provenance while retaining `new`'s source-compatible signature.
    pub fn with_vat_expansion(mut self, vat_expansion: VatExpansion) -> Result<Self, ModelError> {
        if self.vat_expansion.is_some() {
            return Err(ModelError::InvalidVatExpansion(
                "VAT expansion provenance is already attached".into(),
            ));
        }
        if self.currency() != vat_expansion.source_amount().currency() {
            return Err(ModelError::InvalidVatExpansion(
                "voucher currency differs from VAT expansion source currency".into(),
            ));
        }
        vat_expansion.validate_postings(&self.postings)?;
        self.vat_expansion = Some(vat_expansion);
        Ok(self)
    }

    /// The currency shared by all postings (guaranteed non-empty & uniform by
    /// the constructor, so this cannot panic).
    pub fn currency(&self) -> &crate::newtype::Currency {
        self.postings[0].amount.currency()
    }
}

impl Serialize for VoucherDraft {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        VoucherDraftWire {
            document_date: self.document_date,
            posting_date: self.posting_date,
            description: self.description.clone(),
            postings: self.postings.clone(),
            attachment_ref: self.attachment_ref,
            buyer_vat_id: self.buyer_vat_id.clone(),
            vat_expansion: self.vat_expansion.clone(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for VoucherDraft {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = VoucherDraftWire::deserialize(d)?;
        VoucherDraft::new(
            w.document_date,
            w.posting_date,
            w.description,
            w.postings,
            w.attachment_ref,
        )
        .and_then(|draft| match w.buyer_vat_id {
            Some(buyer_vat_id) => draft.with_buyer_vat_id(buyer_vat_id),
            None => Ok(draft),
        })
        .and_then(|draft| match w.vat_expansion {
            Some(expansion) => draft.with_vat_expansion(expansion),
            None => Ok(draft),
        })
        .map_err(D::Error::custom)
    }
}

// ----------------------------------------------------------------------------
// VoucherBody — the hashed fiscal record (canonicalized by the wire crate).
// prev_hash is chain glue (not a field); the hash is SHA256(prev_hash || wire).
// ----------------------------------------------------------------------------

/// Invariant-bearing fields are **private**: a `VoucherBody` is only reachable
/// through `from_draft` / `new` / the validating `Deserialize`, all of which
/// enforce the same checks. Accessors are read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoucherBody {
    wire_version: u32,
    schema_version: u32,
    number: u64,
    document_date: Date,
    posting_date: Date,
    description: String,
    postings: Vec<Posting>,
    attachment_ref: Option<Hash>,
    chart_version: String,
    buyer_vat_id: Option<String>,
    vat_expansion: Option<VatExpansion>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoucherBodyWire {
    wire_version: u32,
    schema_version: u32,
    record_type: String,
    number: u64,
    document_date: Date,
    posting_date: Date,
    description: String,
    postings: Vec<Posting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attachment_ref: Option<Hash>,
    chart_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    buyer_vat_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vat_expansion: Option<VatExpansion>,
}

impl VoucherBody {
    /// The single validating constructor. Enforces: supported wire/schema
    /// versions, `number >= 1`, non-empty description and chart_version, and the
    /// posting invariants — so neither the write path nor the strict read path
    /// can mint a body the other would reject.
    #[allow(clippy::too_many_arguments)]
    fn new(
        wire_version: u32,
        schema_version: u32,
        number: u64,
        document_date: Date,
        posting_date: Date,
        description: String,
        postings: Vec<Posting>,
        attachment_ref: Option<Hash>,
        chart_version: String,
        buyer_vat_id: Option<String>,
        vat_expansion: Option<VatExpansion>,
    ) -> Result<Self, ModelError> {
        if wire_version != WIRE_VERSION {
            return Err(ModelError::InvalidVoucherBody(format!(
                "unsupported wire_version {wire_version}"
            )));
        }
        if schema_version != SCHEMA_VERSION_VOUCHER {
            return Err(ModelError::InvalidVoucherBody(format!(
                "unsupported schema_version {schema_version}"
            )));
        }
        if number < 1 {
            return Err(ModelError::InvalidVoucherBody(
                "voucher number must be >= 1".into(),
            ));
        }
        if description.trim().is_empty() {
            return Err(ModelError::EmptyDescription(
                "description must not be empty".into(),
            ));
        }
        if chart_version.trim().is_empty() {
            return Err(ModelError::InvalidVoucherBody(
                "chart_version must not be empty".into(),
            ));
        }
        check_postings(&postings)?;
        if let Some(buyer_vat_id) = &buyer_vat_id {
            check_normalized_buyer_vat_id(buyer_vat_id)?;
        }
        if let Some(expansion) = &vat_expansion {
            if postings[0].amount.currency() != expansion.source_amount().currency() {
                return Err(ModelError::InvalidVatExpansion(
                    "voucher currency differs from VAT expansion source currency".into(),
                ));
            }
            expansion.validate_postings(&postings)?;
        }
        Ok(VoucherBody {
            wire_version,
            schema_version,
            number,
            document_date,
            posting_date,
            description,
            postings,
            attachment_ref,
            chart_version,
            buyer_vat_id,
            vat_expansion,
        })
    }

    /// Build a hashable body from a validated draft, an allocated number and
    /// the resolved chart version.
    pub fn from_draft(
        draft: &VoucherDraft,
        number: u64,
        chart_version: String,
    ) -> Result<Self, ModelError> {
        VoucherBody::new(
            WIRE_VERSION,
            SCHEMA_VERSION_VOUCHER,
            number,
            draft.document_date,
            draft.posting_date,
            draft.description.clone(),
            draft.postings.clone(),
            draft.attachment_ref,
            chart_version,
            draft.buyer_vat_id.clone(),
            draft.vat_expansion.clone(),
        )
    }

    pub fn wire_version(&self) -> u32 {
        self.wire_version
    }
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn number(&self) -> u64 {
        self.number
    }
    pub fn document_date(&self) -> Date {
        self.document_date
    }
    pub fn posting_date(&self) -> Date {
        self.posting_date
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }
    pub fn attachment_ref(&self) -> Option<Hash> {
        self.attachment_ref
    }
    pub fn chart_version(&self) -> &str {
        &self.chart_version
    }
    pub fn buyer_vat_id(&self) -> Option<&str> {
        self.buyer_vat_id.as_deref()
    }
    pub fn vat_expansion(&self) -> Option<&VatExpansion> {
        self.vat_expansion.as_ref()
    }
}

impl Serialize for VoucherBody {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        VoucherBodyWire {
            wire_version: self.wire_version,
            schema_version: self.schema_version,
            record_type: RECORD_TYPE_VOUCHER.to_string(),
            number: self.number,
            document_date: self.document_date,
            posting_date: self.posting_date,
            description: self.description.clone(),
            postings: self.postings.clone(),
            attachment_ref: self.attachment_ref,
            chart_version: self.chart_version.clone(),
            buyer_vat_id: self.buyer_vat_id.clone(),
            vat_expansion: self.vat_expansion.clone(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for VoucherBody {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = VoucherBodyWire::deserialize(d)?;
        if w.record_type != RECORD_TYPE_VOUCHER {
            return Err(D::Error::custom(format!(
                "INVALID_RECORD_TYPE: expected {RECORD_TYPE_VOUCHER:?}, got {:?}",
                w.record_type
            )));
        }
        // All other invariants (wire/schema version, number >= 1, non-empty
        // description/chart_version, posting invariants) go through the single
        // validating constructor — the strict read path cannot accept a body the
        // write path would refuse to mint.
        VoucherBody::new(
            w.wire_version,
            w.schema_version,
            w.number,
            w.document_date,
            w.posting_date,
            w.description,
            w.postings,
            w.attachment_ref,
            w.chart_version,
            w.buyer_vat_id,
            w.vat_expansion,
        )
        .map_err(D::Error::custom)
    }
}
