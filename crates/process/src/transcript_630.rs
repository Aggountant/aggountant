//! Parser for Incomes Register transcript 630, *Maksajan palkkatietojen
//! yhteenveto (csv)* / "Summary of payer's earnings payment data (csv)".
//!
//! The layout comes from *Raportit ja aineistot tulorekisteristä 2026*, version
//! 1.3, §4.6.2 (the Finnish edition; the English edition omits the period,
//! year, month, payment date and reporting-period start columns, and the
//! Swedish one omits a column and the employer's separate report type), as
//! corrected by files ordered from the e-service in November 2026: the
//! e-service names columns 26–29 "… perusteettomaksi eduksi ilmoitetut tulot"
//! where the document says "… perusteettomat edut", and writes the earnings
//! report types in the plural ("Palkkatietoilmoitukset: …") where the document
//! has the singular. The parser takes the e-service header word for word and
//! in that order, and refuses any other header, including the document's
//! wording that no file has been seen to use, rather than guess which column
//! means what. The document does not say
//! what the separator, the encoding, the quoting, or the date and amount
//! formats are. The parser therefore accepts only what it can verify:
//!
//! - the separator is the one (`;`, `,` or tab) that splits the first line into
//!   exactly the documented header;
//! - UTF-8 (with or without a BOM), otherwise ISO-8859-1, and in both cases the
//!   header must still match exactly;
//! - dates `D.M.YYYY` or `YYYY-MM-DD`; amounts with a decimal comma or point,
//!   no thousands grouping;
//! - only Finnish-language transcripts (the language is chosen when ordering).
//!
//! Only the employer's separate report rows are read. Every other row must
//! still carry a known report type and period, so a file of another kind or a
//! changed layout is refused instead of half-read.
use aggountant_domain::payroll::ReportMonth;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

/// The header the e-service writes: §4.6.2's data items in the documented
/// order, with columns 26–29 named as in the real files.
pub(crate) const HEADER_FI: [&str; 33] = [
    "Ilmoituksen tyyppi",
    "Ajanjakso",
    "Aikaväli",
    "Vuosi",
    "Kuukausi",
    "Maksupäivä",
    "Palkkatietoilmoitusten lukumäärä",
    "Suoritukset yhteensä",
    "Työeläkevakuutusmaksun perusteena olevat tulot",
    "Työtapaturma- ja ammattitautivakuutusmaksun perusteena olevat tulot",
    "Työttömyysvakuutusmaksun perusteena olevat tulot",
    "Sairausvakuutusmaksun perusteena olevat tulot",
    "Työntekijän työeläkevakuutusmaksut",
    "Työntekijän työttömyysvakuutusmaksut",
    "Työntekijän sairausvakuutusmaksut",
    "Ennakonpidätykset",
    "Lähdeverot",
    "Perusteettomat edut yhteensä",
    "Takaisinperittyyn määrään kohdistuvat ennakonpidätykset",
    "Takaisinperittyyn määrään kohdistuvat lähdeverot",
    "Takaisinperityt suoritukset yhteensä",
    "Työeläkevakuutusmaksun perusteena olevat takaisinperityt tulot",
    "Työtapaturma- ja ammattitautivakuutusmaksun perusteena olevat takaisinperityt tulot",
    "Työttömyysvakuutusmaksun perusteena olevat takaisinperityt tulot",
    "Sairausvakuutusmaksun perusteena olevat takaisinperityt tulot",
    "Työeläkevakuutusmaksun perusteena olevat perusteettomaksi eduksi ilmoitetut tulot",
    "Työtapaturma- ja ammattitautivakuutusmaksun perusteena olevat perusteettomaksi eduksi ilmoitetut tulot",
    "Työttömyysvakuutusmaksun perusteena olevat perusteettomaksi eduksi ilmoitetut tulot",
    "Sairausvakuutusmaksun perusteena olevat perusteettomaksi eduksi ilmoitetut tulot",
    "Kohdekauden alkupäivä",
    "Työnantajan erillisilmoitusten lukumäärä",
    "Työnantajan sairausvakuutusmaksut",
    "Työnantajan sairausvakuutusmaksusta tehtävät vähennykset",
];
const COL_TYPE: usize = 0;
const COL_PERIOD: usize = 1;
const COL_RANGE: usize = 2;
const COL_YEAR: usize = 3;
const COL_MONTH: usize = 4;
const COL_START: usize = 29;
const COL_COUNT: usize = 30;
const COL_CONTRIBUTION: usize = 31;
const COL_DEDUCTIONS: usize = 32;

/// "Ilmoituksen tyyppi" values (§4.6.2), the earnings types in the plural as
/// the e-service writes them.
const TYPE_EMPLOYER: &str = "Työnantajan erillisilmoitus";
const TYPES_EARNINGS: [&str; 3] = [
    "Palkkatietoilmoitukset: Maksajan omat suoritukset",
    "Palkkatietoilmoitukset: Sijaismaksajan maksamat suoritukset",
    "Palkkatietoilmoitukset: Sijaismaksajana maksetut suoritukset",
];
/// "Ajanjakso" values (§4.6.2).
const PERIOD_RANGE_TOTAL: &str = "Aikaväli yhteensä";
const PERIODS: [&str; 3] = ["Päivä yhteensä", "Kuukausi yhteensä", PERIOD_RANGE_TOTAL];

/// Why a file was refused. `code` is the machine-readable reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TranscriptError {
    pub code: &'static str,
    pub message: String,
}

fn err(code: &'static str, message: impl Into<String>) -> TranscriptError {
    TranscriptError {
        code,
        message: message.into(),
    }
}

/// One reporting period of the employer's separate report section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TranscriptMonth {
    pub month: ReportMonth,
    /// `PayerSummaryReportCount`: valid reports at generation time; a new report
    /// and its replacements count once, cancelled reports not at all.
    pub report_count: u32,
    pub health_insurance_contribution: Decimal,
    pub deductions: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Transcript630 {
    /// The reporting-period range the transcript was ordered for ("Aikaväli").
    pub covers_from: ReportMonth,
    pub covers_to: ReportMonth,
    pub separator: char,
    pub encoding: &'static str,
    /// Employer's separate report periods, ascending, each at most once.
    pub months: Vec<TranscriptMonth>,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Transcript630, TranscriptError> {
    let (text, encoding) = decode(bytes);
    let mut lines = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());
    let Some((_, header)) = lines.next() else {
        return Err(err("TRANSCRIPT_EMPTY", "the file has no lines"));
    };
    let separator = [';', ',', '\t']
        .into_iter()
        .find(|s| {
            split(header, *s)
                .map(|cells| cells.iter().map(|c| c.trim()).eq(HEADER_FI))
                .unwrap_or(false)
        })
        .ok_or_else(|| unknown_header(header))?;

    let mut ranges: Vec<String> = Vec::new();
    let mut months: BTreeMap<ReportMonth, TranscriptMonth> = BTreeMap::new();
    let mut total: Option<(u32, Decimal, Decimal, usize)> = None;
    let mut data_rows = 0usize;
    for (index, line) in lines {
        let line_no = index + 1;
        let cells = split(line, separator)
            .map_err(|m| err("TRANSCRIPT_INVALID_ROW", format!("line {line_no}: {m}")))?;
        let cells: Vec<&str> = cells.iter().map(|c| c.trim()).collect();
        if cells.len() != HEADER_FI.len() {
            return Err(err(
                "TRANSCRIPT_INVALID_ROW",
                format!(
                    "line {line_no} has {} columns, the header has {}",
                    cells.len(),
                    HEADER_FI.len()
                ),
            ));
        }
        data_rows += 1;
        let report_type = cells[COL_TYPE];
        let period = cells[COL_PERIOD];
        if !PERIODS.contains(&period) {
            return Err(err(
                "TRANSCRIPT_INVALID_ROW",
                format!(
                    "line {line_no}: unknown {} {period:?}",
                    HEADER_FI[COL_PERIOD]
                ),
            ));
        }
        if !cells[COL_RANGE].is_empty() && !ranges.iter().any(|r| r == cells[COL_RANGE]) {
            ranges.push(cells[COL_RANGE].to_string());
        }
        if TYPES_EARNINGS.contains(&report_type) {
            continue;
        }
        if report_type != TYPE_EMPLOYER {
            return Err(err(
                "TRANSCRIPT_INVALID_ROW",
                format!(
                    "line {line_no}: unknown {} {report_type:?}",
                    HEADER_FI[COL_TYPE]
                ),
            ));
        }
        let field = |col: usize| (HEADER_FI[col], cells[col]);
        let count = count(field(COL_COUNT), line_no)?;
        let contribution = amount(field(COL_CONTRIBUTION), line_no)?;
        let deductions = amount(field(COL_DEDUCTIONS), line_no)?;
        if period == PERIOD_RANGE_TOTAL {
            if total.is_some() {
                return Err(err(
                    "TRANSCRIPT_INVALID_ROW",
                    format!("line {line_no}: a second range total for employer's separate reports"),
                ));
            }
            total = Some((count, contribution, deductions, line_no));
            continue;
        }
        let month = reporting_month(&cells, line_no)?;
        let row = TranscriptMonth {
            month,
            report_count: count,
            health_insurance_contribution: contribution,
            deductions,
        };
        if months.insert(month, row).is_some() {
            return Err(err(
                "TRANSCRIPT_INVALID_ROW",
                format!("line {line_no}: reporting period {month} appears twice"),
            ));
        }
    }
    if data_rows == 0 {
        return Err(err("TRANSCRIPT_EMPTY", "the file has a header but no rows"));
    }
    let (covers_from, covers_to) = match ranges.as_slice() {
        [one] => parse_range(one)?,
        [] => {
            return Err(err(
                "TRANSCRIPT_RANGE_UNKNOWN",
                format!("no row states the {}", HEADER_FI[COL_RANGE]),
            ))
        }
        many => {
            return Err(err(
                "TRANSCRIPT_RANGE_UNKNOWN",
                format!("rows state different ranges: {many:?}"),
            ))
        }
    };
    if let Some(m) = months.keys().find(|m| **m < covers_from || **m > covers_to) {
        return Err(err(
            "TRANSCRIPT_INVALID_ROW",
            format!(
                "reporting period {m} is outside the transcript's range {covers_from}..{covers_to}"
            ),
        ));
    }
    if let Some((count, contribution, deductions, line_no)) = total {
        let sum_count: u32 = months.values().map(|m| m.report_count).sum();
        let sum_contribution: Decimal = months
            .values()
            .map(|m| m.health_insurance_contribution)
            .sum();
        let sum_deductions: Decimal = months.values().map(|m| m.deductions).sum();
        if (count, contribution, deductions) != (sum_count, sum_contribution, sum_deductions) {
            return Err(err(
                "TRANSCRIPT_TOTAL_MISMATCH",
                format!("line {line_no}: the range total ({count} reports, {contribution}, {deductions}) is not the sum of the reporting periods ({sum_count}, {sum_contribution}, {sum_deductions})"),
            ));
        }
    }
    Ok(Transcript630 {
        covers_from,
        covers_to,
        separator,
        encoding,
        months: months.into_values().collect(),
    })
}

fn decode(bytes: &[u8]) -> (String, &'static str) {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), "utf-8"),
        // ISO-8859-1 maps every byte to the code point of the same value.
        Err(_) => (bytes.iter().map(|b| char::from(*b)).collect(), "iso-8859-1"),
    }
}

fn unknown_header(header: &str) -> TranscriptError {
    let shown: String = header.chars().take(300).collect();
    let hint = if header.contains("Type of report") || header.contains("Typ av anmälan") {
        " The transcript is not in Finnish: order transcript 630 again with Finnish (suomi) as its language."
    } else {
        ""
    };
    err(
        "TRANSCRIPT_UNKNOWN_HEADER",
        format!(
            "the first line is not the header of a Finnish transcript 630 from the Incomes Register e-service ({} columns starting {:?}); found {shown:?}.{hint}",
            HEADER_FI.len(),
            HEADER_FI[0]
        ),
    )
}

/// Split one CSV line. A field may be enclosed in double quotes, with `""` for
/// a quote inside it.
fn split(line: &str, separator: char) -> Result<Vec<String>, String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    let mut at_start = true;
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                    match chars.peek() {
                        None => {}
                        Some(n) if *n == separator => {}
                        Some(_) => return Err("text after a closing quote".into()),
                    }
                }
            } else {
                cell.push(c);
            }
            continue;
        }
        if c == separator {
            cells.push(std::mem::take(&mut cell));
            at_start = true;
            continue;
        }
        if c == '"' && at_start && cell.trim().is_empty() {
            cell.clear();
            quoted = true;
            at_start = false;
            continue;
        }
        if c == '"' {
            return Err("a quote inside an unquoted field".into());
        }
        at_start = false;
        cell.push(c);
    }
    if quoted {
        return Err("an unterminated quote".into());
    }
    cells.push(cell);
    Ok(cells)
}

fn count((name, value): (&str, &str), line_no: usize) -> Result<u32, TranscriptError> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err(
            "TRANSCRIPT_INVALID_ROW",
            format!("line {line_no}: {name} must be a whole number, got {value:?}"),
        ));
    }
    value.parse().map_err(|_| {
        err(
            "TRANSCRIPT_INVALID_ROW",
            format!("line {line_no}: {name} is too large: {value:?}"),
        )
    })
}

/// An amount with an optional minus sign, a decimal comma or point and at most
/// two decimals. An empty cell is zero: a month with only "no wages payable"
/// reports has no contribution to sum.
fn amount((name, value): (&str, &str), line_no: usize) -> Result<Decimal, TranscriptError> {
    if value.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let bad = || {
        err(
            "TRANSCRIPT_INVALID_ROW",
            format!("line {line_no}: {name} is not an amount like 123,45: {value:?}"),
        )
    };
    let digits = value.strip_prefix('-').unwrap_or(value);
    let (whole, fraction) = match digits.split_once([',', '.']) {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(whole) || fraction.is_some_and(|f| !all_digits(f) || f.len() > 2) {
        return Err(bad());
    }
    Decimal::from_str(&value.replace(',', ".")).map_err(|_| bad())
}

fn date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .filter(|_| value.len() == 10)
        .or_else(|| {
            let mut parts = value.split('.');
            let (d, m, y) = (parts.next()?, parts.next()?, parts.next()?);
            let short = |s: &str, max| {
                (1..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
            };
            if parts.next().is_some()
                || !short(d, 2)
                || !short(m, 2)
                || !short(y, 4)
                || y.len() != 4
            {
                return None;
            }
            NaiveDate::from_ymd_opt(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
        })
}

fn reporting_month(cells: &[&str], line_no: usize) -> Result<ReportMonth, TranscriptError> {
    let value = cells[COL_START];
    let start = date(value).filter(|d| d.day() == 1).ok_or_else(|| {
        err(
            "TRANSCRIPT_INVALID_ROW",
            format!(
                "line {line_no}: {} must be the first day of a month (D.M.YYYY or YYYY-MM-DD), got {value:?}",
                HEADER_FI[COL_START]
            ),
        )
    })?;
    for (col, expected) in [(COL_YEAR, start.year() as u32), (COL_MONTH, start.month())] {
        let cell = cells[col];
        if !cell.is_empty() && cell.parse::<u32>().ok() != Some(expected) {
            return Err(err(
                "TRANSCRIPT_INVALID_ROW",
                format!(
                    "line {line_no}: {} {cell:?} disagrees with {} {value:?}",
                    HEADER_FI[col], HEADER_FI[COL_START]
                ),
            ));
        }
    }
    ReportMonth::parse(&format!("{:04}-{:02}", start.year(), start.month()))
        .map_err(|e| err("TRANSCRIPT_INVALID_ROW", format!("line {line_no}: {e}")))
}

/// "Aikaväli": two dates, `D.M.YYYY` or `YYYY-MM-DD`, separated by a dash with
/// optional spaces. Nothing else is accepted.
fn parse_range(value: &str) -> Result<(ReportMonth, ReportMonth), TranscriptError> {
    let bad = || {
        err(
            "TRANSCRIPT_RANGE_UNKNOWN",
            format!(
                "{} {value:?} is not two dates (D.M.YYYY or YYYY-MM-DD) separated by a dash",
                HEADER_FI[COL_RANGE]
            ),
        )
    };
    let candidates = [" - ", " – ", "–", " — ", "—", "-"];
    let (from, to) = candidates
        .iter()
        .find_map(|sep| {
            let parts: Vec<&str> = value.split(sep).collect();
            match parts.as_slice() {
                [a, b] => Some((date(a.trim())?, date(b.trim())?)),
                _ => None,
            }
        })
        .ok_or_else(bad)?;
    if from > to {
        return Err(bad());
    }
    let month = |d: NaiveDate| ReportMonth::parse(&format!("{:04}-{:02}", d.year(), d.month()));
    Ok((
        month(from).map_err(|_| bad())?,
        month(to).map_err(|_| bad())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(sep: &str) -> String {
        HEADER_FI.join(sep)
    }
    fn row(sep: &str, cells: &[(usize, &str)]) -> String {
        let mut all = vec![""; HEADER_FI.len()];
        for (i, v) in cells {
            all[*i] = v;
        }
        all.join(sep)
    }
    fn employer(sep: &str, start: &str, count: &str, contribution: &str) -> String {
        row(
            sep,
            &[
                (COL_TYPE, TYPE_EMPLOYER),
                (COL_PERIOD, "Kuukausi yhteensä"),
                (COL_RANGE, "1.1.2025 - 31.12.2025"),
                (COL_START, start),
                (COL_COUNT, count),
                (COL_CONTRIBUTION, contribution),
                (COL_DEDUCTIONS, "0"),
            ],
        )
    }

    #[test]
    fn reads_employer_rows_and_skips_earnings_rows() {
        let csv = [
            header(";"),
            row(
                ";",
                &[
                    (COL_TYPE, TYPES_EARNINGS[0]),
                    (COL_PERIOD, "Päivä yhteensä"),
                    (COL_RANGE, "1.1.2025 - 31.12.2025"),
                    (5, "15.3.2025"),
                    (6, "1"),
                ],
            ),
            employer(";", "1.2.2025", "1", "0,00"),
            employer(";", "2025-03-01", "1", "12,34"),
            employer(";", "1.4.2025", "0", ""),
            row(
                ";",
                &[
                    (COL_TYPE, TYPE_EMPLOYER),
                    (COL_PERIOD, PERIOD_RANGE_TOTAL),
                    (COL_RANGE, "1.1.2025 - 31.12.2025"),
                    (COL_COUNT, "2"),
                    (COL_CONTRIBUTION, "12,34"),
                    (COL_DEDUCTIONS, "0"),
                ],
            ),
        ]
        .join("\r\n");
        let t = parse(csv.as_bytes()).unwrap();
        assert_eq!(t.separator, ';');
        assert_eq!(t.covers_from.to_string(), "2025-01");
        assert_eq!(t.covers_to.to_string(), "2025-12");
        assert_eq!(t.months.len(), 3);
        assert_eq!(
            t.months[1].health_insurance_contribution,
            Decimal::new(1234, 2)
        );
        assert_eq!(t.months[2].report_count, 0);
    }

    #[test]
    fn latin1_with_quoted_comma_separated_cells() {
        let csv = format!(
            "{}\n{}\n",
            header(","),
            employer(",", "1.2.2025", "1", "\"5,00\"")
        );
        let latin1: Vec<u8> = csv.chars().map(|c| c as u32 as u8).collect();
        let t = parse(&latin1).unwrap();
        assert_eq!(t.encoding, "iso-8859-1");
        assert_eq!(
            t.months[0].health_insurance_contribution,
            Decimal::new(500, 2)
        );
    }

    #[test]
    fn refuses_what_it_cannot_verify() {
        let ok_row = employer(";", "1.2.2025", "1", "0,00");
        let cases: Vec<(String, &str)> = vec![
            (String::new(), "TRANSCRIPT_EMPTY"),
            (header(";"), "TRANSCRIPT_EMPTY"),
            (
                format!("Summary of payer's earnings payment data\n{}", header(";")),
                "TRANSCRIPT_UNKNOWN_HEADER",
            ),
            (
                header(";").replace("Ajanjakso", "Jakso"),
                "TRANSCRIPT_UNKNOWN_HEADER",
            ),
            (
                format!("{};Extra\n{ok_row};", header(";")),
                "TRANSCRIPT_UNKNOWN_HEADER",
            ),
            (
                format!("{}\n{}", header(";"), employer(";", "2.2.2025", "1", "0")),
                "TRANSCRIPT_INVALID_ROW",
            ),
            (
                format!("{}\n{}", header(";"), employer(";", "1.2.2025", "x", "0")),
                "TRANSCRIPT_INVALID_ROW",
            ),
            (
                format!(
                    "{}\n{}",
                    header(";"),
                    employer(";", "1.2.2025", "1", "1 234,00")
                ),
                "TRANSCRIPT_INVALID_ROW",
            ),
            (
                format!("{}\n{ok_row}\n{ok_row}", header(";")),
                "TRANSCRIPT_INVALID_ROW",
            ),
            (
                format!("{}\n{}", header(";"), employer(";", "1.2.2026", "1", "0")),
                "TRANSCRIPT_INVALID_ROW",
            ),
            (
                format!(
                    "{}\n{}",
                    header(";"),
                    ok_row.replace(TYPE_EMPLOYER, "Muu ilmoitus")
                ),
                "TRANSCRIPT_INVALID_ROW",
            ),
            (
                format!(
                    "{}\n{}",
                    header(";"),
                    ok_row.replace("1.1.2025 - 31.12.2025", "2025")
                ),
                "TRANSCRIPT_RANGE_UNKNOWN",
            ),
            (
                format!("{}\n{ok_row};", header(";")),
                "TRANSCRIPT_INVALID_ROW",
            ),
        ];
        for (csv, code) in cases {
            assert_eq!(parse(csv.as_bytes()).unwrap_err().code, code, "{csv}");
        }
        let total = row(
            ";",
            &[
                (COL_TYPE, TYPE_EMPLOYER),
                (COL_PERIOD, PERIOD_RANGE_TOTAL),
                (COL_RANGE, "1.1.2025 - 31.12.2025"),
                (COL_COUNT, "2"),
            ],
        );
        let csv = format!("{}\n{ok_row}\n{total}", header(";"));
        assert_eq!(
            parse(csv.as_bytes()).unwrap_err().code,
            "TRANSCRIPT_TOTAL_MISMATCH"
        );
    }

    #[test]
    fn real_e_service_file_parses() {
        let bytes = include_bytes!("../tests/fixtures/employer_report/transcript-630-2025.csv");
        let t = parse(bytes).unwrap();
        assert_eq!(t.encoding, "utf-8");
        assert_eq!(t.months.len(), 12);
        // July: one report, the contribution cell left empty (only tax-free
        // compensations were paid), read as zero.
        let july = &t.months[6];
        assert_eq!(july.month.to_string(), "2025-07");
        assert_eq!(july.report_count, 1);
        assert_eq!(july.health_insurance_contribution, Decimal::ZERO);
        assert_eq!(july.deductions, Decimal::ZERO);
    }

    #[test]
    fn the_documents_wording_is_not_the_e_service_header() {
        // §4.6.2 names columns 26-29 "... perusteettomat edut" and the earnings
        // type in the singular; the e-service writes neither, so both are refused.
        let documented = header(";").replace(
            "perusteettomaksi eduksi ilmoitetut tulot",
            "perusteettomat edut",
        );
        let ok_row = employer(";", "1.2.2025", "1", "0,00");
        let csv = format!("{documented}\n{ok_row}");
        assert_eq!(
            parse(csv.as_bytes()).unwrap_err().code,
            "TRANSCRIPT_UNKNOWN_HEADER"
        );
        let singular = row(
            ";",
            &[
                (COL_TYPE, "Palkkatietoilmoitus: Maksajan omat suoritukset"),
                (COL_PERIOD, "Kuukausi yhteensä"),
                (COL_RANGE, "1.1.2025 - 31.12.2025"),
            ],
        );
        let csv = format!("{}\n{singular}\n{ok_row}", header(";"));
        assert_eq!(
            parse(csv.as_bytes()).unwrap_err().code,
            "TRANSCRIPT_INVALID_ROW"
        );
    }

    #[test]
    fn english_transcript_is_told_to_order_in_finnish() {
        let csv = "Type of report;Period;Time range\n";
        let e = parse(csv.as_bytes()).unwrap_err();
        assert_eq!(e.code, "TRANSCRIPT_UNKNOWN_HEADER");
        assert!(e.message.contains("Finnish"));
    }
}
