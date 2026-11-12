//! First-class opening balances for financial-statement projections.
//!
//! An opening balance is fiscal input, but it is not a voucher: it has no voucher
//! number, does not enter the voucher hash chain, and cannot be mistaken for a sealed
//! business event. Its source binds either a prior sealed aggountant year or an
//! explicit external chart cut-over and live-period boundary (ADR-8).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::voucher::check_postings;
use crate::{Hash, ModelError, PeriodId, Posting};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OpeningBalanceSource {
    PriorFiscalYear {
        prior_year: i32,
        source_head_hash: Hash,
        /// Content hash of the version-exact semantic-role policy used to roll
        /// nominal accounts into retained earnings.
        role_policy_hash: Hash,
    },
    ExternalCutover {
        prior_year: i32,
        source_head_hash: Hash,
        source_chart_version: String,
        target_chart_version: String,
        account_mapping_hash: Hash,
        live_period: PeriodId,
    },
}

impl OpeningBalanceSource {
    pub fn prior_year(&self) -> i32 {
        match self {
            Self::PriorFiscalYear { prior_year, .. } | Self::ExternalCutover { prior_year, .. } => {
                *prior_year
            }
        }
    }

    pub fn source_head_hash(&self) -> Hash {
        match self {
            Self::PriorFiscalYear {
                source_head_hash, ..
            }
            | Self::ExternalCutover {
                source_head_hash, ..
            } => *source_head_hash,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningBalance {
    fiscal_year: i32,
    postings: Vec<Posting>,
    source: OpeningBalanceSource,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpeningBalanceWire {
    fiscal_year: i32,
    postings: Vec<Posting>,
    source: OpeningBalanceSource,
}

impl OpeningBalance {
    pub fn new(
        fiscal_year: i32,
        postings: Vec<Posting>,
        source: OpeningBalanceSource,
    ) -> Result<Self, ModelError> {
        if !(1000..=9999).contains(&fiscal_year) {
            return Err(ModelError::InvalidOpeningBalance(format!(
                "fiscal_year must be four digits, got {fiscal_year}"
            )));
        }
        if source.prior_year() >= fiscal_year {
            return Err(ModelError::InvalidOpeningBalance(format!(
                "opening source year {} must precede fiscal year {fiscal_year}",
                source.prior_year()
            )));
        }
        // Which year directly precedes `fiscal_year`, and which months it spans, depend
        // on the company's fiscal calendar: the Ledger takes the prior year from the
        // calendar and the Domain checks the live period against the year's dates.
        if let OpeningBalanceSource::ExternalCutover {
            source_chart_version,
            target_chart_version,
            ..
        } = &source
        {
            if source_chart_version.trim().is_empty() || target_chart_version.trim().is_empty() {
                return Err(ModelError::InvalidOpeningBalance(
                    "cut-over chart versions must not be empty".into(),
                ));
            }
        }
        check_postings(&postings)?;
        Ok(Self {
            fiscal_year,
            postings,
            source,
        })
    }

    pub fn fiscal_year(&self) -> i32 {
        self.fiscal_year
    }
    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }
    pub fn source(&self) -> &OpeningBalanceSource {
        &self.source
    }
}

impl Serialize for OpeningBalance {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        OpeningBalanceWire {
            fiscal_year: self.fiscal_year,
            postings: self.postings.clone(),
            source: self.source.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OpeningBalance {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let wire = OpeningBalanceWire::deserialize(deserializer)?;
        Self::new(wire.fiscal_year, wire.postings, wire.source).map_err(D::Error::custom)
    }
}
