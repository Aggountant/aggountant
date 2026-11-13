//! Period VAT reconciliation for `file-vat`: the VAT-role accounts against the
//! derived return figures (issue `vat-untagged-posting-omitted`). The Domain finds
//! the postings the return would leave out; this module turns that into the
//! preview's attention items and the filing refusal.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use aggountant_domain::{
    reconcile_vat_accounts, DerivedVatFigures, VatAccountReconciliation, VatReconciliationStatus,
};
use aggountant_ledger::Ledger;
use aggountant_model::{AccountKind, PeriodId, VoucherBody};

use crate::ProcessError;

/// The operator's recorded decision to file although untagged VAT-account postings
/// remain. It names the exact vouchers it covers, so it cannot carry over to a
/// period whose discrepancy has changed, and it is stored in the Run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VatReconciliationAcknowledgement {
    /// Why these postings correctly stay out of the return.
    pub reason: String,
    /// The voucher numbers of every untagged posting, as the preview lists them.
    pub vouchers: Vec<u64>,
}

/// One item the preview asks the operator to look at.
#[derive(Debug, Clone, Serialize)]
pub struct VatAttentionItem {
    pub code: &'static str,
    /// `blocking` stops the direct filing; `info` does not.
    pub severity: &'static str,
    pub message: String,
    pub vouchers: Vec<u64>,
}

pub(crate) fn reconcile_period(
    ledger: &Ledger,
    period: &PeriodId,
    vouchers: &[VoucherBody],
    figures: &DerivedVatFigures,
) -> Result<VatAccountReconciliation, ProcessError> {
    let kinds: HashMap<String, AccountKind> = ledger
        .chart_list(None)?
        .into_iter()
        .map(|account| (account.id.as_str().to_string(), account.kind))
        .collect();
    reconcile_vat_accounts(
        period,
        vouchers,
        figures,
        ledger.vat_role_table(),
        &|account: &str| kinds.get(account).copied(),
    )
    .map_err(|e| ProcessError::Store(format!("{}: {e}", e.error_code())))
}

pub(crate) fn attention_items(reconciliation: &VatAccountReconciliation) -> Vec<VatAttentionItem> {
    let mut items = Vec::new();
    let untagged = reconciliation.untagged_vouchers();
    if reconciliation.status == VatReconciliationStatus::Unreconciled {
        items.push(VatAttentionItem {
            code: "VAT_ACCOUNTS_UNRECONCILED",
            severity: "blocking",
            message: format!(
                "{} posting(s) on VAT accounts are not covered by a VAT code, and the VAT \
                 accounts differ from the return (output VAT {}, input VAT {}). Check them against their source. \
                 VAT that belongs in this return is filed in OmaVero with corrected \
                 figures (the closed period cannot be corrected here) and must not be \
                 recorded again with a VAT code later; acknowledge only postings that do \
                 not belong in a return.",
                reconciliation.untagged_postings.len(),
                reconciliation.output.difference.amount_str(),
                reconciliation.input.difference.amount_str(),
            ),
            vouchers: untagged,
        });
    } else if !untagged.is_empty() {
        items.push(VatAttentionItem {
            code: "VAT_UNTAGGED_POSTINGS_COVERED",
            severity: "info",
            message: "Postings on VAT accounts without a VAT code are matched by VAT-coded \
                      entries in the same period, so the return agrees with the VAT accounts."
                .to_string(),
            vouchers: untagged,
        });
    }
    // With no untagged postings the whole difference sits on VAT-coded vouchers.
    // Within one cent per tagged line it is per-line rounding and not worth a note.
    let (output, input) = (&reconciliation.output, &reconciliation.input);
    let beyond = |d: &aggountant_domain::VatDirectionReconciliation| {
        d.difference.amount().abs() > d.rounding_tolerance.amount()
    };
    if reconciliation.untagged_postings.is_empty() && (beyond(output) || beyond(input)) {
        items.push(VatAttentionItem {
            code: "VAT_ACCOUNTS_DIFFER_ON_TAGGED_VOUCHERS",
            severity: "info",
            message: format!(
                "On vouchers with a VAT code the VAT accounts differ from the computed VAT by \
                 more than per-line rounding (output {}, input {}): a VAT leg is missing, on another account, or does not \
                 match its VAT code. The return follows the VAT codes.",
                output.difference.amount_str(),
                input.difference.amount_str(),
            ),
            vouchers: Vec::new(),
        });
    }
    items
}

/// Refuse a filing whose period leaves untagged VAT out of the return, unless the
/// acknowledgement names exactly the vouchers involved and gives a reason.
pub(crate) fn require_reconciled(
    reconciliation: &VatAccountReconciliation,
    acknowledgement: Option<&VatReconciliationAcknowledgement>,
) -> Result<(), ProcessError> {
    let vouchers = reconciliation.untagged_vouchers();
    let reconciled = reconciliation.status == VatReconciliationStatus::Reconciled;
    match acknowledgement {
        None if reconciled => Ok(()),
        None => Err(ProcessError::VatAccountsUnreconciled(format!(
            "period {}: VAT-account postings on voucher(s) {vouchers:?} are not covered by a \
             VAT code and the VAT accounts differ from the return (output VAT {}, input VAT \
             {}); see `file-vat preview`",
            reconciliation.period,
            reconciliation.output.difference.amount_str(),
            reconciliation.input.difference.amount_str(),
        ))),
        Some(ack) => {
            let mut acknowledged = ack.vouchers.clone();
            acknowledged.sort_unstable();
            acknowledged.dedup();
            if reconciled || ack.reason.trim().is_empty() || acknowledged != vouchers {
                return Err(ProcessError::VatReconciliationAckMismatch(format!(
                    "period {}: the acknowledgement must give a reason and name exactly the \
                     vouchers with untagged VAT-account postings {vouchers:?}, got {:?}",
                    reconciliation.period, ack.vouchers
                )));
            }
            Ok(())
        }
    }
}
