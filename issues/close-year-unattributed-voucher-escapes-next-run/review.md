# Review: close-year refuses an abandoned run's unattributed voucher

- Target: commit c5bf08b (`crates/ledger/src/lib.rs`,
  `crates/process/src/close_year.rs`, its tests, `docs/user/*/tasks/year-end.md`)
- Skill: `/llm-review`, rigor critical (the change guards the irreversible
  `SealYear`), risk class: data integrity
- Finder: gpt-6-astra (one call). Verifier: claude-fable-5-2 (one focused
  false-negative call, because the finder's only candidate was dropped locally
  and the surface is high-risk). Two model calls in total.

## Confirmed findings

None.

## Dropped candidates (2)

### F1 — a run interrupted after `ApproveStatements` is left with no gate to reject (finder, must-fix)

Location: `crates/process/src/close_year.rs` `assert_statements_binding`,
reached from `run_seal_year` on resume. Sequence: Run B approves
`ApproveStatements`, the process stops before `SealYear`; Run A then seals a
voucher and stops before attributing it; resuming B is refused and B has no open
gate to reject.

Local check: true as described, but not introduced by this change and not a
books defect. `run_seal_year` already re-ran the whole binding before this
change, so the same wedge exists today for a voucher another run attributed
(`assert_no_unplanned_attributions`) and for stale statements
(`a_resumed_run_does_not_seal_statements_another_run_has_made_stale` leaves Run A
in exactly this state and then completes Run B). The year stays open, nothing is
sealed or attributed, and once another run seals the year, resuming B records
`SUPERSEDED_BY_ANOTHER_CLOSE`. Needs two interruptions at specific points.
Dropped (rare, end result correct).

### F2 — a hand-sealed `closeyear:<year>:…` voucher would block the close (verifier side note)

`voucher seal` accepts a caller-chosen natural key. A voucher sealed by hand
with a `closeyear:<year>:` key is unattributed and unplanned, so it blocks the
close rather than slipping through it. Nobody seals with that key by hand (the
prefix is the close-year run's own namespace) and the failure is a loud
refusal, not wrong books. Dropped; recorded under the result's decisions.

## Verifier answer (false-negative probe)

Asked whether any supported sequence (start, approve/reject, seal-correction,
resume, interrupted between any two ledger commits) lets `Ledger::seal_year`
succeed while a `closeyear:<year>:` voucher is unattributed, or seals statements
missing a close-year voucher no gate showed. Answer: no sequence found.
`seal_year` has one production caller, `run_seal_year`, which runs the full
binding first; together the three checks (planned keys attributed, nothing
unplanned attributed, no unplanned orphan) and unique natural keys mean the
vouchers attributed to the year are exactly the plan and no prefixed voucher is
unattributed. Prefix collisions (`closeyear:2026` vs `closeyear:20261`) cannot
match because the prefix ends in a colon; a voucher attributed to another year
is unreachable because the only caller of `attribute_adjustment` uses the Run's
year and attribution is immutable.

## Coverage

Read: the diff, `close_year.rs` in full around every call site of the guard,
`Ledger::seal_year`, `attribute_adjustment`, `year_vouchers`,
`fiscal_year_data`'s attribution rule. Tested: both new regression tests fail on
the previous `close_year.rs` and pass now; the full `close_year` suite and
`scripts/ci-local.sh` pass. Outside scope: the process journal/fold mechanics
and the wedged-run recovery path (pre-existing).
