# Review: employer-separate-report

- Skill: `/llm-review`, rigor **critical** (authority filing to the Incomes
  Register: resubmission fence, uncertain outcomes, irreversible submissions).
- Scope: the whole change against `main` at `e82307d` (broker
  `tulorekisteri*.rs`, `crates/process/src/file_employer_report.rs`,
  `crates/domain/src/payroll.rs`, `crates/cli/src/wage.rs`, tests).
- Finder: `gpt-6-astra`, one call. Verifier: `claude-fable-5-2`, one call (two
  surviving must-fix candidates; asked to refute them and to judge the fixes).
  Two model calls in total.

## F1 (must-fix): stale e-service evidence settled an in-doubt replacement

- Location: `crates/process/src/file_employer_report.rs`,
  `complete_employer_report_manual`, in-doubt branch.
- Claim: for an in-doubt replacement only `ir_report_id` was compared. A
  replacement keeps the register's report reference, so evidence of the *old*
  version (or no version) marked the replacement accepted, released the fence
  and made an unarrived replacement the month's current report.
- Evidence: local read of the pre-fix code (`receipt.version == Some(0)` was the
  only version check); the broker's own expectation for replacement feedback
  (`version = replaced + 1`, `tulorekisteri.rs` `ExpectedItem::employer`).
  Verifier: survives. Verifier added the case `replaces_version == None`, where
  any version ≥ 2 could still be stale.
- Fix: settling a replacement needs a version newer than the replaced one; when
  the replaced version was never recorded, e-service evidence is refused and
  only the signed status query can settle it. Tests
  `e_service_evidence_settles_a_replacement_only_with_a_newer_version`,
  `e_service_evidence_cannot_settle_a_replacement_of_an_unknown_version`.

## F2 (must-fix): a replacement from "No wages payable" to EUR 0 dropped the pension insurances

- Location: `plan_employer_report` replacement guard;
  `EmployerSeparateReport::sent_pension_insurances`.
- Claim: pension insurances were emitted only with code 101, and the guard only
  compared them when both versions were 101. A 101-with-policy → 102
  replacement silently dropped `PensionInsurances`.
- Evidence: official processing rule, *Data delivery – Schemas – Employer's
  separate reports 2025* v1.0 §2.9: "If the 'Action type' is 'Replacement
  report', the data should match that of the report's previous version." The
  schema makes `PensionInsurances` optional with no 101-only restriction.
  Guidance VH/6456/05.00.00/2025 §4.2 says wages paid after a "No wages
  payable" report are corrected with a replacement, and §5 requires
  cancel-and-new only for a changed pension provider or policy. Verifier:
  survives; recommended carrying the group forward rather than refusing.
- Fix: a replacement repeats the previous version's sent pension insurances
  (a contribution replacement carries them forward); a replacement whose
  pension insurances would differ from the previous version is refused
  (`EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED`). Schema test covers a 102
  replacement with `PensionInsurances`.
- Residual uncertainty: whether the register accepts code 102 together with
  `PensionInsurances` was not tested against the register (no test-environment
  access from here). If it does not, the outcome is a definitive rejection, not
  a silently wrong filing.

## Dropped / not fixed

- F3 (should-fix, finder): `deadline` is always the 5th and `overdue` compares
  against it, so a month can be shown overdue a day or two early when the 5th
  is a weekend or holiday. Not a data-integrity defect: the documented,
  conservative date never lets a report be late. Recorded as a decision in
  `result.md`.
- Verifier note: a replacement of a report recorded from e-service evidence
  (`report == None`) skips the pension comparison. The previous version's
  insurances are unknown to aggountant; a mismatch is a definitive register
  rejection. Left as is.
- Verifier note: an e-service cancellation cannot be recorded, so after
  cancel-and-new the month keeps the cancelled report as current. Pre-existing
  gap outside this slice (the issue puts deletion out of scope); filed as
  `employer-report-record-cancellation`.

## Coverage

Read: the full `file_employer_report.rs`, the broker diff around the employer
operation, `payer_summary_record`, the domain report type, the CLI wiring, and
the process tests. Not covered by any model: the user guide prose and the
skill text (checked by hand and by `scripts/check-user-guide.sh`).
