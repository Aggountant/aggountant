# Assessment of the run-commands-workflow-guard review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Workflow-specific guards lacked regression coverage | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: the guards themselves are correct; the gap was that removing one would
  go unnoticed. One table test over all sixteen entry points closes it. While
  writing it, `leave_file_vat_payment_to_user` turned out to carry its own
  older copy of the workflow check without the "continue it with" hint; that
  copy was removed so the rule lives only in `check_version`.

No confirmed data-integrity finding remains open; nothing was filed.

## Simplicity

- Reuses the existing `check_version` that every command already calls; the
  workflow comparison is added there instead of as a separate guard in each
  command, and the duplicate file-vat check is removed.
- `expected_version` keeps its one table and loses its close-period fallback.
  The Incomes Register workflows keep their own version checks; mapping them
  here would need their private `version()` functions and was left out, so
  `run migrate` refuses them rather than inventing a target.
- No new error code: the refusal uses `INVALID_ANSWER`, which the file-vat
  payment command already used for the same situation.
