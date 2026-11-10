//! Deterministic VAT-filing law (F4, scenario B2). Pure, I/O-free domain
//! functions used by the filing Run. The implementation is split by VAT concern
//! so follow-up work can proceed without a single hot source file.

use std::sync::OnceLock;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use aggountant_model::{
    AccountId, Currency, Date, GeneratedVatRole, Hash, Money, PeriodId, Posting, Side, VatCode,
    VatExpansionDeductibility, VatExpansionDeductibilityProvenance, VatExpansionScope,
    VatExpansionTreatment, VoucherBody, VoucherDraft,
};

use crate::DomainError;

mod code;
mod derive;
mod expand;
mod filing;
mod guards;
mod roles;

pub use code::*;
pub use derive::*;
pub use expand::*;
pub use filing::*;
pub use guards::*;
pub use roles::*;

#[cfg(test)]
mod tests_derive;
#[cfg(test)]
mod tests_expand;
#[cfg(test)]
mod tests_filing;
#[cfg(test)]
mod tests_guards;
#[cfg(test)]
mod tests_margin;
#[cfg(test)]
mod tests_roles;

#[cfg(test)]
use tests_derive::{
    date, eu_sale_postings, reverse_charge_postings, vat_voucher, vat_voucher_sided,
};
#[cfg(test)]
use tests_expand::{
    domestic_half_deductible, domestic_standard, expand_body, expand_input, reverse_charge,
    reverse_charge_half_deductible,
};
#[cfg(test)]
use tests_filing::{eur_money, q1};
