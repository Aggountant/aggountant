use std::path::{Path, PathBuf};

use super::{environment_path, CliError, ConfigFile};

pub(super) struct ResolvedCredentialPath {
    pub(super) value: Option<PathBuf>,
    pub(super) source: &'static str,
}

/// Resolve the one effective platform-local Vero credential path.
///
/// A path supplied by `vero-fi configure` takes precedence for that operation;
/// otherwise the environment overrides the platform configuration file.
pub(super) fn resolve_credential_path(
    config: &ConfigFile,
    supplied: Option<&Path>,
) -> Result<ResolvedCredentialPath, CliError> {
    let (value, source) = if let Some(path) = supplied {
        (Some(path.to_path_buf()), "input")
    } else if let Some(path) = environment_path("AGGOUNTANT_VERO_CREDENTIALS_FILE") {
        (Some(path), "env")
    } else if let Some(path) = &config.vero_credentials_file {
        (Some(path.clone()), "file")
    } else {
        (None, "default")
    };

    if value.as_ref().is_some_and(|path| !path.is_absolute()) {
        return Err(CliError::user(
            "INVALID_CREDENTIAL_PATH",
            "platform-local Vero credential path must be absolute",
        ));
    }

    Ok(ResolvedCredentialPath { value, source })
}
