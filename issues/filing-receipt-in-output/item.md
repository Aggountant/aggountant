---
created: 2026-11-19
updated: 2026-11-19
type: improvement
status: open
priority: normal
preparation: ready
lane: vat-filing
lane_seq: 5
collision: [crates/process/src/lib.rs, crates/cli/src/wage.rs, skills/vero-fi/SKILL.md]
---

# Authority receipt (VAT UniqueIdentifier, Incomes Register IRDeliveryId) missing from approve output and run show

## Description

After an authority filing succeeds, the authority's receipt identifier is hard
to find:

- `file-vat approve-filing` and `file-wage approve` return the generic
  `RunView` (`crates/process/src/lib.rs`), whose `effects` carry only `key`,
  `effect_type` and `status`. The Vero API `UniqueIdentifier` / receipt time and
  the Incomes Register `IRDeliveryId` / `IRItemId` (and report status) are not
  in the output.
- `run show --run-id` returns the same `RunView`, so it does not show them
  either.
- For VAT, `file-vat status --period` does answer from the stored receipt, but
  nothing in the approve output points there.
- For `file-wage` there is no read-only way to see the receipt: `reconcile`
  makes a signed network query, which is the wrong tool just to read what is
  already recorded.

In practice an agent recording a filing in the company's notes ends up
grepping the run journal files under `process/runs/*/` for the identifiers,
which bypasses the product's own interfaces.

Example (Esimerkki Oy, fictional): the October VAT return is approved and
accepted; `file-vat approve-filing` prints the run with
`{"key":"filing","effect_type":"…","status":"done"}` and no receipt. The same
for a travel-expense earnings payment report sent with `file-wage approve`.

## Wanted

- The approve commands' output includes the receipt once the effect has
  completed: for VAT the `UniqueIdentifier` and receipt timestamp (whatever the
  stored receipt holds); for Tulorekisteri the `IRDeliveryId`, `IRItemId`s and
  the delivery status. An unknown outcome says so and names the reconcile
  command.
- `run show` includes the same receipt for completed authority effects (an
  `outcome`/`receipt` field on the effect view, not a raw journal dump).
- A read-only, offline way to show a `file-wage` run's recorded receipt (a
  `file-wage show <run-id>` or receipt in `run show`), distinct from
  `reconcile`.
- The skills (`vero-fi`) say where the receipt is read from, so agents stop
  reading journal files.
