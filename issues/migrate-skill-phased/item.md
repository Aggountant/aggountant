---
created: 2026-11-26
updated: 2026-11-26
type: feature
reporter: jari
status: open
priority: normal
lane: migrate
lane_seq: 10
collision: [skills/migrate-books, crates/cli/src/skill.rs]
---

# migrate skill: phased, per-domain migration (replaces migrate-books)

## Problem

Moving a company to aggountant is rarely one cutover. A company may move its
bookkeeping first and its Incomes Register reporting (pay data, travel
reimbursements, the employer's separate report) or its travel and expense
claims later, each on its own date. The `migrate-books` skill covers one
thing: collecting and archiving the old books and handing the cutover
balances to `aggountant-init`. It does not say how a later domain is moved
when the books are already in aggountant, although the product already has
pieces for it (`file-wage employer-report` imports months reported elsewhere
from transcript 630 and checks the employer-register start in YTJ).

## Expected

One skill, `migrate`, replaces `migrate-books`. It moves a company from
another program domain by domain, each with its own cutover date, in any
order the dependencies allow:

- **Bookkeeping**: what `migrate-books` does today (collect, verify, archive,
  cutover balances, open invoices, first bank statement after the cutover).
- **VAT**: filed returns and receipts to the archive; the first period
  aggountant files.
- **Incomes Register / employer**: the employee register, months reported
  elsewhere (transcript 630), filed reports and their identifiers kept for
  later corrections, payments made but not yet reported.
- **Travel and expense claims** (after ADR-11's slices exist): old claims are
  already vouchers with attachments and go to the archive with them; carried
  forward are open (approved, unpaid) claims at the cutover, unreported
  reimbursements, and each claimant's regular workplace and prior work at
  customer sites for the three-year rule.

The skill says which domains depend on which (for example claims and the
Incomes Register need the employee register), and records per domain what
was moved and from which date, so a later session can continue the next
domain. `aggountant skill install` keeps working for homes that installed
`migrate-books` (rename handled so old installs do not break silently), and
the user guide page on moving from another program follows.

## Acceptance criteria

- [ ] `skills/migrate/SKILL.md` replaces `skills/migrate-books/`; install,
      list and print handle the rename; tests updated.
- [ ] Domains as above, each with its own cutover date and dependencies.
- [ ] The travel-claim part is either written against ADR-11 or left as a
      clearly marked placeholder until its slices land.
- [ ] Guide (fi/en) `move-from-another-program` and `install` updated;
      CHANGELOG notes the rename.
- [ ] The travel-claim part is skipped entirely, with no question about claims, for a company that has none.

## Decisions

### 2026-11-26T02:47:35Z · @conductor

Jari 26.11.2026: the system supports phased migration (bookkeeping first, later Incomes Register / payroll and travel claims separately); the skill is just 'migrate'.

### 2026-11-26T06:00:21Z · @spinoff-01m8b6z13skk6403cxj7xcj1nh

Product owner 2026-11-26: companies that do not use expense claims must see no questions, changes or slowness from them (ADR-11 Consequences, 'Companies that do not use claims'). The acceptance criteria added today make this slice's share of that property testable.

## Comments

### 2026-11-26T02:54:02Z · @worker

What ADR-11 (rewritten 2026-11-26, section 10) expects from the travel-claim part of `migrate`:

- Old claims are not re-entered or resealed. They are already vouchers with their attachments and go to the archive with them. Where the old vouchers live in aggountant's own books, `expense-claim-legacy-migration` can attach them to a claim identity as they are; the skill uses that rather than building documents.
- Carried forward at the domain's cutover date: claims approved but unpaid, reimbursements paid but not yet reported to the Incomes Register, each claimant's regular workplace (as a confirmed, dated entry in the register of `claimant-workplace-register`, with its basis code; never inferred), and earlier work at customer sites the three-year rule needs (as earlier exposure and principal-site assessments in `three-year-site-clock`).
- An open claim that needs completing is entered through the claim commands as a draft, so the program computes and asks for what is missing; the skill keeps no calculation of its own.
- Moving one home's own claim file format is not a product command; the skill describes the generic path, and a home with its own format does its import on its side.
- Dependencies: the employee register (claimants), the claim slices above, and the Incomes Register domain for unreported reimbursements. Until the claim slices land, the travel-claim part stays a clearly marked placeholder, as this issue's acceptance criteria already allow.
