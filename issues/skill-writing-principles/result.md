# Result: skill-writing-principles

`skills/AGENTS.md` has a new section, "What goes into a skill", ahead of the
bundling and e-services sections, which are unchanged. The packaging tests in
`crates/cli/src/skill.rs` now resolve the command paths the skills name
against the binary's command tree.

## Words

| | Before | After |
|---|---:|---:|
| `skills/AGENTS.md` | 478 | 1 204 |
| of which the new section | 0 | 726 |

The file grew because the principles did not exist. The section is prose with
reasons, six paragraphs under bold leads, not a checklist.

## Where the issue's ten points went

| Wanted | In the section |
|---|---|
| A skill carries what the program cannot give | opening paragraph; "The program describes itself" |
| Wanting a format or a state list means fix the program | "The program describes itself" |
| One fact, one home; house rules in SOUL | "One fact, one home" |
| SOUL holds only what applies to every task | "One fact, one home" |
| Rarely needed material into a reference file | "The body is what the task needs every time" |
| Size is a signal, about a thousand words | same paragraph |
| A boundary once, with its reason | "A boundary is stated once, with its reason" |
| Only what works; unconnected paths to `reference/status.md` | "Only what works, and only how it is now" |
| No history, no implementation notes; dated observations stay | same paragraph |
| A slice that changes a command checks the skills | "A changed command takes its skills along" |

Added beyond the list, each from the review of the first draft: a needed
format stays in the skill until the program shows it when the fix is outside
the slice's files; a task skill may name the point where a house rule applies;
a fact two task skills share lives in one and is linked from the other.

Left out on purpose: the review's own wording and its measurements of the
individual skills (only the one observation that explains why the rules exist
is kept, dated); any restatement of the bundling rules already below it; the
stop-when-out-of-scope rule, which the root `CLAUDE.md` owns.

## The test

`named_commands_exist_in_the_command_tree` reads every bundled file and
reports a command path the clap tree does not have. A path is read

- after the word `aggountant` in an inline code span or a fenced block, past
  global options, across `\`-continued lines, for every occurrence on a line;
- in an inline code span that starts with two bare words (`voucher seal`).

The second rule is wider than the issue's `aggountant <command> [<sub>]`. It is
there because the defect that motivated the check was written `journal show`,
without the program name, as most command mentions in the skills are. Its cost
is a convention, stated in `skills/AGENTS.md` and in the failure message: a
command path is written from the top-level command down, and an inline span
naming another program's two-word command (`git status`) would be reported.
No bundled file has such a span today.

`made_up_and_partial_command_paths_are_reported` holds the made-up commands
(`journal show`, `aggountant file-vat tila`, `file_vat`, a bare
`employer-report missing`, a path split over lines) and a passing half
(`--help` forms, `help` subcommands, placeholders, flag-led spans, comments,
JSON). The test checks command paths only; flags, field names and error codes
are not checked. `skill-cli-contract-tests` is the open issue for running the
skills' examples.

All bundled skills pass as they stand; no skill file was changed.

## Review

`review.md` and `assessment.md` beside this file. 16 findings: 13 fixed in the
slice (five in the test, eight in the text), 3 dropped with reasons. No
finding was spun off.

## Simplicity

Reused: the existing test module and `SKILLS` table, and clap's own command
tree through `CommandFactory`, so there is no second list of commands. Left
out: a check of flags or fields (would need to parse arguments per command; it
is the contract-test issue's work), an allow-list or escape syntax for the
two-word rule (no case needs one), and any runtime surface: the check is test
code only.

## Product gaps

None new. Writing the principles used the gaps the skills review had already
named (INPUT sections for stdin commands, messages that carry what skills now
explain); they belong to the rebuild's binary work and are not refiled here.
