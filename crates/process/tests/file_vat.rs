//! RF3 `file-vat` Run tests — the M3 gate at the Run level. The Broker substrate
//! is proven in `aggountant-broker`; here we prove the *Run* composes it correctly:
//! the preview-gate byte-binding, the `Unknown`→NeedsAttention (no blind retry) +
//! deliberate-retry recovery, the payment net-invariant, the audited irreversible
//! filing, and the `Sealed` precondition.

use serde_json::{json, Value};

use aggountant_broker::{
    Broker, PankkiMaksuAdapter, RawResponse, Registry, ScriptedTransport,
    ScriptedVeroFileVatReturnAdapter, ScriptedVeroFiledVatReturnAdapter,
    ScriptedVeroVatPeriodsAdapter, Transport, TransportError, Vault, VeroAttestations,
    VeroFilingMode, VeroFilingProfile, SVC_PANKKI_MAKSU, SVC_VERO_FILED_VAT_RETURN,
    SVC_VERO_FILE_VAT_RETURN, SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::{
    vat_expand, vat_expand_with_role_table, PolicySet, ReverseChargeScope, VatAccountRoleTable,
    VatExpandInput, VatTreatment,
};
use aggountant_ledger::{Ledger, PeriodState, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use aggountant_process::{EffectStatus, FileVatInput, ProcessEngine, RunStatus, RunView, Seam};

fn effect_status(view: &RunView, key: &str) -> EffectStatus {
    view.effects
        .iter()
        .find(|e| e.key == key)
        .unwrap_or_else(|| panic!("no effect {key} in run"))
        .status
}

fn q1() -> PeriodId {
    PeriodId::parse("2026-Q1").unwrap()
}

fn eur(s: &str) -> Money {
    Money::parse(s, Currency::new("EUR").unwrap()).unwrap()
}

/// Seal into `ledger` (Q1, while still Open) a balanced sales and a balanced
/// purchase voucher whose `document_date` (2026-02, so the 25,5 % standard rate)
/// makes the derived VAT aggregates `round(base × 25,5 %)`. Callers pick bases
/// that yield the sales/purchase VAT their `FileVatInput` will claim, so the
/// file-vat cross-check (derived == input) passes.
fn seal_vat_vouchers(ledger: &Ledger, sales_base: &str, purchase_base: &str) {
    let req = |nk: &str| SealRequest {
        natural_key: nk.to_string(),
        actor: "test".to_string(),
        policy_set: PolicySet::default(),
    };
    let d = Date::parse("2026-02-15").unwrap();
    let acc = |id: &str| AccountId::new(id).unwrap();
    let vat = |c: &str| Some(aggountant_model::VatCode::new(c).unwrap());

    // Sales: credit revenue (3000) tagged output VAT, debit bank (1910).
    let sales = VoucherDraft::new(
        d,
        d,
        "sales invoice".into(),
        vec![
            Posting {
                account: acc("1910"),
                amount: eur(sales_base),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: acc("3000"),
                amount: eur(sales_base),
                side: Side::Credit,
                vat_code: vat("sales-domestic-standard"),
            },
        ],
        None,
    )
    .unwrap();
    ledger.seal(&sales, &req("sales-v1")).unwrap();

    // Purchase: debit purchases (4000) tagged input VAT, credit bank (1910).
    let purchase = VoucherDraft::new(
        d,
        d,
        "purchase invoice".into(),
        vec![
            Posting {
                account: acc("4000"),
                amount: eur(purchase_base),
                side: Side::Debit,
                vat_code: vat("purchase-domestic-standard"),
            },
            Posting {
                account: acc("1910"),
                amount: eur(purchase_base),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    ledger.seal(&purchase, &req("purchase-v1")).unwrap();
}

/// Init a home, seal VAT vouchers whose derived aggregates equal
/// (`sales_vat`, `purchase_vat`), then close+seal Q1 — the `file-vat` precondition.
/// Bases are chosen so `round(base × 25,5 %)` hits the target VAT figure exactly.
fn sealed_home_with(sales_base: &str, purchase_base: &str) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    seal_vat_vouchers(&ledger, sales_base, purchase_base);
    let auth = ledger.begin_close(&q1(), "close-run", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();
    home
}

/// The default sealed home: sales base 10000 → 2550,00 output VAT; purchase base
/// 4000 → 1020,00 deductible VAT (both at 25,5 %) — the figures `input()` claims.
fn sealed_home() -> tempfile::TempDir {
    sealed_home_with("10000.00", "4000.00")
}

fn reopen_add_sales_and_reseal(ledger: &Ledger) {
    let authority = ledger.acquire_authority(&q1(), "close-run").unwrap();
    ledger
        .reopen_period(&q1(), &authority, PeriodState::Closing, "closer")
        .unwrap();
    let d = Date::parse("2026-02-20").unwrap();
    ledger
        .seal_with_authority(
            &VoucherDraft::new(
                d,
                d,
                "late sales correction".into(),
                vec![
                    Posting {
                        account: AccountId::new("1910").unwrap(),
                        amount: eur("100.00"),
                        side: Side::Debit,
                        vat_code: None,
                    },
                    Posting {
                        account: AccountId::new("3000").unwrap(),
                        amount: eur("100.00"),
                        side: Side::Credit,
                        vat_code: Some(
                            aggountant_model::VatCode::new("sales-domestic-standard").unwrap(),
                        ),
                    },
                ],
                None,
            )
            .unwrap(),
            &SealRequest {
                natural_key: "late-sales-correction".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
            Some(&authority),
        )
        .unwrap();
    ledger.seal_period(&q1(), &authority, "closer").unwrap();
}

fn assert_no_filing_attempt(broker: &Broker) {
    assert!(broker
        .outbox_entry(SVC_VERO_FILE_VAT_RETURN, "submit-filing:2026-Q1")
        .unwrap()
        .is_none());
}

/// Init a home, seal a **VAT-A domestic-input** voucher (ADR-7 Pattern B): gross
/// 125,50 at 25,5 % splits into net 100,00 on the expense account (`7690`), deductible
/// input VAT 25,50 on `1763`, contra 125,50 on `1910` — all legs **untagged** (the
/// euro sits on the VAT account, not on a code). The legacy tagged path would derive
/// 0,00 here; the account-sum projection files 25,50 of deductible VAT. Then close +
/// seal Q1.
///
/// (Uses the domestic-input pattern rather than a reverse charge because the current
/// default chart `fi-general-2026` carries `1763` but not the reverse-charge accounts
/// `1798`/`2992`/… — those arrive with company-chart adoption, F25. The reverse-charge
/// account-sum + recompute are proven at the domain level in `aggountant-domain`.)
fn domestic_input_draft() -> VoucherDraft {
    let d = Date::parse("2026-02-15").unwrap();
    vat_expand(&VatExpandInput {
        amount: eur("125.50"),
        net_account: AccountId::new("7690").unwrap(),
        contra_account: AccountId::new("1910").unwrap(),
        treatment: VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
        document_date: d,
        posting_date: d,
        description: "domestic purchase with deductible input VAT".into(),
        attachment_ref: None,
    })
    .unwrap()
}

fn sealed_home_domestic_input() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    ledger
        .seal(
            &domestic_input_draft(),
            &SealRequest {
                natural_key: "di-v1".to_string(),
                actor: "test".to_string(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    let auth = ledger.begin_close(&q1(), "close-run", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();
    home
}

fn sealed_home_shared_reverse_charge_services(tagged_sale: bool) -> tempfile::TempDir {
    const CHART_VERSION: &str =
        "fi-general-2026@dd5d3f339cebfbc670f55cbd0942c775904c58523d9b07ad8eb48b589c79b63b";
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let d = Date::parse("2026-02-15").unwrap();
    for (natural_key, amount, scope) in [
        ("eu-service", "100.00", ReverseChargeScope::EuServices),
        (
            "non-eu-service",
            "100.00",
            ReverseChargeScope::NonEuServices,
        ),
    ] {
        let draft = vat_expand_with_role_table(
            &VatExpandInput {
                amount: eur(amount),
                net_account: AccountId::new("7690").unwrap(),
                contra_account: AccountId::new("1910").unwrap(),
                treatment: VatTreatment::ReverseCharge {
                    rate_class: "domestic-standard".into(),
                    scope,
                },
                document_date: d,
                posting_date: d,
                description: natural_key.into(),
                attachment_ref: None,
            },
            CHART_VERSION,
            VatAccountRoleTable::bundled(),
        )
        .unwrap();
        ledger
            .seal(
                &draft,
                &SealRequest {
                    natural_key: natural_key.into(),
                    actor: "test".into(),
                    policy_set: PolicySet::default(),
                },
            )
            .unwrap();
    }
    if tagged_sale {
        let posting = |account: &str, amount: &str, side, code: Option<&str>| Posting {
            account: AccountId::new(account).unwrap(),
            amount: eur(amount),
            side,
            vat_code: code.map(|code| aggountant_model::VatCode::new(code).unwrap()),
        };
        let sale = VoucherDraft::new(
            d,
            d,
            "fictional tagged domestic sale".into(),
            vec![
                posting("1700", "125.50", Side::Debit, None),
                posting(
                    "3000",
                    "100.00",
                    Side::Credit,
                    Some("sales-domestic-standard"),
                ),
                posting("2939", "25.50", Side::Credit, None),
            ],
            None,
        )
        .unwrap();
        ledger
            .seal(
                &sale,
                &SealRequest {
                    natural_key: "tagged-sale".into(),
                    actor: "test".into(),
                    policy_set: PolicySet::default(),
                },
            )
            .unwrap();
    }
    let auth = ledger.begin_close(&q1(), "close-run", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();
    home
}

type Steps = Vec<Result<RawResponse, TransportError>>;

fn ok(status: u16, body: Value) -> Result<RawResponse, TransportError> {
    Ok(RawResponse::new(status, body))
}

fn verotili_ok() -> Steps {
    vec![ok(
        200,
        json!({"FilingPeriod":[{
            "Period":"2026-03-31", "Frequency":"VATQTR",
            "StartDate":"2026-01-01", "EndDate":"2026-03-31",
            "Status":"Missing", "CustomsComparisonDataOnImportsOfGoods":0,
            "DueDate":"2026-05-12"
        }]}),
    )]
}

fn broker_with(home: &std::path::Path, verotili: Steps, submit: Steps, payment: Steps) -> Broker {
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
    let vt: Box<dyn Transport> = Box::new(ScriptedTransport::new(verotili));
    let submit = submit
        .into_iter()
        .map(|step| {
            step.map(|mut response| {
                if response.status == 200 && response.body.get("UniqueIdentifier").is_none() {
                    let id = response
                        .body
                        .get("receipt")
                        .and_then(Value::as_str)
                        .unwrap_or("R");
                    response.body =
                        json!({"UniqueIdentifier":id,"AcceptedTimestamp":"2026-05-01T10:00:00Z"});
                }
                response
            })
        })
        .collect();
    let sb: Box<dyn Transport> = Box::new(ScriptedTransport::new(submit));
    let filed: Box<dyn Transport> = Box::new(ScriptedTransport::new(vec![ok(
        200,
        json!({"Status":"Missing"}),
    )]));
    let pm: Box<dyn Transport> = Box::new(ScriptedTransport::new(payment));
    reg.register(Box::new(ScriptedVeroVatPeriodsAdapter::new(vt)));
    reg.register(Box::new(ScriptedVeroFileVatReturnAdapter::new(sb)));
    reg.register(Box::new(ScriptedVeroFiledVatReturnAdapter::new(filed)));
    reg.register(Box::new(PankkiMaksuAdapter::new(pm)));
    Broker::new(home, vault, reg)
}

fn input() -> FileVatInput {
    serde_json::from_value(json!({
        "period": "2026-Q1",
        "sales_vat": {"amount": "2550.00", "currency": "EUR"},
        "purchase_vat": {"amount": "1020.00", "currency": "EUR"},
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    }))
    .unwrap()
}

// --- Offline/manual fallback ------------------------------------------------

#[test]
fn offline_preview_is_read_only_and_uses_sealed_ledger_figures() {
    let home = sealed_home();
    let database = home.path().join("fiscal.sqlite");
    let before = std::fs::read(&database).unwrap();
    assert!(!home.path().join("process/runs").exists());

    let preview = ProcessEngine::open(home.path())
        .preview_offline_vat(&q1(), &PolicySet::default())
        .unwrap();

    assert_eq!(preview.sales_vat, "2550.00");
    assert_eq!(preview.purchase_vat, "1020.00");
    assert_eq!(preview.net_vat, "1530.00");
    assert_eq!(preview.payment_amount, "1530.00");
    assert!(preview
        .filing_fields
        .get(aggountant_domain::VatReturnField::Field308)
        .is_some());
    assert_eq!(std::fs::read(database).unwrap(), before);
    assert!(!home.path().join("process/runs").exists());
}

#[test]
fn manual_completion_records_channel_and_non_secret_evidence() {
    let home = sealed_home();
    let completed = ProcessEngine::open(home.path())
        .complete_manual_vat(&q1(), "omavero", "archive/vero/2026-q1-receipt.pdf", None)
        .unwrap();
    assert!(completed.period.vat_filed);
    assert_eq!(completed.manual_filing.channel, "omavero");
    assert_eq!(
        completed.manual_filing.evidence_ref,
        "archive/vero/2026-q1-receipt.pdf"
    );
    assert!(completed.manual_filing.remote_status_evidence_ref.is_none());
    let ledger = Ledger::open(home.path()).unwrap();
    assert!(ledger.verify().unwrap().ok);
    assert_eq!(
        ledger.manual_vat_filing_evidence(&q1()).unwrap(),
        Some(completed.manual_filing)
    );
}

#[test]
fn outcome_unknown_blocks_manual_completion_until_resolution_evidence() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![Err(TransportError::Timeout)],
        vec![],
    );
    let started = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let parked = engine
        .answer_file_vat(&started.run.run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(parked.status, RunStatus::NeedsAttention);

    let refused = engine
        .complete_manual_vat(&q1(), "ilmoitin", "archive/vero/manual-receipt.txt", None)
        .unwrap_err();
    assert!(refused.to_string().contains("OUTCOME_UNKNOWN"));
    assert!(
        !Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed
    );

    let completed = engine
        .complete_manual_vat(
            &q1(),
            "ilmoitin",
            "archive/vero/manual-receipt.txt",
            Some("archive/vero/remote-status-missing.txt"),
        )
        .unwrap();
    assert_eq!(completed.manual_filing.channel, "ilmoitin");
    assert_eq!(
        completed
            .manual_filing
            .remote_status_evidence_ref
            .as_deref(),
        Some("archive/vero/remote-status-missing.txt")
    );
}

#[test]
fn manual_completion_rejects_secret_shaped_or_blank_evidence_references() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    for evidence in [
        "",
        " token",
        "token=secret",
        "https://example.test/r?id=secret",
    ] {
        let error = engine
            .complete_manual_vat(&q1(), "omavero", evidence, None)
            .unwrap_err();
        assert!(error.to_string().contains("INVALID_EVIDENCE_REF"));
    }
}

// --- Happy path -------------------------------------------------------------

#[test]
fn happy_path_files_and_pays() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );

    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    assert_eq!(start.net_vat, "1530.00");
    assert_eq!(start.bank_payment_required, "1530.00");
    assert_eq!(start.run.status, RunStatus::AwaitingGate);
    assert_eq!(start.run.gates[0].gate_id, "ApproveFiling");
    let run_id = start.run.run_id.clone();
    let state_path = home
        .path()
        .join("process/runs")
        .join(&run_id)
        .join("file_vat_state.json");
    let state: Value = serde_json::from_slice(&std::fs::read(state_path).unwrap()).unwrap();
    let request: Value =
        serde_json::from_str(state["filing_body"]["request_bytes"].as_str().unwrap()).unwrap();
    assert_eq!(
        request["VATDetails"]["VATOnDomesticSalesByTaxRate"]["HighVATRate"].to_string(),
        "2550.00"
    );
    assert_eq!(
        request["VATDetails"]["DeductibleVAT"].to_string(),
        "1020.00"
    );

    // Approve the filing → filed + audited, payment gate opens.
    let after_filing = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(after_filing.status, RunStatus::AwaitingGate);
    let ledger = Ledger::open(home.path()).unwrap();
    let view = ledger.period_view(&q1()).unwrap();
    assert!(view.vat_filed, "the filing is recorded in fiscal");
    assert_eq!(view.state, PeriodState::Sealed);
    assert_eq!(
        ledger
            .vat_filing_receipt(&q1())
            .unwrap()
            .unwrap()
            .unique_identifier,
        "VERO-R1"
    );
    assert!(ledger.verify().unwrap().ok, "the audited filing verifies");

    // Approve the payment → Completed.
    let done = engine
        .answer_file_vat(&run_id, "ApprovePayment", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
}

#[test]
fn approving_an_old_proposal_after_reopen_change_and_reseal_sends_no_request() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "MUST-NOT-BE-SENT"}))],
        vec![],
    );
    let started = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();

    let ledger = Ledger::open(home.path()).unwrap();
    reopen_add_sales_and_reseal(&ledger);
    let error = engine
        .answer_file_vat(&started.run.run_id, "ApproveFiling", "approve", &broker)
        .unwrap_err();

    assert!(error.to_string().contains("VAT_FILING_SOURCE_CHANGED"));
    assert_no_filing_attempt(&broker);
    assert!(!ledger.period_view(&q1()).unwrap().vat_filed);
}

#[test]
fn crash_resume_refuses_an_old_proposal_before_dispatch() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "MUST-NOT-BE-SENT"}))],
        vec![],
    );
    let started = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    engine
        .answer_file_vat_seam(
            &started.run.run_id,
            "ApproveFiling",
            "approve",
            &broker,
            Seam::CrashAfterGateAnswer,
        )
        .unwrap();

    let ledger = Ledger::open(home.path()).unwrap();
    reopen_add_sales_and_reseal(&ledger);
    let error = ProcessEngine::open(home.path())
        .resume_file_vat(&started.run.run_id, &broker)
        .unwrap_err();

    assert!(error.to_string().contains("VAT_FILING_SOURCE_CHANGED"));
    assert_no_filing_attempt(&broker);
    assert!(!ledger.period_view(&q1()).unwrap().vat_filed);
}

#[test]
fn awaiting_filing_refuses_a_changed_ready_profile_identity() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(home.path(), verotili_ok(), vec![], vec![]);
    let started = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let matching = VeroFilingProfile {
        schema_version: 1,
        mode: VeroFilingMode::Production,
        business_id: Some(aggountant_model::BusinessId::parse("1572860-0").unwrap()),
        contact_name: Some(aggountant_model::FilingContactName::parse("Ada Operator").unwrap()),
        contact_phone: Some(aggountant_model::FilingContactPhone::parse("+358401234567").unwrap()),
        attestations: VeroAttestations::default(),
    };
    engine
        .ensure_file_vat_profile_matches(&started.run.run_id, &matching)
        .unwrap();
    let mut changed = matching;
    changed.contact_name =
        Some(aggountant_model::FilingContactName::parse("Eve Operator").unwrap());
    let error = engine
        .ensure_file_vat_profile_matches(&started.run.run_id, &changed)
        .unwrap_err();
    assert!(error.to_string().contains("VERO_FILING_PROFILE_CHANGED"));
}

// --- ADR-7 (VAT-A): a VAT-A period files real (non-zero) figures -------------

#[test]
fn vat_a_domestic_input_period_files_the_deductible_vat() {
    // The regression this slice closes: before the account-sum projection, a period
    // whose only VAT lives on untagged VAT-A legs derived to 0,00 and filed an empty
    // ALV-ilmoitus. Now it files the 25,50 of deductible input VAT sitting on `1763`.
    let home = sealed_home_domestic_input();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-DI"}))],
        vec![],
    );

    let di_input: FileVatInput = serde_json::from_value(json!({
        "period": "2026-Q1",
        "sales_vat": {"amount": "0.00", "currency": "EUR"},
        "purchase_vat": {"amount": "25.50", "currency": "EUR"},
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    }))
    .unwrap();

    let start = engine
        .start_file_vat(&di_input, &broker, &PolicySet::default())
        .unwrap();
    // Pure deductible VAT → the net is a refund, so no bank payment is due.
    assert_eq!(start.net_vat, "-25.50");
    assert_eq!(start.bank_payment_required, "0.00");
    let run_id = start.run.run_id.clone();
    let state_path = home
        .path()
        .join("process")
        .join("runs")
        .join(&run_id)
        .join("file_vat_state.json");
    let state: Value = serde_json::from_slice(&std::fs::read(state_path).unwrap()).unwrap();
    let request: Value =
        serde_json::from_str(state["filing_body"]["request_bytes"].as_str().unwrap()).unwrap();
    assert_eq!(request["VATDetails"]["DeductibleVAT"].to_string(), "25.50");
    assert!(!request.to_string().contains("308"));

    // Approving the filing completes the Run (no payment leg): the filing is recorded.
    let done = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    let ledger = Ledger::open(home.path()).unwrap();
    assert!(ledger.period_view(&q1()).unwrap().vat_filed);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn file_vat_preview_contains_complete_shared_service_category_fields() {
    for tagged_sale in [false, true] {
        let home = sealed_home_shared_reverse_charge_services(tagged_sale);
        let engine = ProcessEngine::open(home.path());
        let broker = broker_with(home.path(), verotili_ok(), vec![], vec![]);
        let claimed: FileVatInput = serde_json::from_value(json!({
            "period": "2026-Q1",
            "sales_vat": {"amount": if tagged_sale { "76.50" } else { "51.00" }, "currency": "EUR"},
            "purchase_vat": {"amount": "51.00", "currency": "EUR"},
            "business_id": "1572860-0",
            "contact_name": "Ada Operator",
            "contact_phone": "+358401234567",
            "payee_iban": "FI2112345600000785"
        }))
        .unwrap();
        let start = engine
            .start_file_vat(&claimed, &broker, &PolicySet::default())
            .unwrap();
        assert_eq!(start.net_vat, if tagged_sale { "25.50" } else { "0.00" });
        let preview = engine
            .preview_offline_vat(&q1(), &PolicySet::default())
            .unwrap();
        assert_eq!(preview.net_vat, start.net_vat);
        assert_eq!(
            preview
                .filing_fields
                .get(aggountant_domain::VatReturnField::Field301)
                .unwrap()
                .amount_str(),
            if tagged_sale { "51.00" } else { "25.50" }
        );
        let state_path = home
            .path()
            .join("process/runs")
            .join(start.run.run_id)
            .join("file_vat_state.json");
        let state: Value = serde_json::from_slice(&std::fs::read(state_path).unwrap()).unwrap();
        let request: Value =
            serde_json::from_str(state["filing_body"]["request_bytes"].as_str().unwrap()).unwrap();
        assert_eq!(
            request["VATDetails"]["VATOnDomesticSalesByTaxRate"]["HighVATRate"].to_string(),
            if tagged_sale { "51.00" } else { "25.50" }
        );
        assert_eq!(
            request["VATDetails"]["VATOnPurchasesAndImports"]["ServicesFromEUMemberStates"]
                .to_string(),
            "25.50"
        );
        assert_eq!(
            request["VATDetails"]["SalesPurchasesImports"]["PurchasesOfServicesFromEUMemberStates"]
                .to_string(),
            "100.00"
        );
        assert!(!request.to_string().contains("308"));
    }
}

#[test]
fn vat_a_period_refuses_a_zero_filing_that_ignores_the_sealed_legs() {
    // A CLI input that claims 0,00 (the old blind figure) must now be refused: the
    // sealed VAT-A legs derive to 25,50, so the cross-check parks NeedsAttention.
    let home = sealed_home_domestic_input();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(home.path(), verotili_ok(), vec![], vec![]);
    let zero_input: FileVatInput = serde_json::from_value(json!({
        "period": "2026-Q1",
        "sales_vat": {"amount": "0.00", "currency": "EUR"},
        "purchase_vat": {"amount": "0.00", "currency": "EUR"},
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    }))
    .unwrap();
    let err = engine
        .start_file_vat(&zero_input, &broker, &PolicySet::default())
        .unwrap_err();
    assert!(
        format!("{err}").contains("25.50"),
        "the mismatch names the sealed-ledger figures, got: {err}"
    );
}

#[test]
fn vat_a_filing_binds_the_sealed_expansion_policy() {
    let home = sealed_home_domestic_input();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(home.path(), vec![], vec![], vec![]);
    let claimed: FileVatInput = serde_json::from_value(json!({
        "period": "2026-Q1",
        "sales_vat": {"amount": "0.00", "currency": "EUR"},
        "purchase_vat": {"amount": "25.50", "currency": "EUR"},
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    }))
    .unwrap();

    let filing_policy = PolicySet {
        id: "different-policy".into(),
    };
    let error = engine
        .start_file_vat(&claimed, &broker, &filing_policy)
        .unwrap_err();
    assert_eq!(error.error_code(), "PROCESS_STORE");
    assert!(error.to_string().contains("VAT_STRUCTURE"));
    assert!(error.to_string().contains("differs from filing policy_set"));
}

// --- M3: timeout → Unknown → NeedsAttention → deliberate retry recovers -----

#[test]
fn filing_timeout_parks_and_refuses_blind_retry() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    // The filing submit times out first (Unknown); any blind retry is refused.
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![
            Err(TransportError::Timeout),
            ok(
                409,
                json!({"receipt": "VERO-RECOVERED", "idempotency_key": "file:2026-Q1"}),
            ),
        ],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );

    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();

    // Approve → the submit times out → Unknown → NeedsAttention. NOT filed yet.
    let parked = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(parked.status, RunStatus::NeedsAttention);
    let ledger = Ledger::open(home.path()).unwrap();
    assert!(
        !ledger.period_view(&q1()).unwrap().vat_filed,
        "no blind filing on Unknown"
    );

    // Vero has no idempotency/409 contract: even a deliberate retry is refused.
    let error = engine
        .retry_file_vat_submit(&run_id, "submit-filing", &broker)
        .unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ANSWER");
    let still_missing = engine.reconcile_file_vat(&run_id, &broker).unwrap();
    assert_eq!(still_missing.status, RunStatus::NeedsAttention);
    assert!(!ledger.period_view(&q1()).unwrap().vat_filed);
}

#[test]
fn failed_remote_period_preflight_does_not_claim_local_filing_authority() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let unavailable = broker_with(
        home.path(),
        vec![ok(500, json!({"ErrorCode":"TEMPORARY"}))],
        vec![],
        vec![],
    );
    assert!(engine
        .start_file_vat(&input(), &unavailable, &PolicySet::default())
        .is_err());

    let available = broker_with(home.path(), verotili_ok(), vec![], vec![]);
    let start = engine
        .start_file_vat(&input(), &available, &PolicySet::default())
        .unwrap();
    assert_eq!(start.run.status, RunStatus::AwaitingGate);
}

// --- M3: preview-gate binds approval to the sent bytes ----------------------

#[test]
fn tampering_the_payload_after_binding_is_refused() {
    for tamper_remote_period in [false, true] {
        let home = sealed_home();
        let engine = ProcessEngine::open(home.path());
        let broker = broker_with(
            home.path(),
            verotili_ok(),
            vec![ok(200, json!({"receipt": "R1"}))],
            vec![ok(200, json!({"receipt": "PMT"}))],
        );
        let start = engine
            .start_file_vat(&input(), &broker, &PolicySet::default())
            .unwrap();
        let run_id = start.run.run_id.clone();

        let state_path = home
            .path()
            .join("process")
            .join("runs")
            .join(&run_id)
            .join("file_vat_state.json");
        let mut state: Value =
            serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
        if tamper_remote_period {
            state["vero_period"]["DueDate"] = json!("2099-01-01");
        } else {
            state["filing_body"]["request_bytes"] = json!("{}");
        }
        std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

        // Approval binds both exact filing bytes and the authority preflight snapshot.
        let err = engine
            .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
            .unwrap_err();
        assert_eq!(err.error_code(), "JOURNAL_CORRUPT");
        assert!(
            !Ledger::open(home.path())
                .unwrap()
                .period_view(&q1())
                .unwrap()
                .vat_filed
        );
    }
}

// --- M3: payment net-invariant ---------------------------------------------

#[test]
fn payment_amount_must_equal_reconciled_required() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "R1"}))],
        vec![ok(200, json!({"receipt": "PMT"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();
    engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();

    // Tamper only the payment amount (not the bound sent_hash check target) — the
    // net-invariant guard must catch a mismatch vs. bank_payment_required. To reach
    // the invariant we must keep the sent_hash binding consistent, so we tamper the
    // stored `bank_payment_required` to a different figure than the payment body.
    let state_path = home
        .path()
        .join("process")
        .join("runs")
        .join(&run_id)
        .join("file_vat_state.json");
    let mut state: Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state["bank_payment_required"] = json!("1.00");
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    let err = engine
        .answer_file_vat(&run_id, "ApprovePayment", "approve", &broker)
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_ANSWER");
    assert!(err.to_string().contains("bank_payment_required"));
}

// --- Precondition -----------------------------------------------------------

#[test]
fn file_vat_requires_a_sealed_period() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(home.path(), verotili_ok(), vec![], vec![]);
    let err = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
}

// --- M3 hardening: retry cannot bypass approval or send tampered bytes ------

#[test]
fn retry_before_approval_is_refused() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "R"}))],
        vec![],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    // The filing gate is open but NOT approved — a retry must not dispatch.
    let err = engine
        .retry_file_vat_submit(&start.run.run_id, "submit-filing", &broker)
        .unwrap_err();
    // Either the effect is not parked for retry, or the gate is not approved.
    assert!(
        matches!(err.error_code(), "INVALID_ANSWER" | "GATE_NOT_OPEN"),
        "got {}",
        err.error_code()
    );
    assert!(
        !Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed
    );
}

#[test]
fn retry_with_a_tampered_payload_is_refused() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    // Filing submit times out → parked NeedsAttention (gate already approved).
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![
            Err(TransportError::Timeout),
            ok(200, json!({"receipt": "R"})),
        ],
        vec![],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();
    let parked = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(parked.status, RunStatus::NeedsAttention);

    // Tamper the persisted filing body, then retry — the binding recompute must catch
    // it and refuse to send unapproved bytes.
    let state_path = home
        .path()
        .join("process")
        .join("runs")
        .join(&run_id)
        .join("file_vat_state.json");
    let mut state: Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state["filing_body"]["request_bytes"] = json!("{}");
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    let err = engine
        .retry_file_vat_submit(&run_id, "submit-filing", &broker)
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_ANSWER");
    assert!(
        !Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed
    );
}

// --- M3 hardening: a refund files but sends no zero-value payment ------------

#[test]
fn refund_period_completes_after_filing_without_a_payment() {
    // Sales base 400 → 102,00 output; purchase base 2000 → 510,00 deductible
    // (both at 25,5 %). purchase_vat > sales_vat → net is a refund → bank 0.00.
    let home = sealed_home_with("400.00", "2000.00");
    let engine = ProcessEngine::open(home.path());
    let refund_input: FileVatInput = serde_json::from_value(json!({
        "period": "2026-Q1",
        "sales_vat": {"amount": "102.00", "currency": "EUR"},
        "purchase_vat": {"amount": "510.00", "currency": "EUR"},
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    }))
    .unwrap();
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![], // no payment adapter script needed — no payment is sent
    );
    let start = engine
        .start_file_vat(&refund_input, &broker, &PolicySet::default())
        .unwrap();
    assert_eq!(start.bank_payment_required, "0.00");
    // Approving the filing completes the Run — no ApprovePayment gate opens.
    let done = engine
        .answer_file_vat(&start.run.run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert!(done.gates.iter().all(|g| g.gate_id != "ApprovePayment"));
    assert!(
        Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed
    );
}

// --- Reject path ------------------------------------------------------------

#[test]
fn rejecting_the_filing_gate_declines_without_filing() {
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(home.path(), verotili_ok(), vec![], vec![]);
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let after = engine
        .answer_file_vat(&start.run.run_id, "ApproveFiling", "reject", &broker)
        .unwrap();
    assert_eq!(after.status, RunStatus::Rejected);
    assert!(
        !Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed
    );
}

// --- Accounting integrity: derived-vs-input VAT cross-check -----------------

#[test]
fn file_vat_refuses_figures_that_disagree_with_the_sealed_ledger() {
    // The sealed vouchers derive to 2550,00 / 1020,00, but the input claims a
    // (lower) 2000,00 / 900,00 — the cross-check must refuse to file unverified
    // figures, park NeedsAttention, and record the rejection. Nothing is filed.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(home.path(), verotili_ok(), vec![], vec![]);
    let bad_input: FileVatInput = serde_json::from_value(json!({
        "period": "2026-Q1",
        "sales_vat": {"amount": "2000.00", "currency": "EUR"},
        "purchase_vat": {"amount": "900.00", "currency": "EUR"},
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785",
    }))
    .unwrap();

    let err = engine
        .start_file_vat(&bad_input, &broker, &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_FIGURES_MISMATCH");
    // The error names both the input and the derived figures.
    assert!(err.to_string().contains("2550.00"), "got {err}");
    assert!(err.to_string().contains("2000.00"), "got {err}");

    // Nothing was filed; the period is untouched and still verifies.
    let ledger = Ledger::open(home.path()).unwrap();
    assert!(!ledger.period_view(&q1()).unwrap().vat_filed);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn file_vat_accepts_figures_that_match_the_sealed_ledger() {
    // The mirror of the mismatch case: input == derived → the Run starts and
    // parks on the ApproveFiling gate exactly as before the cross-check existed.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "R1"}))],
        vec![ok(200, json!({"receipt": "PMT"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    assert_eq!(start.run.status, RunStatus::AwaitingGate);
    assert_eq!(start.run.gates[0].gate_id, "ApproveFiling");
}

// --- Resume: the S3 spike-notes gap (crash around the broker/fiscal seam) --------

#[test]
fn resume_adopts_fiscal_after_a_crash_between_commit_and_effectstate() {
    // The window the spike-notes flag: the fiscal `file_vat` COMMIT landed
    // (`vat_filed` set) but the process died before journalling `EffectSucceeded`.
    // `resume_file_vat` must adopt fiscal (fiscal wins) WITHOUT re-hitting the
    // authority — proven here by giving resume an *exhausted* broker (no scripted
    // steps left): if resume touched the network it would fail, but it does not.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();

    // Approve the filing, but crash right after the fiscal COMMIT.
    let crashed = engine
        .answer_file_vat_seam(
            &run_id,
            "ApproveFiling",
            "approve",
            &broker,
            Seam::CrashAfterFilingFiscalCommit,
        )
        .unwrap();
    assert_eq!(
        effect_status(&crashed, "submit-filing"),
        EffectStatus::Executing,
        "process EffectState lags the fiscal COMMIT"
    );
    let ledger = Ledger::open(home.path()).unwrap();
    assert!(
        ledger.period_view(&q1()).unwrap().vat_filed,
        "fiscal is authoritative: the filing IS recorded"
    );
    // No payment gate yet (crash was before the postcondition ran).
    assert!(crashed.gates.iter().all(|g| g.gate_id != "ApprovePayment"));

    // Resume with a fresh engine AND an authority that has no steps left to serve:
    // adopting fiscal must not require a network call.
    let no_network = broker_with(home.path(), vec![], vec![], vec![]);
    let resumed = ProcessEngine::open(home.path())
        .resume_file_vat(&run_id, &no_network)
        .unwrap();
    assert_eq!(
        effect_status(&resumed, "submit-filing"),
        EffectStatus::Succeeded
    );
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
    assert_eq!(resumed.gates.last().unwrap().gate_id, "ApprovePayment");

    // The reconciled Run drives to a clean payment + completion (now with a broker).
    let done = ProcessEngine::open(home.path())
        .answer_file_vat(&run_id, "ApprovePayment", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn resume_completes_a_delivered_but_uncommitted_filing() {
    // The other half of the seam: the authority accepted the return (the outbox holds
    // a terminal-success attempt) but the process died BEFORE the fiscal COMMIT — so
    // `vat_filed` is still false and the filing effect is stuck `Executing`. The
    // deliberate `retry` cannot help (it requires a *parked* Rejected effect), so
    // without resume the Run is wedged. Resume re-drives the idempotent submit: the
    // outbox replays the delivered attempt (no double filing) and the COMMIT completes.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();

    let crashed = engine
        .answer_file_vat_seam(
            &run_id,
            "ApproveFiling",
            "approve",
            &broker,
            Seam::CrashAfterFilingBrokerSuccess,
        )
        .unwrap();
    assert_eq!(
        effect_status(&crashed, "submit-filing"),
        EffectStatus::Executing
    );
    let ledger = Ledger::open(home.path()).unwrap();
    assert!(
        !ledger.period_view(&q1()).unwrap().vat_filed,
        "the fiscal COMMIT never happened"
    );

    // A deliberate retry is refused — the effect is Executing, not parked Rejected.
    let retry_err = engine
        .retry_file_vat_submit(&run_id, "submit-filing", &broker)
        .unwrap_err();
    assert_eq!(retry_err.error_code(), "INVALID_ANSWER");

    // Resume unwedges it: the outbox replays the delivered attempt (no extra network
    // step is consumed) and the fiscal COMMIT completes.
    let resumed = ProcessEngine::open(home.path())
        .resume_file_vat(&run_id, &broker)
        .unwrap();
    assert_eq!(
        effect_status(&resumed, "submit-filing"),
        EffectStatus::Succeeded
    );
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
    assert_eq!(resumed.gates.last().unwrap().gate_id, "ApprovePayment");
    assert!(ledger.period_view(&q1()).unwrap().vat_filed);
    assert!(ledger.verify().unwrap().ok, "no double filing");
}

#[test]
fn resume_does_not_blindly_retry_a_parked_unknown_filing() {
    // M3 invariant: a genuine Unknown parks NeedsAttention and must NOT be
    // auto-re-dispatched — recovery from Unknown is the deliberate `retry`'s job.
    // resume must be a no-op on a parked (Rejected) filing effect, not a blind retry.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        // First submit times out (Unknown → parked). The SECOND step is only reached
        // by a *deliberate* retry — resume must never consume it.
        vec![
            Err(TransportError::Timeout),
            ok(
                409,
                json!({"receipt": "VERO-RECOVERED", "idempotency_key": "file:2026-Q1"}),
            ),
        ],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();

    let parked = engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert_eq!(parked.status, RunStatus::NeedsAttention);
    assert_eq!(
        effect_status(&parked, "submit-filing"),
        EffectStatus::Rejected
    );

    // Resume must leave the parked effect untouched (no re-dispatch, no fiscal filing).
    let resumed = ProcessEngine::open(home.path())
        .resume_file_vat(&run_id, &broker)
        .unwrap();
    assert_eq!(
        resumed.status,
        RunStatus::NeedsAttention,
        "resume must not un-park a genuine Unknown"
    );
    assert_eq!(
        effect_status(&resumed, "submit-filing"),
        EffectStatus::Rejected
    );
    assert!(
        !Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed,
        "resume did not blindly file"
    );

    // A Vero filing retry remains forbidden; reconciliation is the only path.
    let error = engine
        .retry_file_vat_submit(&run_id, "submit-filing", &broker)
        .unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ANSWER");
    assert!(
        !Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed
    );
}

#[test]
fn resume_unwedges_a_crash_between_filing_success_and_postcondition() {
    // The wedge: filing EffectSucceeded is journalled but the process dies before
    // finish_filing_postcondition opens ApprovePayment — the Run is left `Running`
    // with no open gate, and neither the filing branch (already succeeded) nor the
    // payment branch (no gate) advances it. resume must converge the postcondition.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();

    let crashed = engine
        .answer_file_vat_seam(
            &run_id,
            "ApproveFiling",
            "approve",
            &broker,
            Seam::CrashAfterFilingSuccess,
        )
        .unwrap();
    assert_eq!(
        effect_status(&crashed, "submit-filing"),
        EffectStatus::Succeeded
    );
    assert_eq!(crashed.status, RunStatus::Running, "wedged: no open gate");
    assert!(crashed.gates.iter().all(|g| g.gate_id != "ApprovePayment"));
    assert!(
        Ledger::open(home.path())
            .unwrap()
            .period_view(&q1())
            .unwrap()
            .vat_filed
    );

    // Resume converges the postcondition (no authority round-trip needed for the gate).
    let no_network = broker_with(home.path(), vec![], vec![], vec![]);
    let resumed = ProcessEngine::open(home.path())
        .resume_file_vat(&run_id, &no_network)
        .unwrap();
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
    assert_eq!(resumed.gates.last().unwrap().gate_id, "ApprovePayment");

    // And it drives to a clean completion.
    let done = ProcessEngine::open(home.path())
        .answer_file_vat(&run_id, "ApprovePayment", "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert!(Ledger::open(home.path()).unwrap().verify().unwrap().ok);
}

#[test]
fn resume_does_not_blindly_retry_a_parked_unknown_payment() {
    // Same M3 guarantee on the payment leg: a payment Unknown parks NeedsAttention;
    // resume must not auto-re-submit it.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![
            Err(TransportError::Timeout),
            ok(200, json!({"receipt": "PMT-RECOVERED"})),
        ],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();
    engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    let parked = engine
        .answer_file_vat(&run_id, "ApprovePayment", "approve", &broker)
        .unwrap();
    assert_eq!(parked.status, RunStatus::NeedsAttention);
    assert_eq!(
        effect_status(&parked, "submit-payment"),
        EffectStatus::Rejected
    );

    let resumed = ProcessEngine::open(home.path())
        .resume_file_vat(&run_id, &broker)
        .unwrap();
    assert_eq!(resumed.status, RunStatus::NeedsAttention);
    assert_eq!(
        effect_status(&resumed, "submit-payment"),
        EffectStatus::Rejected
    );

    // Deliberate retry still recovers (the recovery transport step was untouched).
    let done = engine
        .retry_file_vat_submit(&run_id, "submit-payment", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
}

#[test]
fn resume_completes_an_approved_but_crashed_payment() {
    // Payment approve-then-crash: the filing is done, the operator approved the
    // payment, but the process died before the payment submit ran. Resume re-drives it.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();
    engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();

    let crashed = engine
        .answer_file_vat_seam(
            &run_id,
            "ApprovePayment",
            "approve",
            &broker,
            Seam::CrashAfterGateAnswer,
        )
        .unwrap();
    assert_eq!(
        effect_status(&crashed, "submit-payment"),
        EffectStatus::Planned,
        "the payment submit never ran"
    );
    assert_eq!(crashed.status, RunStatus::Running, "wedged behind the gate");

    let resumed = ProcessEngine::open(home.path())
        .resume_file_vat(&run_id, &broker)
        .unwrap();
    assert_eq!(resumed.status, RunStatus::Completed);
    assert_eq!(
        effect_status(&resumed, "submit-payment"),
        EffectStatus::Succeeded
    );
    assert!(Ledger::open(home.path()).unwrap().verify().unwrap().ok);
}

#[test]
fn resume_is_idempotent_and_a_noop_on_a_settled_run() {
    // Resume must be safe to run repeatedly and a no-op once nothing is outstanding.
    let home = sealed_home();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        verotili_ok(),
        vec![ok(200, json!({"receipt": "VERO-R1"}))],
        vec![ok(200, json!({"receipt": "PMT-1"}))],
    );
    let start = engine
        .start_file_vat(&input(), &broker, &PolicySet::default())
        .unwrap();
    let run_id = start.run.run_id.clone();

    // Parked on the ApproveFiling gate (nothing approved yet): resume changes nothing.
    let a = engine.resume_file_vat(&run_id, &broker).unwrap();
    let b = engine.resume_file_vat(&run_id, &broker).unwrap();
    assert_eq!(a.status, RunStatus::AwaitingGate);
    assert_eq!(b.status, RunStatus::AwaitingGate);
    assert_eq!(a.gates.last().unwrap().gate_id, "ApproveFiling");

    // Drive to completion, then resume twice more — still a no-op, still Completed.
    engine
        .answer_file_vat(&run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    engine
        .answer_file_vat(&run_id, "ApprovePayment", "approve", &broker)
        .unwrap();
    let c = engine.resume_file_vat(&run_id, &broker).unwrap();
    let d = engine.resume_file_vat(&run_id, &broker).unwrap();
    assert_eq!(c.status, RunStatus::Completed);
    assert_eq!(d.status, RunStatus::Completed);
    assert!(Ledger::open(home.path()).unwrap().verify().unwrap().ok);
}

#[test]
fn monthly_filing_is_scoped_to_the_sealed_month() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let july = PeriodId::parse("2026-07").unwrap();
    let authority = ledger.begin_close(&july, "july-close", "closer").unwrap();
    ledger.seal_period(&july, &authority, "closer").unwrap();
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        vec![ok(
            200,
            json!({"FilingPeriod":[{
                "Period":"2026-07-31", "Frequency":"VATMON",
                "StartDate":"2026-07-01", "EndDate":"2026-07-31",
                "Status":"Missing", "DueDate":"2026-09-14"
            }]}),
        )],
        vec![ok(200, json!({"receipt":"JULY"}))],
        vec![],
    );
    let input: FileVatInput = serde_json::from_value(json!({
        "period":"2026-07",
        "sales_vat":{"amount":"0.00","currency":"EUR"},
        "purchase_vat":{"amount":"0.00","currency":"EUR"},
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban":"FI2112345600000785"
    }))
    .unwrap();
    let started = engine
        .start_file_vat(&input, &broker, &PolicySet::default())
        .unwrap();
    engine
        .answer_file_vat(&started.run.run_id, "ApproveFiling", "approve", &broker)
        .unwrap();
    assert!(ledger.period_view(&july).unwrap().vat_filed);
    assert!(
        !ledger
            .period_view(&PeriodId::parse("2026-08").unwrap())
            .unwrap()
            .vat_filed
    );
    assert!(ledger.verify().unwrap().ok);
}
