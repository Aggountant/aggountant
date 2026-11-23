# Result: guide open fact questions

Each question in `attachments/open-questions.md` was answered: the page was
corrected in both `fi/` and `en/`, or the question was closed with no change
needed. Facts were checked against the code, `--help` and the tests. The guide
gate (`scripts/check-user-guide.sh`) passes.

**Changed** means the page was edited. **Closed** means no change was needed,
with the reason. **Open** means the answer could not be settled from the
repository and is passed on to the maintainer (see the end of this file).

## concepts/division-of-work

- `SOUL.md` named: changed. The file name was removed per the style rule on
  internal file names; the sentence keeps "yhteinen roolikuvaus".
- "kuten" in the list of permanent acts: closed. The list is not complete:
  `sealing-and-approval.md` names five acts that need approval (including
  Tulorekisteri filing and the move to Vero.fi production), so "kuten" is
  correct.

## tasks/close-month

1. Open obligations in the approval content: changed. `ClosePeriodContent`
   has the field, but both `run start` and the approval refuse while an
   obligation is open (`refuse_open_obligations`), so on screen the list is
   always empty. It was dropped from the list of what you approve.
2. `PERIOD_CLOSING`: changed. Rejecting the close returns the month to `open`
   (`--answer reject`), so the page now says: if the entry belongs to this
   month, reject the close, record the entry and start the close again;
   otherwise record it in the next month. `PERIOD_SEALED` is a separate item.
   The error-code row was changed to match (it said "approve the close",
   which seals the month and still sends the entry to the next one).

## start/is-it-for-you

1. "muutaman" vs one company: closed. Two companies keep real books with
   aggountant, so "a few" holds; `OSS-RELEASE.md` ("one real company") is out
   of date, and is owned by `/shipshape-init` (open item).
2. Version in running text: closed. The page already says "varhainen
   0.x-versio".
3. "Kokeile ilman riskiä" last: closed. A call to action at the end suits a
   reader who has just decided.
- Also changed, following the decision to remove the meeting offer: "voimme
  myös tavata" became a plain link to the early-users page.

## start/install

- New terminal: changed. The released installer (cargo-dist 0.33) installs to
  `$CARGO_HOME/bin` (default `~/.cargo/bin`) and edits the shell profile, so
  step 2 now says to open a new terminal if the command is not found.
- Windows: changed on `main` by the maintainer's own commit (macOS and
  Linux; "Windows has not been tested"), which landed while this work was in
  progress. On rebase the maintainer's wording was kept as it is.
- SHA-256 checksums: closed. The owner does not act on them.

## reference/status

- Windows: changed on `main` by the maintainer's commit; kept as it is.
- Has 0.2.0 been released: closed. `v0.2.0` was released on 2026-11-21 and is
  the latest release, so the installer gives 0.2.
- Bold leads on limitations: closed. The style guide allows one short bold
  term per item in a long reference list.
- Also changed: the upgrade limitation moved here (see tasks/upgrade), and
  the quarterly direct-filing wording (see concepts/periods).

## README (front page)

- Title "aggountantin käyttäjän opas": closed. The wave 2 lessons settle it.
- "muutaman yrityksen" in three places: closed. The front page and
  is-it-for-you are introductions that link to status, where the fact lives.
- `Landing.astro` "Versio 0.1": closed. It already says "Varhainen
  0.x-versio".

## start/early-users

1. Email address: closed, kept per the decision.
2. "usually within a few days": changed, removed per the decision.
3. Meeting offer: changed, removed per the decision. "Yksityiskohdat voidaan
   käydä läpi tapaamisessa" now says the details can be sent by email. With
   one item left, "Mitä tarjoamme" is a paragraph rather than a list.

## tasks/monthly-routine

- Quarterly filer: changed. One sentence in the intro, with a link to
  periods: with quarterly closing, steps 7 and 8 come after the last month of
  the quarter.
- Approval repeated per step: closed. Steps 1 and 3 say it.
- "Maksa vero itse" and the check: closed. One is the action, the other the
  check.

## concepts/sealing-and-approval

- Bank-import preparation hash: closed. The hash is optional in
  `bank_import.rs`, but the `capture-receipt` skill always runs `seal
  --dry-run` and seals with `--approved-preparation-hash`, so "toimii samoin"
  holds for users. `status.md` keeps the full limitation.
- Reassurance line: closed. It repeated a fact; the style guide removes
  repeats.

## concepts/periods

- "Sulun aikana tehdyt kirjaukset säilyvät": changed, removed. A period in
  `closing` accepts no entries and the close records nothing, so "mitään ei
  poisteta" is enough.
- Run states on this page: changed in `docs/user/AGENTS.md`. No page explains
  run states in general; a run's state (`awaiting_gate`, `rejected`) is
  explained in `troubleshooting.md`, where the user meets it, so the fixed
  home now says "period states".
- Quarterly direct filing tested or supported: changed. It is supported: the
  Vero payload and the CLI tests cover `2026-Q1`. Only real use has been
  monthly. periods, status and file-vat-direct now say that it works for
  quarterly periods but has been tried in real use with monthly ones.

## concepts/filings

- How a Tulorekisteri uncertain outcome ends: changed. `reconcile_wage`
  records the outcome only when the status query finds the report or
  Tulorekisteri reports a rejection; otherwise it keeps the run parked. There
  is no "not found twice" rule, and `complete-manual` records e-service
  evidence. A paragraph now says this.
- Bank holidays: changed. All eleven from `crates/domain/src/bank_days.rs`
  are listed.

## concepts/vat-treatment

- Home of the exempt EU rules: closed. They explain why the treatment is a
  named choice, which is this page's subject, and the error-code row links
  here.
- Who confirms exemption: changed. The exempt intra-Community acquisition
  paragraph now names the actor: the agent uses it only when the exemption is
  clear from the invoice or the company's practices, and otherwise asks you.
  This matches the `capture-receipt` skill and the page's own earlier rule.
  No "ask your bookkeeper" instruction was added.

## concepts/company-home

- "ohjelma" vs "aggountant": closed. The wave 4 ruling settles it
  ("aggountant" first in a section, "ohjelma" after); the heading keeps its
  anchors.
- "Yksi koti, yksi yritys" section: closed, kept.

## concepts/for-bookkeepers

1. Year-end entries in the next year's first open period: changed. One
   sentence plus a link to status#tilinpäätös under "Rajat".
2. A bookkeeper who runs the agent: closed. "Pyydä omistajaa tai agenttia"
   already covers both readers.

## tasks/record-receipts

- Prompt vs worked table: changed. The prompt is now the same software
  licence invoice paid from the company's account that the table shows.
- `PERIOD_CLOSING`: changed, as on close-month.

## tasks/import-bank-statement

1. "nimetty avoimeksi" without being recorded: changed. A seal refuses any
   confirmed transaction without an assignment (`MISSING_ASSIGNMENT`), so
   after a successful import every transaction is recorded. "Valmis, kun" now
   says that, and that a transaction whose receipt is still missing is named
   open in `TODO.md`.
2. Naming codes: changed for `MISSING_ASSIGNMENT` and `ACCOUNT_NOT_FOUND`
   (one item each, plus a line pointing to the error-code page).
   `SOURCE_IDENTITY_UNKNOWN` was left to the parallel camt import work, which
   owns the duplicate and identity wording.

## tasks/correct-voucher

- Reopening a closed month: closed. `Ledger::reopen_period` exists, but no CLI
  command or process reaches it (it is used only in ledger tests), so the
  user has no way to reopen a month. "Suljettua kuukautta ei voi avata
  uudelleen" is correct, and the correction voucher is the only route.
- Reason for telling the agent before sealing: changed. One clause: the agent
  marks it on the voucher, and the marking cannot be added afterwards.

## tasks/upgrade

- Installer over a `cargo install` binary: changed. Both put the binary in
  `~/.cargo/bin`, and the installer moves the new binary over the old one, so
  step 2 says the installer also replaces a version first installed from
  source.
- Limitation home: changed. The full limitation is now in status.md
  (Yrityskoti ja käyttöönotto); the task page has one sentence and a link.

## tasks/report-travel-allowances

- "pääomaosakkuus" vs "Osaomistaja": changed. The question's premise does
  not hold. IncomeEarnerType 2 is "Osakasasemassa maksajaan" (joint owner
  with the payer); "Osaomistaja" (the partial owner of unemployment security
  law) is the next code. This was checked against public Incomes Register
  guidance and payroll vendors' code lists, so the code's mapping and the
  skill agree. The step and the glossary now give the e-service name
  "Osakasasemassa maksajaan" and say that it is not the osaomistajuus of
  unemployment security law.
- "Hakemuksen voi hyväksyä heti": open. It is an observation of the
  certificate service, and the repository cannot settle who approves.
- Earning period asked too: changed. The skill says the program does not
  infer it; the step now says the agent asks for it if the travel claim does
  not show it.

## tasks/file-employer-report

- `--record-hash` vs `--record-hash-file`: closed. `submit --help` says
  `--record-hash` is fine because this hash holds no personal data.
- Naming error codes: changed. `EMPLOYER_REGISTER_MISMATCH`,
  `YTJ_UNAVAILABLE`, `EMPLOYER_REPORT_WAGES_UNSUPPORTED` and
  `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED` are named where the page already
  describes those situations.
- Approval step for setup: closed. `configure` writes the profile, which can
  be rewritten. It is not one of the permanent acts, and the skill sets no
  approval gate for it, so "Tarkista" is accurate.

## tasks/year-end

1. First approval already seals: changed. Per `close-year answer --help` and
   `seal-correction --help`: approving `ApproveAdjustments` seals the
   depreciation and accrual vouchers, each correction is sealed separately
   before that with its own approval, and a rejection ends the run but leaves
   what was sealed.
2. VAT filing enforced: changed. `close-year start` checks only that every
   subperiod is `Sealed`, not `vat_filed`, so step 1 now says checking the
   VAT returns is up to you.

## tasks/file-vat-direct

- Replacement return untested in production: changed. One sentence under
  Rajoitukset (the fact's home is status.md).
- Quarterly periods: changed (see concepts/periods).

## tasks/file-vat-omavero

- Where the reconciliation mechanism lives: closed, kept. Both filing pages
  need it.
- "ei maksa" twice: closed, kept. Each is in a place a manual-route user
  reads.
- `settlement-account-roles.json` by hand: changed. There is no command for
  it (`chart` only lists), so the page now says the agent marks the
  settlement accounts in the file by hand.

## tasks/set-up-vero-api

- Application vs agreement in force: changed. The attestation is
  `api_agreement_active`, and the `vero-fi` skill says to record it only once
  the act is done, not when the application is sent. Prerequisite 2 now says
  the agreement must be in force.
- Who activates production: changed. The user does, per the skill ("aktivoi
  tuotantoluvan").
- Certificate renewal step 3: changed. Readiness binds only the portal
  verification evidence and the production activation to the fingerprint
  (`vero_filing.rs`: the two `*_CERTIFICATE_MISMATCH` blockers). Step 3 was
  replaced with a sentence saying the registration, agreement and
  production-permission confirmations stay valid.
- Rough waiting times: open. The repository has no figure.

## tasks/record-other-entries

- `natural_key`: closed, kept. `NATURAL_KEY_ALREADY_SEALED` refers to it.
- Payroll checks: closed. Payroll is calculated outside aggountant; the
  voucher checks on the page apply, and adding a payroll comparison would be
  new content with no behaviour behind it.

## tasks/monthly-report

1. "kirjanpidossa ei ole vikaa": closed. "Sillä välin kirjaat, suljet ja
   ilmoitat … tavalliseen tapaan" already tells the owner that nothing is
   wrong.
2. A first fiscal year with no earlier year: changed. `report statements
   --help` says a year with no earlier activity does not wait, so the item now
   covers a new company as well as one opened with opening balances.

## tasks/back-up-and-restore

1. Why no command may run: changed. One clause: a copy of the voucher
   database taken in the middle of a write can be broken.
2. `config show`: changed. Step 4 now says the agent first checks that the
   restored home is the one in use.
3. "lähteistään": changed to the receipts and bank statements, linked to
   their task pages.
4. Retention and rhythm: changed. The periods (10 and 6 years) are given with
   a link to the move page, where the Accounting Act passage lives, and the
   page says to take a copy at least after every month close and before
   every upgrade.

## start/set-up-company

1. "Valmis, kun" for a new company: changed. The figures check is now
   conditional on a move from another program.
2. Month boundary: changed. Added to the migration prerequisite, with the
   reason the skill gives.
3. OmaVero transaction statements: changed. Step 5 says the agent asks for
   the year's statements when the tax account has a balance or a payment
   arrangement.

## start/move-from-another-program

1. No skill carries the procedure: open. Whether a skill should carry it is a
   product decision; the page was left as it is.
2. Signing in before the inventory: changed. Steps 1 and 2 were swapped, so
   you sign in before the agent takes the inventory figures from the old
   system.
3. "SIE4-vientiä emme löytäneet": closed, kept. It is the observed claim.

## troubleshooting

- Reason for not changing the payment identifier: changed. The reason
  clause was added.
- Sentences about earlier versions: closed, kept. Whether any home touched
  by those versions is still in use cannot be shown, and the sentences cost
  little.
- Test-seam section: closed, kept. Anyone with `AGGOUNTANT_BROKER_SCRIPT`
  set can meet it, and the result (a period permanently marked filed) is
  serious enough to keep on the owner's page.

## reference/glossary

- Linking entries to fact homes: closed. That is a restructuring of the
  whole page, not a factual question.
- Third "Katso myös" link: closed.
- "Velvoite": changed. One clause saying no obligations arise in ordinary use
  yet, with a link to periods.

## reference/commands

- Table order: closed. Calendar order follows the front page.
- status.md in "Katso myös": changed, added as the third link.
- `close-year` fully in use: closed. status.md lists the year-end entries and
  sealing as ready for real bookkeeping.

## reference/vero-fi-blockers

- `VERO_PRODUCTION_VERIFICATION_UNATTESTED`: changed (doc side). The cell now
  says the program records the tests when the agent runs them with your
  permission and the run succeeds (`verify-portal` sets
  `production_verification_completed`). The code's owner (`human`) was not
  changed; that is a code question for another issue, if wanted.
- Blockers in stages: changed. `vero_readiness` returns early when the
  credential file cannot be read or is invalid, so certificate blockers show
  only afterwards. One paragraph says this.
- Third column: closed. The meaning implies the action, and the certificate
  pair links to "Kun varmenne vaihtuu".

## start/practice-month

- `<details>` notes: closed. The tutorial template has none.
- Link in step 7: changed. "suora ilmoittaminen" now links to the setup page.
  It is not a branch in the exercise.

## reference/error-codes

- `VAT_TREATMENT_CLASS_MISMATCH` hint: closed, kept. The hint already says
  the message names the suitable treatment.
- Actor in every row: closed. A bare imperative is fine where either may act.
- "tavallisimmat": closed. The table does not hold every code, and the gate
  does not require it, so the sentence is accurate.
- Also changed: the `PERIOD_CLOSING` row (see close-month).

## Open items for the maintainer

- `start/move-from-another-program`: should a bundled skill carry the
  migration procedure, or is the page meant as instructions the user gives
  the agent?
- `tasks/report-travel-allowances`: "Hakemuksen voi hyväksyä heti" (who
  approves the certificate-service application) needs someone to look in the
  service.
- `tasks/set-up-vero-api`: is there a rough figure for the waiting times?
- `OSS-RELEASE.md` says the CLI runs "the books of one real company"; two do.
  `/shipshape-init` owns that file.
