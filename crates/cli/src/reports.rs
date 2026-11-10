use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Subcommand, ValueEnum};
use serde_json::{json, Value};

use aggountant_ledger::{render_monthly_report_html, Ledger, MonthlyReportOptions};
use aggountant_model::{Date, PeriodId};

use crate::CliError;

#[derive(Subcommand, Debug)]
pub(crate) enum ReportCmd {
    /// Produce month/YTD income and month-end balance from sealed ledger data.
    Statements {
        /// Calendar month in strict YYYY-MM form.
        #[arg(long)]
        period: String,
        /// Company display name printed in the report.
        #[arg(long)]
        company: String,
        /// Explicit output representation.
        #[arg(long, value_enum)]
        format: ReportFormat,
        /// New output file. Existing files are never overwritten.
        #[arg(long)]
        output: PathBuf,
        /// Optional date from which the caller states history is available; not a calculation cutoff or verification.
        #[arg(long)]
        history_start: Option<String>,
        /// Optional factual limitation shown in native and rendered output.
        #[arg(long)]
        limitation: Option<String>,
    },
}

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
            let ledger = Ledger::open(home)?;
            let report = ledger.monthly_statements_report(
                &period,
                company,
                MonthlyReportOptions {
                    history_start,
                    limitation: limitation.clone(),
                },
            )?;
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
            write_new(output, &bytes)?;
            Ok(json!({
                "schema_version": 1,
                "report_schema_version": report.schema_version,
                "format": format.as_str(),
                "output": output.display().to_string(),
                "bytes": bytes.len(),
                "period": report.period,
                "source_set_hash": report.provenance.source_set_hash,
                "completeness": report.completeness.state,
            }))
        }
    }
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
