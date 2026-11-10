use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use aggountant_vero_vat_api_mock::{
    GetFiledVatReturnFixture, MockVeroVatServer, Scenario, FILE_VAT_RETURN_PATH,
    GET_FILED_VAT_RETURN_PATH, GET_VAT_PERIODS_PATH,
};
use serde_json::{json, Value};

fn post(server: &MockVeroVatServer, path: &str, body: &Value) -> std::io::Result<(u16, Value)> {
    let mut stream = TcpStream::connect(server.addr())?;
    stream.set_read_timeout(Some(Duration::from_millis(40)))?;
    let body = serde_json::to_vec(body).unwrap();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nX-Test-Marker: recorded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        server.addr(),
        body.len()
    )?;
    stream.write_all(&body)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let head = std::str::from_utf8(&response[..split]).unwrap();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = serde_json::from_slice(&response[split + 4..]).unwrap();
    Ok((status, body))
}

fn submitted_return() -> Value {
    json!({
        "ReplacementReturn": false,
        "BusinessId": "1234567-8",
        "FilingPeriod": "2026-07-31",
        "NoActivity": false,
        "VATDetails": {"VATOnDomesticSalesByTaxRate":{"HighVATRate":255.0}},
        "ContactDetails": {"FullName":"Test User","PhoneNumber":"+358401234567"}
    })
}

#[test]
fn binds_only_an_ephemeral_loopback_port_and_records_period_request() {
    let server = MockVeroVatServer::start(Scenario::file_receipt("documented-periods")).unwrap();
    assert!(server.addr().ip().is_loopback());
    assert_ne!(server.addr().port(), 0);
    assert_eq!(server.scenario_name(), "documented-periods");

    let request = json!({"BusinessId":"1234567-8","FilingYear":2026});
    let (status, body) = post(&server, GET_VAT_PERIODS_PATH, &request).unwrap();
    assert_eq!(status, 200);
    assert_eq!(body["FilingPeriod"][0]["Period"], "2026-07-31");
    assert_eq!(body["FilingPeriod"][0]["DueDate"], "2026-09-14");
    assert_eq!(body["FilingPeriod"][0]["Frequency"], "VATMON");
    assert_eq!(body["FilingPeriod"][0]["Status"], "Missing");
    assert!(body["FilingPeriod"][0]["CustomsComparisonDataOnImportsOfGoods"].is_number());

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, GET_VAT_PERIODS_PATH);
    assert_eq!(requests[0].json.as_ref(), Some(&request));
    assert_eq!(requests[0].headers["x-test-marker"], "recorded");
}

#[test]
fn file_vat_return_200_records_receipt_and_exact_request() {
    let server = MockVeroVatServer::start(Scenario::file_receipt("documented-200")).unwrap();
    let request = submitted_return();
    let (status, body) = post(&server, FILE_VAT_RETURN_PATH, &request).unwrap();
    assert_eq!(status, 200);
    assert_eq!(body["UniqueIdentifier"], "mock-vat-return-0001");
    assert_eq!(body["AcceptedTimestamp"], "2026-09-14T10:15:30Z");
    assert_eq!(
        server.requests()[0].body,
        serde_json::to_vec(&request).unwrap()
    );
}

#[test]
fn file_vat_return_400_is_structured() {
    let server =
        MockVeroVatServer::start(Scenario::file_structured_error("documented-400")).unwrap();
    let (status, body) = post(&server, FILE_VAT_RETURN_PATH, &submitted_return()).unwrap();
    assert_eq!(status, 400);
    assert_eq!(body["ErrorCode"], "INVALID_DATA");
    assert!(body["ErrorText"].is_string());
    assert_eq!(body["ErrorDetails"].as_array().unwrap().len(), 1);
}

#[test]
fn file_vat_return_timeout_sends_no_response_but_records_attempt() {
    let server = MockVeroVatServer::start(Scenario::file_timeout(
        "documented-timeout",
        Duration::from_millis(100),
    ))
    .unwrap();
    let error = post(&server, FILE_VAT_RETURN_PATH, &submitted_return()).unwrap_err();
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
    let (status, body) = post(&server, GET_FILED_VAT_RETURN_PATH, &json!({})).unwrap();
    assert_eq!(status, 200);
    assert_eq!(body["Status"], "Missing");
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn file_vat_return_5xx_is_an_ambiguous_failure_shape() {
    let server = MockVeroVatServer::start(Scenario::file_server_error("documented-500")).unwrap();
    let (status, body) = post(&server, FILE_VAT_RETURN_PATH, &submitted_return()).unwrap();
    assert_eq!(status, 500);
    assert_eq!(body["ErrorCode"], "INTERNAL_ERROR");
}

#[test]
fn get_filed_return_matching_reflects_latest_accepted_semantic_values() {
    let server = MockVeroVatServer::start(Scenario::file_receipt("documented-matching")).unwrap();
    let filed = submitted_return();
    post(&server, FILE_VAT_RETURN_PATH, &filed).unwrap();
    let (status, body) = post(
        &server,
        GET_FILED_VAT_RETURN_PATH,
        &json!({"BusinessId":"1234567-8","FilingPeriod":"2026-07-31"}),
    )
    .unwrap();
    assert_eq!(status, 200);
    assert_eq!(body["Status"], "Processed");
    assert_eq!(body["SubmittedDate"], "2026-09-14");
    assert_eq!(
        body["VATOnDomesticSales"],
        filed["VATDetails"]["VATOnDomesticSalesByTaxRate"]
    );
}

#[test]
fn get_filed_return_can_be_stale() {
    let mut scenario = Scenario::file_receipt("documented-stale");
    scenario.get_filed_vat_return = GetFiledVatReturnFixture::Stale {
        body: json!({
            "Status":"Being Processed",
            "SubmittedDate":"2026-08-12",
            "VATOnDomesticSales":{"HighVATRate":100.0}
        }),
    };
    let server = MockVeroVatServer::start(scenario).unwrap();
    post(&server, FILE_VAT_RETURN_PATH, &submitted_return()).unwrap();
    let (_, body) = post(&server, GET_FILED_VAT_RETURN_PATH, &json!({})).unwrap();
    assert_eq!(body["Status"], "Being Processed");
    assert_eq!(body["VATOnDomesticSales"]["HighVATRate"], 100.0);
}

#[test]
fn get_filed_return_can_be_missing_without_implying_safe_retry() {
    let mut scenario = Scenario::file_receipt("documented-missing");
    scenario.get_filed_vat_return = GetFiledVatReturnFixture::Missing;
    let server = MockVeroVatServer::start(scenario).unwrap();
    let (status, body) = post(&server, GET_FILED_VAT_RETURN_PATH, &json!({})).unwrap();
    assert_eq!(status, 200);
    assert_eq!(body, json!({"Status":"Missing"}));
}

#[test]
fn get_filed_return_can_be_a_processed_mismatch() {
    let mut scenario = Scenario::file_receipt("documented-mismatch");
    scenario.get_filed_vat_return = GetFiledVatReturnFixture::Mismatch {
        body: json!({
            "Status":"Processed",
            "SubmittedDate":"2026-09-14",
            "VATOnDomesticSales":{"HighVATRate":999.0}
        }),
    };
    let server = MockVeroVatServer::start(scenario).unwrap();
    post(&server, FILE_VAT_RETURN_PATH, &submitted_return()).unwrap();
    let (_, body) = post(&server, GET_FILED_VAT_RETURN_PATH, &json!({})).unwrap();
    assert_eq!(body["Status"], "Processed");
    assert_ne!(
        body["VATOnDomesticSales"],
        submitted_return()["VATDetails"]["VATOnDomesticSalesByTaxRate"]
    );
}
