use aggountant_model::{
    AccountId, Currency, Date, Hash, Money, Posting, Side, VoucherBody, VoucherDraft,
};
use aggountant_wire::{canonical_bytes, seal_voucher, verify_voucher};

fn draft() -> VoucherDraft {
    let date = Date::parse("2026-05-31").unwrap();
    let eur = Currency::new("EUR").unwrap();
    VoucherDraft::new(
        date,
        date,
        "EU goods sale".into(),
        vec![
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: Money::parse("100.00", eur.clone()).unwrap(),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("3000").unwrap(),
                amount: Money::parse("100.00", eur).unwrap(),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap()
}

#[test]
fn buyer_vat_id_is_hash_bound_and_absence_stays_omitted() {
    let vat_draft = draft().with_buyer_vat_id("SE556677889901".into()).unwrap();
    let body = VoucherBody::from_draft(&vat_draft, 1, "chart@hash".into()).unwrap();
    let (canonical, hash) = seal_voucher(&Hash::zero(), &body).unwrap();
    assert!(String::from_utf8_lossy(&canonical).contains("\"buyer_vat_id\":\"SE556677889901\""));
    assert_eq!(
        verify_voucher(&canonical, &Hash::zero(), &hash)
            .unwrap()
            .buyer_vat_id(),
        Some("SE556677889901")
    );

    let tampered = String::from_utf8(canonical)
        .unwrap()
        .replace("SE556677889901", "SE556677889902");
    assert!(verify_voucher(tampered.as_bytes(), &Hash::zero(), &hash).is_err());

    let plain = VoucherBody::from_draft(&draft(), 1, "chart@hash".into()).unwrap();
    assert!(!String::from_utf8(canonical_bytes(&plain).unwrap())
        .unwrap()
        .contains("buyer_vat_id"));
}

#[test]
fn strict_read_rejects_a_non_normalized_buyer_vat_id() {
    let body = VoucherBody::from_draft(
        &draft().with_buyer_vat_id("SE556677889901".into()).unwrap(),
        1,
        "chart@hash".into(),
    )
    .unwrap();
    let canonical = String::from_utf8(canonical_bytes(&body).unwrap()).unwrap();
    let malformed = canonical.replace("SE556677889901", "se 556677889901");
    assert!(aggountant_wire::decode_voucher_body(malformed.as_bytes()).is_err());
}
