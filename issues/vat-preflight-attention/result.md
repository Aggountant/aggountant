# Result: vat-preflight-attention

When `file-vat start` cannot use the Tax Administration's period list, the Run
now stops visibly. If the `GetVATPeriods` read fails, the response is
malformed, the period is missing, or its remote status is not
`Unfiled`/`Missing`, the `reconcile` effect is journalled `EffectRejected`
with the reason code (`FETCH_FAILED`, `MISSING_CREDENTIAL`, …,
`VERO_VAT_PERIODS_MALFORMED`, `VERO_VAT_PERIOD_MISSING`,
`VERO_VAT_PERIOD_NOT_FILEABLE`). `run show` then shows the Run as
`needs_attention`. The error names the Run and says nothing was sent. Nothing
retries the Run. The user starts a new Run once the cause is fixed, and the
parked Run does not block that, because only the `submit-filing` effect counts
as an unresolved filing.

A filing projection the adapter refuses (for example `VERO_VAT_CROSS_FIELD`)
is now journalled the same way. Before this change it also left the effect
`executing`.

- Code: `crates/process/src/file_vat.rs` (`read_fileable_vero_period`, the
  rejections in `start_file_vat`).
- Tests: `a_failed_period_read_parks_the_run_needs_attention` and
  `a_period_the_authority_will_not_accept_parks_the_run_needs_attention` in
  `crates/process/tests/file_vat.rs`.
- Docs: `docs/user/{en,fi}/tasks/file-vat-direct.md`, `skills/vero-fi/SKILL.md`.
- Review and assessment: `review.md`, `assessment.md`.
