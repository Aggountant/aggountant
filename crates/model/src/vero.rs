use serde::{de, Deserialize, Deserializer, Serialize};

use crate::ModelError;

/// Finnish Business ID validated for the VAT filing boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct BusinessId(String);

impl BusinessId {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() != 9
            || bytes[7] != b'-'
            || !bytes[..7].iter().all(u8::is_ascii_digit)
            || !bytes[8].is_ascii_digit()
        {
            return Err(ModelError::InvalidVeroFilingIdentity(
                "BusinessId must have the form 1234567-8".into(),
            ));
        }
        let weights = [7_u32, 9, 10, 5, 8, 4, 2];
        let sum: u32 = bytes[..7]
            .iter()
            .zip(weights)
            .map(|(digit, weight)| u32::from(*digit - b'0') * weight)
            .sum();
        let remainder = sum % 11;
        let expected = match remainder {
            0 => 0,
            1 => {
                return Err(ModelError::InvalidVeroFilingIdentity(
                    "BusinessId has an invalid checksum".into(),
                ))
            }
            other => 11 - other,
        };
        if u32::from(bytes[8] - b'0') != expected {
            return Err(ModelError::InvalidVeroFilingIdentity(
                "BusinessId has an invalid checksum".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for BusinessId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Mandatory contact name carried only by a VAT filing request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct FilingContactName(String);

impl FilingContactName {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if value.trim() != value
            || value.is_empty()
            || value.chars().count() > 70
            || value.chars().any(char::is_control)
        {
            return Err(ModelError::InvalidVeroFilingIdentity(
                "filing contact name must be 1..70 trimmed printable characters".into(),
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for FilingContactName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

/// Mandatory contact phone for a VAT filing. Formatting punctuation is retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct FilingContactPhone(String);

impl FilingContactPhone {
    pub fn parse(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        let digits = value.chars().filter(char::is_ascii_digit).count();
        let valid_chars = value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')'));
        if value.trim() != value
            || !(5..=15).contains(&digits)
            || value.len() > 30
            || !valid_chars
            || (value.contains('+') && !value.starts_with('+'))
        {
            return Err(ModelError::InvalidVeroFilingIdentity(
                "filing contact phone must contain 5..15 digits and only phone punctuation".into(),
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for FilingContactPhone {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_id_checksum_is_enforced() {
        assert!(BusinessId::parse("1572860-0").is_ok());
        assert!(BusinessId::parse("1572860-1").is_err());
    }

    #[test]
    fn contact_values_reject_blank_and_malformed_values() {
        assert!(FilingContactName::parse("Ada Operator").is_ok());
        assert!(FilingContactName::parse(" ").is_err());
        assert!(FilingContactPhone::parse("+358 40 123 4567").is_ok());
        assert!(FilingContactPhone::parse("call-me").is_err());
    }
}
