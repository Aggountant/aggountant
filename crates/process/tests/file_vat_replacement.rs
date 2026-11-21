//! A replacement VAT return through `file-vat` (issue `vat-replacement-return`).
//!
//! Esimerkki Oy (fictional) files October 2026 with 510.00 € of sales VAT. A
//! missing October invoice (25.50 € of VAT) is sealed in November as a correction
//! to October's return, and October's return is replaced: previewed, approved,
//! sent as a replacement with a reason, receipted next to the first receipt, and
//! recovered without ever sending on a guess.

use serde_json::{json, Value};

use aggountant_broker::{
    Broker, PankkiMaksuAdapter, RawResponse, Registry, ScriptedTransport,
    ScriptedVeroFileVatReturnAdapter, ScriptedVeroFiledVatReturnAdapter,
    ScriptedVeroVatPeriodsAdapter, Transport, TransportError, Vault, SVC_PANKKI_MAKSU,
    SVC_VERO_FILED_VAT_RETURN, SVC_VERO_FILE_VAT_RETURN, SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::{build_draft, DraftInput, PolicySet};
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VatCode};
use aggountant_process::{
    AuthorityOutcome, AuthorityReceipt, EffectStatus, FileVatFilingStatus, FileVatInput,
    FileVatReconcileOutcome, FileVatStatus, FileVatStatusPlan, FixedClock, ProcessEngine,
    ProcessError, RunStatus, RunView, Seam,
};

type Steps = Vec<Result<RawResponse, TransportError>>;

const FIRST_KEY: &str = "submit-filing:2026-10";

fn october() -> PeriodId {
    PeriodId::parse("2026-10").unwrap()
}

fn november() -> PeriodId {
    PeriodId::parse("2026-11").unwrap()
}

fn ok(status: u16, body: Value) -> Result<RawResponse, TransportError> {
    Ok(RawResponse::new(status, body))
}

fn broker(home: &std::path::Path, periods: Steps, submit: Steps, filed: Steps) -> Broker {
    let mut vault = Vault::new();
    let mut registry = Registry::new();
    for service in [
        SVC_VERO_VAT_PERIODS,
        SVC_VERO_FILE_VAT_RETURN,
        SVC_VERO_FILED_VAT_RETURN,
        SVC_PANKKI_MAKSU,
    ] {
        vault.set(service, "secret");
    }
    let transport =
        |steps: Steps| -> Box<dyn Transport> { Box::new(ScriptedTransport::new(steps)) };
    registry.register(Box::new(ScriptedVeroVatPeriodsAdapter::new(transport(
        periods,
    ))));
    registry.register(Box::new(ScriptedVeroFileVatReturnAdapter::new(transport(
        submit,
    ))));
    registry.register(Box::new(ScriptedVeroFiledVatReturnAdapter::new(transport(
        filed,
    ))));
    // A payment would fail the test: no step is scripted.
    registry.register(Box::new(PankkiMaksuAdapter::new(transport(vec![]))));
    Broker::new(home, vault, registry)
}

fn october_period(status: &str) -> Steps {
    vec![ok(
        200,
        json!({"FilingPeriod":[{
            "Period":"2026-10-31", "Frequency":"VATMON",
            "StartDate":"2026-10-01", "EndDate":"2026-10-31",
            "Status":status, "CustomsComparisonDataOnImportsOfGoods":0,
            "DueDate":"2026-12-14"
        }]}),
    )]
}

fn receipt(id: &str, at: &str) -> Result<RawResponse, TransportError> {
    ok(
        200,
        json!({"UniqueIdentifier": id, "AcceptedTimestamp": at}),
    )
}

/// GetFiledVATReturn's processed answer with `sales_vat` at the standard rate.
fn processed(sales_vat: &str, submitted: &str) -> Result<RawResponse, TransportError> {
    let amount: Value = serde_json::from_str(sales_vat).unwrap();
    ok(
        200,
        json!({
            "Status": "Processed",
            "SubmittedDate": submitted,
            "VATOnDomesticSales": {"HighVATRate": amount, "MediumVATRate": 0, "LowVATRate": 0},
            "DeductibleTax": {"DeductibleVAT": 0},
        }),
    )
}

/// The first return as the authority shows it.
fn first_return() -> Result<RawResponse, TransportError> {
    processed("510.00", "2026-11-12")
}

fn input(sales_vat: &str, reasons: Option<Value>) -> FileVatInput {
    let mut input = json!({
        "period": "2026-10",
        "sales_vat": {"amount": sales_vat, "currency": "EUR"},
        "purchase_vat": {"amount": "0.00", "currency": "EUR"},
        "business_id": "1234567-1",
        "contact_name": "Esimerkki Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    });
    if let Some(reasons) = reasons {
        input["replacement_reasons"] = reasons;
    }
    serde_json::from_value(input).unwrap()
}

fn replacement_input() -> FileVatInput {
    input("535.50", Some(json!(["calculation-error"])))
}

fn seal_sale(
    ledger: &Ledger,
    key: &str,
    document: &str,
    posting: &str,
    base: &str,
    corrects: Option<PeriodId>,
) -> u64 {
    let eur = Currency::new("EUR").unwrap();
    let line = |account: &str, side, vat_code: Option<&str>| Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(base, eur.clone()).unwrap(),
        side,
        vat_code: vat_code.map(|code| VatCode::new(code).unwrap()),
    };
    let draft = build_draft(DraftInput {
        document_date: Date::parse(document).unwrap(),
        posting_date: Date::parse(posting).unwrap(),
        description: "Fictional sales invoice".into(),
        postings: vec![
            line("1910", Side::Debit, None),
            line("3000", Side::Credit, Some("sales-domestic-standard")),
        ],
        attachment_ref: None,
        buyer_vat_id: None,
        corrects_vat_period: corrects,
    })
    .unwrap();
    let request = SealRequest {
        natural_key: key.into(),
        actor: "test".into(),
        policy_set: PolicySet::default(),
    };
    ledger.seal(&draft, &request).unwrap().number
}

/// October sealed with one sale of 2000.00 (510.00 € of VAT), not yet filed.
fn sealed_october() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    seal_sale(
        &ledger,
        "october-sale",
        "2026-10-15",
        "2026-10-15",
        "2000.00",
        None,
    );
    let authority = ledger
        .begin_close(&october(), "close-run", "closer")
        .unwrap();
    ledger
        .seal_period(&october(), &authority, "closer")
        .unwrap();
    home
}

/// File October's first return through a Run; its receipt is `RECEIPT-1`.
fn file_first_return(engine: &ProcessEngine, home: &std::path::Path) -> String {
    let broker = broker(
        home,
        october_period("Missing"),
        vec![receipt("RECEIPT-1", "2026-11-12T08:00:00Z")],
        vec![],
    );
    let run_id = engine
        .start_file_vat(&input("510.00", None), &broker, &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    run_id
}

/// A home whose October return is filed and then found 25.50 € short: the missing
/// invoice is sealed in November as a correction to October. Returns the first
/// Run and the correction voucher's number.
fn corrected_home() -> (tempfile::TempDir, ProcessEngine, String, u64) {
    let home = sealed_october();
    let engine = ProcessEngine::open(home.path());
    let first_run = file_first_return(&engine, home.path());
    let correction = seal_sale(
        &Ledger::open(home.path()).unwrap(),
        "october-correction",
        "2026-10-20",
        "2026-11-20",
        "100.00",
        Some(october()),
    );
    (home, engine, first_run, correction)
}

fn start_replacement(
    engine: &ProcessEngine,
    home: &std::path::Path,
    submit: Steps,
) -> (String, Broker) {
    let broker = broker(
        home,
        october_period("Processed"),
        submit,
        vec![first_return()],
    );
    let start = engine
        .start_file_vat_replacement(&replacement_input(), &broker, &PolicySet::default())
        .unwrap();
    (start.run.run_id, broker)
}

fn attempts(home: &std::path::Path, key: &str) -> usize {
    broker(home, vec![], vec![], vec![])
        .outbox_entry(SVC_VERO_FILE_VAT_RETURN, key)
        .unwrap()
        .map_or(0, |entry| entry.attempts.len())
}

/// The outbox key of every replacement Run of the home.
fn replacement_keys(home: &std::path::Path) -> std::collections::BTreeSet<String> {
    let mut keys = std::collections::BTreeSet::new();
    for run in std::fs::read_dir(home.join("process/runs")).unwrap() {
        let Ok(bytes) = std::fs::read(run.unwrap().path().join("file_vat_state.json")) else {
            continue;
        };
        let state: Value = serde_json::from_slice(&bytes).unwrap();
        if let Some(replaces) = state["replacement"]["replaces_filings"].as_u64() {
            let hash = state["filing_sent_hash"].as_str().unwrap();
            keys.insert(format!(
                "submit-filing:2026-10:replaces-{replaces}:{}",
                &hash[..16]
            ));
        }
    }
    keys
}

/// How many replacement requests left for October, under any key.
fn replacement_attempts(home: &std::path::Path) -> usize {
    replacement_keys(home)
        .iter()
        .map(|key| attempts(home, key))
        .sum()
}

fn filings(home: &std::path::Path) -> usize {
    Ledger::open(home)
        .unwrap()
        .vat_filings(&october())
        .unwrap()
        .len()
}

fn sent_request(home: &std::path::Path, run_id: &str) -> Value {
    let path = home
        .join("process/runs")
        .join(run_id)
        .join("file_vat_state.json");
    let state: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    serde_json::from_str(state["filing_body"]["request_bytes"].as_str().unwrap()).unwrap()
}

fn effect(view: &RunView, key: &str) -> EffectStatus {
    view.effects
        .iter()
        .find(|effect| effect.key == key)
        .unwrap()
        .status
}

fn receipt_of(view: &RunView) -> Option<String> {
    let filing = view.effects.iter().find(|e| e.key == "submit-filing")?;
    match filing.outcome.clone()? {
        AuthorityOutcome::Accepted {
            receipt:
                Some(AuthorityReceipt::Vero {
                    unique_identifier, ..
                }),
            ..
        } => Some(unique_identifier),
        _ => None,
    }
}

fn status(engine: &ProcessEngine, home: &std::path::Path, filed: Steps) -> FileVatStatus {
    match engine.file_vat_status_plan(&october()).unwrap() {
        FileVatStatusPlan::Answered(status) => status,
        FileVatStatusPlan::Query(query) => engine
            .query_file_vat_status(&query, &broker(home, vec![], vec![], filed))
            .unwrap(),
    }
}

/// A clock at a Finnish winter local time (UTC+2).
fn finnish_clock(local: &str) -> FixedClock {
    let local = jiff::civil::DateTime::strptime("%Y-%m-%d %H:%M", local).unwrap();
    let utc = jiff::tz::Offset::constant(2).to_timestamp(local).unwrap();
    FixedClock::new(u64::try_from(utc.as_second()).unwrap())
}

fn reconcile_at(
    engine: &ProcessEngine,
    home: &std::path::Path,
    run_id: &str,
    local: &str,
    answer: Result<RawResponse, TransportError>,
) -> aggountant_process::FileVatReconcile {
    let broker = broker(home, vec![], vec![], vec![answer]);
    engine
        .reconcile_file_vat_at(run_id, &broker, &finnish_clock(local))
        .unwrap()
}

fn code(error: &ProcessError) -> String {
    let text = error.to_string();
    let detail = text
        .split_once(": ")
        .map_or(text.as_str(), |(_, rest)| rest);
    detail.split(':').next().unwrap_or_default().to_string()
}

// --- The figures ------------------------------------------------------------

#[test]
fn the_correction_moves_to_octobers_figures_and_out_of_novembers() {
    let (_home, engine, _first, correction) = corrected_home();
    let policy = PolicySet::default();

    let october_preview = engine.preview_offline_vat(&october(), &policy).unwrap();
    assert_eq!(october_preview.sales_vat, "535.50");
    assert_eq!(october_preview.filings.len(), 1);
    let due = october_preview
        .attention
        .iter()
        .find(|item| item.code == "VAT_REPLACEMENT_DUE")
        .expect("the preview says a replacement is due");
    assert_eq!(due.vouchers, [correction]);
    assert!(october_preview
        .filing_instruction
        .contains("file-vat replace"));

    let november_preview = engine.preview_offline_vat(&november(), &policy).unwrap();
    assert_eq!(november_preview.sales_vat, "0.00");
    let elsewhere = november_preview
        .attention
        .iter()
        .find(|item| item.code == "VAT_CORRECTIONS_OF_OTHER_PERIODS")
        .expect("the preview names the voucher it leaves out");
    assert_eq!(elsewhere.vouchers, [correction]);
    // The VAT accounts of November agree with its return without the correction.
    assert!(november_preview
        .attention
        .iter()
        .all(|item| item.code != "VAT_ACCOUNTS_UNRECONCILED"));
}

// --- The replacement --------------------------------------------------------

#[test]
fn a_replacement_is_previewed_approved_sent_and_receipted_beside_the_first() {
    let (home, engine, first_run, correction) = corrected_home();
    let home = home.path();
    let broker = broker(
        home,
        october_period("Processed"),
        vec![receipt("RECEIPT-2", "2026-12-03T09:00:00Z")],
        vec![first_return()],
    );

    let start = engine
        .start_file_vat_replacement(&replacement_input(), &broker, &PolicySet::default())
        .unwrap();
    assert_eq!(start.run.status, RunStatus::AwaitingGate);
    assert_eq!(start.run.gates[0].gate_id, "ApproveFiling");
    assert_eq!(start.net_vat, "535.50");
    // The Run pays nothing: the difference is settled on the tax account.
    assert_eq!(start.bank_payment_required, "0.00");
    let preview = start.replacement.expect("a replacement preview");
    assert_eq!(preview.replaces_filing, 1);
    assert_eq!(preview.changes.len(), 1);
    assert_eq!(preview.changes[0].return_field, Some(301));
    assert_eq!(preview.changes[0].approved.as_f64(), Some(535.5));
    assert_eq!(preview.changes[0].authority.as_f64(), Some(510.0));
    // The approval binds the whole return again, marked as a replacement.
    let request: Value = serde_json::from_str(&start.filing_request).unwrap();
    assert_eq!(request["ReplacementReturn"], true);
    assert_eq!(request["ReplacementReason"], "CLC");
    assert_eq!(
        request["VATDetails"]["VATOnDomesticSalesByTaxRate"]["HighVATRate"].to_string(),
        "535.50"
    );
    let run_id = start.run.run_id;
    assert_eq!(filings(home), 1, "nothing is recorded before the approval");
    assert_eq!(replacement_attempts(home), 0);

    let done = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert!(done
        .gates
        .iter()
        .all(|gate| gate.gate_id != "ApprovePayment"));
    assert_eq!(replacement_attempts(home), 1);
    assert_eq!(
        attempts(home, FIRST_KEY),
        1,
        "the first return is not re-sent"
    );

    // Both filings are in the books; the replacement is in force.
    let ledger = Ledger::open(home).unwrap();
    let recorded = ledger.vat_filings(&october()).unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(
        recorded[0].receipt.as_ref().unwrap().unique_identifier,
        "RECEIPT-1"
    );
    assert_eq!(
        recorded[1].receipt.as_ref().unwrap().unique_identifier,
        "RECEIPT-2"
    );
    assert_eq!(recorded[1].run_id.as_deref(), Some(run_id.as_str()));
    assert!(ledger.verify().unwrap().ok);
    let _ = correction;

    // Each Run shows the receipt of its own request.
    assert_eq!(receipt_of(&done).as_deref(), Some("RECEIPT-2"));
    assert_eq!(
        receipt_of(&engine.show(&first_run).unwrap()).as_deref(),
        Some("RECEIPT-1")
    );

    // Status asks about the return in force and lists both filings.
    let shown = status(&engine, home, vec![processed("535.50", "2026-12-03")]);
    assert_eq!(shown.status, FileVatFilingStatus::Processed);
    assert_eq!(shown.run_id.as_deref(), Some(run_id.as_str()));
    assert_eq!(shown.receipt.unwrap().unique_identifier, "RECEIPT-2");
    assert_eq!(shown.filings.len(), 2);
    assert!(shown.unfiled_corrections.is_empty());
    // While the authority still shows the first return, the replacement is pending.
    let pending = status(&engine, home, vec![first_return()]);
    assert_eq!(
        pending.status,
        FileVatFilingStatus::PendingAuthorityProcessing
    );
}

#[test]
fn status_of_a_filed_period_names_the_corrections_no_filing_covers() {
    let (home, engine, first_run, correction) = corrected_home();
    let shown = status(&engine, home.path(), vec![first_return()]);
    assert_eq!(shown.status, FileVatFilingStatus::Processed);
    assert_eq!(shown.run_id.as_deref(), Some(first_run.as_str()));
    assert_eq!(shown.unfiled_corrections, [correction]);
    assert!(shown.message.contains("file-vat replace"));
}

#[test]
fn a_first_filing_is_still_refused_for_a_filed_period() {
    let (home, engine, _first, _correction) = corrected_home();
    let broker = broker(
        home.path(),
        october_period("Missing"),
        vec![receipt("RECEIPT-X", "2026-12-03T09:00:00Z")],
        vec![],
    );
    let error = engine
        .start_file_vat(&input("535.50", None), &broker, &PolicySet::default())
        .unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert!(error.to_string().contains("file-vat replace"));
    // Reasons on a first filing are refused rather than ignored.
    let error = engine
        .start_file_vat(&replacement_input(), &broker, &PolicySet::default())
        .unwrap_err();
    assert_eq!(code(&error), "VAT_REPLACEMENT_NOT_A_FIRST_FILING");
    assert_eq!(attempts(home.path(), FIRST_KEY), 1);
    assert_eq!(filings(home.path()), 1);
}

#[test]
fn a_replacement_needs_a_filed_period_and_a_reason() {
    let home = sealed_october();
    let engine = ProcessEngine::open(home.path());
    let unused = broker(home.path(), vec![], vec![], vec![]);
    let policy = PolicySet::default();

    let error = engine
        .start_file_vat_replacement(&replacement_input(), &unused, &policy)
        .unwrap_err();
    assert_eq!(error.error_code(), "VAT_PERIOD_NOT_FILED");
    let error = engine
        .start_file_vat_replacement(&input("510.00", None), &unused, &policy)
        .unwrap_err();
    assert_eq!(code(&error), "VAT_REPLACEMENT_REASON_REQUIRED");
    assert!(engine.list().unwrap().is_empty(), "no Run was created");

    // An empty or repeated reason list is refused once the period is filed.
    file_first_return(&engine, home.path());
    for reasons in [json!([]), json!(["calculation-error", "calculation-error"])] {
        let broker = broker(home.path(), october_period("Processed"), vec![], vec![]);
        let error = engine
            .start_file_vat_replacement(&input("510.00", Some(reasons)), &broker, &policy)
            .unwrap_err();
        assert!(error.to_string().contains("VERO_VAT_REPLACEMENT_REASON"));
    }
    assert_eq!(replacement_attempts(home.path()), 0);
}

#[test]
fn several_reasons_are_sent_as_a_comma_separated_list() {
    let (home, engine, _first, _correction) = corrected_home();
    let broker = broker(
        home.path(),
        october_period("Processed"),
        vec![],
        vec![first_return()],
    );
    let reasons = json!(["law-interpretation", "tax-audit-guidance"]);
    let start = engine
        .start_file_vat_replacement(
            &input("535.50", Some(reasons)),
            &broker,
            &PolicySet::default(),
        )
        .unwrap();
    assert_eq!(
        sent_request(home.path(), &start.run.run_id)["ReplacementReason"],
        "LAW,TXA"
    );
}

#[test]
fn a_replacement_is_not_offered_against_what_the_authority_does_not_show_processed() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let policy = PolicySet::default();
    let refused = |filed: Result<RawResponse, TransportError>, periods: &str| {
        let broker = broker(home, october_period(periods), vec![], vec![filed]);
        engine
            .start_file_vat_replacement(&replacement_input(), &broker, &policy)
            .unwrap_err()
    };

    // The return in force is still being processed.
    let error = refused(
        ok(200, json!({"Status": "Being Processed"})),
        "Being Processed",
    );
    assert_eq!(code(&error), "VERO_VAT_RETURN_NOT_PROCESSED");
    // The authority already shows the corrected values (replaced in OmaVero).
    let error = refused(processed("535.50", "2026-12-01"), "Processed");
    assert_eq!(code(&error), "VAT_REPLACEMENT_CHANGES_NOTHING");
    assert!(error.to_string().contains("complete-manual --replacement"));
    // The period is outside the adjustment period.
    let error = refused(first_return(), "Expired");
    assert_eq!(code(&error), "VERO_VAT_PERIOD_NOT_FILEABLE");

    assert_eq!(replacement_attempts(home), 0);
    assert_eq!(filings(home), 1);
    // None of the refused Runs keeps the period from a replacement.
    let (run_id, _) = start_replacement(&engine, home, vec![]);
    assert_eq!(
        engine.show(&run_id).unwrap().status,
        RunStatus::AwaitingGate
    );
}

#[test]
fn a_replacement_previewed_before_another_correction_is_not_sent() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let (run_id, broker) = start_replacement(
        &engine,
        home,
        vec![receipt("RECEIPT-2", "2026-12-03T09:00:00Z")],
    );
    seal_sale(
        &Ledger::open(home).unwrap(),
        "october-correction-2",
        "2026-10-21",
        "2026-11-21",
        "200.00",
        Some(october()),
    );
    let error = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap_err();
    assert!(error.to_string().contains("VAT_FILING_SOURCE_CHANGED"));
    assert_eq!(replacement_attempts(home), 0);
    assert_eq!(filings(home), 1);
}

#[test]
fn a_rejected_replacement_leaves_the_first_return_in_force() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let (run_id, rejecting) = start_replacement(
        &engine,
        home,
        vec![ok(
            400,
            json!({"ErrorCode": "VAT106", "ErrorText": "ReplacementReason needs a value"}),
        )],
    );
    let parked = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &rejecting)
        .unwrap();
    assert_eq!(parked.status, RunStatus::NeedsAttention);
    assert_eq!(filings(home), 1);
    // A stated rejection is not an open outcome: a corrected replacement (other
    // bytes, here another reason) may be sent.
    let broker = broker(
        home,
        october_period("Processed"),
        vec![receipt("RECEIPT-2", "2026-12-03T09:00:00Z")],
        vec![first_return()],
    );
    let second = engine
        .start_file_vat_replacement(
            &input("535.50", Some(json!(["law-interpretation"]))),
            &broker,
            &PolicySet::default(),
        )
        .unwrap()
        .run
        .run_id;
    let done = engine
        .answer_file_vat(&second, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(filings(home), 2);
    assert_eq!(replacement_attempts(home), 2);
}

#[test]
fn of_two_approved_replacements_only_the_first_is_sent() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let (first, first_broker) = start_replacement(
        &engine,
        home,
        vec![receipt("RECEIPT-2", "2026-12-03T09:00:00Z")],
    );
    let other_broker = broker(
        home,
        october_period("Processed"),
        vec![receipt("RECEIPT-3", "2026-12-03T09:05:00Z")],
        vec![first_return()],
    );
    let other = engine
        .start_file_vat_replacement(
            &input("535.50", Some(json!(["law-interpretation"]))),
            &other_broker,
            &PolicySet::default(),
        )
        .unwrap()
        .run
        .run_id;

    engine
        .answer_file_vat(&first, "ApproveFiling", "approve", &first_broker)
        .unwrap();
    // The other Run was prepared against the first return, which is replaced.
    let error = engine
        .answer_file_vat(&other, "ApproveFiling", "approve", &other_broker)
        .unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert_eq!(replacement_attempts(home), 1);
    assert_eq!(filings(home), 2);
}

// --- Never resend on a guess ------------------------------------------------

/// A replacement whose FileVATReturn timed out: the outcome is unknown.
fn replacement_with_unknown_outcome(engine: &ProcessEngine, home: &std::path::Path) -> String {
    let (run_id, broker) = start_replacement(engine, home, vec![Err(TransportError::Timeout)]);
    let parked = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(parked.status, RunStatus::NeedsAttention);
    assert_eq!(replacement_attempts(home), 1);
    assert_eq!(filings(home), 1, "an unknown outcome records no filing");
    run_id
}

fn refuses_a_new_replacement(engine: &ProcessEngine, home: &std::path::Path) {
    let broker = broker(
        home,
        october_period("Processed"),
        vec![receipt("RECEIPT-X", "2026-12-03T09:00:00Z")],
        vec![first_return()],
    );
    let error = engine
        .start_file_vat_replacement(&replacement_input(), &broker, &PolicySet::default())
        .unwrap_err();
    assert_eq!(error.error_code(), "VAT_FILING_UNRESOLVED");
    assert_eq!(replacement_attempts(home), 1, "nothing more was sent");
}

#[test]
fn an_unknown_replacement_blocks_another_until_the_authority_has_answered() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let run_id = replacement_with_unknown_outcome(&engine, home);
    refuses_a_new_replacement(&engine, home);

    let shown = status(&engine, home, vec![]);
    assert_eq!(shown.status, FileVatFilingStatus::OutcomeUnknown);
    assert_eq!(shown.run_id.as_deref(), Some(run_id.as_str()));
    assert_eq!(shown.filings.len(), 1);
    // Recording a manual replacement needs evidence that the outcome was resolved.
    let error = engine
        .complete_manual_vat_replacement(&october(), "omavero", "archive/r.pdf", None)
        .unwrap_err();
    assert!(error.to_string().contains("OUTCOME_UNKNOWN"));

    // The authority still shows the first return, unchanged: that is the
    // replacement's "not found". One such answer does not release the period.
    let first = reconcile_at(&engine, home, &run_id, "2026-12-04 10:00", first_return());
    assert_eq!(first.outcome, FileVatReconcileOutcome::NotFound);
    assert!(first.message.contains("does not show the replacement"));
    refuses_a_new_replacement(&engine, home);
    let early = reconcile_at(&engine, home, &run_id, "2026-12-07 09:59", first_return());
    assert_eq!(early.outcome, FileVatReconcileOutcome::NotFound);

    // A second one a bank day later closes the Run, and a new Run sends once.
    let closed = reconcile_at(&engine, home, &run_id, "2026-12-07 10:00", first_return());
    assert_eq!(closed.outcome, FileVatReconcileOutcome::ClosedNotFiled);
    assert_eq!(effect(&closed.run, "submit-filing"), EffectStatus::Skipped);
    assert_eq!(filings(home), 1);

    let (second, broker) = start_replacement(
        &engine,
        home,
        vec![receipt("RECEIPT-2", "2026-12-08T09:00:00Z")],
    );
    let done = engine
        .answer_file_vat(&second, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    // The same bytes under the same key: the outbox holds both attempts.
    assert_eq!(replacement_attempts(home), 2);
    assert_eq!(filings(home), 2);
}

#[test]
fn an_unknown_replacement_the_authority_shows_processed_is_adopted_without_sending() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let run_id = replacement_with_unknown_outcome(&engine, home);

    let adopted = reconcile_at(
        &engine,
        home,
        &run_id,
        "2026-12-04 10:00",
        processed("535.50", "2099-01-01"),
    );
    assert_eq!(adopted.outcome, FileVatReconcileOutcome::Adopted);
    assert_eq!(adopted.run.status, RunStatus::Completed);
    assert_eq!(replacement_attempts(home), 1, "nothing was sent again");

    let recorded = Ledger::open(home).unwrap().vat_filings(&october()).unwrap();
    assert_eq!(recorded.len(), 2);
    // That read returns no receipt, and none is invented.
    assert!(recorded[1].receipt.is_none());
    assert!(recorded[1].sent_hash.is_some());
    // The period is free for a later replacement again.
    let shown = status(&engine, home, vec![]);
    assert_eq!(shown.status, FileVatFilingStatus::FiledWithoutReceipt);
}

#[test]
fn another_answer_about_an_unknown_replacement_counts_for_nothing() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let run_id = replacement_with_unknown_outcome(&engine, home);

    // A processed return that is neither the first one nor the replacement.
    let other = reconcile_at(
        &engine,
        home,
        &run_id,
        "2026-12-04 10:00",
        processed("600.00", "2099-01-01"),
    );
    assert_eq!(other.outcome, FileVatReconcileOutcome::Unresolved);
    assert_eq!(other.differences.len(), 1);
    // The authority is processing something: the earlier values prove nothing.
    let processing = reconcile_at(
        &engine,
        home,
        &run_id,
        "2026-12-07 10:00",
        ok(200, json!({"Status": "Being Processed"})),
    );
    assert_eq!(processing.outcome, FileVatReconcileOutcome::Unresolved);
    refuses_a_new_replacement(&engine, home);
    assert_eq!(filings(home), 1);
}

#[test]
fn a_replacement_interrupted_after_its_receipt_is_recorded_without_sending() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let (run_id, broker) = start_replacement(
        &engine,
        home,
        vec![receipt("RECEIPT-2", "2026-12-03T09:00:00Z")],
    );
    // The authority accepted the replacement; the process died before the books.
    engine
        .answer_file_vat_seam(
            &run_id,
            "ApproveFiling",
            "approve",
            &broker,
            Seam::CrashAfterFilingBrokerSuccess,
        )
        .unwrap();
    assert_eq!(filings(home), 1);
    refuses_a_new_replacement(&engine, home);
    let shown = status(&engine, home, vec![]);
    assert_eq!(shown.status, FileVatFilingStatus::InterruptedFiled);
    assert_eq!(shown.receipt.unwrap().unique_identifier, "RECEIPT-2");

    drop(broker);
    let idle = idle_broker(home);
    let adopted = engine.reconcile_file_vat(&run_id, &idle).unwrap();
    assert_eq!(adopted.outcome, FileVatReconcileOutcome::Adopted);
    assert_eq!(adopted.run.status, RunStatus::Completed);
    let recorded = Ledger::open(home).unwrap().vat_filings(&october()).unwrap();
    assert_eq!(recorded.len(), 2);
    assert_eq!(
        recorded[1].receipt.as_ref().unwrap().unique_identifier,
        "RECEIPT-2"
    );
    assert_eq!(replacement_attempts(home), 1);
}

fn idle_broker(home: &std::path::Path) -> Broker {
    broker(home, vec![], vec![], vec![])
}

#[test]
fn a_replacement_interrupted_after_the_books_catches_up_without_a_second_filing() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let (run_id, sending) = start_replacement(
        &engine,
        home,
        vec![receipt("RECEIPT-2", "2026-12-03T09:00:00Z")],
    );
    engine
        .answer_file_vat_seam(
            &run_id,
            "ApproveFiling",
            "approve",
            &sending,
            Seam::CrashAfterFilingFiscalCommit,
        )
        .unwrap();
    assert_eq!(filings(home), 2);

    let idle = broker(home, vec![], vec![], vec![]);
    let adopted = engine.reconcile_file_vat(&run_id, &idle).unwrap();
    assert_eq!(adopted.outcome, FileVatReconcileOutcome::Adopted);
    assert_eq!(adopted.run.status, RunStatus::Completed);
    assert_eq!(filings(home), 2);
    assert_eq!(replacement_attempts(home), 1);
}

fn replacement_due(engine: &ProcessEngine) -> Vec<u64> {
    engine
        .preview_offline_vat(&october(), &PolicySet::default())
        .unwrap()
        .attention
        .into_iter()
        .find(|item| item.code == "VAT_REPLACEMENT_DUE")
        .map(|item| item.vouchers)
        .unwrap_or_default()
}

#[test]
fn a_correction_sealed_while_a_replacement_was_unresolved_is_still_due_after_adoption() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    let run_id = replacement_with_unknown_outcome(&engine, home);
    // The replacement was sent with 535.50. While its outcome is open, another
    // missing October invoice is sealed: the sent return does not contain it.
    let later = seal_sale(
        &Ledger::open(home).unwrap(),
        "october-correction-2",
        "2026-10-21",
        "2026-11-21",
        "200.00",
        Some(october()),
    );

    let adopted = reconcile_at(
        &engine,
        home,
        &run_id,
        "2026-12-04 10:00",
        processed("535.50", "2099-01-01"),
    );
    assert_eq!(adopted.outcome, FileVatReconcileOutcome::Adopted);
    assert_eq!(filings(home), 2);

    // The filing was recorded after the second correction was sealed, but it
    // does not cover it: a further replacement is due, and status says so.
    assert_eq!(replacement_due(&engine), [later]);
    let shown = status(&engine, home, vec![]);
    assert_eq!(shown.unfiled_corrections, [later]);
    assert!(shown.message.contains("file-vat replace"));
}

#[test]
fn an_interrupted_first_return_never_claims_a_later_replacement() {
    let home = sealed_october();
    let engine = ProcessEngine::open(home.path());
    let home = home.path();
    // Run 1 is interrupted before anything left; the user files in OmaVero.
    let sending = broker(home, october_period("Missing"), vec![], vec![]);
    let run1 = engine
        .start_file_vat(&input("510.00", None), &sending, &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    engine
        .answer_file_vat_seam(
            &run1,
            "ApproveFiling",
            "approve",
            &sending,
            Seam::CrashBeforeFilingDispatch,
        )
        .unwrap();
    engine
        .complete_manual_vat(
            &october(),
            "omavero",
            "archive/vero/2026-10.pdf",
            Some("archive/vero/2026-10-remote-status.txt"),
        )
        .unwrap();
    // Later the manual return is replaced through the API.
    seal_sale(
        &Ledger::open(home).unwrap(),
        "october-correction",
        "2026-10-20",
        "2026-11-20",
        "100.00",
        Some(october()),
    );
    let (replacement, replacing) = start_replacement(
        &engine,
        home,
        vec![receipt("RECEIPT-2", "2026-12-03T09:00:00Z")],
    );
    engine
        .answer_file_vat(&replacement, "ApproveFiling", "approve", &replacing)
        .unwrap();
    assert_eq!(filings(home), 2);

    // Run 1 is closed as superseded by the manual filing; it claims neither
    // filing and offers no payment approval.
    let closed = engine
        .reconcile_file_vat(&run1, &idle_broker(home))
        .unwrap();
    assert_eq!(closed.outcome, FileVatReconcileOutcome::Superseded);
    assert!(closed.message.contains("archive/vero/2026-10.pdf"));
    assert_eq!(effect(&closed.run, "submit-filing"), EffectStatus::Skipped);
    assert!(closed
        .run
        .gates
        .iter()
        .all(|gate| gate.gate_id != "ApprovePayment"));
    assert_eq!(filings(home), 2);
}

// --- The e-service fallback -------------------------------------------------

#[test]
fn a_replacement_made_in_omavero_is_recorded_with_its_evidence() {
    let (home, engine, _first, correction) = corrected_home();
    let home = home.path();

    // Without the explicit replacement, a filed period is still refused.
    let error = engine
        .complete_manual_vat(&october(), "omavero", "archive/vero/2026-10-r.pdf", None)
        .unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert_eq!(filings(home), 1);

    let recorded = engine
        .complete_manual_vat_replacement(&october(), "omavero", "archive/vero/2026-10-r.pdf", None)
        .unwrap();
    assert_eq!(
        recorded.manual_filing.evidence_ref,
        "archive/vero/2026-10-r.pdf"
    );
    assert_eq!(filings(home), 2);
    // The same command again (a lost answer) records nothing more.
    engine
        .complete_manual_vat_replacement(&october(), "omavero", "archive/vero/2026-10-r.pdf", None)
        .unwrap();
    assert_eq!(filings(home), 2);

    let shown = status(&engine, home, vec![]);
    assert_eq!(shown.status, FileVatFilingStatus::FiledWithoutReceipt);
    assert_eq!(shown.manual_filing.unwrap().channel, "omavero");
    assert_eq!(shown.filings.len(), 2);
    assert_eq!(
        shown.filings[0].receipt.as_ref().unwrap().unique_identifier,
        "RECEIPT-1"
    );
    assert!(shown.unfiled_corrections.is_empty());
    let _ = correction;
}

#[test]
fn a_manual_replacement_is_refused_for_a_period_that_is_not_filed() {
    let home = sealed_october();
    let engine = ProcessEngine::open(home.path());
    let error = engine
        .complete_manual_vat_replacement(&october(), "omavero", "archive/r.pdf", None)
        .unwrap_err();
    assert_eq!(error.error_code(), "VAT_PERIOD_NOT_FILED");
    assert_eq!(filings(home.path()), 0);
}

#[test]
fn a_manual_replacement_with_resolution_evidence_settles_an_unknown_replacement() {
    let (home, engine, _first, _correction) = corrected_home();
    let home = home.path();
    replacement_with_unknown_outcome(&engine, home);

    engine
        .complete_manual_vat_replacement(
            &october(),
            "omavero",
            "archive/vero/2026-10-r.pdf",
            Some("archive/vero/2026-10-remote-status.txt"),
        )
        .unwrap();
    assert_eq!(filings(home), 2);
    // The unknown Run no longer blocks: a later replacement may be prepared.
    let shown = status(&engine, home, vec![]);
    assert_eq!(shown.status, FileVatFilingStatus::FiledWithoutReceipt);
    let broker = broker(
        home,
        october_period("Processed"),
        vec![],
        vec![processed("600.00", "2026-12-05")],
    );
    let start = engine
        .start_file_vat_replacement(&replacement_input(), &broker, &PolicySet::default())
        .unwrap();
    assert_eq!(start.replacement.unwrap().replaces_filing, 2);
    // A second replacement has its own outbox key: nothing was sent under it.
    let key = replacement_keys(home)
        .into_iter()
        .find(|key| key.contains(":replaces-2:"))
        .expect("the Run replaces the second filing");
    assert_eq!(attempts(home, &key), 0);
}
