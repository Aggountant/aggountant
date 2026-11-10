use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use aggountant_broker::vero_vat_api::{
    portal_file_vat_return_request, portal_get_filed_vat_return_request,
    portal_get_vat_periods_request, PORTAL_FILE_VAT_RETURN_PATH, PORTAL_GET_FILED_VAT_RETURN_PATH,
    PORTAL_GET_VAT_PERIODS_PATH,
};
use aggountant_broker::{
    Broker, Envelope, FileVatReceipt, Registry, RemoteOutcome, Vault, VeroFileVatReturnAdapter,
    VeroFiledVatReturnAdapter, VeroVatPeriodsAdapter, SVC_VERO_FILED_VAT_RETURN,
    SVC_VERO_FILE_VAT_RETURN, SVC_VERO_VAT_PERIODS,
};
use aggountant_vero_vat_api_mock::MtlsTestCredentials;
use serde_json::json;

fn credential_json() -> String {
    credential_json_for("production")
}

fn credential_json_for(target: &str) -> String {
    json!({
        "target":target,
        "business_id":"1572860-0",
        "software_key":"portal-secret-software-key",
        "client_certificate_pem":String::from_utf8_lossy(MtlsTestCredentials::CLIENT_CERTIFICATE_PEM),
        "client_private_key_pem":String::from_utf8_lossy(MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM),
        "ca_certificate_pem":String::from_utf8_lossy(MtlsTestCredentials::CA_CERTIFICATE_PEM),
    })
    .to_string()
}

fn envelope(service: &str, operation: &str, body: Vec<u8>) -> Envelope {
    Envelope::new(
        service,
        operation,
        format!("portal:{operation}"),
        json!({"request_bytes":String::from_utf8(body).unwrap()}),
    )
}

fn read_http_request(mut stream: &std::net::TcpStream) -> (String, Vec<u8>) {
    let mut bytes = Vec::new();
    let header_end = loop {
        let mut chunk = [0_u8; 4096];
        let read = stream.read(&mut chunk).unwrap();
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(pos) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
    let path = headers
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string();
    let length = headers
        .lines()
        .find_map(|line| {
            line.split_once(':').and_then(|(name, value)| {
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
        })
        .unwrap_or(0);
    while bytes.len() < header_end + length {
        let mut chunk = [0_u8; 4096];
        let read = stream.read(&mut chunk).unwrap();
        bytes.extend_from_slice(&chunk[..read]);
    }
    (path, bytes[header_end..header_end + length].to_vec())
}

#[test]
fn all_three_portal_requests_use_only_verification_endpoints_and_static_bodies() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let observed = Arc::new(Mutex::new(Vec::new()));
    let server_observed = Arc::clone(&observed);
    let server = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().unwrap();
            let (path, body) = read_http_request(&stream);
            let response = match path.as_str() {
                PORTAL_GET_VAT_PERIODS_PATH => json!({"FilingPeriod":[{
                    "Period":"2023-01-31","Frequency":"VATMON",
                    "StartDate":"2023-01-01","EndDate":"2023-01-31",
                    "Status":"Missing","DueDate":"2023-03-13"
                }]}),
                PORTAL_FILE_VAT_RETURN_PATH => json!({
                    "UniqueIdentifier":"portal-verification-receipt",
                    "AcceptedTimestamp":"2023-03-01T10:15:30Z"
                }),
                PORTAL_GET_FILED_VAT_RETURN_PATH => json!({"Status":"Processed"}),
                other => panic!("unexpected path {other}"),
            };
            server_observed.lock().unwrap().push((path, body));
            let response = serde_json::to_vec(&response).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response.len()
            )
            .unwrap();
            stream.write_all(&response).unwrap();
        }
    });

    let timeout = Duration::from_secs(2);
    let mut registry = Registry::new();
    registry.register(Box::new(
        VeroVatPeriodsAdapter::new_portal_verification_loopback(&base_url, timeout).unwrap(),
    ));
    registry.register(Box::new(
        VeroFileVatReturnAdapter::new_portal_verification_loopback(&base_url, timeout).unwrap(),
    ));
    registry.register(Box::new(
        VeroFiledVatReturnAdapter::new_portal_verification_loopback(&base_url, timeout).unwrap(),
    ));
    let mut vault = Vault::new();
    for service in [
        SVC_VERO_VAT_PERIODS,
        SVC_VERO_FILE_VAT_RETURN,
        SVC_VERO_FILED_VAT_RETURN,
    ] {
        vault.set(service, credential_json());
    }
    let home = tempfile::tempdir().unwrap();
    let broker = Broker::new(home.path(), vault, registry);

    broker
        .fetch(&envelope(
            SVC_VERO_VAT_PERIODS,
            "GetVATPeriods/v1",
            portal_get_vat_periods_request(),
        ))
        .unwrap();
    let filing = envelope(
        SVC_VERO_FILE_VAT_RETURN,
        "FileVATReturn/v2",
        portal_file_vat_return_request(),
    );
    let outcome = broker.submit("portal", "portal:file", &filing).unwrap();
    let receipt = match outcome {
        RemoteOutcome::Delivered { receipt } => FileVatReceipt::decode(&receipt).unwrap(),
        other => panic!("unexpected filing verification outcome: {other:?}"),
    };
    assert_eq!(receipt.unique_identifier, "portal-verification-receipt");
    broker
        .fetch(&envelope(
            SVC_VERO_FILED_VAT_RETURN,
            "GetFiledVATReturn/v2",
            portal_get_filed_vat_return_request(),
        ))
        .unwrap();
    server.join().unwrap();

    let requests = observed.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        [
            PORTAL_GET_VAT_PERIODS_PATH,
            PORTAL_FILE_VAT_RETURN_PATH,
            PORTAL_GET_FILED_VAT_RETURN_PATH,
        ]
    );
    assert_eq!(requests[0].1, portal_get_vat_periods_request());
    assert_eq!(requests[1].1, portal_file_vat_return_request());
    assert_eq!(requests[2].1, portal_get_filed_vat_return_request());
    let recorded = std::fs::read(home.path().join("integration/outbox/outbox.db")).unwrap();
    assert!(!recorded
        .windows(b"portal-secret-software-key".len())
        .any(|part| part == b"portal-secret-software-key"));
    assert!(!recorded
        .windows(b"PRIVATE KEY".len())
        .any(|part| part == b"PRIVATE KEY"));
}

#[test]
fn wrong_target_is_rejected_before_portal_network_io() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let mut registry = Registry::new();
    registry.register(Box::new(
        VeroVatPeriodsAdapter::new_portal_verification_loopback(&base_url, Duration::from_secs(1))
            .unwrap(),
    ));
    let mut vault = Vault::new();
    vault.set(SVC_VERO_VAT_PERIODS, credential_json_for("test"));
    let home = tempfile::tempdir().unwrap();
    let broker = Broker::new(home.path(), vault, registry);
    let error = broker
        .fetch(&envelope(
            SVC_VERO_VAT_PERIODS,
            "GetVATPeriods/v1",
            portal_get_vat_periods_request(),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("target does not match"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn verification_endpoints_are_isolated_and_test_override_is_loopback_only() {
    assert_ne!(PORTAL_GET_VAT_PERIODS_PATH, "/GetVATPeriods/v1");
    assert_ne!(PORTAL_FILE_VAT_RETURN_PATH, "/FileVATReturn/v2");
    assert_ne!(PORTAL_GET_FILED_VAT_RETURN_PATH, "/GetFiledVATReturn/v2");
    assert!(VeroVatPeriodsAdapter::new_portal_verification_loopback(
        "http://example.com",
        Duration::from_secs(1)
    )
    .is_err());
    assert!(VeroFileVatReturnAdapter::new_portal_verification_loopback(
        "https://127.0.0.1:1234",
        Duration::from_secs(1)
    )
    .is_err());
}
