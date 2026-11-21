//! `import camt053 seal`: prepare every voucher of a bank statement, show the
//! preparation with `--dry-run`, and seal exactly the prepared vouchers.
//!
//! Preview and seal run the same preparation, so the preview refuses whatever
//! the seal would refuse before any write (unknown account, a split that does
//! not sum, a closed period, a reused natural key), and the preparation hash the
//! preview returns binds the approval to the vouchers the seal produces.

use super::*;
use aggountant_import::{
    import_preparation_hash, BankTransaction, ParsedStatement, PreparedEntryBinding,
};
use aggountant_ledger::ValidationWarning;
use aggountant_model::{Hash, VoucherDraft};

/// One voucher the import would seal, with the Ledger's dry-run findings.
struct PreparedEntry<'a> {
    txn: &'a BankTransaction,
    source_fingerprint: String,
    draft_request_hash: String,
    draft: VoucherDraft,
    warnings: Vec<ValidationWarning>,
}

struct PreparedImport<'a> {
    attachment: Hash,
    chart_version: String,
    policy_set: String,
    preparation_hash: String,
    entries: Vec<PreparedEntry<'a>>,
}

/// Resolve, project and dry-run every entry. Writes nothing: the statement
/// bytes are addressed, not attached, and validated as the pending attachment.
fn prepare<'a>(
    ledger: &Ledger,
    statement: &'a ParsedStatement,
    raw: &[u8],
    input: &ImportSealInput,
) -> Result<PreparedImport<'a>, CliError> {
    check_assignments_unique(&input.assignments)?;
    // No auto-assign: an entry without an assignment refuses the whole batch.
    let assigned: Vec<(&BankTransaction, &Assignment)> = statement
        .transactions
        .iter()
        .map(|txn| assignment_for(txn, &input.assignments).map(|a| (txn, a)))
        .collect::<Result<_, _>>()?;

    let attachment = ledger.artifact_address(raw);
    let chart_version = ledger.chart_version()?;
    let policy_set = PolicySet::default().id;
    let mut entries = Vec::with_capacity(assigned.len());
    for (txn, assignment) in assigned {
        let draft = project_voucher_with_role_table(
            txn,
            assignment,
            Some(attachment),
            &chart_version,
            ledger.vat_role_table(),
        )?;
        let report =
            ledger.validate_seal_with_pending_artifact(&draft, &txn.natural_key, attachment)?;
        entries.push(PreparedEntry {
            txn,
            source_fingerprint: txn.source_fingerprint(),
            draft_request_hash: ledger.draft_request_hash(&draft)?,
            draft,
            warnings: report.warnings,
        });
    }
    let bindings: Vec<PreparedEntryBinding> = entries
        .iter()
        .map(|entry| PreparedEntryBinding {
            natural_key: &entry.txn.natural_key,
            source_fingerprint: &entry.source_fingerprint,
            draft_request_hash: &entry.draft_request_hash,
        })
        .collect();
    let preparation_hash = import_preparation_hash(
        &input.actor,
        &chart_version,
        &policy_set,
        &attachment,
        &bindings,
    );
    Ok(PreparedImport {
        attachment,
        chart_version,
        policy_set,
        preparation_hash,
        entries,
    })
}

/// Run `import camt053 seal [--dry-run | --approved-preparation-hash H]`.
pub(crate) fn seal_camt053(
    cli: &Cli,
    file: &Path,
    dry_run: bool,
    approved_preparation_hash: Option<&str>,
) -> Result<Value, CliError> {
    // Read the raw bytes once: the exact on-disk bytes are the KPL 2:5
    // supporting source (a UTF-8 round-trip would strip a BOM / normalize line
    // endings and break byte-for-byte retention).
    let raw = read_file_bytes(file)?;
    let xml = decode_camt053(&raw, file)?;
    let statement = parse_camt053(&xml)?;
    let input: ImportSealInput = read_stdin_json()?;
    let ledger = Ledger::open(&home_path(cli)?)?;
    let prepared = prepare(&ledger, &statement, &raw, &input)?;

    if dry_run {
        let vouchers: Vec<Value> = prepared
            .entries
            .iter()
            .map(|entry| {
                json!({
                    "bank_ref": entry.txn.bank_ref,
                    "natural_key": entry.txn.natural_key,
                    "source_fingerprint": entry.source_fingerprint,
                    "would_replay": entry
                        .warnings
                        .iter()
                        .any(|w| w.code == "NATURAL_KEY_ALREADY_SEALED"),
                    "warnings": entry.warnings,
                    "draft": entry.draft,
                })
            })
            .collect();
        return Ok(json!({
            "dry_run": true,
            "account_iban": statement.account_iban,
            "statements": statement.statements,
            "attachment_ref": prepared.attachment.to_hex(),
            "chart_version": prepared.chart_version,
            "policy_set": prepared.policy_set,
            "actor": input.actor,
            "preparation_hash": prepared.preparation_hash,
            "voucher_count": vouchers.len(),
            "vouchers": vouchers,
        }));
    }

    // The approval is optional so existing callers keep working; when given,
    // it must name exactly this preparation, checked before any write.
    if let Some(approved) = approved_preparation_hash {
        if approved != prepared.preparation_hash {
            return Err(CliError::user(
                "IMPORT_PREPARATION_CHANGED",
                format!(
                    "approved preparation {approved:?} does not match current {}; the statement, \
                     the assignments, the actor or the chart changed after the preview — run \
                     --dry-run again and approve the new preparation",
                    prepared.preparation_hash
                ),
            ));
        }
    }

    // Retain the original statement bytes once as the supporting source for
    // every voucher it produces (KPL 2:5).
    let attachment = ledger.attach_artifact(&raw)?;
    if attachment != prepared.attachment {
        return Err(CliError::system(
            "INTERNAL_ERROR",
            format!(
                "stored statement address {} differs from the prepared address {}",
                attachment.to_hex(),
                prepared.attachment.to_hex()
            ),
        ));
    }

    let mut sealed = Vec::new();
    let mut created = 0u64;
    let mut replayed = 0u64;
    for entry in &prepared.entries {
        let result = ledger.seal(
            &entry.draft,
            &SealRequest {
                natural_key: entry.txn.natural_key.clone(),
                actor: input.actor.clone(),
                policy_set: PolicySet {
                    id: prepared.policy_set.clone(),
                },
            },
        )?;
        if result.replayed {
            replayed += 1;
        } else {
            created += 1;
        }
        sealed.push(serde_json::to_value(result).expect("sealed serializes"));
    }
    Ok(json!({
        "dry_run": false,
        "account_iban": statement.account_iban,
        "statements": statement.statements,
        "attachment_ref": attachment.to_hex(),
        "preparation_hash": prepared.preparation_hash,
        "sealed_count": sealed.len(),
        "created_count": created,
        "replayed_count": replayed,
        "sealed": sealed,
    }))
}
