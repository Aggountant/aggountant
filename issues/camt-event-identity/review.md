# Review: camt-event-identity

- Skill: `/llm-review`, rigor **critical**: the change adds a fiscal-store
  migration for homes that already hold sealed vouchers, and decides whether a
  bank transaction is booked a second time. A wrong answer double-books a
  transaction or loses a write silently.
- Scope: commit `62b6acb` against `main` (`crates/ledger/src/bank_event.rs`,
  `crates/ledger/src/store/{mod,sqlite_fiscal}.rs`, `crates/ledger/src/lib.rs`,
  `crates/import/src/camt053.rs`, `crates/cli/src/{bank_import,main,input_help}.rs`,
  `crates/model/src/voucher.rs`, tests, guide, skill).
- Finder: `gpt-6-astra`, one call over the diff with `bank_event.rs` and
  `bank_import.rs` attached: **No findings.**
- Verifier: `claude-fable-5-2`, one call over the diff with `bank_event.rs`,
  `sqlite_fiscal.rs` and `bank_import.rs` attached. Because the finder was
  clean on a high-risk surface it was meant to get one focused false-negative
  question about the batch, migration and double-booking boundary; a quoting
  slip in the call sent it the attachments without the question text, so it
  reviewed the diff unprompted. Its candidates were checked locally as below.
  Two model calls in total.
- Risk class: data migration of live homes; double booking.

## F1 (must-fix): the projection hash depended on the release, not the booking

- Location: `crates/ledger/src/bank_event.rs`, `projection_hash`.
- Claim: the hash covered the whole draft except the attachment, including the
  VAT expansion's `policy_set` and `vat_rates_hash`. A release that edits the
  bundled rates file or renames the policy set re-projects an unchanged booked
  transaction with a `vat_treatment` to a different hash, so every overlapping
  import meeting it would stop with `POSTING_PROJECTION_CHANGED`, and the
  advice in the message (give the same assignment) cannot fix it. Year-end
  releases and yearly exports meet exactly there.
- Evidence: local read of `crates/model/src/vat_expansion.rs` (fields
  `policy_set`, `vat_rates_hash`, `version`) and `domain::vat::expand`, which
  fills them from `PolicySet::default()` and the hash of the bundled rates.
- Fix: the projection is now an explicit structure: dates, description,
  postings (accounts, amounts, sides, VAT codes), buyer VAT ID, corrected VAT
  period and the VAT treatment with source amount, accounts, resolved rate and
  generated legs; the expansion's software state is left out. Unit test
  `the_projection_ignores_the_statement_file_and_the_release`.

## F2 (should-fix): an already booked transaction had to pass today's content checks

- Location: `crates/ledger/src/bank_event.rs`, `resolve_bank_batch`.
- Claim: `check_seal_content` ran on every entry, duplicates included, so a
  booked transaction whose account has since left the chart, or that fails a
  later role or mistag rule, would refuse a batch that seals nothing for it.
- Evidence: local read; the checks run before the store resolves the entry.
- Fix: the checks run only for entries whose key holds no voucher; a booked
  one is judged by its stored fingerprint and projection.

## F3 (should-fix): a document listing one reference as pending and booked conflicted with itself

- Location: `crates/ledger/src/store/sqlite_fiscal.rs`, `commit_bank_batch`;
  `crates/import/src/camt053.rs`, `parse_camt053`.
- Claim: the unbooked-status check ran after the batch's own seals in the same
  transaction, so a multi-report camt.052 that shows a reference pending in one
  report and booked in a later one would refuse itself with
  `SOURCE_FACTS_CHANGED` against a voucher that never commits.
- Evidence: local read; the parser's uniqueness check covers booked entries
  only. Not confirmed that a real export produces such a file.
- Fix: the parser leaves out of `unbooked` a reference the document also
  books, and the store checks unbooked keys before resolving entries. Test
  `an_unbooked_entry_is_listed_unless_the_document_also_books_it`.

## Lower findings fixed

- An unbooked-status conflict wrote an empty `request_hash` into its rejection
  record; it now names the booked voucher's request hash.
- `backfilled_count` in a preview or a refused batch is what would have been
  recorded; the field doc says so.

## Not acted on

- **The fingerprint covers counterparty and remittance text** (verifier F3): if
  a bank re-renders a display field between exports, the transaction stops with
  `SOURCE_FACTS_CHANGED` and the file cannot be imported. This is the issue's
  design (the fingerprint is the bank's facts before booking, and a changed
  fact must stop loudly); no evidence that Holvi exports change these fields.
  Carried as a discussion item, together with the verifier's proposal to make
  an assignment optional for an already booked key.
- **Conflict rejections are normally not written**: the CLI previews before it
  seals, so a conflict is refused by the preview without a rejection record, as
  every other import refusal already was before this change. The seal's own
  conflict audit covers only a change between preview and seal. Documented in
  the code.
- **Legacy statements that cannot be read back are re-parsed on every
  import**: a few parses of local files; dropped.
- **Scope discrepancy** (the issue says old homes are rebuilt): superseded by
  the conductor decision recorded in `item.md`; live homes migrate in place.

## Residual risk

- Generated VAT leg accounts come from the active chart's role table; a new
  chart edition that moves them makes an overlapping re-import of a booked VAT
  transaction stop with `POSTING_PROJECTION_CHANGED`. Loud, not silent.
- Checked by running: the full workspace tests, `scripts/ci-local.sh`, and a
  manual migration check in a scratch home: a home written by the `main`
  binary (fiscal schema v8, two camt vouchers, one with a VAT treatment)
  opened with this build, `voucher verify` gave the same head hash before and
  after, and an overlapping export of the same transactions resolved to two
  duplicate observations with two backfilled identities and no new voucher.
