//! `import camt seal`: prepare every voucher of a bank statement, show the
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
//! anything else is a conflict that stops the batch. A transaction the input
//! binds to a voucher sealed outside the import (`existing_vouchers`) is bound
//! to it, not sealed again; a new transaction that such a voucher may already
//! book stops the batch (`BANK_ENTRY_POSSIBLY_BOOKED`).
//!
//! An outgoing payment whose creditor reference is one of the company's stored
//! tax references (`tax-payments show`) carries `tax_reference_match` in
//! `import camt parse` and the preview. It is a comparison for the agent's
//! proposal, read from master data at display time; it enters no hash, no
//! voucher and no store.
//!
//! A statement with no booked entries prepares and seals nothing but its
//! retained bytes; both outputs carry the parser's `empty_report`, so the
//! preview the user approves says the report was empty, for which account and
//! period.

use std::collections::HashMap;

use super::*;
use aggountant_import::{
    bank_side, booking_for, check_bookings, import_preparation_hash, BankTransaction, Direction,
    EntryBooking, ParsedStatement, PreparedEntryBinding, EVENT_KEY_PREFIX,
};
use aggountant_ledger::{
    BankBatch, BankBatchReport, BankEntryFacts, BankEventBooking, BankEventEntry, BankEventOutcome,
    LegacyBankIdentity, UnbookedBankEvent,
};
use aggountant_model::Hash;
use aggountant_process::{read_tax_payments, TaxPaymentDetails, TaxReferenceKind};

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
    check_bookings(statement, &input.assignments, &input.existing_vouchers)?;
    // No auto-assign: an entry without an assignment or a binding refuses the
    // whole batch.
    let booked: Vec<(&BankTransaction, EntryBooking)> = statement
        .transactions
        .iter()
        .map(|txn| booking_for(txn, &input.assignments, &input.existing_vouchers).map(|b| (txn, b)))
        .collect::<Result<_, _>>()?;

    let attachment = ledger.artifact_address(raw);
    let chart_version = ledger.chart_version()?;
    let policy_set = PolicySet::default().id;
    let mut transactions = Vec::with_capacity(booked.len());
    let mut entries = Vec::with_capacity(booked.len());
    let mut request_hashes = Vec::with_capacity(booked.len());
    for (txn, booking) in booked {
        let (bank_account, booking, request_hash) = match booking {
            EntryBooking::Assigned(assignment) => {
                let draft = project_voucher_with_role_table(
                    txn,
                    assignment,
                    Some(attachment),
                    &chart_version,
                    ledger.vat_role_table(),
                )?;
                let request_hash = ledger.draft_request_hash(&draft)?;
                (
                    &assignment.bank_account,
                    BankEventBooking::New {
                        draft,
                        not_booked_by: assignment.not_booked_by.clone(),
                    },
                    Some(request_hash),
                )
            }
            EntryBooking::Existing(existing) => (
                &existing.bank_account,
                BankEventBooking::Existing {
                    voucher_number: existing.voucher_number,
                },
                None,
            ),
        };
        request_hashes.push(request_hash);
        transactions.push(txn);
        entries.push(BankEventEntry {
            event_key: txn.natural_key.clone(),
            source_fingerprint: txn.source_fingerprint(),
            facts: BankEntryFacts {
                bank_ref: txn.bank_ref.clone(),
                bank_account: bank_account.clone(),
                amount: txn.amount.clone(),
                bank_side: bank_side(txn.direction),
                booking_date: txn.booking_date,
                value_date: txn.value_date,
            },
            booking,
        });
    }
    let bindings: Vec<PreparedEntryBinding> = entries
        .iter()
        .zip(&request_hashes)
        .map(|(entry, request_hash)| {
            let (existing_voucher, not_booked_by) = match &entry.booking {
                BankEventBooking::New { not_booked_by, .. } => (None, not_booked_by.as_slice()),
                BankEventBooking::Existing { voucher_number } => (Some(*voucher_number), &[][..]),
            };
            PreparedEntryBinding {
                natural_key: &entry.event_key,
                source_fingerprint: &entry.source_fingerprint,
                draft_request_hash: request_hash.as_deref(),
                existing_voucher,
                not_booked_by,
            }
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
            parse_camt(&xml).ok()
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
        "bound_count": report.count(BankEventOutcome::BoundExistingVoucher),
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

/// Run `import camt parse`: the parsed statement, with each outgoing payment
/// that carries a stored tax reference marked. The home is optional for
/// parsing; without an initialised one nothing is marked.
pub(crate) fn parse_camt_file(cli: &Cli, file: &Path) -> Result<Value, CliError> {
    let statement = parse_camt(&read_camt_file(file)?)?;
    let (details, warning) = match resolve_home(cli) {
        Ok(home) if home_is_initialized(&home.value) => tax_details(&home.value),
        _ => (None, None),
    };
    let mut out = serde_json::to_value(&statement).expect("parsed statement serializes");
    if let Some(details) = &details {
        for (index, txn) in statement.transactions.iter().enumerate() {
            if let Some(found) = tax_reference_match(txn, details) {
                out["transactions"][index]["tax_reference_match"] = found;
            }
        }
    }
    add_warning(&mut out, warning);
    Ok(out)
}

/// The stored tax payment details for the comparison. They are optional
/// evidence here, so a file that does not read does not block reading the
/// statement or previewing its import: the output says the comparison was not
/// made, and `tax-payments set` repairs the file.
fn tax_details(home: &Path) -> (Option<TaxPaymentDetails>, Option<Value>) {
    match read_tax_payments(home) {
        Ok(details) => (details, None),
        Err(e) => (
            None,
            Some(json!({
                "code": e.error_code(),
                "message": format!(
                    "creditor references were not compared with the company's tax references: {e}"
                ),
            })),
        ),
    }
}

fn add_warning(out: &mut Value, warning: Option<Value>) {
    if let Some(warning) = warning {
        out["warnings"] = json!([warning]);
    }
}

/// The `tax_reference_match` of a transaction: present when it is money out
/// and its valid creditor reference equals one of the stored tax references.
fn tax_reference_match(txn: &BankTransaction, details: &TaxPaymentDetails) -> Option<Value> {
    if txn.direction != Direction::Debit {
        return None;
    }
    let reference = txn.creditor_reference.as_ref()?.canonical.as_ref()?;
    let kind = details.reference_kind(reference)?;
    let name = match kind {
        TaxReferenceKind::General => "general tax reference",
        TaxReferenceKind::SelfAssessed => "self-assessed tax reference",
        TaxReferenceKind::Prepayment => "prepayment reference",
        TaxReferenceKind::PaymentArrangement => "payment arrangement reference",
    };
    Some(json!({
        "kind": kind,
        "period_identified": false,
        "message": format!(
            "the payment carries the company's {name} (`tax-payments show`), so it is a \
             payment to the Tax Administration and is booked against the tax account, not \
             as an expense. The reference does not say which period or return it pays; \
             take that from the amount, the due date and the filed returns."
        ),
    }))
}

/// Run `import camt seal [--dry-run | --approved-preparation-hash H]`.
pub(crate) fn seal_camt(
    cli: &Cli,
    file: &Path,
    dry_run: bool,
    approved_preparation_hash: Option<&str>,
) -> Result<Value, CliError> {
    // Read the raw bytes once: the exact on-disk bytes are the KPL 2:5
    // supporting source (a UTF-8 round-trip would strip a BOM / normalize line
    // endings and break byte-for-byte retention).
    let raw = read_file_bytes(file)?;
    let xml = decode_camt(&raw, file)?;
    let statement = parse_camt(&xml)?;
    let input: ImportSealInput = read_stdin_json()?;
    let ledger = open_existing_ledger(cli)?;
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
        event_key_prefix: EVENT_KEY_PREFIX,
    };

    let preview = ledger.preview_bank_batch(&batch)?;
    if let Some(error) = conflict_error(&preview, prepared.entries.len()) {
        return Err(error);
    }

    if dry_run {
        let (tax_details, tax_warning) = tax_details(&existing_home(cli)?);
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
                });
                // A binding seals nothing, so it has no draft to show.
                if let Some(draft) = entry.draft() {
                    voucher["draft"] = json!(draft);
                }
                if let Some(existing) = &result.voucher {
                    voucher["voucher_number"] = json!(existing.number);
                }
                // Evidence for the proposal, outside the preparation hash.
                if let Some(reference) = &txn.creditor_reference {
                    voucher["creditor_reference"] = json!(reference);
                }
                if let Some(found) = tax_details
                    .as_ref()
                    .and_then(|details| tax_reference_match(txn, details))
                {
                    voucher["tax_reference_match"] = found;
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
        add_empty_report(&mut out, &statement);
        add_warning(&mut out, tax_warning);
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
        "replayed_count": report.count(BankEventOutcome::DuplicateSourceObservation),
        "sealed": sealed,
    });
    merge(&mut out, counters(&report));
    add_empty_report(&mut out, &statement);
    Ok(out)
}

/// Run `import camt observations [--attachment-ref H]`: which bank
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
    let ledger = open_existing_ledger(cli)?;
    let observations = ledger.bank_source_observations(filter.as_ref())?;
    Ok(json!({
        "observation_count": observations.len(),
        "observations": observations,
    }))
}

/// Carry the parser's empty-report result into an import output, so neither
/// the preview nor the seal of a statement without entries reads as a plain
/// success.
fn add_empty_report(out: &mut Value, statement: &ParsedStatement) {
    if let Some(empty) = &statement.empty_report {
        out["empty_report"] = json!(empty);
    }
}

fn merge(target: &mut Value, extra: Value) {
    if let (Some(target), Value::Object(extra)) = (target.as_object_mut(), extra) {
        target.extend(extra);
    }
}
