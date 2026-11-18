# Assessment: close-year guards review

Source review: `review.md` beside this issue.

| id | title | confirmation | likelihood | recommendation | applied |
|---|---|---|---|---|---|
| F1 | Unattributed close-year voucher of an abandoned run escapes the next run | CONFIRMED (reading, verifier) | RARE | SPIN_OFF | filed `close-year-unattributed-voucher-escapes-next-run` |
| F2 | Compute-stage unplanned check wedges a resumed run | CONFIRMED (test) | RARE | FIX | yes, `fa896f3` |
| F3 | Binding check refuses on the SealYear resume path without an open gate | CONFIRMED (reading) | COSMIC_RAY | DISCUSS → keep | no; refusing beats sealing an entry no gate showed |
| F4 | Remediation text promised a replay that different figures refuse | CONFIRMED | OCCASIONAL | FIX | yes, message names "the same figures" |

No confirmed data-integrity finding introduced by this change remains. F1 is
data integrity but predates the change and needs a way to find unattributed
vouchers by key, which is its own slice.

## Simplicity

The three guards reuse the existing refusal codes (`CLOSE_YEAR_BLOCKED`,
`GATE_NOT_OPEN`) and the existing precondition slot in `answer_close_year`
(checks before `GateAnswered` is journaled). The plan's natural keys and the
attributed keys are now built in one place each (`planned_natural_keys`,
`attributed_natural_keys`), shared by the existing missing-attribution check
and the new converse, instead of the inline construction the old check had.
`assert_year_open` is one function called from the three entry points named in
the issue. Left out: recovering a missing entry from the ledger automatically,
and a run-level reject verb for Runs with no open gate.
