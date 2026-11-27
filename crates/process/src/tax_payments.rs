//! The company's Tax Administration payment details: the payee account and the
//! payment references, kept as typed master data in
//! `masterdata/tax-payments.json`. They are company data the Tax
//! Administration gives the company (OmaVero shows them under "Verojen
//! maksaminen ja maksutiedot"), non-secret and not personal data, so the file
//! is plaintext and committed with the home like the rest of `masterdata/`
//! except `pii.sqlite`. It is not the books: nothing here enters a voucher or a
//! filing, only the payment proposal shown to the user.
//!
//! The rules below are the Tax Administration's, from vero.fi *Tili- ja
//! viitenumerot yritys- ja yhteisöasiakkaille* (read 2026-11-27,
//! <https://www.vero.fi/yritykset-ja-yhteisot/verot-ja-maksut/maksaminen-yritykset-ja-yhteisot/tili-ja-viitenumerot/>):
//!
//! - A corporate entity has a customer-specific general tax reference ("yleinen
//!   verojen viitenumero") that pays almost every tax, VAT and prepayments
//!   included. Without it, taxes are paid with tax-specific references; VAT and
//!   other self-assessed taxes share the self-assessed-tax reference.
//! - "Käytä verojen maksamisessa vain joko yleistä verojen viitenumeroa tai
//!   verolajien viitenumeroita": mixing the two lets payments land on other
//!   taxes than intended. So a file holds the general reference or the
//!   tax-specific ones (self-assessed, prepayment), never both.
//! - Taxes in a payment arrangement cannot be paid with the general reference;
//!   they have their own payment details. The arrangement reference is
//!   therefore kept apart and never offered for a VAT payment.
//! - The accounts are the Tax Administration's, not per tax type ("Voit maksaa
//!   verot joko Verohallinnon Danske Bankin tai Nordean tilille"), and Finnish
//!   IBANs. vero.fi does not publish them; the company copies one from OmaVero,
//!   so aggountant checks its form and checksum, not which account it is.
//! - The Danske Bank account, recognisable by its `FI56` start, is replaced by
//!   an OP account on 1 December 2026 and closed at the end of May 2027
//!   (vero.fi news *Toinen Verohallinnon tilinumeroista vaihtuu 1.12.2026*).
//!   Payments to it work until then, so it is a warning, not a refusal.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use aggountant_model::{Iban, PaymentReference};

use crate::ProcessError;

/// The schema version this binary writes and reads.
pub const TAX_PAYMENTS_SCHEMA_VERSION: u32 = 1;

/// The payment details as stored and as `tax-payments set` takes them on stdin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxPaymentDetails {
    pub schema_version: u32,
    /// The Tax Administration account the company pays to, as OmaVero shows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payee_iban: Option<String>,
    /// The customer-specific general tax reference (corporate entities).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_reference: Option<PaymentReference>,
    /// The reference for VAT and other self-assessed taxes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_assessed_reference: Option<PaymentReference>,
    /// The reference for prepayment tax (for a sole trader, the income tax one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepayment_reference: Option<PaymentReference>,
    /// The reference of a payment arrangement; never used for anything else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_arrangement_reference: Option<PaymentReference>,
}

/// Which reference a VAT payment uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VatReferenceKind {
    General,
    SelfAssessed,
}

/// The reference a VAT payment proposal carries, and which one it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VatPaymentReference {
    pub reference: PaymentReference,
    pub kind: VatReferenceKind,
}

/// A note about stored details that are valid but need the user's attention.
#[derive(Debug, Clone, Serialize)]
pub struct TaxPaymentWarning {
    pub code: &'static str,
    pub message: String,
}

/// Where a VAT payment goes, resolved from the input and the master data.
#[derive(Debug, Clone)]
pub struct VatPayee {
    pub payee_iban: String,
    pub reference: Option<VatPaymentReference>,
}

fn invalid(message: impl Into<String>) -> ProcessError {
    ProcessError::TaxPayments {
        code: "TAX_PAYMENT_DETAILS_INVALID",
        message: message.into(),
    }
}

/// Check that `iban` can be a Tax Administration account: a Finnish IBAN with
/// a valid checksum. The message never echoes the value.
pub fn validate_payee_iban(iban: &str) -> Result<(), ProcessError> {
    Iban::new(iban).map_err(|e| invalid(format!("payee_iban: {e}")))?;
    if !iban.starts_with("FI") || iban.len() != 18 {
        return Err(invalid(
            "payee_iban: the Tax Administration's accounts are Finnish IBANs (FI and 16 digits)",
        ));
    }
    Ok(())
}

impl TaxPaymentDetails {
    /// Parse and validate the details from JSON text, naming the field whose
    /// value is refused (serde alone reports only a line and column).
    pub fn from_json(text: &str) -> Result<Self, ProcessError> {
        let members: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(text).map_err(|e| invalid(format!("not a JSON object: {e}")))?;
        for (field, value) in &members {
            if field.ends_with("_reference") {
                if let Some(raw) = value.as_str() {
                    PaymentReference::new(raw).map_err(|e| invalid(format!("{field}: {e}")))?;
                }
            }
        }
        if let Some(iban) = members
            .get("payee_iban")
            .and_then(serde_json::Value::as_str)
        {
            validate_payee_iban(iban)?;
        }
        let details: TaxPaymentDetails = serde_json::from_value(serde_json::Value::Object(members))
            .map_err(|e| invalid(e.to_string()))?;
        details.validate()?;
        Ok(details)
    }

    /// Everything the types alone do not check.
    pub fn validate(&self) -> Result<(), ProcessError> {
        if self.schema_version != TAX_PAYMENTS_SCHEMA_VERSION {
            return Err(invalid(format!(
                "schema_version must be {TAX_PAYMENTS_SCHEMA_VERSION}"
            )));
        }
        if let Some(iban) = &self.payee_iban {
            validate_payee_iban(iban)?;
        }
        if self.general_reference.is_some()
            && (self.self_assessed_reference.is_some() || self.prepayment_reference.is_some())
        {
            return Err(ProcessError::TaxPayments {
                code: "TAX_PAYMENT_REFERENCES_MIXED",
                message: "keep either general_reference or the tax-specific references \
                          (self_assessed_reference, prepayment_reference), not both: the Tax \
                          Administration asks a company to pay all taxes one way, because \
                          mixing them allocates payments to other taxes than intended"
                    .into(),
            });
        }
        let named = [
            ("general_reference", &self.general_reference),
            ("self_assessed_reference", &self.self_assessed_reference),
            ("prepayment_reference", &self.prepayment_reference),
            (
                "payment_arrangement_reference",
                &self.payment_arrangement_reference,
            ),
        ];
        for (i, (first, a)) in named.iter().enumerate() {
            for (second, b) in &named[i + 1..] {
                if a.is_some() && a == b {
                    return Err(invalid(format!(
                        "{first} and {second} are the same reference; each one directs \
                         payments to a different debt, so check them in OmaVero"
                    )));
                }
            }
        }
        Ok(())
    }

    /// The reference a VAT payment uses: the general reference when the company
    /// pays with it, otherwise the self-assessed-tax reference. Never the
    /// payment arrangement reference.
    pub fn vat_reference(&self) -> Option<VatPaymentReference> {
        match (&self.general_reference, &self.self_assessed_reference) {
            (Some(reference), _) => Some(VatPaymentReference {
                reference: reference.clone(),
                kind: VatReferenceKind::General,
            }),
            (None, Some(reference)) => Some(VatPaymentReference {
                reference: reference.clone(),
                kind: VatReferenceKind::SelfAssessed,
            }),
            (None, None) => None,
        }
    }

    pub fn warnings(&self) -> Vec<TaxPaymentWarning> {
        let mut warnings = Vec::new();
        if self
            .payee_iban
            .as_deref()
            .is_some_and(|iban| iban.starts_with("FI56"))
        {
            warnings.push(TaxPaymentWarning {
                code: "TAX_PAYEE_IBAN_RETIRING",
                message: "payee_iban starts with FI56, the Tax Administration's Danske Bank \
                          account, which is replaced by an OP account on 1 December 2026 and \
                          closed at the end of May 2027; payments to it after that are \
                          returned. Copy the Nordea or the new OP account from OmaVero \
                          (Verojen maksaminen ja maksutiedot)."
                    .into(),
            });
        }
        warnings
    }
}

pub fn tax_payments_path(home: &Path) -> PathBuf {
    home.join("masterdata").join("tax-payments.json")
}

/// The stored details, or `None` when the home has none yet.
pub fn read_tax_payments(home: &Path) -> Result<Option<TaxPaymentDetails>, ProcessError> {
    let path = tax_payments_path(home);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(ProcessError::Store(format!(
                "masterdata/tax-payments.json unreadable: {e}"
            )))
        }
    };
    let corrupt = |why: String| ProcessError::TaxPayments {
        code: "TAX_PAYMENT_DETAILS_CORRUPT",
        message: format!(
            "masterdata/tax-payments.json is not valid ({why}); write it again with \
             `aggountant tax-payments set`"
        ),
    };
    let text = String::from_utf8(bytes).map_err(|_| corrupt("not UTF-8".into()))?;
    TaxPaymentDetails::from_json(&text)
        .map(Some)
        .map_err(|e| corrupt(e.to_string()))
}

/// Validate and replace the stored details atomically.
pub fn write_tax_payments(home: &Path, details: &TaxPaymentDetails) -> Result<(), ProcessError> {
    details.validate()?;
    let store = |e: std::io::Error| ProcessError::Store(e.to_string());
    let path = tax_payments_path(home);
    let dir = path
        .parent()
        .expect("tax-payments.json has a parent directory");
    if !dir.is_dir() {
        return Err(ProcessError::Store(
            "masterdata/ does not exist; is this an initialised company home?".into(),
        ));
    }
    let mut bytes = serde_json::to_vec_pretty(details)
        .map_err(|e| ProcessError::Store(format!("tax payment details: {e}")))?;
    bytes.push(b'\n');
    let temporary = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&temporary).map_err(store)?;
        file.write_all(&bytes).map_err(store)?;
        file.sync_all().map_err(store)?;
    }
    fs::rename(&temporary, &path).map_err(store)
}

/// Resolve where a VAT payment goes: the payee account from `input_iban` or
/// the master data (they must agree when both are given), and the VAT
/// reference from the master data, which is the only source of references so
/// that a payment arrangement reference cannot reach a VAT payment.
pub fn resolve_vat_payee(home: &Path, input_iban: Option<&str>) -> Result<VatPayee, ProcessError> {
    let stored = read_tax_payments(home)?;
    let stored_iban = stored.as_ref().and_then(|d| d.payee_iban.as_deref());
    if let Some(iban) = input_iban {
        validate_payee_iban(iban)?;
    }
    let payee_iban = match (input_iban, stored_iban) {
        (Some(input), Some(stored)) if input != stored => {
            return Err(ProcessError::TaxPayments {
                code: "TAX_PAYEE_IBAN_MISMATCH",
                message: "payee_iban in the input differs from masterdata/tax-payments.json; \
                          leave it out to use the stored account, or correct the stored one \
                          with `aggountant tax-payments set`"
                    .into(),
            })
        }
        (Some(iban), _) | (None, Some(iban)) => iban.to_string(),
        (None, None) => {
            return Err(ProcessError::TaxPayments {
                code: "TAX_PAYEE_IBAN_MISSING",
                message: "no Tax Administration account: store it once with `aggountant \
                          tax-payments set` (payee_iban, as OmaVero shows it under Verojen \
                          maksaminen ja maksutiedot), or give payee_iban in the input"
                    .into(),
            })
        }
    };
    Ok(VatPayee {
        payee_iban,
        reference: stored.as_ref().and_then(TaxPaymentDetails::vat_reference),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(raw: &str) -> Option<PaymentReference> {
        Some(PaymentReference::new(raw).unwrap())
    }

    fn details() -> TaxPaymentDetails {
        TaxPaymentDetails {
            schema_version: 1,
            payee_iban: Some("FI2112345600000785".into()),
            general_reference: None,
            self_assessed_reference: reference("12345672"),
            prepayment_reference: reference("11112220"),
            payment_arrangement_reference: reference("33334442"),
        }
    }

    #[test]
    fn vat_uses_the_self_assessed_reference_never_the_arrangement_one() {
        let d = details();
        d.validate().unwrap();
        let vat = d.vat_reference().unwrap();
        assert_eq!(vat.reference.as_str(), "12345672");
        assert_eq!(vat.kind, VatReferenceKind::SelfAssessed);

        let only_arrangement = TaxPaymentDetails {
            self_assessed_reference: None,
            prepayment_reference: None,
            ..details()
        };
        assert_eq!(only_arrangement.vat_reference(), None);
    }

    #[test]
    fn a_company_paying_with_the_general_reference_pays_vat_with_it() {
        let d = TaxPaymentDetails {
            general_reference: reference("76543216"),
            self_assessed_reference: None,
            prepayment_reference: None,
            ..details()
        };
        d.validate().unwrap();
        let vat = d.vat_reference().unwrap();
        assert_eq!(vat.reference.as_str(), "76543216");
        assert_eq!(vat.kind, VatReferenceKind::General);
    }

    #[test]
    fn general_and_tax_specific_references_are_not_mixed() {
        let d = TaxPaymentDetails {
            general_reference: reference("76543216"),
            ..details()
        };
        assert_eq!(
            d.validate().unwrap_err().error_code(),
            "TAX_PAYMENT_REFERENCES_MIXED"
        );
    }

    #[test]
    fn the_same_reference_twice_is_refused() {
        let d = TaxPaymentDetails {
            payment_arrangement_reference: reference("12345672"),
            ..details()
        };
        let err = d.validate().unwrap_err();
        assert_eq!(err.error_code(), "TAX_PAYMENT_DETAILS_INVALID");
        assert!(err.to_string().contains("self_assessed_reference"));
    }

    #[test]
    fn payee_iban_must_be_a_finnish_iban() {
        assert!(validate_payee_iban("FI2112345600000785").is_ok());
        assert!(validate_payee_iban("FI2112345600000786").is_err());
        // A valid German IBAN is not a Tax Administration account.
        assert!(validate_payee_iban("DE89370400440532013000").is_err());
        let err = validate_payee_iban("FI2112345600000786").unwrap_err();
        assert!(!err.to_string().contains("12345600000786"));
    }

    #[test]
    fn the_retiring_danske_account_is_a_warning() {
        let d = TaxPaymentDetails {
            payee_iban: Some("FI5680001200000045".into()),
            ..details()
        };
        d.validate().unwrap();
        let codes: Vec<_> = d.warnings().iter().map(|w| w.code).collect();
        assert_eq!(codes, ["TAX_PAYEE_IBAN_RETIRING"]);
        assert!(details().warnings().is_empty());
    }

    #[test]
    fn resolve_takes_the_stored_account_and_refuses_a_different_input() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir_all(home.path().join("masterdata")).unwrap();
        let missing = resolve_vat_payee(home.path(), None).unwrap_err();
        assert_eq!(missing.error_code(), "TAX_PAYEE_IBAN_MISSING");
        let from_input = resolve_vat_payee(home.path(), Some("FI2112345600000785")).unwrap();
        assert_eq!(from_input.payee_iban, "FI2112345600000785");
        assert!(from_input.reference.is_none());

        write_tax_payments(home.path(), &details()).unwrap();
        let stored = resolve_vat_payee(home.path(), None).unwrap();
        assert_eq!(stored.payee_iban, "FI2112345600000785");
        assert_eq!(stored.reference.unwrap().reference.as_str(), "12345672");
        assert!(resolve_vat_payee(home.path(), Some("FI2112345600000785")).is_ok());
        let other = resolve_vat_payee(home.path(), Some("FI9112345600000786")).unwrap_err();
        assert_eq!(other.error_code(), "TAX_PAYEE_IBAN_MISMATCH");
    }

    #[test]
    fn from_json_names_the_refused_field() {
        let err = TaxPaymentDetails::from_json(
            r#"{"schema_version":1,"prepayment_reference":"11112221"}"#,
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "TAX_PAYMENT_DETAILS_INVALID");
        assert!(err.to_string().contains("prepayment_reference"), "{err}");
        let unknown =
            TaxPaymentDetails::from_json(r#"{"schema_version":1,"vat_reference":"12345672"}"#)
                .unwrap_err();
        assert!(unknown.to_string().contains("vat_reference"), "{unknown}");
        let ok = TaxPaymentDetails::from_json(
            r#"{"schema_version":1,"payee_iban":"FI2112345600000785","self_assessed_reference":"12345672"}"#,
        )
        .unwrap();
        assert_eq!(ok.vat_reference().unwrap().reference.as_str(), "12345672");
    }

    #[test]
    fn a_hand_broken_file_is_reported_as_corrupt() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir_all(home.path().join("masterdata")).unwrap();
        fs::write(
            tax_payments_path(home.path()),
            r#"{"schema_version":1,"self_assessed_reference":"12345673"}"#,
        )
        .unwrap();
        let err = read_tax_payments(home.path()).unwrap_err();
        assert_eq!(err.error_code(), "TAX_PAYMENT_DETAILS_CORRUPT");
    }
}
