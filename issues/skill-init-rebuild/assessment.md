# Assessment: review of `skills/aggountant-init/SKILL.md`

Review: `/llm-skill-review`, lenses executor and design-fidelity, one model (`gpt-6-astra`), 2026-11-21. Raw output is in `review.md` beside this file. The model's agreement is not evidence; each row was checked against the file, the code or the built binary.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | "Continue the previous chart" has no executable path [executor 1] | CONFIRMED | OCCASIONAL | NEUTRAL | MODERATE | HIGH | SPIN-OFF |
| F2 | First fiscal year and opening date depend on each other [executor 2] | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX (applied) |
| F3 | Mid-month cut-over cannot give a zero month result [executor 3] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX (applied) |
| F4 | Questions that change balances came after the opening approval [executor 4] | CONFIRMED | REGULAR | NEUTRAL | NONE | HIGH | FIX (applied) |
| F5 | Year-to-date check contradicts the year-boundary case [executor 5] | CONFIRMED | REGULAR | NEUTRAL | NONE | HIGH | FIX (applied) |
| F6 | One date named for two boundaries [executor 6] | CONFIRMED | RARE | IMPROVES | NONE | MED | FIX (applied) |
| F7 | "The total matches" does not tie the mapping to the voucher [executor 7] | CONFIRMED | RARE | NEUTRAL | NONE | MED | FIX (applied, one clause) |
| F8 | The 1 January exception was lost from the fiscal-year paragraph [design-fidelity 1] | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX (applied) |
| F9 | `AGENTS.md`/`TODO.md` content repeats SOUL; `TODO.md` got a mandatory layout [design-fidelity 2] | CONFIRMED in part | OCCASIONAL | IMPROVES | NONE | MED | FIX (applied in part) |
| F10 | `home upgrade` still a small procedure; "after that" followed a dry run [design-fidelity 3] | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX (applied) |
| F11 | Set-up questions for `AGENTS.md` are discovery, not house rules [design-fidelity 4] | INTENTIONAL | - | - | - | HIGH | KEEP |
| F12 | Fiscal-year paragraph keeps when and why [design-fidelity 5] | INTENTIONAL | - | - | - | HIGH | KEEP |

FIX: 9   SPIN-OFF: 1   KEEP: 2   DROP: 0

## Evidence

- **F1** `chart` has only `list` (`chart --help`); `init` always writes `fi-general-2026`; no help or guide page installs another chart. The question is inherited from the old skill and is also in the guide (`start/set-up-company.md` step 3), which this slice does not own. Left as it was in the skill; filed as `distinctly-tired-hearing`.
- **F2** Both readings were valid in the old text too. For a company whose year starts 1 July one could define the old year and open on 30 June under the strict cut-over condition, or define the new year and open on 1 July. Checked the second on a synthetic home: `fiscal-year define --start 2026-07-01 --end 2027-06-30`, then `fiscal-year show --date 2026-06-30` answers `FISCAL_YEAR_UNDEFINED`, a balance-sheet-only voucher seals on 2026-07-01 and `report statements --period 2026-07` runs. The skill now says which to take: the calendar-year company uses the boundary cut-over (31 December always exists), the other defines the new year first. This is the rewrite's recommendation, not something the old text decided; see `result.md`.
- **F3** Monthly reports select by posting month, so an opening with result accounts dated mid-month lands in that month's result whatever the day. One sentence: it happens anyway mid-month, so propose a month boundary.
- **F4** Sections read in order sealed the opening before the questions that can change its balances (shareholder-paid invoices, tax account interest). A pointer in the opening paragraph; the section order stays because the questions refer to the opening.
- **F5** At a year boundary the imported voucher has no result accounts, so the year-to-date result is zero, not the source report's. The check now says so.
- **F6** "the day from which aggountant bookkeeping starts and to which the previous system ends" is two days. Now named separately.
- **F7** Added "and to the opening voucher" to the mapping file's totals. Not expanded into a reconciliation procedure.
- **F8** `crates/model/src/fiscal_year.rs` `year_of`: a date before the first defined year resolves to a calendar year when that year starts 1 January. "ellei se ala 1.1." restored.
- **F9** The `AGENTS.md` list is what set-up asks the company (see F11) and stays; SOUL names the file's purpose, not the questions, and sends a missing file here. The `TODO.md` sentence did repeat SOUL's three contents and prescribe three sections: now "a section for each thing SOUL names" for a new file, with the example kept.
- **F10** Reduced to the two triggers and the reason not to repair by hand. The `MASTERDATA_CORRUPT` trigger stays because the error does not name the remedy (reproduced; filed as `nominally-undesirable-afterthought`). The hand-repair reason is not in the help: the chart version is a content hash (`init --help`) that sealed vouchers pin.
- **F11**, **F12** The reviewer's own reading: legitimate. Where expense drafts and mileage are kept, and the tax authority's mileage-log fields, are in no other file.

## Simplicity

One file changed. It reuses SOUL for every house rule (what the two files hold, upkeep, secrets, commit timing, pending work is not a fact, `obligation`) and `--help` for `fiscal-year define` and `home upgrade`. No new reference file: at about 930 words the body is under the thousand-word mark of `skills/AGENTS.md`, and everything in it is needed on every set-up. Left out on purpose: a set-up checklist, the list of commands for inspecting an existing home, and any description of the cut-over mechanism beyond the condition the agent must meet before sealing.
