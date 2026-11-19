---
created: 2026-11-19
updated: 2026-11-19
type: improvement
status: open
priority: normal
preparation: ready
lane: cli-canon
lane_seq: 6
collision: [crates/cli/src/main.rs]
---

# CLI friction from production use: knowledge company id, chart list output, report statements to stdout

## Description

Three small CLI frictions met by an accounting agent in production use. Each
made the agent guess and fail once.

1. **`knowledge company` takes the fact id in two different ways.** `add`
   reads `id` from the stdin JSON (`{id, statement, scope, source}`), while
   `update` and `retire` require `--id <ID>` and do not accept the id
   positionally or in the JSON. An agent that has just added
   `{"id":"bank-fees-practice",…}` naturally tries
   `knowledge company update bank-fees-practice < change.json` and gets a
   usage error.
2. **`chart list` without `--json` prints only the chart version and the
   account count**, not the accounts. The command exists to show the accounts,
   so the human output should list them (number, name, kind; one per line).
3. **`report statements` has no stdout default.** `--format` and `--output`
   are both required, so a quick look at a month's result needs a temporary
   file. A missing `--output` could write to stdout (or `--output -`), with
   `--format` defaulting to `native-json` when writing to stdout; existing
   files are still never overwritten.

Example (Esimerkki Oy, fictional): `aggountant chart list` →
`chart fi-general-2026@…: 214 accounts`; the agent then has to rerun with
`--json` and parse it to see whether 2740 exists.

## Wanted

- `knowledge company update|retire` accept the id as a positional argument
  as well as `--id`, and (for symmetry) an `id` in the stdin JSON that must
  agree with the argument when both are given. `--help` examples for all
  three.
- `chart list` human output lists the accounts.
- `report statements` writes to stdout when `--output` is omitted or `-`;
  `--format` defaults to `native-json` there. `--help` shows the quick form.
- Tests for each; no change to JSON output shapes.
