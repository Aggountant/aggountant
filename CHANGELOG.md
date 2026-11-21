# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While the version is 0.x, a minor version bump may contain breaking changes.

<!-- oss-changelog:unreleased-start -->
## [Unreleased]

### Added

### Changed

### Fixed
<!-- oss-changelog:unreleased-end -->

## [0.2.0] - 2026-11-21

### Added

- **VAT filing: correcting a filed return.** `file-vat replace` sends a
  replacement return for a period whose return is filed, with one or more
  reasons (`CLC`, `LGL`, `TXA`, `LAW`), through the same approval gate as a
  first return; the gate shows the amounts the replacement changes. A
  correcting voucher says which return it belongs to with the new draft field
  `corrects_vat_period`, decided at the seal; without the field the voucher's
  VAT stays in the return of the period it is posted in. `file-vat status` and
  `preview` list every filing of a period and the corrections no filing covers
  yet. `file-vat complete-manual --replacement` records a replacement made in
  OmaVero. The replacement run pays nothing; the difference is settled on the
  tax account.
- **VAT filing: reconciliation before the close.** `file-vat preview` runs on
  an open or closing period and reconciles its VAT accounts, so an untagged
  VAT posting is found while it can still be corrected in the same period. New
  output fields `period_state` and `fileable`.
- **VAT filing:** `voucher vat-codes --document-date YYYY-MM-DD` lists every
  `vat_code` a posting may carry on that date with its rate; `voucher draft`
  and `voucher seal` help explain where the code goes.
- **VAT filing: EU goods purchases on the default chart.** A receipt or bank
  row with the reverse-charge scope `eu-goods` can be recorded on the chart
  `init` creates, and its figures land in the EU goods fields of the return.
  Existing homes get the target with `home upgrade`.
- **Incomes Register: corrections of an accepted travel report.**
  `file-wage correction preview|submit|status|complete-manual` replaces or
  cancels an accepted report through aggountant: previewed offline, approved
  by record hash, sent once, and an unknown outcome is settled by a status
  query, never by resending. `complete-manual` records a correction made in
  the e-service.
- **Incomes Register: profile commands.** `tulorekisteri status` reports
  offline what the channel still needs, each blocker with a code, an owner
  and what to do; `tulorekisteri configure` writes the channel profile as a
  merge, like `vero-fi configure`. `file-wage employer-report configure`
  writes the employer's separate report settings and checks the registration
  month against the employer register in PRH/YTJ open data. The profile no
  longer needs to be edited by hand.
- **Incomes Register:** `file-wage preview` and `file-wage start --dry-run`
  mark the `record_hash` as sensitive in their output (`sensitive`): it is
  derived from the personal identity code.
- **Bank import: preview bound to the approval.** `import camt053 seal
  --dry-run` returns every voucher it would seal and a `preparation_hash`
  without writing anything; `--approved-preparation-hash` refuses the seal
  (`IMPORT_PREPARATION_CHANGED`) unless it prepares exactly those vouchers.
  `import camt053 parse` shows each statement's period, opening and closing
  balance and whether the entries reconcile with them.
- **Month close: the approval shows its content.** `run preview close-period`
  shows, without changing anything, what a close would lock: the vouchers,
  the period's income statement and result, the closing balances, open
  obligations and what blocks the close. The `ApproveClose` gate carries the
  same content, and an approval is refused with `CLOSE_CONTENT_CHANGED` if the
  books changed after the gate opened.
- **Year-end close: fixed-asset register.** A sealed fiscal year stores its
  assets with their closing book values, and the next year's close carries
  them from there. An opening book value is typed once, when an asset enters;
  one typed again that disagrees with the sealed year is refused. A disposed
  asset is named in the new `removed_assets` input. The first close under
  0.2.0 takes its assets from the input as before, because years sealed
  earlier have no register.
- **Year-end close:** `close-year start --dry-run` shows the depreciation
  method, rate and amount per asset, the carried register and everything that
  would block the start, without creating a run. It also works before the
  year's months are closed.
- **Year-end close:** every `close-year` command returns `message` and
  `next_command`, naming the open gate and what approving it seals
  (`start --dry-run` returns only `message`).
- **Agent surface:** `aggountant explain <CODE>` explains an error code from
  the guide's error-code reference (`--lang fi|en`), offline; error JSON
  carries the guide's "what to do" as `hint` (in Finnish).
- **Agent surface:** every command that reads JSON on stdin describes its
  input in an INPUT section of `--help`, with a synthetic example.
- **Agent surface:** a command that used the `AGGOUNTANT_BROKER_SCRIPT` test
  seam marks its JSON output and its JSON error line with `test_seam`, and
  without `--json` warns on stderr, because in that mode a period can be
  marked filed though nothing was sent.
- **Skills:** `close-year` (the year-end close), `tulorekisteri` (travel
  allowance reports and the employer's separate report) and `e-services`
  (working OmaVero and the Incomes Register e-service in the user's browser).
  Skills can ship reference files besides `SKILL.md`; `skill print <name>
  --file <FILE>` prints any of them, and `skill list --json` lists each
  skill's files and requirements.

- ADR-10 slice 1: multi-file bundled skills and the e-services skill (`eservice-skill-bundle`).
- Error hints and aggountant explain <CODE> from the guide's error-code reference (`cli-error-hints`).
- Fixed-asset register: carry assets and book values from one close-year to the next (`fixed-asset-register`).
- Replacement VAT return through file-vat for an already filed period (`vat-replacement-return`).
- Skill rebuild: a close-year skill for the year-end close (`skill-close-year`).
- close-year: show depreciation rates and computed depreciation without starting a run (`close-year-depreciation-preview`).

### Changed

- **Receipts and bank rows.** A receipt or purchase invoice that is also paid
  from the bank account is now recorded against 2700 Ostovelat (2740 when the
  owner paid), and the bank statement row is assigned to the same account to
  clear it, so the purchase is booked once. The `capture-receipt` skill, the
  user guide and the practice month follow this one convention; previously the
  skill recorded receipts against the bank account and the guide left the
  statement row undefined.
- **Skills rebuilt.** `vero-fi` is the VAT skill only; the Incomes Register
  moved to `tulorekisteri`. `capture-receipt` now covers the whole month:
  receipts, the bank statement, special entries, reconciliation and the close.
  The skills are shorter and leave input formats, state names and error codes
  to the program's help and output.
- **Set-up always uses the default chart.** The previous system's accounts are
  mapped onto it in the opening voucher; the skill and the guide no longer
  offer keeping the old chart, which nothing could install.
- `vero-fi configure` merges into the stored profile and reports every changed
  field with its value before and after; `--replace` writes the input as the
  whole profile.
- `close-year answer` requires `--answer`. `close-year` help is written in
  plain language: what each gate seals, what a correction is, and that
  `resume` finishes what was approved, the year seal included.
- `import camt053 seal` validates the whole batch before the first seal, and
  never reports a statement with entries in several currencies as reconciled.
- A replacement of an employer's separate report that was given outside
  aggountant is built from the original's pension insurances as the user
  states them, not from the current profile.
- The Incomes Register readiness check names every unmet requirement instead
  of one sentence.
- `report statements` for a month of a new fiscal year still waits for the
  previous year's seal, and the refusal now says why and what lifts it.
- `MASTERDATA_CORRUPT` names `home upgrade` when the upgrade would add the
  file the home lacks.
- `knowledge company update` and `retire` take the fact id positionally as
  well as with `--id`, or as `id` in the stdin JSON (which must agree with an
  argument given alongside it).
- `chart list` text output lists the accounts (number, name, kind), one per
  line.
- `report statements` prints the report to stdout when `--output` is omitted
  or `-`, defaulting to `--format native-json`; with `--json` the summary
  object carries the document in `content`.
- Dates and times no longer depend on the unmaintained `chrono` crate. Stored
  and sent timestamps keep the same bytes; date arguments are parsed strictly
  as `YYYY-MM-DD`.

- Bank statement import has no preview, and parse hides the statement balances (`camt-import-preview-and-balances`).
- CLI friction from production use: knowledge company id, chart list output, report statements to stdout (`cli-production-friction`).
- Decide how e-service operating knowledge accumulates and reaches agents (`eservice-knowledge`).
- EU goods purchases cannot be recorded on the default chart (`default-chart-eu-goods-target`).
- Guide: the new year's monthly reports wait for the prior year's seal (`guide-new-year-reports-wait-seal`).
- Help does not say which vat_code values a manual voucher may carry (`voucher-vat-code-help`).
- INPUT section in --help for every stdin JSON command (`cli-stdin-input-help`).
- Incomes Register profile has no command, no help and no itemised readiness (`tulorekisteri-profile-no-command`).
- MASTERDATA_CORRUPT for a missing bundled file does not name home upgrade (`nominally-undesirable-afterthought`).
- Migrate from chrono to jiff or time (`migrate-from-chrono`).
- Month close approval gate shows a hash, not what the user approves (`virtually-tidy-owl`).
- Outputs mark sensitive record_hash and the active test seam; specific code for an unknown VAT class (`filing-output-markers`).
- Set-up offers keeping the previous chart of accounts, but no path installs one (`distinctly-tired-hearing`).
- Skill rebuild: SOUL as the one home of house rules and of how to read the tool (`skill-soul-rebuild`).
- Skill rebuild: split vero-fi into VAT and a new tulorekisteri skill with reference files (`skill-vero-split`).
- Skill rebuild: streamline aggountant-init (`skill-init-rebuild`).
- Skill rebuild: the month's bookkeeping skill, with the bank import and the month close (`skill-month-rebuild`).
- Skill writing principles in skills/AGENTS.md and a test that named commands exist (`skill-writing-principles`).
- VAT reconciliation is only available after the period is locked (`vat-reconciliation-before-close`).
- close-year help: plain descriptions instead of design-document labels (`close-year-plain-help`).
- close-year output: message and next_command naming the open gate and what its approval seals (`close-year-next-command`).
- employer-report: a configure command for the employer_report profile, with the YTJ registration check (`employer-report-configure`).
- vero-fi configure merges into the stored profile instead of replacing it (`vero-configure-merge`).

### Removed

- The default answer of `close-year answer`.
- Whole-profile replacement as the default of `vero-fi configure` (use
  `--replace`).
- The Incomes Register sections of the `vero-fi` skill (now in
  `tulorekisteri`) and the consultation route in the skills.

### Fixed

- A statement or report row that sums to zero on a credit-side account
  printed as `-0.00`; it now prints as `0.00` in the monthly report and the
  year-end statements. Sealed years are not recomputed.
- `file-wage complete-manual` on an already accepted report exited
  successfully without recording anything; it is now refused.
- A bank statement import that failed on a later entry left the earlier
  entries sealed and could leave the statement file stored without vouchers.
- A failed write of `archive/<year>/tilinpaatos.json` after the year seal went
  unreported. The year is sealed and the books are correct either way; the
  run now says the archive is missing and `close-year resume` rewrites it.
- `employee` help claimed an encrypted store. The employee data is in a
  separate database that `.gitignore` keeps out of git, and it is not
  encrypted at rest; the help now says so.
- The shipped skills named a command that does not exist and a consultation
  route that is not connected; the guide's error-code reference lacked seven
  codes.
- Refusals on the receipt-capture paths repeated their code
  (`VAT_CODE: VAT_CODE: …`).

- A correction made in the e-service cannot be recorded; file-wage complete-manual succeeds without recording it (`filing-correction-unrecordable`).
- Flaky company_knowledge_cli test: stdin write hits Broken pipe when the CLI refuses early (`company-knowledge-cli-broken-pipe-flake`).
- Flaky test: employer_report_history_cli writes to the stdin of a child that may already have exited (`employer-report-test-stdin-flake`).
- Guide says a bank row for an already recorded receipt is not recorded again, but the import requires every row (`bank-row-for-recorded-receipt`).
- Monthly report prints a cleared liability account as −0,00 (`report-negative-zero-balance`).
- Shipped skills: four wrong facts (journal show, consult route, SOUL unreachable through skill print, codes missing from the guide) (`skill-fact-fixes`).
- close-year answer defaults to approve, also on the gate that seals the fiscal year (`close-year-explicit-answer`).
- close-year: a failed archive write after SealYear is silent (`close-year-archive-write-error`).
- employee help claims an encrypted store; the store is plaintext (`employee-store-plaintext-help`).
- employer-report: replacing an externally filed month skips the pension-insurance check (`employer-replacement-external-unchecked`).

### Upgrading from 0.1.0

Upgrade when no month close, year-end close or filing is in progress
(`aggountant run list`), and take these steps in order.

1. **Back up the home first.** The first command that opens the books migrates
   `fiscal.sqlite` from schema v7 to v8 (the new fixed-asset register), and
   0.1.0 refuses to open a v8 store. The backup is the only way back: copy the
   whole home, including `fiscal.sqlite` and the `fiscal.sqlite-*` files next
   to it, as the guide's "Back up and restore your books" page describes.
2. **Install 0.2.0**, then run `aggountant skill install` and restart the
   agent. The skills were rebuilt for this version's commands: `vero-fi` now
   covers the VAT return only, the Incomes Register work moved to the new
   `tulorekisteri` skill, and `close-year` and `e-services` are new.
   `skill install` without a name installs all six and overwrites the old
   files in place; `skill install vero-fi` also brings `tulorekisteri` and
   `e-services`.
3. **Run `aggountant home upgrade --dry-run`, then `--apply`.** In a home on
   the default chart it updates `masterdata/reference/vat-account-roles.json`
   with the reverse-charge target for EU goods purchases. Only a file that is
   byte for byte what 0.1.0 wrote is updated; an edited one is left alone and
   reported as `skipped`. Every existing role stays, so sealed vouchers verify
   as before. Commit the changed file.

Commands that behave differently, for scripts and agent habits carried over
from 0.1.0:

- `close-year answer` has no default answer; `--answer approve|reject` is
  required. In 0.1.0 leaving it out approved the gate.
- `vero-fi configure` merges its stdin JSON into the stored profile: a field
  left out keeps its value and `null` clears it. In 0.1.0 the input replaced
  the whole profile; that is now `vero-fi configure --replace`. A merge cannot
  clear the recorded verification evidence
  (`VERO_EVIDENCE_WOULD_BE_DROPPED`).
- `file-vat preview` succeeds on a period that is not closed and returns draft
  figures with `fileable: false`. In 0.1.0 it failed with `PERIOD_NOT_SEALED`,
  so a caller that took a successful preview as "ready to file" must read
  `fileable`. `file-vat start` refuses an unclosed period as before.
- `file-wage complete-manual` on a report that is already accepted fails with
  `WAGE_REPORT_ALREADY_ACCEPTED`. In 0.1.0 it exited successfully and recorded
  nothing. Corrections go through `file-wage correction`.
- `file-wage employer-report preview|submit --replace` for a report that was
  given outside aggountant and recorded with `complete-manual` fails with
  `EMPLOYER_REPORT_PREVIOUS_UNKNOWN` until the original report's pension
  insurances are stated with `--previous-pension-insurance` or
  `--previous-no-pension-insurance`. For a report aggountant sent itself the
  flags are not needed and are refused.
- A month close started under 0.2.0 binds its approval to the figures the
  gate shows. If the books change while the gate is open, for example by a
  voucher in an earlier month that is still open, the approval is refused
  with `CLOSE_CONTENT_CHANGED`; reject the close and start it again. 0.1.0
  approved such a close.
- `import camt053 seal` checks the whole statement before sealing anything. A
  closed period or a conflicting entry now refuses the whole batch, where
  0.1.0 sealed the entries before the failing one.
- `capture-receipt record` refuses an unknown VAT rate class with
  `CAPTURE_VAT_CLASS_UNKNOWN` instead of `INVALID_ANSWER`.
- `--as-of` accepts only `YYYY-MM-DD`; `2026-1-5` is refused.
- Error JSON gains a `hint` key for the codes the guide explains. Existing
  keys are unchanged.

The receipt convention changed (see Changed below): a receipt that is also
paid from the bank account is recorded against 2700 Ostovelat, and the bank
statement row clears it. It applies to receipts recorded from now on. Nothing
happens to vouchers that are already sealed, and nothing needs to be
re-recorded. One case needs a look: a receipt that 0.1.0 recorded directly
against the bank account already credits that account, so importing its
statement row credits it a second time. Check the bank account against the
statement for such a month, as the guide's month-end check describes, and
correct a difference with a new voucher.

A year-end close that is waiting at a gate should be finished or rejected
before upgrading. If its statements contain a row that 0.1.0 printed as
`-0.00`, 0.2.0 prints it as `0.00` (see Fixed) and refuses the
`ApproveStatements` approval, because the statements changed; reject the run
and start a new one. A month close waiting at `ApproveClose` can still be
approved after the upgrade.

## [0.1.0] - 2026-11-19

### Added

- First public release. The `aggountant` CLI and its bundled agent skills
  cover a Finnish limited company's bookkeeping: company home setup, bank
  statement import (camt.053/camt.052), receipt capture, period close, VAT
  returns filed directly to Vero.fi, Tulorekisteri travel-expense reports and
  monthly employer's separate reports, and the year-end close.
- Prebuilt binaries for macOS arm64 and Linux arm64/x86_64 (static musl) on
  GitHub Releases, with a shell installer.

[0.1.0]: https://github.com/aggountant/aggountant/releases/tag/v0.1.0
