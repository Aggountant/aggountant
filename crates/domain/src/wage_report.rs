//! The deliberately narrow, tax-free travel-only earnings report. No salary mapping.
use crate::bank_days::{is_finnish_bank_day, next_finnish_bank_day};
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
#[serde(try_from = "WireLine", into = "WireLine")]
pub enum TravelLine {
    Kilometres {
        amount: Decimal,
        kilometers: i32,
    },
    /// One daily-allowance amount and every per diem type it contains. The
    /// Incomes Register reports them together: "Arvoja voi valita useita
    /// samalle summalle" (Verohallinto, detailed guidance on benefits and
    /// reimbursements of expenses, 2.1.3 Päiväraha).
    DailyAllowance {
        amount: Decimal,
        allowance_codes: AllowanceCodes,
    },
}

/// The per diem types (`AllowanceCode`) of one 331 amount, in ascending order,
/// so the same set always serializes and is sent the same way.
/// [`WageReport::validate`] refuses an empty set, a duplicate and a code
/// outside 1–4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowanceCodes(Vec<u8>);
impl AllowanceCodes {
    pub fn new(mut codes: Vec<u8>) -> Self {
        codes.sort_unstable();
        Self(codes)
    }
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

/// The JSON shape of a travel line. A 331 line takes `allowance_codes`, or
/// `allowance_code` for one type, the shape every report had before several
/// types were supported. A one-type line is written back as `allowance_code`,
/// so stored state and the proposal hash bound to it stay byte-identical.
#[derive(Serialize, Deserialize)]
#[serde(tag = "income_type", deny_unknown_fields)]
enum WireLine {
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
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        allowance_code: Option<u8>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        allowance_codes: Option<Vec<u8>>,
    },
}
/// A key that is given must carry a value; `null` is refused, not read as absent.
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
impl TryFrom<WireLine> for TravelLine {
    type Error = String;
    fn try_from(line: WireLine) -> Result<Self, String> {
        Ok(match line {
            WireLine::Kilometres { amount, kilometers } => Self::Kilometres { amount, kilometers },
            WireLine::DailyAllowance {
                amount,
                allowance_code,
                allowance_codes,
            } => {
                let codes = match (allowance_code, allowance_codes) {
                    (Some(code), None) => vec![code],
                    (None, Some(codes)) => codes,
                    (Some(_), Some(_)) => {
                        return Err(
                            "a 331 line takes allowance_codes or allowance_code, not both".into(),
                        )
                    }
                    (None, None) => return Err("a 331 line requires allowance_codes".into()),
                };
                Self::DailyAllowance {
                    amount,
                    allowance_codes: AllowanceCodes::new(codes),
                }
            }
        })
    }
}
impl From<TravelLine> for WireLine {
    fn from(line: TravelLine) -> Self {
        match line {
            TravelLine::Kilometres { amount, kilometers } => {
                Self::Kilometres { amount, kilometers }
            }
            TravelLine::DailyAllowance {
                amount,
                allowance_codes,
            } => {
                let (allowance_code, allowance_codes) = match allowance_codes.as_slice() {
                    [code] => (Some(*code), None),
                    codes => (None, Some(codes.to_vec())),
                };
                Self::DailyAllowance {
                    amount,
                    allowance_code,
                    allowance_codes,
                }
            }
        }
    }
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
                    allowance_codes,
                } => {
                    let codes = allowance_codes.as_slice();
                    if codes.is_empty()
                        || codes.windows(2).any(|w| w[0] == w[1])
                        || codes.iter().any(|c| !matches!(c, 1..=4))
                    {
                        return Err(bad("331 allowance_codes must be one or more distinct codes: 1 (meal), 2 (partial), 3 (full), 4 (international)"));
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
    /// The statutory deadline of this report. Every line a `WageReport` can
    /// carry is a tax-exempt travel reimbursement, so this is
    /// [`travel_report_deadline`] of the payment date. A report that combines a
    /// month's payments carries the latest of them as its payment date.
    pub fn deadline(&self) -> CivilDate {
        travel_report_deadline(self.payment_date.inner())
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

/// The Incomes Register deadline for a report of `income_type` paid on
/// `payment_date`. The deadline depends on the income type: tax-exempt travel
/// reimbursements (311 kilometre allowance, 331 daily allowance) follow
/// [`travel_report_deadline`]. Any other income type is refused rather than
/// given a guessed deadline; a taxable type, when one is modelled, is added
/// here with the five-day rule of pay data (`palkkatieto_deadline`).
pub fn report_deadline(
    income_type: &str,
    payment_date: CivilDate,
) -> Result<CivilDate, DomainError> {
    match income_type {
        "311" | "331" => Ok(travel_report_deadline(payment_date)),
        other => Err(DomainError::Tulolaji(format!(
            "no Incomes Register deadline is modelled for income type {other:?}; only 311 and 331 are"
        ))),
    }
}

/// The statutory deadline of a tax-exempt travel reimbursement report: the
/// 5th day of the month after the payment month, moved to the next business
/// day when the 5th is not one.
///
/// Laki tulotietojärjestelmästä (53/2018) 12 § 11 mom.: tax-exempt
/// reimbursements of expenses are reported "kuukausittain viimeistään
/// maksukuukautta seuraavan kalenterikuukauden viidentenä päivänä". Several
/// payments of one month may go in one report with the latest payment date
/// (Verohallinto, "Reporting reimbursements of expenses to the Incomes
/// Register").
///
/// The shift: Verohallinto's detailed guidance "Wages: reporting data to the
/// Incomes Register" (section 2.1) says that a filing deadline falling on a
/// Saturday, Sunday or other holiday may be met on the following business day,
/// under section 5 of laki säädettyjen määräaikain laskemisesta (150/1930).
/// That section names pyhäpäivät (Sundays and church holidays), Independence
/// Day, May Day, Christmas Eve, Midsummer Eve and weekday Saturdays. Its
/// weekday holidays are exactly the Finnish bank holidays of
/// [`crate::bank_days`], so the first day it allows is the next bank day.
pub fn travel_report_deadline(payment_date: CivilDate) -> CivilDate {
    let fifth = payment_date
        .first_of_month()
        .checked_add(1.month())
        .expect("a payment date validated after 2019 has a following month")
        .with()
        .day(5)
        .build()
        .expect("every month has a 5th day");
    if is_finnish_bank_day(fifth) {
        fifth
    } else {
        next_finnish_bank_day(fifth)
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
            allowance_codes: AllowanceCodes::new(vec![5]),
        }];
        assert!(r.validate(today).is_err());
        r.lines = vec![TravelLine::DailyAllowance {
            amount: Decimal::from_str_exact("1.001").unwrap(),
            allowance_codes: AllowanceCodes::new(vec![3]),
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
    fn date(s: &str) -> CivilDate {
        s.parse().unwrap()
    }
    #[test]
    fn travel_deadline_is_the_fifth_of_the_following_month() {
        for (paid, due) in [
            // Thursday 5 November 2026.
            ("2026-10-25", "2026-11-05"),
            // Every day of the month has the same deadline, month end included.
            ("2026-10-01", "2026-11-05"),
            ("2026-10-31", "2026-11-05"),
            ("2026-01-31", "2026-02-05"),
            ("2028-02-29", "2028-03-06"), // 5 March 2028 is a Sunday.
            // December → January of the next year: 5 January 2027 is a Tuesday.
            ("2026-12-31", "2027-01-05"),
            // Saturday 5 September 2026 → Monday 7 September.
            ("2026-08-14", "2026-09-07"),
            // Sunday 5 July 2026 → Monday 6 July.
            ("2026-06-30", "2026-07-06"),
            // Easter 2026 is 5 April: Sunday, then Easter Monday 6 April → Tuesday 7 April.
            ("2026-03-02", "2026-04-07"),
            // Easter Monday 5 April 2021 → Tuesday 6 April.
            ("2021-03-31", "2021-04-06"),
            // Good Friday 5 April 2075 (Easter 7 April) → Tuesday 9 April.
            ("2075-03-15", "2075-04-09"),
            // Ascension Day 5 May 2016 → Friday 6 May.
            ("2016-04-29", "2016-05-06"),
            // Saturday 5 January 2030, Epiphany falls on the Sunday → Monday 7 January.
            ("2029-12-10", "2030-01-07"),
            // Sunday 5 January 2025, then Epiphany Monday 6 January → Tuesday 7 January.
            ("2024-12-20", "2025-01-07"),
        ] {
            assert_eq!(travel_report_deadline(date(paid)), date(due), "{paid}");
        }
    }
    #[test]
    fn deadline_is_a_function_of_the_income_type() {
        let paid = date("2026-10-25");
        for code in ["311", "331"] {
            assert_eq!(report_deadline(code, paid).unwrap(), date("2026-11-05"));
        }
        assert!(report_deadline("101", paid).is_err());
        let mut r = report();
        assert_eq!(r.deadline(), date("2026-11-05"));
        r.lines = vec![TravelLine::DailyAllowance {
            amount: Decimal::ONE,
            allowance_codes: AllowanceCodes::new(vec![3]),
        }];
        assert_eq!(r.deadline(), date("2026-11-05"));
        // Pay data keeps its five-day rule.
        let pay = aggountant_model::Date::parse("2026-10-25").unwrap();
        assert_eq!(crate::palkkatieto_deadline(pay).to_string(), "2026-10-30");
    }
    fn daily(line: serde_json::Value) -> Result<TravelLine, serde_json::Error> {
        serde_json::from_value(line)
    }
    #[test]
    fn one_daily_allowance_carries_several_per_diem_types() {
        let today = CivilDate::constant(2026, 10, 24);
        // The shape of every report before several types: read as a set of
        // one and written back byte-identically, so stored proposal hashes hold.
        let legacy = json!({"income_type":"331","amount":"54.00","allowance_code":3});
        let line = daily(legacy.clone()).unwrap();
        assert_eq!(
            line,
            TravelLine::DailyAllowance {
                amount: Decimal::from_str_exact("54.00").unwrap(),
                allowance_codes: AllowanceCodes::new(vec![3]),
            }
        );
        assert_eq!(
            serde_json::to_string(&line).unwrap(),
            r#"{"income_type":"331","amount":"54.00","allowance_code":3}"#
        );
        assert_eq!(
            daily(json!({"income_type":"331","amount":"54.00","allowance_codes":[3]})).unwrap(),
            line
        );
        // Several types: one order, whatever order they were given in.
        let several =
            daily(json!({"income_type":"331","amount":"79.00","allowance_codes":[3,2]})).unwrap();
        assert_eq!(
            serde_json::to_value(&several).unwrap(),
            json!({"income_type":"331","amount":"79.00","allowance_codes":[2,3]})
        );
        let mut r = report();
        r.lines = vec![several];
        r.validate(today).unwrap();
        // Exactly one of the two keys, and a given key has a value.
        for bad in [
            json!({"income_type":"331","amount":"1.00"}),
            json!({"income_type":"331","amount":"1.00","allowance_code":3,"allowance_codes":[2]}),
            json!({"income_type":"331","amount":"1.00","allowance_code":null}),
            json!({"income_type":"331","amount":"1.00","allowance_codes":null}),
            json!({"income_type":"331","amount":"1.00","allowance_codes":[2],"kilometers":1}),
            json!({"income_type":"311","amount":"1.00","kilometers":1,"allowance_codes":[2]}),
        ] {
            assert!(daily(bad.clone()).is_err(), "{bad}");
        }
        // Empty, duplicate and unknown codes are refused by validation.
        for codes in [vec![], vec![3, 3], vec![2, 5], vec![0]] {
            let mut r = report();
            r.lines = vec![TravelLine::DailyAllowance {
                amount: Decimal::ONE,
                allowance_codes: AllowanceCodes::new(codes.clone()),
            }];
            assert!(r.validate(today).is_err(), "{codes:?}");
        }
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
