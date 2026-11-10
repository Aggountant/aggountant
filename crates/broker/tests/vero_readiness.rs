use std::path::Path;

use aggountant_broker::{
    evaluate_vero_readiness, VeroAttestations, VeroCredentialTarget, VeroFilingMode,
    VeroFilingProfile, VeroReadinessPurpose,
};
use aggountant_model::{BusinessId, FilingContactName, FilingContactPhone};
use aggountant_vero_vat_api_mock::MtlsTestCredentials;
use chrono::{TimeZone, Utc};
use serde_json::json;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 13, 12, 0, 0).unwrap()
}

fn profile(mode: VeroFilingMode) -> VeroFilingProfile {
    VeroFilingProfile {
        schema_version: 1,
        mode,
        business_id: Some(BusinessId::parse("1572860-0").unwrap()),
        contact_name: Some(FilingContactName::parse("Ada Operator").unwrap()),
        contact_phone: Some(FilingContactPhone::parse("+358401234567").unwrap()),
        attestations: VeroAttestations::default(),
    }
}

fn write_credentials(path: &Path, target: &str, business_id: &str, token: Option<&str>) {
    std::fs::write(
        path,
        json!({
            "target": target,
            "business_id": business_id,
            "software_key": "not-returned-by-status",
            "client_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_CERTIFICATE_PEM),
            "client_private_key_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM),
            "ca_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CA_CERTIFICATE_PEM),
            "authorization_token": token,
        })
        .to_string(),
    )
    .unwrap();
}

fn codes(report: &aggountant_broker::VeroReadinessReport) -> Vec<&str> {
    report.blockers.iter().map(|blocker| blocker.code).collect()
}

#[test]
fn disabled_and_incomplete_profiles_report_all_known_blockers() {
    let home = tempfile::tempdir().unwrap();
    let report = evaluate_vero_readiness(
        home.path(),
        Ok(VeroFilingProfile::default()),
        None,
        VeroReadinessPurpose::ProductionFiling,
        now(),
    );
    let codes = codes(&report);
    assert!(codes.contains(&"VERO_DISABLED"));
    assert!(codes.contains(&"VERO_BUSINESS_ID_MISSING"));
    assert!(codes.contains(&"VERO_CONTACT_NAME_MISSING"));
    assert!(codes.contains(&"VERO_CONTACT_PHONE_MISSING"));
    assert!(codes.contains(&"VERO_CREDENTIAL_PATH_MISSING"));
    assert!(!report.ready);
}

#[test]
fn target_business_id_and_delegated_use_mismatches_fail_closed_without_secret_echo() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let credential = outside.path().join("credential.json");
    write_credentials(&credential, "test", "1234567-1", Some("never-echo-this"));
    let report = evaluate_vero_readiness(
        home.path(),
        Ok(profile(VeroFilingMode::Production)),
        Some(&credential),
        VeroReadinessPurpose::ProductionFiling,
        now(),
    );
    let codes = codes(&report);
    assert!(codes.contains(&"VERO_CREDENTIAL_TARGET_MISMATCH"));
    assert!(codes.contains(&"VERO_CREDENTIAL_BUSINESS_ID_MISMATCH"));
    assert!(codes.contains(&"VERO_DELEGATED_USE_UNSUPPORTED"));
    let output = serde_json::to_string(&report).unwrap();
    assert!(!output.contains("never-echo-this"));
    assert!(!output.contains("PRIVATE KEY"));
}

#[test]
fn readiness_reports_transport_material_validation_error_without_secret_echo() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let credential = outside.path().join("credential.json");
    write_credentials(&credential, "test", "1572860-0", None);
    let mut bundle: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&credential).unwrap()).unwrap();
    bundle["software_key"] = json!("  ");
    std::fs::write(&credential, serde_json::to_vec(&bundle).unwrap()).unwrap();

    let report = evaluate_vero_readiness(
        home.path(),
        Ok(profile(VeroFilingMode::Production)),
        Some(&credential),
        VeroReadinessPurpose::TestVerification,
        now(),
    );
    let blocker = report
        .blockers
        .iter()
        .find(|blocker| blocker.code == "VERO_CREDENTIAL_INVALID")
        .unwrap();
    assert_eq!(blocker.message, "Vero SoftwareKey is missing");
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("PRIVATE KEY"));
}

#[test]
fn portal_verification_uses_production_credentials_but_not_production_mode() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let credential = outside.path().join("credential.json");
    write_credentials(&credential, "production", "1572860-0", None);

    let report = evaluate_vero_readiness(
        home.path(),
        Ok(profile(VeroFilingMode::Disabled)),
        Some(&credential),
        VeroReadinessPurpose::PortalVerification,
        now(),
    );
    assert!(report.ready, "{:?}", codes(&report));
    assert_eq!(report.mode, Some(VeroFilingMode::Disabled));
    assert_eq!(report.target, VeroCredentialTarget::Production);
}

#[test]
fn production_requires_attestations_bound_to_the_current_certificate() {
    let home = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let credential = outside.path().join("credential.json");
    write_credentials(&credential, "production", "1572860-0", None);

    let initial = evaluate_vero_readiness(
        home.path(),
        Ok(profile(VeroFilingMode::Production)),
        Some(&credential),
        VeroReadinessPurpose::ProductionFiling,
        now(),
    );
    assert!(!initial.ready);
    assert!(codes(&initial).contains(&"VERO_PORTAL_VERIFICATION_MISSING"));
    assert!(codes(&initial).contains(&"VERO_ACTIVATION_CERTIFICATE_MISMATCH"));
    let fingerprint = initial.certificate.unwrap().fingerprint_sha256;

    let mut ready_profile = profile(VeroFilingMode::Production);
    ready_profile.attestations.software_registered = true;
    ready_profile.attestations.api_agreement_active = true;
    ready_profile.attestations.production_verification_completed = true;
    ready_profile.attestations.production_permission_active = true;
    ready_profile
        .attestations
        .activation_certificate_fingerprint_sha256 = Some(fingerprint.clone());
    ready_profile.attestations.portal_verification =
        Some(aggountant_broker::vero_filing::PortalVerificationEvidence {
            observed_at: now(),
            certificate_fingerprint_sha256: fingerprint,
            operations: vec![
                "GetVATPeriods/v1".into(),
                "FileVATReturn/v2".into(),
                "GetFiledVATReturn/v2".into(),
            ],
            file_vat_return_receipt:
                r#"{"UniqueIdentifier":"verification","AcceptedTimestamp":"2026-09-13T12:00:00Z"}"#
                    .into(),
        });
    let mut mismatched_profile = ready_profile.clone();
    mismatched_profile
        .attestations
        .activation_certificate_fingerprint_sha256 = Some("a".repeat(64));
    mismatched_profile
        .attestations
        .portal_verification
        .as_mut()
        .unwrap()
        .certificate_fingerprint_sha256 = "a".repeat(64);
    let mismatched = evaluate_vero_readiness(
        home.path(),
        Ok(mismatched_profile),
        Some(&credential),
        VeroReadinessPurpose::ProductionFiling,
        now(),
    );
    assert!(!mismatched.ready);
    assert!(codes(&mismatched).contains(&"VERO_ACTIVATION_CERTIFICATE_MISMATCH"));
    assert!(codes(&mismatched).contains(&"VERO_PORTAL_VERIFICATION_CERTIFICATE_MISMATCH"));

    assert!(ready_profile.attestations.test_verification.is_none());
    let report = evaluate_vero_readiness(
        home.path(),
        Ok(ready_profile),
        Some(&credential),
        VeroReadinessPurpose::ProductionFiling,
        now(),
    );
    assert!(report.ready, "{:?}", codes(&report));
    assert_eq!(report.target, VeroCredentialTarget::Production);
}
