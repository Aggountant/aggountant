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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
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
        if receipt.unique_identifier.trim().is_empty()
            || receipt.accepted_timestamp.trim().is_empty()
        {
            return Err(AdapterError::Malformed(
                "HTTP 200 omitted a FileVATReturn receipt field".into(),
            ));
        }
        Ok(receipt)
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

impl FiledVatReturn {
    pub fn semantically_matches(&self, expected: &FileVatReturn) -> bool {
        self.semantically_matches_attempt(expected, None)
    }

    pub fn semantically_matches_attempt(
        &self,
        expected: &FileVatReturn,
        earliest_submitted_date: Option<&str>,
    ) -> bool {
        // Being Processed may still expose the previous return's values, so it is
        // never sufficient evidence for recovery of the just-attempted filing.
        if matches!(self.status.as_deref(), Some(status) if status != "Processed") {
            return false;
        }
        if let Some(earliest) = earliest_submitted_date {
            let Some(submitted) = self.values.get("SubmittedDate").and_then(Value::as_str) else {
                return false;
            };
            if submitted < earliest {
                return false;
            }
        }
        let expected_details = expected.vat_details.as_ref();
        if expected.no_activity == Some(true) {
            return expected_details.is_none();
        }
        let Some(details) = expected_details.and_then(Value::as_object) else {
            return false;
        };
        let mut expected_semantic = Map::new();
        if let Some(value) = details.get("VATOnDomesticSalesByTaxRate") {
            expected_semantic.insert("VATOnDomesticSales".into(), value.clone());
        }
        if let Some(value) = details.get("VATOnPurchasesAndImports") {
            expected_semantic.insert("VATOnPurchasesAndImports".into(), value.clone());
        }
        if let Some(value) = details.get("DeductibleVAT") {
            expected_semantic.insert("DeductibleVAT".into(), value.clone());
        }
        if let Some(value) = details.get("SalesPurchasesImports") {
            expected_semantic.insert("SalesPurchasesImports".into(), value.clone());
        }

        let mut actual_semantic = Map::new();
        for key in [
            "VATOnDomesticSales",
            "VATOnPurchasesAndImports",
            "SalesPurchasesImports",
        ] {
            if let Some(value) = self.values.get(key).filter(|value| !value.is_null()) {
                actual_semantic.insert(key.into(), value.clone());
            }
        }
        if let Some(value) = self
            .values
            .get("DeductibleTax")
            .and_then(Value::as_object)
            .and_then(|value| value.get("DeductibleVAT"))
            .filter(|value| !value.is_null())
        {
            actual_semantic.insert("DeductibleVAT".into(), value.clone());
        }
        let mut expected = Value::Object(expected_semantic);
        let mut actual = Value::Object(actual_semantic);
        normalize_numbers(&mut expected);
        normalize_numbers(&mut actual);
        expected == actual
    }
}

fn normalize_numbers(value: &mut Value) {
    match value {
        Value::Number(number) => {
            if let Ok(decimal) = Decimal::from_str(&number.to_string()) {
                *value = Value::String(decimal.normalize().to_string());
            }
        }
        Value::Array(values) => values.iter_mut().for_each(normalize_numbers),
        Value::Object(values) => values.values_mut().for_each(normalize_numbers),
        _ => {}
    }
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

fn accepted_body<T: for<'de> Deserialize<'de> + Serialize>(
    bytes: &[u8],
) -> Result<Value, AdapterError> {
    let typed: T = serde_json::from_slice(bytes).map_err(|_| {
        AdapterError::Malformed("Vero response does not match the operation schema".into())
    })?;
    serde_json::to_value(typed)
        .map_err(|_| AdapterError::Malformed("Vero response serialization failed".into()))
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
            200 => {
                let receipt: FileVatReceipt = serde_json::from_value(response.body.clone())
                    .map_err(|_| {
                        AdapterError::Malformed(
                            "HTTP 200 omitted a complete FileVATReturn receipt".into(),
                        )
                    })?;
                if receipt.unique_identifier.is_empty() || receipt.accepted_timestamp.is_empty() {
                    return Err(AdapterError::Malformed(
                        "HTTP 200 omitted a complete FileVATReturn receipt".into(),
                    ));
                }
                Ok(AdapterResponse::Accepted {
                    receipt: receipt.encode()?,
                    body: response.body,
                })
            }
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
            200 => {
                let receipt: FileVatReceipt = serde_json::from_slice(&response).map_err(|_| {
                    AdapterError::Malformed(
                        "HTTP 200 omitted a complete FileVATReturn receipt".into(),
                    )
                })?;
                if receipt.unique_identifier.trim().is_empty()
                    || receipt.accepted_timestamp.trim().is_empty()
                {
                    return Err(AdapterError::Malformed(
                        "HTTP 200 omitted a complete FileVATReturn receipt".into(),
                    ));
                }
                Ok(AdapterResponse::Accepted {
                    receipt: receipt.encode()?,
                    body: accepted_body::<FileVatReceipt>(&response)?,
                })
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

    fn eur(value: &str) -> Money {
        Money::parse(value, Currency::new("EUR").unwrap()).unwrap()
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
}
