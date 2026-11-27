//! `tax-payments show|set`: the company's Tax Administration payment details
//! (payee account and payment references) in `masterdata/tax-payments.json`.
//! The rules and their sources are in `aggountant_process::tax_payments`.

use std::path::Path;

use aggountant_process::{
    read_tax_payments, tax_payments_path, write_tax_payments, TaxPaymentDetails,
};
use clap::Subcommand;
use serde_json::{json, Value};

use super::{read_stdin_text, CliError};

pub(crate) const TAX_PAYMENTS_SET: &str = r#"INPUT (JSON on stdin)
  The whole set of payment details; it replaces what is stored, so a field
  left out is removed. Copy the values from OmaVero: Maksutilanne > Verojen
  maksaminen ja maksutiedot. Unknown fields are refused.
  schema_version required, 1.
  payee_iban     optional. A Tax Administration account (Finnish IBAN, no
                 spaces). Payments to the Danske Bank account starting with
                 FI56 work until the end of May 2027; it is accepted with the
                 warning TAX_PAYEE_IBAN_RETIRING.
  general_reference
                 optional. The general tax reference a limited company or
                 other corporate entity pays almost all taxes with, VAT and
                 prepayments included.
  self_assessed_reference
                 optional. The reference for VAT and other self-assessed
                 taxes, for a company that pays with tax-specific references.
  prepayment_reference
                 optional. The prepayment (for a sole trader, income tax)
                 reference.
  payment_arrangement_reference
                 optional. The reference of a payment arrangement. Never used
                 for a VAT payment.
  A reference is the domestic form (4-20 digits, 7-3-1 check digit) or the RF
  form, without spaces; a wrong check digit is refused
  (TAX_PAYMENT_DETAILS_INVALID). general_reference together with
  self_assessed_reference or prepayment_reference is refused
  (TAX_PAYMENT_REFERENCES_MIXED): the Tax Administration asks a company to pay
  all taxes one way. `file-vat` payment proposals use general_reference when
  it is stored, otherwise self_assessed_reference.

Example JSON (fictional account and references):
```json
{"schema_version":1,"payee_iban":"FI2112345600000785","self_assessed_reference":"12345672","prepayment_reference":"11112220"}
```
Check first, then write; `tax-payments show` prints what is stored:
  aggountant --home ./accounting --json tax-payments set --dry-run < tax-payments.json
  aggountant --home ./accounting --json tax-payments set < tax-payments.json
  aggountant --home ./accounting --json tax-payments show"#;

#[derive(Subcommand, Debug)]
pub(crate) enum TaxPaymentsCmd {
    /// Print the stored payment details, the reference a VAT payment uses (`vat_reference`) and any warning. `configured` is false when nothing is stored. Example: tax-payments show
    Show,
    /// Replace the payment details in masterdata/tax-payments.json with stdin JSON and print them before and after. Commit the file with git like the rest of masterdata/.
    #[command(after_help = TAX_PAYMENTS_SET)]
    Set {
        /// Validate and print the result; write nothing.
        #[arg(long)]
        dry_run: bool,
    },
}

pub(crate) fn run(cmd: &TaxPaymentsCmd, home: &Path) -> Result<Value, CliError> {
    match cmd {
        TaxPaymentsCmd::Show => {
            let stored = read_tax_payments(home)?;
            Ok(json!({
            "schema_version": 1,
            "path": display_path(home),
            "configured": stored.is_some(),
            "details": stored,
            "vat_reference": stored.as_ref().and_then(TaxPaymentDetails::vat_reference),
            "warnings": stored.as_ref().map(TaxPaymentDetails::warnings).unwrap_or_default(),
            }))
        }
        TaxPaymentsCmd::Set { dry_run } => {
            let details = TaxPaymentDetails::from_json(&read_stdin_text()?)?;
            // `set` is also how a hand-broken file is repaired, so a stored file
            // that no longer reads is reported and replaced, not a refusal.
            let (stored, stored_invalid) = match read_tax_payments(home) {
                Ok(stored) => (stored, false),
                Err(e) if e.error_code() == "TAX_PAYMENT_DETAILS_CORRUPT" => (None, true),
                Err(e) => return Err(e.into()),
            };
            if !dry_run {
                write_tax_payments(home, &details)?;
            }
            Ok(json!({
                "schema_version": 1,
                "path": display_path(home),
                "written": !dry_run,
                "changed": stored.as_ref() != Some(&details),
                "before": stored,
                "before_invalid": stored_invalid,
                "after": details,
                "vat_reference": details.vat_reference(),
                "warnings": details.warnings(),
            }))
        }
    }
}

fn display_path(home: &Path) -> String {
    tax_payments_path(home).display().to_string()
}

pub(crate) fn human_line(cmd: &TaxPaymentsCmd, value: &Value) -> String {
    let reference = match value["vat_reference"]["reference"].as_str() {
        Some(reference) => format!(
            "VAT reference {reference} ({})",
            value["vat_reference"]["kind"].as_str().unwrap_or("?")
        ),
        None => "no VAT reference".to_string(),
    };
    let warnings = value["warnings"].as_array().map_or(0, Vec::len);
    match cmd {
        TaxPaymentsCmd::Show if value["configured"] == false => {
            "tax payment details not stored (tax-payments set)".to_string()
        }
        TaxPaymentsCmd::Show => format!("tax payment details: {reference}, {warnings} warning(s)"),
        TaxPaymentsCmd::Set { .. } => format!(
            "tax payment details {}: {reference}, {warnings} warning(s)",
            if value["written"] == true {
                "written"
            } else {
                "checked (dry run)"
            }
        ),
    }
}
