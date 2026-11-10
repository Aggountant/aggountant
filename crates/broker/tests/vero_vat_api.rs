use std::time::Duration;

use aggountant_broker::{
    project_file_vat_return, Broker, Envelope, FileVatReceipt, FiledVatReturn, Registry,
    RemoteOutcome, Vault, VeroFileVatReturnAdapter, VeroFiledVatReturnAdapter,
    VeroVatPeriodsAdapter, SVC_VERO_FILED_VAT_RETURN, SVC_VERO_FILE_VAT_RETURN,
    SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::{VatReturnField, VatReturnFields};
use aggountant_model::{
    BusinessId, Currency, FilingContactName, FilingContactPhone, Money, PeriodId,
};
use aggountant_vero_vat_api_mock::{
    GetFiledVatReturnFixture, MockVeroVatServer, MtlsTestCredentials, Scenario,
};
use serde_json::json;

fn credential_json(certificate: &[u8], key: &[u8], token: Option<&str>) -> String {
    json!({
        "target":"test",
        "business_id":"1572860-0",
        "software_key":"test-software-key",
        "client_certificate_pem":String::from_utf8_lossy(certificate),
        "client_private_key_pem":String::from_utf8_lossy(key),
        "ca_certificate_pem":String::from_utf8_lossy(MtlsTestCredentials::CA_CERTIFICATE_PEM),
        "authorization_token":token,
    })
    .to_string()
}

fn broker(server: &MockVeroVatServer, credentials: String, timeout: Duration) -> Broker {
    let url = format!("https://localhost:{}", server.addr().port());
    let mut registry = Registry::new();
    registry.register(Box::new(VeroVatPeriodsAdapter::new(&url, timeout).unwrap()));
    registry.register(Box::new(
        VeroFileVatReturnAdapter::new(&url, timeout).unwrap(),
    ));
    registry.register(Box::new(
        VeroFiledVatReturnAdapter::new(&url, timeout).unwrap(),
    ));
    let mut vault = Vault::new();
    for service in [
        SVC_VERO_VAT_PERIODS,
        SVC_VERO_FILE_VAT_RETURN,
        SVC_VERO_FILED_VAT_RETURN,
    ] {
        vault.set(service, credentials.clone());
    }
    Broker::new(
        tempfile::tempdir().unwrap().keep().as_path(),
        vault,
        registry,
    )
}

fn trusted_broker(server: &MockVeroVatServer, timeout: Duration) -> Broker {
    broker(
        server,
        credential_json(
            MtlsTestCredentials::CLIENT_CERTIFICATE_PEM,
            MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM,
            None,
        ),
        timeout,
    )
}

fn request() -> aggountant_broker::FileVatReturn {
    let mut fields = VatReturnFields::new();
    let eur = Currency::new("EUR").unwrap();
    fields
        .try_insert(
            VatReturnField::Field301,
            Money::parse("25.50", eur.clone()).unwrap(),
        )
        .unwrap();
    fields
        .try_insert(VatReturnField::Field307, Money::parse("5.00", eur).unwrap())
        .unwrap();
    project_file_vat_return(
        &PeriodId::parse("2026-07").unwrap(),
        &fields,
        &BusinessId::parse("1572860-0").unwrap(),
        &FilingContactName::parse("Ada Operator").unwrap(),
        &FilingContactPhone::parse("+358401234567").unwrap(),
    )
    .unwrap()
}

fn env(service: &str, operation: &str, bytes: &[u8]) -> Envelope {
    Envelope::new(
        service,
        operation,
        "test-effect",
        json!({"request_bytes":String::from_utf8(bytes.to_vec()).unwrap()}),
    )
}

#[test]
fn trusted_mtls_client_sends_exact_projected_bytes_and_required_headers() {
    let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("adapter-success")).unwrap();
    let broker = trusted_broker(&server, Duration::from_secs(2));
    let request = request();
    let bytes = request.bytes().unwrap();

    let periods = broker
        .fetch(&env(
            SVC_VERO_VAT_PERIODS,
            "GetVATPeriods/v1",
            br#"{"BusinessId":"1572860-0","FilingYear":2026}"#,
        ))
        .unwrap();
    assert_eq!(periods["FilingPeriod"][0]["DueDate"], "2026-09-14");

    let outcome = broker
        .submit(
            "test",
            "file:2026-07",
            &env(SVC_VERO_FILE_VAT_RETURN, "FileVATReturn/v2", &bytes),
        )
        .unwrap();
    let receipt = match outcome {
        RemoteOutcome::Delivered { receipt } => FileVatReceipt::decode(&receipt).unwrap(),
        other => panic!("unexpected outcome {other:?}"),
    };
    assert_eq!(receipt.unique_identifier, "mock-vat-return-0001");

    let filed_value = broker
        .fetch(&env(
            SVC_VERO_FILED_VAT_RETURN,
            "GetFiledVATReturn/v2",
            br#"{"BusinessId":"1572860-0","FilingPeriod":"2026-07-31"}"#,
        ))
        .unwrap();
    let filed: FiledVatReturn = serde_json::from_value(filed_value).unwrap();
    assert!(filed.semantically_matches(&request));

    let requests = server.requests();
    assert_eq!(requests[1].body, bytes);
    assert_eq!(requests[1].headers["vero-softwarekey"], "test-software-key");
    assert_eq!(requests[1].headers["accept"], "application/json");
    assert_eq!(requests[1].headers["user-agent"], "aggountant/0.1");
    assert!(!requests[1].headers.contains_key("vero-authorizationtoken"));
}

#[test]
fn optional_behalf_of_token_is_sent_only_when_configured() {
    let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("behalf-of")).unwrap();
    let broker = broker(
        &server,
        credential_json(
            MtlsTestCredentials::CLIENT_CERTIFICATE_PEM,
            MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM,
            Some("documented-token"),
        ),
        Duration::from_secs(2),
    );
    broker
        .fetch(&env(SVC_VERO_VAT_PERIODS, "GetVATPeriods/v1", b"{}"))
        .unwrap();
    assert_eq!(
        server.requests()[0].headers["vero-authorizationtoken"],
        "documented-token"
    );
}

#[test]
fn dispatch_reuses_transport_material_validation_before_network_io() {
    let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("invalid-material")).unwrap();
    let mut credentials: serde_json::Value = serde_json::from_str(&credential_json(
        MtlsTestCredentials::CLIENT_CERTIFICATE_PEM,
        MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM,
        None,
    ))
    .unwrap();
    credentials["software_key"] = json!("  ");
    let broker = broker(&server, credentials.to_string(), Duration::from_secs(1));

    let error = broker
        .fetch(&env(SVC_VERO_VAT_PERIODS, "GetVATPeriods/v1", b"{}"))
        .unwrap_err();
    assert!(error.to_string().contains("Vero SoftwareKey is missing"));
    assert!(server.requests().is_empty());
}

#[test]
fn absent_and_untrusted_client_identity_never_reaches_an_operation() {
    let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("client-auth")).unwrap();
    let absent = broker(
        &server,
        credential_json(b"", b"", None),
        Duration::from_secs(1),
    );
    assert!(absent
        .fetch(&env(SVC_VERO_VAT_PERIODS, "GetVATPeriods/v1", b"{}"))
        .is_err());

    let untrusted = broker(
        &server,
        credential_json(
            include_bytes!("../../vero-vat-api-mock/src/test_pki/untrusted-client.pem.fixture"),
            include_bytes!("../../vero-vat-api-mock/src/test_pki/untrusted-client-key.pem.fixture"),
            None,
        ),
        Duration::from_secs(1),
    );
    assert!(untrusted
        .fetch(&env(SVC_VERO_VAT_PERIODS, "GetVATPeriods/v1", b"{}"))
        .is_err());
    assert!(server.requests().is_empty());
}

#[test]
fn structured_400_is_rejected_while_timeout_and_5xx_are_unknown() {
    for (scenario, expected_code) in [
        (
            Scenario::file_structured_error("bad-request"),
            Some("INVALID_DATA"),
        ),
        (
            Scenario::file_timeout("timeout", Duration::from_millis(250)),
            None,
        ),
        (Scenario::file_server_error("server-error"), None),
    ] {
        let server = MockVeroVatServer::start_mtls(scenario).unwrap();
        let broker = trusted_broker(&server, Duration::from_millis(50));
        let result = broker
            .submit(
                "test",
                "file:2026-07",
                &env(
                    SVC_VERO_FILE_VAT_RETURN,
                    "FileVATReturn/v2",
                    &request().bytes().unwrap(),
                ),
            )
            .unwrap();
        match expected_code {
            Some(code) => match result {
                RemoteOutcome::Rejected { code: got, message } => {
                    assert_eq!(got, code);
                    assert!(message.contains("ErrorDetails="), "{message}");
                }
                other => panic!("expected structured rejection, got {other:?}"),
            },
            None => assert_eq!(result, RemoteOutcome::Unknown),
        }
    }
}

#[test]
fn reconciliation_distinguishes_matching_stale_missing_and_mismatch() {
    let expected = request();
    let expected_value = serde_json::to_value(&expected).unwrap();
    let details = &expected_value["VATDetails"];
    let matching = json!({
        "Status":"Processed", "SubmittedDate":"2026-09-14",
        "VATOnDomesticSales":details["VATOnDomesticSalesByTaxRate"],
        "DeductibleTax":{"DeductibleVAT":details["DeductibleVAT"]}
    });
    let mut mismatch = matching.clone();
    mismatch["DeductibleTax"]["DeductibleVAT"] = json!(999);

    for (fixture, should_match) in [
        (
            GetFiledVatReturnFixture::Stale {
                body: json!({"Status":"Processed","BusinessId":"1572860-0"}),
            },
            false,
        ),
        (GetFiledVatReturnFixture::Missing, false),
        (GetFiledVatReturnFixture::Mismatch { body: mismatch }, false),
        (GetFiledVatReturnFixture::Mismatch { body: matching }, true),
    ] {
        let mut scenario = Scenario::file_receipt("reconcile");
        scenario.get_filed_vat_return = fixture;
        let server = MockVeroVatServer::start_mtls(scenario).unwrap();
        let broker = trusted_broker(&server, Duration::from_secs(2));
        let value = broker
            .fetch(&env(
                SVC_VERO_FILED_VAT_RETURN,
                "GetFiledVATReturn/v2",
                br#"{"BusinessId":"1572860-0","FilingPeriod":"2026-07-31"}"#,
            ))
            .unwrap();
        let filed: FiledVatReturn = serde_json::from_value(value).unwrap();
        assert_eq!(filed.semantically_matches(&expected), should_match);
    }
}
