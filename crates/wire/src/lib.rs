//! # aggountant-wire (F3)
//!
//! Canonical wire format for the fiscal hash chain. Implements the aggountant
//! JCS profile from **ADR-2**: RFC 8785 canonicalization rules (lexicographic
//! object-key ordering by UTF-16 code unit, minimal string escaping, no
//! whitespace) applied to a strict profile in which every domain-meaningful
//! scalar is already a **string** (enforced by the Model layer). JSON numbers
//! appear only for a small enumerated set of non-negative integers (versions,
//! voucher number).
//!
//! We **own the encoder** (`ADR-2 §Decision.3`): the critical hashing path has
//! no runtime dependency on an external JCS crate. Decimals reach the wire only
//! through `aggountant-model`'s canonical string codecs; `rust_decimal` never
//! appears here as a JSON number.
//!
//! The chain hash is `SHA256(prev_hash_bytes || canonical_wire)` (`ADR-1
//! §Decision`, the normative allocation sketch). `prev_hash` is chain glue, not
//! a field of the hashed body.

#![forbid(unsafe_code)]

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use aggountant_model::{Hash, VoucherBody};

#[derive(Debug, Error)]
pub enum WireError {
    #[error("WIRE_SERIALIZE: {0}")]
    Serialize(String),
    #[error("WIRE_FLOAT_FORBIDDEN: a float reached the canonical wire path ({0})")]
    FloatForbidden(String),
    #[error("NON_CANONICAL_WIRE: {0}")]
    NonCanonical(String),
    #[error("WIRE_DECODE: {0}")]
    Decode(String),
}

/// Canonicalize any `Serialize` value to JCS bytes (`ADR-2`).
///
/// Because the aggountant profile keeps all decimals as strings, the only numbers
/// that can appear are integers; a float is a profile violation and is rejected
/// loudly rather than silently canonicalized via ECMAScript number rules.
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, WireError> {
    let json = serde_json::to_value(value).map_err(|e| WireError::Serialize(e.to_string()))?;
    let mut out = String::new();
    write_canonical(&json, &mut out)?;
    Ok(out.into_bytes())
}

fn write_canonical(value: &Value, out: &mut String) -> Result<(), WireError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                out.push_str(&u.to_string());
            } else if let Some(i) = n.as_i64() {
                out.push_str(&i.to_string());
            } else {
                // A non-integer JSON number means an f64 leaked into a hashed
                // struct. ADR-2 §Consequences names this the key hazard.
                return Err(WireError::FloatForbidden(n.to_string()));
            }
        }
        Value::String(s) => escape_json_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            // Sort member names lexicographically by UTF-16 code units (RFC 8785).
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| utf16_cmp(a, b));
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                escape_json_string(key, out);
                out.push(':');
                write_canonical(&map[*key], out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// Compare two strings by their UTF-16 code-unit sequences (RFC 8785 §3.2.3).
fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// JSON string production per RFC 8785 §3.2.2.2 (minimal escaping; UTF-8 output;
/// forward slash is *not* escaped).
fn escape_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{0009}' => out.push_str("\\t"),
            '\u{000A}' => out.push_str("\\n"),
            '\u{000C}' => out.push_str("\\f"),
            '\u{000D}' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The chain hash over a canonical body: `SHA256(prev_hash || canonical_wire)`.
pub fn chain_hash(prev_hash: &Hash, canonical_wire: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(canonical_wire);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Hash::from_bytes(out)
}

/// Canonicalize a voucher body and compute its chain hash in one step.
pub fn seal_voucher(prev_hash: &Hash, body: &VoucherBody) -> Result<(Vec<u8>, Hash), WireError> {
    let canonical = canonical_bytes(body)?;
    let hash = chain_hash(prev_hash, &canonical);
    Ok((canonical, hash))
}

/// Strict fiscal read path (`ADR-2 §Decision.4`): parse stored bytes into a
/// validated `VoucherBody` (rejecting duplicate keys, unknown fields, trailing
/// bytes, non-canonical scalar aliases), then re-encode with the frozen encoder
/// and require **byte-equality** with the stored bytes, else `NON_CANONICAL_WIRE`.
///
/// Structural strictness comes from two independent layers. (1) The body's wire
/// struct derives `Deserialize` with `deny_unknown_fields`; serde's derived impl
/// rejects duplicate fields ("duplicate field `x`") and `serde_json::from_slice`
/// rejects trailing content and non-integer numbers for integer fields. (2) The
/// byte-equality re-encode is the ultimate backstop: any input that decodes but
/// is not the frozen canonical form (extra whitespace, reordered keys, scalar
/// aliases) fails here as `NON_CANONICAL_WIRE`, independent of serde's leniency.
pub fn decode_voucher_body(stored: &[u8]) -> Result<VoucherBody, WireError> {
    let body: VoucherBody =
        serde_json::from_slice(stored).map_err(|e| WireError::Decode(e.to_string()))?;
    let reencoded = canonical_bytes(&body)?;
    if reencoded != stored {
        return Err(WireError::NonCanonical(format!(
            "stored bytes are not canonical for wire_version {}",
            body.wire_version()
        )));
    }
    Ok(body)
}

/// Verify a voucher's stored bytes against `prev_hash` and the expected chain
/// hash: strict-decode, re-encode byte-equal, then recompute the chain hash.
pub fn verify_voucher(
    stored: &[u8],
    prev_hash: &Hash,
    expected_hash: &Hash,
) -> Result<VoucherBody, WireError> {
    let body = decode_voucher_body(stored)?;
    let recomputed = chain_hash(prev_hash, stored);
    if &recomputed != expected_hash {
        return Err(WireError::NonCanonical(format!(
            "HASH_CHAIN_BROKEN at voucher {}: recomputed {} != stored {}",
            body.number(),
            recomputed,
            expected_hash
        )));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_keys_are_sorted_and_whitespace_stripped() {
        let v = json!({"b": 1, "a": 2, "c": {"z": 1, "y": 2}});
        let bytes = canonical_bytes(&v).unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"a":2,"b":1,"c":{"y":2,"z":1}}"#
        );
    }

    #[test]
    fn strings_are_jcs_escaped() {
        let mut out = String::new();
        escape_json_string("a\"b\\c\n\t/ä", &mut out);
        // forward slash NOT escaped; non-ASCII passes through as UTF-8.
        assert_eq!(out, "\"a\\\"b\\\\c\\n\\t/ä\"");
    }

    #[test]
    fn control_char_uses_lowercase_u_escape() {
        let mut out = String::new();
        escape_json_string("\u{0001}", &mut out);
        assert_eq!(out, "\"\\u0001\"");
    }

    #[test]
    fn floats_are_rejected() {
        let v = json!({"x": 1.5});
        assert!(matches!(
            canonical_bytes(&v),
            Err(WireError::FloatForbidden(_))
        ));
    }

    #[test]
    fn utf16_key_ordering() {
        // RFC 8785 sorts by UTF-16 code units. Verify the comparator is stable
        // for the ASCII keys we actually use.
        assert_eq!(utf16_cmp("amount", "currency"), std::cmp::Ordering::Less);
        assert_eq!(utf16_cmp("z", "aa"), std::cmp::Ordering::Greater);
    }
}
