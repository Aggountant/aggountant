use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Subcommand, ValueEnum};
use serde_json::{json, Value};

use aggountant_ledger::{render_monthly_report_html, Ledger, LedgerError, MonthlyReportOptions};
use aggountant_model::{Date, PeriodId};

use crate::CliError;

#[derive(Subcommand, Debug)]
pub(crate) enum ReportCmd {
    /// Produce month/YTD income and month-end balance from sealed ledger data.
    #[command(after_help = STATEMENTS_HELP)]
    Statements {
        /// Calendar month in strict YYYY-MM form.
        #[arg(long)]
        period: String,
        /// Company display name printed in the report.
        #[arg(long)]
        company: String,
        /// Output representation. Required with an output file; defaults to
        /// native-json when the report goes to stdout.
        #[arg(long, value_enum)]
        format: Option<ReportFormat>,
        /// New output file, or `-` for stdout (the default). Existing files are
        /// never overwritten.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Optional date from which the caller states history is available; not a calculation cutoff or verification.
        #[arg(long)]
        history_start: Option<String>,
        /// Optional factual limitation shown in native and rendered output.
        #[arg(long)]
        limitation: Option<String>,
    },
}

const STATEMENTS_HELP: &str = r#"OUTPUT
  Without --output (or with --output -) the report goes to stdout, native-json
  unless --format says otherwise. In text mode stdout is the report document
  itself; with --json it is the usual summary object with the document's exact
  text in "content" and "output":"-". With --output FILE, --format is
  required, the file must not exist, and stdout carries only the summary.

NEW FISCAL YEAR
  A month of a fiscal year is refused with PERIOD_NOT_SEALED until the
  previous fiscal year is sealed by its year-end close (`close-year`): the
  balance sheet needs that year's closing balances as its opening position.
  The new year's months can be recorded, closed and VAT-filed meanwhile.
  A year with no earlier activity in aggountant does not wait, nor does one
  whose previous year holds only opening-balance vouchers (balance-sheet
  accounts, dated that year's last day) and no period ever operated here.

EXAMPLES
  Quick look at a month:
    aggountant report statements --period 2026-03 --company "Esimerkki Oy"
  Write an HTML file:
    aggountant --json report statements --period 2026-03 --company "Esimerkki Oy" \
      --format html --output 2026-03.html"#;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum ReportFormat {
    NativeJson,
    Html,
}

impl ReportFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::NativeJson => "native-json",
            Self::Html => "html",
        }
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                CliError::user(
                    "OUTPUT_EXISTS",
                    format!("refusing to overwrite existing output {}", path.display()),
                )
            } else {
                CliError::system(
                    "OUTPUT_WRITE_ERROR",
                    format!("could not create output {}: {error}", path.display()),
                )
            }
        })?;
    file.write_all(bytes).map_err(|error| {
        CliError::system(
            "OUTPUT_WRITE_ERROR",
            format!("could not write output {}: {error}", path.display()),
        )
    })
}

/// The report's only `PERIOD_NOT_SEALED` is the ledger's opening-balance gate:
/// a month of a fiscal year whose predecessor is not sealed yet. Users meet it
/// every January, so the refusal says why and what lifts it, in front of the
/// ledger's own reason.
fn opening_not_sealed(period: &PeriodId, reason: &str) -> CliError {
    CliError::user(
        "PERIOD_NOT_SEALED",
        format!(
            "PERIOD_NOT_SEALED: report statements for {period} needs its fiscal year's opening \
             balances, and they exist only once the previous fiscal year is sealed by its \
             year-end close ({reason}). Way out: finish the previous year's `close-year` run \
             (its last approval, ApproveStatements, seals the year and cannot be undone), then \
             make the report again. Nothing is wrong with the books meanwhile: the new year's \
             months can still be recorded, closed with `close-period` and their VAT filed; \
             only this report waits."
        ),
    )
}

pub(crate) fn run(cmd: &ReportCmd, home: &Path) -> Result<Value, CliError> {
    match cmd {
        ReportCmd::Statements {
            period,
            company,
            format,
            output,
            history_start,
            limitation,
        } => {
            let period = PeriodId::parse(period)
                .map_err(|error| CliError::user(error.error_code(), error.to_string()))?;
            if !period.is_month() {
                return Err(CliError::user(
                    "INVALID_PERIOD_ID",
                    format!("report statements requires YYYY-MM, got {period}"),
                ));
            }
            if company.trim().is_empty() {
                return Err(CliError::user(
                    "INVALID_COMPANY",
                    format!("--company must not be empty, got {company:?}"),
                ));
            }
            if limitation
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(CliError::user(
                    "INVALID_LIMITATION",
                    "--limitation must not be empty or whitespace-only",
                ));
            }
            let history_start = history_start
                .as_deref()
                .map(Date::parse)
                .transpose()
                .map_err(|error| CliError::user(error.error_code(), error.to_string()))?;
            let output = output.as_deref().filter(|path| *path != Path::new("-"));
            let format = match (format, output) {
                (Some(format), _) => *format,
                (None, None) => ReportFormat::NativeJson,
                (None, Some(path)) => {
                    return Err(CliError::user(
                        "FORMAT_REQUIRED",
                        format!(
                            "--format is required with --output {}; omit --output to print native-json to stdout",
                            path.display()
                        ),
                    ))
                }
            };
            let ledger = Ledger::open(home)?;
            let report = ledger
                .monthly_statements_report(
                    &period,
                    company,
                    MonthlyReportOptions {
                        history_start,
                        limitation: limitation.clone(),
                    },
                )
                .map_err(|error| match error {
                    LedgerError::PeriodNotSealed(reason) => opening_not_sealed(&period, &reason),
                    other => other.into(),
                })?;
            let bytes = match format {
                ReportFormat::NativeJson => {
                    let mut bytes = serde_json::to_vec_pretty(&report).map_err(|error| {
                        CliError::system("REPORT_SERIALIZE_ERROR", error.to_string())
                    })?;
                    bytes.push(b'\n');
                    bytes
                }
                ReportFormat::Html => render_monthly_report_html(&report).into_bytes(),
            };
            let output_json = match output {
                Some(path) => {
                    write_new(path, &bytes)?;
                    json!(path.display().to_string())
                }
                None => json!("-"),
            };
            let mut summary = json!({
                "schema_version": 1,
                "report_schema_version": report.schema_version,
                "format": format.as_str(),
                "output": output_json,
                "bytes": bytes.len(),
                "period": report.period,
                "source_set_hash": report.provenance.source_set_hash,
                "completeness": report.completeness.state,
            });
            if output.is_none() {
                // Bytes from serde_json and the HTML renderer are UTF-8.
                let content = String::from_utf8(bytes).map_err(|error| {
                    CliError::system("REPORT_SERIALIZE_ERROR", error.to_string())
                })?;
                summary["content"] = json!(content);
            }
            Ok(summary)
        }
    }
}

/// The report document a stdout-mode `report statements` prints verbatim in
/// text mode, instead of the summary line. `--json` keeps the envelope, which
/// carries the same bytes in `content`.
pub(crate) fn stdout_content(value: &Value) -> Option<&str> {
    value["content"].as_str()
}

pub(crate) fn human_line(value: &Value) -> String {
    format!(
        "wrote {} report for {} to {} ({} bytes; completeness {})",
        value["format"].as_str().unwrap_or("?"),
        value["period"].as_str().unwrap_or("?"),
        value["output"].as_str().unwrap_or("?"),
        value["bytes"],
        value["completeness"].as_str().unwrap_or("?")
    )
}
