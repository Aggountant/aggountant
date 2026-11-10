//! Validating newtypes (F1). Every constructor validates; `serde` deserialize
//! runs the *same* validation (it never bypasses it — `candidate.md §4`,
//! `ADR-2 §Decision.4`). Invalid states are unrepresentable.
//!
//! Wire-visible scalars serialize as their canonical **string** form
//! (`ADR-2`): decimals as strings with scale preserved, dates as `YYYY-MM-DD`,
//! hashes as fixed-length lowercase hex. `rust_decimal::Decimal` never appears
//! as a JSON number.

use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ModelError;

/// Validate the canonical decimal grammar (`ADR-2 §Decision.2`):
/// optional single leading `-` (never on zero), integer part is `0` or has no
/// leading zero, optional fractional part with >= 1 digit, no `+`, no exponent.
/// Scale (trailing zeros) is **preserved** — `"1234.50"` is not `"1234.5"`.
pub(crate) fn parse_canonical_decimal(s: &str) -> Result<Decimal, ModelError> {
    let bad = |why: &str| ModelError::InvalidMoney(format!("{why}: {s:?}"));
    if s.is_empty() {
        return Err(bad("empty decimal"));
    }
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int_part, frac_part) = match digits.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (digits, None),
    };
    if int_part.is_empty() || !int_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("non-digit or empty integer part"));
    }
    if int_part.len() > 1 && int_part.starts_with('0') {
        return Err(bad("leading zero in integer part"));
    }
    if let Some(f) = frac_part {
        if f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad("empty or non-digit fractional part"));
        }
    }
    let value = Decimal::from_str(s).map_err(|e| bad(&format!("unparseable ({e})")))?;
    if neg && value.is_zero() {
        return Err(bad("negative zero is not canonical"));
    }
    // The canonical string must round-trip through Display so that the strict
    // read path's re-encode is byte-identical (ADR-2 §Decision.4). rust_decimal
    // preserves scale in Display; this assertion guards that assumption.
    if value.to_string() != s {
        return Err(bad("not in canonical Display form (scale/format drift)"));
    }
    Ok(value)
}

// ----------------------------------------------------------------------------
// Currency
// ----------------------------------------------------------------------------

/// ISO-4217-style currency code (3 uppercase ASCII letters). Part of `Money`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Currency(String);

impl Currency {
    pub fn new(code: impl Into<String>) -> Result<Self, ModelError> {
        let code = code.into();
        if code.len() == 3 && code.bytes().all(|b| b.is_ascii_uppercase()) {
            Ok(Currency(code))
        } else {
            Err(ModelError::InvalidCurrency(format!(
                "expected 3 uppercase ASCII letters, got {code:?}"
            )))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Currency {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Currency::new(raw).map_err(D::Error::custom)
    }
}

// ----------------------------------------------------------------------------
// Money
// ----------------------------------------------------------------------------

/// `Money(Decimal, Currency)` — never `f64`. Currency is part of the type
/// (`candidate.md §3`). Scale is preserved; equality is therefore
/// **scale-sensitive** and matches the canonical wire bytes (`ADR-2`): callers
/// that want `1234.5 == 1234.50` must normalize scale at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        Money { amount, currency }
    }
    /// Parse from the canonical decimal string + currency (validating).
    pub fn parse(amount: &str, currency: Currency) -> Result<Self, ModelError> {
        Ok(Money {
            amount: parse_canonical_decimal(amount)?,
            currency,
        })
    }
    pub fn amount(&self) -> Decimal {
        self.amount
    }
    pub fn currency(&self) -> &Currency {
        &self.currency
    }
    /// Canonical decimal string (scale preserved).
    pub fn amount_str(&self) -> String {
        self.amount.to_string()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MoneyWire {
    amount: String,
    currency: Currency,
}

impl Serialize for Money {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        MoneyWire {
            amount: self.amount.to_string(),
            currency: self.currency.clone(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = MoneyWire::deserialize(d)?;
        let amount = parse_canonical_decimal(&w.amount).map_err(D::Error::custom)?;
        Ok(Money {
            amount,
            currency: w.currency,
        })
    }
}

// ----------------------------------------------------------------------------
// AccountId
// ----------------------------------------------------------------------------

/// Chart-of-accounts identifier. Finnish charts are numeric strings; we require
/// 1..=10 ASCII digits (no whitespace, no silent trimming — `AGENTS-AI-FIRST-CLI §1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AccountId(String);

impl AccountId {
    pub fn new(id: impl Into<String>) -> Result<Self, ModelError> {
        let id = id.into();
        if (1..=10).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit()) {
            Ok(AccountId(id))
        } else {
            Err(ModelError::InvalidAccountId(format!(
                "expected 1..=10 ASCII digits, got {id:?}"
            )))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for AccountId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AccountId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        AccountId::new(String::deserialize(d)?).map_err(D::Error::custom)
    }
}

// ----------------------------------------------------------------------------
// VatCode
// ----------------------------------------------------------------------------

/// Semantic VAT code (e.g. `purchase-domestic-standard`) — never a percentage
/// (`candidate.md §3`). The rate resolves from code + document date + policy;
/// S1 does no rate resolution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VatCode(String);

impl VatCode {
    pub fn new(code: impl Into<String>) -> Result<Self, ModelError> {
        let code = code.into();
        let ok = !code.is_empty()
            && code.len() <= 64
            && code
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !code.starts_with('-')
            && !code.ends_with('-');
        if ok {
            Ok(VatCode(code))
        } else {
            Err(ModelError::InvalidVatCode(format!(
                "expected lowercase kebab-case [a-z0-9-], got {code:?}"
            )))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VatCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for VatCode {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for VatCode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        VatCode::new(String::deserialize(d)?).map_err(D::Error::custom)
    }
}

// ----------------------------------------------------------------------------
// Date
// ----------------------------------------------------------------------------

/// ISO-8601 calendar date `YYYY-MM-DD`. Deserialize is strict: it rejects any
/// non-canonical alias (unpadded fields, slashes, trailing bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date(chrono::NaiveDate);

/// Fiscal dates are four-digit calendar years; reject anything outside this so
/// `%Y` never emits a signed/extended year that would round-trip oddly.
const MIN_YEAR: i32 = 1000;
const MAX_YEAR: i32 = 9999;

impl Date {
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Result<Self, ModelError> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
            return Err(ModelError::InvalidDate(format!(
                "year {year} outside {MIN_YEAR}..={MAX_YEAR}"
            )));
        }
        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .map(Date)
            .ok_or_else(|| ModelError::InvalidDate(format!("{year:04}-{month:02}-{day:02}")))
    }
    pub fn parse(s: &str) -> Result<Self, ModelError> {
        let d = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|e| ModelError::InvalidDate(format!("{s:?} ({e})")))?;
        // Reject non-canonical aliases (e.g. "2026-1-2"): the reformat must match.
        if d.format("%Y-%m-%d").to_string() != s {
            return Err(ModelError::InvalidDate(format!(
                "{s:?} is not canonical YYYY-MM-DD"
            )));
        }
        if !(MIN_YEAR..=MAX_YEAR).contains(&chrono::Datelike::year(&d)) {
            return Err(ModelError::InvalidDate(format!(
                "{s:?} year outside {MIN_YEAR}..={MAX_YEAR}"
            )));
        }
        Ok(Date(d))
    }
    pub fn inner(&self) -> chrono::NaiveDate {
        self.0
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Date::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

// ----------------------------------------------------------------------------
// Hash
// ----------------------------------------------------------------------------

/// A 32-byte hash (SHA-256), serialized as 64-char lowercase hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Hash([u8; 32]);

impl Hash {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }
    /// The genesis / zero hash used as the previous-hash of the first voucher.
    pub const fn zero() -> Self {
        Hash([0u8; 32])
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    pub fn parse_hex(s: &str) -> Result<Self, ModelError> {
        if s.len() != 64
            || !s
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(ModelError::InvalidHash(format!(
                "expected 64 lowercase hex chars, got {s:?}"
            )));
        }
        let mut out = [0u8; 32];
        hex::decode_to_slice(s, &mut out)
            .map_err(|e| ModelError::InvalidHash(format!("{s:?} ({e})")))?;
        Ok(Hash(out))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Hash::parse_hex(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

// ----------------------------------------------------------------------------
// PII newtypes (S5) — the *forbidden* payload for the external-disclosure boundary
// ----------------------------------------------------------------------------
//
// `Iban` and `FinnishPersonId` are validating newtypes for the two sensitive
// scalars an external `consult-llm` disclosure must NEVER carry
// (`redaction-strategy-options.md` §4.1). They exist so the disclosure projection
// (`aggountant_domain::ConsultQuestion`) can be *typed* to exclude them: a value still
// typed as `Iban`/`FinnishPersonId` cannot be placed into a disclosure fact — that is
// a compile error, not a review miss. (A caller who deliberately `.expose()`s and
// stringifies has explicitly opted out; the barrier stops the *accidental* leak.)
// They are deliberately opaque — no `Display`, no `Serialize`, and a **redacting
// `Debug`** — so the raw value never slips into a git-native journal / disclosure body
// / log through a stray format string, and validation errors never echo the raw value.

/// A Finnish IBAN (or any IBAN-shaped account number): 2 uppercase country
/// letters, 2 check digits, then 1..=30 uppercase-alphanumeric BBAN characters,
/// validated by the ISO 7064 mod-97-10 checksum. PII — never disclosed externally.
#[derive(Clone, PartialEq, Eq)]
pub struct Iban(String);

impl fmt::Debug for Iban {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redact: never print the account number, even through `{:?}` / panic / trace.
        f.write_str("Iban([REDACTED])")
    }
}

impl Iban {
    pub fn new(raw: impl Into<String>) -> Result<Self, ModelError> {
        let raw = raw.into();
        // Errors must not echo the raw value (it is PII on the valid path and
        // near-PII on the invalid one): report only the reason.
        let bad = |why: &str| ModelError::InvalidIban(why.to_string());
        // No silent trimming/space-stripping — the caller passes the canonical form.
        if !(6..=34).contains(&raw.len()) {
            return Err(bad("IBAN must be 6..=34 characters"));
        }
        let b = raw.as_bytes();
        if !b[0].is_ascii_uppercase() || !b[1].is_ascii_uppercase() {
            return Err(bad("must start with a 2-letter country code"));
        }
        if !b[2].is_ascii_digit() || !b[3].is_ascii_digit() {
            return Err(bad("characters 3..4 must be check digits"));
        }
        if !raw
            .bytes()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Err(bad("only A-Z and 0-9 are allowed"));
        }
        // mod-97-10: move the first four chars to the end, map letters to 10..35,
        // and require the resulting integer ≡ 1 (mod 97).
        let rearranged = format!("{}{}", &raw[4..], &raw[..4]);
        let mut remainder: u32 = 0;
        for c in rearranged.bytes() {
            let value = if c.is_ascii_digit() {
                (c - b'0') as u32
            } else {
                (c - b'A') as u32 + 10
            };
            // Fold two decimal digits at a time for letters (value up to 35).
            remainder = if value >= 10 {
                (remainder * 100 + value) % 97
            } else {
                (remainder * 10 + value) % 97
            };
        }
        if remainder != 1 {
            return Err(bad("mod-97 checksum failed"));
        }
        Ok(Iban(raw))
    }

    /// Access the raw digits — for the *trusted-authority* path only (a payment
    /// order to the bank), never for an external disclosure.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// The 31-character residue table for the HETU check character (`ddmmyyzzz % 31`).
const HETU_CHECK: &[u8; 31] = b"0123456789ABCDEFHJKLMNPRSTUVWXY";

/// A Finnish personal identity code (HETU), `ddmmyyCzzzQ`: a real calendar birth
/// date, a century marker, a 3-digit individual number, and the mod-31 check
/// character. PII — never disclosed externally. (Mirrors `payroll::validate_hetu`,
/// but as a first-class newtype so it can be structurally excluded from a
/// disclosure.)
#[derive(Clone, PartialEq, Eq)]
pub struct FinnishPersonId(String);

impl fmt::Debug for FinnishPersonId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FinnishPersonId([REDACTED])")
    }
}

impl FinnishPersonId {
    pub fn new(raw: impl Into<String>) -> Result<Self, ModelError> {
        let raw = raw.into();
        // Do not echo the raw code in errors (near-PII); report only the reason.
        let bad = |why: &str| ModelError::InvalidPersonId(why.to_string());
        let b = raw.as_bytes();
        if b.len() != 11 {
            return Err(bad("HETU must be 11 characters (ddmmyyCzzzQ)"));
        }
        if !b[0..6].iter().all(u8::is_ascii_digit) {
            return Err(bad("the ddmmyy part must be 6 digits"));
        }
        let century = match b[6] {
            b'+' => 1800,
            b'-' | b'Y' | b'X' | b'W' | b'V' | b'U' => 1900,
            b'A' | b'B' | b'C' | b'D' | b'E' | b'F' => 2000,
            _ => return Err(bad("invalid century marker")),
        };
        if !b[7..10].iter().all(u8::is_ascii_digit) {
            return Err(bad("the individual number must be 3 digits"));
        }
        let day: u32 = raw[0..2].parse().map_err(|_| bad("bad day"))?;
        let month: u32 = raw[2..4].parse().map_err(|_| bad("bad month"))?;
        let yy: i32 = raw[4..6].parse().map_err(|_| bad("bad year"))?;
        if chrono::NaiveDate::from_ymd_opt(century + yy, month, day).is_none() {
            return Err(bad("impossible birth date"));
        }
        let ddmmyyzzz: u64 = format!("{}{}", &raw[0..6], &raw[7..10])
            .parse()
            .map_err(|_| bad("checksum preimage not numeric"))?;
        if b[10] != HETU_CHECK[(ddmmyyzzz % 31) as usize] {
            return Err(bad("checksum character mismatch"));
        }
        Ok(FinnishPersonId(raw))
    }

    /// Access the raw code — for the *trusted-authority* path only (a Tulorekisteri
    /// report), never for an external disclosure.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod pii_tests {
    use super::*;

    #[test]
    fn iban_accepts_valid_and_rejects_bad_checksum() {
        assert!(Iban::new("FI2112345600000785").is_ok());
        assert!(Iban::new("FI5542345670000081").is_ok());
        assert!(Iban::new("GB82WEST12345698765432").is_ok());
        // A one-digit corruption breaks the mod-97 check.
        assert_eq!(
            Iban::new("FI2112345600000786").unwrap_err().error_code(),
            "INVALID_IBAN"
        );
        // Structural rejections: lowercase, spaces, missing check digits.
        assert!(Iban::new("fi2112345600000785").is_err());
        assert!(Iban::new("FI 21 1234 5600 0007 85").is_err());
        assert!(Iban::new("FIAB12345600000785").is_err());
    }

    #[test]
    fn finnish_person_id_accepts_valid_and_rejects_bad() {
        assert!(FinnishPersonId::new("131052-308T").is_ok());
        assert!(FinnishPersonId::new("010101A123N").is_ok());
        // Bad checksum, impossible date, wrong length, bad century marker.
        assert_eq!(
            FinnishPersonId::new("131052-308U")
                .unwrap_err()
                .error_code(),
            "INVALID_PERSON_ID"
        );
        assert!(FinnishPersonId::new("310252-999X").is_err());
        assert!(FinnishPersonId::new("13105-308T").is_err());
        assert!(FinnishPersonId::new("131052Q308T").is_err());
    }

    #[test]
    fn pii_newtypes_have_a_redacting_debug_and_do_not_echo_raw() {
        // Debug must NOT reveal the raw value (guards `{:?}` / panic / trace paths).
        let iban = Iban::new("FI2112345600000785").unwrap();
        let dbg = format!("{iban:?}");
        assert_eq!(dbg, "Iban([REDACTED])");
        assert!(!dbg.contains("FI2112345600000785"));

        let hetu = FinnishPersonId::new("131052-308T").unwrap();
        let dbg = format!("{hetu:?}");
        assert_eq!(dbg, "FinnishPersonId([REDACTED])");
        assert!(!dbg.contains("131052"));

        // Validation errors must not echo the raw (near-)PII either.
        let err = Iban::new("FI2112345600000786").unwrap_err();
        assert!(!err.to_string().contains("FI2112345600000786"), "{err}");
        let err = FinnishPersonId::new("131052-308U").unwrap_err();
        assert!(!err.to_string().contains("131052"), "{err}");

        // `expose` is the only readout, and it is named to flag intent at the call site.
        assert_eq!(iban.expose().len(), 18);
    }
}
