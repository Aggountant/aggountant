//! `file-vat status` against the loopback Vero API mock: a receipted filing is
//! compared with GetFiledVATReturn/v2, the read never sends FileVATReturn or
//! changes local state, and a failed read is "status unknown", never "not filed".

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use aggountant_broker::{
    Broker, Registry, Vault, VeroFileVatReturnAdapter, VeroFiledVatReturnAdapter,
    VeroVatPeriodsAdapter, SVC_VERO_FILED_VAT_RETURN, SVC_VERO_FILE_VAT_RETURN,
    SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use aggountant_process::{
    FileVatFilingStatus, FileVatInput, FileVatStatus, FileVatStatusPlan, ProcessEngine,
    ProcessError,
};
use aggountant_vero_vat_api_mock::{
    FileVatReturnFixture, GetFiledVatReturnFixture, MockVeroVatServer, MtlsTestCredentials,
    Scenario, FILE_VAT_RETURN_PATH, GET_FILED_VAT_RETURN_PATH,
};

fn july() -> PeriodId {
    PeriodId::parse("2026-07").unwrap()
}

fn eur(amount: &str) -> Money {
    Money::parse(amount, Currency::new("EUR").unwrap()).unwrap()
}

/// Esimerkki Oy's July: sales base 1000,00 (255,00 VAT) and purchases 400,00
/// (102,00 deductible VAT), then the month sealed.
fn sealed_july() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let date = Date::parse("2026-07-15").unwrap();
    let posting = |account: &str, amount: &str, side, vat: Option<&str>| Posting {
        account: AccountId::new(account).unwrap(),
        amount: eur(amount),
        side,
        vat_code: vat.map(|code| aggountant_model::VatCode::new(code).unwrap()),
    };
    for (key, lines) in [
        (
            "sales",
            vec![
                posting("1910", "1000.00", Side::Debit, None),
                posting(
                    "3000",
                    "1000.00",
                    Side::Credit,
                    Some("sales-domestic-standard"),
                ),
            ],
        ),
        (
            "purchase",
            vec![
                posting(
                    "4000",
                    "400.00",
                    Side::Debit,
                    Some("purchase-domestic-standard"),
                ),
                posting("1910", "400.00", Side::Credit, None),
            ],
        ),
    ] {
        let draft = VoucherDraft::new(date, date, key.into(), lines, None).unwrap();
        ledger
            .seal(
                &draft,
                &SealRequest {
                    natural_key: key.into(),
                    actor: "test".into(),
                    policy_set: PolicySet::default(),
                },
            )
            .unwrap();
    }
    let authority = ledger.begin_close(&july(), "july-close", "closer").unwrap();
    ledger.seal_period(&july(), &authority, "closer").unwrap();
    home
}

fn input() -> FileVatInput {
    serde_json::from_value(json!({
        "period": "2026-07",
        "sales_vat": {"amount": "255.00", "currency": "EUR"},
        "purchase_vat": {"amount": "102.00", "currency": "EUR"},
        "business_id": "1234567-1",
        "contact_name": "Esimerkki Oy",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    }))
    .unwrap()
}

fn broker(home: &Path, server: &MockVeroVatServer) -> Broker {
    let url = format!("https://localhost:{}", server.addr().port());
    let timeout = Duration::from_secs(2);
    let credentials = json!({
        "target": "test",
        "business_id": "1234567-1",
        "software_key": "test-software-key",
        "client_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_CERTIFICATE_PEM),
        "client_private_key_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM),
        "ca_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CA_CERTIFICATE_PEM),
        "authorization_token": null,
    })
    .to_string();
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
    Broker::new(home, vault, registry)
}

fn scenario(get_filed_vat_return: GetFiledVatReturnFixture) -> Scenario {
    Scenario {
        get_filed_vat_return,
        ..Scenario::file_receipt("status")
    }
}

/// Start and approve the July filing against `server`; returns the run id.
fn file_july(home: &Path, server: &MockVeroVatServer) -> String {
    let engine = ProcessEngine::open(home);
    let broker = broker(home, server);
    let started = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    engine
        .answer_file_vat(&started.run.run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    started.run.run_id
}

fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn count(server: &MockVeroVatServer, path: &str) -> usize {
    server
        .requests()
        .iter()
        .filter(|request| request.path == path)
        .count()
}

/// Run `file-vat status` and prove it left every local file and the authority's
/// FileVATReturn endpoint untouched.
fn status(home: &Path, server: &MockVeroVatServer) -> Result<FileVatStatus, ProcessError> {
    let before = snapshot(home);
    let filed_before = count(server, FILE_VAT_RETURN_PATH);
    let engine = ProcessEngine::open(home);
    let result = match engine.file_vat_status_plan(&july())? {
        FileVatStatusPlan::Answered(status) => Ok(status),
        FileVatStatusPlan::Query(query) => {
            engine.query_file_vat_status(&query, &broker(home, server))
        }
    };
    assert_eq!(count(server, FILE_VAT_RETURN_PATH), filed_before);
    assert_eq!(snapshot(home), before, "status must not write local state");
    result
}

#[test]
fn processed_return_is_reported_without_a_filing_request() {
    let home = sealed_july();
    let server =
        MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Matching)).unwrap();
    let run_id = file_july(home.path(), &server);
    assert_eq!(count(&server, FILE_VAT_RETURN_PATH), 1);

    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::Processed);
    assert!(reported.authority_checked);
    assert_eq!(reported.run_id.as_deref(), Some(run_id.as_str()));
    assert_eq!(
        reported.receipt.as_ref().unwrap().unique_identifier,
        "mock-vat-return-0001"
    );
    assert_eq!(reported.remote_status.as_deref(), Some("Processed"));
    assert_eq!(count(&server, GET_FILED_VAT_RETURN_PATH), 1);
    assert!(reported.message.contains("do not file this period again"));

    // Repeating the read is safe and still sends no filing.
    assert_eq!(
        status(home.path(), &server).unwrap().status,
        FileVatFilingStatus::Processed
    );
    assert_eq!(count(&server, FILE_VAT_RETURN_PATH), 1);
}

#[test]
fn missing_after_a_receipt_is_received_not_lost() {
    let home = sealed_july();
    let server =
        MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Missing)).unwrap();
    file_july(home.path(), &server);
    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::Received);
    assert_eq!(reported.remote_status.as_deref(), Some("Missing"));
}

#[test]
fn being_processed_and_an_earlier_return_are_pending() {
    for body in [
        json!({"Status": "Being Processed"}),
        json!({"Status": "Processed", "SubmittedDate": "2026-08-01"}),
    ] {
        let home = sealed_july();
        let server =
            MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Stale { body }))
                .unwrap();
        file_july(home.path(), &server);
        assert_eq!(
            status(home.path(), &server).unwrap().status,
            FileVatFilingStatus::PendingAuthorityProcessing
        );
    }
}

#[test]
fn a_processed_return_with_other_values_is_flagged() {
    let home = sealed_july();
    let body = json!({
        "Status": "Processed",
        "SubmittedDate": "2026-09-15",
        "VATOnDomesticSales": {"HighVATRate": 999.0},
        "DeductibleTax": {"DeductibleVAT": 1.0},
    });
    let server =
        MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Mismatch { body }))
            .unwrap();
    file_july(home.path(), &server);
    assert_eq!(
        status(home.path(), &server).unwrap().status,
        FileVatFilingStatus::ProcessedValuesDiffer
    );
}

#[test]
fn an_unknown_remote_status_is_not_interpreted() {
    let home = sealed_july();
    let body = json!({"Status": "Archived"});
    let server =
        MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Stale { body })).unwrap();
    file_july(home.path(), &server);
    let reported = status(home.path(), &server).unwrap();
    assert_eq!(
        reported.status,
        FileVatFilingStatus::UnrecognizedRemoteStatus
    );
    assert_eq!(reported.remote_status.as_deref(), Some("Archived"));
}

#[test]
fn an_unreachable_authority_is_status_unknown_not_not_filed() {
    let home = sealed_july();
    let server =
        MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Matching)).unwrap();
    file_july(home.path(), &server);
    let engine = ProcessEngine::open(home.path());
    let broker = broker(home.path(), &server);
    drop(server);

    let before = snapshot(home.path());
    let FileVatStatusPlan::Query(query) = engine.file_vat_status_plan(&july()).unwrap() else {
        panic!("a receipted filing needs the authority's view");
    };
    let error = engine.query_file_vat_status(&query, &broker).unwrap_err();
    assert_eq!(error.error_code(), "VAT_STATUS_UNKNOWN");
    assert_eq!(error.kind(), "system");
    assert!(
        error.to_string().contains("mock-vat-return-0001"),
        "{error}"
    );
    assert!(error.to_string().contains("do not file again"), "{error}");
    assert_eq!(snapshot(home.path()), before);
}

#[test]
fn an_unknown_outcome_points_to_reconcile_without_asking_the_authority() {
    let home = sealed_july();
    let server = MockVeroVatServer::start_mtls(Scenario::file_server_error("unknown")).unwrap();
    let run_id = file_july(home.path(), &server);
    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::OutcomeUnknown);
    assert!(!reported.authority_checked);
    assert_eq!(
        reported.next_command.as_deref(),
        Some(format!("aggountant file-vat reconcile --run-id {run_id}").as_str())
    );
    assert_eq!(count(&server, GET_FILED_VAT_RETURN_PATH), 0);
}

#[test]
fn a_reconciled_filing_has_no_receipt_to_ask_about() {
    let home = sealed_july();
    let server = MockVeroVatServer::start_mtls(Scenario::file_server_error("unknown")).unwrap();
    let run_id = file_july(home.path(), &server);
    drop(server);
    // Visibility caught up: the authority now shows the matching return.
    let request: Value = {
        let state = home
            .path()
            .join("process/runs")
            .join(&run_id)
            .join("file_vat_state.json");
        let state: Value = serde_json::from_slice(&std::fs::read(state).unwrap()).unwrap();
        serde_json::from_str(state["filing_body"]["request_bytes"].as_str().unwrap()).unwrap()
    };
    let details = &request["VATDetails"];
    let body = json!({
        "Status": "Processed",
        "SubmittedDate": "2099-01-01",
        "VATOnDomesticSales": details["VATOnDomesticSalesByTaxRate"],
        "VATOnPurchasesAndImports": details["VATOnPurchasesAndImports"],
        "DeductibleTax": {"DeductibleVAT": details["DeductibleVAT"]},
        "SalesPurchasesImports": details["SalesPurchasesImports"],
    });
    let server =
        MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Stale { body })).unwrap();
    ProcessEngine::open(home.path())
        .reconcile_file_vat(&run_id, &broker(home.path(), &server))
        .unwrap();
    assert!(
        Ledger::open(home.path())
            .unwrap()
            .period_view(&july())
            .unwrap()
            .vat_filed
    );

    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::FiledWithoutReceipt);
    assert_eq!(reported.run_id.as_deref(), Some(run_id.as_str()));
    assert!(reported.receipt.is_none());
}

#[test]
fn a_manual_filing_and_no_filing_are_answered_locally() {
    let home = sealed_july();
    let server =
        MockVeroVatServer::start_mtls(scenario(GetFiledVatReturnFixture::Matching)).unwrap();
    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::NotFiled);
    assert!(reported.run_id.is_none());

    ProcessEngine::open(home.path())
        .complete_manual_vat(&july(), "omavero", "archive/vero/2026-07.pdf", None)
        .unwrap();
    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::FiledWithoutReceipt);
    assert_eq!(reported.manual_filing.unwrap().channel, "omavero");
    assert!(
        server.requests().is_empty(),
        "nothing was sent to the authority"
    );
}

#[test]
fn a_rejected_filing_is_not_filed_and_names_the_run() {
    let home = sealed_july();
    let server =
        MockVeroVatServer::start_mtls(Scenario::file_structured_error("rejected")).unwrap();
    let run_id = file_july(home.path(), &server);
    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::NotFiled);
    assert_eq!(reported.run_id.as_deref(), Some(run_id.as_str()));
    assert!(
        reported.message.contains("rejected"),
        "{}",
        reported.message
    );
}

#[test]
fn a_success_status_without_a_readable_receipt_is_not_reported_as_not_filed() {
    let home = sealed_july();
    let server = MockVeroVatServer::start_mtls(Scenario {
        file_vat_return: FileVatReturnFixture::Receipt {
            unique_identifier: String::new(),
            accepted_timestamp: "2026-09-14T10:15:30Z".into(),
        },
        ..Scenario::file_receipt("receiptless-200")
    })
    .unwrap();
    let run_id = file_july(home.path(), &server);
    let reported = status(home.path(), &server).unwrap();
    assert_eq!(reported.status, FileVatFilingStatus::OutcomeUnknown);
    assert_eq!(
        reported.next_command.as_deref(),
        Some(format!("aggountant file-vat reconcile --run-id {run_id}").as_str())
    );
}
