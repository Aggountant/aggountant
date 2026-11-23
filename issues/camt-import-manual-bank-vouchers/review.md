# Review: camt-import-manual-bank-vouchers

- Skill: `/llm-review`, rigor routine on a high-risk surface: the change
  decides whether a bank transaction gets a new voucher, so a miss is a
  silent, permanent double booking in production books.
- Scope: commits `18cfc22` (implementation) and `ec5b5d2` (first review
  fix) against `06be3de`: `crates/ledger/src/bank_event.rs`,
  `crates/ledger/src/store/{mod.rs,sqlite_fiscal.rs}`,
  `crates/import/src/lib.rs`, `crates/cli/src/{bank_import.rs,main.rs,input_help.rs}`,
  tests, guide pages, skill.
- Finder: `gpt-6-astra`, one call on the diff plus `sqlite_fiscal.rs`.
  Verifier: `claude-fable-5-2`, one call, asked to refute the fix of the
  finder's candidate and to answer one false-negative question on the "no
  path books one bank entry twice" boundary. It reproduced its claims with
  tests in a scratch copy. Two model calls in total.

## F1 (must-fix): binding one payment hid the voucher's other payments

- Location: `crates/ledger/src/store/sqlite_fiscal.rs`
  (`vouchers_not_bound_to_bank_event`), `crates/ledger/src/bank_event.rs`
  (`possible_bookings`, `bound_in_batch`).
- Claim: the candidate search skipped any voucher already bound to a bank
  event. A hand voucher can book several payments (Dr 7690 50 / Cr 1910 20 /
  Cr 1910 30); binding the 20.00 entry removed it from the search, so the
  30.00 entry was sealed a second time, in one batch or in a later one.
- Evidence: local read; `voucher seal` validates balance only, nothing limits
  a voucher to one bank posting. Confirmed.
- Fix (ec5b5d2): a bound voucher stayed a candidate while it had more
  postings on the bank account than bound events.

## F2 (must-fix): the posting count of the F1 fix still missed duplicates

- Raised by the verifier against ec5b5d2, reproduced there by test.
- (a) An overlapping export that repeats the binding counted the binding
  twice (stored plus in batch), so the 30.00 entry was sealed again.
- (b) The count mixed accounts: a transfer Dr 1920 / Cr 1910 bound on the
  1910 statement hid its 1920 leg from the 1920 statement's entry.
- Fix (third commit): no count at all. A bound voucher always stays a
  candidate (which posting a binding stands for is not recorded); the user
  rules it out with `not_booked_by`, and the conflict for a bound voucher
  names the correction-voucher route when it books the entry too. Regression
  tests for both paths in `crates/ledger/tests/bank_event.rs`.

## Pre-existing, out of scope (filed)

- Own-account transfer booked twice when both accounts' statements are
  imported: the candidate search skips vouchers under the import's key prefix,
  and the same gap existed before this change (there was no search at all).
  Including them needs several bank events per voucher, a schema decision.
  Filed as `camt-transfer-between-imported-accounts` (unlaned).

## Uncertainty and accepted limits

- Matching misses a hand voucher dated more than 7 days from both bank dates,
  a bank posting that sums several payments, and an amount that differs, when
  the voucher does not name the bank reference. These are the documented
  trade-off of the brief's evidence rule.

## Refuted by the verifier

- Preview and seal diverge: both run `resolve_bank_batch`; the search does not
  depend on `write`.
- An early conflict could still commit: the store gets
  `write && early_conflicts.is_empty()` and commits only then.
- Alias idempotency rows open a second seal: a later assignment hits
  `read_bank_event`; a generic seal under the key hits the idempotency row.
- Two bindings of one voucher in a batch: the second is
  `BANK_VOUCHER_MISMATCH`.

## Dropped

None of the finder's candidates were dropped (one candidate, confirmed).

## Coverage

Read: the whole diff, `seal_in_tx`, `load_voucher`, the v9 migration and FK
setting, `voucher verify` (does not read `idempotency`). Tested: targeted
ledger and CLI tests, `cargo test --workspace`, the user-guide check. Outside
scope: other import formats, receipt matching.
