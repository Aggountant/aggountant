# Result: English user guide mirroring the Finnish guide

`docs/user/en/` now has the same 32 files as `docs/user/fi/`, path for path:
the front page, four `start/` pages, fourteen `tasks/`, seven `concepts/`,
`troubleshooting.md` and five `reference/` pages. Each pair has the same
headings in the same order. The nine numbered English chapters are removed.
`docs/user/AGENTS.md` loses the transitional rule for the old chapters and
gains the English writing conventions below. `docs/user/practice/README.md`
links to the English tutorial too; the practice material and script are
unchanged, because the English tutorial uses the same files and figures.

## How it was checked

All against `aggountant 0.1.0` built from this tree (release build), in
throwaway homes under the worktree's `history/`:

- `docs/user/practice/run-practice-month.sh <binary>` passes. The English
  tutorial quotes the same figures and outputs as the Finnish one. The text
  form of `run start close-period` prints the `run "cp-…" — "awaiting_gate"`
  line the tutorial quotes (the script reads the JSON form, so it does not
  show it).
- Every `aggountant …` command and flag in both languages (99 mentions each)
  exists in the binary's `--help`, using a scratch checker that walks the
  subcommand tree and was tested against a planted bad flag. The subcommands
  named only in prose (`file-vat retry`, `knowledge company retire`,
  `capture-receipt start/answer/resume`, `hunt-receipts`, `intake`,
  `run-payroll`, `close-year resume`, `obligation resolve`,
  `file-wage complete-manual`, `employee show`, `vero-fi verify`) exist.
- Every relative link and `#anchor` in `docs/user/` resolves by GitHub's
  heading-slug rule, every page has one H1, and every fi/en pair has the same
  heading levels in the same order (scratch checker, tested against a planted
  bad anchor and a changed heading level).
- Every error code the Finnish guide names exists in `crates/`. The Vero.fi
  blocker codes and their `human`/`agent` owners match
  `crates/broker/src/vero_filing.rs` and a fresh home's `vero-fi status`. The
  VAT rates stated (25.5 % from 2024-09-01, 14 % → 13.5 % on 2026-01-01) match
  `crates/domain/data/vat-rates.json`. `skill list` lists the three skills;
  `voucher show` without `--json` prints only number and hash; the report
  prints "Kattavuus: varmistamaton".
- No commits touched `crates/` or `skills/` after the Finnish guide landed.
- Privacy scan of `en/`: no home paths, host names, e-mail addresses,
  business IDs other than 1234567-1 or IBANs other than the practice
  material's fictional one.

## Drift fixed in the Finnish guide

- `fi/tasks/year-end.md` listed "Tilinpäätöskirjaukset on kirjattu
  aggountantiin" under "Valmis, kun". The same page (step 4) and
  `reference/status.md` say year-end entries cannot yet be recorded in the
  closed year, so the check could not be met. It now reads "Olet sopinut
  tekijän kanssa, miten tilinpäätöskirjaukset huomioidaan."

Nothing else in the Finnish guide disagreed with the binary or the code.

## Decisions for later review

- **"VAT" in English prose, "ALV" glossed.** `docs/user/AGENTS.md` lists ALV
  among the names kept in Finnish. Keeping "ALV return" throughout reads
  unnaturally, and the old English chapters already used "VAT return" in
  headings. So the tax is "VAT"; "ALV" is explained on the front page and in
  the glossary. OmaVero, Tulorekisteri, Y-tunnus (Business ID), tilinpäätös,
  kirjanpitolaki and Ilmoitin.fi stay in Finnish with a gloss at first use.
- **Finnish names on screen are quoted with English in parentheses**:
  account names ("1910 Pankkitili (bank account)"), VAT return field names
  (307, 308) and the report's "Kattavuus: varmistamaton". The pages say the
  chart and the report are in Finnish.
- **English notation for figures and dates** (€2,376.00, 20 March) in prose
  and tables. aggountant's output is quoted exactly as it prints.
- **"Manual entry" for erikoiskirjaus.** The old English chapters said
  "special entry", which is a literal rendering; "manual entry" (as in manual
  journal entry) is the usual English bookkeeping term. Other choices:
  voucher (tosite, as in the CLI), seal, period, fiscal year, company home,
  company knowledge store, practice home, obligation, acknowledgement
  (kuittaus), attestation (vakuutus), readiness, posting (vienti).
- **Practice home path** is `~/aggountant-practice` in the English tutorial
  (`~/aggountant-harjoitus` in Finnish). The practice files keep their Finnish
  names and content, and the page says they are in Finnish as real Finnish
  material would be. No change to the practice script was needed.
- **English conventions written into `docs/user/AGENTS.md`**, so the next
  writer keeps them, instead of being left only in this report.
- **Practice README** gets a link to the English tutorial (one line, in
  Finnish like the rest of that file).

## Observations

- The Finnish guide's length (about 3 200 lines) carries over to English
  (about 3 400 lines). The trimming suggestion in
  `issues/user-guide-rewrite-fi/result.md` applies to both languages now; a
  trim must change both in the same commit.
- `aggountant period --help` still says it can "mark a sealed period's VAT
  return filed", but `period` has only `show` (already noted in the Finnish
  result; help text only, the guide does not rely on it).
