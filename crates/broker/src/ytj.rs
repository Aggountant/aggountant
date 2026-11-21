//! Read-only lookup of a company's register entries from the PRH/YTJ open data
//! API (<https://avoindata.prh.fi>, CC BY 4.0). No credentials, no outbox: the
//! query has no effect at the remote side and nothing is ever sent but the
//! Business ID.
//!
//! The lookup sits behind [`YtjLookup`] so callers and tests never need the
//! network. A failed or unreadable answer is an error, never an empty register:
//! "the company is not in the employer register" must only come from a response
//! that says so.
use aggountant_model::{BusinessId, Date};
use serde::Deserialize;
use std::time::Duration;

/// The companies endpoint of the YTJ open data API, version 3.
pub const YTJ_COMPANIES_URL: &str = "https://avoindata.prh.fi/opendata-ytj-api/v3/companies";

/// One entry of `registeredEntries`, as the API documents it. `register` is a
/// code of the `REK` list (`7` = Työnantajarekisteri); `type` is a code of the
/// `REK_KDI` list read together with the register (`7_41`, `7_42` registered,
/// `7_99` an erroneous line).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(default)]
    pub descriptions: Vec<EntryDescription>,
    pub registration_date: Option<Date>,
    pub end_date: Option<Date>,
    pub register: String,
    pub authority: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDescription {
    /// `1` Finnish, `2` Swedish, `3` English.
    pub language_code: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl RegisterEntry {
    /// The Finnish status text, falling back to any language.
    pub fn status_text(&self) -> Option<&str> {
        let text = |code: &str| {
            self.descriptions
                .iter()
                .find(|d| d.language_code == code)
                .and_then(|d| d.description.as_deref())
        };
        text("1").or_else(|| {
            self.descriptions
                .iter()
                .find_map(|d| d.description.as_deref())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum YtjError {
    /// No usable answer: connection, timeout, or a non-200 status. Never means
    /// "not registered".
    #[error("YTJ open data API could not be reached: {0}")]
    Unavailable(String),
    /// A 200 answer that is not the documented shape.
    #[error("YTJ open data API answered in an unexpected shape: {0}")]
    InvalidResponse(String),
}

/// Where register entries come from. `Ok(None)` means the API answered and
/// knows no company with this Business ID.
pub trait YtjLookup {
    fn register_entries(&self, id: &BusinessId) -> Result<Option<Vec<RegisterEntry>>, YtjError>;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompaniesResponse {
    total_results: u64,
    companies: Vec<Company>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Company {
    business_id: BusinessIdField,
    registered_entries: Vec<RegisterEntry>,
}

#[derive(Deserialize)]
struct BusinessIdField {
    value: String,
}

/// Read a `/v3/companies?businessId=` answer. Only a company whose Business ID
/// is exactly `id` counts; two such companies is an unexpected shape.
pub fn parse_companies(
    body: &[u8],
    id: &BusinessId,
) -> Result<Option<Vec<RegisterEntry>>, YtjError> {
    let response: CompaniesResponse = serde_json::from_slice(body)
        .map_err(|e| YtjError::InvalidResponse(format!("companies response: {e}")))?;
    let mut matching = response
        .companies
        .into_iter()
        .filter(|c| c.business_id.value == id.as_str());
    let Some(company) = matching.next() else {
        if response.total_results != 0 {
            return Err(YtjError::InvalidResponse(format!(
                "{} results but none for {}",
                response.total_results,
                id.as_str()
            )));
        }
        return Ok(None);
    };
    if matching.next().is_some() {
        return Err(YtjError::InvalidResponse(format!(
            "several companies with Business ID {}",
            id.as_str()
        )));
    }
    Ok(Some(company.registered_entries))
}

/// The production lookup over HTTPS.
pub struct HttpYtj {
    client: reqwest::blocking::Client,
}

impl HttpYtj {
    pub fn new() -> Result<Self, YtjError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .https_only(true)
            .user_agent(concat!("aggountant/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| YtjError::Unavailable(e.to_string()))?;
        Ok(Self { client })
    }
}

impl YtjLookup for HttpYtj {
    fn register_entries(&self, id: &BusinessId) -> Result<Option<Vec<RegisterEntry>>, YtjError> {
        let response = self
            .client
            .get(YTJ_COMPANIES_URL)
            .query(&[("businessId", id.as_str())])
            .send()
            .map_err(|e| YtjError::Unavailable(e.to_string()))?;
        let status = response.status();
        if status != reqwest::StatusCode::OK {
            return Err(YtjError::Unavailable(format!("HTTP {status}")));
        }
        let body = response
            .bytes()
            .map_err(|e| YtjError::Unavailable(e.to_string()))?;
        parse_companies(&body, id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> BusinessId {
        BusinessId::parse(s).unwrap()
    }

    #[test]
    fn reads_entries_of_the_exact_business_id_and_tolerates_extra_fields() {
        let body = r#"{"totalResults":1,"companies":[{"businessId":{"value":"1234567-1","source":"3"},
            "names":[],"tradeRegisterStatus":"1","lastModified":"2026-01-01T00:00:00",
            "registeredEntries":[{"type":"41","descriptions":[{"languageCode":"3","description":"Registered"},
            {"languageCode":"1","description":"Rekisterissä"}],"registrationDate":"2020-02-01","register":"7","authority":"1"}]}]}"#.as_bytes();
        let entries = parse_companies(body, &id("1234567-1")).unwrap().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status_text(), Some("Rekisterissä"));
        assert_eq!(entries[0].end_date, None);
    }

    #[test]
    fn unknown_company_is_none_and_a_broken_answer_is_an_error() {
        let empty = br#"{"totalResults":0,"companies":[]}"#;
        assert_eq!(parse_companies(empty, &id("1234567-1")).unwrap(), None);
        for bad in [
            &b"<html>"[..],
            br#"{"companies":[]}"#,
            br#"{"totalResults":1,"companies":[]}"#,
        ] {
            assert!(matches!(
                parse_companies(bad, &id("1234567-1")),
                Err(YtjError::InvalidResponse(_))
            ));
        }
    }
}
