//! A filed VAT return is corrected, not re-filed (issue `vat-replacement-return`):
//! - a later voucher sealed with `corrects_vat_period` belongs to the corrected
//!   period's return and to no other, and is accepted only for a filed period;
//! - a replacement return is a further audited filing event of the period, taken
//!   with its own capability and bound to the filing it replaces;
//! - the first-filing path still refuses a filed period.

use aggountant_domain::{build_draft, DraftInput, PolicySet};
use aggountant_ledger::{Ledger, SealRequest, VatFilingEvidence};
use aggountant_model::{
    AccountId, Currency, Date, Instant, Money, PeriodId, Posting, Side, VatCode, VoucherDraft,
};

fn october() -> PeriodId {
    PeriodId::parse("2026-10").unwrap()
}

fn november() -> PeriodId {
    PeriodId::parse("2026-11").unwrap()
}

fn request(natural_key: &str) -> SealRequest {
    SealRequest {
        natural_key: natural_key.into(),
        actor: "test".into(),
        policy_set: PolicySet::default(),
    }
}

/// A domestic sale of `base` at the standard rate, posted on `posting_date`.
fn sale(
    document_date: &str,
    posting_date: &str,
    base: &str,
    corrects: Option<PeriodId>,
) -> DraftInput {
    let eur = Currency::new("EUR").unwrap();
    let posting = |account: &str, side, vat_code: Option<&str>| Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(base, eur.clone()).unwrap(),
        side,
        vat_code: vat_code.map(|code| VatCode::new(code).unwrap()),
    };
    DraftInput {
        document_date: Date::parse(document_date).unwrap(),
        posting_date: Date::parse(posting_date).unwrap(),
        description: "Fictional sales invoice".into(),
        postings: vec![
            posting("1910", Side::Debit, None),
            posting("3000", Side::Credit, Some("sales-domestic-standard")),
        ],
        attachment_ref: None,
        buyer_vat_id: None,
        corrects_vat_period: corrects,
    }
}

fn draft(input: DraftInput) -> VoucherDraft {
    build_draft(input).unwrap()
}

/// October holds one sale and is sealed; `filed` also files its return.
fn home(filed: bool) -> (tempfile::TempDir, Ledger) {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(dir.path()).unwrap();
    ledger
        .seal(
            &draft(sale("2026-10-15", "2026-10-15", "2000.00", None)),
            &request("october-sale"),
        )
        .unwrap();
    let auth = ledger
        .begin_close(&october(), "close-run", "closer")
        .unwrap();
    ledger.seal_period(&october(), &auth, "closer").unwrap();
    if filed {
        let auth = ledger
            .acquire_filing_authority(&october(), "file-run")
            .unwrap();
        ledger
            .file_vat_with_receipt(
                &october(),
                &auth,
                "sent-1",
                "RECEIPT-1",
                "2026-11-12T08:00:00Z",
                "filer",
            )
            .unwrap();
    }
    (dir, ledger)
}

fn correction() -> VoucherDraft {
    draft(sale("2026-10-20", "2026-11-20", "100.00", Some(october())))
}

fn sales_vat(ledger: &Ledger, period: &PeriodId) -> String {
    let vouchers = ledger.vat_period_vouchers(period).unwrap();
    ledger
        .derive_vat_filing_figures(period, &vouchers)
        .unwrap()
        .sales_vat
        .amount_str()
}

#[test]
fn a_correction_voucher_is_counted_in_the_corrected_return_only() {
    let (_dir, ledger) = home(true);
    let sealed = ledger.seal(&correction(), &request("correction")).unwrap();
    // An ordinary November sale stays in November.
    ledger
        .seal(
            &draft(sale("2026-11-05", "2026-11-05", "1000.00", None)),
            &request("november-sale"),
        )
        .unwrap();

    assert_eq!(sales_vat(&ledger, &october()), "535.50");
    assert_eq!(sales_vat(&ledger, &november()), "255.00");
    // In the books the voucher is a November voucher.
    let november_numbers: Vec<u64> = ledger
        .period_vouchers(&november())
        .unwrap()
        .iter()
        .map(|voucher| voucher.number())
        .collect();
    assert!(november_numbers.contains(&sealed.number));
    assert_eq!(
        ledger
            .voucher(sealed.number)
            .unwrap()
            .body
            .corrects_vat_period(),
        Some(&october())
    );
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn a_correction_is_refused_unless_the_period_is_filed() {
    // Sealed but not filed: the period is reopened instead.
    let (_dir, ledger) = home(false);
    let before = ledger.rejection_count().unwrap();
    let error = ledger
        .seal(&correction(), &request("correction"))
        .unwrap_err();
    assert_eq!(error.error_code(), "VAT_PERIOD_NOT_FILED");
    assert_eq!(ledger.rejection_count().unwrap(), before + 1);
    assert_eq!(
        ledger
            .validate_seal(&correction(), "correction")
            .unwrap_err()
            .error_code(),
        "VAT_PERIOD_NOT_FILED"
    );

    // An open period that was never closed has no return either.
    let (_dir, ledger) = home(true);
    let open = draft(sale(
        "2026-09-20",
        "2026-11-20",
        "100.00",
        Some(PeriodId::parse("2026-09").unwrap()),
    ));
    assert_eq!(
        ledger
            .seal(&open, &request("open"))
            .unwrap_err()
            .error_code(),
        "VAT_PERIOD_NOT_FILED"
    );
}

#[test]
fn a_correction_must_be_posted_after_the_period_it_corrects() {
    let own_period = build_draft(sale("2026-10-20", "2026-10-25", "100.00", Some(october())));
    assert_eq!(own_period.unwrap_err().error_code(), "INVALID_VOUCHER_BODY");
}

#[test]
fn corrections_are_listed_by_when_they_were_sealed() {
    let (_dir, ledger) = home(true);
    let recorded_at = |filing: usize| {
        let filings = ledger.vat_filings(&october()).unwrap();
        Instant::parse(&filings[filing].recorded_at).unwrap()
    };
    let first = recorded_at(0);
    assert!(ledger
        .vat_corrections_sealed_after(&october(), first)
        .unwrap()
        .is_empty());
    let sealed = ledger.seal(&correction(), &request("correction")).unwrap();
    assert_eq!(
        ledger
            .vat_corrections_sealed_after(&october(), first)
            .unwrap(),
        [sealed.number]
    );
    // A correction to another period is not October's.
    assert!(ledger
        .vat_corrections_sealed_after(&november(), first)
        .unwrap()
        .is_empty());

    let auth = ledger
        .acquire_replacement_authority(&october(), "replace-run", 1)
        .unwrap();
    ledger
        .replace_vat_filing(
            &october(),
            &auth,
            VatFilingEvidence::Reconciled {
                sent_hash: "sent-2",
            },
            "filer",
        )
        .unwrap();
    assert!(ledger
        .vat_corrections_sealed_after(&october(), recorded_at(1))
        .unwrap()
        .is_empty());
}

#[test]
fn a_manual_replacement_entered_twice_is_recorded_once() {
    let (_dir, ledger) = home(true);
    let record = |evidence_ref: &str| {
        let replaces = ledger.vat_filings(&october()).unwrap().len();
        let auth = ledger
            .acquire_replacement_authority(&october(), "manual-file-vat", replaces)
            .unwrap();
        ledger
            .replace_vat_filing(
                &october(),
                &auth,
                VatFilingEvidence::Manual {
                    channel: "omavero",
                    evidence_ref,
                    remote_status_evidence_ref: None,
                },
                "filer",
            )
            .unwrap();
        ledger.vat_filings(&october()).unwrap().len()
    };
    assert_eq!(record("archive/vero/2026-10-r1.pdf"), 2);
    assert_eq!(
        record("archive/vero/2026-10-r1.pdf"),
        2,
        "the same evidence replays"
    );
    // Another replacement, with its own evidence, is a further filing.
    assert_eq!(record("archive/vero/2026-10-r2.pdf"), 3);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn a_replacement_is_a_second_audited_filing_and_the_latest_is_in_force() {
    let (_dir, ledger) = home(true);
    let auth = ledger
        .acquire_replacement_authority(&october(), "replace-run", 1)
        .unwrap();
    let evidence = VatFilingEvidence::Receipt {
        sent_hash: "sent-2",
        unique_identifier: "RECEIPT-2",
        accepted_timestamp: "2026-12-03T09:00:00Z",
    };
    let view = ledger
        .replace_vat_filing(&october(), &auth, evidence, "filer")
        .unwrap();
    assert!(view.vat_filed);
    // Open→Closing, Closing→Sealed, the first filing, the replacement.
    assert_eq!(view.head_transition_seq, 4);

    let filings = ledger.vat_filings(&october()).unwrap();
    assert_eq!(filings.len(), 2);
    assert_eq!(filings[0].filing, 1);
    assert_eq!(
        filings[0].receipt.as_ref().unwrap().unique_identifier,
        "RECEIPT-1"
    );
    assert_eq!(filings[1].filing, 2);
    assert_eq!(filings[1].sent_hash.as_deref(), Some("sent-2"));
    assert_eq!(filings[1].run_id.as_deref(), Some("replace-run"));
    assert_eq!(
        ledger
            .vat_filing_receipt(&october())
            .unwrap()
            .unwrap()
            .unique_identifier,
        "RECEIPT-2"
    );
    assert!(ledger.verify().unwrap().ok, "both filing events verify");

    // The same replacement again (a crash before the Run recorded it) replays.
    let replayed = ledger
        .replace_vat_filing(&october(), &auth, evidence, "filer")
        .unwrap();
    assert_eq!(replayed.head_transition_seq, 4);
    assert_eq!(ledger.vat_filings(&october()).unwrap().len(), 2);
}

#[test]
fn a_replacement_prepared_against_an_older_filing_is_refused() {
    let (_dir, ledger) = home(true);
    let first = ledger
        .acquire_replacement_authority(&october(), "run-a", 1)
        .unwrap();
    let second = ledger
        .acquire_replacement_authority(&october(), "run-b", 1)
        .unwrap();
    ledger
        .replace_vat_filing(
            &october(),
            &first,
            VatFilingEvidence::Reconciled {
                sent_hash: "sent-a",
            },
            "filer",
        )
        .unwrap();

    let before = ledger.rejection_count().unwrap();
    let error = ledger
        .replace_vat_filing(
            &october(),
            &second,
            VatFilingEvidence::Reconciled {
                sent_hash: "sent-b",
            },
            "filer",
        )
        .unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert_eq!(ledger.rejection_count().unwrap(), before + 1);
    assert_eq!(ledger.vat_filings(&october()).unwrap().len(), 2);
    // A new authority must name the current number of filings.
    assert_eq!(
        ledger
            .acquire_replacement_authority(&october(), "run-b", 1)
            .unwrap_err()
            .error_code(),
        "PERIOD_TRANSITION_CONFLICT"
    );
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn the_two_capabilities_do_not_stand_in_for_each_other() {
    // Nothing to replace before the first filing.
    let (_dir, ledger) = home(false);
    assert_eq!(
        ledger
            .acquire_replacement_authority(&october(), "replace-run", 0)
            .unwrap_err()
            .error_code(),
        "VAT_PERIOD_NOT_FILED"
    );

    // A filed period takes no second first filing, by either first-filing call.
    let (_dir, ledger) = home(true);
    let error = ledger
        .acquire_filing_authority(&october(), "file-run-2")
        .unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert!(error.to_string().contains("file-vat replace"));
    assert_eq!(ledger.vat_filings(&october()).unwrap().len(), 1);
}

#[test]
fn a_manual_replacement_is_recorded_with_its_evidence() {
    let (_dir, ledger) = home(true);
    let auth = ledger
        .acquire_replacement_authority(&october(), "manual-file-vat", 1)
        .unwrap();
    ledger
        .replace_vat_filing(
            &october(),
            &auth,
            VatFilingEvidence::Manual {
                channel: "omavero",
                evidence_ref: "archive/vero/2026-10-replacement.pdf",
                remote_status_evidence_ref: None,
            },
            "filer",
        )
        .unwrap();
    let manual = ledger
        .manual_vat_filing_evidence(&october())
        .unwrap()
        .unwrap();
    assert_eq!(manual.evidence_ref, "archive/vero/2026-10-replacement.pdf");
    // The return in force has no API receipt; the first filing keeps its own.
    assert!(ledger.vat_filing_receipt(&october()).unwrap().is_none());
    let filings = ledger.vat_filings(&october()).unwrap();
    assert_eq!(
        filings[0].receipt.as_ref().unwrap().unique_identifier,
        "RECEIPT-1"
    );
    assert!(ledger.verify().unwrap().ok);
}
