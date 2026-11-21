//! `tulorekisteri configure|status`: the Incomes Register channel profile
//! (`.aggountant/tulorekisteri.json`) and its offline, itemised readiness.
//!
//! The profile's `employer_report` block has its own writer
//! (`file-wage employer-report configure`, which checks the employer register),
//! so `configure` keeps it as stored and refuses to change it.

use std::path::{Path, PathBuf};

use aggountant_broker::tulorekisteri::{
    evaluate_readiness, load_wage_profile, wage_profile_path, write_wage_profile,
    TulorekisteriReadiness, WageProfile,
};
use aggountant_model::Instant;
use clap::Subcommand;
use serde_json::{json, Value};

use super::{config_path, read_config, read_stdin_text, vero_fi, CliError};

#[derive(Subcommand, Debug)]
pub enum TulorekisteriCmd {
    /// Offline readiness of the Incomes Register channel: every unmet requirement as a blocker with a code, an owner (human or agent) and what to do. Makes no network request; the same requirements gate every send. Example: tulorekisteri status
    Status,
    /// Merge the channel settings (schema_version, mode, business_id, reporting_permission_active, credentials_file) from stdin JSON into .aggountant/tulorekisteri.json and report the changes and the resulting readiness. The employer_report block is kept as stored; `file-wage employer-report configure` writes it.
    #[command(after_help = crate::input_help::TULOREKISTERI_CONFIGURE)]
    Configure {
        /// Validate and report the changes and the resulting readiness; write nothing.
        #[arg(long)]
        dry_run: bool,
        /// Write the input as the whole profile instead of merging it into the stored one; drops every stored field the input leaves out, the employer_report block included. Also the way to overwrite a stored profile that is not valid.
        #[arg(long)]
        replace: bool,
    },
}

/// The fields `configure` writes. `employer_report` is owned by its own command.
const FIELDS: [&str; 5] = [
    "schema_version",
    "mode",
    "business_id",
    "reporting_permission_active",
    "credentials_file",
];
const MODES: [&str; 2] = ["disabled", "production"];

pub(super) fn execute(cmd: &TulorekisteriCmd, home: &Path) -> Result<Value, CliError> {
    match cmd {
        TulorekisteriCmd::Status => {
            let profile = load_wage_profile(home);
            let path = credential_path(profile.as_ref().ok().and_then(Option::as_ref))?;
            report(evaluate_readiness(
                home,
                profile,
                path.as_deref(),
                Instant::now(),
            ))
        }
        TulorekisteriCmd::Configure { dry_run, replace } => {
            configure(home, &read_stdin_text()?, *dry_run, *replace)
        }
    }
}

/// The credential file the channel uses: the profile's own `credentials_file`,
/// else the Vero credential path (input, environment, platform configuration).
pub(super) fn credential_path(profile: Option<&WageProfile>) -> Result<Option<PathBuf>, CliError> {
    if let Some(path) = profile.and_then(|p| p.credentials_file.clone()) {
        return Ok(Some(path));
    }
    let config = read_config(&config_path()?)?;
    Ok(vero_fi::resolve_credential_path(&config, None)?.value)
}

fn report(readiness: TulorekisteriReadiness) -> Result<Value, CliError> {
    let mut value = serde_json::to_value(readiness)
        .map_err(|_| CliError::system("INTERNAL_ERROR", "readiness output failed"))?;
    value["schema_version"] = json!(1);
    Ok(value)
}

fn configure(home: &Path, input: &str, dry_run: bool, replace: bool) -> Result<Value, CliError> {
    let invalid = |message: String| CliError::user("INVALID_INPUT", message);
    let patch: Value = serde_json::from_str(input)
        .map_err(|e| invalid(format!("could not parse stdin JSON: {e}")))?;
    let Some(members) = patch.as_object() else {
        return Err(invalid("stdin JSON must be an object".into()));
    };
    for key in members.keys() {
        if key == "employer_report" {
            return Err(invalid(
                "employer_report is written by `file-wage employer-report configure`, which checks it against the employer register; leave it out".into(),
            ));
        }
        if !FIELDS.contains(&key.as_str()) {
            return Err(invalid(format!(
                "unknown field `{key}`; expected one of {}",
                FIELDS.join(", ")
            )));
        }
    }

    let path = wage_profile_path(home);
    let stored = match std::fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => {
            return Err(CliError::system(
                "TULOREKISTERI_PROFILE_READ_ERROR",
                format!("{} could not be read", path.display()),
            ))
        }
    };
    // The merge base is the stored file as a JSON object, so the employer_report
    // block passes through as written; the result is then read as the profile type,
    // which refuses anything malformed.
    let before = match &stored {
        Some(bytes) if !replace => match serde_json::from_slice::<Value>(bytes) {
            Ok(value @ Value::Object(_)) => value,
            _ => {
                return Err(CliError::user(
                    "TULOREKISTERI_PROFILE_INVALID",
                    "the stored tulorekisteri.json is not a JSON object, so the input cannot be merged into it; write a complete profile with --replace",
                ))
            }
        },
        _ => Value::Object(Default::default()),
    };
    let mut merged = before.clone();
    if replace {
        merged = patch;
    } else {
        vero_fi::merge_patch(&mut merged, &patch);
    }
    let profile: WageProfile = serde_json::from_value(merged.clone()).map_err(|e| {
        let hint = if stored.is_none() || replace {
            " (schema_version, mode, business_id and reporting_permission_active are required when there is no stored profile to merge into)"
        } else {
            ""
        };
        invalid(format!("invalid Tulorekisteri profile: {e}{hint}"))
    })?;
    if profile.schema_version != 1 {
        return Err(invalid(format!(
            "schema_version must be 1, got {}",
            profile.schema_version
        )));
    }
    if !MODES.contains(&profile.mode.as_str()) {
        return Err(invalid(format!(
            "mode must be \"disabled\" or \"production\", got {:?}",
            profile.mode
        )));
    }
    if profile
        .credentials_file
        .as_ref()
        .is_some_and(|p| !p.is_absolute())
    {
        return Err(CliError::user(
            "INVALID_CREDENTIAL_PATH",
            "credentials_file must be an absolute path outside the accounting home",
        ));
    }

    let credential = credential_path(Some(&profile))?;
    let readiness = evaluate_readiness(
        home,
        Ok(Some(profile.clone())),
        credential.as_deref(),
        Instant::now(),
    );
    if profile.mode == "production" && !readiness.ready {
        let items: Vec<String> = readiness
            .blockers
            .iter()
            .map(|b| format!("{} ({}): {}", b.code, b.owner, b.message))
            .collect();
        return Err(CliError::user(
            "TULOREKISTERI_NOT_READY",
            format!(
                "a production profile is written only when the channel is ready: {}",
                items.join("; ")
            ),
        ));
    }

    let mut after = serde_json::to_value(&profile)
        .map_err(|_| CliError::system("INTERNAL_ERROR", "profile serialization failed"))?;
    if profile.employer_report.is_none() {
        after
            .as_object_mut()
            .expect("a profile serializes as an object")
            .remove("employer_report");
    }
    let mut changes = Vec::new();
    vero_fi::diff_leaves("", &before, &after, &mut changes);

    if !dry_run {
        let mut text = serde_json::to_string_pretty(&after)
            .map_err(|_| CliError::system("INTERNAL_ERROR", "profile serialization failed"))?;
        text.push('\n');
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|_| {
                CliError::system(
                    "TULOREKISTERI_PROFILE_WRITE_ERROR",
                    format!("{} could not be created", dir.display()),
                )
            })?;
        }
        write_wage_profile(home, text.as_bytes())
            .map_err(|message| CliError::system("TULOREKISTERI_PROFILE_WRITE_ERROR", message))?;
    }
    let mut value = report(readiness)?;
    value["dry_run"] = json!(dry_run);
    value["written"] = json!(!dry_run);
    value["merged"] = json!(!replace);
    value["changes"] = json!(changes);
    if dry_run {
        value["would"] = json!([{
            "action": if stored.is_some() { "update" } else { "create" },
            "resource": "tulorekisteri_profile",
            "path": path,
        }]);
    }
    Ok(value)
}
