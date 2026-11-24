# ADR-11 — Expense claims: trips and lines in a claim, approved into sealed vouchers, state derived from facts

- **Status:** Proposed (2026-11-24)
- **Date:** 2026-11-24
- **Authors:** technical-decision worker for issue `expense-claim-model`
  (taskfleet run `01m87gzzt5thzm0qb8xrqabanq`), from the product owner's
  decisions of 2026-11-24 and four research reports kept outside this
  repository (content and regulation, storage, calculation, workflow, plus
  one on the regular workplace). Reviewed with `/llm-review`; the review and
  its assessment are in `issues/expense-claim-model/`. Decider: Jari
  Mustonen.
- **Refines:** [ADR-5](adr-5-seal-timing) (the claim draft is a kind of
  intake: approval seals at once, must-seal blocks the period close) and
  [ADR-4](adr-4-cost-centre) (a rebilling reference reaches the postings as
  an opaque code). Bound by `crates/AGENTS.md`: the Domain computes and
  refuses, a skill keeps no copy of a calculation or a state machine, and a
  Run exists only for the program's own irreversible outside act. Builds on
  the open issues `wage-report-from-claim`, `rebilling-reference`,
  `travel-report-statutory-deadline` and `employee-pii-entry`.

---

## Context

"Kululasku" (expense claim) is not a legal term. The law knows three money
flows that a claim usually bundles, and treats each differently in income
tax, VAT, the Incomes Register (Tulorekisteri) and the books:

- **scheduled tax-free travel allowances**: mileage allowance
  (kilometrikorvaus), domestic and foreign per diem (päiväraha), meal
  allowance (ateriakorvaus) and night travel allowance (yömatkaraha), tax-free
  up to the Tax Administration's yearly maximums ([TVL 71–73 §](https://www.finlex.fi/fi/lainsaadanto/1992/1535),
  decision [970/2025](https://www.finlex.fi/api/media/statute/728416/mainPdf/main.pdf)
  for 2026 and [VH/5639/00.01.00/2026](https://www.vero.fi/syventavat-vero-ohjeet/paatokset/47405/)
  for 2027). They carry no deductible VAT, and mileage and per diem are
  reported to the Incomes Register (income types 311 and 331);
- **reimbursements of the company's own costs** that the person paid
  (hotel, tickets, parking, a tool), tax-free against a receipt, VAT
  deductible under the general rules, not reported;
- **taxable reimbursements**, which are pay and need withholding.

The travel claim (matkalasku) has a statutory content
([ennakkoperintäasetus 1124/1996, 17 § 2–4 mom](https://www.finlex.fi/fi/lainsaadanto/1996/1124)):
purpose, destination, route where needed, means of travel, start and end
times, for foreign trips the country where each travel day ended, kilometres
and unit prices for one's own vehicle, and the receipts or other evidence
attached. For a shareholder the stakes are higher than for an employee: the
Tax Administration's guidance on hidden dividends says mileage allowances that
cannot be shown to rest on real drives are always a hidden dividend
([Peitelty osinko, VH/3393/00.01.00/2025](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48147/peitelty-osinko11/)).

Today aggountant has the pieces around a claim but not the claim.
`capture-receipt` books a receipt the owner paid against `2740`; `file-wage`
files a travel report from hand-assembled lines; `voucher seal` can seal a
payable voucher with one attachment. A production home keeps its claims as
files: one file per line and a monthly human-readable summary, approved at
the month's bookkeeping, sealed as a payable voucher with the summary as its
attachment, paid from the bank and reported by hand. It found the right
concepts, and it shows where a file-only claim breaks: the summary was
edited after approval (payment and report references were written into it),
so the file the agent reads is no longer the one that was sealed; the line
files are not sealed at all; and the calculation and the states live in the
home's agent instructions, which `crates/AGENTS.md` forbids.

The product owner decided on 2026-11-24 (binding here):

- A claim is a container of one or more trips and lines, **not tied to a
  month**; monthly and per-trip claims both work. Claim, trip and lines are
  enough; the agent builds the claim the user wants from them.
- The company's own purchases are not gathered into the claim. The one
  exception is the trip's record of free meals and company-paid meals,
  because they change the per diem.
- The record format is this ADR's to decide, with one constraint: the
  program computes from the trip, so every field used in a computation is
  typed and validated by the program; free explanation may be text.
- Lifecycle steps may come from an outside travel-claim system and are
  imported as facts; some steps are internal and silent, not user steps.
- In scope from the first version: claimants who are not employees (board
  member, unpaid owner), rebilling of receipt costs **and** scheduled
  allowances, the three-year rule, and the foreign per diem table by country.
  Finland only; Finland-specific rules kept apart so another country's module
  can come later, without generalising now. Sole traders are out.
- The Incomes Register deadline shown is the statutory one (issue
  `travel-report-statutory-deadline`).
- The approver's identity is recorded. Self-approval is allowed (the
  claimant says "hyväksyn"); a second approver is possible; no heavy
  multi-stage approval.
- Exceeding a scheduled maximum is refused and explained, until the product
  has payroll.
- Evidence: the recording time is stored, the statutory travel-claim fields
  are mandatory, evidence can be attached to a drive, and the program runs
  plausibility checks.
- Regular workplace (varsinainen työpaikka): a dated entry in the personal
  data store plus a snapshot sealed into each trip; no tax-free travel
  allowance without a valid entry; asked when unknown or when there is reason
  to think it changed, not as a yearly routine. The product uses the legal
  words with a plain explanation in parentheses.

What is left to decide is how this is modelled, stored, computed and
processed, and how it connects to vouchers, bank import, the Incomes Register
and rebilling.

## Decision

**A claim is a program-owned draft of typed trips and lines, written only
through the CLI and computed by a pure Finnish travel module in the Domain.
One content-bound command approves it and, in the same transaction, writes
the frozen claim document to the artifact store and seals the vouchers that
carry its hash. After approval nothing is written into the claim: payment is
matched in the bank import, the Incomes Register report goes through the
existing `file-wage` Run, and the claim's state, deadlines and next command
are derived every time from those facts.** There is no long-lived claim Run.

Command names below are indicative; the slices settle them.

### 1. Concepts and their boundaries

- **Claim (kululasku).** One claimant, any number of trips and lines, an
  opaque `claim_id`. The claim has no period of its own; the accounting
  period of each part comes from its own dates (section 8). A monthly claim
  is just a claim whose content falls in one month.
- **Claimant (hakija).** A person in the existing personal-data register
  (`employee_id`), employee or not. The register gains a dated
  **relationship** fact: employee, managing director, board member, unpaid
  shareholder, other recipient of a personal fee
  ([EPL 13 §](https://www.finlex.fi/fi/lainsaadanto/1996/1118)); and a dated
  **part-owner** fact for the Incomes Register's "osakasasemassa maksajaan"
  field, never inferred. A contractor or consultant is refused: their costs
  belong on their own invoice and are never tax-free reimbursements (ohje
  10.5). A tax card is not required for a travel-only claimant
  (`employee-pii-entry`).
- **Trip (matka).** The facts the law and the computation need: purpose,
  destinations (with distance by shortest route and its source), route where
  needed, means of travel, departure and return as instants, where the trip
  started (home, regular workplace, lodging kept for work, other), the
  location at each travel-day boundary for foreign trips, the free meals and
  the company-paid meals per travel day, whether the employer provided
  lodging, night work or a night flight, for a trip without per diem whether
  the work kept the claimant from eating at the usual place and whether an
  employer lunch benefit was usable near the work site (decision 14 §,
  ohje 8.6), the drives with vehicle, kilometres and the surcharges claimed,
  an optional rebilling reference, and a free-text note. The trip is where
  company-paid items enter the claim: a company-paid meal is a trip fact
  (amount, optionally the voucher that booked it), not a line.
- **Lines.** Two kinds:
  - *scheduled allowance lines* (mileage, per diem, meal allowance, night
    travel allowance) are **computed** from trips, never typed in. The draft
    may record a lower amount the company chooses to pay (the tax-free part is
    what was paid, ohje 8.1.2); a higher one is refused;
  - *receipt lines*: a cost the claimant paid, with the receipt artifact,
    date, gross amount, merchant, business purpose, receipt kind (VAT-itemised
    receipt, simplified invoice, card slip only, no receipt with the
    claimant's statement), country, an optional trip, an optional rebilling
    reference. Goods bought on a trip that are not travel costs are receipt
    lines without a trip.
- **Finland is a module, not a parameter.** The claim, receipt lines,
  approval, links and status are country-neutral. The trip carries
  `jurisdiction: FI`, and everything Finnish (the workplace roles, the travel
  day, the meal halving, the rate table, the 311/331 mapping) is in
  `domain::travel::fi` and the Finnish trip section. Another value refuses.
  No trait or plug-in layer is built now; when the first other country comes,
  the split is made along this line.

### 2. Where each kind of data lives, and what the seal covers

| Data | Where | In git | Erasable | Why |
|---|---|---|---|---|
| Draft: trips, receipt lines, open questions | `intake/expense-claims/<claim_id>/draft.json` in the home, written atomically by the program only; the directory gets its own `.gitignore` | no | yes, removed when the version is sealed or the draft discarded | Changes daily, holds personal data, is not a voucher. Its own `.gitignore` protects homes whose root `.gitignore` predates it (the root file is never rewritten). An intake kind under ADR-5, so it inherits ADR-5's accepted-change log (content hashes, not content) when that log is built; the document records the draft's last revision hash, and a discard keeps its reason in that log. |
| Receipts, tickets, drive evidence | `artifacts/` at capture, the draft holds the hash | no | per file, after retention | Content-addressed evidence is fixed the moment it is captured. |
| Approved claim document | `artifacts/`, canonical JSON through the wire encoder, its own `record_type` and `schema_version` | no | after retention, without breaking the chain | It is the travel claim, a self-made voucher (KPL 2:5). Its hash is in the vouchers (below), so any change shows. Outside the chain, so the personal data in it can be erased when retention ends. |
| Vouchers sealed by the approval | `fiscal.sqlite`, sealed | no | no | Each carries the document hash as `attachment_ref`. |
| Links: claim version → document hash → vouchers; claim → settlement voucher; receipt hash → claim | `fiscal.sqlite`, new tables outside the hash chain, written in the seal's transaction (precedent: `bank_event`) | no | no | KPL 2:6 trail in both directions, checkable by the program. |
| Workplace register, relationship, part-owner | `masterdata/pii.sqlite`, new append-only tables (section 3) | no | yes | Personal data, rarely changing, not owned by one claim. |
| Name, HETU, IBAN | `masterdata/pii.sqlite`, as now | no | yes | Unchanged. |
| Travel allowance rates | `crates/domain/data/fi-travel-allowances.json`, bundled | in the binary | — | Law, versioned like the VAT and payroll rate tables. |
| Human-readable claim | rendered on demand (`claim show --format markdown`) | not stored | — | A stored copy of the view is exactly what drifted in the file-based practice. |

**The document contains:** claim id, version and the hash of the version it
supersedes; business ID; `employee_id` and the claimant's name (the voucher
must identify the claimant without the program; HETU and IBAN stay out);
the claimant facts in force (relationship, part-owner) with their source;
every trip with its workplace snapshot; every receipt line with its artifact
hash; the computation (travel days, applied rate per line, rate-table edition
and its hash, decision identifiers, program version); totals by kind; the
proposed postings by voucher; the Incomes Register lines and the lines left
out with the reason; recording times and sources; outside lifecycle facts
(section 7); and the approval record (section 6).

**The seal covers** the document, through its hash in every voucher of the
version; the receipts, through their hashes in the document; and so the whole
travel claim with its attachments. It does **not** cover payment, the payment
date, the Incomes Register receipt, rebilling status, the draft's history,
raw material (calendar, chat, location history) or the register's present
values. The link rows are outside the chain like `bank_event`, and
`claim verify` checks them against the vouchers and checks that every
document exists in the artifact store and hashes right, which `voucher verify`
alone does not.

### 3. The regular-workplace register and the trip snapshot

The register is bitemporal and append-only in `pii.sqlite`:

- **place**: opaque id, kind (company premises, premises of a company in the
  same interest group, home, self-arranged space, customer site, venue,
  other), label, location for distances, municipality;
- **assignment** of a role to a person on a place: regular workplace
  (primary or other), secondary workplace, tracked customer site, or the
  explicit "no regular workplace" (an outside board member), with
  `valid_from`/`valid_to` (when the fact holds) and `recorded_at` (when the
  program learned it), a closed list of basis codes tied to the guidance (for
  example *company premises in the owner's home*, ohje 3.4; *employer
  premises visited weekly*, ohje 5.2.2 and KHO 2017:178; *remote work by
  choice*, ohje 10.7) with `other` needing an explanation, who confirmed it
  and with what evidence, and `supersedes` with a reason for a correction;
- for the three-year rule: the **principal-site assessment** (yes, no,
  unknown, from a date), **earlier exposure** at the site the program cannot
  see (another employer, before aggountant), the agreed **assignment end**
  (date, open-ended, unknown) and **gap confirmations** (worked elsewhere
  during a six-month gap).

A row is never edited; a correction is a new row that supersedes it. The
query the computation uses is "roles in force for this person on this date,
as known now".

**Snapshot.** Each trip carries the register's state on the trip's start
date: the roles, the start place's role, the destination's classification
(special work site, secondary workplace, regular workplace), the site clock
for a tracked site, the basis codes, the digest of the register rows used,
and the decision and guidance versions applied. It is refreshed on every
draft write and frozen in the document. A sealed trip therefore keeps its
own moment (the nature of a place is judged by the circumstances when the
trip is made, ohje 4.1). When the register is later corrected backwards,
`claim verify` lists the sealed claims whose snapshot differs from what is
now known; it changes nothing, and each is corrected as a new version
(section 7) if the user decides so.

**Asking.** A missing entry for the trip's date is an open question
(`REGULAR_WORKPLACE_UNKNOWN`). The agent asks the user in plain words and
proposes an entry with its basis; only the user's confirmation is written.
The program asks again only with a reason: a pattern of trips that
contradicts the entry (weekly visits to a place recorded as occasional), a
new recurring destination, a changed assignment end, or a return after a
six-month gap. No yearly routine question.

### 4. What the program computes, checks and only records

**Computes** (pure, deterministic, `domain::travel::fi`): mileage per drive
from the vehicle, kilometres, surcharges and passengers; travel days from
elapsed time (more than 6 h, more than 10 h, the last partial day's 2 h and
6 h, abroad 2 h and 10 h); the 15 km and 5 km distance test from the given
distances; halving by free meals per travel day; the foreign per diem by the
country where each travel day ends, the return-day rule and the
under-10-hour rule; the meal allowance amount and its exclusion with per
diem; the night travel allowance's 4 hours between 21 and 07; the deduction
of a company-paid meal from the per diem (ohje example 60); the Incomes
Register lines; the proposed postings; the rebilling cost base. Amounts are
computed exactly and rounded once per claim, half away from zero, as payroll
does.

**The rate table** is a bundled JSON with one edition per year, each with
the decision's statute number, diary number, issue date and URL, all domestic
amounts and the foreign per diem by ISO 3166 country with sub-keys for the
cities and regions the decision lists, plus its "not separately listed"
row, used only for a valid country code and shown in the preview. The
edition is chosen by the date of the part of the trip (decision 19 §), never
by payment or approval date; a missing edition refuses
(`TRAVEL_RATES_MISSING`), nothing is extrapolated, and the table hash is
stamped into the document. Every new edition ships test vectors, including a
trip across the year boundary, and a row count and spot check against the
decision for the foreign table.

**Checks, a human decides** (warnings shown in the preview and the
document): kilometres against the declared route and the per diem distance;
recurring destination and the site share of weekdays; a receipt outside
every trip; a possible duplicate receipt by date, amount and merchant;
recorded long after the trip (section 5); company card purchases at
restaurants or hotels during the trip that the trip does not mention, once
bank import and claims are linked.

**Refuses** (an error blocks approval; the draft can still be written):

- no valid regular-workplace entry for the trip's date: no tax-free mileage,
  per diem, meal or night allowance, and no travel receipt costs of that trip;
  receipt lines without a trip are unaffected;
- the destination is a regular workplace on that date
  (`TRIP_TO_REGULAR_WORKPLACE`); a secondary workplace gets travel and
  lodging costs only, no per diem, meal or night allowance, and nothing for
  the journey between lodging kept because of the secondary workplace and
  that workplace (decision 970/2025, 4 § 2 mom), so the trip records a start
  at such lodging as its own start-place role;
- an amount above the scheduled maximum (`ALLOWANCE_ABOVE_MAXIMUM`), with
  the explanation that the excess is pay and needs withholding, which the
  product cannot do yet;
- the three-year rule: a principal site past its last tax-free day, a
  principal site agreed open-ended or known from the start to last more than
  three years (TVL 72 a §, ohje 5.3.1–5.3.3); a site whose principal status
  is unknown and whose share or age calls for it is an open question;
- free meals not answered on a trip that earns a per diem (an empty answer is
  not "no free meals"); for a meal allowance, the usual-eating-place and
  lunch-benefit answers not given;
- overlapping trips or the same travel day twice for one claimant, against
  the other open drafts and the versions in force of other claims (a
  correction draft is checked without its own predecessor, and a superseded
  or cancelled version no longer counts); a meal allowance with a per diem; a
  night allowance with lodging paid for that night;
- a statutory field missing (section 5);
- the same receipt artifact already sealed, by `capture-receipt` or another
  claim;
- the cases the sources do not settle: a free meal and a company-paid meal
  on the same travel day, a company-paid meal above half the per diem, a
  foreign trip that passes through Finland and continues abroad, a trip that
  starts from a holiday home, leisure combined with the trip, a workplace
  change inside a trip (split the trip), border-crossing allowance, posting
  (komennus), and work abroad.

Defaults where the sources are silent, shown in the preview when the result
depends on them: a travel day that crosses the year boundary gets the lower
of the two years' amounts (tax-free under either reading); a travel day is
24 hours of elapsed time, so a daylight-saving change counts as it happened.

**Only records** (asked, confirmed by the user, stored with source and
date, never inferred): the regular workplace and the other register facts,
whether a meal was a free meal in the legal sense, whether the usual eating
place was out of reach, what part of the time was work, the relationship
and part ownership, the receipt kind and whether a cost is entertainment,
whether a cost is rebilled and at what price, and whether each drive really
happened.

VAT on receipt lines uses the Domain's existing expansion and the same
treatments as `capture-receipt`, reverse charge included: a card slip
without VAT itemisation, a cost with no receipt, entertainment, and foreign
VAT charged by a foreign seller (a hotel or restaurant abroad) carry no
deduction in the Finnish return
([AVL 102 a §, 114 §](https://www.finlex.fi/fi/lainsaadanto/1993/1501);
KILA general guidance 2021, 5.4). The seller's country alone decides
nothing; an unclear treatment is `clarification-required`, never `no-vat`.
The treatment is decided and sealed at approval like any other.

### 5. Evidence that the trips happened

- **Recording time.** Every trip and line carries `recorded_at` (written by
  the program) and `source` (the user's statement on a date, an imported
  outside record, or a reconstruction the user confirmed item by item). A trip
  recorded more than 60 days after it ended is marked "recorded afterwards"
  in the preview and the document; it is allowed, not hidden.
- **Statutory fields are mandatory**: purpose, destination address, route
  when the user marks it needed, means of travel, start and end date and
  time, the country at each foreign travel-day end, and for a drive the
  kilometres (the unit price comes from the table). The start-place role and
  the vehicle's ownership (own, controlled, company car benefit) are product
  requirements, because without them the program cannot tell whether a trip is
  a work trip or whether a drive earns mileage.
- **Evidence links.** A trip or drive may list artifact hashes: tickets,
  parking or fuel receipts, a meeting invitation, an exported route. They are
  sealed with the document.
- **No silent reconstruction.** The agent may propose drives from a calendar
  or mail, but a drive enters the draft only when the user confirms it;
  proposals are not stored as facts.
- **Plausibility checks** are the warnings and refusals of section 4.

### 6. Approval

`claim preview` returns the content and its hash. `claim approve` takes the
hash and one or more approvals, each with the approver's name, an optional
`employee_id`, a role (claimant, managing director, board member,
accountant, other), the time, and optional evidence (an artifact hash, for
example an approving e-mail). All approvals bind the same hash; a changed
draft refuses as `CLAIM_PREPARATION_CHANGED`. The claimant's statement (the
receiptless costs and the free meals are declared truthfully) is part of the
approval.

- **Self-approval** (approver is the claimant) is allowed and marked
  `self_approved` in the preview, the document and the status, with an
  optional note such as "one-person company: no separate verifier possible".
  KILA's principle that someone other than the bookkeeper verifies is
  followed where possible; a fake separate control is not modelled.
- **A second approver** is just a second entry in the same command,
  typically an approval received by e-mail and attached as evidence. There is
  no stored "first approval awaiting the second"; an approval waiting in the
  draft would be an unfinished state with no owner.
- The program cannot authenticate who is at the keyboard (one trusted user,
  v1); the approval records who the operator states approved. A stronger
  identity is a multi-user concern.

### 7. Workflow and derived state

The commands: write the draft (`claim trip add|revise|remove`, `claim line
add|revise|remove`, JSON on stdin; each write returns the open questions),
`claim show`, `claim preview`, `claim approve`, `claim discard --reason`,
`claim status <id>`, `claim list --open`, `claim settle` (non-bank payment),
`claim correct`, `claim cancel`, `claim verify`, the register commands
(`claimant workplace add|correct|show --date`), and `file-wage from-claim`.

**No Run.** A claim's later steps are decided by others: the user pays from
the bank, the Incomes Register answers, a customer pays a rebilled invoice.
A Run waiting for them would sit open for weeks, collide with workflow
version locks on every release, and show unfinished work after the fact was
settled elsewhere, which is how VAT runs were left hanging. The program
records each fact when it happens and derives the state from the facts, the
way `run show` derives a Run's state. The `file-wage` Run stays a Run: it is
the program's own irreversible filing.

**Every unfinished state has a closing command.**

| Derived state | From | Closing commands |
|---|---|---|
| Draft with open questions | draft exists, questions remain | answer (draft write, register entry) or `claim discard --reason` |
| Ready for approval | draft, no open questions | `claim approve` or `claim discard --reason` |
| Approved, unpaid | version link, no settlement | bank import settles it; `claim settle` with evidence (offset or another account); `claim cancel` |
| Paid, report due *(deadline)* | settlement, reportable lines, no accepted report | `file-wage from-claim` then the `file-wage` Run; `file-wage complete-manual` for a report given in the e-service |
| Report outcome unknown | the `file-wage` Run is `OutcomeUnknown` | `file-wage reconcile` (as now; never a resend) |
| Register correction outside aggountant | a correction reduced an amount already paid and reported | `file-wage correction complete-manual` with the e-service evidence |
| Complete | settled, and reported or nothing to report | none |
| Superseded / cancelled | a later version or a cancellation exists | none |

A refusal leaves no claim state to close: a preview that refuses records
nothing, and a refused approval leaves the draft as it was, with the
ledger's rejection record as for any refused seal. `claim list --open`
answers "what is unfinished, by when, and which command next" across
claimants in one call, with `next_command`. The period close treats a draft
with trips or lines in the period as unresolved intake under ADR-5's
must-seal rule, dispositioned by approval, discard or a reasoned deferral; unpaid
or unreported approved claims are shown as warnings, not blockers, so the
close is never held hostage to a payment or a report.

**Outside travel-claim systems.** When a claim comes from another system,
`claim import-external` creates the draft from its record and stores that
system's lifecycle steps (submitted, reviewed, approved by whom and when) as
`external_events` facts with the system's name and evidence. They go into the
document and the preview but drive no state of their own. aggountant's own
approve-and-seal is still the bookkeeping step, and the outside approval is
shown as evidence for it. Internal steps (a draft write, a recomputation)
are silent: they leave no user-visible state.

**Corrections.** `claim correct` opens a draft copied from version *n*; its
approval seals version *n+1* (superseding *n*), correction vouchers for the
difference in the first open period, and the links. If version *n* was
reported, the Incomes Register follows the Tax Administration's *Correcting
data in the Incomes Register* (VH/7200/05.00.00/2025):

- a reporting error with the right amount paid is fixed by a replacement
  report on the same `source_reference`, which the program prepares as its
  own approval;
- an underpaid amount is a new report for the date the difference is paid,
  with its own reference (section 9);
- an amount actually overpaid is not removed from 311 or 331: the excess
  becomes taxable pay that can be recovered as unjust enrichment (the
  guidance's example 23), which `file-wage` cannot report. The program
  books the correction and a receivable from the claimant, and the claim
  shows "Incomes Register correction needed outside aggountant" until
  `file-wage correction complete-manual` records the e-service evidence. This
  is the same line as refusing an excess allowance until payroll exists.

`claim cancel` is a correction to empty content and follows the same rules.
A sealed voucher and a filed report are never changed.

### 8. Vouchers the approval seals, and payment

In one transaction, through a new batch seal in `ledger` (the bank import's
batch is the precedent), the approval writes the document and seals:

- **one voucher per receipt line**, dated on the receipt date, with the
  Domain's VAT expansion and the claimant payable as the counter account,
  exactly as `capture-receipt` would book it. The VAT period follows the
  receipt, not the claim, and a receipt in a sealed VAT period is refused as
  it is for `capture-receipt`;
- **one voucher per calendar month in which a trip of the claim ends**,
  holding that month's scheduled allowances (no VAT) and crediting the
  claimant payable, dated the month's last day (accrual, KPL 2:3).

Every one of these vouchers has the document hash as its `attachment_ref`
and a natural key naming the claim, the version and the part, so a voucher
leads to its claim and the claim to its vouchers. A closed period refuses; a
fiscal-year boundary is crossed only by the year close's accrual (siirtovelka),
which lists trips with no sealed claim.

The payable is a chart role `expense-claim-payable` in a new role file
(masterdata by addition); `fi-general-2026` maps it to `2740`. The amount
outstanding per claim is derived from the links, not from the account
balance, so a shared account stays reconcilable per claimant.

**Payment.** The bank import resolves a payment to a claimant as the
settlement of named claims: it seals the payment voucher (payable to bank)
and writes the settlement link in the same batch. The named claims'
outstanding amounts must add up to the payment; partial payment is refused
for now. A payment made otherwise (offset against a receivable from the
claimant, another account whose statement is not imported) is recorded with
`claim settle`, which seals the settling voucher with its evidence. The
settlement voucher's date is the Incomes Register payment date.

### 9. The Incomes Register

- `file-wage from-claim` (issue `wage-report-from-claim`) builds the report
  input from sealed documents only: 311 with the paid amount and the total
  kilometres rounded to a whole number, 331 with the paid amount, meal
  allowance as 331 type 1 (not 303), receipt lines, lodging and the night
  travel allowance listed as left out with the reason, the part-owner flag
  from the register, the payment date from the settlement. No personal data
  in the input.
- **One 331 amount carries every per diem type it contains.** The
  register's guidance lists the types included in one daily-allowance amount
  and allows several at once, and the vendored schema allows it
  (`WageReportsToIRTypes.xsd`: `AllowanceCode` `maxOccurs="unbounded"`).
  `file-wage` today takes one type per line, so a month with full and partial
  per diems becomes two 331 transactions. `TravelLine::DailyAllowance`
  changes to a non-empty set of codes and the record emits one 331
  transaction; process state written with the single code is still read.
- **Monthly combination.** Tax-free reimbursements may be reported once a
  month, by the 5th of the month after the payment month, combining that
  month's payments with the latest payment date
  ([laki tulotietojärjestelmästä 53/2018, 12 §](https://www.finlex.fi/fi/lainsaadanto/2018/53)).
  `from-claim` therefore accepts several settlements of one claimant in
  one calendar month. With one claim the `source_reference` is the claim id
  (as the file-based practice already used); with several it is
  `expense-claims-<employee_id>-<YYYY-MM>` of the payment month; a
  supplementary payment after a correction gets `<claim_id>-p<n>`. The pay
  period runs from the earliest to the latest content date. **A settlement
  belongs to exactly one report**: the `file-wage` process state lists the
  settlements it reports, `from-claim` refuses a settlement already in a
  report, and a reported settlement changes only through a correction of its
  report. That list is also how a claim's status finds its report.
- **The deadline** shown is the statutory one, from the
  `travel-report-statutory-deadline` slice. A taxable line, if one ever
  exists, keeps the five-day rule. A month whose only payments are tax-free
  reimbursements may still need the employer's separate report, which
  `file-wage employer-report` already covers; `claim list --open` points to
  it.

### 10. Rebilling

A trip and a receipt line may carry an opaque rebilling reference (issue
`rebilling-reference`). The trip's reference applies to its scheduled
allowances. The program computes the cost base per reference: a receipt's
amount net of the VAT deducted (gross when nothing was deducted) and the
allowance amount paid. Whether and at what price the customer is invoiced,
and at which sales VAT rate, belongs to the billing system and the user. The
reference reaches the sealed postings as an opaque code (an ADR-4 dimension
is the natural carrier), and the allowance voucher splits its postings by
reference. Rebilling is never a step of the claim: a customer's payment does
not show the claimant was paid, and approving a claim does not authorise an
invoice.

### 11. Vocabulary

The product uses the law's words and adds a plain explanation in parentheses
the first time a view or guide page uses one:

| Term | In the product |
|---|---|
| varsinainen työpaikka | varsinainen työpaikka (paikka, jossa teet työtä vakituisesti, esimerkiksi oma toimisto tai kotitoimisto) |
| toissijainen työpaikka | toissijainen työpaikka (oman yhtiön toinen vakituinen toimipaikka toisessa kunnassa) |
| erityinen työntekemispaikka | erityinen työntekemispaikka (tilapäinen paikka, kuten asiakkaan luona) |
| matkavuorokausi | matkavuorokausi (24 tunnin jakso matkan alkamisesta, ei kalenteripäivä) |
| ilmainen ateria | ilmainen ateria (työnantajan tai esimerkiksi asiakkaan tarjoama ateria, ei hotellin aamiainen) |
| pääasiallinen työskentelypaikka | pääasiallinen työskentelypaikka (paikka, jossa teet yli puolet työpäivistäsi tai työajastasi) |
| kululasku, matka, rivi | kululasku (korvaushakemus), matka, rivi |

English views use "regular workplace", "secondary workplace", "special work
site", "travel day", "free meal" with the same explanations. The user
guide's glossary gets these entries in the slice that first shows them.

### 12. Migrating a file-based practice

A home that already keeps claims as files moves without resealing anything:

- **Approved months stay as sealed.** Their voucher already binds the
  summary file the user approved. `claim register-legacy` records a link row
  of kind `legacy` (claim id in the format the home used, version 1,
  document hash = the voucher's existing `attachment_ref`, the voucher
  number), adds the settlement link from the existing payment voucher, and
  finds the existing `file-wage` report by its `source_reference`. A second,
  structured document for the same booking would be a second voucher for one
  transaction, so none is made. `claim show` renders a legacy version's sealed
  summary as it is.
- **A summary edited after approval** is non-authoritative and is left
  where it is; the guide says so. Its additions (payment, report receipt) are
  already in the books and the `file-wage` state.
- **The open month** is imported into a draft with `claim import --legacy`,
  which reads the line files and reports what is missing (free meals, start
  place, workplace entry) instead of filling it in; it is run on a throwaway
  copy of the home first.
- **Claimant facts** (regular workplace, part ownership) are entered once in
  the register, confirmed by the user.

Claim ids stay opaque; the home's existing format remains valid. New claims
get a generated id from the claimant and the first content date.

## Consequences

- The calculation and the claim's states move out of agent instructions into
  the program, so two companies get the same per diem for the same trip, and a
  release that fixes a rule fixes it everywhere.
- The file the agent reads is always either the live draft or the sealed
  document; there is no edited copy of an approved claim.
- The Incomes Register report is built from sealed content and the bank's
  payment date instead of being retyped.
- A claimant who never had a regular-workplace entry cannot be reimbursed
  tax-free until one is confirmed. That is one question per person, asked
  once, and it is deliberate: a wrong answer costs a correction voucher, a
  replacement report and, for an owner, possibly a hidden dividend.
- New surfaces: a draft store under `intake/`, a document schema that is
  frozen per version like the wire format, three link tables in
  `fiscal.sqlite` (a one-way migration), the register tables in
  `pii.sqlite` (also one-way), a role file, a rate table that must be
  updated every autumn before the first January trip is computed, and a
  batch seal in `ledger`.
- Draft, documents and register add personal data (routes, times, home as a
  workplace) to stores that are still unencrypted and have no deletion path;
  the user guide already says so for the PII store and artifacts, and that
  work is not part of this ADR.

## Rejected alternatives

- **Agent-written files** (a trip as Markdown with front matter, lines as
  YAML, as the product owner sketched and the file-based practice does).
  The program would validate only at preview, the agent would keep the
  format in its instructions, and an approved file stays writable. The
  readable view survives as `claim show --format markdown`; free text has its
  place as the trip's note.
- **Everything in `fiscal.sqlite`.** Daily draft edits would be fiscal
  writes, every new rule input a one-way migration, and personal data would
  sit in a store built never to delete.
- **The claim as metadata of one payable voucher.** No place for a draft, no
  structured source for the Incomes Register, a correction rests on a
  description.
- **One Run from capture to report, or from approval to payment.** Waits on
  acts the program does not perform, stays open for weeks, hits version locks
  and still needs cancel, paid-elsewhere and correction commands outside it.
- **One payable voucher holding all receipts.** Receipt VAT would move to
  the claim's period, and the same receipt could be booked by
  `capture-receipt` and a claim without the duplicate check seeing it.
- **Company-paid purchases as claim lines.** The product owner decided
  against a separate structure; only meals (and provided lodging) matter, as
  trip facts.
- **Regular workplace as a field of the employee record, in company
  knowledge, or only on each trip.** A field has no history (the record is
  replaced whole); company knowledge is prose in git and may hold no personal
  data; per-trip only means the same question every trip or silent copying.
- **A stored "awaiting second approval" state.** An unfinished state with no
  closing owner; the second approval is evidence in the one approve command.
- **Two 331 transactions for one month.** Not what the register's guidance
  describes; the schema allows the set.

## Open questions for the product owner

1. **Year-boundary travel day.** Lower of the two years' amounts (default
   here), or the amount of the year in which the travel day ends?
2. **Company-paid meal above half the per diem.** Refuse (default here), or
   pay half the per diem as for a partly self-paid meal?
3. **Rounding.** Once per claim, half away from zero (default here), or
   truncate so the tax-free maximum is never exceeded by a fraction of a
   cent?
4. **Payable account.** Map the claim payable to `2740`, shared with pension
   and unemployment insurance payables (default here, reconciled per claim
   from links), or give it its own account in the next chart edition?
5. **Retention of claim documents.** Six years (the bookkeeping minimum) or
   about ten (while an Incomes Register report can still be corrected)? Until
   decided, nothing is deleted, because deletion is not built.
6. **Leisure combined with a trip.** Refused in the first version (default
   here), or computed with the work time the user states?
7. **Non-bank settlement.** Is the offset date the Incomes Register payment
   date for a claim settled against a receivable from the claimant? Until
   confirmed, `claim settle` by offset asks the user to state the date with a
   reason.
8. **Waiting for an approver.** Does an accounting firm need a visible
   "awaiting approval by X since date" in the first version? If so it needs
   its own closing command (withdraw or expire on content change).
9. **Travel advances.** Out of the first version (assumed here)?

## Implementation slices

Filed as unlaned issues for triage; dependencies are written in each issue.
The first three can be tried on their own: compute a trip, record a
workplace, build a draft.

| # | Issue | What it delivers | Needs |
|---|---|---|---|
| 1 | `travel-allowance-rates-and-calculator` | Rate table 2026 and 2027 with the foreign per diem, `domain::travel::fi`, a read-only compute preview | — |
| 2 | `claimant-workplace-register` | Bitemporal workplace register, relationship and part-owner facts, the workplace refusals | 1 for the refusals |
| 3 | `expense-claim-draft` | Program-owned draft, trip and line commands, open questions, plausibility checks, Markdown view | 1, 2 |
| 4 | `expense-claim-approve-seal` | Claim document, approve-and-seal batch, link tables, payable role, `claim verify` | 3 |
| 5 | `expense-claim-status` | Derived status, `claim list --open`, deadlines, month-close checks | 4 |
| 6 | `expense-claim-payment-matching` | Bank import settles claims, `claim settle` | 4 |
| 7 | `wage-report-multiple-allowance-codes` | One 331 with several per diem types | — |
| 8 | `wage-report-from-claim` *(existing)* | Incomes Register input from sealed documents, monthly combination | 4, 6, 7 |
| 9 | `travel-report-statutory-deadline` *(existing)* | Statutory deadline | — |
| 10 | `rebilling-reference` *(existing)* | Rebilling reference, extended to trips and allowances | — |
| 11 | `three-year-site-clock` | Site clock, gaps, principal assessment, refusals | 2, 3 |
| 12 | `expense-claim-correction` | `claim correct`, `claim cancel`, Incomes Register follow-up | 4, 6, 8 |
| 13 | `expense-claim-legacy-migration` | Register sealed file-based months, import an open month | 3, 4, 6 |
| 14 | `expense-claim-external-import` | Import from an outside travel-claim system | 4 |

`employee-pii-entry` (optional tax card) is a prerequisite for travel-only
claimants; the existing issues carry a note on how this ADR changes or
confirms them.
