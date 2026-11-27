//! Runs journalled before a FileVATReturn 2xx without a receipt was classified as
//! an unknown outcome carry `MALFORMED: …` or `HTTP_2xx: …` for that answer. The
//! journal is crate-private, so such a Run can only be reproduced from inside.

use aggountant_broker::{
    Broker, RawResponse, Registry, ScriptedTransport, ScriptedVeroVatPeriodsAdapter, Vault,
    SVC_VERO_VAT_PERIODS,
};
use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use serde_json::json;

use super::{
    FileVatFilingStatus, FileVatInput, FileVatStatusPlan, EFFECT_RECONCILE, EFFECT_SUBMIT_FILING,
    EFFECT_SUBMIT_PAYMENT, GATE_APPROVE_FILING, VAT_INTERRUPTED_BEFORE_FILING,
};
use crate::journal::{EventPayload, Journal};
use crate::{EffectStatus, ProcessEngine, RunStatus};

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
    store_vat_reference(home.path());
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
        let request = SealRequest {
            natural_key: key.into(),
            actor: "test".into(),
            policy_set: PolicySet::default(),
        };
        ledger.seal(&draft, &request).unwrap();
    }
    let authority = ledger.begin_close(&july(), "july-close", "closer").unwrap();
    ledger.seal_period(&july(), &authority, "closer").unwrap();
    home
}

fn periods_broker(home: &std::path::Path) -> Broker {
    let periods = json!({"FilingPeriod":[{
        "Period":"2026-07-31", "Frequency":"VATMONTH",
        "StartDate":"2026-07-01", "EndDate":"2026-07-31",
        "Status":"Missing", "CustomsComparisonDataOnImportsOfGoods":0,
        "DueDate":"2026-09-12"
    }]});
    let mut registry = Registry::new();
    registry.register(Box::new(ScriptedVeroVatPeriodsAdapter::new(Box::new(
        ScriptedTransport::new(vec![Ok(RawResponse::new(200, periods))]),
    ))));
    let mut vault = Vault::new();
    vault.set(SVC_VERO_VAT_PERIODS, "scripted");
    Broker::new(home, vault, registry)
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

/// Start a July Run and journal the filing dispatch ending in `error_code`, the
/// way a Run before the fix recorded a receiptless 2xx.
fn run_with_filing_rejection(home: &std::path::Path, error_code: &str) {
    let engine = ProcessEngine::open(home);
    let started = engine
        .start_file_vat(&input(), &periods_broker(home), &PolicySet::default())
        .unwrap();
    let journal = engine.open_journal(&started.run.run_id).unwrap();
    journal
        .append(EventPayload::EffectExecuting {
            effect_key: EFFECT_SUBMIT_FILING.into(),
        })
        .unwrap();
    journal
        .append(EventPayload::EffectRejected {
            effect_key: EFFECT_SUBMIT_FILING.into(),
            error_code: error_code.into(),
        })
        .unwrap();
}

#[test]
fn an_older_receiptless_success_journal_blocks_manual_completion() {
    for code in [
        "MALFORMED: HTTP 200 omitted a complete FileVATReturn receipt",
        "HTTP_204: unexpected FileVATReturn response",
    ] {
        let home = sealed_july();
        run_with_filing_rejection(home.path(), code);
        let engine = ProcessEngine::open(home.path());
        let refused = engine
            .complete_manual_vat(&july(), "omavero", "archive/vero/2026-07.pdf", None)
            .unwrap_err();
        assert!(
            refused.to_string().contains("OUTCOME_UNKNOWN"),
            "{code}: {refused}"
        );
        let completed = engine
            .complete_manual_vat(
                &july(),
                "omavero",
                "archive/vero/2026-07.pdf",
                Some("archive/vero/2026-07-remote-status.txt"),
            )
            .unwrap();
        assert!(completed.period.vat_filed, "{code}");
    }
}

#[test]
fn a_rejection_the_authority_stated_does_not_block_manual_completion() {
    let home = sealed_july();
    run_with_filing_rejection(home.path(), "INVALID_DATA: Field missing");
    let completed = ProcessEngine::open(home.path())
        .complete_manual_vat(&july(), "omavero", "archive/vero/2026-07.pdf", None)
        .unwrap();
    assert!(completed.period.vat_filed);
}

// --- Runs that stopped before their filing was attempted ---------------------
//
// Older builds left two shapes behind in real homes: a reconcile left
// `executing` (the process died, or an error returned without journalling) and a
// reconcile `rejected` (then shown as `needs_attention`). The journal format has
// not changed since, so these fixtures write exactly the events those builds
// wrote into a home of the current store version.

/// Write the journal of a July Run an older build left in its reconcile: the
/// three effects planned, the reconcile executing and, with `rejection`, rejected.
fn legacy_reconcile_run(home: &std::path::Path, run_id: &str, rejection: Option<&str>) {
    let journal = Journal::new(&home.join("process").join("runs").join(run_id));
    journal
        .append(EventPayload::RunStarted {
            workflow: "file-vat".into(),
            workflow_version: crate::file_vat_workflow_version(),
            git_commit: "0000000".into(),
            policy_set: PolicySet::default().id,
            period: "2026-07".into(),
        })
        .unwrap();
    for (key, effect_type) in [
        (EFFECT_RECONCILE, "Reconcile"),
        (
            EFFECT_SUBMIT_FILING,
            "ExchangeAdapter(vero-file-vat-return)",
        ),
        (EFFECT_SUBMIT_PAYMENT, "ExchangeAdapter(pankki-maksu)"),
    ] {
        journal
            .append(EventPayload::EffectPlanned {
                effect_key: key.into(),
                effect_type: effect_type.into(),
            })
            .unwrap();
    }
    journal
        .append(EventPayload::EffectExecuting {
            effect_key: EFFECT_RECONCILE.into(),
        })
        .unwrap();
    if let Some(code) = rejection {
        journal
            .append(EventPayload::EffectRejected {
                effect_key: EFFECT_RECONCILE.into(),
                error_code: code.into(),
            })
            .unwrap();
    }
}

fn reconcile_code(view: &crate::RunView) -> Option<String> {
    view.effects
        .iter()
        .find(|effect| effect.key == EFFECT_RECONCILE)
        .and_then(|effect| effect.error_code.clone())
}

/// July with its first return recorded as filed by hand, so a later Run of it is
/// a replacement.
fn filed_july() -> tempfile::TempDir {
    let home = sealed_july();
    ProcessEngine::open(home.path())
        .complete_manual_vat(&july(), "omavero", "archive/vero/2026-07.pdf", None)
        .unwrap();
    home
}

fn status_of(engine: &ProcessEngine) -> FileVatFilingStatus {
    match engine.file_vat_status_plan(&july()).unwrap() {
        FileVatStatusPlan::Answered(status) => status.status,
        FileVatStatusPlan::Query(_) => panic!("a Run that sent nothing needs no query"),
    }
}

#[test]
fn an_older_rejected_reconcile_reads_not_filed_and_blocks_nothing() {
    for (home, expected) in [
        (sealed_july(), FileVatFilingStatus::NotFiled),
        (filed_july(), FileVatFilingStatus::FiledWithoutReceipt),
    ] {
        legacy_reconcile_run(
            home.path(),
            "fv-legacy-rejected",
            Some("VAT_FIGURES_MISMATCH"),
        );
        let engine = ProcessEngine::open(home.path());

        // Derived on read: no write turns `needs_attention` into `not_filed`.
        let view = engine.show("fv-legacy-rejected").unwrap();
        assert_eq!(view.status, RunStatus::NotFiled, "{expected:?}");
        assert_eq!(
            reconcile_code(&view).as_deref(),
            Some("VAT_FIGURES_MISMATCH")
        );
        let events = engine.journal("fv-legacy-rejected").unwrap().len();
        let resumed = engine.resume("fv-legacy-rejected").unwrap();
        assert_eq!(resumed.status, RunStatus::NotFiled);
        assert_eq!(engine.journal("fv-legacy-rejected").unwrap().len(), events);

        assert_eq!(status_of(&engine), expected);
        engine
            .refuse_while_filing_unresolved(&july(), None)
            .unwrap();
    }
}

#[test]
fn resume_settles_an_older_reconcile_left_executing() {
    for (home, expected) in [
        (sealed_july(), FileVatFilingStatus::NotFiled),
        (filed_july(), FileVatFilingStatus::FiledWithoutReceipt),
    ] {
        legacy_reconcile_run(home.path(), "fv-legacy-executing", None);
        let engine = ProcessEngine::open(home.path());
        assert_eq!(
            engine.show("fv-legacy-executing").unwrap().status,
            RunStatus::Running
        );

        let settled = engine.resume("fv-legacy-executing").unwrap();
        assert_eq!(settled.status, RunStatus::NotFiled, "{expected:?}");
        assert_eq!(
            reconcile_code(&settled).as_deref(),
            Some(VAT_INTERRUPTED_BEFORE_FILING)
        );
        assert!(settled
            .effects
            .iter()
            .filter(|effect| effect.key != EFFECT_RECONCILE)
            .all(|effect| effect.status == EffectStatus::Planned));
        // Settled once: a second resume records nothing.
        let events = engine.journal("fv-legacy-executing").unwrap().len();
        engine.resume("fv-legacy-executing").unwrap();
        assert_eq!(engine.journal("fv-legacy-executing").unwrap().len(), events);

        assert_eq!(status_of(&engine), expected);
        engine
            .refuse_while_filing_unresolved(&july(), None)
            .unwrap();
    }

    // The first return can then be filed by a new Run.
    let home = sealed_july();
    legacy_reconcile_run(home.path(), "fv-legacy-executing", None);
    let engine = ProcessEngine::open(home.path());
    engine.resume("fv-legacy-executing").unwrap();
    let started = engine
        .start_file_vat(
            &input(),
            &periods_broker(home.path()),
            &PolicySet::default(),
        )
        .unwrap();
    assert_eq!(started.run.status, RunStatus::AwaitingGate);
}

#[test]
fn resume_refuses_to_settle_a_reconcile_whose_filing_may_have_begun() {
    let later_steps: [&dyn Fn(&Journal); 2] = [
        &|journal| {
            journal
                .append(EventPayload::EffectExecuting {
                    effect_key: EFFECT_SUBMIT_FILING.into(),
                })
                .unwrap();
        },
        &|journal| {
            journal
                .append(EventPayload::GateOpened {
                    gate_id: GATE_APPROVE_FILING.into(),
                    gate_kind: "Preview".into(),
                    snapshot_hash: "0".repeat(64),
                })
                .unwrap();
        },
    ];
    for later_step in later_steps {
        let home = sealed_july();
        legacy_reconcile_run(home.path(), "fv-legacy-began", None);
        later_step(&Journal::new(
            &home
                .path()
                .join("process")
                .join("runs")
                .join("fv-legacy-began"),
        ));
        let engine = ProcessEngine::open(home.path());
        let events = engine.journal("fv-legacy-began").unwrap().len();

        let refused = engine.resume("fv-legacy-began").unwrap_err();
        assert_eq!(refused.error_code(), "VAT_FILING_UNRESOLVED");
        let message = refused.to_string();
        assert!(
            message.contains("file-vat status --period 2026-07"),
            "{message}"
        );
        assert!(
            message.contains("file-vat reconcile --run-id fv-legacy-began"),
            "{message}"
        );
        assert_eq!(engine.journal("fv-legacy-began").unwrap().len(), events);
        assert_ne!(
            engine.show("fv-legacy-began").unwrap().status,
            RunStatus::NotFiled
        );
    }
}

/// Store the company's VAT payment reference (fictional), which a first
/// return's payment proposal requires.
fn store_vat_reference(home: &std::path::Path) {
    crate::tax_payments::write_tax_payments(
        home,
        &serde_json::from_str(r#"{"schema_version":1,"self_assessed_reference":"12345672"}"#)
            .unwrap(),
    )
    .unwrap();
}
