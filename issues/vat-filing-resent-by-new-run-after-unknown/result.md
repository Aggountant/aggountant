# Result: vat-filing-resent-by-new-run-after-unknown

A new `file-vat` Run can no longer send a period whose earlier filing may have
been accepted. It is refused with `VAT_FILING_UNRESOLVED` before any
FileVATReturn request, until `file-vat reconcile` has either adopted the earlier
filing or recorded two "not found" answers a full Finnish bank day apart.

This work was harvested from a worker that died on 2026-11-16 (disk full and
provider quota). Its branch sat directly on the current `main`; its calendar
module and `recovery.rs` were complete (no truncated file), built, and passed
the existing tests. It was adopted as two commits, then finished: hardening,
tests, CLI help, guide, skill, review.

- `crates/domain/src/bank_days.rs` (new): Finnish bank days (Monday–Friday
  without New Year's Day, Epiphany, Good Friday, Easter Monday, 1 May,
  Ascension Day, Midsummer Eve, Independence Day and 24–26 December), the
  Gregorian Easter computus, Finnish local time with the EU summer-time rule,
  and `one_bank_day_after`.
- `crates/process/src/file_vat/recovery.rs` (new): `unresolved_filing`, the one
  rule for the dispatch guard, `complete-manual` and `file-vat status`;
  `refuse_while_filing_unresolved`; and `reconcile_file_vat(_at)`, which records
  every GetFiledVATReturn answer on the Run as an `Observe(GetFiledVATReturn)`
  effect (status and check time in its note, in the hash-chained journal) and
  returns `outcome` (`adopted`, `not_found`, `closed_not_filed`, `unresolved`),
  `authority_checks` and `next_check_from`.
- `crates/process/src/file_vat.rs`: the guard runs in `run_submit_filing` (the
  one dispatch path), and earlier in `start_file_vat` (no Run is created) and in
  the `ApproveFiling` answer (the approval is not recorded, so the Run can be
  approved once the period is free). A Run closed as not filed never dispatches,
  and on resume it does not adopt a filing a later Run made.
- `crates/broker/src/vero_vat_api.rs`: only `Status == "Processed"` can match a
  filing (review finding V2).
- CLI: `VAT_FILING_UNRESOLVED` is caller-actionable (exit 1); the `reconcile`
  help describes the rule; the JSON gains the fields above next to the Run view.

## Acceptance

- Run 1 `OUTCOME_UNKNOWN`, Run 2 refused before any request, both when it
  starts afterwards and when it was waiting for approval:
  `a_new_run_is_refused_before_any_request_while_an_earlier_filing_is_unknown`;
  through the CLI: `an_unknown_filing_refuses_a_new_run_and_reconcile_records_the_answer`.
- One "not found", a second on the same day, over a weekend, and from Maundy
  Thursday over Good Friday to Easter Monday are not enough:
  `not_found_answers_without_a_bank_day_between_do_not_release_the_period`;
  across midnight: `two_answers_across_midnight_are_not_a_bank_day_apart`.
- Two "not found" a bank day apart close Run 1 (filing and payment skipped, the
  answers in its journal) and Run 2 files once (two outbox attempts in all:
  Run 1's timeout and Run 2's receipt); a later resume of Run 1 changes
  nothing: `two_not_found_answers_a_bank_day_apart_close_run_1_and_run_2_files_once`.
- "Found" adopts and sends nothing, even after an earlier "not found":
  `a_found_return_is_adopted_and_nothing_is_sent`.
- A timeout, a 500 and an unreadable body record nothing; another status starts
  the count over: `a_failed_or_unreadable_read_records_nothing`.
- Calendar tests: Easter-dependent holidays (2025, 2026, computus checked for
  2024–2027, 2038, 2285), Midsummer Eve (2025–2029), fixed holidays, Christmas
  and New Year boundaries, DST switches, and `one_bank_day_after`.
- Guide (fi + en): `tasks/file-vat-direct.md` (VAT filing section),
  `concepts/filings.md`, `troubleshooting.md`, `reference/error-codes.md`.
  `skills/vero-fi` describes the recovery and that the agent must not route
  around it.

Gate: `scripts/ci-local.sh` fails only at `cargo deny` on RUSTSEC-2026-0543
("chrono is unmaintained"), which fails `main` identically and comes from no
change here (the lock file and manifests are unchanged). The other steps were
run by hand from the same script: fmt, clippy `-D warnings`, build and test on
stable and 1.89.0, and `scripts/check-user-guide.sh`, all green. Review:
`review.md` and `assessment.md` beside this file.

## Decisions for later review

- **"At least one bank day after" means the same Finnish clock time on the next
  bank day, not just a later date.** Friday 10:00 → Monday 10:00; Monday 23:59 →
  Tuesday 23:59, never before the next bank day starts. Alternative: any time on
  the next bank day (the first implementation), rejected after review because
  23:59 and 00:00 would close a Run minutes after sending, while a "not found"
  right after sending proves nothing. Stricter, so it can only delay a refiling,
  never cause a second one. Days in between are counted as whole 24-hour days, so
  across a summer-time switch the clock time can shift by an hour.
- **Finnish time is computed in code (UTC+2, UTC+3 under the EU rule), not with
  a time-zone database.** The rule has been stable since 1996; a new dependency
  for one zone did not seem worth it. If the EU drops summer time this function
  changes.
- **Any answer other than `Missing` or a match starts the count over.** For
  example `Being Processed` or a processed return with other figures: the
  authority shows something for the period, so earlier "not found" answers no
  longer describe it. Such a Run stays parked; the user compares the return in
  OmaVero.
- **An interrupted Run (filing `Executing`, or `Succeeded` without `vat_filed`)
  also blocks new Runs, and `reconcile` does not resolve it yet.** That recovery
  is `perfectly-uppity-knife`. Until it lands, the exit is `complete-manual`
  with `--remote-status-evidence-ref`. Before this change a new Run could file
  such a period, which could be a duplicate. Because the rule is shared,
  `complete-manual` now also asks for remote-status evidence in this case.
- **Refiling after a close can only send the same request.** The broker binds
  every attempt under `submit-filing:<period>` to the first attempt's bytes.
  If the books changed in between (a correction voucher), the new Run is refused
  before anything is sent, and the message points to filing manually with the
  recorded "not found" answers as evidence. Alternative: a per-Run outbox key,
  rejected because the shared key is what prevents a double filing.
- **A closed Run shows `Completed`**, with `submit-filing` and `submit-payment`
  skipped and the reason `NOT_FILED: … answered Missing on … and on …`. No new
  Run status was added; `file-vat status` then answers `not_filed`.
- **The broker change (V2) is outside the brief's main file list** but within
  its "only if needed" allowance: adoption on a body without a status would mark
  a period filed on no evidence.
