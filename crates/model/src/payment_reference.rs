//! A creditor payment reference (viitenumero) as a Finnish bank payment carries
//! it: either the domestic reference or the international RF creditor
//! reference. The Tax Administration allocates a payment to a tax only by its
//! reference ("Et voi maksaa veroa pelkällä tilinumerolla, vaan tarvitset aina
//! myös viitenumeron", vero.fi, *Mistä löydän tilinumerot ja viitenumerot?*,
//! read 2026-11-27), so a mistyped reference sends money to the wrong debt or
//! back to the payer. Both forms carry a check digit; this type refuses a value
//! whose check digit does not hold.
//!
//! - Domestic: 4 to 20 digits, the last a check digit computed with the weights
//!   7, 3, 1 from the right over the other digits, subtracted from the next full
//!   ten (10 gives 0). Finanssiala, *Kotimaisen viitenumeron rakenne*
//!   (1.11.2009), <https://www.finanssiala.fi/wp-content/uploads/2021/03/kotimaisen_viitteen_rakenneohje.pdf>.
//! - RF (ISO 11649): `RF`, two check digits, then 1 to 21 letters or digits;
//!   the check digits are the ISO 7064 mod 97-10 used by the IBAN. Finanssiala,
//!   *Kansainvälisen viitteen käyttöönotto suomalaisessa yrityksessä*
//!   (November 2022), <https://www.finanssiala.fi/wp-content/uploads/2022/12/kansainvalisen-viitteen-kayttoonotto-suomalaisessa-yrityksessa.pdf>.
//!
//! The canonical form has no spaces and, for the domestic form, no leading
//! zeros ("Etunollia ei tulosteta"): the printed grouping (five digits, or four
//! characters for RF) is presentation, and accepting either silently would let
//! two spellings of one reference compare unequal.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ModelError;

/// A validated payment reference in its canonical, unspaced form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentReference(String);

impl PaymentReference {
    pub fn new(raw: impl Into<String>) -> Result<Self, ModelError> {
        let raw = raw.into();
        let bad = |why: &str| ModelError::InvalidPaymentReference(why.to_string());
        if raw.chars().any(char::is_whitespace) {
            return Err(bad(
                "give the reference without spaces (the printed grouping is not part of it)",
            ));
        }
        if raw.starts_with("RF") || raw.starts_with("rf") {
            validate_rf(&raw).map_err(bad)?;
        } else {
            validate_domestic(&raw).map_err(bad)?;
        }
        Ok(PaymentReference(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The domestic check digit of `base` (the digits before it).
fn domestic_check_digit(base: &[u8]) -> u8 {
    const WEIGHTS: [u32; 3] = [7, 3, 1];
    let sum: u32 = base
        .iter()
        .rev()
        .enumerate()
        .map(|(i, digit)| u32::from(digit - b'0') * WEIGHTS[i % 3])
        .sum();
    ((10 - sum % 10) % 10) as u8
}

fn validate_domestic(raw: &str) -> Result<(), &'static str> {
    let bytes = raw.as_bytes();
    if !bytes.iter().all(u8::is_ascii_digit) {
        return Err("a domestic reference has only digits; an RF reference starts with RF");
    }
    if !(4..=20).contains(&bytes.len()) {
        return Err("a domestic reference has 4 to 20 digits, the check digit included");
    }
    // "Etunollia ei tulosteta": a reference has no leading zeros, so one
    // reference has one spelling and two stored references compare by text.
    if bytes[0] == b'0' {
        return Err("a domestic reference has no leading zeros");
    }
    let (base, check) = bytes.split_at(bytes.len() - 1);
    if domestic_check_digit(base) != check[0] - b'0' {
        return Err("the check digit of the domestic reference does not match (7-3-1 weights)");
    }
    Ok(())
}

fn validate_rf(raw: &str) -> Result<(), &'static str> {
    let bytes = raw.as_bytes();
    if !raw.starts_with("RF") {
        return Err("an RF reference starts with uppercase RF");
    }
    if !(5..=25).contains(&bytes.len()) {
        return Err("an RF reference is RF, two check digits and 1 to 21 characters");
    }
    if !bytes[2].is_ascii_digit() || !bytes[3].is_ascii_digit() {
        return Err("characters 3 and 4 of an RF reference are check digits");
    }
    if !bytes[4..]
        .iter()
        .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
    {
        return Err("an RF reference has only A-Z and 0-9 after its check digits");
    }
    // ISO 7064 mod 97-10: move the first four characters to the end, letters
    // become 10..35, and the number is 1 modulo 97.
    let mut remainder: u32 = 0;
    for c in bytes[4..].iter().chain(&bytes[..4]) {
        let value = if c.is_ascii_digit() {
            u32::from(c - b'0')
        } else {
            u32::from(c - b'A') + 10
        };
        remainder = if value >= 10 {
            (remainder * 100 + value) % 97
        } else {
            (remainder * 10 + value) % 97
        };
    }
    if remainder != 1 {
        return Err("the check digits of the RF reference do not match (mod 97)");
    }
    Ok(())
}

impl fmt::Display for PaymentReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for PaymentReference {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PaymentReference {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        PaymentReference::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domestic_reference_follows_the_finanssiala_example() {
        // Finanssiala's worked example: base 123456 gives the check digit 1.
        assert_eq!(domestic_check_digit(b"123456"), 1);
        assert!(PaymentReference::new("1234561").is_ok());
        assert!(PaymentReference::new("1234562").is_err());
    }

    #[test]
    fn domestic_reference_length_is_four_to_twenty_digits() {
        assert!(PaymentReference::new("1232").is_ok()); // 1·1+2·3+3·7 = 28 → 2
        assert!(PaymentReference::new("123").is_err());
        // Leading zeros do not change the check digit, but are not written.
        assert!(PaymentReference::new("01234561").is_err());
        let base = "1234567890123456789";
        let check = domestic_check_digit(base.as_bytes());
        assert!(PaymentReference::new(format!("{base}{check}")).is_ok());
        let long = "12345678901234567890";
        let check = domestic_check_digit(long.as_bytes());
        assert!(PaymentReference::new(format!("{long}{check}")).is_err());
    }

    #[test]
    fn rf_reference_uses_mod_97() {
        // The ISO 11649 example reference RF18 5390 0754 7034.
        assert!(PaymentReference::new("RF18539007547034").is_ok());
        assert!(PaymentReference::new("RF19539007547034").is_err());
        assert!(PaymentReference::new("rf18539007547034").is_err());
        assert!(PaymentReference::new("RF18").is_err());
    }

    #[test]
    fn spaces_and_letters_are_refused() {
        assert!(PaymentReference::new("12345 61").is_err());
        assert!(PaymentReference::new("RF18 5390 0754 7034").is_err());
        assert!(PaymentReference::new("12A4561").is_err());
        assert!(PaymentReference::new("").is_err());
    }

    #[test]
    fn serde_runs_the_same_validation() {
        let ok: PaymentReference = serde_json::from_str("\"1234561\"").unwrap();
        assert_eq!(serde_json::to_string(&ok).unwrap(), "\"1234561\"");
        assert!(serde_json::from_str::<PaymentReference>("\"1234562\"").is_err());
    }
}
