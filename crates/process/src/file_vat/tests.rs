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

use super::{FileVatInput, EFFECT_SUBMIT_FILING};
use crate::journal::EventPayload;
use crate::ProcessEngine;

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
