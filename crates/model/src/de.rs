//! Serde helpers shared by the input types of several crates.

use serde::{Deserialize, Deserializer};

/// Deserialize a memberless variant, refusing any member besides the tag.
///
/// Serde's container-level `deny_unknown_fields` does not reach a unit variant of
/// an internally tagged enum: `{"type":"no-vat","rate_class":"x"}` would be
/// accepted as `NoVat` with `rate_class` silently dropped. Put
/// `#[serde(deserialize_with = "aggountant_model::de::deny_members")]` on such a
/// variant so a member that signals a different intent is refused instead.
pub fn deny_members<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct NoMembers {}
    NoMembers::deserialize(deserializer).map(|NoMembers {}| ())
}
