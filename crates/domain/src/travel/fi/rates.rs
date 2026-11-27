//! The bundled rate table `data/fi-travel-allowances.json`: one edition per
//! year, each copied from the Tax Administration's decision on tax-free travel
//! allowances for that year (Finlex statute and diary number in the edition),
//! plus the territory definitions the decisions attach to the foreign table.
//!
//! The table is parsed on first use only. A command that computes no claim
//! never reaches [`table`], so companies that do not use expense claims pay
//! nothing for it; [`reads_on_this_thread`] lets a test prove that.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::OnceLock;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

const TABLE_JSON: &str = include_str!("../../../data/fi-travel-allowances.json");

thread_local! {
    static READS: Cell<u64> = const { Cell::new(0) };
}

/// How many times code on the current thread has read the travel rate table.
/// Exists for the test that proves commands unrelated to claims never read
/// it; a thread-local count keeps parallel tests from seeing each other.
pub fn reads_on_this_thread() -> u64 {
    READS.with(Cell::get)
}

fn count_read() {
    READS.with(|reads| reads.set(reads.get() + 1));
}

pub(super) fn table() -> &'static RateTable {
    count_read();
    static TABLE: OnceLock<RateTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(TABLE_JSON).expect("bundled fi-travel-allowances.json parses")
    })
}

/// `fi-travel-allowances@sha256:<hex>` over the bundled bytes, stamped into
/// every computation so a preview names the exact table it used.
pub(super) fn table_version() -> &'static str {
    count_read();
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        let digest = Sha256::digest(TABLE_JSON.as_bytes());
        format!("fi-travel-allowances@sha256:{}", hex::encode(digest))
    })
}

fn decimal<'de, D: Deserializer<'de>>(d: D) -> Result<Decimal, D::Error> {
    let s = String::deserialize(d)?;
    Decimal::from_str_exact(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RateTable {
    /// Checked by the tests; the computation stamps the content hash instead.
    #[allow(dead_code)]
    pub table: String,
    pub territories: Territories,
    pub editions: Vec<Edition>,
}

impl RateTable {
    /// The edition for a calendar year; `None` when the year has none. A
    /// missing year is never filled from a neighbouring one.
    pub fn edition(&self, year: i16) -> Option<&Edition> {
        self.editions.iter().find(|e| e.year == year)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Territories {
    #[allow(dead_code)]
    pub source: String,
    #[allow(dead_code)]
    pub source_url: String,
    pub aliases: BTreeMap<String, Alias>,
    pub unsettled: BTreeMap<String, String>,
    pub domestic: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Alias {
    pub country: String,
    pub region: Option<String>,
    pub basis: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Edition {
    pub year: i16,
    /// The calendar year of `year` (decision 19 §); the tests check that.
    #[allow(dead_code)]
    pub valid_from: aggountant_model::Date,
    #[allow(dead_code)]
    pub valid_to: aggountant_model::Date,
    pub decision: Decision,
    pub mileage_eur_per_km: VehicleRates,
    pub car_surcharges_eur_per_km: CarSurcharges,
    #[serde(deserialize_with = "decimal")]
    pub passenger_surcharge_eur_per_km: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub car_benefit_fuel_eur_per_km: Decimal,
    pub domestic: Domestic,
    pub foreign: Foreign,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub title: String,
    pub statute: String,
    pub diary_number: String,
    pub issued: String,
    pub published: String,
    pub url: String,
    pub vero_fi_url: String,
    pub foreign_rows: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VehicleRates {
    #[serde(deserialize_with = "decimal")]
    pub car: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub motorboat_up_to_50hp: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub motorboat_over_50hp: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub snowmobile: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub atv: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub motorcycle: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub moped: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub other: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CarSurcharges {
    #[serde(deserialize_with = "decimal")]
    pub trailer: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub caravan: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub heavy_load: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub heavy_equipment: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub dog: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub forest_road: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Domestic {
    #[serde(deserialize_with = "decimal")]
    pub partial_day: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub full_day: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub meal_allowance: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub meal_allowance_two_meals: Decimal,
    #[serde(deserialize_with = "decimal")]
    pub night_travel_allowance: Decimal,
    #[allow(dead_code)]
    #[serde(deserialize_with = "decimal")]
    pub border_crossing_allowance: Decimal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Foreign {
    #[serde(deserialize_with = "decimal")]
    pub not_listed: Decimal,
    pub countries: BTreeMap<String, Country>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Country {
    pub row: String,
    #[serde(deserialize_with = "decimal")]
    pub amount: Decimal,
    #[serde(default)]
    pub regions: BTreeMap<String, Region>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Region {
    pub row: String,
    #[serde(deserialize_with = "decimal")]
    pub amount: Decimal,
}

/// The row of the foreign table a country (and optional city or region)
/// resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ForeignRow {
    /// The ISO 3166 code the row is listed under (after territory aliases).
    pub country: String,
    pub region: Option<String>,
    /// The decision's own row name, or "Maa, jota ei ole erikseen mainittu".
    pub row: String,
    pub amount: Decimal,
    /// The territory definition that moved the code to another row, if any.
    pub alias_basis: Option<String>,
}

/// Why a country answer cannot be resolved to a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CountryProblem {
    /// Not an ISO 3166-1 alpha-2 code (or `XK`).
    Invalid,
    /// A city or region the decision does not list for this country.
    InvalidRegion,
    /// Finland (or Åland): not a foreign country.
    Domestic,
    /// A territory the decision and its definitions do not place.
    Unsettled(String),
}

pub(super) const NOT_LISTED_ROW: &str = "Maa, jota ei ole erikseen mainittu";

/// Whether `code` names Finland (including Åland) in the territory list.
pub(super) fn is_domestic(code: &str) -> bool {
    table().territories.domestic.iter().any(|c| c == code)
}

/// Resolve a country code and optional region to its row in `edition`. An
/// unknown code never falls to the "not separately listed" row: only a valid
/// ISO 3166 code that the decision does not list does.
pub(super) fn foreign_row(
    edition: &Edition,
    code: &str,
    region: Option<&str>,
) -> Result<ForeignRow, CountryProblem> {
    let territories = &table().territories;
    if !is_country_code(code) {
        return Err(CountryProblem::Invalid);
    }
    if territories.domestic.iter().any(|c| c == code) {
        return Err(CountryProblem::Domestic);
    }
    if let Some(reason) = territories.unsettled.get(code) {
        return Err(CountryProblem::Unsettled(reason.clone()));
    }
    let (country, region, alias_basis) = match territories.aliases.get(code) {
        Some(alias) => {
            let region = match (alias.region.as_deref(), region) {
                (Some(_), Some(_)) => return Err(CountryProblem::InvalidRegion),
                (Some(own), None) => Some(own),
                (None, given) => given,
            };
            (alias.country.as_str(), region, Some(alias.basis.clone()))
        }
        None => (code, region, None),
    };
    match edition.foreign.countries.get(country) {
        Some(listed) => match region {
            None => Ok(ForeignRow {
                country: country.to_string(),
                region: None,
                row: listed.row.clone(),
                amount: listed.amount,
                alias_basis,
            }),
            Some(key) => listed
                .regions
                .get(key)
                .map(|r| ForeignRow {
                    country: country.to_string(),
                    region: Some(key.to_string()),
                    row: r.row.clone(),
                    amount: r.amount,
                    alias_basis: alias_basis.clone(),
                })
                .ok_or(CountryProblem::InvalidRegion),
        },
        None if region.is_some() => Err(CountryProblem::InvalidRegion),
        None => Ok(ForeignRow {
            country: country.to_string(),
            region: None,
            row: NOT_LISTED_ROW.to_string(),
            amount: edition.foreign.not_listed,
            alias_basis,
        }),
    }
}

/// Whether `code` is an ISO 3166-1 alpha-2 code, or `XK` (Kosovo, which the
/// decision lists and which has no official alpha-2 code).
pub(super) fn is_country_code(code: &str) -> bool {
    code == "XK" || ISO_3166_ALPHA2.binary_search(&code).is_ok()
}

/// ISO 3166-1 alpha-2 codes in use, sorted (the 249 codes of the tz database's
/// `iso3166.tab`).
const ISO_3166_ALPHA2: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX", "AZ",
    "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS",
    "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE",
    "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF",
    "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM",
    "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA",
    "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG",
    "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS",
    "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO",
    "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_list_is_sorted_and_complete() {
        assert_eq!(ISO_3166_ALPHA2.len(), 249);
        assert!(ISO_3166_ALPHA2.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn editions_are_calendar_years_with_their_decision() {
        let t = table();
        assert_eq!(t.table, "fi-travel-allowances");
        for e in &t.editions {
            assert_eq!(e.valid_from.to_string(), format!("{}-01-01", e.year));
            assert_eq!(e.valid_to.to_string(), format!("{}-12-31", e.year));
            assert!(e.decision.title.ends_with(&format!("vuonna {}", e.year)));
        }
    }

    #[test]
    fn every_listed_country_and_alias_target_is_a_code() {
        let t = table();
        for e in &t.editions {
            for code in e.foreign.countries.keys() {
                assert!(is_country_code(code), "{code}");
                assert!(!t.territories.domestic.contains(code));
            }
        }
        for (code, alias) in &t.territories.aliases {
            assert!(is_country_code(code));
            assert!(t.editions[0].foreign.countries.contains_key(&alias.country));
        }
    }
}
