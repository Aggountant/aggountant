# Result: user documentation design

Delivered beside this file:

- `design.md`: readers, what they do on day one, monthly and at year end,
  what to borrow from Diátaxis, GitHub Docs, hledger, Beancount, the Good Docs
  Project and Procountor (with links), the page list and navigation, templates
  for the five page types, how Finnish and English stay in step, the format
  recommendation, a section-by-section migration map from the current guide,
  facts to settle before the rewrite, and seven decisions for Jari with a
  recommendation each.
- Three Finnish sample pages, one per main page type:
  `naytesivu-aloitus-harjoittele-ensimmainen-kuukausi.md` (tutorial),
  `naytesivu-tehtava-sulje-kuukausi.md` (task),
  `naytesivu-selitys-sinetointi-ja-hyvaksynta.md` (concept).

The commands, outputs and figures in the samples were run against the 0.1.0
binary in throwaway homes with synthetic Esimerkki Oy material. `design.md`
("How the samples were checked") lists what was run.

## Decisions for later review

- **Five page types, folders by type, front page by moment.** Diátaxis's four
  types plus troubleshooting as its own type (GitHub Docs, Good Docs). The
  front page groups pages by when they are needed (aloita, joka kuukausi,
  harvoin, vuoden lopussa, kun jokin menee vikaan) because the readers arrive
  with a task that is due, not with a page type in mind.
- **31 pages per language instead of 10**, with the rule that every fact has
  one home page and other pages link to it. Pages I considered and merged to
  keep the count down: certificate renewal (into `set-up-vero-api`), the
  Tulorekisteri setup and the employee record (into `report-travel-allowances`),
  the home-files and states reference pages (into `concepts/company-home` and
  `concepts/periods` plus the glossary).
- **A practice home and bundled practice material for the tutorial.** Sealing
  and closing are permanent, so the first lesson happens somewhere disposable.
  The practice statement starts at 0,00 € and has the owner paying in share
  capital. That keeps the bank balance, the statement and the balance sheet in
  agreement without opening balances or a negative receivable. It also needs
  no real company in the test fixture.
- **Commands folded under the steps** in a `<details>` block, which GitHub and
  mdBook both render.
- **English slugs, identical paths in both languages.** This reverses a rule in
  `docs/user/AGENTS.md`, so it is listed as question 3 for Jari.
- **Markdown on GitHub, mdBook-compatible**, matching the approved
  `docs_site: none`. I checked that hledger's site is mdBook by its page chrome.
- **Sample-page file names** are Finnish and prefixed `naytesivu-` so they read
  as samples inside the issue. A comment at the top of each gives its future
  path.
- **The sample pages describe agent behaviour in general terms** and quote only
  aggountant's own output exactly, since the agent's wording varies.
- **`design.md` is in English**, like the rest of the issue; the samples and the
  templates are in Finnish because `fi/` is the original.

## Observations from checking the binary

- Filed as the unlaned intake issue `bank-row-for-recorded-receipt`: the guide
  says a bank row for an already recorded receipt is not recorded again, but
  `import camt053 seal` requires an assignment for every booked row. The
  run policy allows that one file outside `issues/user-docs-design/`.
- Not filed, recorded in `design.md` for the rewrite: the README's install
  region describes a GitHub Release installer although no release exists yet
  (issue `readme-plain-intro` owns the README); the import skips pending (`PDNG`)
  entries, which the guide does not mention; `voucher show` without `--json`
  prints only the number and hash.
- Not filed, small: after a period is sealed, `capture-receipt record --dry-run`
  for a date in it still reports `ledger_preflight_valid: true`, and only the
  seal refuses with `PERIOD_SEALED`. Nothing wrong is stored, but a user can be
  asked to approve a preview that cannot be sealed. It is worth a look if
  previews are ever approved in batches (`month-batch-approval`).
- Re-importing an already sealed statement prints "sealed 2 vouchers" again,
  although no new vouchers are created. The tutorial says what actually happens
  instead of quoting that line.

## Follow-ups proposed (not filed)

- Rewrite `docs/user/` along `design.md` once Jari has answered the questions.
- Add `docs/user/practice/` with the synthetic material.
- Add a guide check to `scripts/ci-local.sh`: the same file set in both
  languages, the same headings per page, every `aggountant` command and flag in
  the guide present in `--help`, and the practice statement still imports.
