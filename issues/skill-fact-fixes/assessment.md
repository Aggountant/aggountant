# Assessment of the skill-fact-fixes review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `VERO_VAT_PERIOD_NOT_FILEABLE` is not an `error_code` | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX — applied |
| F2 | init skill still says obligations come from the exceptional-receipt route | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX — applied |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: confirmed in `file_vat.rs`; the row now names where the code appears.
  The underlying `PROCESS_STORE`/system classification of a caller-fixable
  refusal predates this slice. It is a real but separate wart (an agent sees
  exit 2 for "period already filed at Vero"); not filed here because nothing
  in this run observed it causing a wrong action, and the message plus
  `run show` carry the reason.
- F2: confirmed; the only obligation-opening site is the consult Run. Same
  kind of wrong fact the issue is about, in a file this slice already edits.

No data-integrity finding remains open. No follow-up issue filed.

## Simplicity

- Reused: the bundling table in `skill.rs` is still the one list of what a
  skill ships. `--file` resolves through `linked_file`, the same rule the
  link test used to restate inline; the test now calls it, so "what a skill
  may link to" and "what `skill print --file` can print" are one rule.
- Repetition removed: the link test's own path resolution.
- Left out on purpose: `--with-requires` (printing several files in one
  stream would need a separator format and break byte identity), a new
  error-code row for `SKILL_FILE_NOT_FOUND` (the skill commands' codes are not
  in the guide's list, and the message lists the valid files), and any
  rewrite of the skills beyond the wrong facts.
