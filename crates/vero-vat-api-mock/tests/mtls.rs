use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use aggountant_vero_vat_api_mock::{
    MockVeroVatServer, MtlsTestCredentials, Scenario, FILE_VAT_RETURN_PATH,
    GET_FILED_VAT_RETURN_PATH, GET_VAT_PERIODS_PATH,
};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde_json::{json, Value};

fn certificates(pem: &[u8]) -> Vec<CertificateDer<'static>> {
    CertificateDer::pem_slice_iter(pem)
        .collect::<Result<_, _>>()
        .unwrap()
}

fn private_key(pem: &[u8]) -> PrivateKeyDer<'static> {
    PrivateKeyDer::from_pem_slice(pem).unwrap()
}

fn client_config(identity: Option<(&[u8], &[u8])>) -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    for certificate in certificates(MtlsTestCredentials::CA_CERTIFICATE_PEM) {
        roots.add(certificate).unwrap();
    }
    let builder = ClientConfig::builder().with_root_certificates(roots);
    let config = match identity {
        Some((certificate, key)) => builder
            .with_client_auth_cert(certificates(certificate), private_key(key))
            .unwrap(),
        None => builder.with_no_client_auth(),
    };
    Arc::new(config)
}

fn connect(
    server: &MockVeroVatServer,
    config: Arc<ClientConfig>,
) -> StreamOwned<ClientConnection, TcpStream> {
    let connection =
        ClientConnection::new(config, ServerName::try_from("localhost").unwrap()).unwrap();
    let stream = TcpStream::connect(server.addr()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    StreamOwned::new(connection, stream)
}

fn post(
    server: &MockVeroVatServer,
    config: Arc<ClientConfig>,
    path: &str,
    body: &Value,
) -> std::io::Result<(u16, Value)> {
    let mut stream = connect(server, config);
    let body = serde_json::to_vec(body).unwrap();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nX-Test-Marker: mtls-recorded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "no response"))?;
    let head = std::str::from_utf8(&response[..split]).unwrap();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = serde_json::from_slice(&response[split + 4..]).unwrap();
    Ok((status, body))
}

fn trusted_client() -> Arc<ClientConfig> {
    client_config(Some((
        MtlsTestCredentials::CLIENT_CERTIFICATE_PEM,
        MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM,
    )))
}

#[test]
fn trusted_client_serves_all_scenarios_and_records_exact_requests() {
    let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("mtls-success")).unwrap();
    assert!(server.addr().ip().is_loopback());
    assert_ne!(server.addr().port(), 0);
    assert_eq!(server.base_url(), format!("https://{}", server.addr()));

    let period_request = json!({"BusinessId":"1234567-8","FilingYear":2026});
    let (status, periods) = post(
        &server,
        trusted_client(),
        GET_VAT_PERIODS_PATH,
        &period_request,
    )
    .unwrap();
    assert_eq!(status, 200);
    assert_eq!(periods["FilingPeriod"][0]["Status"], "Missing");

    let filed = json!({
        "ReplacementReturn":false,
        "BusinessId":"1234567-8",
        "FilingPeriod":"2026-07-31",
        "NoActivity":false,
        "VATDetails":{"VATOnDomesticSalesByTaxRate":{"HighVATRate":255.0}},
        "ContactDetails":{"FullName":"Ada Operator","PhoneNumber":"04012345"}
    });
    assert_eq!(
        post(&server, trusted_client(), FILE_VAT_RETURN_PATH, &filed)
            .unwrap()
            .0,
        200
    );
    let (_, returned) = post(
        &server,
        trusted_client(),
        GET_FILED_VAT_RETURN_PATH,
        &json!({}),
    )
    .unwrap();
    assert_eq!(returned["Status"], "Processed");
    assert_eq!(
        returned["VATOnDomesticSales"],
        filed["VATDetails"]["VATOnDomesticSalesByTaxRate"]
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].json.as_ref(), Some(&period_request));
    assert_eq!(requests[0].headers["x-test-marker"], "mtls-recorded");
    assert_eq!(requests[1].body, serde_json::to_vec(&filed).unwrap());
    assert_eq!(requests[2].path, GET_FILED_VAT_RETURN_PATH);
}

#[test]
fn handshake_refuses_a_client_without_a_certificate() {
    let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("mtls-no-client")).unwrap();
    let error = post(
        &server,
        client_config(None),
        GET_VAT_PERIODS_PATH,
        &json!({}),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("CertificateRequired"),
        "unexpected TLS error: {error}"
    );
    assert!(server.requests().is_empty());
}

#[test]
fn handshake_refuses_a_client_signed_by_an_untrusted_ca() {
    let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("mtls-untrusted")).unwrap();
    let untrusted = client_config(Some((
        include_bytes!("../src/test_pki/untrusted-client.pem.fixture"),
        include_bytes!("../src/test_pki/untrusted-client-key.pem.fixture"),
    )));
    let error = post(&server, untrusted, GET_VAT_PERIODS_PATH, &json!({})).unwrap_err();
    assert!(
        error.to_string().contains("UnknownCA"),
        "unexpected TLS error: {error}"
    );
    assert!(server.requests().is_empty());
}
