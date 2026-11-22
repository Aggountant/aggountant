# Result: camt-event-identity

`import camt053 seal` books each bank transaction once, however many
overlapping exports it arrives in, and writes a statement whole or not at all.

## What changed for the user

- Every transaction of a statement resolves to `new_voucher` or
  `duplicate_source_observation` (already booked with the same bank facts and
  the same booking; nothing is sealed, the file is recorded as a source). The
  preview shows the outcome per transaction, with the voucher number of a
  duplicate.
- Anything else stops the whole batch before any write: `SOURCE_FACTS_CHANGED`
  (amount, dates, direction, parties or remittance differ, or the file lists a
  booked transaction as not booked), `POSTING_PROJECTION_CHANGED` (same facts,
  different booking) or `SOURCE_IDENTITY_UNKNOWN` (a voucher holds the key but
  its transaction cannot be read back from its attachment). The error's new
  `details` member lists every conflict with its voucher and the counters.
- Outputs carry `new_voucher_count`, `duplicate_observation_count`,
  `conflict_count` and `backfilled_count`; `created_count`, `replayed_count`
  and `sealed` stay for existing callers.
- `import camt053 observations [--attachment-ref H]` lists which transactions
  each retained statement file contained.

## How

- `crates/ledger/src/bank_event.rs`: the batch API (`preview_bank_batch`,
  `seal_bank_batch`), the projection hash (dates, description, postings and VAT
  treatment, without the attachment or the release's VAT policy state) and the
  backfill of legacy identities.
- `crates/ledger/src/store/sqlite_fiscal.rs`: fiscal schema v9 (`bank_event`,
  `bank_source_observation`, additive), `commit_bank_batch` in one
  `BEGIN IMMEDIATE` transaction, and `seal_in_tx` shared with the single seal.
- `crates/import/src/camt053.rs`: non-booked entries are listed in `unbooked`
  so a status change is caught.
- `crates/cli/src/bank_import.rs`: preflight without writing, then attach and
  seal; reads legacy identities from the statements old vouchers retained.

## Existing homes

They open with this build and migrate to fiscal v9 by adding two empty tables;
no voucher row changes, and `voucher verify` gives the same head hash. The
first import with this build backfills the identity of every voucher the
previous import sealed, from its attached statement, in the import's own
transaction. A binary from before this change refuses a v9 home (newer schema),
so going back means restoring the pre-install copy.

Verified in a scratch home written by the `main` binary: same head hash before
and after, and an overlapping export of its two transactions (one with a VAT
treatment) gave two duplicate observations, two backfilled identities and no
new voucher.

## Review

`/llm-review` (critical) and assessment beside this file: three real findings
fixed (the projection hash depended on the release; booked transactions were
held to today's content checks; a pending-and-booked reference conflicted with
itself), one product question left open (optional assignment for a booked
transaction; whether display-only bank fields should stop an import).
`simplicity.md` records the simplicity review.

## Tests

`crates/cli/tests/import_camt053.rs` (overlapping yearly exports, changed
amount, date, status and assignment, legacy vouchers, unreadable identity),
`crates/ledger/tests/bank_event.rs` (v8 home migration and backfill, late
conflict writes nothing and is audited, preview writes nothing),
`crates/import/tests/parse.rs`, unit tests for the projection and the v9
migration. `scripts/ci-local.sh` passes.
