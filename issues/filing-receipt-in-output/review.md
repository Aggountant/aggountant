# Review: filing-receipt-in-output

- Skill: `/llm-review`, rigor **routine**: the change is a read path (it adds
  a field to Run views), but it reports authority filing outcomes, so a wrong
  `accepted` or a misattributed receipt would be silently wrong bookkeeping
  evidence.
- Scope: commit `01f77ac` against its parent (`crates/process/src/authority.rs`
  new; `lib.rs`, `file_vat.rs`, `file_vat/recovery.rs`, `file_wage.rs`,
  `file_employer_report.rs`; `crates/cli/src/main.rs`, `wage.rs`; tests, user
  guide, `skills/vero-fi`).
- Finder: `gpt-6-astra`, one call (the diff plus the issue, `authority.rs`,
  `file_wage.rs`, `file_employer_report.rs`, `file_vat/status.rs`). Verifier:
  none. The one surviving candidate was confirmed by an executed test before
  any second model would have seen it, and the other was judged on
  likelihood, not on contested evidence. One model call in total.
- Risk class: authority filing evidence (receipt attribution).

## F1 (must-fix → fixed): a VAT receipt could be attributed to a Run that never sent it

- Location: `crates/process/src/authority.rs`, `vat_accepted` (looked up the
  Ledger receipt by period only).
- Claim: `resume_file_vat` marks an approved Run's filing effect succeeded
  whenever fiscal shows the period filed, without comparing request bytes
  (`file_vat.rs`, the "fiscal wins" reconcile branch, which says so in its
  comment). Run A approved and crashed before dispatch (effect `Planned`, which
  does not block a new Run); Run B files the period with different bytes;
  resuming A adopts fiscal, and the new `outcome` showed A as accepted with B's
  receipt.
- Evidence: local read of `resume_file_vat` and `unresolved_filing` (a
  `Planned` effect is not unresolved). Reproduced by test
  `a_receipt_is_not_attributed_to_a_run_whose_request_it_does_not_acknowledge`
  (fails on the period-only lookup).
- Fix: `ProcessEngine::vat_receipt_of_run` in `file_vat/status.rs` attaches
  the receipt only when the Run is the one `filing_run_with_receipt` (the rule
  `file-vat status` already uses) picks or bound identical request bytes;
  otherwise the outcome is `accepted` without a receipt and `next_command`
  points at `file-vat status`. The adoption itself still marks A succeeded;
  that predates this change and is the follow-up the code comment names.

## Dropped (1)

- **`show` now opens the Ledger for a succeeded `file-vat` filing, so it can
  fail after an irreversible payment submit or migrate an old home.** True
  that `run show` of a filed `file-vat` Run now calls `Ledger::open`. Every
  home that reaches a VAT filing has already opened the Ledger (start, approve
  and the filing itself require it), every other command opens it too
  (`file-vat status` included), and the direct Vero route has no payment
  adapter, so the "fails after payment" path has no realistic trigger. Dropped
  on likelihood.

## Coverage

Read: the whole diff; `resume_file_vat`, `run_submit_filing`,
`run_submit_payment`, `unresolved_filing`, `filing_run_with_receipt`,
`file_vat_filing_runs`; the Incomes Register receipt format in
`broker/src/tulorekisteri.rs`; `EmployerReportReceipt::parse` and `classify`.
Tested: workspace tests and clippy through `scripts/ci-local.sh` (stable and
MSRV). Outside scope: `run-payroll` Report effects (not authority filings of
the issue; they get no `outcome`).
