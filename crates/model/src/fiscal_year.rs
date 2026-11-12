//! Fiscal years and the calendar that maps a date to its fiscal year.
//!
//! A fiscal year is identified by an `i32` label: the calendar year in which it
//! **ends** (how Vero identifies a tilikausi). A calendar fiscal year's label is the
//! year itself, so every year stored before fiscal-year definitions existed keeps its
//! meaning. Labels are unique and strictly increasing in time, which keeps integer
//! comparisons between years valid; they may skip a value after an 18-month year, so
//! the previous year comes from [`FiscalCalendar::previous`], never from `year - 1`.

use chrono::{Datelike, Months, NaiveDate};
use serde::Serialize;

use crate::error::ModelError;
use crate::newtype::Date;
use crate::period::PeriodId;

/// Kirjanpitolaki 1:4: a fiscal year is 12 months; when operations start or end or
/// the timing of the financial statements changes it may be shorter or longer, but
/// at most 18 months.
pub const MAX_FISCAL_YEAR_MONTHS: u32 = 18;

/// One fiscal year: whole calendar months from `start` to `end`, labelled by the
/// calendar year of `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct FiscalYear {
    year: i32,
    start: Date,
    end: Date,
}

fn date_of(d: NaiveDate) -> Result<Date, ModelError> {
    Date::from_ymd(d.year(), d.month(), d.day())
}

fn month_index(d: NaiveDate) -> i64 {
    i64::from(d.year()) * 12 + i64::from(d.month0())
}

fn add_months(d: NaiveDate, months: u32) -> Result<NaiveDate, ModelError> {
    d.checked_add_months(Months::new(months))
        .ok_or_else(|| ModelError::InvalidDate(format!("{d} + {months} months overflows")))
}

impl FiscalYear {
    /// A fiscal year of whole months, 1 to [`MAX_FISCAL_YEAR_MONTHS`] long.
    pub fn new(start: Date, end: Date) -> Result<Self, ModelError> {
        let (s, e) = (start.inner(), end.inner());
        if s.day() != 1 {
            return Err(ModelError::InvalidFiscalYear(format!(
                "fiscal year must start on the first day of a month, got {start}"
            )));
        }
        if e.succ_opt().is_none_or(|next| next.day() != 1) {
            return Err(ModelError::InvalidFiscalYear(format!(
                "fiscal year must end on the last day of a month, got {end}"
            )));
        }
        if e < s {
            return Err(ModelError::InvalidFiscalYear(format!(
                "fiscal year end {end} is before its start {start}"
            )));
        }
        let months = month_index(e) - month_index(s) + 1;
        if months > i64::from(MAX_FISCAL_YEAR_MONTHS) {
            return Err(ModelError::InvalidFiscalYear(format!(
                "fiscal year {start}..{end} is {months} months; kirjanpitolaki 1:4 allows at \
                 most {MAX_FISCAL_YEAR_MONTHS} months"
            )));
        }
        Ok(Self {
            year: e.year(),
            start,
            end,
        })
    }

    /// The calendar year `year` as a fiscal year.
    pub fn calendar(year: i32) -> Result<Self, ModelError> {
        Self::new(Date::from_ymd(year, 1, 1)?, Date::from_ymd(year, 12, 31)?)
    }

    /// The label: the calendar year in which this fiscal year ends.
    pub fn year(&self) -> i32 {
        self.year
    }

    pub fn start(&self) -> Date {
        self.start
    }

    pub fn end(&self) -> Date {
        self.end
    }

    pub fn months(&self) -> u32 {
        (month_index(self.end.inner()) - month_index(self.start.inner()) + 1) as u32
    }

    pub fn contains(&self, date: Date) -> bool {
        self.start <= date && date <= self.end
    }

    pub fn is_calendar_year(&self) -> bool {
        let (s, e) = (self.start.inner(), self.end.inner());
        (s.month(), s.day(), e.month(), e.day()) == (1, 1, 12, 31) && s.year() == e.year()
    }

    /// The calendar months that make up this year, in order.
    pub fn month_periods(&self) -> Result<Vec<PeriodId>, ModelError> {
        let first = self.start.inner();
        (0..self.months())
            .map(|offset| {
                let d = add_months(first, offset)?;
                PeriodId::new_month(d.year(), d.month() as u8)
            })
            .collect()
    }

    /// The 12-month year that starts the day after this one ends.
    fn following_twelve(&self) -> Result<Self, ModelError> {
        let start = self
            .end
            .inner()
            .succ_opt()
            .ok_or_else(|| ModelError::InvalidDate(format!("no day after {}", self.end)))?;
        Self::twelve_from(start)
    }

    fn twelve_from(start: NaiveDate) -> Result<Self, ModelError> {
        let end = add_months(start, 12)?
            .pred_opt()
            .expect("a date after a valid start has a predecessor");
        Self::new(date_of(start)?, date_of(end)?)
    }
}

/// The company's fiscal years, derived from its explicit definitions:
///
/// - no definitions: calendar years;
/// - inside a defined year: that year;
/// - after a defined year: 12-month years continue from its end until the next
///   definition, which must start on one of those boundaries;
/// - before the first definition: calendar years when it starts on 1 January (a
///   calendar-year company changing its fiscal year); otherwise no fiscal year (a
///   company whose first year starts mid-calendar-year).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FiscalCalendar {
    defined: Vec<FiscalYear>,
}

impl FiscalCalendar {
    /// Validate definitions given in time order.
    pub fn new(defined: Vec<FiscalYear>) -> Result<Self, ModelError> {
        for pair in defined.windows(2) {
            let (prev, next) = (&pair[0], &pair[1]);
            if next.start <= prev.end {
                return Err(ModelError::InvalidFiscalYear(format!(
                    "fiscal year {}..{} overlaps or precedes {}..{}",
                    next.start, next.end, prev.start, prev.end
                )));
            }
            // Walk the 12-month continuation of `prev` to the boundary `next` must hit.
            let mut before = *prev;
            loop {
                let following = before.following_twelve()?;
                if following.start >= next.start {
                    if following.start != next.start {
                        return Err(ModelError::InvalidFiscalYear(format!(
                            "fiscal year {}..{} does not start where a fiscal year ends: the \
                             year before it runs {}..{}",
                            next.start, next.end, following.start, following.end
                        )));
                    }
                    break;
                }
                before = following;
            }
            if before.year >= next.year {
                return Err(ModelError::InvalidFiscalYear(format!(
                    "fiscal years {}..{} and {}..{} both end in {}; aggountant identifies a \
                     fiscal year by the calendar year it ends in and cannot keep two in one \
                     calendar year. Use one longer transition year instead (at most \
                     {MAX_FISCAL_YEAR_MONTHS} months, kirjanpitolaki 1:4)",
                    before.start, before.end, next.start, next.end, next.year
                )));
            }
        }
        Ok(Self { defined })
    }

    /// The explicit definitions, in time order.
    pub fn defined(&self) -> &[FiscalYear] {
        &self.defined
    }

    /// This calendar with `year` appended. Definitions are append-only: `year` must
    /// start after every existing definition.
    pub fn with_definition(&self, year: FiscalYear) -> Result<Self, ModelError> {
        if let Some(last) = self.defined.last() {
            if year.start <= last.end {
                return Err(ModelError::InvalidFiscalYear(format!(
                    "fiscal year {}..{} must start after the last defined fiscal year {}..{}; \
                     definitions cannot be changed or inserted before an existing one",
                    year.start, year.end, last.start, last.end
                )));
            }
        }
        let mut defined = self.defined.clone();
        defined.push(year);
        Self::new(defined)
    }

    /// The fiscal year containing `date`.
    pub fn year_of(&self, date: Date) -> Result<FiscalYear, ModelError> {
        let Some(first) = self.defined.first() else {
            return FiscalYear::calendar(date.inner().year());
        };
        if date < first.start {
            let s = first.start.inner();
            if s.month() == 1 && s.day() == 1 {
                return FiscalYear::calendar(date.inner().year());
            }
            return Err(ModelError::FiscalYearUndefined(format!(
                "{date} is before the first fiscal year {}..{}",
                first.start, first.end
            )));
        }
        let anchor = self
            .defined
            .iter()
            .rev()
            .find(|year| year.start <= date)
            .expect("date is on or after the first definition");
        if date <= anchor.end {
            return Ok(*anchor);
        }
        let continuation = anchor
            .end
            .inner()
            .succ_opt()
            .ok_or_else(|| ModelError::InvalidDate(format!("no day after {}", anchor.end)))?;
        // Years are whole months, so the month distance alone picks the year.
        let whole_years = (month_index(date.inner()) - month_index(continuation)) / 12;
        FiscalYear::twelve_from(add_months(continuation, (whole_years * 12) as u32)?)
    }

    /// The fiscal year labelled `year` (ending in that calendar year).
    pub fn year(&self, year: i32) -> Result<FiscalYear, ModelError> {
        let missing = || {
            ModelError::FiscalYearUndefined(format!("no fiscal year ends in calendar year {year}"))
        };
        let last_day = Date::from_ymd(year, 12, 31)?;
        let containing = self.year_of(last_day).map_err(|_| missing())?;
        if containing.year == year {
            return Ok(containing);
        }
        // The year containing 31.12. ends later; the one ending in `year`, if any, is
        // its predecessor (labels are unique).
        let previous = self.previous(&containing).map_err(|_| missing())?;
        if previous.year == year {
            Ok(previous)
        } else {
            Err(missing())
        }
    }

    /// The fiscal year before `year`.
    pub fn previous(&self, year: &FiscalYear) -> Result<FiscalYear, ModelError> {
        let day_before = year
            .start
            .inner()
            .pred_opt()
            .ok_or_else(|| ModelError::InvalidDate(format!("no day before {}", year.start)))?;
        self.year_of(date_of(day_before)?)
    }

    /// The fiscal year after `year`.
    pub fn next(&self, year: &FiscalYear) -> Result<FiscalYear, ModelError> {
        let day_after = year
            .end
            .inner()
            .succ_opt()
            .ok_or_else(|| ModelError::InvalidDate(format!("no day after {}", year.end)))?;
        self.year_of(date_of(day_after)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn fy(start: &str, end: &str) -> FiscalYear {
        FiscalYear::new(d(start), d(end)).unwrap()
    }

    fn range(year: &FiscalYear) -> (String, String) {
        (year.start().to_string(), year.end().to_string())
    }

    #[test]
    fn lengths_follow_kirjanpitolaki_1_4() {
        assert_eq!(fy("2026-07-01", "2027-06-30").months(), 12);
        assert_eq!(fy("2026-07-01", "2027-12-31").months(), 18);
        assert_eq!(fy("2026-07-01", "2026-07-31").months(), 1);
        let err = FiscalYear::new(d("2026-07-01"), d("2028-01-31")).unwrap_err();
        assert_eq!(err.error_code(), "INVALID_FISCAL_YEAR");
        assert!(err.to_string().contains("kirjanpitolaki 1:4"), "{err}");
        assert!(err.to_string().contains("19 months"), "{err}");
        for (start, end) in [
            ("2026-07-02", "2027-06-30"),
            ("2026-07-01", "2027-06-29"),
            ("2026-07-01", "2026-06-30"),
        ] {
            assert_eq!(
                FiscalYear::new(d(start), d(end)).unwrap_err().error_code(),
                "INVALID_FISCAL_YEAR",
                "{start}..{end}"
            );
        }
    }

    #[test]
    fn label_is_the_ending_calendar_year() {
        let year = fy("2026-07-01", "2027-06-30");
        assert_eq!(year.year(), 2027);
        assert!(!year.is_calendar_year());
        assert!(FiscalYear::calendar(2026).unwrap().is_calendar_year());
        let months: Vec<String> = year
            .month_periods()
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(months.first().unwrap(), "2026-07");
        assert_eq!(months.last().unwrap(), "2027-06");
        assert_eq!(months.len(), 12);
    }

    #[test]
    fn empty_calendar_is_calendar_years() {
        let cal = FiscalCalendar::default();
        let year = cal.year_of(d("2026-03-14")).unwrap();
        assert_eq!(year, FiscalYear::calendar(2026).unwrap());
        assert_eq!(cal.year(2026).unwrap(), year);
        assert_eq!(cal.previous(&year).unwrap().year(), 2025);
        assert_eq!(cal.next(&year).unwrap().year(), 2027);
    }

    #[test]
    fn july_to_june_company_from_its_first_year() {
        let cal = FiscalCalendar::new(vec![fy("2026-07-01", "2027-06-30")]).unwrap();
        assert_eq!(cal.year_of(d("2026-07-01")).unwrap().year(), 2027);
        assert_eq!(cal.year_of(d("2027-06-30")).unwrap().year(), 2027);
        let later = cal.year_of(d("2027-07-01")).unwrap();
        assert_eq!(range(&later), ("2027-07-01".into(), "2028-06-30".into()));
        assert_eq!(later.year(), 2028);
        assert_eq!(
            range(&cal.year_of(d("2031-06-30")).unwrap()),
            ("2030-07-01".into(), "2031-06-30".into())
        );
        assert_eq!(
            cal.year_of(d("2026-06-30")).unwrap_err().error_code(),
            "FISCAL_YEAR_UNDEFINED"
        );
        assert_eq!(cal.year(2030).unwrap().start(), d("2029-07-01"));
        assert_eq!(
            cal.year(2026).unwrap_err().error_code(),
            "FISCAL_YEAR_UNDEFINED"
        );
        assert_eq!(
            cal.previous(&cal.year(2027).unwrap())
                .unwrap_err()
                .error_code(),
            "FISCAL_YEAR_UNDEFINED"
        );
    }

    #[test]
    fn long_first_year_skips_a_label() {
        let cal = FiscalCalendar::new(vec![fy("2026-07-01", "2027-12-31")]).unwrap();
        let first = cal.year(2027).unwrap();
        assert_eq!(first.months(), 18);
        assert_eq!(
            cal.next(&first).unwrap(),
            FiscalYear::calendar(2028).unwrap()
        );
        assert_eq!(
            cal.year(2026).unwrap_err().error_code(),
            "FISCAL_YEAR_UNDEFINED"
        );
    }

    #[test]
    fn calendar_company_changing_to_july() {
        let cal = FiscalCalendar::new(vec![fy("2027-01-01", "2027-06-30")]).unwrap();
        assert_eq!(
            cal.year_of(d("2026-12-31")).unwrap(),
            FiscalYear::calendar(2026).unwrap()
        );
        assert_eq!(cal.year_of(d("2027-03-01")).unwrap().months(), 6);
        assert_eq!(cal.year_of(d("2027-07-01")).unwrap().year(), 2028);
        // 18-month change year: 2027 has no fiscal year of its own.
        let long = FiscalCalendar::new(vec![fy("2027-01-01", "2028-06-30")]).unwrap();
        assert_eq!(
            long.year(2027).unwrap_err().error_code(),
            "FISCAL_YEAR_UNDEFINED"
        );
        assert_eq!(
            long.previous(&long.year(2028).unwrap()).unwrap().year(),
            2026
        );
    }

    #[test]
    fn two_years_ending_in_one_calendar_year_are_refused() {
        let base = FiscalCalendar::new(vec![fy("2026-07-01", "2027-06-30")]).unwrap();
        let err = base
            .with_definition(fy("2027-07-01", "2027-12-31"))
            .unwrap_err();
        assert_eq!(err.error_code(), "INVALID_FISCAL_YEAR");
        assert!(err.to_string().contains("both end in 2027"), "{err}");
        // The 18-month alternative is accepted.
        let changed = base
            .with_definition(fy("2027-07-01", "2028-12-31"))
            .unwrap();
        assert_eq!(
            changed.year_of(d("2029-01-01")).unwrap(),
            FiscalYear::calendar(2029).unwrap()
        );
    }

    #[test]
    fn definitions_must_start_on_a_boundary_and_append() {
        let base = FiscalCalendar::new(vec![fy("2026-07-01", "2027-06-30")]).unwrap();
        assert!(base
            .with_definition(fy("2028-01-01", "2028-12-31"))
            .is_err());
        assert!(base
            .with_definition(fy("2026-01-01", "2026-06-30"))
            .is_err());
        // A later boundary (after one generated year) is fine.
        let later = base
            .with_definition(fy("2028-07-01", "2029-12-31"))
            .unwrap();
        assert_eq!(later.year(2028).unwrap().start(), d("2027-07-01"));
        assert_eq!(later.year(2029).unwrap().months(), 18);
        assert_eq!(later.year(2030).unwrap().start(), d("2030-01-01"));
    }

    #[test]
    fn month_end_dates_resolve_to_the_right_year() {
        let cal = FiscalCalendar::new(vec![fy("2026-02-01", "2027-01-31")]).unwrap();
        for (date, label) in [
            ("2027-01-31", 2027),
            ("2027-02-01", 2028),
            ("2028-01-31", 2028),
            ("2028-02-29", 2029),
            ("2040-01-31", 2040),
        ] {
            assert_eq!(cal.year_of(d(date)).unwrap().year(), label, "{date}");
        }
    }
}
