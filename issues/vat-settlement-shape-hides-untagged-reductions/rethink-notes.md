# Rethink notes (2026-11-16)

The issue body was rewritten with `/rethink-instructions` so that the product
owner can approve it and an implementer can build it.

## What changed and why

- **Opens with the user's problem.** The old text started from
  `classify_vat_account_postings`. The new one first says what goes wrong for
  the company: a correction booked without a VAT code passes as a VAT payment,
  and the return is filed wrong without a word.
- **Written around the 2026-11-16 decision.** The old "Possible fix" offered a
  chart role or a voucher marker. That choice is made (chart role), so the
  marker option is gone and the role is specified: what it means, which
  accounts carry it (1910 and 1920; not 1900, not 1730; the bundled chart has
  no tax-account account), and what it is checked against.
- **Corrected a misleading premise.** The old text said a settlement would
  need "its reduction paired with such a leg". The advance-refund example
  already has a 1910 leg, so pairing alone closes nothing. The rule is now:
  every non-VAT posting is a settlement-role leg or an interest or penalty line.
  The disqualifying leg (2620, 1150, 2000) is what makes the difference.
- **Worked out what `home upgrade` means here.** The upgrade creates only
  missing files, and every existing home already has `vat-account-roles.json`.
  A field in that file would never reach an existing home (it would be
  reported as `differs` forever), so the issue asks for a separate,
  chart-version-keyed file that the upgrade can add.
- **Defined behaviour without a role.** Fail closed: the payment is flagged,
  the existing acknowledgement lets filing continue, and the preview names the
  missing roles and `home upgrade` instead of looking like broken books. The
  home still opens.
- **Raised the frozen-roles rule.** `crates/ledger/AGENTS.md` says role changes
  need a new chart edition. The issue explains why a detection-only role does
  not breach that rule's purpose, and asks for the scope to be written into the
  rule.
- **Said honestly what stays open.** Gap 2 of the old text (an expense-booked
  refund in the same voucher as a larger payment) cannot be told apart by a
  bank role. It is now listed as not closed, to be pinned by a test.
- **Sharp acceptance criteria** replace the absent ones. The empty
  `## Description` heading and the duplicated H1 are gone, and the title now
  states the user effect.

## Verified against source

- The settlement test in `settles_vat`, its callers (`Ledger::untagged_vat_warnings`,
  `process/src/vat_reconciliation.rs`) and the `Settlement` class are as
  described. I traced all three gap-1 examples and the gap-2 example through
  `settles_vat` by hand. Each is classified `Settlement`.
- The bundled role table has one chart (`fi-general-2026@dd5d3f33…`), and its
  chart has 1900, 1910, 1920 and 1730 but no tax-account account.
- `GitMasterDataRefStore::upgrade` reports an existing file with other bytes as
  `Differs` (CLI: `skipped`, reason `differs`). It adds role data only when the
  bundled table activates the home's own active chart.
- No existing test pins a gap-1 shape as a settlement. The test `kind_of` does
  not know 2620, 1150 or 2000.

## For the product owner

- Gap 2 stays open under this decision. Closing it needs either a role for
  interest and penalty accounts or a voucher marker.
- Fail-closed means a company on its own chart, which gets no bundled roles,
  acknowledges its VAT payment every month until it has settlement-role data.
- The `collision` field names only `crates/domain/src/vat/reconcile.rs`. The
  work will also touch `crates/ledger/src/store/git_masterdata.rs`. I left the
  field alone as the brief required, but whoever schedules the work should
  consider adding it.

No separate defect was found, so nothing was filed.
