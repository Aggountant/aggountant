//! The deliberately narrow, tax-free travel-only earnings report. No salary mapping.
use crate::{resolve_tulolaji, DomainError};
use aggountant_model::{BusinessId, Date, FilingContactName, FilingContactPhone, Instant};
use jiff::civil::Date as CivilDate;
use jiff::ToSpan;
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
    pub fn validate(&self, today: CivilDate) -> Result<(), DomainError> {
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
        if self.payment_date.inner() < CivilDate::constant(2019, 1, 1)
            || self.payment_date.inner() > today + 45.days()
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
    pub fn deadline(&self) -> CivilDate {
        self.payment_date.inner() + 5.days()
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

/// How an accepted travel report is corrected in the Incomes Register.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WageCorrectionAction {
    /// A replacement report (`ActionCode` 2): the whole report again, corrected.
    Replacement,
    /// A cancellation (`InvalidationsToIR`, record type 105): the report is
    /// withdrawn and nothing takes its place.
    Cancellation,
}

/// One correction to the report the Incomes Register holds as `ir_report_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WageCorrection {
    pub action: WageCorrectionAction,
    /// A replacement: the report as it should read. A cancellation: the report
    /// as last known; only its payer goes into the record.
    pub report: WageReport,
    pub ir_report_id: String,
    /// The version being corrected, when known. It is sent, so the register
    /// refuses to correct anything but the latest version.
    pub corrects_version: Option<u32>,
}

impl WageCorrection {
    pub fn validate(&self, today: CivilDate) -> Result<(), DomainError> {
        let bad = |s: &str| DomainError::Payroll(s.into());
        if self.action == WageCorrectionAction::Replacement {
            self.report.validate(today)?;
        }
        if self.ir_report_id.len() != 32
            || !self.ir_report_id.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(bad(
                "a correction needs the 32-character Incomes Register report reference",
            ));
        }
        if self.corrects_version == Some(0) {
            return Err(bad("corrected report version must be positive"));
        }
        Ok(())
    }
    /// Digest of the whole correction and its creation time. It carries no
    /// personal identity code, so it is safe in process state, and a changed
    /// correction or a later attempt never shares it.
    pub fn correction_hash(&self, timestamp: Instant) -> String {
        let mut h = Sha256::new();
        h.update(b"aggountant/wage-correction/v1");
        let body = serde_json::to_vec(&(self, timestamp)).expect("correction serializes");
        h.update((body.len() as u64).to_be_bytes());
        h.update(&body);
        hex::encode(h.finalize())
    }
    /// The record reference, unique per correction record (at most 40 characters).
    pub fn delivery_id(&self, timestamp: Instant) -> String {
        format!("c-{}", &self.correction_hash(timestamp)[..38])
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
        let today = CivilDate::constant(2026, 10, 24);
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
        assert!(r.validate(CivilDate::constant(2026, 1, 1)).is_err());
    }
    #[test]
    fn correction_identity_is_per_record_and_needs_a_register_reference() {
        let today = CivilDate::constant(2026, 10, 24);
        let at: Instant = "2026-11-02T08:00:00Z".parse().unwrap();
        let c = WageCorrection {
            action: WageCorrectionAction::Replacement,
            report: report(),
            ir_report_id: "22222222222233334444555555555555".into(),
            corrects_version: Some(1),
        };
        c.validate(today).unwrap();
        assert_eq!(c.delivery_id(at).len(), 40);
        assert_ne!(c.delivery_id(at), c.report.delivery_id());
        let later: Instant = "2026-11-02T08:00:01Z".parse().unwrap();
        assert_ne!(c.delivery_id(at), c.delivery_id(later));
        let mut cancel = c.clone();
        cancel.action = WageCorrectionAction::Cancellation;
        assert_ne!(c.correction_hash(at), cancel.correction_hash(at));
        // A cancellation does not judge the report it withdraws.
        cancel.report.lines.clear();
        cancel.validate(today).unwrap();
        let mut bad = c.clone();
        bad.report.lines.clear();
        assert!(bad.validate(today).is_err());
        bad = c.clone();
        bad.ir_report_id = "not-a-guid".into();
        assert!(bad.validate(today).is_err());
        bad = c;
        bad.corrects_version = Some(0);
        assert!(bad.validate(today).is_err());
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
