# Result

The refusal from `assert_corrections_sealed` now names `ApproveAdjustments`
(via the `GATE_APPROVE_ADJ` constant, so it cannot drift from the gate again),
keeps the stable `correction:<id>` keys, and tells the user to seal each one with
`aggountant close-year seal-correction --run-id <run> --correction <id>`, with the
run id filled in. The test `a_d7_correction_must_be_sealed_before_approve_adjustments`
asserts the gate name, the absence of `ApproveStatements` and the seal hint.

The user guide does not quote the message, so it is unchanged.
`scripts/ci-local.sh` passed. Review: focused self-review only, as the brief
allowed for this message-only change; no bookkeeping behaviour changed.

## Decisions for later review

- The hint shows `--correction <id>` with a note that `<id>` is the part after
  `correction:`, instead of one full command per correction. Alternative: list a
  ready command per correction; rejected to keep the message short when several
  are unsealed.
