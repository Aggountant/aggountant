//! Golden tests for the date and time text aggountant writes and reads.
//!
//! Sealed vouchers, fiscal-store and journal timestamps and authority payloads
//! carry these strings, and records already written must keep hashing and
//! parsing. `golden/instants.tsv` was generated with chrono 0.4.45 over 72
//! instants (1900 to 2099, leap days, the Finnish summer-time switches) times 12
//! sub-second values. Its expected strings never change: a mismatch is a format
//! break in the code, not a stale fixture.
//!
//! Only the helpers above `Row` know which library produces the strings.

use aggountant_model::{Date, Instant};

/// Format 1, `+00:00` offset with 0, 3, 6 or 9 fractional digits.
fn format1(secs: i64, nanos: u32) -> String {
    instant(secs, nanos).to_string()
}

/// Format 2, the serde form: as format 1 but with `Z`.
fn format2(secs: i64, nanos: u32) -> String {
    let value = serde_json::to_value(instant(secs, nanos)).unwrap();
    value.as_str().unwrap().to_string()
}

/// Format 3, whole seconds with `Z`.
fn format3(secs: i64, nanos: u32) -> String {
    instant(secs, nanos).to_whole_seconds_z()
}

/// Read an RFC 3339 string back as (epoch seconds, non-negative nanoseconds).
fn parse(text: &str) -> (i64, u32) {
    split(Instant::parse(text).unwrap())
}

fn instant(secs: i64, nanos: u32) -> Instant {
    Instant::from_unix(secs, nanos).unwrap()
}

/// Read back through serde, the path stored records take.
fn parse_serde(text: &str) -> (i64, u32) {
    split(serde_json::from_value(text.into()).unwrap())
}

fn split(t: Instant) -> (i64, u32) {
    let secs = t.unix_seconds();
    let back = Instant::from_unix(secs, 0).unwrap();
    let nanos = t.duration_since(back).as_nanos();
    (secs, u32::try_from(nanos).unwrap())
}

struct Row {
    secs: i64,
    nanos: u32,
    format1: String,
    format2: String,
    format3: String,
}

fn rows() -> Vec<Row> {
    include_str!("golden/instants.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            assert_eq!(f.len(), 5, "{line}");
            Row {
                secs: f[0].parse().unwrap(),
                nanos: f[1].parse().unwrap(),
                format1: f[2].into(),
                format2: f[3].into(),
                format3: f[4].into(),
            }
        })
        .collect()
}

#[test]
fn the_matrix_is_complete() {
    assert_eq!(rows().len(), 72 * 12);
}

#[test]
fn instants_are_written_byte_for_byte_as_before() {
    for r in rows() {
        assert_eq!(format1(r.secs, r.nanos), r.format1, "format 1 {}", r.secs);
        assert_eq!(format2(r.secs, r.nanos), r.format2, "format 2 {}", r.secs);
        assert_eq!(format3(r.secs, r.nanos), r.format3, "format 3 {}", r.secs);
    }
}

#[test]
fn every_written_instant_reads_back_to_the_same_instant() {
    for r in rows() {
        for text in [&r.format1, &r.format2] {
            assert_eq!(parse(text), (r.secs, r.nanos), "{text}");
            assert_eq!(parse_serde(text), (r.secs, r.nanos), "serde {text}");
        }
        assert_eq!(parse(&r.format3), (r.secs, 0), "{}", r.format3);
    }
}

#[test]
fn dates_are_strict_yyyy_mm_dd_both_ways() {
    for text in [
        "1000-01-01",
        "2000-02-29",
        "2026-01-05",
        "2026-12-31",
        "9999-12-31",
    ] {
        let date = Date::parse(text).unwrap();
        assert_eq!(date.to_string(), text);
        assert_eq!(serde_json::to_value(date).unwrap(), text);
        assert_eq!(serde_json::from_value::<Date>(text.into()).unwrap(), date);
    }
    for text in [
        "2026-1-5",
        "2026-01-5",
        "2026-01-05T00:00",
        "2026-01-05 ",
        " 2026-01-05",
        "+2026-01-05",
        "02026-01-05",
        "2026/01/05",
        "20260105",
        "2026-02-29",
        "2026-13-01",
        "2026-00-10",
        "2026-04-31",
        "0999-12-31",
        "",
    ] {
        assert!(Date::parse(text).is_err(), "{text:?} must be rejected");
        assert!(
            serde_json::from_value::<Date>(text.into()).is_err(),
            "serde {text:?} must be rejected"
        );
    }
    assert_eq!(
        Date::from_ymd(2026, 3, 9).unwrap().to_string(),
        "2026-03-09"
    );
    assert!(Date::from_ymd(2026, 2, 29).is_err());
    assert!(Date::from_ymd(10000, 1, 1).is_err());
}
