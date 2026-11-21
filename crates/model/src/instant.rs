//! A UTC instant and the RFC 3339 text aggountant writes for it.
//!
//! Every stored or sent timestamp goes through this type: `sealed_at`, journal
//! and fiscal-store `ts`, the outbox, authority payloads and evidence records.
//! Its forms are the ones the books already hold, so they never change:
//!
//! 1. `Display`: `2026-09-13T12:00:00.123+00:00`;
//! 2. serde: the same with `Z` instead of `+00:00`;
//! 3. [`Instant::to_whole_seconds_z`]: `2026-09-13T12:00:00Z`, truncated.
//!
//! Forms 1 and 2 carry 0, 3, 6 or 9 fractional digits: the fewest of those that
//! hold the instant exactly. The date-time library's own output uses the
//! fewest digits of any count and must never reach a stored value; a sealed
//! record's text is hashed as written.

use std::fmt;
use std::str::FromStr;

use jiff::tz::Offset;
use jiff::{civil, SignedDuration, Timestamp};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ModelError;

/// A point in time, kept to the nanosecond and written in UTC.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Instant(Timestamp);

impl Instant {
    /// The current time from the system clock.
    pub fn now() -> Self {
        Instant(Timestamp::now())
    }

    /// `secs` seconds and `nanos` (below one billion) after the Unix epoch;
    /// `None` when out of range.
    pub fn from_unix(secs: i64, nanos: u32) -> Option<Self> {
        if nanos >= 1_000_000_000 {
            return None;
        }
        let whole = Timestamp::from_second(secs).ok()?;
        whole
            .checked_add(SignedDuration::from_nanos(i64::from(nanos)))
            .ok()
            .map(Instant)
    }

    /// Read an RFC 3339 instant with any offset, as UTC.
    pub fn parse(text: &str) -> Result<Self, ModelError> {
        text.parse::<Timestamp>().map(Instant).map_err(|e| {
            ModelError::InvalidDate(format!("{text:?} is not an RFC 3339 instant ({e})"))
        })
    }

    /// Whole seconds since the Unix epoch, rounded towards the past.
    pub fn unix_seconds(&self) -> i64 {
        let secs = self.0.as_second();
        if self.0.subsec_nanosecond() < 0 {
            secs - 1
        } else {
            secs
        }
    }

    /// The date-time library's value, for arithmetic.
    pub fn inner(&self) -> Timestamp {
        self.0
    }

    /// The civil date and time in UTC.
    pub fn utc(&self) -> civil::DateTime {
        Offset::UTC.to_datetime(self.0)
    }

    /// The calendar date in UTC.
    pub fn utc_date(&self) -> civil::Date {
        self.utc().date()
    }

    /// This instant moved by `duration`; `None` when out of range.
    pub fn checked_add(&self, duration: SignedDuration) -> Option<Self> {
        self.0.checked_add(duration).ok().map(Instant)
    }

    /// The signed time from `earlier` to this instant.
    pub fn duration_since(&self, earlier: Instant) -> SignedDuration {
        self.0.duration_since(earlier.0)
    }

    /// Form 3: whole seconds, truncated, with `Z`.
    pub fn to_whole_seconds_z(&self) -> String {
        let mut out = String::with_capacity(20);
        self.write_seconds(&mut out).expect("writing to a String");
        out.push('Z');
        out
    }

    fn write_seconds(&self, f: &mut impl fmt::Write) -> fmt::Result {
        let t = self.utc();
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            t.year(),
            t.month(),
            t.day(),
            t.hour(),
            t.minute(),
            t.second()
        )
    }

    fn write_rfc3339(&self, f: &mut impl fmt::Write, zone: &str) -> fmt::Result {
        self.write_seconds(f)?;
        let nanos = self.utc().subsec_nanosecond();
        if nanos == 0 {
        } else if nanos % 1_000_000 == 0 {
            write!(f, ".{:03}", nanos / 1_000_000)?;
        } else if nanos % 1_000 == 0 {
            write!(f, ".{:06}", nanos / 1_000)?;
        } else {
            write!(f, ".{nanos:09}")?;
        }
        f.write_str(zone)
    }
}

impl From<Timestamp> for Instant {
    fn from(timestamp: Timestamp) -> Self {
        Instant(timestamp)
    }
}

/// Form 1, with the `+00:00` offset.
impl fmt::Display for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_rfc3339(f, "+00:00")
    }
}

/// The serde form, so that `{:?}` and stored JSON read the same.
impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_rfc3339(f, "Z")
    }
}

impl FromStr for Instant {
    type Err = ModelError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Instant::parse(text)
    }
}

/// Form 2, with `Z`.
impl Serialize for Instant {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut out = String::with_capacity(30);
        self.write_rfc3339(&mut out, "Z")
            .expect("writing to a String");
        s.serialize_str(&out)
    }
}

impl<'de> Deserialize<'de> for Instant {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Instant::parse(&String::deserialize(d)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn before_the_epoch_counts_down_from_the_whole_second() {
        let t = Instant::from_unix(-1, 1).unwrap();
        assert_eq!(t.to_string(), "1969-12-31T23:59:59.000000001+00:00");
        assert_eq!(t.unix_seconds(), -1);
        assert_eq!(t.to_whole_seconds_z(), "1969-12-31T23:59:59Z");
        assert!(Instant::from_unix(0, 1_000_000_000).is_none());
    }

    #[test]
    fn any_offset_reads_as_utc() {
        let t = Instant::parse("2026-09-13T15:00:00.5+03:00").unwrap();
        assert_eq!(t.to_string(), "2026-09-13T12:00:00.500+00:00");
        assert_eq!(format!("{t:?}"), "2026-09-13T12:00:00.500Z");
        assert!(Instant::parse("2026-09-13").is_err());
        assert!(Instant::parse("2026-09-13T12:00:00").is_err());
    }
}
