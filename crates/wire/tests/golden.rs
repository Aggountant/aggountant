//! Versioned JCS golden fixture (`ADR-2 §Decision.5`). The committed
//! `tests/golden/wire-v1/*` files pin the canonical bytes and the chain hash of
//! a fixed voucher body. CI only *verifies* them; regenerating requires the
//! explicit `--features regen-goldens` flag. A mismatch here is exactly the
//! signal ADR-2 wants: a serialization change silently altered a historical
//! hash. This is the M1 "reproducible hash across versions/rebuilds" proof.

use std::fs;
use std::path::PathBuf;

use aggountant_model::{
    AccountId, Currency, Date, Hash, Money, Posting, Side, VatCode, VoucherBody, VoucherDraft,
};
use aggountant_wire::{canonical_bytes, decode_voucher_body, seal_voucher, verify_voucher};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/wire-v1")
}

/// A fully-fixed voucher body — no clocks, no randomness, so its canonical bytes
/// and hash are deterministic across machines, toolchains and rebuilds.
fn fixture_body() -> VoucherBody {
    let d = Date::parse("2026-03-14").unwrap();
    let eur = Currency::new("EUR").unwrap();
    let postings = vec![
        Posting {
            account: AccountId::new("7690").unwrap(),
            amount: Money::parse("124.00", eur.clone()).unwrap(),
            side: Side::Debit,
            vat_code: Some(VatCode::new("purchase-domestic-standard").unwrap()),
        },
        Posting {
            account: AccountId::new("1910").unwrap(),
            amount: Money::parse("124.00", eur).unwrap(),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    let draft = VoucherDraft::new(
        d,
        d,
        "Hosting invoice — Q1".into(),
        postings,
        Some(Hash::from_bytes([0x11; 32])),
    )
    .unwrap();
    VoucherBody::from_draft(&draft, 1, "chart-fi-2026@0000000000000001".into()).unwrap()
}

#[test]
fn golden_voucher_wire_v1_is_reproducible() {
    let body = fixture_body();
    let (canonical, hash) = seal_voucher(&Hash::zero(), &body).unwrap();

    let json_path = golden_dir().join("voucher_genesis.json");
    let sha_path = golden_dir().join("voucher_genesis.sha256");

    if cfg!(feature = "regen-goldens") {
        fs::create_dir_all(golden_dir()).unwrap();
        fs::write(&json_path, &canonical).unwrap();
        fs::write(&sha_path, format!("{}\n", hash.to_hex())).unwrap();
        eprintln!("regenerated goldens in {}", golden_dir().display());
    }

    let want_json = fs::read(&json_path).unwrap_or_else(|e| {
        panic!(
            "missing golden {}: {e} (run with --features regen-goldens)",
            json_path.display()
        )
    });
    let want_hash = fs::read_to_string(&sha_path)
        .unwrap_or_else(|e| panic!("missing golden {}: {e}", sha_path.display()));

    assert_eq!(
        canonical,
        want_json,
        "canonical wire drifted from golden:\n got: {}\nwant: {}",
        String::from_utf8_lossy(&canonical),
        String::from_utf8_lossy(&want_json)
    );
    assert_eq!(
        hash.to_hex(),
        want_hash.trim(),
        "chain hash drifted from golden"
    );
}

#[test]
fn golden_strict_read_path_roundtrips() {
    let body = fixture_body();
    let (canonical, hash) = seal_voucher(&Hash::zero(), &body).unwrap();

    // Strict decode must reconstruct the exact body and pass byte-equality.
    let decoded = decode_voucher_body(&canonical).unwrap();
    assert_eq!(decoded, body);

    // Full chain verification against the genesis prev-hash.
    let verified = verify_voucher(&canonical, &Hash::zero(), &hash).unwrap();
    assert_eq!(verified, body);
}

#[test]
fn duplicate_keys_are_rejected() {
    // serde's derived struct rejects duplicate fields; the byte-equality check is
    // the backstop regardless. Inject a duplicate `number` key into valid bytes.
    let body = fixture_body();
    let canonical = String::from_utf8(canonical_bytes(&body).unwrap()).unwrap();
    let dup = canonical.replacen("\"number\":1,", "\"number\":1,\"number\":1,", 1);
    let err = decode_voucher_body(dup.as_bytes()).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[test]
fn non_canonical_bytes_are_rejected() {
    // Take the canonical bytes and inject insignificant whitespace: still valid
    // JSON, but not byte-equal to the frozen encoding → NON_CANONICAL_WIRE.
    let body = fixture_body();
    let canonical = canonical_bytes(&body).unwrap();
    let mut tampered = Vec::from(b"  " as &[u8]);
    tampered.extend_from_slice(&canonical);
    let err = decode_voucher_body(&tampered).unwrap_err();
    assert!(err.to_string().contains("NON_CANONICAL_WIRE"), "{err}");
}
