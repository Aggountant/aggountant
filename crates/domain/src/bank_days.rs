//! Finnish bank days.
//!
//! A bank day is Monday to Friday, excluding the Finnish bank holidays: New
//! Year's Day, Epiphany, Good Friday, Easter Monday, 1 May, Ascension Day,
//! Midsummer Eve, Independence Day, Christmas Eve, Christmas Day and St
//! Stephen's Day. Easter follows the Gregorian computus, so Good Friday, Easter
//! Monday and Ascension Day move with it; Midsummer Eve is the Friday between
//! 19 and 25 June.
//!
//! Dates are Finnish calendar dates. [`finnish_date`] turns an instant into
//! one, so a check made late in the evening UTC counts on the next Finnish day;
//! [`one_bank_day_after`] is the earliest instant a full bank day later.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};

/// Easter Sunday of `year` in the Gregorian calendar (the anonymous computus).
pub fn easter_sunday(year: i32) -> NaiveDate {
    let a = year.rem_euclid(19);
    let b = year.div_euclid(100);
    let c = year.rem_euclid(100);
    let d = b.div_euclid(4);
    let e = b.rem_euclid(4);
    let f = (b + 8).div_euclid(25);
    let g = (b - f + 1).div_euclid(3);
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let i = c.div_euclid(4);
    let k = c.rem_euclid(4);
    let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
    let m = (a + 11 * h + 22 * l).div_euclid(451);
    let month = (h + l - 7 * m + 114).div_euclid(31);
    let day = (h + l - 7 * m + 114).rem_euclid(31) + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32)
        .expect("the computus yields a date in March or April")
}

/// Whether `date` is a Finnish bank holiday that can fall on a weekday.
pub fn is_finnish_bank_holiday(date: NaiveDate) -> bool {
    let easter = easter_sunday(date.year());
    let moving = [
        easter - Duration::days(2),  // Good Friday
        easter + Duration::days(1),  // Easter Monday
        easter + Duration::days(39), // Ascension Day
    ];
    if moving.contains(&date) {
        return true;
    }
    let midsummer_eve =
        date.month() == 6 && (19..=25).contains(&date.day()) && date.weekday() == Weekday::Fri;
    midsummer_eve
        || matches!(
            (date.month(), date.day()),
            (1, 1) | (1, 6) | (5, 1) | (12, 6) | (12, 24) | (12, 25) | (12, 26)
        )
}

/// Whether `date` is a Finnish bank day.
pub fn is_finnish_bank_day(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun) && !is_finnish_bank_holiday(date)
}

/// The first bank day strictly after `date`.
pub fn next_finnish_bank_day(date: NaiveDate) -> NaiveDate {
    let mut next = date + Duration::days(1);
    while !is_finnish_bank_day(next) {
        next += Duration::days(1);
    }
    next
}

/// The earliest instant one bank day after `instant`: the same Finnish clock time
/// on the next bank day, and never before that day begins. A check at 23:59 on a
/// Monday is followed by one at 23:59 on Tuesday, not at midnight; one on a
/// Friday by one at the same time on Monday. The days in between are counted as
/// whole 24-hour days, so across a summer-time switch the clock time can differ
/// by an hour.
pub fn one_bank_day_after(instant: DateTime<Utc>) -> DateTime<Utc> {
    let date = finnish_date(instant);
    let next = next_finnish_bank_day(date);
    let same_time = instant + Duration::days((next - date).num_days());
    same_time.max(start_of_finnish_day(next))
}

/// Finnish local time of an instant. Finland keeps UTC+2, and UTC+3 from the last
/// Sunday of March to the last Sunday of October, switching at 01:00 UTC (the EU
/// summer-time rule).
pub fn finnish_local_time(instant: DateTime<Utc>) -> NaiveDateTime {
    let year = instant.year();
    let switch = |month: u32| {
        let last_day = NaiveDate::from_ymd_opt(year, month + 1, 1)
            .expect("March and October are followed by a month")
            .pred_opt()
            .expect("a month has a last day");
        let sunday =
            last_day - Duration::days(i64::from(last_day.weekday().num_days_from_sunday()));
        Utc.from_utc_datetime(&sunday.and_hms_opt(1, 0, 0).expect("01:00 exists"))
    };
    let summer = switch(3) <= instant && instant < switch(10);
    let offset = Duration::hours(if summer { 3 } else { 2 });
    (instant + offset).naive_utc()
}

/// The Finnish calendar date of an instant.
pub fn finnish_date(instant: DateTime<Utc>) -> NaiveDate {
    finnish_local_time(instant).date()
}

/// The first instant of a Finnish calendar date.
fn start_of_finnish_day(date: NaiveDate) -> DateTime<Utc> {
    let midnight = date.and_hms_opt(0, 0, 0).expect("midnight exists");
    [3, 2]
        .into_iter()
        .map(|offset| Utc.from_utc_datetime(&(midnight - Duration::hours(offset))))
        .find(|candidate| finnish_date(*candidate) == date)
        .expect("a Finnish date starts at UTC+3 or UTC+2")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    fn utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value).unwrap().to_utc()
    }

    #[test]
    fn easter_matches_known_years() {
        for (year, easter) in [
            (2024, "2024-03-31"),
            (2025, "2025-04-20"),
            (2026, "2026-04-05"),
            (2027, "2027-03-28"),
            (2038, "2038-04-25"),
            (2285, "2285-03-22"),
        ] {
            assert_eq!(easter_sunday(year), d(easter), "{year}");
        }
    }

    #[test]
    fn easter_dependent_holidays_are_not_bank_days() {
        // 2026: Easter Sunday 5 April.
        for holiday in ["2026-04-03", "2026-04-06", "2026-05-14"] {
            assert!(is_finnish_bank_holiday(d(holiday)), "{holiday}");
            assert!(!is_finnish_bank_day(d(holiday)), "{holiday}");
        }
        // 2025: Easter Sunday 20 April.
        for holiday in ["2025-04-18", "2025-04-21", "2025-05-29"] {
            assert!(!is_finnish_bank_day(d(holiday)), "{holiday}");
        }
        // Maundy Thursday, the Tuesday after Easter and the day after Ascension
        // are ordinary bank days.
        for day in ["2026-04-02", "2026-04-07", "2026-05-15"] {
            assert!(is_finnish_bank_day(d(day)), "{day}");
        }
        // Over Easter the next bank day after Maundy Thursday is the Tuesday.
        assert_eq!(next_finnish_bank_day(d("2026-04-02")), d("2026-04-07"));
    }

    #[test]
    fn midsummer_eve_is_the_friday_between_19_and_25_june() {
        for eve in ["2026-06-19", "2027-06-25", "2025-06-20", "2029-06-22"] {
            assert!(!is_finnish_bank_day(d(eve)), "{eve}");
        }
        // The Friday a week before, and Midsummer Day itself on a Saturday.
        assert!(is_finnish_bank_day(d("2026-06-12")));
        assert!(!is_finnish_bank_holiday(d("2026-06-26")));
        assert!(is_finnish_bank_day(d("2027-06-18")));
        // Thursday before Midsummer Eve: the next bank day is Monday.
        assert_eq!(next_finnish_bank_day(d("2026-06-18")), d("2026-06-22"));
    }

    #[test]
    fn fixed_holidays_and_year_boundary() {
        for holiday in [
            "2026-01-01",
            "2026-01-06",
            "2026-05-01",
            "2026-12-24",
            "2026-12-25",
            "2027-12-06",
        ] {
            assert!(!is_finnish_bank_day(d(holiday)), "{holiday}");
        }
        // Christmas 2026: Thursday 24th to Saturday 26th. From Wednesday 23rd the
        // next bank day is Monday 28th; from Thursday 31st it is Monday 4 January
        // (1 January is a holiday, 2–3 a weekend).
        assert_eq!(next_finnish_bank_day(d("2026-12-23")), d("2026-12-28"));
        assert_eq!(next_finnish_bank_day(d("2026-12-31")), d("2027-01-04"));
        // 2027: Epiphany on Wednesday.
        assert_eq!(next_finnish_bank_day(d("2027-01-05")), d("2027-01-07"));
    }

    #[test]
    fn one_bank_day_after_is_the_same_time_on_the_next_bank_day() {
        // Times are Finnish: UTC+2 in winter, UTC+3 in summer.
        for (from, earliest) in [
            // A weekday: the next day at the same time, not at midnight.
            ("2026-11-16T21:59:00Z", "2026-11-17T21:59:00Z"),
            // Friday and Saturday: Monday.
            ("2026-11-20T08:00:00Z", "2026-11-23T08:00:00Z"),
            ("2026-11-21T08:00:00Z", "2026-11-23T08:00:00Z"),
            // Maundy Thursday: over Good Friday and Easter Monday to Tuesday.
            ("2026-04-02T07:00:00Z", "2026-04-07T07:00:00Z"),
            // Thursday before Midsummer Eve: Monday.
            ("2026-06-18T07:00:00Z", "2026-06-22T07:00:00Z"),
            // New Year's Eve: over New Year's Day and the weekend to 4 January.
            ("2026-12-31T08:00:00Z", "2027-01-04T08:00:00Z"),
        ] {
            assert_eq!(one_bank_day_after(utc(from)), utc(earliest), "{from}");
        }
        // Friday 00:30 before the October switch: 72 hours later is still Sunday
        // in Finland, so the next bank day's start is the floor.
        assert_eq!(
            one_bank_day_after(utc("2026-10-22T21:30:00Z")),
            utc("2026-10-25T22:00:00Z")
        );
        assert_eq!(finnish_date(utc("2026-10-25T22:00:00Z")), d("2026-10-26"));
    }

    /// Finnish local time as messages and notes write it (`%Y-%m-%d %H:%M`),
    /// pinned around the summer-time switches, a leap day and the year ends.
    #[test]
    fn finnish_local_time_is_written_as_before() {
        for (instant, local) in [
            ("2026-03-29T00:59:59.999999999Z", "2026-03-29 02:59"),
            ("2026-03-29T01:00:00Z", "2026-03-29 04:00"),
            ("2026-10-25T00:59:59Z", "2026-10-25 03:59"),
            ("2026-10-25T01:00:00Z", "2026-10-25 03:00"),
            ("2024-02-28T22:00:00.120Z", "2024-02-29 00:00"),
            ("2024-03-31T00:30:00Z", "2024-03-31 02:30"),
            ("2024-03-31T01:30:00Z", "2024-03-31 04:30"),
            ("2024-10-27T00:30:00Z", "2024-10-27 03:30"),
            ("2024-10-27T01:30:00Z", "2024-10-27 03:30"),
            ("2026-12-31T21:59:00Z", "2026-12-31 23:59"),
            ("2026-12-31T22:00:00Z", "2027-01-01 00:00"),
            ("2037-10-25T00:59:00Z", "2037-10-25 03:59"),
            ("2000-01-01T00:00:00+02:00", "2000-01-01 00:00"),
        ] {
            assert_eq!(
                finnish_local_time(utc(instant))
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
                local,
                "{instant}"
            );
        }
    }

    #[test]
    fn finnish_date_follows_the_local_offset() {
        // Winter (UTC+2): 22:30 UTC is already the next day in Finland.
        assert_eq!(finnish_date(utc("2026-11-16T21:59:59Z")), d("2026-11-16"));
        assert_eq!(finnish_date(utc("2026-11-16T22:00:00Z")), d("2026-11-17"));
        // Summer (UTC+3).
        assert_eq!(finnish_date(utc("2026-06-18T20:59:59Z")), d("2026-06-18"));
        assert_eq!(finnish_date(utc("2026-06-18T21:00:00Z")), d("2026-06-19"));
        // The switches in 2026: 29 March and 25 October at 01:00 UTC.
        assert_eq!(finnish_date(utc("2026-03-29T00:59:00Z")), d("2026-03-29"));
        assert_eq!(finnish_date(utc("2026-03-28T21:30:00Z")), d("2026-03-28"));
        assert_eq!(finnish_date(utc("2026-03-29T21:30:00Z")), d("2026-03-30"));
        assert_eq!(finnish_date(utc("2026-10-25T21:30:00Z")), d("2026-10-25"));
        // New Year's Eve in UTC is New Year's Day in Finland.
        assert_eq!(finnish_date(utc("2026-12-31T22:30:00Z")), d("2027-01-01"));
    }
}
