use aggountant_model::*;
use aggountant_wire::{canonical_bytes, seal_voucher, verify_voucher};

fn body_with_provenance() -> VoucherBody {
    let eur = Currency::new("EUR").unwrap();
    let d = Date::parse("2026-02-15").unwrap();
    let posting = |account, amount, side| Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(amount, eur.clone()).unwrap(),
        side,
        vat_code: None,
    };
    let expansion = VatExpansion::new(
        1,
        VatExpansionTreatment::ReverseCharge {
            rate_class: "domestic-standard".into(),
            scope: VatExpansionScope::EuServices,
        },
        Money::parse("100.00", eur.clone()).unwrap(),
        AccountId::new("4020").unwrap(),
        AccountId::new("1910").unwrap(),
        Some("25.5".into()),
        "fi.core.2026-01@stub".into(),
        Hash::from_bytes([8; 32]),
        vec![
            GeneratedVatLeg {
                posting_index: 2,
                role: GeneratedVatRole::Input,
            },
            GeneratedVatLeg {
                posting_index: 3,
                role: GeneratedVatRole::Output,
            },
        ],
    )
    .unwrap();
    VoucherBody::from_draft(
        &VoucherDraft::new(
            d,
            d,
            "VAT RC".into(),
            vec![
                posting("4020", "100.00", Side::Debit),
                posting("1910", "100.00", Side::Credit),
                posting("1798", "25.50", Side::Debit),
                posting("2992", "25.50", Side::Credit),
            ],
            None,
        )
        .unwrap()
        .with_vat_expansion(expansion)
        .unwrap(),
        1,
        "chart@hash".into(),
    )
    .unwrap()
}

#[test]
fn provenance_is_hash_bound_and_absence_stays_omitted() {
    let body = body_with_provenance();
    let (bytes, hash) = seal_voucher(&Hash::zero(), &body).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("vat_expansion"));
    assert_eq!(
        verify_voucher(&bytes, &Hash::zero(), &hash)
            .unwrap()
            .vat_expansion(),
        body.vat_expansion()
    );
    let tampered = String::from_utf8(bytes.clone())
        .unwrap()
        .replacen("\"25.5\"", "\"24.0\"", 1);
    assert_ne!(
        aggountant_wire::chain_hash(&Hash::zero(), tampered.as_bytes()),
        hash
    );
    assert!(
        verify_voucher(tampered.as_bytes(), &Hash::zero(), &hash).is_err(),
        "a changed sealed preimage must not verify with its original hash"
    );
    let d = Date::parse("2026-02-15").unwrap();
    let eur = Currency::new("EUR").unwrap();
    let plain = VoucherDraft::new(
        d,
        d,
        "plain".into(),
        vec![
            Posting {
                account: AccountId::new("4020").unwrap(),
                amount: Money::parse("1.00", eur.clone()).unwrap(),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: Money::parse("1.00", eur).unwrap(),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    assert!(!String::from_utf8(canonical_bytes(&plain).unwrap())
        .unwrap()
        .contains("vat_expansion"));
}
