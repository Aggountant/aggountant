---
created: 2026-11-19
updated: 2026-11-19
type: improvement
status: open
priority: normal
preparation: ready
lane: cli-canon
lane_seq: 5
collision: [crates/cli/src/main.rs]
---

# voucher validate refuses the seal envelope that voucher seal accepts

## Description

`voucher validate` and `voucher seal` read different stdin shapes for the same
voucher:

- `voucher seal` reads `SealInput` (`crates/cli/src/main.rs`): `{ "draft": {…},
  "natural_key": "…", "actor": "…" }`, with `deny_unknown_fields`.
- `voucher validate` reads a bare `VoucherDraft`, so the very file that `seal`
  accepts is refused with `unknown field draft`.

An agent that prepares one seal file per voucher and wants to validate it first
has to unwrap `draft` into a second file. The natural workflow ("validate this
file, then seal this file") fails on its first step, and the unwrapped copy can
drift from the file that is actually sealed.

Example (Esimerkki Oy, fictional): `seal-0042.json` holds
`{"draft":{…},"natural_key":"bank:2026-10-03:1","actor":"agent"}`.
`aggountant voucher validate < seal-0042.json` → error `unknown field draft`;
`aggountant voucher seal < seal-0042.json` → sealed.

## Wanted

`voucher validate` accepts the seal envelope as well as a bare draft (the
envelope's `draft` is what gets validated; `natural_key` and `actor` may also be
checked, e.g. an already-used natural key reported as a warning). Existing
bare-draft input keeps working. `--help` for both commands shows the shape
each accepts with an example, and the guide/skill says "validate the same file
you seal".
