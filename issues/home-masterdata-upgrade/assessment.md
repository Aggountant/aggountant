# Assessment of the review findings

| ID | Finding | Classification | Likelihood | Outcome | Notes |
|---|---|---|---|---|---|
| F1 | A failed temporary write leaves a stray file | CONFIRMED | RARE (disk full during upgrade) | FIX (done) | The temporary file is now removed on a write error. Not a data-integrity issue: no existing file is touched, and `open` and a re-run ignore the leftover. |

No data-integrity finding. The verifier's probe confirmed that apply only
creates the files the plan listed, never replaces an existing one, and does not
change the active chart.

Simplicity: the upgrade reuses the store's own readers (`open` was split into
`read_active_chart`, `read_snapshots` and `parse_vat_roles`, so the open checks
and the upgrade checks are the same code), init's rendering of the snapshot and
role files (`render_chart_file`, `bundled_vat_roles_bytes`, `snapshot_path`,
which init now uses too), and `validate_chart_vat_roles` to predict whether the
home opens. The only new structure is `BUNDLED_CHARTS`, a list with one entry:
appending to it is how a newer binary ships a new edition, so it is the point of
this feature, not a future abstraction. Left out on purpose: merging new roles
into an existing role file, any repair of differing files, and an audit log in
the home (git history of `masterdata/reference/` serves that).
