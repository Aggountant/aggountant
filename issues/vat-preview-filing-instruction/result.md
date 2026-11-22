# Result

`file-vat preview` on a sealed, unfiled period now names both filing routes
instead of saying aggountant cannot file:

- direct filing through the Vero.fi API when `aggountant vero-fi status`
  reports ready: `file-vat start` with these figures, then the `ApproveFiling`
  gate (`file-vat approve-filing`);
- otherwise manual filing in OmaVero (or a self-prepared Ilmoitin.fi file),
  recorded with `file-vat complete-manual`.

The preview stays offline and evaluates no readiness, so the text says it did
not check which route is available. Wiring the local readiness evaluator into
the preview was left out on purpose: it would read the Vero profile and the
credential bundle path, which the preview's contract (no profile, credentials
or network) excludes, and the dedicated `vero-fi status` already answers it.
The other preview states (open, closing, already filed) are unchanged.

Tests: `offline_preview_is_read_only_and_uses_sealed_ledger_figures` asserts
both routes, the "not checked" statement and the absence of the old claim.
User guide: the "guidance text is out of date" limitation is removed from
`reference/status.md` and `tasks/file-vat-omavero.md` (en, fi), and the
preview step explains why both routes are named.

Review: focused self-review of a one-string change plus docs, as the brief
allows for a small text change; no model review. Simplicity: no new field or
abstraction; one `&'static str` replaced, existing commands referenced.
`scripts/ci-local.sh` passed.
