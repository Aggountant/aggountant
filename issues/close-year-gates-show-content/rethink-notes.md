# Rethink notes (2026-11-16)

Rewrote the issue with `/rethink-instructions` so the product owner can read
the user problem first and an implementer gets what the source does not make
obvious.

## Changed

- **Structure.** Removed the empty `## Description` heading and the second H1
  inside the body. The body now opens with what goes wrong for the user in plain
  words, without type names. The title says "approval steps" and "what the user
  approves" instead of "statements they bind": the gap covers all four
  approvals, not only the statements.
- **Verified against source and corrected or sharpened:**
  - The `ApproveAdjustments` hash covers depreciation lines and accrual inputs
    only. D7 corrections are outside it and are sealed before the gate can be
    approved (`assert_corrections_sealed`). The old text implied the
    corrections were part of what that gate binds.
  - `start` returns the depreciation lines but not the accruals. That still
    holds.
  - The tase-erittelyt and liitetiedot are built at gate-open time and thrown
    away after hashing. They first become visible in
    `archive/<year>/tilinpaatos.json` after sealing. The old text did not say
    this.
  - The monthly-report workaround is real, but it is a different projection
    from the bound statements, and it does not cover the breakdowns or notes.
    The old text presented it as a full workaround.
- **Added knowledge an implementer lacks:** show the bytes that were hashed,
  not a recomputation (the reason the Preview gate exists); the TE/LT content
  is rebuilt from the ledger, so it has to be persisted or checked against
  the hash; the open gate must be readable in a later session, and `resume`
  drives the run forward, so it is not a read command; the tase-erittelyt
  construction is already duplicated, and display would add a third copy.
- **Acceptance criteria** added: content per gate, a hash-equality test per
  gate, readable after resume, one tase-erittelyt builder, the user guide
  updated in fi and en with exact files.
- **Left to the implementer, on purpose:** whether to attach the content to
  every output or to add `close-year show`, and whether the adjustments view
  lists the already-sealed corrections.

## Dropped

The line "Found while fixing `close-year-adjustment-attribution`; the user
guide listed it as the second reason…" went. The guide no longer has that
warning, and the origin is already in the frontmatter `source_ref`.

## Separate defect filed

`close-year-correction-block-names-wrong-gate` (bug, intake, untriaged):
`assert_corrections_sealed` refuses `ApproveAdjustments` with a message that
names `ApproveStatements`. It is left over from before corrections moved
ahead of `ApproveAdjustments`.
