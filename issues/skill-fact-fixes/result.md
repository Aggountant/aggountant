# Result: skill-fact-fixes

The four wrong facts are fixed.

1. `vero-fi` now points to `run show --run-id` for a rejected Incomes
   Register report; there is no `journal` command.
2. The consult route is gone from the skills. `capture-receipt` tells the
   agent to ask the user (or their bookkeeper) about an exceptional receipt and
   note the open question in the home's `TODO.md`; it and `aggountant-init`
   say receipt work creates no obligations. `SOUL.md` says `knowledge
   list`/`show` belong to the consult route, which is not in use. The route
   itself stays described only in `reference/status.md`.
3. `aggountant skill print <name> --file <FILE>` prints any file the skill's
   install writes, named as the skill's text links it: `SOUL.md` for
   `aggountant-init`, or `../aggountant-init/SOUL.md` from any skill that
   requires it. An unknown file fails with `SKILL_FILE_NOT_FOUND` and lists
   what is available. Text output is the file's bytes; `--json` adds `file`
   and the real `path_in_repo`. `skill list --json` adds `files` and
   `requires` per skill. Every skill's "Lue ensin SOUL.md" line names
   `aggountant skill print aggountant-init --file SOUL.md`. The install guide
   (fi, en) shows the command.
4. The seven codes are in `docs/user/{fi,en}/reference/error-codes.md`, in
   alphabetical order; the two `voucher validate` warnings are marked as
   warnings, and `VERO_VAT_PERIOD_NOT_FILEABLE` says it shows in the message
   and in `run show`, not as `error_code`.

Existing inputs and outputs are unchanged: `skill print <name>` without
`--file` prints `SKILL.md` as before, and the list fields are additive.

Tests: `crates/cli/tests/skill_cli.rs` (`--file` own and required files,
refusals, list fields); the unit link test now uses the same resolver as
`--file`. `scripts/ci-local.sh` passes.

Review and assessment: `review.md`, `assessment.md` (two text findings,
both fixed).
