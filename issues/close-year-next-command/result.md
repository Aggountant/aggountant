# Result: close-year-next-command

Every `close-year` command (`start`, `answer`, `seal-correction`, `resume`,
`show`) now returns `message` and `next_command` beside the run view, computed
in one place, `ProcessEngine::close_year_next`:

- open gate, reported by a command other than `show`: the message names the
  gate and what approving it seals (`ApproveAdjustments`: the depreciation and
  accrual vouchers, permanently; `ApproveTaseErittelyt` / `ApproveLiitetiedot`:
  nothing; `ApproveStatements`: the fiscal year, irreversibly), and
  `next_command` is `close-year show --run-id …`;
- reported by `show`: `next_command` is the first unsealed correction's
  `seal-correction` at `ApproveAdjustments`, otherwise
  `close-year answer … --answer '<approve|reject>'` (a quoted placeholder that
  the CLI refuses if run unchanged);
- a run another close-year run superseded: reject its open gate;
- rejected, or completed with its archive: no `next_command`;
- completed without its archive, or stopped between gates: `close-year resume`.

The skill no longer labels which approvals are permanent; it follows
`message` and `next_command`. Review and assessment: `review.md`,
`assessment.md` in this directory (covering also `close-year-plain-help` and
`close-year-archive-write-error`, landed in the same change).
