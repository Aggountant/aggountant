# Assessment of the cli-production-friction review

Assessed against the fix commit that follows `4d17453`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `update`/`retire` stopped refusing duplicate JSON keys | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | FIX (confirmed regression of strict input on a write path) — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: reproduced with the binary. An agent that merges an old revision with
  its changes can emit a key twice. The old binary refused that input; the
  first version of this slice silently kept the last value. Fixed by parsing
  into CLI-local strict structs, with tests for both a duplicate field and a
  duplicate `id`.

No follow-up issue filed.

## Simplicity

- Reused: `read_stdin_json` (one stdin parse path and its `INVALID_INPUT`
  error), the engine's `UpdateCompanyKnowledge` / `RetireCompanyKnowledge` and
  `CompanyKnowledgeSource`, the existing `write_new` refuse-to-overwrite gate,
  and the `skill print` pattern in `main` for printing raw bytes in text mode.
- Repetition: the CLI input structs restate the engine's field lists once
  (`UpdateInput`, `RetireInput`). The alternatives were going through
  `serde_json::Value`, which loses duplicate-key refusal (F1), or putting a CLI
  `id` field into the process crate's input types, which would leak a
  CLI-argument concern into the engine. The id rule lives in one function,
  `resolve_id`.
- Left out: no `--format` default for file output (still required, so an
  existing call cannot silently change format), no new JSON shape for file
  mode, and no column alignment or `--at` filtering in `chart list` text.
