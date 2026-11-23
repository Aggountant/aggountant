---
created: 2026-11-23
updated: 2026-11-23
type: task
status: open
priority: normal
lane: unlaned
collision: [docs/user]
---

# User guide: open factual questions from the prose cleanup

## Description

In November 2026 every page of the user guide (`docs/user/`, 35 page pairs)
was rewritten for readable prose, one worker per page, following
`docs/user/AGENTS-STYLE.md`. The workers fixed the stale facts they could
verify and left the rest as questions. `attachments/open-questions.md` holds
them by page.

Most are factual questions a worker can settle against the code and `--help`
(for example whether `close-year` is presented as fully in use, whether the
installer replaces a binary first installed with `cargo install`, what
"Sulun aikana tehdyt kirjaukset säilyvät" refers to, whether the bank-import
preparation hash is always passed in the skill flow). A few are product
decisions for the maintainer (which platforms are supported, the response
time promised on the early-users page).

## Acceptance Criteria

- [ ] Each question in the attachment is answered: the page is corrected, or
      the question is closed as not needing a change.
- [ ] The guide gate (`scripts/check-user-guide.sh`) passes.

## Decisions

### 2026-11-23T18:43:37Z · @conductor

Product owner decisions (2026-11-23) on the maintainer questions: (1) Platforms: the guide says aggountant is developed and tested on macOS and Linux; Windows support could be added later but is not offered now, so do not send Windows users to cargo install as if it worked. (2) The early-users page keeps the maintainer's email address. (3) The page promises no response time at all; remove 'usually within a few days'. (4) Remove the meeting offer ('Sovitaan tapaaminen'); the page just says how to get in touch.
