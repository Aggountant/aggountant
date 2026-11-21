# Result: the month's bookkeeping skill

`skills/capture-receipt/SKILL.md` is now the skill for a whole month, in the
order the work happens: material, receipts (with the contra-account section),
bank statement, special entries and their VAT marking, reconciliation, close,
hand-over to the filings.

## Words

| | Words |
|---|---:|
| Before | 1501 |
| Draft sent to review | 1 149 |
| After the review fixes | 1220 |

The target was about 1 100. The file ends about 100 over because the review
found twelve places where the shorter draft would have led an agent wrong, and
each fix is a sentence of knowledge, not procedure. About half of the text is
new subject matter (bank import, reconciliation, close) that no skill covered
before.

## Name: kept

The skill is still `capture-receipt`. A rename would leave the old skill
installed and still routing in every existing agent, because
`skill install` never deletes anything, so users would have two month skills
with different text. The other skills, the guide and the tests refer to it by
name, and two of those files were being rebuilt by other workers at the same
time. The name's cost is only that it undersells the content, and routing is
done by the description, which now names the month, the bank statement, the
special entries, reconciliation and the close. A rename becomes reasonable once
the installer can retire a skill.

## What moved where

- The receipt input format, the list of VAT options and rate classes, the
  preview and approval commands, and the assignment example: gone from the
  skill, because `capture-receipt record --help` and
  `import camt053 seal --help` now carry them (`cli-stdin-input-help`).
- "Mistä aineisto tulee", the `TODO.md` section, the
  `knowledge company` / `obligation` distinction, secrets, the approval
  principle and "Käyttäjän aika": gone, SOUL has them. The skill keeps the two
  points where they bite in this task: a batch approval shows every line, and
  the close proposal is assembled by the agent.
- Special entries' VAT marking: from a 150-word bullet to its own section.
- The contra-account section: kept. Its month-end paragraph moved to the new
  reconciliation section, the assignment JSON went to the help, and it gained
  two review fixes (default-chart numbers; what to do with a receipt whose
  payment was already booked as an expense).

## What was dropped, and why

- The consultation route for unusual receipts (`capture-receipt start`): not
  connected to anything, per the guide's status page.
- The hash's contents, `projection_valid`, `ledger_preflight_valid`,
  `replayed`, `CAPTURE_PREPARATION_CHANGED`, `NATURAL_KEY_ALREADY_SEALED`,
  `MISSING_ASSIGNMENT`: the program's output says these when they happen.
- "Reverse charge on a missing role pair is the right outcome because
  hard-coded accounts would break chart ownership": implementation reasoning.
- The `home upgrade` hint for missing settlement roles: `vero-fi` has it,
  at the point where the warning appears.

## What is new

- Bank import: every line needs an assignment or the statement is refused;
  the import seals at once with no preview, so the list shown to the user is
  the approval; check account and dates from `parse` before sealing; sales
  VAT is not expanded; how a VAT payment line is assigned; only Holvi tested.
- `eu-goods` on the default chart: there is no correct route today. The skill
  says so, says why the two tempting workarounds file the tax in the wrong
  fields, and what to tell the user.
- Reconciliation: the two balances, what each kind of difference means, and
  that they are necessary but not sufficient.
- Close: `run start close-period` is not a preview and locks the month at
  once; the period closed is the VAT period, and months and quarters do not
  mix within a year; what blocks it; what the agent shows before approval;
  reject reopens; an interrupted close is answered, not restarted.

Every command, flag, field and code the text names was checked against the
binary built from this worktree, by running a synthetic month (receipt, refused
`eu-goods`, tagged and untagged special entries, statement import and replay,
report, close with reject and approve, quarter refusal) in a scratch home.

## The close, once `virtually-tidy-owl` lands

Today the approval gate shows a hash, so the skill tells the agent to assemble
the proposal itself: which month, how many vouchers, the result and bank
reconciliation from the report, the state of payables, and what stays open.
When the gate carries that content, the paragraph starting "Hyväksyntäportti
näyttää vain tiivisteen" shrinks to one sentence: show the user what the gate
returns, and add what the program cannot know (the open items in `TODO.md`
and whether the bank balance matches the statement). If the gate also gets a
`next_command`, the mentions of `run answer` and `period show` go too.

## Product gaps (spinoff proposals)

Filed unlaned through intake; each names the skill lines to remove when fixed.

- `default-chart-eu-goods-target`: EU goods purchases cannot be recorded on
  the default chart, and the refusal does not say what to do.
- `camt-import-preview-and-balances`: the bank import has no dry run or
  approval hash, and `parse` does not show the statement's period and
  balances.
- `vat-reconciliation-before-close`: the VAT reconciliation runs only on a
  sealed period, so an untagged VAT posting is found when it can no longer be
  corrected there.
- `voucher-vat-code-help`: no help says which `vat_code` values exist or
  where the code goes.

Also filed: `employer-report-test-stdin-flake`, the test flake described under
Checks.

Not filed, because `virtually-tidy-owl` is the same place: a second
`run start close-period` on a month already closing fails with
`PERIOD_TRANSITION_CONFLICT` and no `next_command` pointing at the waiting
run.

## Review

`review.md` (one model, executor and edge-case lenses) and `assessment.md`:
17 findings, 12 fixed, 5 dropped with reasons.

## Checks

The packaging and skill tests pass. `scripts/ci-local.sh` failed once in a test
this change does not touch (`employer_report_history_cli`, a broken pipe when
the test writes to the stdin of a child that has already exited); eight reruns
of that test passed and it is filed as `employer-report-test-stdin-flake`. The
second full run's result is in the commit that closes this issue.
