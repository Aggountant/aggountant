# Review: skill-writing-principles (skills/AGENTS.md principles and the command-path test)

- Target: commit `b0628fd` (`git diff 1acae34..b0628fd`): `skills/AGENTS.md` section "What goes into a skill", and the tests added to `crates/cli/src/skill.rs`.
- Instruction review (`/llm-skill-review`): gpt-6-astra, lenses executor and design-fidelity (design = the issue's "Wanted"), 2 calls.
- Code review (`/llm-review`): finder claude-fable-5-2, 1 call, rigor routine, risk class low (test-only code). No verifier call: each surviving candidate was settled locally by adding its input to the test, which failed before the fix and passes after.
- Line numbers refer to `b0628fd`.

## Code findings (crates/cli/src/skill.rs)

### C1. should-fix: a command on a continued shell line is not checked
- Location: `crates/cli/src/skill.rs:401`
- Claim: fenced lines are checked one by one, so `aggountant --json \` + newline + `journal show` passes.
- Evidence: reproduced by the finder; local reading confirms; the skills have continued invocations.
- Fix: join `\`-continued lines before checking.

### C2. should-fix: only the first `aggountant` on a line is checked
- Location: `crates/cli/src/skill.rs:381`
- Claim: `aggountant init && aggountant journal show` passes.
- Evidence: `position` returns the first match.
- Fix: check every occurrence.

### C3. should-fix: a misspelled word in subcommand position ends the walk silently
- Location: `crates/cli/src/skill.rs:361`
- Claim: `aggountant file_vat status` and `aggountant Journal show` pass, because a token that is not lowercase-kebab is treated as the start of arguments.
- Evidence: `is_command_word` rejects `_` and capitals; snake_case field names sit beside kebab-case commands in the skills.
- Fix: under a command with subcommands, any token that starts with a letter or digit must be a subcommand.

### C4. should-fix: `aggountant help <command>` is reported as nonexistent
- Location: `crates/cli/src/skill.rs:417`
- Claim: the unbuilt clap tree lacks the generated `help` subcommand.
- Evidence: clap adds it in `build()`.
- Fix: build the tree in the tests.

### C5. should-fix: non-command text is reported as a made-up command
- Location: `crates/cli/src/skill.rs:393`, `:401`
- Claim: an inline span such as `git status`, or a `#` comment line in a fenced block that mentions aggountant, fails the build, and the message gives the wrong remedy.
- Evidence: by construction of the two-bare-words rule.
- Fix: skip comment lines; say in the failure message that a two-bare-word inline span is read as a command path. No cheap discriminator keeps `journal show` caught while passing `git status`.

## Instruction findings (skills/AGENTS.md)

### S1. fix-or-file leaves the agent without a needed format
- Location: `skills/AGENTS.md:26` "Fix that in the slice, or file it as an issue; the copy in the skill is not the fix."
- Claim: filing alone removes the format from both places; the design says "fix the program".

### S2. flags and fields: forbidden or sometimes needed
- Location: `skills/AGENTS.md:19`, `:62`
- Claim: one sentence sends flags and fields to `--help`, another expects the writer to verify the ones a skill names.

### S3. a fact two task skills share has no stated home
- Location: `skills/AGENTS.md:31`

### S4. "Setup … goes into a reference file" read literally empties aggountant-init; the design's "rarely needed material" became "not monthly"
- Location: `skills/AGENTS.md:40`

### S5. thousand-word rule against a bounded slice
- Location: `skills/AGENTS.md:42`
- Claim: a worker adding a paragraph to an oversized skill may not be allowed to split it.

### S6. "stated once": per skill or across skills
- Location: `skills/AGENTS.md:46`
- Claim: unclear whether a task skill may name the point where SOUL's approval rule applies.

### S7. history with present consequences
- Location: `skills/AGENTS.md:52`
- Claim: upgrade knowledge about data written by older versions might be read as banned history.

### S8. which status page
- Location: `skills/AGENTS.md:50`
- Claim: the guide has `status.md` in two languages.

### S9. boundaries narrowed to two categories
- Location: `skills/AGENTS.md:46` "The real limits here are few: what cannot be undone, and what the user approves."

### S10. "updates the skills" where the design says "checks"
- Location: `skills/AGENTS.md:56`

### S11. intentional drift: the documented test contract is wider than the issue (unprefixed two-word spans) and states its limit (flags, fields, codes unchecked)
- Location: `skills/AGENTS.md:59`

## Dropped candidates (1)
- `brew install aggountant jq` in a fenced block read as a command: no skill installs the program this way; speculative.

## Coverage
Read: the whole of `skill.rs`, the `Cli` declaration, all bundled skill files. Run: the skill tests against the real command tree. Not checked: flags, fields and error codes named in skills (out of the test's scope by design).
