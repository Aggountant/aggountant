# Result: Finnish user guide rewrite (first draft)

`docs/user/fi/` now follows the tree in `issues/user-docs-design/design.md`:
the front page by moment, four `start/` pages, fourteen `tasks/`, seven
`concepts/`, `troubleshooting.md` and five `reference/` pages (31 pages plus
`README.md`). The nine numbered chapters are removed. `docs/user/practice/`
holds the synthetic practice material and a script that runs the practice
month. `docs/user/AGENTS.md` carries the new rules, and the five page-type
templates moved from the design into `docs/user/AGENTS-TEMPLATES.md`.
`docs/user/en/` and `README.md` are unchanged.

## How it was checked

All against `aggountant 0.1.0` built from this tree (release build), in
throwaway homes under a scratch directory:

- `docs/user/practice/run-practice-month.sh <binary>` runs the whole tutorial
  as the agent would and asserts every figure the page quotes: two parsed
  rows (pending row skipped), `sealed 2 vouchers`, re-import keeps 2 vouchers,
  receipt `#3` and `replayed=true`, month result −123,50 €, Pankkitili
  2 376,00 €, balance totals 2 407,50 €, close `awaiting_gate` → `completed`,
  `PERIOD_SEALED` after the close, fields 307 = 31,50 € and 308 = −31,50 €,
  payment 0,00 €. Passes.
- Every `aggountant …` command and flag mentioned in `docs/user/fi/` (91
  mentions) exists in the binary's `--help` (a scratch checker that walks the
  subcommand tree; it was tested against a planted bad flag).
- Every relative link and `#anchor` in `docs/user/` resolves, using GitHub's
  heading-slug rule (scratch checker, tested against planted breakage). One H1
  per page.
- Outputs quoted on the pages were taken from the binary: `parsed … — 2
  transactions`, `period … vat_filed true` after `complete-manual`,
  `chain ok — N vouchers, head #N`, the report's "Kattavuus: varmistamaton",
  `run … "rejected"` leaving the period `closing` and a new close refused with
  `PERIOD_TRANSITION_CONFLICT`, `MISSING_ASSIGNMENT`, `ACCOUNT_NOT_FOUND`,
  `MASTERDATA_NOT_INITIALIZED` with the stray empty directory left behind,
  `PERIOD_NOT_SEALED` for `complete-manual` on a closing period, the Vero.fi
  blocker codes and owners of a fresh home, and the bundled VAT rate table.
- `cargo install --git https://github.com/aggountant/aggountant aggountant --locked`
  installs 0.1.0 into a disposable root (the path `start/install.md` gives).
- Privacy scan of every changed file for home paths, host names, e-mail
  addresses, business IDs other than 1234567-1 and IBANs other than the
  fictional one: none.

The practice material has no binary files (camt XML, two text receipts, a
shell script), so there is no SHA-256 to record.

## Migration check

Every section of the old Finnish chapters, its new home, and whether the
facts are carried. "Status" is `reference/status.md`. Besides this reading, a
mechanical pass confirmed that every inline code token (commands, flags,
codes, field names, figures with %, €, days or years) and every command in the
old chapters' code blocks appears in the new tree; the few that were carried
only in words (`ApproveFiling`, `ApprovePayment`, `--answer reject`,
`--remote-status-evidence-ref`, `payee_iban`, `record_hash`, `employee_id`,
`tax_card.withholding_pct`, `knowledge company retire/list/search/history`,
`obligation resolve`, `mkdir .aggountant`) were added back where a user sees
them.

| Old (fi) | Section | New home | Carried |
|---|---|---|---|
| `README.md` | introduction | `README.md` | yes |
| `README.md` | Sisällys | `README.md`, rebuilt by moment | yes |
| `README.md` | Mitä on valmista ja mitä ei | status "Yleiskuva"; one line on the front page | yes |
| 01 | introduction, division of work | `concepts/division-of-work.md` | yes |
| 01 | Kenelle | `start/is-it-for-you.md` | yes |
| 01 | Mitä aggountant ei tee | `start/is-it-for-you.md` | yes |
| 01 | Miksi tositetta ei voi muokata | `concepts/sealing-and-approval.md` | yes |
| 02 | Ohjelman asennus | `start/install.md` (`--help`, per-command help, agent reads it) | yes |
| 02 | Mikä yrityskoti on (table) | table at the end of `concepts/company-home.md` | yes, plus `.aggountant/tulorekisteri.json` |
| 02 | Kodin perustaminen | `start/set-up-company.md` (init, 98 accounts, `.gitignore`, `ALREADY_INITIALIZED`, no company questions, `chart list`); practice home in `start/practice-month.md` | yes |
| 02 | Miten ohjelma löytää kodin | `concepts/company-home.md` (order, marker, `config show`, config file); wrong path → `troubleshooting.md` and status | yes |
| 02 | Git ja varmuuskopio | why: `concepts/company-home.md`; how: `tasks/back-up-and-restore.md`; no backup command: status | yes |
| 02 | Käyttöönotto aiemmasta järjestelmästä | `start/set-up-company.md`; history import: status | yes |
| 02 | Tilikausi ja kaudet | `concepts/periods.md`; calendar year only: status | yes |
| 03 | introduction, skills | `start/install.md` | yes |
| 03 | Skillien asennus | `start/install.md`; reinstall after upgrade: `tasks/upgrade.md` | yes |
| 03 | SOUL.md | `concepts/division-of-work.md` | yes |
| 03 | Mitä agentti tekee itse | `concepts/division-of-work.md` | yes |
| 03 | Mitä sinä hyväksyt (table), binding, good request | `concepts/sealing-and-approval.md`; approval step of each task | yes |
| 03 | Yrityksen tietovarasto | `concepts/company-home.md` | yes |
| 03 | Kodin AGENTS.md | writing: `start/set-up-company.md`; what it is: `concepts/company-home.md` | yes |
| 03 | Salasanat eivät kuulu kotiin | `concepts/company-home.md`; one line in `set-up-company.md` and `set-up-vero-api.md` | yes |
| 04 | introduction, order | `tasks/monthly-routine.md` | yes |
| 04 | Pankin tiliote | `tasks/import-bank-statement.md`; limits in status | yes, corrected (see below) |
| 04 | Kuitit ja laskut | `tasks/record-receipts.md`; VAT choices and rates: `concepts/vat-treatment.md` | yes |
| 04 | Erikoiskirjaukset | `tasks/record-other-entries.md` | yes |
| 04 | Poikkeukselliset kuitit ja konsultointi | status; one line in `record-receipts.md` | yes |
| 04 | Puuttuvat kuitit, `hunt-receipts` | `record-receipts.md` "Kun kuittia ei löydy"; status | yes |
| 04 | Velvoitteet | `concepts/periods.md#velvoitteet`; check in `close-month.md` | yes |
| 04 | Kauden sulku | `tasks/close-month.md` (from the sample) | yes |
| 04 | rejected close stays `closing` | status "Hylätty sulku"; `troubleshooting.md`; warning in `close-month.md` | yes |
| 05 | introduction, two ways to file | `tasks/file-vat-omavero.md`, `tasks/file-vat-direct.md` | yes |
| 05 | Ilmoitettavat luvut | preview step of both VAT pages; `PERIOD_NOT_SEALED`; replacement return: status | yes |
| 05 | preview text says file by hand | status | yes |
| 05 | Vero.fi-käyttöönotto, mitä tarvitset | `tasks/set-up-vero-api.md` | yes |
| 05 | Tunnukset koneellesi | `tasks/set-up-vero-api.md` step 1; fingerprint and key leak there too | yes |
| 05 | Valmiuden tarkistus (blocker table) | `reference/vero-fi-blockers.md`, grouped by owner as the code assigns them | yes, plus `VERO_PROFILE_INVALID` and the three expiry warnings |
| 05 | Käyttöönoton vaiheet | `tasks/set-up-vero-api.md` | yes |
| 05 | Varmenteen uusiminen | `tasks/set-up-vero-api.md#kun-varmenne-vaihtuu` | yes |
| 05 | Suora ALV-ilmoitus | `tasks/file-vat-direct.md` | yes |
| 05 | Maksu (`ApprovePayment`) | status; one line in `file-vat-direct.md`; `troubleshooting.md` | yes |
| 05 | Ilmoitus ei näy OmaVerossa | `troubleshooting.md`; why: `concepts/filings.md` | yes |
| 05 | Epävarma lopputulos | why: `concepts/filings.md`; what to do: `troubleshooting.md` | yes |
| 05 | Ilmoitus käsin OmaVerossa | `tasks/file-vat-omavero.md` | yes |
| 05 | Kuukausi- ja neljännesvuosikaudet | `concepts/periods.md` | yes |
| 06 | Mitä voi tehdä; payroll not usable | status "Palkat"; payroll voucher in `record-other-entries.md` | yes |
| 06 | Työntekijän tiedot | `tasks/report-travel-allowances.md`; plaintext store, display, entry: status | yes |
| 06 | Matkakorvaukset, käyttöönotto | `tasks/report-travel-allowances.md`; marker role: `concepts/company-home.md` | yes |
| 06 | Ilmoituksen teko | `tasks/report-travel-allowances.md` | yes |
| 06 | Epävarma lopputulos ja varapolku | `concepts/filings.md`; `troubleshooting.md` (two sections) | yes |
| 07 | Kuukausiraportti | `tasks/monthly-report.md` | yes |
| 07 | Kausien ja tilikauden tila | `concepts/periods.md`; `reference/commands.md` | yes |
| 07 | Ketjun tarkistus, tositteiden haku | `concepts/sealing-and-approval.md`; `tasks/correct-voucher.md`; `reference/commands.md` | yes |
| 07 | Tilinpäätös, `close-year` | `tasks/year-end.md`; detail in status | yes |
| 08 | Miten virhe näkyy | `reference/error-codes.md` | yes |
| 08 | Tavallisimmat virheet | `reference/error-codes.md`, alphabetical | yes, plus five codes users meet |
| 08 | Keskeytynyt ajo | `troubleshooting.md#ajo-keskeytyi` | yes |
| 08 | Väärin sinetöity tosite | `tasks/correct-voucher.md` | yes |
| 08 | Ketjun tarkistus epäonnistuu | `troubleshooting.md`; restore in `tasks/back-up-and-restore.md` | yes |
| 08 | Tositteet näyttävät kadonneen | `troubleshooting.md` | yes |
| 08 | Kausi jäi suljettavaksi | `troubleshooting.md` | yes |
| 08 | Ohjelman päivitys | `tasks/upgrade.md` | yes |
| 08 | Testiympäristön muuttuja | `troubleshooting.md` and status | yes |
| 09 | all entries | `reference/glossary.md` | yes, plus four terms the new pages use |

## Statements corrected rather than copied

- **Bank row for an already recorded receipt.** Chapter 4 said such a row "is
  not recorded again". The import requires an assignment for every booked
  row, so the guide now says that, names the double-booking risk, says the
  bank-balance check catches it, and tells the owner to ask the agent how each
  such row is assigned (`import-bank-statement.md`, status
  "Pankkitililtä maksettu ostolasku", `troubleshooting.md`). No convention is
  invented; issue `bank-row-for-recorded-receipt` owns that choice.
- **Pending bank entries** are skipped by the import (design fact 3); now in
  `import-bank-statement.md` and the tutorial.
- **`voucher show`** prints only number and hash without `--json` (design
  fact 4); pages say "ask the agent to show the voucher".
- **Installation** describes the source install that works today (`cargo
  install --git …`, verified), not the release installer.

## Decisions for later review

- **Status page names the tracking issue** for each limitation, as the design
  asked, by slug in backticks rather than a link, because `issues/` is outside
  a future mdBook source tree. Added limitations that the old guide did not
  mention but open issues describe as current behaviour: VAT fields 309–311
  (EU and zero-rated sales) are not derived, a correction-dominated negative
  VAT period cannot be filed (and errors as `VAT_CURRENCY`), an empty camt
  report imports silently, the payee IBAN is given by hand each time, and an
  invoice cannot be attached to a voucher created from the bank statement.
- **Year end** says plainly that year-end entries cannot yet be booked into
  the closed year. I first wrote "book the accountant's entries as special
  entries" and removed it: with December sealed they would land in the next
  year's result, which is the same flaw `close-year` has.
- **VAT coding of special entries.** The VAT return is summed only from
  VAT-coded postings (`crates/domain/src/vat/code.rs`), so
  `record-other-entries.md` and `concepts/vat-treatment.md` tell the owner to
  check that the agent's VAT legs carry a VAT code, and the status page lists
  it. Reproduced (a sales voucher with 503,90 € on 2939 and no code gives a
  0,00 € return) and filed from this run as the unlaned intake issue
  `vat-untagged-posting-omitted`.
- **Receipts as text files** (`.txt`) instead of the design's PDFs, so the
  material stays readable by the publication scanner and needs no generator.
  The tutorial says real receipts are usually PDFs or images.
- **Counterparties without business IDs** in the practice material: a made-up
  ID with a valid check digit can be a real company's. The rule is in
  `docs/user/AGENTS.md`.
- **`run-practice-month.sh` lives in `practice/`** so the coming parity check
  (`user-guide-parity-check`) can call it. It is not wired into
  `scripts/ci-local.sh` here; that is the parity-check issue's scope.
- **Templates in `docs/user/AGENTS-TEMPLATES.md`**, not copied into
  `AGENTS.md`, to keep the main rules readable. The design issue still holds
  the seven decisions as open questions; this draft takes all seven as
  recommended.
- **Length.** The Finnish guide is about 3 200 lines against about 1 300
  before, more than the design's estimate. The growth is the tutorial, the
  "is it for you" and bookkeeper pages, troubleshooting by symptom and the
  status page's honest detail. Worth trimming after Jari reads it.
- **Small wording choices:** "suora ALV-ilmoittaminen" for the Vero API path,
  "harjoituskoti" for the practice home, "kirjaus" for an import assignment
  (the user never sees "kohdistus" on screen), "Tulorekisteri-profiili" for
  `.aggountant/tulorekisteri.json`.
- **Transitional rule for English:** until `user-guide-rewrite-en` lands, a
  fact change goes to the Finnish page and the English chapter carrying it
  (`docs/user/AGENTS.md`).

## Observations

- `aggountant period --help` says it can "mark a sealed period's VAT return
  filed", but `period` has only `show`. Not filed: it is help text only and
  the guide does not rely on it.
- The README's Documentation section still says the guide "explains the
  monthly cycle, VAT and Vero.fi, …"; it links to `docs/user/fi/README.md`,
  which still exists. No change needed for the link; the README is outside
  this issue.
