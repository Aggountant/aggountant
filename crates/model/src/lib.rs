//! # aggountant-model (F1)
//!
//! The Model layer: validating newtypes and the local voucher invariants. This
//! crate is the floor of the layer stack (`candidate.md §2`) — it depends on no
//! other aggountant crate. Every invalid state is unrepresentable, and `serde`
//! deserialize runs the same validation as the constructors (`candidate.md §4`,
//! `ADR-2`).
//!
//! No `f32`/`f64` anywhere in this crate: money and other exact scalars use
//! `rust_decimal::Decimal`, and they reach the wire only as canonical strings
//! (`ADR-2 §Decision`). Enforced workspace-wide by `clippy.toml` (D1) and by the
//! F16 structural check.

#![forbid(unsafe_code)]

pub mod account;
pub mod error;
pub mod newtype;
pub mod opening_balance;
pub mod period;
pub mod vat_expansion;
pub mod vero;
pub mod voucher;

pub use account::{Account, AccountKind};
pub use error::ModelError;
pub use newtype::{AccountId, Currency, Date, FinnishPersonId, Hash, Iban, Money, VatCode};
pub use opening_balance::{OpeningBalance, OpeningBalanceSource};
pub use period::PeriodId;
pub use vat_expansion::{
    GeneratedVatLeg, GeneratedVatRole, VatExpansion, VatExpansionDeductibility,
    VatExpansionDeductibilityProvenance, VatExpansionScope, VatExpansionTreatment,
    VAT_EXPANSION_VERSION,
};
pub use vero::{BusinessId, FilingContactName, FilingContactPhone};
pub use voucher::{
    Posting, Side, VoucherBody, VoucherDraft, RECORD_TYPE_VOUCHER, SCHEMA_VERSION_VOUCHER,
    WIRE_VERSION,
};

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn eur() -> Currency {
        Currency::new("EUR").unwrap()
    }

    fn posting(acc: &str, amount: &str, side: Side) -> Posting {
        Posting {
            account: AccountId::new(acc).unwrap(),
            amount: Money::parse(amount, eur()).unwrap(),
            side,
            vat_code: None,
        }
    }

    #[test]
    fn balanced_two_posting_voucher_constructs() {
        let d = Date::parse("2026-03-14").unwrap();
        let v = VoucherDraft::new(
            d,
            d,
            "Server hosting".into(),
            vec![
                posting("7690", "100.00", Side::Debit),
                posting("1910", "100.00", Side::Credit),
            ],
            None,
        );
        assert!(v.is_ok(), "{v:?}");
    }

    #[test]
    fn buyer_vat_id_round_trips_from_draft_into_sealed_body() {
        let d = Date::parse("2026-03-14").unwrap();
        let draft = VoucherDraft::new(
            d,
            d,
            "EU sale".into(),
            vec![
                posting("1910", "100.00", Side::Debit),
                posting("3000", "100.00", Side::Credit),
            ],
            None,
        )
        .unwrap()
        .with_buyer_vat_id("SE556677889901".into())
        .unwrap();
        let body = VoucherBody::from_draft(&draft, 1, "chart@hash".into()).unwrap();

        assert_eq!(draft.buyer_vat_id(), Some("SE556677889901"));
        assert_eq!(body.buyer_vat_id(), draft.buyer_vat_id());
        assert_eq!(
            serde_json::from_value::<VoucherBody>(serde_json::to_value(&body).unwrap())
                .unwrap()
                .buyer_vat_id(),
            Some("SE556677889901")
        );
    }

    #[test]
    fn buyer_vat_id_wire_value_must_already_be_normalized() {
        let d = Date::parse("2026-03-14").unwrap();
        let draft = VoucherDraft::new(
            d,
            d,
            "EU sale".into(),
            vec![
                posting("1910", "100.00", Side::Debit),
                posting("3000", "100.00", Side::Credit),
            ],
            None,
        )
        .unwrap();
        assert!(draft.with_buyer_vat_id("se 556677889901".into()).is_err());
    }

    #[test]
    fn unbalanced_voucher_is_rejected() {
        let d = Date::parse("2026-03-14").unwrap();
        let err = VoucherDraft::new(
            d,
            d,
            "bad".into(),
            vec![
                posting("7690", "100.00", Side::Debit),
                posting("1910", "90.00", Side::Credit),
            ],
            None,
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "UNBALANCED_VOUCHER");
    }

    #[test]
    fn single_posting_is_rejected() {
        let d = Date::parse("2026-03-14").unwrap();
        let err = VoucherDraft::new(
            d,
            d,
            "x".into(),
            vec![posting("7690", "1.00", Side::Debit)],
            None,
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "INSUFFICIENT_POSTINGS");
    }

    #[test]
    fn mixed_currency_is_rejected() {
        let d = Date::parse("2026-03-14").unwrap();
        let usd = Currency::new("USD").unwrap();
        let err = VoucherDraft::new(
            d,
            d,
            "x".into(),
            vec![
                posting("7690", "100.00", Side::Debit),
                Posting {
                    account: AccountId::new("1910").unwrap(),
                    amount: Money::parse("100.00", usd).unwrap(),
                    side: Side::Credit,
                    vat_code: None,
                },
            ],
            None,
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "MIXED_CURRENCY");
    }

    #[test]
    fn deserialize_rejects_unbalanced_voucher() {
        // The invariant is not bypassable via serde (candidate.md §4).
        let json = r#"{
            "document_date":"2026-03-14","posting_date":"2026-03-14",
            "description":"bad",
            "postings":[
                {"account":"7690","amount":{"amount":"100.00","currency":"EUR"},"side":"debit"},
                {"account":"1910","amount":{"amount":"90.00","currency":"EUR"},"side":"credit"}
            ]}"#;
        let err = serde_json::from_str::<VoucherDraft>(json).unwrap_err();
        assert!(err.to_string().contains("UNBALANCED_VOUCHER"), "{err}");
    }

    #[test]
    fn money_deserialize_rejects_non_canonical_decimal() {
        // leading zero
        assert!(Money::parse("01.5", eur()).is_err());
        // plus sign
        assert!(Money::parse("+1.5", eur()).is_err());
        // exponent
        assert!(Money::parse("1e3", eur()).is_err());
        // negative zero
        assert!(Money::parse("-0.00", eur()).is_err());
        // scale preserved / canonical
        let m = Money::parse("1234.50", eur()).unwrap();
        assert_eq!(m.amount_str(), "1234.50");
    }

    #[test]
    fn decimal_scale_is_preserved_by_display() {
        // The whole ADR-2 read path depends on this rust_decimal property.
        assert_eq!(Decimal::from_str("1234.50").unwrap().to_string(), "1234.50");
        assert_eq!(Decimal::from_str("0.001").unwrap().to_string(), "0.001");
    }

    #[test]
    fn date_rejects_non_canonical() {
        assert!(Date::parse("2026-1-2").is_err());
        assert!(Date::parse("2026/01/02").is_err());
        assert!(Date::parse("2026-01-02").is_ok());
    }

    #[test]
    fn empty_description_has_its_own_error_code() {
        let d = Date::parse("2026-03-14").unwrap();
        let err = VoucherDraft::new(
            d,
            d,
            "   ".into(),
            vec![
                posting("7690", "1.00", Side::Debit),
                posting("1910", "1.00", Side::Credit),
            ],
            None,
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "EMPTY_DESCRIPTION");
    }

    #[test]
    fn date_rejects_out_of_range_years() {
        assert!(Date::parse("0001-01-01").is_err());
        assert!(Date::from_ymd(999, 1, 1).is_err());
        assert!(Date::from_ymd(10000, 1, 1).is_err());
        assert!(Date::parse("2026-03-14").is_ok());
    }

    fn voucher_body_json(number: &str, schema_version: &str, description: &str) -> String {
        format!(
            r#"{{"wire_version":1,"schema_version":{schema_version},"record_type":"voucher",
                 "number":{number},"document_date":"2026-03-14","posting_date":"2026-03-14",
                 "description":{description},
                 "postings":[
                   {{"account":"7690","amount":{{"amount":"1.00","currency":"EUR"}},"side":"debit"}},
                   {{"account":"1910","amount":{{"amount":"1.00","currency":"EUR"}},"side":"credit"}}
                 ],"chart_version":"c1"}}"#
        )
    }

    #[test]
    fn voucher_body_read_path_rejects_invalid_records() {
        // A well-formed record deserializes.
        assert!(
            serde_json::from_str::<VoucherBody>(&voucher_body_json("1", "1", "\"ok\"")).is_ok()
        );
        // number 0, bad schema_version, and empty description are all rejected on
        // the strict read path — the write path could never mint them.
        assert!(
            serde_json::from_str::<VoucherBody>(&voucher_body_json("0", "1", "\"ok\"")).is_err()
        );
        assert!(
            serde_json::from_str::<VoucherBody>(&voucher_body_json("1", "2", "\"ok\"")).is_err()
        );
        assert!(
            serde_json::from_str::<VoucherBody>(&voucher_body_json("1", "1", "\"  \"")).is_err()
        );
    }

    #[test]
    fn hash_hex_roundtrips_and_rejects_uppercase() {
        let h = Hash::from_bytes([0xab; 32]);
        assert_eq!(h.to_hex().len(), 64);
        assert_eq!(Hash::parse_hex(&h.to_hex()).unwrap(), h);
        assert!(Hash::parse_hex(&"AB".repeat(32)).is_err());
    }

    #[test]
    fn external_cutover_opening_balance_binds_mapping_and_live_period() {
        let opening = OpeningBalance::new(
            2026,
            vec![
                posting("1910", "100.00", Side::Debit),
                posting("2000", "100.00", Side::Credit),
            ],
            OpeningBalanceSource::ExternalCutover {
                prior_year: 2025,
                source_head_hash: Hash::from_bytes([1; 32]),
                source_chart_version: "briox-2025@source".into(),
                target_chart_version: "fi-general-2026@target".into(),
                account_mapping_hash: Hash::from_bytes([2; 32]),
                live_period: PeriodId::new(2026, 3).unwrap(),
            },
        )
        .unwrap();
        let bytes = serde_json::to_vec(&opening).unwrap();
        assert_eq!(
            serde_json::from_slice::<OpeningBalance>(&bytes).unwrap(),
            opening
        );
    }

    #[test]
    fn external_cutover_live_period_must_be_in_the_opening_year() {
        let err = OpeningBalance::new(
            2026,
            vec![
                posting("1910", "1.00", Side::Debit),
                posting("2000", "1.00", Side::Credit),
            ],
            OpeningBalanceSource::ExternalCutover {
                prior_year: 2025,
                source_head_hash: Hash::from_bytes([1; 32]),
                source_chart_version: "old".into(),
                target_chart_version: "new".into(),
                account_mapping_hash: Hash::from_bytes([2; 32]),
                live_period: PeriodId::new(2025, 4).unwrap(),
            },
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "INVALID_OPENING_BALANCE");
    }
}
