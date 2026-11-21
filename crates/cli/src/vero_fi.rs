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

/// Evidence the verify commands record into the profile. `configure` keeps it
/// as stored: a merge input cannot clear or change it, because it cannot be
/// re-created by typing it and losing it blocks production. `--replace` is the
/// explicit way to write a whole profile without it.
const RECORDED_EVIDENCE: [&str; 2] = ["test_verification", "portal_verification"];

/// The outcome of applying one `vero-fi configure` input to the stored profile.
pub(super) struct ConfigureOutcome {
    pub(super) profile: aggountant_broker::VeroFilingProfile,
    pub(super) credential_path: Option<PathBuf>,
    /// Leaf fields whose stored value differs from the resulting profile.
    pub(super) changes: Vec<serde_json::Value>,
}

/// Apply a configure input to the stored profile text (`None` when there is no
/// profile yet). Without `replace` the input is a JSON merge patch (RFC 7386):
/// absent fields keep their stored values and `null` clears a field. With
/// `replace` the input is the whole profile, as configure behaved before
/// merging.
pub(super) fn apply_configure_input(
    stored: Option<&[u8]>,
    input: &str,
    replace: bool,
) -> Result<ConfigureOutcome, CliError> {
    use serde_json::Value;

    let mut patch: Value = serde_json::from_str(input)
        .map_err(|e| CliError::user("INVALID_INPUT", format!("could not parse stdin JSON: {e}")))?;
    let Some(members) = patch.as_object_mut() else {
        return Err(CliError::user(
            "INVALID_INPUT",
            "stdin JSON must be an object",
        ));
    };
    let credential_path = match members.remove("credential_path") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            serde_json::from_value::<PathBuf>(value)
                .map_err(|e| CliError::user("INVALID_INPUT", format!("credential_path: {e}")))?,
        ),
    };

    // The stored profile, normalised through its type so that the merge base
    // and the change list are not disturbed by how the file happens to be
    // formatted. An invalid stored profile cannot be merged into; `--replace`
    // overwrites it.
    let stored_profile = stored.map(|bytes| {
        serde_json::from_slice::<aggountant_broker::VeroFilingProfile>(bytes)
            .ok()
            .filter(|profile| profile.validate().is_ok())
    });
    let before = match stored_profile {
        Some(Some(profile)) => serde_json::to_value(profile)
            .map_err(|_| CliError::system("INTERNAL_ERROR", "profile serialization failed"))?,
        Some(None) if !replace => {
            return Err(CliError::user(
                "VERO_PROFILE_INVALID",
                "the stored vero-filing.json is not a valid profile, so the input cannot be merged into it; write a complete profile with --replace",
            ))
        }
        Some(None) | None => Value::Object(Default::default()),
    };

    let merged = if replace {
        patch
    } else {
        reject_unknown_nulls("", &patch, PROFILE_FIELDS)?;
        let mut merged = before.clone();
        merge_patch(&mut merged, &patch);
        merged
    };
    let profile: aggountant_broker::VeroFilingProfile = serde_json::from_value(merged)
        .map_err(|e| {
            let hint = if stored.is_none() || replace {
                " (schema_version and mode are required when there is no stored profile to merge into)"
            } else {
                ""
            };
            CliError::user("INVALID_INPUT", format!("could not parse stdin JSON: {e}{hint}"))
        })?;
    profile
        .validate()
        .map_err(|message| CliError::user("INVALID_VERO_PROFILE", message))?;

    let after = serde_json::to_value(&profile)
        .map_err(|_| CliError::system("INTERNAL_ERROR", "profile serialization failed"))?;
    if !replace {
        for name in RECORDED_EVIDENCE {
            let stored_evidence = &before["attestations"][name];
            if !stored_evidence.is_null() && stored_evidence != &after["attestations"][name] {
                return Err(CliError::user(
                    "VERO_EVIDENCE_WOULD_BE_DROPPED",
                    format!(
                        "the input would clear or change attestations.{name}, which a verify command recorded; leave the field out to keep it, or write a complete profile with --replace to drop it on purpose"
                    ),
                ));
            }
        }
    }

    let mut changes = Vec::new();
    diff_leaves("", &before, &after, &mut changes);
    Ok(ConfigureOutcome {
        profile,
        credential_path,
        changes,
    })
}

/// The profile's field names, level by level: a field name and the fields of
/// its object value. Only the merge needs it: a `null` member is a deletion and
/// never reaches `deny_unknown_fields`, so a misspelt name would otherwise be
/// accepted as a silent no-op. Every other member is checked by deserialising
/// the merged profile.
struct Field(&'static str, &'static [Field]);
const PROFILE_FIELDS: &[Field] = &[
    Field("schema_version", &[]),
    Field("mode", &[]),
    Field("business_id", &[]),
    Field("contact_name", &[]),
    Field("contact_phone", &[]),
    Field(
        "attestations",
        &[
            Field("software_registered", &[]),
            Field("api_agreement_active", &[]),
            Field("production_verification_completed", &[]),
            Field("production_permission_active", &[]),
            Field("activation_certificate_fingerprint_sha256", &[]),
            Field(
                "test_verification",
                &[
                    Field("observed_at", &[]),
                    Field("certificate_fingerprint_sha256", &[]),
                ],
            ),
            Field(
                "portal_verification",
                &[
                    Field("observed_at", &[]),
                    Field("certificate_fingerprint_sha256", &[]),
                    Field("operations", &[]),
                    Field("file_vat_return_receipt", &[]),
                ],
            ),
        ],
    ),
];

fn reject_unknown_nulls(
    prefix: &str,
    patch: &serde_json::Value,
    fields: &[Field],
) -> Result<(), CliError> {
    let Some(members) = patch.as_object() else {
        return Ok(());
    };
    for (key, value) in members {
        let known = fields.iter().find(|Field(name, _)| name == key);
        match known {
            None if value.is_null() => {
                return Err(CliError::user(
                    "INVALID_INPUT",
                    format!("could not parse stdin JSON: unknown field `{prefix}{key}`"),
                ))
            }
            Some(Field(_, nested)) => {
                reject_unknown_nulls(&format!("{prefix}{key}."), value, nested)?
            }
            None => {}
        }
    }
    Ok(())
}

/// RFC 7386 JSON merge patch.
pub(super) fn merge_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    use serde_json::Value;
    let Value::Object(patch_members) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(Default::default());
    }
    let Value::Object(target_members) = target else {
        unreachable!("target was made an object above");
    };
    for (key, value) in patch_members {
        if value.is_null() {
            target_members.remove(key);
        } else {
            merge_patch(
                target_members.entry(key.clone()).or_insert(Value::Null),
                value,
            );
        }
    }
}

/// Every leaf (non-object value) that differs, as `{field, before, after}` with
/// a dotted field path; a missing leaf is `null`.
pub(super) fn diff_leaves(
    prefix: &str,
    before: &serde_json::Value,
    after: &serde_json::Value,
    out: &mut Vec<serde_json::Value>,
) {
    use serde_json::Value;
    let empty = serde_json::Map::new();
    match (before, after) {
        (Value::Object(_), _) | (_, Value::Object(_)) => {
            let b = before.as_object().unwrap_or(&empty);
            let a = after.as_object().unwrap_or(&empty);
            let mut keys: Vec<&String> = b.keys().chain(a.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                diff_leaves(
                    &path,
                    b.get(key).unwrap_or(&Value::Null),
                    a.get(key).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        _ if before != after => out.push(serde_json::json!({
            "field": prefix,
            "before": before,
            "after": after,
        })),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(prefix: &str, value: &serde_json::Value, out: &mut Vec<String>) {
        for (key, nested) in value.as_object().into_iter().flatten() {
            out.push(format!("{prefix}{key}"));
            paths(&format!("{prefix}{key}."), nested, out);
        }
    }

    fn field_paths(prefix: &str, fields: &[Field], out: &mut Vec<String>) {
        for Field(name, nested) in fields {
            out.push(format!("{prefix}{name}"));
            field_paths(&format!("{prefix}{name}."), nested, out);
        }
    }

    /// PROFILE_FIELDS names exactly the fields a fully populated profile
    /// serialises, so a field added to the profile cannot be missed here.
    #[test]
    fn profile_fields_match_the_profile_type() {
        let fingerprint = "a".repeat(64);
        let full = serde_json::json!({
            "schema_version": 1,
            "mode": "production",
            "business_id": "1234567-1",
            "contact_name": "Maija Meikäläinen",
            "contact_phone": "+358401234567",
            "attestations": {
                "software_registered": true,
                "api_agreement_active": true,
                "production_verification_completed": true,
                "production_permission_active": true,
                "activation_certificate_fingerprint_sha256": fingerprint,
                "test_verification": {
                    "observed_at": "2026-09-12T08:15:30Z",
                    "certificate_fingerprint_sha256": fingerprint
                },
                "portal_verification": {
                    "observed_at": "2026-09-13T09:00:00Z",
                    "certificate_fingerprint_sha256": fingerprint,
                    "operations": ["GetVATPeriods/v1", "FileVATReturn/v2", "GetFiledVATReturn/v2"],
                    "file_vat_return_receipt": "{\"UniqueIdentifier\":\"verification\",\"AcceptedTimestamp\":\"2026-09-13T09:00:00Z\"}"
                }
            }
        });
        let profile: aggountant_broker::VeroFilingProfile = serde_json::from_value(full).unwrap();
        profile.validate().unwrap();
        let mut serialised = Vec::new();
        paths("", &serde_json::to_value(profile).unwrap(), &mut serialised);
        let mut declared = Vec::new();
        field_paths("", PROFILE_FIELDS, &mut declared);
        serialised.sort();
        declared.sort();
        assert_eq!(serialised, declared);
    }
}
