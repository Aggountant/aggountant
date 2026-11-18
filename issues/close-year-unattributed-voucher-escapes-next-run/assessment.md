# Assessment of the review

Source: `review.md` beside this file (head c5bf08b).

| id | finding | confirmation | likelihood | readability | architecture | confidence | recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Run interrupted after `ApproveStatements` has no gate to reject once another run's orphan appears | CONFIRMED (pre-existing for every binding refusal on resume) | RARE | WORSENS (a new recovery gate kind) | MODERATE | HIGH | DROP (1b) |
| F2 | Hand-sealed `closeyear:<year>:` voucher blocks the close | CONFIRMED | COSMIC_RAY | NEUTRAL | NONE | HIGH | DROP (1b) |

F1: the year stays open and the books are right; a later run that includes the
entry completes the close, after which the wedged run resolves as superseded on
resume. The same wedge already exists for the other binding refusals, so a
recovery gate would be a separate design if it is ever wanted, not part of this
fix.

F2: a refusal, not wrong books, and only reachable by sealing a voucher by hand
inside the close-year run's own key namespace.

No confirmed data-integrity finding; nothing blocks the merge. Nothing is filed.

## Simplicity

The change reuses the existing unplanned-voucher guard and its three call sites
rather than adding a fourth check: one function now covers both an attributed
and an unattributed close-year voucher outside the plan. The `closeyear:<year>:`
prefix is stated once (`close_year_key_prefix`) and the three key builders use
it. The ledger gets one generic read by key prefix; it does not learn the
close-year namespace. Left out: a check inside `Ledger::seal_year` (it would put
a process key convention in the ledger, and `run_seal_year` is the only caller
and already runs the binding first) and any automatic attribution.
