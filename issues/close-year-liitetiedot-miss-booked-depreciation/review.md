# Review: close-year guards (three issues, combined diff)

- Scope: `5fb11fb..HEAD` over `crates/process/src/close_year.rs`,
  `crates/process/tests/close_year.rs`, `docs/user/*/tasks/year-end.md`; the
  fixes for `close-year-seal-after-year-sealed`,
  `close-year-correction-on-rejected-run` and
  `close-year-liitetiedot-miss-booked-depreciation`.
- Rigor: critical (irreversible `SealYear`, data integrity of sealed
  statements). Risk class: irreversible bookkeeping state.
- Finder: gpt-6-astra (other provider than the host agent). Verifier:
  claude-fable-5-2, one batched call (the surviving candidate plus one
  focused false-negative question on post-answer refusals). Two model calls.

## F1 (must-fix, pre-existing) — an unattributed close-year voucher escapes the next run

Location: `crates/process/src/close_year.rs` `seal_attributed`,
`attributed_natural_keys`, `assert_no_unplanned_attributions`.
Claim: a Run that stops between `Ledger::seal` and `attribute_adjustment` and is
abandoned leaves a sealed, unattributed `closeyear:<year>:` voucher; a later Run
without that entry passes every check and seals the year without it.
Evidence: local reading (`fiscal_year_data` defaults to posting year;
`seal_year` has no such check); verifier: survives, pre-existing, not changed by
this diff. Impact: wrong result in two years, narrow trigger (crash window plus
abandoning the Run plus omitting the entry). Fix: out of this change's scope;
filed `close-year-unattributed-voucher-escapes-next-run`.

## F2 (should-fix, introduced) — the compute-stage check wedged a resumed run

Location: `run_compute_statements` (the added `assert_no_unplanned_attributions`).
Claim: after `ApproveAdjustments` is journaled and the process stops, another
Run's attributed correction makes the resume refuse with no unanswered gate.
Evidence: reproduced by `a_resumed_run_with_an_entry_outside_its_plan_can_still_be_rejected`
(fails with the check in place). Fixed in `fa896f3`: the check is removed from
that stage; the approval precondition and the statements binding still refuse.

## F3 (discuss, introduced, accepted) — the binding check can refuse on the SealYear resume path

Location: `assert_statements_binding` reached from `run_seal_year` on resume.
Claim (verifier): `ApproveStatements` is answered, `SealYear` fails, another Run
then attributes a voucher outside the plan that leaves every hash equal (a
correction between two expense accounts on the same income-statement row);
resume refuses with no open gate. On base the year would be sealed with an
entry no gate showed. Refusal is the data-integrity-preserving outcome; the
user starts a new Run, which includes the entry and seals.

## F4 (info, introduced wording) — remediation text and changed figures

The unplanned-entry message told the user the re-supplied entry's seal
replays; with different figures the ledger refuses (`IDEMPOTENCY_KEY_REUSE`)
after `ApproveAdjustments` is answered (pre-existing post-answer refusal). The
message now says the figures must be the same and that different figures are
refused when the adjustments are sealed.

## Dropped / not findings

- `assert_year_open` inside `seal_attributed` after the answer (verifier item
  2): the same condition is checked before the answer, so it only fires when
  another process seals the year in between — the multi-process case outside
  the single-user model; a later resume records `SUPERSEDED_BY_ANOTHER_CLOSE`.
- Other post-answer refusals the verifier listed (prior year not sealed,
  unbalanced statements, hash refusals on resume): exist on the base commit
  unchanged; not part of this change.

## Coverage

Read: the whole `close_year.rs`, the ledger's seal/attribute/seal_year/
fiscal_year_data paths (verifier), the tests added here. Ran: the full
workspace test suite on stable and 1.89. Not covered: the CLI layer (no change).
