# Review: close-period answer placeholder and `a|b` help texts

- Skill: `/llm-review`, rigor **routine**, attention on the irreversible
  boundary: the change is the command an agent is told to run at the gate
  whose approval seals a period permanently.
- Scope: the slice commit (`crates/process/src/close_period.rs`,
  `crates/process/tests/close_period.rs`, `crates/cli/tests/close_period_cli.rs`,
  `crates/cli/src/main.rs` help texts, `docs/user/{en,fi}/tasks/close-month.md`),
  plus a read-only sweep of `crates/`, `skills/` and `docs/user/` for other
  command-shaped `a|b` choices.
- Finder: `gpt-6-astra`, one call (diff, `close_period.rs`, the issue).
  Verifier: not called. The one candidate was confirmed locally by reading the
  help text and the seal path, so a refutation call could not change the
  outcome. One model call in total.
- Risk class: guidance toward irreversible bookkeeping actions.

## Diff itself

No defects found. The quoted placeholder reaches `ProcessEngine::answer`
unchanged and is refused with `INVALID_ANSWER` before any `GateAnswered` event
or seal (`crates/process/src/lib.rs`, the `answer != "approve" && answer !=
"reject"` check). The new test
`close_period_content_is_shown_and_bound_by_start` asserts the exact
`next_command`, feeds the placeholder to `answer` and checks the period stays
`closing`.

## F1 (must-fix): `hunt-receipts answer --help` prints `--answer approve|discard|acknowledge`

- Location: `crates/cli/src/main.rs`, `HuntReceiptsCmd::Answer` doc comment.
- Claim: the help showed
  `--gate ApproveMatch:<txn>|Clarify:<txn> --answer approve|discard|acknowledge`.
  Copied into a shell with a real gate, `--answer approve|discard|acknowledge`
  runs `--answer approve` (which seals the matched voucher via
  `seal_match` → `ledger.seal`) and pipes into two failing commands.
- Evidence: the doc comment wrapped across two lines, which is why the slice's
  first single-line grep missed it; `hunt_receipts.rs` seals on `approve`.
- Fix: applied, the help names each gate's answers in prose
  (`--answer approve` or `--answer discard`, …).

## Dropped / coverage

- Dropped: none.
- Sweep result (finder and local grep, including wrapped doc comments): the
  other `a|b` matches are JSON value enumerations in `input_help.rs`, prose,
  error messages (`… accepts approve|reject, got …`) and already-quoted
  placeholders. No other emitted `next_command` contains an unquoted choice.
- Not covered: behaviour of the binary in an agent harness; only the strings
  and the refusal path were checked.
