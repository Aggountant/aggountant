# Assessment of the close-period placeholder review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `hunt-receipts answer --help` prints `--answer approve\|discard\|acknowledge` | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: help text, not `next_command`, so an agent has to copy the example
  rather than run it; still the same unsafe literal reading as the issue, and
  the brief asks for every such string to be fixed in this change. Within the
  brief's scope (help strings), so fixed here.

## Simplicity

- No new code path: the `next_command` string is the one already built in
  `close_period_view`, now with the same quoted placeholder close-year uses,
  and the refusal is the existing `INVALID_ANSWER` check in `answer`.
- The help texts are reworded in place; no shared constant, since each names
  different gates and answers and clap needs static doc text.
- Left out: a generic "placeholder" helper shared with close-year
  (`answer_command` there builds a different command); the two strings are
  each written once.
