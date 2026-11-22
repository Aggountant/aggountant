//! `import camt053 seal`: prepare every voucher of a bank statement, show the
//! preparation with `--dry-run`, and seal exactly the prepared vouchers.
//!
//! Preview and seal run the same preparation and the same Ledger batch
//! resolution, so the preview refuses whatever the seal would refuse before any
//! write (unknown account, a split that does not sum, a closed period, a changed
//! transaction), and the preparation hash the preview returns binds the approval
//! to the vouchers the seal produces. The seal resolves the batch once more
//! without writing before it stores the statement, so a refused batch leaves no
//! trace, and then writes the whole batch in one Ledger transaction.
//!
//! A transaction already in the books (the same event key from another export)
//! is a duplicate observation when its bank facts and its booking are unchanged;
//! anything else is a conflict that stops the batch.

use std::collections::HashMap;

use super::*;
use aggountant_import::{
    import_preparation_hash, BankTransaction, ParsedStatement, PreparedEntryBinding,
    EVENT_KEY_PREFIX,
};
use aggountant_ledger::{
    BankBatch, BankBatchReport, BankEventEntry, BankEventOutcome, LegacyBankIdentity,
    UnbookedBankEvent,
};
use aggountant_model::Hash;

struct PreparedImport<'a> {
    attachment: Hash,
    chart_version: String,
    policy_set: String,
    preparation_hash: String,
    transactions: Vec<&'a BankTransaction>,
    entries: Vec<BankEventEntry>,
}

/// Resolve and project every entry. Writes nothing: the statement bytes are
/// addressed, not attached.
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
    let mut transactions = Vec::with_capacity(assigned.len());
    let mut entries = Vec::with_capacity(assigned.len());
    let mut request_hashes = Vec::with_capacity(assigned.len());
    for (txn, assignment) in assigned {
        let draft = project_voucher_with_role_table(
            txn,
            assignment,
            Some(attachment),
            &chart_version,
            ledger.vat_role_table(),
        )?;
        request_hashes.push(ledger.draft_request_hash(&draft)?);
        transactions.push(txn);
        entries.push(BankEventEntry {
            event_key: txn.natural_key.clone(),
            source_fingerprint: txn.source_fingerprint(),
            draft,
        });
    }
    let bindings: Vec<PreparedEntryBinding> = entries
        .iter()
        .zip(&request_hashes)
        .map(|(entry, request_hash)| PreparedEntryBinding {
            natural_key: &entry.event_key,
            source_fingerprint: &entry.source_fingerprint,
            draft_request_hash: request_hash,
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
        transactions,
        entries,
    })
}

/// The source fingerprints of vouchers an import sealed before bank events were
/// recorded, read from the statement each voucher retained. A voucher whose
/// statement cannot be read, parsed or does not contain its transaction is left
/// out; the Ledger then refuses any batch that meets its event key
/// (`SOURCE_IDENTITY_UNKNOWN`) instead of booking it again.
fn legacy_identities(ledger: &Ledger) -> Result<Vec<LegacyBankIdentity>, CliError> {
    let mut statements: HashMap<Hash, Option<ParsedStatement>> = HashMap::new();
    let mut identities = Vec::new();
    for voucher in ledger.bank_vouchers_without_event(EVENT_KEY_PREFIX)? {
        let Some(attachment) = voucher.attachment_ref else {
            continue;
        };
        let statement = statements.entry(attachment).or_insert_with(|| {
            let raw = ledger.read_artifact(&attachment).ok()?;
            let xml = std::str::from_utf8(raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&raw))
                .ok()?
                .to_owned();
            parse_camt053(&xml).ok()
        });
        if let Some(txn) = statement.as_ref().and_then(|s| {
            s.transactions
                .iter()
                .find(|t| t.natural_key == voucher.event_key)
        }) {
            identities.push(LegacyBankIdentity {
                event_key: voucher.event_key,
                source_fingerprint: txn.source_fingerprint(),
            });
        }
    }
    Ok(identities)
}

/// The batch counters every output and conflict error carries.
fn counters(report: &BankBatchReport) -> Value {
    json!({
        "new_voucher_count": report.count(BankEventOutcome::NewVoucher),
        "duplicate_observation_count": report.count(BankEventOutcome::DuplicateSourceObservation),
        "conflict_count": report.conflicts.len(),
        "backfilled_count": report.backfilled_count,
    })
}

/// Refuse a batch with conflicts. The code is the first conflict's; `details`
/// lists every conflict, so one run shows all of them.
fn conflict_error(report: &BankBatchReport, entry_count: usize) -> Option<CliError> {
    let first = report.conflicts.first()?;
    let listed: Vec<String> = report
        .conflicts
        .iter()
        .map(|c| format!("{}: {}", c.error_code, c.message))
        .collect();
    let mut details = counters(report);
    details["entry_count"] = json!(entry_count);
    details["conflicts"] = serde_json::to_value(&report.conflicts).expect("conflicts serialize");
    Some(
        CliError::user(
            first.error_code,
            format!(
                "{} of {entry_count} bank transactions conflict with the books and nothing was \
                 written: {}",
                report.conflicts.len(),
                listed.join("; ")
            ),
        )
        .with_details(details),
    )
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
    let backfill = legacy_identities(&ledger)?;
    let unbooked: Vec<UnbookedBankEvent> = statement
        .unbooked
        .iter()
        .map(|u| UnbookedBankEvent {
            event_key: u.natural_key.clone(),
            status: u.status.clone(),
        })
        .collect();
    let batch = BankBatch {
        artifact: prepared.attachment,
        entries: &prepared.entries,
        unbooked: &unbooked,
        backfill: &backfill,
        actor: &input.actor,
        policy_set: &prepared.policy_set,
    };

    let preview = ledger.preview_bank_batch(&batch)?;
    if let Some(error) = conflict_error(&preview, prepared.entries.len()) {
        return Err(error);
    }

    if dry_run {
        let vouchers: Vec<Value> = prepared
            .transactions
            .iter()
            .zip(&prepared.entries)
            .zip(&preview.results)
            .map(|((txn, entry), result)| {
                let mut voucher = json!({
                    "bank_ref": txn.bank_ref,
                    "natural_key": entry.event_key,
                    "source_fingerprint": entry.source_fingerprint,
                    "outcome": result.outcome,
                    "would_replay": result.outcome == BankEventOutcome::DuplicateSourceObservation,
                    "warnings": result.warnings,
                    "draft": entry.draft,
                });
                if let Some(existing) = &result.voucher {
                    voucher["voucher_number"] = json!(existing.number);
                }
                voucher
            })
            .collect();
        let mut out = json!({
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
        });
        merge(&mut out, counters(&preview));
        return Ok(out);
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

    let report = ledger.seal_bank_batch(&batch)?;
    if let Some(error) = conflict_error(&report, prepared.entries.len()) {
        return Err(error);
    }
    let sealed: Vec<Value> = prepared
        .transactions
        .iter()
        .zip(&report.results)
        .map(|(txn, result)| {
            let mut value = serde_json::to_value(&result.voucher).expect("sealed serializes");
            value["bank_ref"] = json!(txn.bank_ref);
            value["outcome"] = json!(result.outcome);
            value
        })
        .collect();
    let new_vouchers = report.count(BankEventOutcome::NewVoucher);
    let mut out = json!({
        "dry_run": false,
        "account_iban": statement.account_iban,
        "statements": statement.statements,
        "attachment_ref": attachment.to_hex(),
        "preparation_hash": prepared.preparation_hash,
        "sealed_count": sealed.len(),
        "created_count": new_vouchers,
        "replayed_count": sealed.len() - new_vouchers,
        "sealed": sealed,
    });
    merge(&mut out, counters(&report));
    Ok(out)
}

/// Run `import camt053 observations [--attachment-ref H]`: which bank
/// transactions each retained statement file contained.
pub(crate) fn observations(cli: &Cli, attachment_ref: Option<&str>) -> Result<Value, CliError> {
    let filter = attachment_ref
        .map(|raw| {
            Hash::parse_hex(raw).map_err(|e| {
                CliError::user(
                    "INVALID_HASH",
                    format!("--attachment-ref must be a 64-character lowercase hex hash: {e}"),
                )
            })
        })
        .transpose()?;
    let ledger = Ledger::open(&home_path(cli)?)?;
    let observations = ledger.bank_source_observations(filter.as_ref())?;
    Ok(json!({
        "observation_count": observations.len(),
        "observations": observations,
    }))
}

fn merge(target: &mut Value, extra: Value) {
    if let (Some(target), Value::Object(extra)) = (target.as_object_mut(), extra) {
        target.extend(extra);
    }
}
