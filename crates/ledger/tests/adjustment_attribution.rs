//! Tests for `closeyear-adjustment-attribution`: a year-end adjustment
//! (poistot/jaksotukset/D7) posts to the *next open* period (A10 — the closing
//! year's quarters are already Sealed) but must affect the *closing* year's
//! tuloslaskelma/tase. `Ledger::attribute_adjustment` binds such a voucher to its
//! statement year, and `year_vouchers` (hence `compute_statements` and the sealed
//! `statements_hash`) folds it into that year while treating it as a prior-year
//! carry-forward — never a double-count — when the next year is projected.
//!
//! Before this fix, the adjustment fell into the calendar year it posted in, so the
//! closing year's depreciation expense and result were understated and misattributed
//! to the following year (the headline M6 gap the whole S6 review flagged).

use aggountant_domain::{compute_statements, kpa_policy, PolicySet, StatementProvenance};
use aggountant_ledger::{Ledger, LedgerError, SealRequest};
use aggountant_model::{
    AccountId, Currency, Date, Money, OpeningBalance, PeriodId, Posting, Side, VoucherDraft,
};
use rust_decimal::Decimal;

const POLICY_SET: &str = "fi.core.2026-01@stub";

fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn eur() -> Currency {
    Currency::new("EUR").unwrap()
}

/// Seal a two-posting voucher (`debit`/`credit`) with `doc_date == posting_date`,
/// returning the allocated voucher number.
fn seal(ledger: &Ledger, date: &str, debit: &str, credit: &str, amount: &str, nk: &str) -> u64 {
    seal_dated(ledger, date, date, debit, credit, amount, nk)
}

/// Seal a voucher whose document and posting dates differ — the shape of a year-end
/// adjustment (documents the closing year's end, posts into the next open period).
fn seal_dated(
    ledger: &Ledger,
    doc_date: &str,
    posting_date: &str,
    debit: &str,
    credit: &str,
    amount: &str,
    nk: &str,
) -> u64 {
    let postings = vec![
        Posting {
            account: AccountId::new(debit).unwrap(),
            amount: Money::parse(amount, eur()).unwrap(),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: AccountId::new(credit).unwrap(),
            amount: Money::parse(amount, eur()).unwrap(),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    let draft = VoucherDraft::new(
        Date::parse(doc_date).unwrap(),
        Date::parse(posting_date).unwrap(),
        "test".into(),
        postings,
        None,
    )
    .unwrap();
    ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: nk.into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap()
        .number
}

/// The 2025 base year: capital injection, a sale (income 10000) and an expense
/// (2000). Result before any year-end adjustment = 8000; bank = 13000.
fn seal_base_year_2025(ledger: &Ledger) {
    seal(ledger, "2025-01-05", "1910", "2000", "5000.00", "cap");
    seal(ledger, "2025-03-31", "1910", "3000", "10000.00", "sale");
    seal(ledger, "2025-06-30", "7690", "1910", "2000.00", "exp");
}

/// A `2000.00` depreciation adjustment (6000 expense debit / 1160 accumulated-
/// depreciation credit) documenting 2025-12-31 but posted into 2026-Q1 (the next
/// open period). Returns its voucher number.
fn seal_depreciation_into_2026(ledger: &Ledger) -> u64 {
    seal_dated(
        ledger,
        "2025-12-31",
        "2026-01-02",
        "6000",
        "1160",
        "2000.00",
        "poisto-2025",
    )
}

/// The `result` of `compute_statements(year)` over the ledger's current attribution
/// state, parsed to a `Decimal`.
fn projected_result(ledger: &Ledger, year: i32) -> Decimal {
    let data = ledger.fiscal_year_data(year).unwrap();
    let vouchers = &data.vouchers;
    let chart = ledger.chart_list(None).unwrap();
    let provenance = StatementProvenance {
        chart_version: ledger.chart_version().unwrap(),
        policy_set: POLICY_SET.to_string(),
        kpa_policy_version: aggountant_domain::kpa_policy_version().to_string(),
    };
    let statements = compute_statements(
        &data.fiscal_year,
        data.opening_balance.as_ref(),
        vouchers,
        &chart,
        kpa_policy(),
        provenance,
    )
    .unwrap();
    Decimal::from_str_exact(&statements.result).unwrap()
}

/// Signed balance an opening voucher carries for `account` (debit +, credit −).
fn opening_signed(opening: &OpeningBalance, account: &str) -> Decimal {
    opening
        .postings()
        .iter()
        .filter(|p| p.account.as_str() == account)
        .map(Posting::signed_amount)
        .sum()
}

/// Seal every quarter of `year` (empty periods are fine) and then seal the year with
/// a well-formed stand-in `statements_hash`.
fn seal_whole_year(ledger: &Ledger, year: i32) {
    for q in 1..=4 {
        let p = PeriodId::new(year, q).unwrap();
        let auth = ledger.begin_close(&p, "cy", "test").unwrap();
        ledger.seal_period(&p, &auth, "test").unwrap();
    }
    ledger
        .seal_year(year, &"a".repeat(64), "cy", "test")
        .unwrap();
}

#[test]
fn attribution_moves_the_adjustment_into_the_closing_years_result() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger);

    // Before attribution the depreciation lands in 2026 (its posting year): 2025 is
    // understated. A later-year projection now refuses to invent opening balances from
    // an unsealed prior year.
    assert_eq!(projected_result(&ledger, 2025), Decimal::from(8000));
    let err = ledger.fiscal_year_data(2026).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");

    let view = ledger
        .attribute_adjustment(adj, 2025, "cy", "test")
        .unwrap();
    assert_eq!(view.effective_year, 2025);
    assert_eq!(view.voucher_number, adj);
    assert!(!view.replayed);

    // After attribution 2025 absorbs the depreciation. Only after SealYear may its
    // closing position become 2026 opening data.
    assert_eq!(projected_result(&ledger, 2025), Decimal::from(6000));
    seal_whole_year(&ledger, 2025);
    assert_eq!(projected_result(&ledger, 2026), Decimal::ZERO);
}

#[test]
fn the_attributed_voucher_is_current_in_the_closing_year_and_prior_in_the_next() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger);
    ledger
        .attribute_adjustment(adj, 2025, "cy", "test")
        .unwrap();

    // 2025's projection lists the adjustment among its own vouchers.
    let y2025 = ledger.year_vouchers(2025).unwrap();
    assert!(
        y2025.iter().any(|v| v.number() == adj),
        "the attributed adjustment must be a current voucher of its statement year"
    );

    // 2026's sealed-voucher projection does NOT list it (it is prior-year now). After
    // 2025 is sealed, its balance-sheet effect rides in as first-class opening data.
    seal_whole_year(&ledger, 2025);
    let data = ledger.fiscal_year_data(2026).unwrap();
    assert!(
        data.vouchers.iter().all(|v| v.number() != adj),
        "an adjustment attributed to a prior year must not double-count in the next year"
    );
    let opening = data.opening_balance.as_ref().expect("opening balance");
    assert_eq!(opening_signed(opening, "1160"), Decimal::from(-2000));
    assert_eq!(
        opening_signed(opening, "6000"),
        Decimal::ZERO,
        "an income/expense account is rolled into retained earnings, not carried raw"
    );
}

#[test]
fn attributing_changes_the_closing_years_close_package() {
    // Seal the adjustment, close-package-hash the year, attribute it (year still Open),
    // hash again. The hash must fold the attributed adjustment — i.e. differ.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger);
    let liite = "1".repeat(64);

    let before = ledger
        .close_package_hash_for_year(2025, POLICY_SET, &liite)
        .unwrap();
    ledger
        .attribute_adjustment(adj, 2025, "cy", "test")
        .unwrap();
    let after = ledger
        .close_package_hash_for_year(2025, POLICY_SET, &liite)
        .unwrap();
    assert_ne!(
        before, after,
        "attributing a year-end adjustment must change the closing year's package"
    );
}

#[test]
fn full_close_sequence_seals_then_refuses_a_late_attribution() {
    // The intended end-to-end order: post adjustment, attribute it while the year is
    // Open, seal the year binding the attributed statements, then a *later* attribution
    // to that now-Sealed year is refused (its bound statements must not change).
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger);
    ledger
        .attribute_adjustment(adj, 2025, "cy", "test")
        .unwrap();

    // Compute the statements hash over the attributed projection and seal 2025 with it.
    let data = ledger.fiscal_year_data(2025).unwrap();
    let vouchers = &data.vouchers;
    let chart = ledger.chart_list(None).unwrap();
    let provenance = StatementProvenance {
        chart_version: ledger.chart_version().unwrap(),
        policy_set: POLICY_SET.to_string(),
        kpa_policy_version: aggountant_domain::kpa_policy_version().to_string(),
    };
    let stmts = compute_statements(
        &data.fiscal_year,
        data.opening_balance.as_ref(),
        vouchers,
        &chart,
        kpa_policy(),
        provenance,
    )
    .unwrap();
    let sh = stmts.statements_hash();
    for q in 1..=4 {
        let p = PeriodId::new(2025, q).unwrap();
        let auth = ledger.begin_close(&p, "cy", "test").unwrap();
        ledger.seal_period(&p, &auth, "test").unwrap();
    }
    let view = ledger.seal_year(2025, &sh, "cy", "test").unwrap();
    assert_eq!(view.statements_hash.as_deref(), Some(sh.as_str()));

    // A second adjustment posted into 2026, attributed back to the now-Sealed 2025, is refused.
    let late = seal_dated(
        &ledger,
        "2025-12-31",
        "2026-02-01",
        "7690",
        "1910",
        "10.00",
        "late-adj",
    );
    let err = ledger
        .attribute_adjustment(late, 2025, "cy", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT", "{err}");
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn an_intermediate_year_cannot_seal_over_an_open_prior_year() {
    // The first-class opening invariant makes the old "2026 Sealed while 2025 Open"
    // attribution scenario unrepresentable: SealYear itself refuses that dependency.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_dated(
        &ledger,
        "2025-12-31",
        "2027-01-02",
        "6000",
        "1160",
        "2000.00",
        "poisto-across",
    );
    for q in 1..=4 {
        let p = PeriodId::new(2026, q).unwrap();
        let auth = ledger.begin_close(&p, "cy", "test").unwrap();
        ledger.seal_period(&p, &auth, "test").unwrap();
    }
    let err = ledger
        .seal_year(2026, &"a".repeat(64), "cy", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED", "{err}");
    assert!(err.to_string().contains("2025"), "{err}");
    assert!(ledger.adjustment_attribution(adj).unwrap().is_none());
}

#[test]
fn attribution_is_idempotent_for_the_same_year() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger);

    let first = ledger
        .attribute_adjustment(adj, 2025, "cy", "test")
        .unwrap();
    assert!(!first.replayed);
    let second = ledger
        .attribute_adjustment(adj, 2025, "cy", "test")
        .unwrap();
    assert!(second.replayed, "a repeat to the same year replays");
    assert_eq!(second.effective_year, 2025);

    // Exactly one attribution row survived (verify counts it once).
    assert_eq!(ledger.verify().unwrap().attribution_count, 1);
    assert_eq!(
        ledger.adjustment_attributions_for_year(2025).unwrap().len(),
        1
    );
}

#[test]
fn re_attributing_a_voucher_to_a_different_year_is_refused() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger);
    ledger
        .attribute_adjustment(adj, 2025, "cy", "test")
        .unwrap();

    // 2024 is Open here, so this is refused on immutability, not the sealed-year guard.
    let err = ledger
        .attribute_adjustment(adj, 2024, "cy", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "IDEMPOTENCY_KEY_REUSE", "{err}");
    // The immutable original still stands.
    assert_eq!(
        ledger
            .adjustment_attribution(adj)
            .unwrap()
            .unwrap()
            .effective_year,
        2025
    );
}

#[test]
fn attributing_to_a_sealed_year_is_refused_and_audited() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger);
    // Seal 2024 (empty) — a target year whose statements are already bound.
    seal_whole_year(&ledger, 2024);

    let rejections_before = ledger.rejection_count().unwrap();
    let err = ledger
        .attribute_adjustment(adj, 2024, "cy", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT", "{err}");
    assert!(err.to_string().contains("Sealed"), "{err}");
    assert!(
        ledger.rejection_count().unwrap() > rejections_before,
        "a refused attribution is recorded in the audited rejection log"
    );
    // No attribution was recorded.
    assert!(ledger.adjustment_attribution(adj).unwrap().is_none());
}

#[test]
fn attributing_to_a_year_after_the_posting_year_is_refused() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let adj = seal_depreciation_into_2026(&ledger); // posts in 2026

    let rejections_before = ledger.rejection_count().unwrap();
    let err = ledger
        .attribute_adjustment(adj, 2027, "cy", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT", "{err}");
    assert!(err.to_string().contains("later statement year"), "{err}");
    assert!(
        ledger.rejection_count().unwrap() > rejections_before,
        "the future-year refusal is audited"
    );
}

#[test]
fn attributing_an_unknown_voucher_is_refused_and_audited() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);

    let rejections_before = ledger.rejection_count().unwrap();
    let err = ledger
        .attribute_adjustment(9999, 2025, "cy", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "VOUCHER_NOT_FOUND", "{err}");
    assert!(matches!(err, LedgerError::VoucherNotFound(_)));
    assert!(
        ledger.rejection_count().unwrap() > rejections_before,
        "the unknown-voucher refusal is audited"
    );
}

#[test]
fn attributing_to_the_same_year_it_posts_in_is_allowed() {
    // Not every adjustment crosses a year boundary; attributing to the posting year
    // itself is a harmless explicit no-op (effective_year == posting_year), so the
    // guard is `>`, not `>=`.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let v = seal(&ledger, "2025-06-30", "7690", "1910", "10.00", "same-year");
    let view = ledger.attribute_adjustment(v, 2025, "cy", "test").unwrap();
    assert_eq!(view.effective_year, 2025);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn verify_folds_the_attribution_chain() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_base_year_2025(&ledger);
    let a1 = seal_depreciation_into_2026(&ledger);
    let a2 = seal_dated(
        &ledger,
        "2025-12-31",
        "2026-01-03",
        "7690",
        "1910",
        "50.00",
        "jaksotus-2025",
    );
    ledger.attribute_adjustment(a1, 2025, "cy", "test").unwrap();
    ledger.attribute_adjustment(a2, 2025, "cy", "test").unwrap();

    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.attribution_count, 2);
}

#[test]
fn verify_detects_a_tampered_attribution() {
    let home = init_home();
    let adj;
    {
        let ledger = Ledger::open(home.path()).unwrap();
        seal_base_year_2025(&ledger);
        adj = seal_depreciation_into_2026(&ledger);
        ledger
            .attribute_adjustment(adj, 2025, "cy", "test")
            .unwrap();
        assert!(ledger.verify().unwrap().ok);
    }

    // Silently re-point the attribution to a different statement year without
    // recomputing its chain hash — exactly the tamper that would move a voucher
    // between (possibly sealed) years' results.
    {
        let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        conn.execute(
            "UPDATE adjustment_attribution SET effective_year = 2099 WHERE voucher_number = ?1",
            rusqlite::params![adj],
        )
        .unwrap();
    }

    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger.verify().unwrap_err();
    assert_eq!(err.error_code(), "HASH_CHAIN_BROKEN", "{err}");
}
