//! Canonical calendar fiscal-period identifiers.
//!
//! Legacy calendar quarters (`YYYY-Qn`) remain accepted and byte-identical. New
//! bookkeeping may select an independently closable calendar month (`YYYY-MM`).
//! `containing` deliberately remains the legacy quarter constructor; code handling
//! an explicitly selected period must use `contains` instead of projecting a date.

use std::fmt;
use std::str::FromStr;

use chrono::Datelike;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ModelError;
use crate::newtype::Date;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum PeriodKind {
    Month(u8),
    Quarter(u8),
}

/// A canonical calendar month or legacy calendar quarter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PeriodId {
    year: i32,
    kind: PeriodKind,
}

const MIN_YEAR: i32 = 1000;
const MAX_YEAR: i32 = 9999;

impl PeriodId {
    /// Construct a legacy quarter (`1..=4`).
    pub fn new(year: i32, quarter: u8) -> Result<Self, ModelError> {
        Self::validate_year(year)?;
        if !(1..=4).contains(&quarter) {
            return Err(ModelError::InvalidPeriodId(format!(
                "quarter {quarter} outside 1..=4"
            )));
        }
        Ok(Self {
            year,
            kind: PeriodKind::Quarter(quarter),
        })
    }

    /// Construct a calendar month (`1..=12`).
    pub fn new_month(year: i32, month: u8) -> Result<Self, ModelError> {
        Self::validate_year(year)?;
        if !(1..=12).contains(&month) {
            return Err(ModelError::InvalidPeriodId(format!(
                "month {month} outside 1..=12"
            )));
        }
        Ok(Self {
            year,
            kind: PeriodKind::Month(month),
        })
    }

    fn validate_year(year: i32) -> Result<(), ModelError> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
            return Err(ModelError::InvalidPeriodId(format!(
                "year {year} outside {MIN_YEAR}..={MAX_YEAR}"
            )));
        }
        Ok(())
    }

    /// Legacy constructor: the calendar quarter containing `date`.
    pub fn containing(date: Date) -> Self {
        let d = date.inner();
        Self::new(d.year(), ((d.month0() / 3) + 1) as u8)
            .expect("valid Date yields a valid PeriodId")
    }

    /// The calendar month containing `date`.
    pub fn month_containing(date: Date) -> Self {
        let d = date.inner();
        Self::new_month(d.year(), d.month() as u8).expect("valid Date yields a valid PeriodId")
    }

    /// Parse strict canonical `YYYY-MM` or legacy `YYYY-Qn` syntax.
    pub fn parse(s: &str) -> Result<Self, ModelError> {
        let bad = || ModelError::InvalidPeriodId(format!("expected YYYY-MM or YYYY-Qn, got {s:?}"));
        if !s.is_ascii() {
            return Err(bad());
        }
        if s.len() == 7 && s.as_bytes()[4] == b'-' {
            let year = &s[..4];
            let month = &s[5..];
            if year.bytes().all(|b| b.is_ascii_digit()) && month.bytes().all(|b| b.is_ascii_digit())
            {
                return Self::new_month(
                    year.parse().map_err(|_| bad())?,
                    month.parse().map_err(|_| bad())?,
                );
            }
        }
        if s.len() == 7 && &s[4..6] == "-Q" {
            let year = &s[..4];
            let quarter = &s[6..];
            if year.bytes().all(|b| b.is_ascii_digit())
                && quarter.bytes().all(|b| b.is_ascii_digit())
            {
                return Self::new(
                    year.parse().map_err(|_| bad())?,
                    quarter.parse().map_err(|_| bad())?,
                );
            }
        }
        Err(bad())
    }

    pub fn year(&self) -> i32 {
        self.year
    }

    /// Quarter number. For a monthly id this is its containing quarter, preserving
    /// the legacy accessor without using it for selected-period membership.
    pub fn quarter(&self) -> u8 {
        match self.kind {
            PeriodKind::Month(month) => (month - 1) / 3 + 1,
            PeriodKind::Quarter(quarter) => quarter,
        }
    }

    pub fn is_month(&self) -> bool {
        matches!(self.kind, PeriodKind::Month(_))
    }

    pub fn month(&self) -> Option<u8> {
        match self.kind {
            PeriodKind::Month(month) => Some(month),
            PeriodKind::Quarter(_) => None,
        }
    }

    /// Inclusive calendar-month bounds (`1..=12`) of this period.
    pub fn month_bounds(&self) -> (u8, u8) {
        match self.kind {
            PeriodKind::Month(month) => (month, month),
            PeriodKind::Quarter(quarter) => {
                let first = (quarter - 1) * 3 + 1;
                (first, first + 2)
            }
        }
    }

    pub fn contains(&self, date: Date) -> bool {
        let d = date.inner();
        let (first, last) = self.month_bounds();
        d.year() == self.year && (first..=last).contains(&(d.month() as u8))
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        if self.year != other.year {
            return false;
        }
        let (a0, a1) = self.month_bounds();
        let (b0, b1) = other.month_bounds();
        a0 <= b1 && b0 <= a1
    }

    pub fn as_string(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for PeriodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            PeriodKind::Month(month) => write!(f, "{:04}-{month:02}", self.year),
            PeriodKind::Quarter(quarter) => write!(f, "{:04}-Q{quarter}", self.year),
        }
    }
}

impl FromStr for PeriodId {
    type Err = ModelError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for PeriodId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PeriodId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_and_boundaries() {
        for (date, month, quarter) in [
            ("2026-06-30", "2026-06", "2026-Q2"),
            ("2026-07-01", "2026-07", "2026-Q3"),
            ("2026-07-31", "2026-07", "2026-Q3"),
            ("2026-08-01", "2026-08", "2026-Q3"),
            ("2024-02-29", "2024-02", "2024-Q1"),
            ("2026-01-01", "2026-01", "2026-Q1"),
            ("2026-12-31", "2026-12", "2026-Q4"),
        ] {
            let date = Date::parse(date).unwrap();
            assert_eq!(PeriodId::month_containing(date).to_string(), month);
            assert_eq!(PeriodId::containing(date).to_string(), quarter);
            assert!(PeriodId::parse(month).unwrap().contains(date));
            assert!(PeriodId::parse(quarter).unwrap().contains(date));
        }
    }

    #[test]
    fn strict_parse_and_legacy_roundtrip() {
        for good in ["2026-01", "2026-12", "2026-Q1", "2026-Q4"] {
            assert_eq!(PeriodId::parse(good).unwrap().to_string(), good);
        }
        for bad in [
            "2026-1",
            "2026-00",
            "2026-13",
            "2026-q1",
            "2026-Q0",
            "2026-Q5",
            "26-Q1",
            "2026-Q1 ",
            "2026Q1",
            "2026-Q12",
            "２０２６-01",
        ] {
            assert!(PeriodId::parse(bad).is_err(), "{bad} must be rejected");
        }
    }

    #[test]
    fn overlap_is_calendar_based() {
        let july = PeriodId::parse("2026-07").unwrap();
        assert!(july.overlaps(&PeriodId::parse("2026-Q3").unwrap()));
        assert!(!july.overlaps(&PeriodId::parse("2026-Q2").unwrap()));
        assert!(!july.overlaps(&PeriodId::parse("2027-Q3").unwrap()));
    }

    #[test]
    fn serde_preserves_quarter_and_month_strings() {
        for raw in ["2026-Q2", "2026-07"] {
            let p = PeriodId::parse(raw).unwrap();
            let json = serde_json::to_string(&p).unwrap();
            assert_eq!(json, format!("\"{raw}\""));
            assert_eq!(serde_json::from_str::<PeriodId>(&json).unwrap(), p);
        }
    }
}
