//! A narrow, stateful HTTP contract mock for Vero's VAT operations.
//!
//! The server is test infrastructure: it always binds an ephemeral IPv4 loopback
//! port, reads no credentials, and has no outbound-network code. Named fixtures
//! keep authority observations explicit and replaceable without growing a general
//! API-emulation framework.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig, ServerConnection, StreamOwned};
use serde_json::{json, Value};

/// Documented SelfAssessedTaxes operation paths used by the adapter.
pub const GET_VAT_PERIODS_PATH: &str = "/GetVATPeriods/v1";
pub const FILE_VAT_RETURN_PATH: &str = "/FileVATReturn/v2";
pub const GET_FILED_VAT_RETURN_PATH: &str = "/GetFiledVATReturn/v2";

/// Deterministic, test-only credentials trusted by [`MockVeroVatServer::start_mtls`].
///
/// These checked-in credentials are intentionally public and must never be used
/// outside tests. The server certificate is kept private to the mock because
/// clients need only its CA certificate and their own identity.
pub struct MtlsTestCredentials;

impl MtlsTestCredentials {
    pub const CA_CERTIFICATE_PEM: &'static [u8] = include_bytes!("test_pki/ca.pem.fixture");
    pub const CLIENT_CERTIFICATE_PEM: &'static [u8] = include_bytes!("test_pki/client.pem.fixture");
    pub const CLIENT_PRIVATE_KEY_PEM: &'static [u8] =
        include_bytes!("test_pki/client-key.pem.fixture");
}

/// A request captured before a fixture response is selected.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub json: Option<Value>,
}

/// FileVATReturn behavior selected for one named scenario.
#[derive(Clone, Debug)]
pub enum FileVatReturnFixture {
    /// HTTP 200 with both receipt fields. The submitted body becomes observable by
    /// [`GetFiledVatReturnFixture::Matching`].
    Receipt {
        unique_identifier: String,
        accepted_timestamp: String,
    },
    /// HTTP 400 using Vero's structured error fields.
    StructuredError {
        error_code: String,
        error_text: String,
        /// Kept as raw JSON because official observations may contain structured
        /// field-level details rather than a flat string list.
        error_details: Value,
    },
    /// Accept the request but send no response before this deterministic delay.
    Timeout { delay: Duration },
    /// An ambiguous authority failure.
    ServerError { status: u16, body: Value },
}

/// GetFiledVATReturn behavior, including the documented visibility-lag cases.
#[derive(Clone, Debug)]
pub enum GetFiledVatReturnFixture {
    /// Return the latest body accepted by FileVATReturn, with `Processed` status.
    Matching,
    /// Return a previous accepted return while a newer return may still be processing.
    Stale { body: Value },
    /// Return the documented `Missing` status without claiming non-delivery.
    Missing,
    /// Return a processed return whose semantic values differ from the submission.
    Mismatch { body: Value },
}

/// One fixture-selected service scenario.
///
/// `name` is deliberately persisted with the fixture so an observed official
/// sandbox/test response can later be checked in as a clearly named regression case.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub soap: Option<SoapScenario>,
    pub name: String,
    pub vat_periods_response: Value,
    pub file_vat_return: FileVatReturnFixture,
    pub get_filed_vat_return: GetFiledVatReturnFixture,
}

impl Scenario {
    /// A scenario with representative documented period and successful filing data.
    pub fn file_receipt(name: impl Into<String>) -> Self {
        Self {
            soap: None,
            name: name.into(),
            vat_periods_response: standard_vat_periods(),
            file_vat_return: FileVatReturnFixture::Receipt {
                unique_identifier: "mock-vat-return-0001".into(),
                accepted_timestamp: "2026-09-14T10:15:30Z".into(),
            },
            get_filed_vat_return: GetFiledVatReturnFixture::Matching,
        }
    }

    pub fn file_structured_error(name: impl Into<String>) -> Self {
        Self {
            soap: None,
            name: name.into(),
            vat_periods_response: standard_vat_periods(),
            file_vat_return: FileVatReturnFixture::StructuredError {
                error_code: "INVALID_DATA".into(),
                error_text: "VAT return contains invalid data".into(),
                error_details: json!(["VATDetails.VATOnDomesticSalesByTaxRate is required"]),
            },
            get_filed_vat_return: GetFiledVatReturnFixture::Missing,
        }
    }

    pub fn file_timeout(name: impl Into<String>, delay: Duration) -> Self {
        Self {
            soap: None,
            name: name.into(),
            vat_periods_response: standard_vat_periods(),
            file_vat_return: FileVatReturnFixture::Timeout { delay },
            get_filed_vat_return: GetFiledVatReturnFixture::Missing,
        }
    }

    pub fn file_server_error(name: impl Into<String>) -> Self {
        Self {
            soap: None,
            name: name.into(),
            vat_periods_response: standard_vat_periods(),
            file_vat_return: FileVatReturnFixture::ServerError {
                status: 500,
                body: json!({"ErrorCode":"INTERNAL_ERROR","ErrorText":"Temporary failure"}),
            },
            get_filed_vat_return: GetFiledVatReturnFixture::Missing,
        }
    }
}

fn standard_vat_periods() -> Value {
    json!({
        "FilingPeriod": [{
            "Period": "2026-07-31",
            "Frequency": "VATMON",
            "StartDate": "2026-07-01",
            "EndDate": "2026-07-31",
            "Status": "Missing",
            "CustomsComparisonDataOnImportsOfGoods": 0.0,
            "DueDate": "2026-09-14"
        }]
    })
}

#[derive(Default)]
struct State {
    requests: Vec<RecordedRequest>,
    latest_accepted_return: Option<Value>,
}

/// Running mock service. Dropping it shuts down its worker thread.
pub struct MockVeroVatServer {
    addr: SocketAddr,
    scenario_name: String,
    https: bool,
    state: Arc<Mutex<State>>,
    stop: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl MockVeroVatServer {
    /// Start plain HTTP on `127.0.0.1:0` for isolated mock tests.
    /// No caller-supplied address is accepted by design.
    pub fn start(scenario: Scenario) -> io::Result<Self> {
        Self::start_with_transport(scenario, None)
    }

    /// Start HTTPS with mandatory authentication by the checked-in test CA.
    ///
    /// Clients use [`MtlsTestCredentials`] for the trusted CA and client identity.
    /// No caller-supplied address or credential is accepted by design.
    pub fn start_wage_mtls(scenario: Scenario) -> io::Result<Self> {
        Self::start_with_transport(scenario, Some(wage_server_config()?))
    }
    pub fn start_mtls(scenario: Scenario) -> io::Result<Self> {
        Self::start_with_transport(scenario, Some(test_server_config()?))
    }

    fn start_with_transport(
        scenario: Scenario,
        tls: Option<Arc<ServerConfig>>,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        debug_assert!(addr.ip().is_loopback() && addr.port() != 0);

        let https = tls.is_some();
        let state = Arc::new(Mutex::new(State::default()));
        let worker_state = Arc::clone(&state);
        let scenario_name = scenario.name.clone();
        let (stop_tx, stop_rx) = mpsc::channel();
        let worker = thread::Builder::new()
            .name(format!("vero-vat-mock-{scenario_name}"))
            .spawn(move || serve(listener, Arc::new(scenario), worker_state, stop_rx, tls))?;

        Ok(Self {
            addr,
            scenario_name,
            https,
            state,
            stop: Some(stop_tx),
            worker: Some(worker),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn base_url(&self) -> String {
        let scheme = if self.https { "https" } else { "http" };
        format!("{scheme}://{}", self.addr)
    }

    pub fn scenario_name(&self) -> &str {
        &self.scenario_name
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.state
            .lock()
            .expect("mock state poisoned")
            .requests
            .clone()
    }
}

impl Drop for MockVeroVatServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        // Wake the nonblocking loop promptly. This can only connect to the address
        // this instance just bound on loopback.
        let _ = TcpStream::connect(self.addr);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn test_server_config() -> io::Result<Arc<ServerConfig>> {
    let mut roots = RootCertStore::empty();
    for certificate in pem_certificates(include_bytes!("test_pki/ca.pem.fixture"))? {
        roots
            .add(certificate)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    }
    let client_verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let config = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(
            pem_certificates(include_bytes!("test_pki/server.pem.fixture"))?,
            pem_private_key(include_bytes!("test_pki/server-key.pem.fixture"))?,
        )
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Arc::new(config))
}

fn pem_certificates(pem: &[u8]) -> io::Result<Vec<CertificateDer<'static>>> {
    CertificateDer::pem_slice_iter(pem)
        .collect::<Result<_, _>>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn pem_private_key(pem: &[u8]) -> io::Result<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_slice(pem)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn serve(
    listener: TcpListener,
    scenario: Arc<Scenario>,
    state: Arc<Mutex<State>>,
    stop: mpsc::Receiver<()>,
    tls: Option<Arc<ServerConfig>>,
) {
    loop {
        if stop.try_recv().is_ok() {
            return;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                // BSD and macOS hand out accepted sockets that inherit the
                // listener's non-blocking mode; Linux does not.
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let scenario = Arc::clone(&scenario);
                let state = Arc::clone(&state);
                let tls = tls.clone();
                // A deliberate FileVATReturn timeout must not prevent the adapter
                // from immediately making its safe reconciliation read.
                thread::spawn(move || {
                    if let Some(config) = tls {
                        if let Ok(connection) = ServerConnection::new(config) {
                            let mut stream = StreamOwned::new(connection, stream);
                            let _ = handle(&mut stream, &scenario, &state);
                            stream.conn.send_close_notify();
                            let _ = stream.conn.complete_io(&mut stream.sock);
                            // A TLS 1.3 client sends its request before learning
                            // that its certificate was refused. Closing with that
                            // data unread makes the kernel reset the connection,
                            // and the client sees a broken pipe instead of the
                            // alert. Half-close and drain until the client hangs up.
                            let _ = stream.sock.shutdown(std::net::Shutdown::Write);
                            let mut sink = [0u8; 4096];
                            while matches!(stream.sock.read(&mut sink), Ok(n) if n > 0) {}
                        }
                    } else {
                        let mut stream = stream;
                        let _ = handle(&mut stream, &scenario, &state);
                    }
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(1));
            }
            Err(_) => return,
        }
    }
}

fn handle<S: Read + Write>(
    stream: &mut S,
    scenario: &Scenario,
    state: &Arc<Mutex<State>>,
) -> io::Result<()> {
    let request = read_request(stream)?;
    let method = request.method.clone();
    let path = request.path.clone();
    let request_json = request.json.clone();
    state
        .lock()
        .expect("mock state poisoned")
        .requests
        .push(request);

    if method != "POST" {
        return write_json(stream, 405, &json!({"error":"method not allowed"}));
    }

    if let Some(soap) = &scenario.soap {
        let (body, status) = if path.ends_with("/WageReportService.svc") {
            thread::sleep(soap.send_delay);
            (&soap.send_body, soap.send_status)
        } else if path.ends_with("/StatusService.svc") {
            (&soap.status_body, 200)
        } else {
            return write_json(stream, 404, &json!({}));
        };
        write!(stream,"HTTP/1.1 {status} Fixture\r\nContent-Type: text/xml;charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len())?;
        return stream.write_all(body.as_bytes());
    }
    match path.as_str() {
        GET_VAT_PERIODS_PATH => write_json(stream, 200, &scenario.vat_periods_response),
        FILE_VAT_RETURN_PATH => match &scenario.file_vat_return {
            FileVatReturnFixture::Receipt {
                unique_identifier,
                accepted_timestamp,
            } => {
                if let Some(body) = request_json {
                    state
                        .lock()
                        .expect("mock state poisoned")
                        .latest_accepted_return = Some(body);
                }
                write_json(
                    stream,
                    200,
                    &json!({
                        "UniqueIdentifier": unique_identifier,
                        "AcceptedTimestamp": accepted_timestamp
                    }),
                )
            }
            FileVatReturnFixture::StructuredError {
                error_code,
                error_text,
                error_details,
            } => write_json(
                stream,
                400,
                &json!({
                    "ErrorCode": error_code,
                    "ErrorText": error_text,
                    "ErrorDetails": error_details
                }),
            ),
            FileVatReturnFixture::Timeout { delay } => {
                thread::sleep(*delay);
                Ok(())
            }
            FileVatReturnFixture::ServerError { status, body } => write_json(stream, *status, body),
        },
        GET_FILED_VAT_RETURN_PATH => {
            let body = match &scenario.get_filed_vat_return {
                GetFiledVatReturnFixture::Matching => state
                    .lock()
                    .expect("mock state poisoned")
                    .latest_accepted_return
                    .as_ref()
                    .map(filed_response_for)
                    .unwrap_or_else(|| json!({"Status":"Missing"})),
                GetFiledVatReturnFixture::Stale { body }
                | GetFiledVatReturnFixture::Mismatch { body } => body.clone(),
                GetFiledVatReturnFixture::Missing => json!({"Status":"Missing"}),
            };
            write_json(stream, 200, &body)
        }
        _ => write_json(stream, 404, &json!({"error":"unknown mock operation"})),
    }
}

fn filed_response_for(request: &Value) -> Value {
    let details = request.get("VATDetails").and_then(Value::as_object);
    let mut response = serde_json::Map::new();
    response.insert("Status".into(), json!("Processed"));
    response.insert("SubmittedDate".into(), json!("2026-09-14"));
    if let Some(details) = details {
        if let Some(value) = details.get("VATOnDomesticSalesByTaxRate") {
            response.insert("VATOnDomesticSales".into(), value.clone());
        }
        if let Some(value) = details.get("VATOnPurchasesAndImports") {
            response.insert("VATOnPurchasesAndImports".into(), value.clone());
        }
        if let Some(value) = details.get("DeductibleVAT") {
            response.insert("DeductibleTax".into(), json!({"DeductibleVAT":value}));
        }
        if let Some(value) = details.get("SalesPurchasesImports") {
            response.insert("SalesPurchasesImports".into(), value.clone());
        }
    }
    Value::Object(response)
}

fn read_request(stream: &mut impl Read) -> io::Result<RecordedRequest> {
    const MAX_REQUEST_BYTES: usize = 1024 * 1024;
    let mut bytes = Vec::new();
    let header_end = loop {
        if bytes.len() >= MAX_REQUEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request too large",
            ));
        }
        let mut chunk = [0_u8; 4096];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete headers",
            ));
        }
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(pos) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break pos + 4;
        }
    };

    let header_text = std::str::from_utf8(&bytes[..header_end - 4])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "headers are not UTF-8"))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let path = request_parts.next().unwrap_or_default().to_string();
    if request_parts.next().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid request line",
        ));
    }

    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid header"))?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    let content_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid content-length"))?
        .unwrap_or(0);
    if header_end + content_length > MAX_REQUEST_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request too large",
        ));
    }
    while bytes.len() < header_end + content_length {
        let mut chunk = [0_u8; 4096];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete body",
            ));
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    let body = bytes[header_end..header_end + content_length].to_vec();
    let json = serde_json::from_slice(&body).ok();

    Ok(RecordedRequest {
        method,
        path,
        headers,
        body,
        json,
    })
}

fn write_json(stream: &mut impl Write, status: u16, body: &Value) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Fixture Response",
    };
    let body = serde_json::to_vec(body).expect("JSON Value serialization cannot fail");
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)
}

/// Synthetic SOAP fixtures, never connected to an authority.
#[derive(Clone, Debug)]
pub struct SoapScenario {
    pub send_body: String,
    pub send_status: u16,
    pub send_delay: Duration,
    pub status_body: String,
}
pub struct WageTestCredentials;
impl WageTestCredentials {
    pub const CA: &'static str = include_str!("wage_pki/ca.pem.fixture");
    pub const CERT: &'static str = include_str!("wage_pki/client.pem.fixture");
    pub const KEY: &'static str = include_str!("wage_pki/client-key.pem.fixture");
}
fn wage_server_config() -> io::Result<Arc<ServerConfig>> {
    let mut roots = RootCertStore::empty();
    for c in pem_certificates(WageTestCredentials::CA.as_bytes())? {
        roots.add(c).map_err(io::Error::other)?;
    }
    let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(io::Error::other)?;
    let config = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            pem_certificates(include_bytes!("wage_pki/server.pem.fixture"))?,
            pem_private_key(include_bytes!("wage_pki/server-key.pem.fixture"))?,
        )
        .map_err(io::Error::other)?;
    Ok(Arc::new(config))
}
