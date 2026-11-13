//! Period VAT reconciliation in `file-vat` (issue `vat-untagged-posting-omitted`):
//! output VAT on a VAT account without a VAT code must show in the preview and stop
//! the direct filing before anything is sent. Esimerkki Oy, March 2026.

use serde_json::{json, Value};

use aggountant_broker::{
    Broker, PankkiMaksuAdapter, RawResponse, Registry, ScriptedTransport,
    ScriptedVeroFileVatReturnAdapter, ScriptedVeroFiledVatReturnAdapter,
    ScriptedVeroVatPeriodsAdapter, Transport, TransportError, Vault, SVC_PANKKI_MAKSU,
    SVC_VERO_FILED_VAT_RETURN, SVC_VERO_FILE_VAT_RETURN, SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::{
    vat_expand, PolicySet, VatExpandInput, VatReconciliationStatus, VatTreatment,
};
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{
    AccountId, Currency, Date, Money, PeriodId, Posting, Side, VatCode, VoucherDraft,
};
use aggountant_process::{FileVatInput, ProcessEngine, RunStatus};

fn march() -> PeriodId {
    PeriodId::parse("2026-03").unwrap()
}

fn eur(s: &str) -> Money {
    Money::parse(s, Currency::new("EUR").unwrap()).unwrap()
}

fn line(account: &str, side: Side, amount: &str, code: Option<&str>) -> Posting {
    Posting {
        account: AccountId::new(account).unwrap(),
        amount: eur(amount),
        side,
        vat_code: code.map(|c| VatCode::new(c).unwrap()),
    }
}

fn seal(ledger: &Ledger, key: &str, postings: Vec<Posting>) {
    let d = Date::parse("2026-03-05").unwrap();
    let draft = VoucherDraft::new(d, d, "myyntilasku".into(), postings, None).unwrap();
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

/// The sale from the issue: 2 480,00 € gross, 503,90 € VAT on 2939, tagged only
/// when `code` is given (on the revenue line, where the return reads it).
fn sale(code: Option<&str>) -> Vec<Posting> {
    vec![
        line("1700", Side::Debit, "2480.00", None),
        line("3000", Side::Credit, "1976.10", code),
        line("2939", Side::Credit, "503.90", None),
    ]
}

fn close(ledger: &Ledger) {
    let auth = ledger.begin_close(&march(), "close-run", "closer").unwrap();
    ledger.seal_period(&march(), &auth, "closer").unwrap();
}

fn home_with(vouchers: Vec<Vec<Posting>>) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    for (i, postings) in vouchers.into_iter().enumerate() {
        seal(&ledger, &format!("v{i}"), postings);
    }
    close(&ledger);
    home
}

fn ok(body: Value) -> Result<RawResponse, TransportError> {
    Ok(RawResponse::new(200, body))
}

fn broker(home: &std::path::Path) -> Broker {
    let mut vault = Vault::new();
    let mut reg = Registry::new();
    for s in [
        SVC_VERO_VAT_PERIODS,
        SVC_VERO_FILE_VAT_RETURN,
        SVC_VERO_FILED_VAT_RETURN,
        SVC_PANKKI_MAKSU,
    ] {
        vault.set(s, "secret");
    }
    let periods: Box<dyn Transport> = Box::new(ScriptedTransport::new(vec![ok(json!({
        "FilingPeriod":[{
            "Period":"2026-03-31", "Frequency":"VATMON",
            "StartDate":"2026-03-01", "EndDate":"2026-03-31",
            "Status":"Missing", "CustomsComparisonDataOnImportsOfGoods":0,
            "DueDate":"2026-05-12"
        }]
    }))]));
    let none = || -> Box<dyn Transport> { Box::new(ScriptedTransport::new(vec![])) };
    reg.register(Box::new(ScriptedVeroVatPeriodsAdapter::new(periods)));
    reg.register(Box::new(ScriptedVeroFileVatReturnAdapter::new(none())));
    reg.register(Box::new(ScriptedVeroFiledVatReturnAdapter::new(none())));
    reg.register(Box::new(PankkiMaksuAdapter::new(none())));
    Broker::new(home, vault, reg)
}

fn input(sales_vat: &str, acknowledgement: Option<Value>) -> FileVatInput {
    let mut value = json!({
        "period": "2026-03",
        "sales_vat": {"amount": sales_vat, "currency": "EUR"},
        "purchase_vat": {"amount": "0.00", "currency": "EUR"},
        "business_id": "1234567-1",
        "contact_name": "Maija Meikäläinen",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    });
    if let Some(ack) = acknowledgement {
        value["vat_reconciliation_acknowledgement"] = ack;
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn preview_reports_the_untagged_output_vat_of_the_issue() {
    let home = home_with(vec![sale(None)]);
    let database = home.path().join("fiscal.sqlite");
    let before = std::fs::read(&database).unwrap();

    let preview = ProcessEngine::open(home.path())
        .preview_offline_vat(&march(), &PolicySet::default())
        .unwrap();

    assert_eq!(preview.sales_vat, "0.00");
    let r = &preview.vat_reconciliation;
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.output.untagged.amount_str(), "503.90");
    assert_eq!(r.untagged_vouchers(), vec![1]);
    assert_eq!(r.untagged_postings[0].account, "2939");
    let item = &preview.attention[0];
    assert_eq!(
        (item.code, item.severity),
        ("VAT_ACCOUNTS_UNRECONCILED", "blocking")
    );
    assert_eq!(item.vouchers, vec![1]);
    assert!(item.message.contains("503.90"), "{}", item.message);
    assert_eq!(std::fs::read(database).unwrap(), before);
    assert!(!home.path().join("process/runs").exists());
}

#[test]
fn filing_refuses_untagged_output_vat_before_contacting_the_authority() {
    let home = home_with(vec![sale(None)]);
    let engine = ProcessEngine::open(home.path());
    let broker = broker(home.path());

    let err = engine
        .start_file_vat(&input("0.00", None), &broker, &PolicySet::default())
        .unwrap_err();

    assert_eq!(err.error_code(), "VAT_ACCOUNTS_UNRECONCILED");
    assert!(err.to_string().contains("[1]"), "{err}");
    assert!(err.to_string().contains("503.90"), "{err}");
    assert!(broker
        .outbox_entry(SVC_VERO_VAT_PERIODS, "periods:2026-03")
        .unwrap()
        .is_none());
    assert!(broker
        .outbox_entry(SVC_VERO_FILE_VAT_RETURN, "submit-filing:2026-03")
        .unwrap()
        .is_none());
    assert!(
        !Ledger::open(home.path())
            .unwrap()
            .period_view(&march())
            .unwrap()
            .vat_filed
    );
}

#[test]
fn an_acknowledgement_must_name_exactly_the_untagged_vouchers_and_a_reason() {
    let home = home_with(vec![sale(None)]);
    let engine = ProcessEngine::open(home.path());
    for ack in [
        json!({"reason": "", "vouchers": [1]}),
        json!({"reason": "Myynti on jo ilmoitettu", "vouchers": [2]}),
        json!({"reason": "Myynti on jo ilmoitettu", "vouchers": []}),
    ] {
        let err = engine
            .start_file_vat(
                &input("0.00", Some(ack)),
                &broker(home.path()),
                &PolicySet::default(),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "VAT_RECONCILIATION_ACK_MISMATCH");
    }
}

#[test]
fn an_acknowledged_discrepancy_is_recorded_in_the_run() {
    let home = home_with(vec![sale(None)]);
    let engine = ProcessEngine::open(home.path());
    let reason = "Tilin 2939 vienti on avaavan tositteen ALV-saldo";
    let start = engine
        .start_file_vat(
            &input("0.00", Some(json!({"reason": reason, "vouchers": [1]}))),
            &broker(home.path()),
            &PolicySet::default(),
        )
        .unwrap();

    assert_eq!(start.run.status, RunStatus::AwaitingGate);
    let ack = start.vat_reconciliation_acknowledgement.as_ref().unwrap();
    assert_eq!(
        (ack.reason.as_str(), ack.vouchers.as_slice()),
        (reason, &[1][..])
    );
    let state = std::fs::read_to_string(
        home.path()
            .join("process/runs")
            .join(&start.run.run_id)
            .join("file_vat_state.json"),
    )
    .unwrap();
    assert!(state.contains(reason), "{state}");
}

#[test]
fn an_acknowledgement_is_refused_when_nothing_needs_one() {
    let home = home_with(vec![sale(Some("sales-domestic-standard"))]);
    let err = ProcessEngine::open(home.path())
        .start_file_vat(
            &input(
                "503.91",
                Some(json!({"reason": "varmuuden vuoksi", "vouchers": []})),
            ),
            &broker(home.path()),
            &PolicySet::default(),
        )
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_RECONCILIATION_ACK_MISMATCH");
}

#[test]
fn a_tagged_sale_reconciles_and_files_without_acknowledgement() {
    let home = home_with(vec![sale(Some("sales-domestic-standard"))]);
    let engine = ProcessEngine::open(home.path());

    let preview = engine
        .preview_offline_vat(&march(), &PolicySet::default())
        .unwrap();
    assert_eq!(preview.sales_vat, "503.91");
    assert_eq!(
        preview.vat_reconciliation.status,
        VatReconciliationStatus::Reconciled
    );
    // The per-line computation files one cent more than the invoice's VAT leg.
    assert_eq!(
        preview.vat_reconciliation.output.difference.amount_str(),
        "-0.01"
    );
    assert!(preview.attention.iter().all(|item| item.severity == "info"));

    let start = engine
        .start_file_vat(
            &input("503.91", None),
            &broker(home.path()),
            &PolicySet::default(),
        )
        .unwrap();
    assert_eq!(start.run.status, RunStatus::AwaitingGate);
    assert!(start.vat_reconciliation_acknowledgement.is_none());
}

#[test]
fn input_vat_on_the_input_vat_account_reconciles() {
    let d = Date::parse("2026-03-10").unwrap();
    let purchase = vat_expand(&VatExpandInput {
        amount: eur("125.50"),
        net_account: AccountId::new("7690").unwrap(),
        contra_account: AccountId::new("1910").unwrap(),
        treatment: VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
        document_date: d,
        posting_date: d,
        description: "toimistotarvikkeet".into(),
        attachment_ref: None,
    })
    .unwrap();
    // Sealed as the expansion produced it, so the generated-leg markers are kept.
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    ledger
        .seal(
            &purchase,
            &SealRequest {
                natural_key: "purchase".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    close(&ledger);

    let preview = ProcessEngine::open(home.path())
        .preview_offline_vat(&march(), &PolicySet::default())
        .unwrap();
    let r = &preview.vat_reconciliation;
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert_eq!(r.input.derived.amount_str(), "25.50");
    assert_eq!(r.input.ledger_movement.amount_str(), "25.50");
    assert_eq!(r.input.difference.amount_str(), "0.00");
    assert!(preview.attention.is_empty());
}

#[test]
fn preview_audits_a_period_whose_return_is_already_filed() {
    let home = home_with(vec![sale(None)]);
    let engine = ProcessEngine::open(home.path());
    engine
        .complete_manual_vat(&march(), "omavero", "omavero-receipt-2026-03", None)
        .unwrap();
    let database = home.path().join("fiscal.sqlite");
    let before = std::fs::read(&database).unwrap();

    let preview = engine
        .preview_offline_vat(&march(), &PolicySet::default())
        .unwrap();

    assert_eq!(
        preview.vat_reconciliation.status,
        VatReconciliationStatus::Unreconciled
    );
    assert_eq!(preview.vat_reconciliation.untagged_vouchers(), vec![1]);
    assert_eq!(std::fs::read(database).unwrap(), before);
    assert!(
        Ledger::open(home.path())
            .unwrap()
            .period_view(&march())
            .unwrap()
            .vat_filed
    );
}

#[test]
fn a_vat_coded_correction_in_the_same_month_lets_the_filing_proceed() {
    let correction = vec![
        line("3000", Side::Debit, "1976.10", None),
        line(
            "3000",
            Side::Credit,
            "1976.10",
            Some("sales-domestic-standard"),
        ),
    ];
    let home = home_with(vec![sale(None), correction]);
    let engine = ProcessEngine::open(home.path());

    let preview = engine
        .preview_offline_vat(&march(), &PolicySet::default())
        .unwrap();
    assert_eq!(
        preview.vat_reconciliation.status,
        VatReconciliationStatus::Reconciled
    );
    assert_eq!(preview.attention.len(), 1);
    assert_eq!(
        (preview.attention[0].code, preview.attention[0].severity),
        ("VAT_UNTAGGED_POSTINGS_COVERED", "info")
    );

    let start = engine
        .start_file_vat(
            &input("503.91", None),
            &broker(home.path()),
            &PolicySet::default(),
        )
        .unwrap();
    assert_eq!(start.run.status, RunStatus::AwaitingGate);
}

#[test]
fn a_run_started_before_the_check_cannot_dispatch_an_unreconciled_filing() {
    // A Run persisted by an earlier version carries no acknowledgement; approving
    // it must refuse before the return is sent.
    let home = home_with(vec![sale(None)]);
    let engine = ProcessEngine::open(home.path());
    let start = engine
        .start_file_vat(
            &input(
                "0.00",
                Some(json!({"reason": "avaavan tositteen ALV-saldo", "vouchers": [1]})),
            ),
            &broker(home.path()),
            &PolicySet::default(),
        )
        .unwrap();
    let state_path = home
        .path()
        .join("process/runs")
        .join(&start.run.run_id)
        .join("file_vat_state.json");
    let mut state: Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state
        .as_object_mut()
        .unwrap()
        .remove("vat_reconciliation_acknowledgement")
        .unwrap();
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    let broker = broker(home.path());
    let err = engine
        .answer_file_vat(&start.run.run_id, "ApproveFiling", "approve", &broker)
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_ACCOUNTS_UNRECONCILED");
    assert!(broker
        .outbox_entry(SVC_VERO_FILE_VAT_RETURN, "submit-filing:2026-03")
        .unwrap()
        .is_none());
}
