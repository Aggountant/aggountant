//! Chart-of-accounts node (`candidate.md §3`). Lives in `masterdata` and
//! carries a `chart_version`; the Model layer owns only the type.

use serde::{Deserialize, Serialize};

use crate::newtype::AccountId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AccountKind {
    Asset,
    Liability,
    Equity,
    Income,
    Expense,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub kind: AccountKind,
}
