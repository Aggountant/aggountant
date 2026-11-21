# Review: cli-error-hints

- Skill: `/llm-review`, rigor **routine**: the change is read-only text output
  (an `explain` command and an added `hint` field); it writes nothing and
  decides nothing about the books.
- Scope: the working diff against `c8e5245` (`crates/cli/src/explain.rs`,
  `crates/cli/src/main.rs`, `crates/cli/tests/explain_cli.rs`, both
  languages' `reference/error-codes.md` and `reference/commands.md`,
  `docs/user/AGENTS.md`).
- Finder: `gpt-6-astra`, one call with the diff. Verifier: not called; the
  finder returned no candidates and the surface is not high-risk. One model
  call in total.
- Risk class: agent-facing output; backward compatibility of the error JSON.

## Findings

No findings. The finder reported that it checked all 49 entries in both
languages against the guide cells and the link mapping, and that the existing
error fields are unchanged.

## Local checks

- `error_code`, `error_kind` and `message` are emitted exactly as before;
  `hint` is an added key, present only for codes the guide's table lists
  (`error_without_a_guide_entry_has_no_hint`).
- The hint is the guide's text, read from the bundled page, so it cannot drift
  from the guide; unit tests hold the table format, the fi/en code lists and
  the codes' existence in crate sources.
- `scripts/ci-local.sh` passed (fmt, clippy, deny, stable and MSRV tests, the
  user-guide gate including the site build and the practice month).

Dropped candidates: none. Residual risk: a future guide edit that breaks the
table format fails the unit tests rather than silently dropping hints.
