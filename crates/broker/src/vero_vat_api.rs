//! Narrow ADR-9 adapter for Vero SelfAssessedTaxes VAT operations.
//!
//! This module owns the sole numbered-field projection, the three response
//! schemas, and the TLS/mTLS HTTP boundary. It is intentionally not a general
//! Vero SDK.

use std::str::FromStr;
use std::time::Duration;

use aggountant_domain::{VatReturnField, VatReturnFields};
use aggountant_model::{BusinessId, FilingContactName, FilingContactPhone, PeriodId};
use chrono::{Datelike, NaiveDate};
use reqwest::blocking::Client;
use reqwest::{StatusCode, Url};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

use crate::{
    vero_filing::VeroCredentials, AdapterError, AdapterKind, AdapterResponse, CredentialHandle,
    DeliverySemantics, Envelope, RawResponse, RequestResponse, Transport, TransportError,
};

pub const SVC_VERO_VAT_PERIODS: &str = "vero-vat-periods";
pub const SVC_VERO_FILE_VAT_RETURN: &str = "vero-file-vat-return";
pub const SVC_VERO_FILED_VAT_RETURN: &str = "vero-filed-vat-return";

/// Fixed SelfAssessedTaxes production operation paths.
pub const GET_VAT_PERIODS_PATH: &str = "/Return/SAT/GetVATPeriods/v1";
pub const FILE_VAT_RETURN_PATH: &str = "/Return/SAT/FileVATReturn/v2";
pub const GET_FILED_VAT_RETURN_PATH: &str = "/Return/SAT/GetFiledVATReturn/v2";

/// The test service exposes the operations without the production SAT prefix.
const TEST_GET_VAT_PERIODS_PATH: &str = "/GetVATPeriods/v1";
const TEST_FILE_VAT_RETURN_PATH: &str = "/FileVATReturn/v2";
const TEST_GET_FILED_VAT_RETURN_PATH: &str = "/GetFiledVATReturn/v2";

/// Fixed API Portal integration-verification endpoints. These are deliberately
/// separate from the production filing paths and are never caller-configurable.
pub const PORTAL_GET_VAT_PERIODS_PATH: &str = "/Return/SAT/Verification/GetVATPeriods/v1";
pub const PORTAL_FILE_VAT_RETURN_PATH: &str = "/Return/SAT/Verification/FileVATReturn/v2";
pub const PORTAL_GET_FILED_VAT_RETURN_PATH: &str = "/Return/SAT/Verification/GetFiledVATReturn/v2";

pub const PORTAL_VERIFICATION_OPERATIONS: [&str; 3] = [
    "GetVATPeriods/v1",
    "FileVATReturn/v2",
    "GetFiledVATReturn/v2",
];

pub fn portal_get_vat_periods_request() -> Vec<u8> {
    br#"{"BusinessId":"1572860-0","FilingYear":2023}"#.to_vec()
}

pub fn portal_file_vat_return_request() -> Vec<u8> {
    br#"{"BusinessId":"1572860-0","FilingPeriod":"2023-01-31","ContactDetails":{"FullName":"Integration verification","PhoneNumber":"029512000"},"NoActivity":true,"ReplacementReturn":false}"#.to_vec()
}

pub fn portal_get_filed_vat_return_request() -> Vec<u8> {
    br#"{"BusinessId":"1572860-0","FilingPeriod":"2023-01-31"}"#.to_vec()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct ContactDetails {
    pub full_name: String,
    pub phone_number: String,
}

/// Final FileVATReturn/v2 request. `vat_details` contains only API fields; local
/// field 308 is deliberately never represented here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct FileVatReturn {
    pub business_id: String,
    pub filing_period: String,
    pub contact_details: ContactDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_activity: Option<bool>,
    #[serde(rename = "VATDetails", skip_serializing_if = "Option::is_none")]
    pub vat_details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_return: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_reason: Option<String>,
}

impl FileVatReturn {
    pub fn bytes(&self) -> Result<Vec<u8>, AdapterError> {
        serde_json::to_vec(self)
            .map_err(|_| AdapterError::Malformed("FileVATReturn serialization failed".into()))
    }

    pub fn sent_hash(&self) -> Result<String, AdapterError> {
        let mut hash = Sha256::new();
        hash.update(self.bytes()?);
        Ok(hex::encode(hash.finalize()))
    }
}

/// Project sealed, numbered fields exactly once into FileVATReturn/v2.
pub fn project_file_vat_return(
    period: &PeriodId,
    fields: &VatReturnFields,
    business_id: &BusinessId,
    contact_name: &FilingContactName,
    contact_phone: &FilingContactPhone,
) -> Result<FileVatReturn, AdapterError> {
    let mut details = Map::new();
    for (field, money) in fields.iter() {
        if field == VatReturnField::Field308 {
            continue;
        }
        let (group, name) = api_field(field);
        let amount = decimal_number(&money.amount_str())?;
        if group.is_empty() {
            details.insert(name.into(), amount);
        } else {
            let group_value = details
                .entry(group)
                .or_insert_with(|| Value::Object(Map::new()));
            let object = group_value
                .as_object_mut()
                .expect("projection groups are objects");
            object.insert(name.into(), amount);
        }
    }
    validate_cross_fields(fields)?;
    let no_activity = details.is_empty();
    Ok(FileVatReturn {
        business_id: business_id.as_str().into(),
        filing_period: filing_period_end(period)?,
        contact_details: ContactDetails {
            full_name: contact_name.as_str().into(),
            phone_number: contact_phone.as_str().into(),
        },
        no_activity: Some(no_activity),
        vat_details: (!no_activity).then_some(Value::Object(details)),
        replacement_return: Some(false),
        replacement_reason: None,
    })
}

fn filing_period_end(period: &PeriodId) -> Result<String, AdapterError> {
    let (_, last_month) = period.month_bounds();
    let (next_year, next_month) = if last_month == 12 {
        (period.year() + 1, 1)
    } else {
        (period.year(), u32::from(last_month) + 1)
    };
    let next = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .ok_or_else(|| AdapterError::Malformed("VAT filing period date is invalid".into()))?;
    let end = next
        .pred_opt()
        .ok_or_else(|| AdapterError::Malformed("VAT filing period date underflow".into()))?;
    debug_assert_eq!(end.month(), u32::from(last_month));
    Ok(end.format("%Y-%m-%d").to_string())
}

fn api_field(field: VatReturnField) -> (&'static str, &'static str) {
    use VatReturnField::*;
    match field {
        Field301 => ("VATOnDomesticSalesByTaxRate", "HighVATRate"),
        Field302 => ("VATOnDomesticSalesByTaxRate", "MediumVATRate"),
        Field303 => ("VATOnDomesticSalesByTaxRate", "LowVATRate"),
        Field305 => ("VATOnPurchasesAndImports", "GoodsFromEUMemberStates"),
        Field306 => ("VATOnPurchasesAndImports", "ServicesFromEUMemberStates"),
        Field307 => ("", "DeductibleVAT"),
        Field308 => unreachable!("field 308 is local-only"),
        Field309 => ("SalesPurchasesImports", "ZeroVATRateTurnover"),
        Field310 => ("SalesPurchasesImports", "SalesOfGoodsToEUMemberStates"),
        Field311 => ("SalesPurchasesImports", "SalesOfServicesToEUMemberStates"),
        Field312 => (
            "SalesPurchasesImports",
            "PurchasesOfGoodsFromEUMemberStates",
        ),
        Field313 => (
            "SalesPurchasesImports",
            "PurchasesOfServicesFromEUMemberStates",
        ),
        Field314 => ("SalesPurchasesImports", "ImportsOfGoodsFromOutsideEU"),
        Field315 => ("VATOnPurchasesAndImports", "ImportOfGoodsOutsideEU"),
        Field316 => (
            "SalesPurchasesImports",
            "SalesOfConstructionServicesAndScrapMetal",
        ),
        Field317 => (
            "SalesPurchasesImports",
            "PurchasesOfConstructionServicesAndScrapMetal",
        ),
        Field318 => (
            "VATOnPurchasesAndImports",
            "ConstructionServicesAndScrapMetal",
        ),
    }
}

fn validate_cross_fields(fields: &VatReturnFields) -> Result<(), AdapterError> {
    for (tax, base) in [
        (VatReturnField::Field305, VatReturnField::Field312),
        (VatReturnField::Field306, VatReturnField::Field313),
        (VatReturnField::Field315, VatReturnField::Field314),
        (VatReturnField::Field318, VatReturnField::Field317),
    ] {
        if fields.get(tax).is_some() && fields.get(base).is_none() {
            return Err(AdapterError::Rejected {
                code: "VERO_VAT_CROSS_FIELD".into(),
                message: format!(
                    "VAT return fields {} and {} must be reported together",
                    tax.number(),
                    base.number()
                ),
            });
        }
    }
    Ok(())
}

fn decimal_number(value: &str) -> Result<Value, AdapterError> {
    Decimal::from_str(value).map_err(|_| AdapterError::Malformed("invalid VAT decimal".into()))?;
    Number::from_str(value)
        .map(Value::Number)
        .map_err(|_| AdapterError::Malformed("VAT decimal cannot be represented as JSON".into()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GetVatPeriodsResponse {
    #[serde(rename = "FilingPeriod")]
    pub vat_periods: Vec<VatPeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct VatPeriod {
    pub period: String,
    pub frequency: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
    #[serde(default)]
    pub customs_comparison_data_on_imports_of_goods: Option<Value>,
    pub due_date: String,
}

/// The FileVATReturn receipt. Unknown fields are accepted: a field the Tax
/// Administration adds to its answer must not turn a delivered filing into an
/// unreadable one. `encode()` stores only the two fields that prove receipt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct FileVatReceipt {
    pub unique_identifier: String,
    pub accepted_timestamp: String,
}

impl FileVatReceipt {
    pub fn encode(&self) -> Result<String, AdapterError> {
        serde_json::to_string(self)
            .map_err(|_| AdapterError::Malformed("receipt serialization failed".into()))
    }
    pub fn decode(value: &str) -> Result<Self, AdapterError> {
        let receipt: Self = serde_json::from_str(value)
            .map_err(|_| AdapterError::Malformed("incomplete FileVATReturn receipt".into()))?;
        if !receipt.is_complete() {
            return Err(AdapterError::Malformed(
                "HTTP 200 omitted a FileVATReturn receipt field".into(),
            ));
        }
        Ok(receipt)
    }

    fn is_complete(&self) -> bool {
        !self.unique_identifier.trim().is_empty() && !self.accepted_timestamp.trim().is_empty()
    }
}

/// Classify a FileVATReturn answer in the 2xx range. A success status is never a
/// rejection: only a 200 carrying both receipt fields proves delivery, and any
/// other 2xx (a missing or empty field, an unreadable body, a status other than
/// 200) may still mean the return was accepted, so its outcome is unknown and is
/// resolved by asking the Tax Administration, never by sending again.
/// `body` is `None` when the answer was not JSON. Only a JSON object can be a
/// receipt; serde would otherwise read a two-element array positionally.
fn file_vat_return_success(
    status: u16,
    body: Option<Value>,
) -> Result<AdapterResponse, AdapterError> {
    let receipt = body
        .filter(Value::is_object)
        .and_then(|body| serde_json::from_value::<FileVatReceipt>(body).ok());
    match receipt {
        Some(receipt) if status == 200 && receipt.is_complete() => Ok(AdapterResponse::Accepted {
            receipt: receipt.encode()?,
            body: serde_json::to_value(&receipt)
                .map_err(|_| AdapterError::Malformed("receipt serialization failed".into()))?,
        }),
        _ => Err(AdapterError::AmbiguousDelivery {
            code: format!("HTTP_{status}_WITHOUT_RECEIPT"),
            detail: "Vero answered with a success status but no readable receipt; the filing \
                     may have been accepted"
                .into(),
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct VeroApiError {
    pub error_code: String,
    pub error_text: String,
    #[serde(default)]
    pub error_details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FiledVatReturn {
    #[serde(rename = "Status")]
    pub status: Option<String>,
    #[serde(flatten)]
    pub values: Map<String, Value>,
}

impl<'de> Deserialize<'de> for FiledVatReturn {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut values = Map::<String, Value>::deserialize(deserializer)?;
        let status = values
            .remove("Status")
            .and_then(|value| value.as_str().map(ToOwned::to_owned));
        Ok(Self { status, values })
    }
}

/// One amount where the authority's processed return differs from the approved
/// request. `approved` and `authority` are JSON numbers; `authority` is the raw
/// answer when it is not an amount at all. Amounts only, never personal data.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VatValueDifference {
    /// The FileVATReturn/v2 request path, such as
    /// `VATOnDomesticSalesByTaxRate.HighVATRate`.
    pub field: String,
    /// The VAT return field number (301 to 318) when the path is one aggountant
    /// sends.
    pub return_field: Option<u16>,
    pub approved: Value,
    pub authority: Value,
}

impl VatValueDifference {
    /// `301 VATOnDomesticSalesByTaxRate.HighVATRate: approved 255, authority 0`.
    pub fn describe(&self) -> String {
        let field = match self.return_field {
            Some(number) => format!("{number} {}", self.field),
            None => self.field.clone(),
        };
        format!(
            "{field}: approved {}, authority {}",
            self.approved, self.authority
        )
    }
}

/// The amount groups of the request and where GetFiledVATReturn/v2 reports them.
/// `DeductibleVAT` is a top-level request amount that the answer nests in
/// `DeductibleTax`, beside VAT-relief keys aggountant never sends.
const AMOUNT_GROUPS: [(&str, &str); 3] = [
    ("VATOnDomesticSalesByTaxRate", "VATOnDomesticSales"),
    ("VATOnPurchasesAndImports", "VATOnPurchasesAndImports"),
    ("SalesPurchasesImports", "SalesPurchasesImports"),
];

/// The top-level answer keys the comparison reads or deliberately leaves out;
/// `Status` is held apart. The net tax is derived from the compared amounts.
const ANSWER_KEYS_READ: [&str; 7] = [
    "SubmittedDate",
    "VATOnCashBasis",
    "VATOnDomesticSales",
    "VATOnPurchasesAndImports",
    "DeductibleTax",
    "SalesPurchasesImports",
    "TaxPayableOrNegativeTaxThatQualifiesForRefund",
];

impl FiledVatReturn {
    pub fn semantically_matches(&self, expected: &FileVatReturn) -> bool {
        self.semantically_matches_attempt(expected, None)
    }

    /// The approved return, processed on or after `earliest_submitted_date`, with
    /// every amount equal to the approved request.
    pub fn semantically_matches_attempt(
        &self,
        expected: &FileVatReturn,
        earliest_submitted_date: Option<&str>,
    ) -> bool {
        self.processed_differences(expected, earliest_submitted_date)
            .is_some_and(|differences| differences.is_empty())
    }

    /// How a processed return differs from the approved request: empty when the
    /// amounts match. `None` when the answer is not a processed return submitted on
    /// or after `earliest_submitted_date`, so its amounts say nothing about this
    /// filing. Being Processed may still expose the previous return's values, and
    /// an answer without a status proves nothing either.
    pub fn processed_differences(
        &self,
        expected: &FileVatReturn,
        earliest_submitted_date: Option<&str>,
    ) -> Option<Vec<VatValueDifference>> {
        if self.status.as_deref() != Some("Processed") {
            return None;
        }
        if let Some(earliest) = earliest_submitted_date {
            let submitted = self.values.get("SubmittedDate").and_then(Value::as_str)?;
            if submitted < earliest {
                return None;
            }
        }
        Some(self.value_differences(expected))
    }

    /// Compare the amounts of this answer with the approved request, field by
    /// field. GetFiledVATReturn/v2 returns every amount of every group (zeros
    /// included) while the request carries only the fields that were reported, so
    /// a field missing or null on either side is zero, and numbers and numeric
    /// strings compare as decimals. A value the request never carries (an unknown
    /// key, VAT relief, payment-basis accounting) is ignored only while it holds
    /// nothing: an amount the comparison does not know must never let a
    /// different return pass as the approved one. The net tax is derived from the
    /// compared amounts and is not compared itself.
    pub fn value_differences(&self, expected: &FileVatReturn) -> Vec<VatValueDifference> {
        let mut differences = Vec::new();
        let empty = Map::new();
        let details = match (expected.no_activity == Some(true), &expected.vat_details) {
            (true, None) => &empty,
            (false, Some(Value::Object(details))) => details,
            // An approved request that contradicts itself never matches anything.
            (_, other) => {
                differences.push(VatValueDifference {
                    field: "VATDetails".into(),
                    return_field: None,
                    approved: other.clone().unwrap_or(Value::Null),
                    authority: Value::Null,
                });
                return differences;
            }
        };

        for (request_group, answer_group) in AMOUNT_GROUPS {
            let approved = details.get(request_group);
            let authority = self.values.get(answer_group);
            let (Some(approved), Some(authority)) = (
                group_object(approved, &empty),
                group_object(authority, &empty),
            ) else {
                differences.push(VatValueDifference {
                    field: request_group.into(),
                    return_field: None,
                    approved: approved.cloned().unwrap_or(Value::Null),
                    authority: authority.cloned().unwrap_or(Value::Null),
                });
                continue;
            };
            for (name, value) in approved {
                compare_amount(
                    &mut differences,
                    format!("{request_group}.{name}"),
                    return_field_number(request_group, name),
                    Some(value),
                    authority.get(name),
                );
            }
            for (name, value) in authority {
                if !approved.contains_key(name) {
                    unapproved_value(
                        &mut differences,
                        format!("{request_group}.{name}"),
                        return_field_number(request_group, name),
                        value,
                    );
                }
            }
        }

        // The answer nests DeductibleVAT beside the VAT-relief fields of periods
        // before 2025, which aggountant never sends: relief there is another return.
        match group_object(self.values.get("DeductibleTax"), &empty) {
            Some(tax) => {
                compare_amount(
                    &mut differences,
                    "DeductibleVAT".into(),
                    return_field_number("", "DeductibleVAT"),
                    details.get("DeductibleVAT"),
                    tax.get("DeductibleVAT"),
                );
                for (name, value) in tax.iter().filter(|(name, _)| *name != "DeductibleVAT") {
                    unapproved_value(
                        &mut differences,
                        format!("DeductibleTax.{name}"),
                        None,
                        value,
                    );
                }
            }
            None => differences.push(VatValueDifference {
                field: "DeductibleVAT".into(),
                return_field: return_field_number("", "DeductibleVAT"),
                approved: details.get("DeductibleVAT").cloned().unwrap_or(Value::Null),
                authority: self.values["DeductibleTax"].clone(),
            }),
        }
        // Payment-basis accounting is never sent, so it can only be off.
        if let Some(value) = self.values.get("VATOnCashBasis") {
            unapproved_value(&mut differences, "VATOnCashBasis".into(), None, value);
        }
        // An amount anywhere else in the answer (a renamed or new group) is one
        // the comparison cannot place, so it is a difference rather than unseen.
        for (name, value) in &self.values {
            if !ANSWER_KEYS_READ.contains(&name.as_str()) && holds_amount(value) {
                differences.push(VatValueDifference {
                    field: name.clone(),
                    return_field: None,
                    approved: Value::Null,
                    authority: value.clone(),
                });
            }
        }

        // Anything else in the approved details is outside what aggountant sends;
        // it can only match when it carries nothing.
        for (name, value) in details {
            let known =
                name == "DeductibleVAT" || AMOUNT_GROUPS.iter().any(|(group, _)| group == name);
            if !known && carries_value(value) {
                differences.push(VatValueDifference {
                    field: name.clone(),
                    return_field: None,
                    approved: value.clone(),
                    authority: Value::Null,
                });
            }
        }
        differences
    }
}

/// A group as an object; a missing or null group has no amounts. Anything else
/// is not a shape the schema allows.
fn group_object<'a>(
    value: Option<&'a Value>,
    empty: &'a Map<String, Value>,
) -> Option<&'a Map<String, Value>> {
    match value {
        None | Some(Value::Null) => Some(empty),
        Some(Value::Object(object)) => Some(object),
        Some(_) => None,
    }
}

fn compare_amount(
    differences: &mut Vec<VatValueDifference>,
    field: String,
    return_field: Option<u16>,
    approved: Option<&Value>,
    authority: Option<&Value>,
) {
    let shown = |value: Option<&Value>, parsed: &Result<Decimal, ()>| match parsed {
        Ok(amount) => amount_json(*amount),
        Err(()) => value.cloned().unwrap_or(Value::Null),
    };
    let (left, right) = (amount(approved), amount(authority));
    if matches!((&left, &right), (Ok(a), Ok(b)) if a == b) {
        return;
    }
    differences.push(VatValueDifference {
        field,
        return_field,
        approved: shown(approved, &left),
        authority: shown(authority, &right),
    });
}

/// A value the authority shows where the approved request has none: a
/// difference unless it carries nothing (null, false or a zero amount).
fn unapproved_value(
    differences: &mut Vec<VatValueDifference>,
    field: String,
    return_field: Option<u16>,
    value: &Value,
) {
    if carries_value(value) {
        compare_amount(differences, field, return_field, None, Some(value));
    }
}

/// A non-zero number or numeric string anywhere inside `value`. Text such as an
/// identifier or a date is not an amount.
fn holds_amount(value: &Value) -> bool {
    match value {
        Value::Object(values) => values.values().any(holds_amount),
        Value::Array(values) => values.iter().any(holds_amount),
        Value::Number(_) | Value::String(_) => {
            matches!(amount(Some(value)), Ok(found) if !found.is_zero())
        }
        _ => false,
    }
}

fn carries_value(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => false,
        other => !matches!(amount(Some(other)), Ok(zero) if zero.is_zero()),
    }
}

/// An amount as a decimal: missing and null are zero, a JSON number or a numeric
/// string is its value, anything else is not an amount.
fn amount(value: Option<&Value>) -> Result<Decimal, ()> {
    let text = match value {
        None | Some(Value::Null) => return Ok(Decimal::ZERO),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::String(text)) => text.trim().to_owned(),
        Some(_) => return Err(()),
    };
    Decimal::from_str(&text)
        .or_else(|_| Decimal::from_scientific(&text))
        .map_err(|_| ())
}

fn amount_json(amount: Decimal) -> Value {
    let text = amount.normalize().to_string();
    Number::from_str(&text).map_or(Value::String(text), Value::Number)
}

/// The VAT return field number of a request path aggountant sends.
fn return_field_number(group: &str, name: &str) -> Option<u16> {
    VatReturnField::ALL
        .iter()
        .copied()
        .filter(|field| *field != VatReturnField::Field308)
        .find(|field| api_field(*field) == (group, name))
        .map(VatReturnField::number)
}

#[derive(Clone)]
struct HttpConfig {
    base_url: Url,
    timeout: Duration,
    expected_credential_target: Option<crate::vero_filing::VeroCredentialTarget>,
}

impl HttpConfig {
    fn new(base_url: &str, timeout: Duration) -> Result<Self, AdapterError> {
        let base_url = Url::parse(base_url)
            .map_err(|_| AdapterError::Malformed("invalid Vero API base URL".into()))?;
        if base_url.scheme() != "https" {
            return Err(AdapterError::Malformed("Vero API requires HTTPS".into()));
        }
        Ok(Self {
            base_url,
            timeout,
            expected_credential_target: None,
        })
    }

    fn portal(base_url: &str, timeout: Duration) -> Result<Self, AdapterError> {
        let mut config = Self::new(base_url, timeout)?;
        config.expected_credential_target =
            Some(crate::vero_filing::VeroCredentialTarget::Production);
        Ok(config)
    }

    fn loopback(base_url: &str, timeout: Duration) -> Result<Self, AdapterError> {
        let base_url = Url::parse(base_url)
            .map_err(|_| AdapterError::Malformed("invalid loopback URL".into()))?;
        if base_url.scheme() != "http"
            || !base_url
                .host_str()
                .is_some_and(|host| host == "localhost" || host == "127.0.0.1")
        {
            return Err(AdapterError::Malformed(
                "verification test URL must be HTTP loopback".into(),
            ));
        }
        Ok(Self {
            base_url,
            timeout,
            expected_credential_target: Some(crate::vero_filing::VeroCredentialTarget::Production),
        })
    }

    fn post(
        &self,
        path: &str,
        body: &[u8],
        handle: &CredentialHandle,
    ) -> Result<(StatusCode, Vec<u8>), AdapterError> {
        let credentials = VeroCredentials::parse(handle.expose_secret()).map_err(|_| {
            AdapterError::Unauthorized("Vero credential handle is incomplete".into())
        })?;
        if self
            .expected_credential_target
            .is_some_and(|target| credentials.target != target)
        {
            return Err(AdapterError::Unauthorized(
                "Vero credential target does not match the operation".into(),
            ));
        }
        let material = credentials
            .transport_material()
            .map_err(AdapterError::Unauthorized)?;
        let client = mtls_client(material.identity, material.trust_certificate, self.timeout)?;
        let url = self
            .base_url
            .join(path)
            .map_err(|_| AdapterError::Malformed("invalid Vero operation URL".into()))?;
        let mut request = client
            .post(url)
            .header("Vero-SoftwareKey", material.software_key)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "aggountant/0.1")
            .body(body.to_vec());
        if let Some(token) = credentials
            .authorization_token
            .filter(|v| !v.trim().is_empty())
        {
            request = request.header("Vero-AuthorizationToken", token);
        }
        let response = request.send().map_err(|error| {
            if error.is_timeout() {
                AdapterError::Timeout("no definitive Vero response".into())
            } else if error.is_connect() {
                AdapterError::RetryableTransport("Vero connection failed before a response".into())
            } else {
                AdapterError::AmbiguousDelivery {
                    code: "HTTP_TRANSPORT".into(),
                    detail: "Vero request ended without a definitive response".into(),
                }
            }
        })?;
        let status = response.status();
        let bytes = response
            .bytes()
            .map_err(|_| AdapterError::AmbiguousDelivery {
                code: "HTTP_BODY".into(),
                detail: "Vero response body was incomplete".into(),
            })?;
        Ok((status, bytes.to_vec()))
    }
}

pub struct VeroVatPeriodsAdapter {
    http: HttpConfig,
    path: &'static str,
}
pub struct VeroFileVatReturnAdapter {
    http: HttpConfig,
    path: &'static str,
}
pub struct VeroFiledVatReturnAdapter {
    http: HttpConfig,
    path: &'static str,
}

/// Deterministic seams for process/CLI tests. They share the live adapter's
/// schemas and deliberately have no synthetic 409 recovery.
pub struct ScriptedVeroVatPeriodsAdapter {
    transport: Box<dyn Transport>,
}
pub struct ScriptedVeroFileVatReturnAdapter {
    transport: Box<dyn Transport>,
}
pub struct ScriptedVeroFiledVatReturnAdapter {
    transport: Box<dyn Transport>,
}

impl ScriptedVeroVatPeriodsAdapter {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }
}
impl ScriptedVeroFileVatReturnAdapter {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }
}
impl ScriptedVeroFiledVatReturnAdapter {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }
}

fn scripted_round_trip(
    transport: &dyn Transport,
    envelope: &Envelope,
) -> Result<RawResponse, AdapterError> {
    let request = crate::RawRequest {
        service: envelope.service.clone(),
        operation: envelope.operation.clone(),
        idempotency_key: envelope.natural_key.clone(),
        body: envelope.body.clone(),
    };
    transport.round_trip(&request).map_err(|error| match error {
        TransportError::Timeout => AdapterError::Timeout("no definitive Vero response".into()),
        TransportError::ConnectionRefused(_) => {
            AdapterError::RetryableTransport("Vero connection refused".into())
        }
    })
}

macro_rules! constructor {
    ($name:ident, $production_path:expr, $test_path:expr, $verification_path:expr) => {
        impl $name {
            pub fn new(base_url: &str, timeout: Duration) -> Result<Self, AdapterError> {
                let http = HttpConfig::new(base_url, timeout)?;
                let path = if http.base_url.as_str() == crate::vero_filing::VERO_PRODUCTION_BASE_URL
                {
                    $production_path
                } else {
                    $test_path
                };
                Ok(Self { http, path })
            }

            pub fn new_portal_verification(timeout: Duration) -> Result<Self, AdapterError> {
                Ok(Self {
                    http: HttpConfig::portal(
                        crate::vero_filing::VERO_PRODUCTION_BASE_URL,
                        timeout,
                    )?,
                    path: $verification_path,
                })
            }

            /// Deterministic integration-test seam. It is deliberately restricted
            /// to plain-HTTP loopback and cannot redirect a real verification call.
            #[doc(hidden)]
            pub fn new_portal_verification_loopback(
                base_url: &str,
                timeout: Duration,
            ) -> Result<Self, AdapterError> {
                Ok(Self {
                    http: HttpConfig::loopback(base_url, timeout)?,
                    path: $verification_path,
                })
            }
        }
    };
}
constructor!(
    VeroVatPeriodsAdapter,
    GET_VAT_PERIODS_PATH,
    TEST_GET_VAT_PERIODS_PATH,
    PORTAL_GET_VAT_PERIODS_PATH
);
constructor!(
    VeroFileVatReturnAdapter,
    FILE_VAT_RETURN_PATH,
    TEST_FILE_VAT_RETURN_PATH,
    PORTAL_FILE_VAT_RETURN_PATH
);
constructor!(
    VeroFiledVatReturnAdapter,
    GET_FILED_VAT_RETURN_PATH,
    TEST_GET_FILED_VAT_RETURN_PATH,
    PORTAL_GET_FILED_VAT_RETURN_PATH
);

fn request_bytes(envelope: &Envelope) -> Result<Vec<u8>, AdapterError> {
    envelope
        .body
        .get("request_bytes")
        .and_then(Value::as_str)
        .map(str::as_bytes)
        .map(ToOwned::to_owned)
        .ok_or_else(|| AdapterError::Malformed("Vero envelope lacks bound request_bytes".into()))
}

fn read_call(
    http: &HttpConfig,
    path: &str,
    envelope: &Envelope,
    cred: &CredentialHandle,
) -> Result<AdapterResponse, AdapterError> {
    let body = request_bytes(envelope)?;
    let (status, response) = http.post(path, &body, cred)?;
    match status.as_u16() {
        200 => Ok(AdapterResponse::Accepted {
            receipt: String::new(),
            body: serde_json::from_slice(&response)
                .map_err(|_| AdapterError::Malformed("Vero read response is not JSON".into()))?,
        }),
        401 | 403 => Err(AdapterError::Unauthorized(format!(
            "HTTP_{}",
            status.as_u16()
        ))),
        500..=599 => Err(AdapterError::RetryableTransport(format!(
            "HTTP_{}",
            status.as_u16()
        ))),
        other => Err(AdapterError::Rejected {
            code: format!("HTTP_{other}"),
            message: "Vero read was rejected".into(),
        }),
    }
}

impl RequestResponse for VeroVatPeriodsAdapter {
    fn service(&self) -> &str {
        SVC_VERO_VAT_PERIODS
    }
    fn kind(&self) -> AdapterKind {
        AdapterKind::Pull
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtLeastOnce
    }
    fn call(
        &self,
        envelope: &Envelope,
        cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        let response = read_call(&self.http, self.path, envelope, cred)?;
        if let AdapterResponse::Accepted { body, .. } = &response {
            let _: GetVatPeriodsResponse = serde_json::from_value(body.clone()).map_err(|_| {
                AdapterError::Malformed("GetVATPeriods response is incomplete".into())
            })?;
        }
        Ok(response)
    }
}

impl RequestResponse for VeroFiledVatReturnAdapter {
    fn service(&self) -> &str {
        SVC_VERO_FILED_VAT_RETURN
    }
    fn kind(&self) -> AdapterKind {
        AdapterKind::Pull
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtLeastOnce
    }
    fn call(
        &self,
        envelope: &Envelope,
        cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        let response = read_call(&self.http, self.path, envelope, cred)?;
        if let AdapterResponse::Accepted { body, .. } = &response {
            let _: FiledVatReturn = serde_json::from_value(body.clone()).map_err(|_| {
                AdapterError::Malformed("GetFiledVATReturn response is incomplete".into())
            })?;
        }
        Ok(response)
    }
}

impl RequestResponse for ScriptedVeroVatPeriodsAdapter {
    fn service(&self) -> &str {
        SVC_VERO_VAT_PERIODS
    }
    fn kind(&self) -> AdapterKind {
        AdapterKind::Pull
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtLeastOnce
    }
    fn call(
        &self,
        envelope: &Envelope,
        _cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        let response = scripted_round_trip(self.transport.as_ref(), envelope)?;
        if response.status != 200 {
            return Err(AdapterError::Rejected {
                code: format!("HTTP_{}", response.status),
                message: "GetVATPeriods rejected".into(),
            });
        }
        let _: GetVatPeriodsResponse = serde_json::from_value(response.body.clone())
            .map_err(|_| AdapterError::Malformed("GetVATPeriods response is incomplete".into()))?;
        Ok(AdapterResponse::Accepted {
            receipt: String::new(),
            body: response.body,
        })
    }
}

impl RequestResponse for ScriptedVeroFiledVatReturnAdapter {
    fn service(&self) -> &str {
        SVC_VERO_FILED_VAT_RETURN
    }
    fn kind(&self) -> AdapterKind {
        AdapterKind::Pull
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtLeastOnce
    }
    fn call(
        &self,
        envelope: &Envelope,
        _cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        let response = scripted_round_trip(self.transport.as_ref(), envelope)?;
        if response.status != 200 {
            return Err(AdapterError::Rejected {
                code: format!("HTTP_{}", response.status),
                message: "GetFiledVATReturn rejected".into(),
            });
        }
        let _: FiledVatReturn = serde_json::from_value(response.body.clone()).map_err(|_| {
            AdapterError::Malformed("GetFiledVATReturn response is incomplete".into())
        })?;
        Ok(AdapterResponse::Accepted {
            receipt: String::new(),
            body: response.body,
        })
    }
}

impl RequestResponse for ScriptedVeroFileVatReturnAdapter {
    fn service(&self) -> &str {
        SVC_VERO_FILE_VAT_RETURN
    }
    fn kind(&self) -> AdapterKind {
        AdapterKind::Submit
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtMostOnce
    }
    fn call(
        &self,
        envelope: &Envelope,
        _cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        let response = scripted_round_trip(self.transport.as_ref(), envelope)?;
        match response.status {
            status @ 200..=299 => file_vat_return_success(status, Some(response.body)),
            400 => {
                let error: VeroApiError = serde_json::from_value(response.body).map_err(|_| {
                    AdapterError::Malformed("Vero 400 response is not structured".into())
                })?;
                {
                    let details = error
                        .error_details
                        .map(|value| format!("; ErrorDetails={value}"))
                        .unwrap_or_default();
                    Err(AdapterError::Rejected {
                        code: error.error_code,
                        message: format!("{}{details}", error.error_text),
                    })
                }
            }
            500..=599 => Err(AdapterError::AmbiguousDelivery {
                code: format!("HTTP_{}", response.status),
                detail: "Vero may have accepted the filing".into(),
            }),
            other => Err(AdapterError::Rejected {
                code: format!("HTTP_{other}"),
                message: "unexpected FileVATReturn response".into(),
            }),
        }
    }
}

impl RequestResponse for VeroFileVatReturnAdapter {
    fn service(&self) -> &str {
        SVC_VERO_FILE_VAT_RETURN
    }
    fn kind(&self) -> AdapterKind {
        AdapterKind::Submit
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtMostOnce
    }
    fn call(
        &self,
        envelope: &Envelope,
        cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        let body = request_bytes(envelope)?;
        let (status, response) = self.http.post(self.path, &body, cred)?;
        match status.as_u16() {
            status @ 200..=299 => {
                file_vat_return_success(status, serde_json::from_slice(&response).ok())
            }
            400 => {
                let error: VeroApiError = serde_json::from_slice(&response).map_err(|_| {
                    AdapterError::Malformed("Vero 400 response is not a structured error".into())
                })?;
                {
                    let details = error
                        .error_details
                        .map(|value| format!("; ErrorDetails={value}"))
                        .unwrap_or_default();
                    Err(AdapterError::Rejected {
                        code: error.error_code,
                        message: format!("{}{details}", error.error_text),
                    })
                }
            }
            401 | 403 => Err(AdapterError::Unauthorized(format!(
                "HTTP_{}",
                status.as_u16()
            ))),
            500..=599 => Err(AdapterError::AmbiguousDelivery {
                code: format!("HTTP_{}", status.as_u16()),
                detail: "Vero may have accepted the filing".into(),
            }),
            other => Err(AdapterError::Rejected {
                code: format!("HTTP_{other}"),
                message: "unexpected FileVATReturn response".into(),
            }),
        }
    }
}

/// Shared mTLS boundary: retain public roots and hostname validation. Never follow
/// redirects with authority credentials or a signed report.
pub(crate) fn mtls_client(
    identity: reqwest::Identity,
    trust: reqwest::Certificate,
    timeout: Duration,
) -> Result<Client, AdapterError> {
    Client::builder()
        .identity(identity)
        .add_root_certificate(trust)
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(|_| AdapterError::Unauthorized("mTLS client configuration failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aggountant_model::{Currency, Money};
    use serde_json::json;

    fn eur(value: &str) -> Money {
        Money::parse(value, Currency::new("EUR").unwrap()).unwrap()
    }

    #[test]
    fn a_no_activity_return_matches_only_a_remote_return_without_amounts() {
        let expected = project_file_vat_return(
            &PeriodId::parse("2026-07").unwrap(),
            &VatReturnFields::new(),
            &BusinessId::parse("1234567-1").unwrap(),
            &FilingContactName::parse("Esimerkki Oy").unwrap(),
            &FilingContactPhone::parse("+358401234567").unwrap(),
        )
        .unwrap();
        assert_eq!(expected.no_activity, Some(true));
        let filed = |body: Value| -> FiledVatReturn { serde_json::from_value(body).unwrap() };
        assert!(filed(json!({"Status": "Processed"})).semantically_matches(&expected));
        // Without a status nothing is proven, even with a plausible submission date.
        assert!(!filed(json!({"SubmittedDate": "2099-01-01"}))
            .semantically_matches_attempt(&expected, Some("2026-08-01")));
        assert!(filed(json!({
            "Status": "Processed",
            "VATOnDomesticSales": {"HighVATRate": 0.0},
            "DeductibleTax": {"DeductibleVAT": 0},
        }))
        .semantically_matches(&expected));
        for amount in [json!(255.0), json!("255.00")] {
            assert!(!filed(json!({
                "Status": "Processed",
                "VATOnDomesticSales": {"HighVATRate": amount},
            }))
            .semantically_matches(&expected));
        }
    }

    #[test]
    fn fixed_operation_paths_distinguish_production_test_and_verification() {
        let timeout = Duration::from_secs(1);

        assert_eq!(
            VeroVatPeriodsAdapter::new(crate::vero_filing::VERO_PRODUCTION_BASE_URL, timeout)
                .unwrap()
                .path,
            "/Return/SAT/GetVATPeriods/v1"
        );
        assert_eq!(
            VeroFileVatReturnAdapter::new(crate::vero_filing::VERO_PRODUCTION_BASE_URL, timeout)
                .unwrap()
                .path,
            "/Return/SAT/FileVATReturn/v2"
        );
        assert_eq!(
            VeroFiledVatReturnAdapter::new(crate::vero_filing::VERO_PRODUCTION_BASE_URL, timeout)
                .unwrap()
                .path,
            "/Return/SAT/GetFiledVATReturn/v2"
        );

        assert_eq!(
            VeroVatPeriodsAdapter::new(crate::vero_filing::VERO_TEST_BASE_URL, timeout)
                .unwrap()
                .path,
            "/GetVATPeriods/v1"
        );
        assert_eq!(
            VeroFileVatReturnAdapter::new(crate::vero_filing::VERO_TEST_BASE_URL, timeout)
                .unwrap()
                .path,
            "/FileVATReturn/v2"
        );
        assert_eq!(
            VeroFiledVatReturnAdapter::new(crate::vero_filing::VERO_TEST_BASE_URL, timeout)
                .unwrap()
                .path,
            "/GetFiledVATReturn/v2"
        );

        assert_eq!(
            VeroVatPeriodsAdapter::new_portal_verification(timeout)
                .unwrap()
                .path,
            "/Return/SAT/Verification/GetVATPeriods/v1"
        );
        assert_eq!(
            VeroFileVatReturnAdapter::new_portal_verification(timeout)
                .unwrap()
                .path,
            "/Return/SAT/Verification/FileVATReturn/v2"
        );
        assert_eq!(
            VeroFiledVatReturnAdapter::new_portal_verification(timeout)
                .unwrap()
                .path,
            "/Return/SAT/Verification/GetFiledVATReturn/v2"
        );
    }

    #[test]
    fn every_supported_numbered_field_has_one_nested_api_destination() {
        let mut fields = VatReturnFields::new();
        for field in VatReturnField::ALL {
            fields.try_insert(*field, eur("1.00")).unwrap();
        }
        let request = project_file_vat_return(
            &PeriodId::parse("2026-Q1").unwrap(),
            &fields,
            &BusinessId::parse("1572860-0").unwrap(),
            &FilingContactName::parse("Ada").unwrap(),
            &FilingContactPhone::parse("04012345").unwrap(),
        )
        .unwrap();
        let value = serde_json::to_value(request).unwrap();
        let details = value["VATDetails"].as_object().unwrap();
        let expected_paths = [
            ("VATOnDomesticSalesByTaxRate", "HighVATRate"),
            ("VATOnDomesticSalesByTaxRate", "MediumVATRate"),
            ("VATOnDomesticSalesByTaxRate", "LowVATRate"),
            ("VATOnPurchasesAndImports", "GoodsFromEUMemberStates"),
            ("VATOnPurchasesAndImports", "ServicesFromEUMemberStates"),
            ("SalesPurchasesImports", "ZeroVATRateTurnover"),
            ("SalesPurchasesImports", "SalesOfGoodsToEUMemberStates"),
            ("SalesPurchasesImports", "SalesOfServicesToEUMemberStates"),
            (
                "SalesPurchasesImports",
                "PurchasesOfGoodsFromEUMemberStates",
            ),
            (
                "SalesPurchasesImports",
                "PurchasesOfServicesFromEUMemberStates",
            ),
            ("SalesPurchasesImports", "ImportsOfGoodsFromOutsideEU"),
            ("VATOnPurchasesAndImports", "ImportOfGoodsOutsideEU"),
            (
                "SalesPurchasesImports",
                "SalesOfConstructionServicesAndScrapMetal",
            ),
            (
                "SalesPurchasesImports",
                "PurchasesOfConstructionServicesAndScrapMetal",
            ),
            (
                "VATOnPurchasesAndImports",
                "ConstructionServicesAndScrapMetal",
            ),
        ];
        for (group, field) in expected_paths {
            assert!(
                details[group].get(field).is_some(),
                "missing {group}.{field}"
            );
        }
        assert!(details.get("DeductibleVAT").is_some());
        let leaf_count: usize = details
            .values()
            .map(|group| group.as_object().map_or(1, Map::len))
            .sum();
        assert_eq!(leaf_count, VatReturnField::ALL.len() - 1);
        assert!(!value.to_string().contains("308"));
    }

    #[test]
    fn projection_omits_308_and_requires_bases_for_purchase_vat() {
        let mut fields = VatReturnFields::new();
        fields
            .try_insert(VatReturnField::Field307, eur("12.00"))
            .unwrap();
        fields
            .try_insert(VatReturnField::Field308, eur("-12.00"))
            .unwrap();
        let request = project_file_vat_return(
            &PeriodId::parse("2026-07").unwrap(),
            &fields,
            &BusinessId::parse("1572860-0").unwrap(),
            &FilingContactName::parse("Ada Operator").unwrap(),
            &FilingContactPhone::parse("+358401234567").unwrap(),
        )
        .unwrap();
        let json = serde_json::to_value(request).unwrap();
        assert_eq!(json["FilingPeriod"], "2026-07-31");
        assert!(json.to_string().contains("12.00"));
        assert!(!json.to_string().contains("308"));

        fields
            .try_insert(VatReturnField::Field315, eur("1.00"))
            .unwrap();
        assert!(project_file_vat_return(
            &PeriodId::parse("2026-07").unwrap(),
            &fields,
            &BusinessId::parse("1572860-0").unwrap(),
            &FilingContactName::parse("Ada").unwrap(),
            &FilingContactPhone::parse("04012345").unwrap(),
        )
        .is_err());

        let mut base_only = VatReturnFields::new();
        base_only
            .try_insert(VatReturnField::Field314, eur("1.00"))
            .unwrap();
        assert!(project_file_vat_return(
            &PeriodId::parse("2026-07").unwrap(),
            &base_only,
            &BusinessId::parse("1572860-0").unwrap(),
            &FilingContactName::parse("Ada").unwrap(),
            &FilingContactPhone::parse("04012345").unwrap(),
        )
        .is_ok());
    }

    /// Esimerkki Oy's October: 25.5 % domestic sales with 255,00 VAT and one
    /// purchase deduction of 102,00, as `file-vat` projects and approves it.
    fn october_request() -> FileVatReturn {
        let mut fields = VatReturnFields::new();
        fields
            .try_insert(VatReturnField::Field301, eur("255.00"))
            .unwrap();
        fields
            .try_insert(VatReturnField::Field307, eur("102.00"))
            .unwrap();
        fields
            .try_insert(VatReturnField::Field308, eur("153.00"))
            .unwrap();
        project_file_vat_return(
            &PeriodId::parse("2026-10").unwrap(),
            &fields,
            &BusinessId::parse("1234567-1").unwrap(),
            &FilingContactName::parse("Esimerkki Oy").unwrap(),
            &FilingContactPhone::parse("+358401234567").unwrap(),
        )
        .unwrap()
    }

    /// A GetFiledVATReturn/v2 answer for October as the schema shapes it: every
    /// required amount present, zeros included, plus keys the request never has.
    fn october_answer() -> Value {
        json!({
            "Status": "Processed",
            "SubmittedDate": "2026-11-12",
            "VATOnCashBasis": false,
            "VATOnDomesticSales": {"HighVATRate": 255.0, "MediumVATRate": 0.0, "LowVATRate": 0.0},
            "VATOnPurchasesAndImports": {
                "GoodsFromEUMemberStates": 0.0,
                "ServicesFromEUMemberStates": 0.0,
                "ImportOfGoodsOutsideEU": 0.0,
                "ConstructionServicesAndScrapMetal": 0.0
            },
            "DeductibleTax": {
                "DeductibleVAT": 102.0,
                "FileVATRelief": false,
                "TurnoverForVATRelief": 0.0,
                "TaxForVATRelief": 0.0,
                "AmountOfVATRelief": 0.0
            },
            "TaxPayableOrNegativeTaxThatQualifiesForRefund": 153.0,
            "SalesPurchasesImports": {
                "ZeroVATRateTurnover": 0.0,
                "SalesOfGoodsToEUMemberStates": 0.0,
                "SalesOfServicesToEUMemberStates": 0.0,
                "PurchasesOfGoodsFromEUMemberStates": 0.0,
                "PurchasesOfServicesFromEUMemberStates": 0.0,
                "ImportsOfGoodsFromOutsideEU": 0.0,
                "SalesOfConstructionServicesAndScrapMetal": 0.0,
                "PurchasesOfConstructionServicesAndScrapMetal": 0.0
            }
        })
    }

    fn differences(answer: Value) -> Vec<VatValueDifference> {
        let filed: FiledVatReturn = serde_json::from_value(answer).unwrap();
        filed.value_differences(&october_request())
    }

    #[test]
    fn the_schema_shaped_answer_matches_the_request_it_processed() {
        // The request carries two amounts; the answer carries every amount of
        // every group as zero plus the net tax and VAT-relief keys.
        assert_eq!(differences(october_answer()), vec![]);
        let filed: FiledVatReturn = serde_json::from_value(october_answer()).unwrap();
        assert!(filed.semantically_matches_attempt(&october_request(), Some("2026-11-01")));
    }

    #[test]
    fn structural_variants_of_the_same_amounts_match() {
        let mut variants = Vec::new();
        // A zero-valued group or key omitted by the authority, or null.
        let mut omitted = october_answer();
        let answer = omitted.as_object_mut().unwrap();
        answer.remove("VATOnPurchasesAndImports");
        answer["SalesPurchasesImports"] = Value::Null;
        answer["VATOnDomesticSales"]
            .as_object_mut()
            .unwrap()
            .remove("LowVATRate");
        variants.push(omitted);
        // Numeric strings instead of numbers, with other scales.
        let mut strings = october_answer();
        strings["VATOnDomesticSales"]["HighVATRate"] = json!("255");
        strings["VATOnDomesticSales"]["MediumVATRate"] = json!("0.00");
        strings["DeductibleTax"]["DeductibleVAT"] = json!(" 102.000 ");
        variants.push(strings);
        // An extra zero-valued key in a group, in another key order.
        let mut extra = october_answer();
        extra["VATOnDomesticSales"] = json!({
            "LowVATRate": 0, "ReducedVATRate135": 0.0, "MediumVATRate": 0, "HighVATRate": 255
        });
        variants.push(extra);
        // Exponent notation for a number.
        let mut exponent = october_answer();
        exponent["DeductibleTax"]["DeductibleVAT"] = serde_json::from_str("1.02e2").unwrap();
        variants.push(exponent);
        for variant in variants {
            assert_eq!(differences(variant.clone()), vec![], "{variant}");
        }

        // A key present with zero in the request but omitted from the answer.
        let mut request = october_request();
        request.vat_details.as_mut().unwrap()["VATOnPurchasesAndImports"] =
            json!({"GoodsFromEUMemberStates": 0.00});
        let mut answer = october_answer();
        answer
            .as_object_mut()
            .unwrap()
            .remove("VATOnPurchasesAndImports");
        let filed: FiledVatReturn = serde_json::from_value(answer).unwrap();
        assert_eq!(filed.value_differences(&request), vec![]);
    }

    #[test]
    fn a_genuine_amount_difference_is_named_with_both_values() {
        let mut answer = october_answer();
        answer["VATOnDomesticSales"]["HighVATRate"] = json!(250.0);
        answer["VATOnPurchasesAndImports"]["GoodsFromEUMemberStates"] = json!("12.50");
        let found = differences(answer.clone());
        assert_eq!(
            found,
            vec![
                VatValueDifference {
                    field: "VATOnDomesticSalesByTaxRate.HighVATRate".into(),
                    return_field: Some(301),
                    approved: json!(255),
                    authority: json!(250),
                },
                VatValueDifference {
                    field: "VATOnPurchasesAndImports.GoodsFromEUMemberStates".into(),
                    return_field: Some(305),
                    approved: json!(0),
                    authority: json!(12.5),
                },
            ]
        );
        assert_eq!(
            found[0].describe(),
            "301 VATOnDomesticSalesByTaxRate.HighVATRate: approved 255, authority 250"
        );
        let filed: FiledVatReturn = serde_json::from_value(answer).unwrap();
        assert!(!filed.semantically_matches(&october_request()));
        assert_eq!(
            filed.processed_differences(&october_request(), None),
            Some(found)
        );

        let mut deduction = october_answer();
        deduction["DeductibleTax"]["DeductibleVAT"] = json!(0.0);
        assert_eq!(
            differences(deduction),
            vec![VatValueDifference {
                field: "DeductibleVAT".into(),
                return_field: Some(307),
                approved: json!(102),
                authority: json!(0),
            }]
        );
    }

    #[test]
    fn an_amount_the_comparison_cannot_read_never_matches() {
        // An unknown key holding an amount: some other return, not this one.
        let mut extra = october_answer();
        extra["SalesPurchasesImports"]["SalesOfSomethingNew"] = json!(40.0);
        assert_eq!(
            differences(extra),
            vec![VatValueDifference {
                field: "SalesPurchasesImports.SalesOfSomethingNew".into(),
                return_field: None,
                approved: json!(0),
                authority: json!(40),
            }]
        );
        // VAT relief or payment-basis accounting, which aggountant never sends.
        let mut relief = october_answer();
        relief["VATOnCashBasis"] = json!(true);
        relief["DeductibleTax"]["FileVATRelief"] = json!(true);
        relief["DeductibleTax"]["TaxForVATRelief"] = json!(153.0);
        let fields: Vec<String> = differences(relief)
            .into_iter()
            .map(|difference| difference.field)
            .collect();
        assert_eq!(
            fields,
            [
                "DeductibleTax.FileVATRelief",
                "DeductibleTax.TaxForVATRelief",
                "VATOnCashBasis"
            ]
        );
        // An amount outside the groups the comparison reads, as with a renamed
        // group; text that is not an amount is not one.
        let mut renamed = october_answer();
        let answer = renamed.as_object_mut().unwrap();
        let sales = answer.remove("VATOnDomesticSales").unwrap();
        answer.insert("VATOnDomesticSalesByTaxRate".into(), sales);
        answer.insert("BusinessId".into(), json!("1234567-1"));
        let found = differences(renamed);
        assert_eq!(
            found.iter().map(|d| d.field.as_str()).collect::<Vec<_>>(),
            [
                "VATOnDomesticSalesByTaxRate.HighVATRate",
                "VATOnDomesticSalesByTaxRate"
            ]
        );
        // A known amount that is not a number, and a group that is not an object.
        let mut text = october_answer();
        text["DeductibleTax"]["DeductibleVAT"] = json!("n/a");
        let found = differences(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].authority, json!("n/a"));
        let mut rows = october_answer();
        rows["VATOnDomesticSales"] = json!([{"TaxRate": 25.5, "VAT": 255.0}]);
        let found = differences(rows);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].field, "VATOnDomesticSalesByTaxRate");
        assert_eq!(
            found[0].approved,
            october_request().vat_details.unwrap()["VATOnDomesticSalesByTaxRate"]
        );
        // Statuses other than Processed, and an earlier submission, prove nothing.
        let mut pending = october_answer();
        pending["Status"] = json!("Being Processed");
        let pending: FiledVatReturn = serde_json::from_value(pending).unwrap();
        assert_eq!(
            pending.processed_differences(&october_request(), None),
            None
        );
        let earlier: FiledVatReturn = serde_json::from_value(october_answer()).unwrap();
        assert_eq!(
            earlier.processed_differences(&october_request(), Some("2026-11-13")),
            None
        );
    }
}
