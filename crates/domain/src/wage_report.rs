//! The deliberately narrow, tax-free travel-only earnings report. No salary mapping.
use crate::{resolve_tulolaji, DomainError};
use aggountant_model::{BusinessId, Date, FilingContactName, FilingContactPhone};
use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WageReport {
    pub business_id: BusinessId,
    pub source_reference: String,
    pub employee_id: String,
    pub payment_date: Date,
    pub period_start: Date,
    pub period_end: Date,
    /// Joint ownership (IncomeEarnerType 2), not unemployment-law partial ownership.
    pub joint_owner: bool,
    pub contact_name: FilingContactName,
    pub contact_phone: FilingContactPhone,
    pub lines: Vec<TravelLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "income_type", deny_unknown_fields)]
pub enum TravelLine {
    #[serde(rename = "311")]
    Kilometres {
        #[serde(with = "decimal_string")]
        amount: Decimal,
        kilometers: i32,
    },
    #[serde(rename = "331")]
    DailyAllowance {
        #[serde(with = "decimal_string")]
        amount: Decimal,
        allowance_code: u8,
    },
}
impl TravelLine {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Kilometres { .. } => "311",
            Self::DailyAllowance { .. } => "331",
        }
    }
    pub fn amount(&self) -> String {
        let mut d = match self {
            Self::Kilometres { amount, .. } | Self::DailyAllowance { amount, .. } => *amount,
        };
        d.rescale(2);
        d.to_string()
    }
}
impl WageReport {
    pub fn validate(&self, today: NaiveDate) -> Result<(), DomainError> {
        let bad = |s: &str| DomainError::Payroll(s.into());
        for (name, value) in [
            ("source_reference", &self.source_reference),
            ("employee_id", &self.employee_id),
        ] {
            if value.is_empty()
                || value.len() > 200
                || value.trim() != value
                || value.chars().any(char::is_control)
            {
                return Err(bad(&format!("{name} must be 1..200 bytes without surrounding whitespace or control characters")));
            }
        }
        if self.period_end < self.period_start {
            return Err(bad("period_end must be >= period_start"));
        }
        if self.payment_date.inner() < NaiveDate::from_ymd_opt(2019, 1, 1).unwrap()
            || self.payment_date.inner() > today + Duration::days(45)
        {
            return Err(bad(
                "payment_date must be >= 2019-01-01 and no more than 45 days in the future",
            ));
        }
        if self.lines.is_empty() || self.lines.len() > 100 {
            return Err(bad("supply 1..100 travel lines"));
        }
        for line in &self.lines {
            resolve_tulolaji(line.code())?;
            let amount = match line {
                TravelLine::Kilometres { amount, kilometers } => {
                    if *kilometers <= 0 {
                        return Err(bad("311 requires positive integer kilometers"));
                    }
                    amount
                }
                TravelLine::DailyAllowance {
                    amount,
                    allowance_code,
                } => {
                    if !matches!(allowance_code, 1..=4) {
                        return Err(bad("331 allowance_code must be 1 (meal), 2 (partial), 3 (full), or 4 (international)"));
                    }
                    amount
                }
            };
            if amount.scale() > 2
                || *amount <= Decimal::ZERO
                || *amount >= Decimal::from(1_000_000_000_000_i64)
            {
                return Err(bad(
                    "travel amount must be positive EUR cents, below 1000000000000",
                ));
            }
        }
        Ok(())
    }
    pub fn deadline(&self) -> NaiveDate {
        self.payment_date.inner() + Duration::days(5)
    }
    /// Full framed natural-key digest retained locally; bounded public references are not payload hashes.
    pub fn identity_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(b"aggountant/wage-report/v1");
        for s in [
            self.business_id.as_str(),
            &self.source_reference,
            &self.employee_id,
            &self.payment_date.to_string(),
        ] {
            h.update((s.len() as u64).to_be_bytes());
            h.update(s.as_bytes());
        }
        hex::encode(h.finalize())
    }
    pub fn delivery_id(&self) -> String {
        format!("d-{}", &self.identity_hash()[..38])
    }
    pub fn report_id(&self) -> String {
        format!("r-{}", &self.identity_hash()[..38])
    }
}

mod decimal_string {
    use super::*;
    pub fn serialize<S: serde::Serializer>(d: &Decimal, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&d.to_string())
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Decimal, D::Error> {
        let s = String::deserialize(d)?;
        if s.is_empty() || s.trim() != s || !s.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
            return Err(serde::de::Error::custom(
                "amount must be an unsigned decimal string",
            ));
        }
        Decimal::from_str_exact(&s).map_err(|_| serde::de::Error::custom("invalid decimal amount"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn report() -> WageReport {
        serde_json::from_value(json!({"business_id":"1234567-1","employee_id":"emp-1","source_reference":"expense-1","payment_date":"2026-10-25","period_start":"2026-09-01","period_end":"2026-09-30","joint_owner":true,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","lines":[{"income_type":"311","amount":"120.00","kilometers":200}]})).unwrap()
    }
    #[test]
    fn strict_subset_and_stable_natural_identity() {
        let mut r = report();
        let today = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        r.validate(today).unwrap();
        let identity = r.identity_hash();
        r.joint_owner = false;
        assert_eq!(r.identity_hash(), identity);
        assert_eq!(r.delivery_id().len(), 40);
        assert_eq!(r.report_id().len(), 40);
        r.lines = vec![TravelLine::Kilometres {
            amount: Decimal::ONE,
            kilometers: 0,
        }];
        assert!(r.validate(today).is_err());
        r.lines = vec![TravelLine::DailyAllowance {
            amount: Decimal::ONE,
            allowance_code: 5,
        }];
        assert!(r.validate(today).is_err());
        r.lines = vec![TravelLine::DailyAllowance {
            amount: Decimal::from_str_exact("1.001").unwrap(),
            allowance_code: 3,
        }];
        assert!(r.validate(today).is_err());
        r = report();
        r.source_reference = " ".into();
        assert!(r.validate(today).is_err());
        r = report();
        assert!(r
            .validate(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
            .is_err());
    }
    #[test]
    fn reject_missing_ownership_unknown_fields_and_non_decimal_strings() {
        let v = serde_json::to_value(report()).unwrap();
        for replacement in [json!(-1), json!(1.5), json!(" 1.00"), json!("1e2")] {
            let mut bad = v.clone();
            bad["lines"][0]["amount"] = replacement;
            assert!(serde_json::from_value::<WageReport>(bad).is_err());
        }
        let mut bad = v.clone();
        bad.as_object_mut().unwrap().remove("joint_owner");
        assert!(serde_json::from_value::<WageReport>(bad).is_err());
        let mut bad = v;
        bad["lines"][0]["unexpected"] = json!(true);
        assert!(serde_json::from_value::<WageReport>(bad).is_err());
    }
}
