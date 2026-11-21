# Assessment: skill-writing-principles

Source: `history/review-skill-writing-principles.md` at `b0628fd`. All FIX rows were applied in the same slice.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | C1. should-fix: a command on a continued shell line is not checked [^F1] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F2 | C2. should-fix: only the first `aggountant` on a line is checked [^F2] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F3 | C3. should-fix: a misspelled word in subcommand position ends the walk silently [^F3] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F4 | C4. should-fix: `aggountant help <command>` is reported as nonexistent [^F4] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | LOW | FIX |
| F5 | C5. should-fix: non-command text is reported as a made-up command [^F5] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F6 | S1. fix-or-file leaves the agent without a needed format [^F6] | CONFIRMED | REGULAR | NEUTRAL | NONE | HIGH | FIX |
| F7 | S2. flags and fields: forbidden or sometimes needed [^F7] | CONFIRMED | REGULAR | NEUTRAL | NONE | MED | FIX |
| F8 | S3. a fact two task skills share has no stated home [^F8] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F9 | S4. "Setup … goes into a reference file" read literally empties aggountant-init; the design's "rarely needed material" became "not monthly" [^F9] | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX |
| F10 | S5. thousand-word rule against a bounded slice [^F10] | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b) |
| F11 | S6. "stated once": per skill or across skills [^F11] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F12 | S7. history with present consequences [^F12] | INCORRECT | - | - | - | MED | DROP (Rule 1a) |
| F13 | S8. which status page [^F13] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F14 | S9. boundaries narrowed to two categories [^F14] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F15 | S10. "updates the skills" where the design says "checks" [^F15] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F16 | S11. intentional drift: the documented test contract is wider than the issue (unprefixed two-word spans) and states its limit (flags, fields, codes unchecked) [^F16] | INCORRECT | - | - | - | HIGH | DROP (Rule 1a) |

FIX: 13   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 3

## Evidence

- **F1** (crates/cli/src/skill.rs, unknown_command_paths): Input `aggountant --json --home /tmp/esimerkki \` + `file_vat status` added to the test; it was not reported before joining continued lines and is reported after.
- **F2** (crates/cli/src/skill.rs, unknown_command_paths): `aggountant init && aggountant Journal show` added to the test; reported only once every occurrence is walked.
- **F3** (crates/cli/src/skill.rs, first_unknown_command): `file_vat` and `Journal` are now reported; the loosened walk passes on all bundled skill files against the real command tree.
- **F4** (crates/cli/src/skill.rs, command_tree): `aggountant help file-vat` and `aggountant file-vat help status` are in the passing half of the test and pass only with the built tree. No skill uses the form today, hence LOW.
- **F5** (crates/cli/src/skill.rs, unknown_command_paths): By construction of the two-bare-words rule. Comment lines are skipped (test case added); the failure message and skills/AGENTS.md now state the rule. The rule itself stays: without it the motivating defect, an unprefixed `journal show`, is not caught.
- **F6** (skills/AGENTS.md, The program describes itself): Quoted text allowed filing alone. Now: fix in the slice; when outside the slice's files, file it and keep what the agent cannot work without until the program shows it.
- **F7** (skills/AGENTS.md, The program describes itself): The two sentences did pull apart. Now a skill names a command, or a flag or field a consequence hangs on, to say when and why, not to document it.
- **F8** (skills/AGENTS.md, One fact, one home): No home was stated. One sentence added: it lives in one skill and the other links to it (the bundling section already allows links to required skills).
- **F9** (skills/AGENTS.md, The body is what the task needs every time): aggountant-init's task is setup; the heading and sentence now speak of what a task needs rarely, as the issue does.
- **F10** (skills/AGENTS.md, The body is what the task needs every time): The root CLAUDE.md already tells a worker whose safe change does not fit its file set to record why and stop; the rebuild slices bring the existing skills under the mark. Repeating the stop rule here adds text for a case the repository already answers.
- **F11** (skills/AGENTS.md, One fact, one home): Added: task skills name the point in their own task where a house rule or an approval applies.
- **F12** (skills/AGENTS.md, Only what works, and only how it is now): The sentence gives its own criterion, "give the agent nothing to act on"; knowledge that decides a present action is not covered by it.
- **F13** (skills/AGENTS.md, Only what works, and only how it is now): docs/user/fi/reference/status.md and docs/user/en/reference/status.md both exist; text now says both languages.
- **F14** (skills/AGENTS.md, A boundary is stated once): The colon made the two categories exhaustive; now "such as".
- **F15** (skills/AGENTS.md, A changed command takes its skills along): Issue says "checks"; text now says checks, and updates what no longer holds.
- **F16** (skills/AGENTS.md, A changed command takes its skills along): Not a defect: the reviewer itself labels it intentional drift. The wider rule is what catches the issue's own example, which was written without the `aggountant` prefix; result.md records it.

[^F1]: review line 10, claude-fable-5-2
[^F2]: review line 16, claude-fable-5-2
[^F3]: review line 22, claude-fable-5-2
[^F4]: review line 28, claude-fable-5-2
[^F5]: review line 34, claude-fable-5-2
[^F6]: review line 42, gpt-6-astra
[^F7]: review line 46, gpt-6-astra
[^F8]: review line 50, gpt-6-astra
[^F9]: review line 53, gpt-6-astra
[^F10]: review line 56, gpt-6-astra
[^F11]: review line 60, gpt-6-astra
[^F12]: review line 64, gpt-6-astra
[^F13]: review line 68, gpt-6-astra
[^F14]: review line 72, gpt-6-astra
[^F15]: review line 75, gpt-6-astra
[^F16]: review line 78, gpt-6-astra
