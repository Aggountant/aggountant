# Result: cli-error-hints

The program now explains an error code when it arrives:

```
aggountant explain PERIOD_SEALED [--lang fi|en] [--json]
aggountant explain            # lists the codes the guide explains
```

and every error JSON whose `error_code` the guide's table lists carries a
`hint`, the table's "Mitä tehdään" cell:

```json
{"error_code":"PERIOD_SEALED","error_kind":"user","message":"…","hint":"kirjaa seuraavalle avoimelle kaudelle"}
```

`error_code`, `error_kind` and `message` are unchanged; `hint` is an added key,
absent for codes without a guide entry. An unknown code is
`ERROR_CODE_UNKNOWN` (exit 1); a `VERO_…` code points to the Vero.fi blocker
page.

## One source

`crates/cli/src/explain.rs` bundles `docs/user/{fi,en}/reference/error-codes.md`
with `include_str!` and parses the table, so the text the program prints is the
guide's text and cannot drift. Relative links become site URLs
(`https://aggountant.dev/…`, English under `/en/`). Unit tests fail if a row
stops parsing, if the two languages list different codes or in a different
order, or if the guide documents a code no crate source names.
`docs/user/AGENTS.md` records that the table format is now a contract.

## Language of the hint: Finnish

The hint uses the Finnish guide. The Finnish pages are the original (the
guide is written in Finnish first, English follows), the skills whose code
explanations the hint replaces are Finnish, and the agent relays the advice to
a Finnish-speaking owner, so it needs no translation step. The rest of the
error JSON stays English, as before; `explain --lang en` gives the English
entry for anyone who needs it.

## Next

The skill rebuild can drop its per-code explanations and tell the agent to read
`hint` or run `aggountant explain <CODE>`. `skills/` was not edited here.

## Verified

- `scripts/ci-local.sh`: all gates passed (including the user-guide gate and
  the practice month).
- Tests: `crates/cli/src/explain.rs` (5 unit tests),
  `crates/cli/tests/explain_cli.rs` (4 CLI tests, including a real
  `ALREADY_INITIALIZED` error carrying the hint).
- Review: `review.md` (no findings), `assessment.md`.
