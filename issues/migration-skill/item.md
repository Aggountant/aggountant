---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: done
priority: normal
closed: 2026-11-24
---

# Bundled skill for moving books from another program

## Description

`docs/user/fi/start/move-from-another-program.md` describes the whole
migration procedure: the inventory of what exists in the old program, the
read-only export, checksums of the retained archives, and the hand-off into a
new company home. No bundled skill carries this procedure, so the agent only
knows it if the user hands the page over. Every other recurring task the
agent performs has its own skill.

Decision (maintainer, 2026-11-24): the procedure becomes a bundled skill. The
user guide page then keeps what the user does and decides (what to collect,
signing in to the old service, what to check), and the agent's procedure
lives in the skill.

## Acceptance Criteria

- [x] A new skill under `skills/` carries the migration procedure, written by
      `skills/AGENTS.md` (Finnish, what the program cannot tell the agent,
      relying on `SOUL.md` instead of repeating it), and `aggountant skill
      list` / `skill install` include it.
- [x] The program-specific notes (Briox, Holvi) live where they serve the
      agent; the guide keeps what the user needs.
- [x] `docs/user/{fi,en}/start/move-from-another-program.md` describes the
      user's part and points the agent to the skill; both languages in step.
- [x] Skill and guide tests and `scripts/check-user-guide.sh` pass.

## Implementation Notes

- The skill is `migrate-books` (`skills/migrate-books/`), registered in
  `crates/cli/src/skill.rs` with `requires: ["aggountant-init"]`. Its body
  is the procedure and the reason for each step; `briox.md` and `holvi.md`
  are reference files opened for the program the company uses.
- The program notes are reference files of this skill and not `e-services`
  entries: an `e-services` entry needs a dated observation and a *Rajapinta*
  line, and these notes carry only the year 2026. If they are re-observed
  with dates, moving them is a small change.
- `aggountant-init` names `migrate-books` in prose at the opening balances
  and does not link to it. Every skill requires `aggountant-init`, so a link
  would install the migration skill with each of them.
- The guide page gained two user steps that were implicit before (deciding
  where the archive is kept, going through what is missing) and kept the
  collection table, the cutover statement rule and the limitations. Its
  program notes are cut to what the user acts on. `start/install.md` lists
  seven skills.
- Three sentences in the first draft of the skill said more than the guide
  had; they were removed or softened. No fact about either service is new.
- `result.md`, `review.md` and `assessment.md` are beside this file.

## Tests Run

- `cargo test -p aggountant skill`: the bundling unit tests (metadata,
  relative links resolve within the install, named commands exist) and
  `crates/cli/tests/skill_cli.rs` (seven skills listed and installed, the new
  skill's three files).
- `scripts/check-user-guide.sh`: language pairs, headings, links, the site
  build and the practice month.
- `scripts/ci-local.sh`: fmt, clippy, cargo-deny, build and test on stable
  and 1.89.

## Resolution

### 2026-11-24T05:53:43Z · @issuectl

The migration procedure is the bundled skill migrate-books; the guide page keeps the user's part. See result.md.
