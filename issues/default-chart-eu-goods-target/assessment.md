# Assessment of the default-chart-eu-goods-target review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| R1 | Stale "never rewrites" doc comment on `Ledger::upgrade_masterdata` | CONFIRMED | ALWAYS | IMPROVES | NONE | HIGH | FIX — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 2 (see review)

- R1: the comment is the API contract a caller reads; left as it was it states
  the opposite of what the code now does. One-line fix in this slice.

No data-integrity finding was raised or remains open. No follow-up issue was
filed.

## Simplicity

- Reused: the existing upgrade plan/apply loop and its `UpgradeItem` report,
  `validate_chart_vat_roles` as the gate for the replacement (the same check
  `open` runs), `write_file_synced` / `fsync_dir`, and the CLI's existing
  `would` list (a new `action` value instead of a new list on dry-run).
- Repetition: the rule "this retired file may be replaced" lives in one place,
  `retired_vat_roles_extended`, backed by one domain predicate,
  `only_adds_reverse_charge_targets_to`; the unit test and the run-time check
  call the same code. The VAT return fields needed no change: they already
  came from the sealed treatment.
- Left out on purpose: a structural merge into edited company role files (a
  company's own file stays theirs; the refusal now tells them what to add); a
  new chart edition (it would not reach existing homes, since upgrade never
  switches the active chart); a separate targets file (a second source for the
  same table); changing the `INVALID_ANSWER` code on the capture-receipt
  domain-error mapping (it covers every domain error on that path, wider than
  this issue; the message now names the scope and the remedy).
