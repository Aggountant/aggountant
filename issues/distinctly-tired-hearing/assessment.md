# Assessment of the distinctly-tired-hearing review

| # | Finding | Conf | Like | Read | Arch | Recommendation |
|---|---|---|---|---|---|---|
| 1 | No suitable destination account | UNABLE_TO_VERIFY | RARE | WORSE | NONE | DROP |
| 2 | Collapsed distinctions under "jos mahdollista" | CONFIRMED | OCCASIONAL | BETTER | NONE | FIX — applied |
| 3 | User insists on the old chart | CONFIRMED (handled) | — | — | — | DROP |
| 4 | Stale "keep old chart" knowledge record | CONFIRMED (handled) | — | — | — | DROP |
| 5 | Guide's opening date contradicts skill | INCORRECT | — | — | — | DROP |
| 6 | Guide denies the upgrade command | CONFIRMED | COMMON | BETTER | NONE | FIX — applied |

FIX: 2   DISCUSS: 0   DROP: 4

- 1: The 98-account default chart has an "other" account in every balance
  group (1730, 2560, 2740), so a source account always has a reasoned home;
  what is lost is the distinction, which finding 2 covers. SOUL already
  covers uncertainty. Adding a waiting rule would grow the skill.
- 2: Verified against `crates/ledger/data/fi-general-2026.json`: no named
  account for shareholder debt or the tax account, and both can land on
  2740. Rewritten to "Jos verotili ja osakasvelat osuisivat samalle tilille,
  sovi niiden erottelu", shorter than before.
- 5: Step 4's sub-bullets in both guide languages already give the non-1.1.
  exception; no contradiction.
- 6: `aggountant home upgrade` exists (`crates/cli/src/home.rs`). The
  limitation now links to the upgrade page in both languages.

No data-integrity finding remains open; nothing filed.

## Simplicity

The change removes a question and the decision record it produced; the skill
went from 935 to 929 words. It reuses the existing mapping-file paragraph
instead of adding a chart section, and the guide gains one limitation line
per page plus one status entry pointing at `sie4-chart-import`, the issue that
owns a future chart import. Nothing was added to the program.
