# Assessment of the close-year next-step review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `--answer approve\|reject` in `next_command` runs as a shell pipe and approves | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |
| F1b | The same string in close-period's `next_command` | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | SPIN_OFF — filed `close-period-answer-placeholder-pipe` (outside this slice's files) |
| F2 | An obstruction at the archive path read as a written archive | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 2   FIX (with care): 0   SPIN-OFF: 1   DISCUSS: 0   DROP: 0

- F1: an agent that follows SOUL and runs `next_command` literally would
  approve a gate nobody answered; at `ApproveStatements` that seals the year.
  Likelihood OCCASIONAL: agents usually substitute the answer, but the program
  must not make the unsafe literal reading the one that succeeds. Fixed with a
  quoted placeholder that the CLI refuses.
- F2: rare (the write has to fail and leave something at the path), but the
  issue's point is that the failure is never silent. Fixed with `is_file()`.

## Simplicity

- One rule for "what comes next" lives in `ProcessEngine::close_year_next`;
  the CLI only merges `message` and `next_command` into each command's output.
  The engine methods' return types are unchanged, so no test or caller had to
  move.
- The archive failure needs no new journal event or workflow effect (which
  would need a workflow-version migration): the archive file's existence is
  the truth, and the `seal-year` note carries the failure reason.
- Left out: putting the gate content into every command's output (show
  already does it, and the next command points there), and a separate
  "rewrite archive" command (resume already owns completing a sealed run).
- Remaining repetition: the per-gate "what approving seals" wording is also in
  `close-year answer --help`; the help is static text clap needs at compile
  time, so it cannot read the runtime function.
