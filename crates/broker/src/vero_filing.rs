//! Local, fail-closed readiness for the own-organisation Vero.fi filing path.
//!
//! This is deliberately filing-specific. It is not a company registry, a PKI
//! framework, or persistent secret storage.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use aggountant_model::{BusinessId, FilingContactName, FilingContactPhone};
use chrono::{DateTime, Duration, Utc};
use reqwest::{Certificate, Identity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x509_parser::pem::parse_x509_pem;
use x509_parser::prelude::{FromDer, X509Certificate};

pub const VERO_FILING_PROFILE: &str = "vero-filing.json";
pub const VERO_TEST_BASE_URL: &str = "https://apitest.vero.fi/";
pub const VERO_PRODUCTION_BASE_URL: &str = "https://api.vero.fi/";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VeroFilingMode {
    Disabled,
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VeroCredentialTarget {
    Test,
    Production,
}

impl VeroCredentialTarget {
    pub fn base_url(self) -> &'static str {
        match self {
            Self::Test => VERO_TEST_BASE_URL,
            Self::Production => VERO_PRODUCTION_BASE_URL,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VeroAttestations {
    #[serde(default)]
    pub software_registered: bool,
    #[serde(default)]
    pub api_agreement_active: bool,
    #[serde(default)]
    pub production_verification_completed: bool,
    #[serde(default)]
    pub production_permission_active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_certificate_fingerprint_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_verification: Option<TestVerificationEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portal_verification: Option<PortalVerificationEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestVerificationEvidence {
    pub observed_at: DateTime<Utc>,
    pub certificate_fingerprint_sha256: String,
}

/// Non-secret evidence that the fixed API Portal verification scenarios passed.
/// This is onboarding evidence, not a filing target or a tax filing record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortalVerificationEvidence {
    pub observed_at: DateTime<Utc>,
    pub certificate_fingerprint_sha256: String,
    pub operations: Vec<String>,
    pub file_vat_return_receipt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VeroFilingProfile {
    pub schema_version: u32,
    pub mode: VeroFilingMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_id: Option<BusinessId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_name: Option<FilingContactName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_phone: Option<FilingContactPhone>,
    #[serde(default)]
    pub attestations: VeroAttestations,
}

impl Default for VeroFilingProfile {
    fn default() -> Self {
        Self {
            schema_version: 1,
            mode: VeroFilingMode::Disabled,
            business_id: None,
            contact_name: None,
            contact_phone: None,
            attestations: VeroAttestations::default(),
        }
    }
}

impl VeroFilingProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("vero-filing.json schema_version must be 1".into());
        }
        if let Some(fingerprint) = &self.attestations.activation_certificate_fingerprint_sha256 {
            validate_fingerprint(fingerprint)?;
        }
        if let Some(evidence) = &self.attestations.test_verification {
            validate_fingerprint(&evidence.certificate_fingerprint_sha256)?;
        }
        if let Some(evidence) = &self.attestations.portal_verification {
            validate_fingerprint(&evidence.certificate_fingerprint_sha256)?;
            if evidence.operations != crate::vero_vat_api::PORTAL_VERIFICATION_OPERATIONS
                || crate::vero_vat_api::FileVatReceipt::decode(&evidence.file_vat_return_receipt)
                    .is_err()
            {
                return Err("portal verification evidence is incomplete".into());
            }
        }
        Ok(())
    }
}

pub fn profile_path(accounting_home: &Path) -> PathBuf {
    accounting_home.join(VERO_FILING_PROFILE)
}

pub fn read_vero_profile(accounting_home: &Path) -> Result<VeroFilingProfile, String> {
    let path = profile_path(accounting_home);
    let bytes = fs::read(&path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => format!("{} is missing", path.display()),
        _ => format!("{} could not be read", path.display()),
    })?;
    let profile: VeroFilingProfile = serde_json::from_slice(&bytes)
        .map_err(|_| format!("{} is not a valid Vero filing profile", path.display()))?;
    profile.validate()?;
    Ok(profile)
}

pub fn write_vero_profile(
    accounting_home: &Path,
    profile: &VeroFilingProfile,
) -> Result<(), String> {
    profile.validate()?;
    fs::create_dir_all(accounting_home)
        .map_err(|_| "accounting home could not be created".to_string())?;
    let path = profile_path(accounting_home);
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(profile)
        .map_err(|_| "Vero filing profile serialization failed".to_string())?;
    {
        let mut file = fs::File::create(&temporary)
            .map_err(|_| "temporary Vero filing profile could not be created".to_string())?;
        file.write_all(&bytes)
            .map_err(|_| "temporary Vero filing profile could not be written".to_string())?;
        file.write_all(b"\n")
            .map_err(|_| "temporary Vero filing profile could not be written".to_string())?;
        file.sync_all()
            .map_err(|_| "temporary Vero filing profile could not be synced".to_string())?;
    }
    fs::rename(&temporary, &path)
        .map_err(|_| "Vero filing profile could not be replaced".to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct VeroCertificateMetadata {
    pub fingerprint_sha256: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
}

/// Secret JSON held behind `CredentialHandle`. Serialization is deliberately
/// absent: the bundle must never enter status output or accounting-home state.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VeroCredentials {
    pub target: VeroCredentialTarget,
    pub business_id: BusinessId,
    pub software_key: String,
    pub client_certificate_pem: String,
    pub client_private_key_pem: String,
    pub ca_certificate_pem: String,
    #[serde(default)]
    pub authorization_token: Option<String>,
}

pub(crate) struct VeroTransportMaterial<'a> {
    pub(crate) software_key: &'a str,
    pub(crate) identity: Identity,
    pub(crate) trust_certificate: Certificate,
}

impl VeroCredentials {
    pub(crate) fn parse(secret: &str) -> Result<Self, String> {
        serde_json::from_str(secret).map_err(|_| "Vero credential bundle is malformed".into())
    }

    pub(crate) fn transport_material(&self) -> Result<VeroTransportMaterial<'_>, String> {
        if self.software_key.trim().is_empty() {
            return Err("Vero SoftwareKey is missing".into());
        }
        let identity_pem = format!(
            "{}\n{}",
            self.client_certificate_pem, self.client_private_key_pem
        );
        let identity = Identity::from_pem(identity_pem.as_bytes())
            .map_err(|_| "Vero client identity is invalid".to_string())?;
        let trust_certificate = Certificate::from_pem(self.ca_certificate_pem.as_bytes())
            .map_err(|_| "Vero trust certificate is invalid".to_string())?;
        Ok(VeroTransportMaterial {
            software_key: &self.software_key,
            identity,
            trust_certificate,
        })
    }

    pub(crate) fn validate_transport_material(&self) -> Result<VeroCertificateMetadata, String> {
        self.transport_material()?;
        certificate_metadata(self.client_certificate_pem.as_bytes())
    }
}

fn certificate_metadata(pem: &[u8]) -> Result<VeroCertificateMetadata, String> {
    let (_, pem) = parse_x509_pem(pem).map_err(|_| "Vero client certificate is invalid")?;
    let (_, certificate) = X509Certificate::from_der(&pem.contents)
        .map_err(|_| "Vero client certificate is invalid")?;
    let not_before = DateTime::from_timestamp(certificate.validity().not_before.timestamp(), 0)
        .ok_or("Vero client certificate has an invalid not-before time")?;
    let not_after = DateTime::from_timestamp(certificate.validity().not_after.timestamp(), 0)
        .ok_or("Vero client certificate has an invalid not-after time")?;
    Ok(VeroCertificateMetadata {
        fingerprint_sha256: hex::encode(Sha256::digest(&pem.contents)),
        not_before,
        not_after,
    })
}

fn validate_fingerprint(value: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("certificate fingerprint must be 64 lowercase hexadecimal characters".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VeroReadinessPurpose {
    ProductionFiling,
    TestVerification,
    PortalVerification,
}

#[derive(Debug, Clone, Serialize)]
pub struct VeroReadinessBlocker {
    pub code: &'static str,
    pub owner: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VeroReadinessWarning {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VeroReadinessReport {
    pub ready: bool,
    pub mode: Option<VeroFilingMode>,
    pub target: VeroCredentialTarget,
    pub target_base_url: &'static str,
    pub profile_path: String,
    pub credential_path: Option<String>,
    pub business_id: Option<String>,
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub certificate: Option<VeroCertificateMetadata>,
    pub blockers: Vec<VeroReadinessBlocker>,
    pub warnings: Vec<VeroReadinessWarning>,
}

impl VeroReadinessReport {
    pub fn require_ready(&self) -> Result<(), String> {
        if self.ready {
            Ok(())
        } else {
            Err(self
                .blockers
                .iter()
                .map(|blocker| blocker.code)
                .collect::<Vec<_>>()
                .join(","))
        }
    }
}

/// The sole local readiness evaluator used by status, configure, verification,
/// and production dispatch. It reports metadata only and never retains secrets.
pub fn evaluate_vero_readiness(
    accounting_home: &Path,
    profile: Result<VeroFilingProfile, String>,
    credential_path: Option<&Path>,
    purpose: VeroReadinessPurpose,
    now: DateTime<Utc>,
) -> VeroReadinessReport {
    let target = match purpose {
        VeroReadinessPurpose::ProductionFiling | VeroReadinessPurpose::PortalVerification => {
            VeroCredentialTarget::Production
        }
        VeroReadinessPurpose::TestVerification => VeroCredentialTarget::Test,
    };
    let mut report = VeroReadinessReport {
        ready: false,
        mode: None,
        target,
        target_base_url: target.base_url(),
        profile_path: profile_path(accounting_home).display().to_string(),
        credential_path: credential_path.map(|path| path.display().to_string()),
        business_id: None,
        contact_name: None,
        contact_phone: None,
        certificate: None,
        blockers: Vec::new(),
        warnings: Vec::new(),
    };

    let profile = match profile {
        Ok(profile) => profile,
        Err(message) => {
            blocker(&mut report, "VERO_PROFILE_INVALID", "agent", message);
            VeroFilingProfile::default()
        }
    };
    report.mode = Some(profile.mode);
    report.business_id = profile.business_id.as_ref().map(|v| v.as_str().into());
    report.contact_name = profile.contact_name.as_ref().map(|v| v.as_str().into());
    report.contact_phone = profile.contact_phone.as_ref().map(|v| v.as_str().into());

    if purpose == VeroReadinessPurpose::ProductionFiling
        && profile.mode != VeroFilingMode::Production
    {
        blocker(
            &mut report,
            "VERO_DISABLED",
            "human",
            "direct Vero.fi filing is disabled".into(),
        );
    }
    if profile.business_id.is_none() {
        blocker(
            &mut report,
            "VERO_BUSINESS_ID_MISSING",
            "human",
            "own-organisation BusinessId is missing".into(),
        );
    }
    if profile.contact_name.is_none() {
        blocker(
            &mut report,
            "VERO_CONTACT_NAME_MISSING",
            "human",
            "filing contact name is missing".into(),
        );
    }
    if profile.contact_phone.is_none() {
        blocker(
            &mut report,
            "VERO_CONTACT_PHONE_MISSING",
            "human",
            "filing contact phone is missing".into(),
        );
    }

    if purpose == VeroReadinessPurpose::ProductionFiling {
        for (present, code, message) in [
            (
                profile.attestations.software_registered,
                "VERO_SOFTWARE_REGISTRATION_UNATTESTED",
                "software registration is not attested",
            ),
            (
                profile.attestations.api_agreement_active,
                "VERO_API_AGREEMENT_UNATTESTED",
                "active Vero API agreement is not attested",
            ),
            (
                profile.attestations.production_verification_completed,
                "VERO_PRODUCTION_VERIFICATION_UNATTESTED",
                "production-certificate verification is not attested",
            ),
            (
                profile.attestations.production_permission_active,
                "VERO_PRODUCTION_PERMISSION_UNATTESTED",
                "production permission is not attested",
            ),
        ] {
            if !present {
                blocker(&mut report, code, "human", message.into());
            }
        }
        if profile.attestations.portal_verification.is_none() {
            blocker(
                &mut report,
                "VERO_PORTAL_VERIFICATION_MISSING",
                "agent",
                "API Portal integration-verification evidence is missing".into(),
            );
        }
    }

    let Some(path) = credential_path else {
        blocker(
            &mut report,
            "VERO_CREDENTIAL_PATH_MISSING",
            "agent",
            "platform-local Vero credential path is missing".into(),
        );
        return report;
    };
    if path.starts_with(accounting_home) {
        blocker(
            &mut report,
            "VERO_CREDENTIAL_IN_ACCOUNTING_HOME",
            "agent",
            "credential bundle must be outside the accounting home".into(),
        );
    }
    let secret = match fs::read_to_string(path) {
        Ok(secret) => secret,
        Err(_) => {
            blocker(
                &mut report,
                "VERO_CREDENTIAL_UNAVAILABLE",
                "agent",
                "credential bundle is not readable".into(),
            );
            return report;
        }
    };
    let credentials = match VeroCredentials::parse(&secret) {
        Ok(credentials) => credentials,
        Err(message) => {
            blocker(&mut report, "VERO_CREDENTIAL_INVALID", "agent", message);
            return report;
        }
    };
    if credentials.target != target {
        blocker(
            &mut report,
            "VERO_CREDENTIAL_TARGET_MISMATCH",
            "agent",
            "credential target does not match the requested operation".into(),
        );
    }
    if profile.business_id.as_ref() != Some(&credentials.business_id) {
        blocker(
            &mut report,
            "VERO_CREDENTIAL_BUSINESS_ID_MISMATCH",
            "human",
            "credential BusinessId does not match the filing profile".into(),
        );
    }
    if credentials
        .authorization_token
        .as_deref()
        .is_some_and(|token| !token.trim().is_empty())
    {
        blocker(
            &mut report,
            "VERO_DELEGATED_USE_UNSUPPORTED",
            "human",
            "authorization token is not supported for own-organisation v1".into(),
        );
    }
    let certificate = match credentials.validate_transport_material() {
        Ok(metadata) => metadata,
        Err(message) => {
            blocker(&mut report, "VERO_CREDENTIAL_INVALID", "agent", message);
            return report;
        }
    };
    if now < certificate.not_before {
        blocker(
            &mut report,
            "VERO_CERTIFICATE_NOT_YET_VALID",
            "human",
            format!("certificate is valid from {}", certificate.not_before),
        );
    }
    if now >= certificate.not_after {
        blocker(
            &mut report,
            "VERO_CERTIFICATE_EXPIRED",
            "human",
            format!("certificate expired at {}", certificate.not_after),
        );
    } else {
        let remaining = certificate.not_after - now;
        for (days, code) in [
            (7, "VERO_CERTIFICATE_EXPIRES_WITHIN_7_DAYS"),
            (30, "VERO_CERTIFICATE_EXPIRES_WITHIN_30_DAYS"),
            (90, "VERO_CERTIFICATE_EXPIRES_WITHIN_90_DAYS"),
        ] {
            if remaining <= Duration::days(days) {
                report.warnings.push(VeroReadinessWarning {
                    code,
                    message: format!("certificate expires at {}", certificate.not_after),
                });
                break;
            }
        }
    }
    if purpose == VeroReadinessPurpose::ProductionFiling {
        if profile
            .attestations
            .activation_certificate_fingerprint_sha256
            .as_deref()
            != Some(certificate.fingerprint_sha256.as_str())
        {
            blocker(
                &mut report,
                "VERO_ACTIVATION_CERTIFICATE_MISMATCH",
                "human",
                "production activation is not bound to the selected certificate".into(),
            );
        }
        if profile
            .attestations
            .portal_verification
            .as_ref()
            .map(|evidence| evidence.certificate_fingerprint_sha256.as_str())
            != Some(certificate.fingerprint_sha256.as_str())
        {
            blocker(
                &mut report,
                "VERO_PORTAL_VERIFICATION_CERTIFICATE_MISMATCH",
                "agent",
                "API Portal verification is not bound to the selected certificate".into(),
            );
        }
    }
    report.certificate = Some(certificate);
    report.ready = report.blockers.is_empty();
    report
}

fn blocker(
    report: &mut VeroReadinessReport,
    code: &'static str,
    owner: &'static str,
    message: String,
) {
    report.blockers.push(VeroReadinessBlocker {
        code,
        owner,
        message,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_rejects_unknown_modes_and_fields() {
        assert!(serde_json::from_str::<VeroFilingProfile>(
            r#"{"schema_version":1,"mode":"test","attestations":{}}"#
        )
        .is_err());
        assert!(serde_json::from_str::<VeroFilingProfile>(
            r#"{"schema_version":1,"mode":"disabled","secret":"x","attestations":{}}"#
        )
        .is_err());
    }

    #[test]
    fn default_profile_contains_no_secret_or_machine_path_field() {
        let value = serde_json::to_value(VeroFilingProfile::default()).unwrap();
        assert!(value.get("credential_path").is_none());
        assert!(value.get("software_key").is_none());
        assert_eq!(value["mode"], "disabled");
    }

    #[test]
    fn portal_evidence_is_non_secret_and_does_not_change_filing_mode() {
        let mut profile = VeroFilingProfile::default();
        profile.attestations.portal_verification = Some(PortalVerificationEvidence {
            observed_at: Utc::now(),
            certificate_fingerprint_sha256: "a".repeat(64),
            operations: vec![
                "GetVATPeriods/v1".into(),
                "FileVATReturn/v2".into(),
                "GetFiledVATReturn/v2".into(),
            ],
            file_vat_return_receipt:
                r#"{"UniqueIdentifier":"verification","AcceptedTimestamp":"2023-03-01T10:15:30Z"}"#
                    .into(),
        });
        profile.validate().unwrap();
        let stored = serde_json::to_string(&profile).unwrap();
        assert_eq!(profile.mode, VeroFilingMode::Disabled);
        for forbidden in [
            "software_key",
            "private_key",
            "certificate_pem",
            "credential_path",
        ] {
            assert!(!stored.contains(forbidden));
        }
    }
}
