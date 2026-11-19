# Review: cli-production-friction

- Skill: `/llm-review`, rigor **routine**: CLI input/output conveniences; the
  only write path touched is `knowledge company update|retire`, which writes
  company-knowledge prose, not the books.
- Scope: commit `4d17453` against `07a3a7c` (`crates/cli/src/knowledge.rs`,
  `crates/cli/src/reports.rs`, `crates/cli/src/main.rs`, tests, guide, SOUL,
  CHANGELOG).
- Finder: `claude-fable-5-2`, one call (the diff plus `knowledge.rs` and
  `reports.rs`). No verifier: the one candidate was reproduced locally against
  the built binary, so a refutation call had nothing left to decide. One model
  call in total.
- Risk class: strict input validation on a write path.

## F1 (should-fix): `update`/`retire` stopped refusing duplicate JSON keys

- Location: `crates/cli/src/knowledge.rs`, the first version read stdin into a
  `serde_json::Value`, removed `id`, then deserialized the rest.
- Claim: `Value` keeps the last of duplicate keys, so the
  `deny_unknown_fields` structs never saw the duplicate. Before the change
  `update`/`retire` refused `{"statement":"A","statement":"B",…}`; after it
  they wrote `"B"` with exit 0. A duplicate `id` also slipped past the new
  "JSON id must agree with the argument" check, and with no argument could
  retire a different fact than the first `id` named.
- Evidence: reproduced locally. `update f1` with a duplicated `statement`
  exited 0 and recorded `SECOND`; `add` with the same input still refuses
  (`duplicate field`).
- Fix: CLI-local `UpdateInput` / `RetireInput` structs with an optional `id`,
  `deny_unknown_fields`, parsed directly from stdin text and mapped into the
  engine's input. The test now sends a duplicate `statement` to `update` and a
  duplicate `id` to `retire` and expects `INVALID_INPUT`.

## Dropped candidates

None. The finder reported one candidate.

## Coverage

The finder ran against the HEAD binary and checked the following, which I
re-read in the code: JSON ids that are empty or path-like (the engine's
kebab-case id check refuses them); positional plus `--id` (a clap usage error,
exit 1); stdout bytes matching the envelope's `bytes` and `content`;
`--output -` creating no file while `--output ./-` still writes a file; file
mode without `--format` (`FORMAT_REQUIRED` before the ledger opens); file-mode
summary shape unchanged (no `content`); `chart list` rows tab-separated.
Residual: `"id": null` in the stdin JSON is now treated as absent. That is
harmless, because an id is then still required from the argument.
