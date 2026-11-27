# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While the version is 0.x, a minor version bump may contain breaking changes.

<!-- oss-changelog:unreleased-start -->
## [Unreleased]

### Added

- **Onboarding readiness checklist and one source request.** The
  `aggountant-init` skill now shows a seven-item readiness checklist (identity,
  fiscal year, cutover, source coverage, account mapping and opening balances,
  reconciliations, first close), read from what the home already holds, so a
  half-finished onboarding continues where it stopped without re-asking
  recorded facts. From the missing items the agent writes one source request
  tailored to the company and notes it in the home's `TODO.md`; an Esimerkki Oy
  example ships as `aineistopyynto.md`. The opening-balance dating and account
  mapping guidance, which applies only to companies moving from another
  system, moved unchanged into the skill's new `avaavat-saldot.md`. Run
  `aggountant skill install` to get both files.

- **VAT return fields 309–311 from the books.** Sales tagged
  `sales-zero-rated` (exports and other zero-rated sales), `sales-eu-goods`
  and `sales-eu-services` now report their value without VAT in fields 309,
  310 and 311 of the return preview and of the direct Vero API filing
  (`ZeroVATRateTurnover`, `SalesOfGoodsToEUMemberStates`,
  `SalesOfServicesToEUMemberStates`); a credit note reduces the same field.
  A period without such sales files exactly what it filed before. A new
  voucher that puts such a sale together with a `purchase-` line is refused
  (`VAT_STRUCTURE`): record them as two vouchers. No store migration.

- **Tax Administration payment details as company master data
  (`tax-payments set|show`).** The payee account and the payment references
  (general tax reference, self-assessed taxes, prepayment, payment
  arrangement) are stored once in `masterdata/tax-payments.json`, plaintext
  and committed with the home; no store migration. References and the account
  are refused when their check digits do not hold, and the general reference
  is not stored together with tax-specific ones, as the Tax Administration
  asks companies to pay all taxes one way. A stored Danske Bank account
  (`FI56…`, closed at the end of May 2027) is accepted with the warning
  `TAX_PAYEE_IBAN_RETIRING`. `file-vat start` and `replace` take the account
  from it (`payee_iban` on stdin is now optional and must match), and the
  payment proposal, `run show` and the closed month's `file-vat preview` name
  the VAT reference: the general reference when stored, otherwise the
  self-assessed one, never the arrangement reference. **A first return now
  needs the VAT reference:** without it `file-vat start` refuses before a Run
  exists (`TAX_PAYMENT_REFERENCE_MISSING`); store it once with `tax-payments
  set`. A return whose account or reference was corrected after its Run
  started is not approved with the old one (`TAX_PAYMENT_DETAILS_CHANGED`;
  reject and start again). Runs started earlier read as before.

- **Travel allowances computed from a trip (`claim compute`).** A new
  read-only command takes trips as JSON on stdin and computes the scheduled
  tax-free allowances: travel days, domestic and foreign per diems with the
  halving by free meals and the deduction of company-paid meals, meal and
  night travel allowances, and mileage with its surcharges. The result shows
  the Tulorekisteri lines (311, 331), proposed postings per month and the Tax
  Administration decision used (2026: 970/2025; 2027: 929/2026), with the
  bundled table's hash. Situations the decision and guidance do not settle,
  missing answers and amounts above the maximum are refused with a code
  instead of guessed. Nothing is written; the rate table is read only by this
  command. Each trip's workplace comes from the claimant's regular-workplace
  register (below), so the input names the claimant (`employee_id`) and the
  trip's registered `places`, and the command reads the company home. New guide page "Laske matkan
  verovapaat korvaukset" (Compute the tax-free allowances of a trip).

- **Expense claims: the regular-workplace register.** The new `claimant`
  commands record, for each person who files expense claims, their places,
  their regular workplace (varsinainen työpaikka), secondary workplace and
  tracked customer sites, their relationship to the company and their
  shareholding as dated facts the user confirms, each workplace entry with a
  basis code tied to the Tax Administration's guidance
  (VH/630/00.01.00/2026). A fact is never edited: `claimant correct`
  supersedes it, and `claimant show --as-known-at` answers what was known at
  an earlier moment. `claim compute` classifies each trip from the register
  for its start date (`claimant snapshot` shows the same classification): a
  trip with no relationship or regular-workplace entry on that date is
  refused and computes nothing (`CLAIMANT_RELATIONSHIP_UNKNOWN`,
  `REGULAR_WORKPLACE_UNKNOWN`), a trip to a regular workplace is refused, a
  secondary workplace earns travel costs only, and the home is never a
  destination (`TRIP_TO_HOME`). A contractor cannot be a claimant. The guide has a new
  task page, "Kirjaa varsinainen työpaikka" (Record your regular workplace),
  and the glossary gained the legal terms. Every command opening a home
  migrates `masterdata/pii.sqlite` from schema v1 to v2. The migration is
  additive (two empty tables) and one-way: an older binary refuses the
  migrated store, so back the store up before installing. A company that does
  not use expense claims sees no question and no change in any other
  command's output.

- **Expense claims: collecting a claim draft.** The new commands `claim trip
  add|revise|remove` and `claim line add|revise|remove` build an expense claim
  of trips and receipt lines over days or weeks, one trip or a whole month,
  in a program-owned draft at `intake/expense-claims/<claim_id>/draft.json`;
  `claim show [--format markdown]` renders it on request, `claim list` lists
  the open drafts and `claim discard --reason` removes one, keeping only its
  id, last revision hash and the reason. Every write validates the typed
  fields, computes the trips with `claim compute`'s calculation and the
  claimant register, and answers with machine-readable open questions (the
  travel claim's statutory content under ennakkoperintäasetus 17 §, where the
  trip started, free and company-paid meals per travel day, vehicle
  ownership, missing receipts, the register's workplace questions),
  refusals that block approval (overlapping trips, a receipt already booked
  or repeated, a vehicle not the claimant's, a trip cost on a trip the
  register does not make a work trip) and plausibility checks (kilometres
  against the distance, a receipt outside every trip, a possible duplicate).
  A trip recorded after the end of the month following the one it ended in
  is marked as recorded afterwards. Receipts are attached to the artifact
  store first and the draft keeps their hash. The draft directory gets its
  own `.gitignore`, so no draft reaches git even in a home whose root
  `.gitignore` predates it; a draft edited by hand that no longer validates
  is refused (`CLAIM_DRAFT_INVALID`). A claim cannot be approved yet. New
  guide page "Kokoa kululasku" (Collect an expense claim) and a new
  `expense-claim` skill. No store migration; a company that does not use
  expense claims gets no directory, no question and no change in any other
  command's output. `claim compute` now checks every drive (its date within
  the trip, positive kilometres, car-only surcharges) also on a trip the
  register refuses or classifies as a trip to the regular workplace, so a
  malformed drive is `TRAVEL_INPUT_INVALID` there too instead of passing
  unchecked.

- **Travel advances booked by hand.** The user guide has a new task page,
  "Kirjaa matkaennakko ja sen tilitys" (Record a travel advance and its
  settlement), and the `capture-receipt` skill bundles `matkaennakot.md`: the
  advance is a receivable on `1730`, the expense claim is booked gross on
  `2740`, the offset is a voucher of its own dated on the day the difference
  is paid, and the advance is never reported to Tulorekisteri while the
  claim's mileage and per diems are reported in full. The `tulorekisteri` and
  `close-year` skills gained one point each on advances.

- **Bank import: empty statements are named.** A camt.053 or camt.052
  statement with balances but no booked entries still imports (a quiet month
  needs no workaround: `{"assignments":[]}` previews and seals it with no
  vouchers), but `import camt parse`, the `--dry-run` preview and the seal now
  carry `empty_report` with the account, the statement periods, the number of
  entries the bank has not booked yet and a message, and the human output
  prints it. A wrong account, a wrong month or a truncated export no longer
  looks like a plain success with zero transactions.

- **Bank import: transaction codes, charges, original amounts and
  reversals.** `import camt parse` now shows, when the bank's file gives
  them, each transaction's bank transaction code (`transaction_code`,
  `BkTxCd`), its charges (`charges`, `Chrgs`), the instructed amount of a
  foreign-currency payment (`instructed_amount`, `AmtDtls/InstdAmt`) and
  reversal or return information with the reason (`reversal`, `RvslInd` /
  `RtrInf`). A statement with a reversal (`RvslInd` true) no longer stops
  with `INVALID_CAMT`: the reversal is booked in its own direction under its
  own reference, normally to the original payment's accounts. The new fields
  are outside an entry's identity, so already imported statements re-import
  as duplicate observations, and files without these elements parse as
  before. No store migration.

### Changed

- **Skills: `migrate-books` is now `migrate`, phased by domain.** The skill
  moves a company from another program one domain at a time, each on its own
  cutover date: the books (what `migrate-books` did), VAT returns, Incomes
  Register reports and travel and expense claims, with the dependencies
  between them and a company fact per domain recording what moved from which
  date. A company that has no expense claims is not asked about them; the
  claim part waits for the claim commands where they are missing. `skill
  install` turns an earlier install's `migrate-books/SKILL.md` into a pointer
  to `migrate` (reported under `renamed`; nothing is deleted), `skill list
  --json` lists the rename, and `skill install migrate-books` or `skill print
  migrate-books` fails with `SKILL_RENAMED` naming the new name. `migrate`
  installs `vero-fi`, `tulorekisteri` and `e-services` with it. Guide pages
  "Move your books from another program" and "Install" follow.

- **`file-wage`: one per diem line with several types.** A 331 daily
  allowance line now takes `allowance_codes`, every per diem type its amount
  contains (for example `[2, 3]` for partial and full per diems), and the
  report carries one 331 transaction with an `AllowanceCode` for each, as the
  Tax Administration's Incomes Register guidance describes. The lines
  `claim compute` prints are accepted as they are. The single
  `allowance_code` is still read, and a one-type line is written and sent
  exactly as before, so open and finished reports keep working. No store
  migration.

- **`file-wage` shows the statutory deadline for travel reports.** A
  tax-exempt travel report (income types 311 and 331) is due on the 5th of
  the month after the payment month (laki tulotietojärjestelmästä 12 § 11
  mom.), or on the next business day when the 5th is a Saturday, Sunday or
  holiday (laki säädettyjen määräaikain laskemisesta 5 §). `file-wage
  preview` showed five days after the payment date. The guide and the
  `tulorekisteri` skill say that one report may combine a month's payments
  to one earner with the latest payment date.

- **Breaking: the bank import verb is `import camt` and its parse error is
  `INVALID_CAMT`.** The parser reads camt.052 as well as camt.053, so the
  names no longer claim camt.053. `import camt053` (and `import camt052`)
  remain working aliases of `import camt`, so existing agents and scripts
  keep running. The error code `INVALID_CAMT053` is now `INVALID_CAMT`;
  a caller that matches on the old code must match the new one. The exit
  code and the bank event identity are unchanged.

### Fixed

- **Store refusals no longer suggest resetting the home.** When the voucher
  database (`fiscal`) or the personal data database (`pii`) was written by a
  newer aggountant, or predates the schema-migration framework, the refusal
  (`STORE_ERROR`, unchanged) used to suggest resetting the home. It now says
  that nothing was changed, that the home may hold live books and must not be
  modified, deleted or reset, and to open it with the aggountant version that
  wrote it (or a newer one). The guide's troubleshooting page has a new
  section for this refusal. No store migration.

- **A Run id of the wrong workflow is refused before anything changes.**
  `run answer` and `run resume` handle month closes, but given a
  capture-receipt, hunt-receipts, payroll or year-end Run id by mistake they
  ran the month-close recovery on it, which could move that Run's open month to
  closing under a Run that never seals it and leave the Run unreadable. Every
  workflow's answer, resume and retry commands now refuse a Run of another
  workflow with `INVALID_ANSWER`, naming the commands that continue it. A Run
  of a workflow this version does not know, and `run migrate` for a Run whose
  workflow has no version to move to, are refused the same way instead of being
  treated as a month close.

- **VAT: a `file-vat` run that stopped before filing now ends.** A
  `file-vat start` or `replace` refused before the approval (figures that do
  not match the books, unreconciled VAT accounts, a failed or refusing period
  read) used to leave its run `needs_attention` for good, and a run that
  stopped mid-reconcile stayed `running`. Such a run now has the new final
  status `not_filed` in `run show`: the return was not filed by it and nothing
  was sent. Runs earlier versions left rejected read `not_filed` as they are;
  `run resume` settles one left with `reconcile` executing (code
  `VAT_INTERRUPTED_BEFORE_FILING`), and refuses with `VAT_FILING_UNRESOLVED`
  if its journal shows a filing step began. Every error during the reconcile
  is now journalled. `run resume` no longer applies month-close recovery to a
  `file-vat` run. A rejected effect in `run show` now carries its
  `error_code`. A caller that matches on `needs_attention` for these runs
  must accept `not_filed`.
<!-- oss-changelog:unreleased-end -->

## [0.3.0] - 2026-11-24

### Added

- **Skills:** `migrate-books` carries the agent's procedure for moving a
  company's books from another program (inventory, read-only export, a
  checksummed archive, the hand-off to setup), with reference files for Briox
  and Holvi. The guide page "Move your books from another program" keeps the
  user's part.

- **Bank import: overlapping statements.** A bank transaction is booked once
  however many exports contain it. `import camt053 seal` recognises a
  transaction already in the books by its bank reference and account: when
  the bank's facts and the assignment are the same as when it was sealed,
  nothing is sealed and the file is recorded as one more source of the
  transaction (`outcome: duplicate_source_observation`). Two yearly
  statements that share the transactions around the turn of the year can now
  be imported one after the other. Every entry of the preview and the seal
  carries its `outcome`, and the output counts new, duplicate, bound and
  backfilled transactions; the text line reads `sealed N new vouchers from
  "…", M transactions already in the books`.
- **Bank import: transactions booked by hand.** Before it seals a new voucher
  for a statement entry, the import looks for a voucher sealed outside the
  import (for example with `voucher seal`) that may already book it: one
  whose natural key or description names the bank reference, or one that
  posts the same amount on the same side of the same bank account within 7
  days. Such a voucher stops the batch (`BANK_ENTRY_POSSIBLY_BOOKED`) instead
  of the transaction being booked twice. The new stdin field
  `existing_vouchers` binds the entry to that voucher without changing it,
  and an assignment's `not_booked_by` rules a voucher out.
- **Bank import:** `import camt053 observations` lists, per retained
  statement file, which transactions it contained and the voucher that books
  each.
- **Incomes Register:** `file-wage approve`, `file-wage correction submit` and
  `file-wage employer-report submit` take `--record-hash-file PATH` (`-` is
  stdin, except for a replacement correction, whose report already comes on
  stdin) as an alternative to `--record-hash`, so the sensitive travel-report
  hash need not appear in the command line, shell history or a transcript.
  Help, skill and guide capture the hash from the preview the user reviewed
  into a private file, instead of computing a fresh one at approval time.
- **User guide:** a page on moving the books from another program: what to
  collect, how long it must be kept, how to keep the collected archive
  trustworthy, and how it enters aggountant.

### Changed

- **Only `init` creates a company home.** Every other command that opens a
  home refuses a path without one with `HOME_NOT_INITIALIZED`, writes
  nothing, and names the path and how it was chosen (`--home`,
  `$AGGOUNTANT_HOME`, the configuration file, an `.aggountant` marker or the
  `./accounting` fallback). In 0.2.0 only the voucher reads refused; the
  other commands created an empty home under a mistyped path and answered
  from empty books.
- **VAT treatments are checked against their rate class.** A purchase
  treatment (`domestic-input`, `reverse-charge` and their `-partial`
  variants) with one of the zero-rated sale classes `eu-goods`, `eu-services`
  or `zero-rated` is refused with `VAT_TREATMENT_CLASS_MISMATCH`, whose
  message names the treatment that fits. In 0.2.0 a domestic purchase or an
  EU reverse charge with such a class sealed at 0 % without VAT legs; a
  non-EU service reverse charge was already refused, now with this code.
  Sealed vouchers are not re-checked.
- **VAT: exempt intra-Community acquisition of goods.** The one purchase
  treatment that keeps the `zero-rated` class is
  `{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}`,
  for goods bought from another EU state whose acquisition is exempt in
  Finland: no tax and no deduction, and the value is reported in the return's
  EU goods purchases. `--help`, the guide and the refusal messages now
  describe this booking instead of sending the user to a bookkeeper.
- **Members that do not belong to a VAT treatment are refused**
  (`INVALID_INPUT`): a `deductibility` on `domestic-input` instead of
  `domestic-input-partial` in an `import camt053 seal` assignment, and any
  member on `{"type":"no-vat"}` there or in a `capture-receipt` VAT proposal.
  In 0.2.0 they were dropped silently, which could seal a full deduction
  instead of a partial one, or a receipt without its input VAT.
- **Bank import: one transaction per statement.** `import camt053 seal`
  writes the whole statement in one ledger transaction, so a write that fails
  on a later entry leaves nothing sealed from the statement. An already booked
  transaction still needs its assignment or binding, and it must be the one
  its voucher was sealed or bound with.
- `import camt053 seal --dry-run` no longer lists the warning
  `NATURAL_KEY_ALREADY_SEALED` for a transaction already in the books; read
  the entry's `outcome` (or `would_replay`) instead. An entry bound to an
  existing voucher has no `draft`. `import camt053 parse` lists the
  statement's entries that are not booked (pending ones, for example) under
  `unbooked`.
- `file-vat preview` on a closed period names both filing routes, direct
  filing through the Vero.fi API with `file-vat start` and manual filing in
  OmaVero recorded with `file-vat complete-manual`, and says that the offline
  preview did not check which one is available. In 0.2.0 it described only
  manual filing.
- **User guide fact pass.** The pages were checked against the code and
  `--help` and rewritten to say who acts in each step. Corrected facts
  include: installation starts with the prebuilt installer; macOS and Linux
  are the supported platforms and Windows has not been tested; `close-year`
  is listed as in use; the error-code reference's "what to do" cells, which
  `aggountant explain` and the error JSON's `hint` print, are instructions.
- The README is in Finnish, with an English translation in `README.en.md`.

### Fixed

- `run show` and `run start` for an open month-close approval returned a
  `next_command` ending in `--answer approve|reject`, which a shell runs as a
  pipe that approves the close. It now carries the quoted placeholder
  `'<approve|reject>'`, which `run answer` refuses (`INVALID_ANSWER`), and the
  `--help` of the other approval commands no longer prints an `a|b` choice
  inside a command.
- A refusal of a partial VAT treatment suggested a replacement without its
  required `deductibility`, so copying the suggestion did not parse.
- `url` 2.5.9 for RUSTSEC-2026-0592.

### Upgrading from 0.2.0

Upgrade when no month close, year-end close or filing is in progress
(`aggountant run list`), and take these steps in order. The guide's
[upgrade page](https://aggountant.dev/en/tasks/upgrade/) describes the whole
procedure.

1. **Back up the home first.** The first command that opens the books
   migrates `fiscal.sqlite` from schema v8 to v9. The migration is additive:
   it adds the bank-transaction identity tables, and sealed vouchers, their
   hashes and `voucher verify` are unchanged. It is one-way: 0.2.0 refuses to
   open a v9 store, so the backup is the only way back. Copy the whole home,
   including `fiscal.sqlite` and the `fiscal.sqlite-*` files next to it, as
   the guide's
   [back-up page](https://aggountant.dev/en/tasks/back-up-and-restore/)
   describes.
2. **Install 0.3.0**, then run `aggountant skill install` and restart the
   agent. `capture-receipt` now handles overlapping statements and bank
   transactions booked by hand, and `tulorekisteri` approves with
   `--record-hash-file`.
3. `aggountant home upgrade` has nothing new to add in 0.3.0; a home brought
   up to date under 0.2.0 needs no master-data change.

The first statement import after the upgrade reads the identity of the
vouchers that 0.2.0's import sealed back from the statements they retained,
in the same transaction (`backfilled_count`); those vouchers are not changed.

Commands that refuse what 0.2.0 accepted, for scripts and agent habits
carried over from 0.2.0:

- A command other than `init` run against a path with no home fails with
  `HOME_NOT_INITIALIZED` instead of creating an empty home there. Point it at
  the company home (`aggountant config show` shows how the home is chosen);
  run `init` only for a new company.
- A purchase treatment with the class `eu-goods`, `eu-services` or
  `zero-rated` fails with `VAT_TREATMENT_CLASS_MISMATCH`. A taxable EU
  purchase is `reverse-charge` (or `reverse-charge-partial` with its
  `deductibility`) with `scope` `eu-goods` or `eu-services` and the domestic
  class whose Finnish rate applies. An intra-Community acquisition of goods
  exempt in Finland stays
  `{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}`,
  so that its value is still reported in the EU goods purchases. A purchase
  with no Finnish VAT otherwise is `no-vat`.
- A VAT treatment member that does not belong to its type, in an
  `import camt053 seal` assignment or a `capture-receipt` VAT proposal, fails
  with `INVALID_INPUT`. Use the `-partial` type for a partial deduction, and
  give `no-vat` no other member.
- `import camt053 seal` stops the whole statement, before anything is
  written, when a transaction already in the books comes with changed bank
  facts or a different booking, or when a voucher sealed outside the import
  may already book a new entry. A transaction already in the books with the
  same facts and booking is not a conflict. `details.conflicts` names each
  entry and its voucher:
  - `SOURCE_FACTS_CHANGED`: the bank's facts about a booked transaction
    differ from the earlier export, or the file lists it as not booked. Find
    out from the bank what changed before booking anything for it.
  - `POSTING_PROJECTION_CHANGED`: a booked transaction was given a different
    assignment. Give it the assignment its voucher was sealed with
    (`voucher show`), or the same `existing_vouchers` binding if it was bound
    to a voucher; if that booking was wrong, correct it with a correction
    voucher.
  - `SOURCE_IDENTITY_UNKNOWN`: a voucher holds the transaction's reference,
    but the transaction cannot be found in the statement attached to it. The
    transaction is not booked again; check that voucher and its attachment.
  - `BANK_ENTRY_POSSIBLY_BOOKED`: a voucher sealed outside the import may
    already book a new entry. If it does, move the entry from `assignments`
    to `existing_vouchers` with that voucher's number (its bank leg must
    match, or the batch stops with `BANK_VOUCHER_MISMATCH`); if it does not,
    add the voucher's number to the assignment's `not_booked_by`. A voucher
    can be bound to one bank transaction only; if it books several, reverse
    the other parts with a correction voucher that refers to it, then list it
    in `not_booked_by` for those entries.
- A month-close `next_command` carries `'<approve|reject>'`, which must be
  replaced with the answer; run as is, it fails with `INVALID_ANSWER`.

Known limitation: an own-account transfer between two bank accounts whose
statements are both imported can be booked twice, once from each statement,
when each import books it straight against the other bank account; nothing
stops the second
([camt-transfer-between-imported-accounts](https://github.com/aggountant/aggountant/tree/main/issues/camt-transfer-between-imported-accounts)).
Book such transfers through a clearing account, so that each statement books
only its own leg.

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
