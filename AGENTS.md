# aggountant

AI-first toolkit for Finnish bookkeeping: kirjanpito, ALV-raportointi and
tilinpäätös. It ships agent skills plus a CLI that the skills call into: files
as the database, driven by agents. The Rust workspace is in `crates/`; the
project's own skills are authored in `skills/`, bundled into the binary and
installed with `aggountant skill install`. `docs/user/` is the user guide.

Companies keep their live books with this code: it seals vouchers, closes
periods and files VAT returns and earnings reports with the Finnish
authorities. Treat it as production software for real bookkeeping, not as a
prototype. Silently wrong bookkeeping is the worst failure it can have.

## Product invariants

The rules a change must not break are written next to the code they govern, in
`crates/AGENTS.md` and each crate's own `AGENTS.md`. Read them before changing a
crate. The ones that shape almost every change:

- `ledger` is the only path that changes the books; nothing else writes the
  fiscal store.
- A sealed voucher is never edited. A mistake is corrected by a new voucher
  that refers to it.
- A filing whose outcome is uncertain is never resent automatically. Its
  outcome is found out first, because a second submission to an authority
  cannot be taken back.
- VAT treatment is decided when a voucher is sealed and is never inferred from
  an account number afterwards.

## Code is the record

The code is the one description of what aggountant does. Besides code, the
repository keeps only what has an owner who notices when it goes stale:

- **instructions**: the `AGENTS.md` files, including the rules the code cannot
  show;
- **work that is not code yet**: open issues, and ADRs and plans whose
  implementation has not landed;
- **the user guide** in `docs/user/`, which changes in the same commit as the
  command it describes.

Everything else is working material: design and options pages, research,
benchmarks and architecture descriptions. A second description of the system
drifts from the code within weeks, and a stale one is worse than none because a
reader cannot tell which parts still hold. So once the code carries a
decision, move any rule the code cannot show into the nearest `AGENTS.md` and
remove the page in the same change. Its intent stays reachable through git
history by path and through the `Refs-Issue` trailer of the commits that
implemented it. `docs/AGENTS.md` owns the structure of `docs/` and the ADR life
cycle.

## Publication surface

Everything in this repository is public: code, fixtures, issues and their
slugs, plans and attachments under `issues/`, commit messages, branch names and
pull requests. None of it may contain a real company's data: no figures,
customer or counterparty names, account numbers, business IDs, personal data or
credentials, and no machine hostnames or paths from anyone's home directory.

Evidence is synthetic. A defect found in real bookkeeping becomes an issue, a
test or a fixture only after it has been reproduced with a made-up company and
made-up amounts (the user guide's Esimerkki Oy, Business ID 1234567-1, and
fictional IBANs are the convention). If a real case cannot be reduced that way,
describe the shape of the problem, not the data.

## Building it

One change is one issue and, for agents, one worktree. Open an issue before
building, because plans, analyses and designs live under `issues/<slug>/`
(`plan.md`, `analysis.md`, `design.md`) and nowhere else. Issue mechanics are in
the `/issue` skill, `issues/AGENTS.md` and `.issuectl/AGENTS.md`; outside
contributors report through GitHub issues as `CONTRIBUTING.md` describes.
Commits carry `Refs-Issue: @<slug>` or `Fixes-Issue: @<slug>`. A slice is
reviewed after it lands, so it carries its own review: the review report and
its assessment go beside the issue before the change merges.

**Simplicity.** Agent-written code tends to add generic abstractions,
compatibility layers and features for a future that has not arrived, and to
state the same business rule, validation, serialisation, storage path or CLI
conversion in several places. This project wants one clear current path. Before
a plan or a slice is accepted, look at what existing interface or data it
reuses, what repetition it removes or why the remaining repetition is
justified, and what abstraction or future feature it deliberately leaves out.
Look for the same rule expressed in several places and merge them when that
makes the whole smaller; similar syntax alone is not a reason to abstract.
Write the outcome down, in the plan for a plan and in the review or assessment
for an implementation, so the person judging the landed slice can see the
question was asked.

**CLI surface.** The AI-first CLI canon is binding for every command: strict
input validation, `--json` output, JSONL logs, no interactive prompts,
informative errors, meaningful exit codes, composable commands. It is installed
as the `ai-first-cli-canon` skill (`.claude/skills/`, `.codex/prompts/`). Those
directories hold copies installed and owned by their tools (`project-canon`,
`issuectl`); do not edit them by hand, since a reinstall overwrites the edit.

**Gates.** `scripts/ci-local.sh` runs fmt, clippy, cargo-deny, and build plus
test on both stable and the MSRV (1.89). GitHub Actions runs the same checks on
pull requests and on pushes to `main`, but run the script yourself before you
push: a red `main` blocks everyone who branches from it.

**Single user, v1.** aggountant is one trusted user or agent on one machine,
one data home per company (decided 2026-08-06; `crates/AGENTS.md`).
A review finding that only matters under several users, concurrent processes or
a hostile trust boundary is correct as the code stands. Record it in
`issues/multi-user-considerations` with a one-line source reference (which
review, slice, date) instead of fixing it; the register describes its own entry
format. Data integrity is not a multi-user concern: silently wrong bookkeeping
is always real.

## Scheduling: lanes and worktrees

`issuectl dag` is the work queue: lanes, blocking, pick order. Its structure
(lane order, collision edges, `blocked_by`) is stateful, so it is
merge-updated, never regenerated from scratch.

A lane is a serial queue and only its head is spawnable, so the number of lanes
is the parallelism budget. Lane boundaries follow hot-file conflicts, not
themes: two issues in different lanes must be mergeable independently, and a
theme lane ("all ALV work") is right only when the issues really touch the same
files. Overlap across lanes is expressed with a `collision:` token, a shared
string (typically a path) that keeps the two from running at once. A new lane
is cheap; a wrong lane boundary costs merge conflicts. When one file collects
issues and forces a deep lane, splitting the file is scheduling work, not
cosmetics: in August 2026 splitting a 5 000-line VAT module into submodules
turned a nine-deep serial lane into three parallel ones. Whoever splits a file
records which issue goes to which module, and lanes and collision tokens are
re-mapped afterwards, because a token naming the old file no longer protects
anything.

The `parkki` lane holds real findings that are never scheduled, such as the
multi-user register: in the DAG so they stop surfacing in triage, never picked.
`issuectl dag` has no notion of a parked lane and still reports its head as
spawnable, so leaving it out is up to whoever picks.

Parallel workers are safe only because their file sets do not overlap. A
worker brief therefore names the files the worker may touch and gives explicit
permission to stop. If the safe change does not fit inside that set, the worker
records why and stops without code changes rather than widening its scope or
landing half a change; whoever launched it relaunches with the right boundary.

## Triage of unlaned issues

Review skills over-produce polish findings once the real work is done. Let them
write freely and filter here, at triage, so nothing is lost to self-censorship
when the finding is written. The filter is user impact on real data. Close
`wontfix` when an issue describes a once-in-a-blue-moon window whose end result
is still correct, hardening against hostile input in a local tool with one
trusted user, or an edge case that no source we actually use produces. The issue
usually admits this itself ("no direct user impact", "low real-world
likelihood", "forward-looking hardening, not a current bug"); that line is the
closing reason, so put it in the close comment and the decision stays
reversible. Multi-user findings go to the register instead, and data integrity
is never filtered out.

## Git and layout

Work on `main` in small logical commits and integrate with a rebase onto the
current `main`. Never force-push `main`: other checkouts and workers branch
from it, and rewritten history strands them. If a rebase conflict cannot be
resolved confidently, stop and report it rather than forcing. Outside
contributors use pull requests as `CONTRIBUTING.md` describes.

`history/` is gitignored agent scratch. Directories that carry agent guidance
have an `AGENTS.md` with `CLAUDE.md` symlinked to it, and `AGENTS-<TOPIC>.md`
for a topic that outgrows it; durable knowledge goes in the most specific one.
