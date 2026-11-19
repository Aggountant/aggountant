---
created: 2026-11-19
updated: 2026-11-19
type: improvement
status: open
priority: normal
lane: home-todo
collision: [skills/aggountant-init]
preparation: ready
---

# Company home TODO.md for pending items, open periods and deadlines

## Description

A company home has nowhere to keep what is waiting. `aggountant obligation` covers
only obligations raised by exceptional receipts, and an open one blocks
`close-period`. Everything else an agent and a user track between sessions has no
home:

- an invoice or receipt promised by someone else (a founder's registration fee paid
  from their own account, a sister company's rebilling invoice that waits for the
  company's e-invoice address);
- which months are still open and what each needs before closing (for example a
  bank statement fetched only up to the middle of the month);
- statutory deadlines (the VAT return of an annual VAT period, the end of the first
  fiscal year and the financial statements).

In real use an agent first put such items into `knowledge company` records, which is
wrong: knowledge holds facts and practices with sources, not pending work, and a
"pending" fact has to be retired later. Without an agreed place, the next session
either asks the user again or forgets.

Example (Esimerkki Oy, fictional): the share capital has been booked, but the
founder's registration fee and a 16,19 € domain invoice from a sibling company are
still to come, October cannot be closed before the fee is known, and the VAT return
for 1.11.–31.12. is due 28.2.

## Wanted

The same pattern as the home's `AGENTS.md`: a prose file the company owns and the
program does not parse.

- `aggountant-init` has the agent create `<home>/TODO.md` during onboarding (three
  sections are enough: waiting on others, period closing status, deadlines) and
  write the home's `AGENTS.md` so that every session reads it first and updates it in
  the same commit as the entry that resolves an item.
- `capture-receipt` (and `vero-fi` where a filing creates or clears a deadline)
  reads and updates it: an item is removed when its voucher is sealed or the matter
  is settled, and anything newly waiting is added instead of being stored as
  company knowledge.
- The skills say plainly what goes where: pending work in `TODO.md`, facts and
  practices in `knowledge company`, blocking obligations in `obligation`.
- The user guide (`set-up-company.md`, `company-home.md`, the monthly routine)
  describes the file in the same commit, fi and en.

Out of scope: a program command or a parsed format for the file.

## Acceptance Criteria

- [ ] `aggountant-init` asks for `TODO.md` and the home `AGENTS.md` pointer.
- [ ] `capture-receipt` and `vero-fi` read and maintain it, and say what does not belong in knowledge.
- [ ] User guide fi + en describe the file; the home table in `company-home.md` lists it.
- [ ] Skill bundle tests and docs checks pass.

## Tests Run

## Implementation Notes
