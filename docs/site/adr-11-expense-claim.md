# ADR-11 — Expense claims: the program computes the claim, one approval seals it, and its state is derived from what happens afterwards

- **Status:** Accepted (2026-11-26)
- **Date:** 2026-11-24, rewritten and accepted 2026-11-26
- **Authors:** technical-decision worker for issue `expense-claim-model`
  (taskfleet run `01m87gzzt5thzm0qb8xrqabanq`), from the product owner's
  decisions of 2026-11-24 and research kept outside this repository; the
  review and its assessment are in `issues/expense-claim-model/`. Rewritten
  after a rethink (2026-11-25) that separated the product owner's decisions
  from the first writer's proposals and narrowed the text to what was
  decided; the product owner's answers of 2026-11-26 to the questions it
  raised are folded in, applied by spinoff run `01m8aw1tra9jdfqq7gvdwa56w8`,
  and his answers of the same day to the ADR's own open questions 1–9 by
  spinoff run `01m8azxjdt7ce6e5r7vkgn6f2p`. Decider: Jari Mustonen, who
  accepted the ADR on 2026-11-26.
- **Refines:** [ADR-5](adr-5-seal-timing) (a claim draft is intake: approval
  seals at once, and an unresolved draft blocks the period close).

---

## Context

Today aggountant has the pieces around an expense claim (kululasku) but not
the claim: `capture-receipt` books a receipt the owner paid, `file-wage`
files a travel report from lines assembled by hand, and `voucher seal` seals
a payable voucher with one attachment. A company that kept its claims as
files on top of these showed where that breaks:

- the monthly summary was edited after approval, when payment and report
  references were written into it, so the file the agent reads is no longer
  the one that was sealed;
- the line files were never sealed at all;
- the per diem calculation and the claim's states lived in the agent's
  instructions, which `crates/AGENTS.md` forbids ("Skills compose, the
  program enforces"), and which means the agent works the law out again
  every month.

The law does not know the word "kululasku". A claim bundles three money flows
that tax, VAT, the Incomes Register and the books each treat differently:
scheduled tax-free travel allowances (mileage, per diem, meal and night
travel allowance, tax-free up to the Tax Administration's yearly maximums,
[TVL 71–73 §](https://www.finlex.fi/fi/lainsaadanto/1992/1535)),
reimbursements of company costs the person paid against a receipt, and
taxable reimbursements, which are pay. A travel claim has statutory content
([ennakkoperintäasetus 17 §](https://www.finlex.fi/fi/lainsaadanto/1996/1124)).
For a shareholder the stakes are higher than for an employee: mileage
allowances that cannot be shown to rest on real drives are always a hidden
dividend ([Peitelty osinko](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48147/peitelty-osinko11/)).

### What the product owner decided on 2026-11-24

These bind the rest of this ADR.

- A claim is a container of one or more trips and lines, not tied to a
  month; monthly and per-trip claims both work. Claim, trip and lines are
  enough, and the agent builds the claim the user wants from them.
- The company's own purchases are not gathered into the claim. The exception
  is the trip's record of free meals and company-paid meals, which the agent
  fills in, because they change the per diem.
- The record format is this ADR's to decide, under one constraint: the
  program computes from the trip, so every field used in a computation is
  typed and validated by the program. Free explanation may be text.
- Lifecycle steps may come from an outside travel-claim system and are
  imported as facts. Some steps are internal and silent, not user steps.
- In the first version: claimants who are not employees (board member,
  unpaid owner), rebilling of receipt costs and of scheduled allowances,
  tracking of the three-year rule, and the foreign per diem table by
  country. Finland only, with the Finnish rules kept apart so that another
  country can come later, and without generalising now. Sole traders are out.
- The Incomes Register deadline shown is the statutory one.
- The approver's identity is recorded. Self-approval is allowed (the
  claimant says "hyväksyn"), a second approver is possible, and there is no
  heavy multi-stage approval. Where no separate verifier can exist, the
  approval may say so.
- Exceeding a scheduled maximum is refused and explained, until the product
  has payroll.
- Evidence: the recording time is stored, the statutory travel-claim fields
  are mandatory, evidence can be attached to a drive, and the program runs
  plausibility checks.
- Regular workplace (varsinainen työpaikka): a dated entry in the personal
  data store and a snapshot sealed into each trip. No tax-free travel
  allowance without a valid entry. It is asked when unknown or when there is
  reason to think it changed, not as a yearly routine.
- The product uses the law's words, and a plain explanation may follow in
  parentheses.

### What the product owner decided on 2026-11-26

The first text of this ADR made several choices without asking. The product
owner settled them:

- A Run exists only for the program's own irreversible outside act; the
  claim has no long Run (section 6).
- A trip in a month already closed is booked in the first open period of the
  same fiscal year (section 5).
- The draft is program-owned and written only by commands, not
  agent-written Markdown and YAML (section 1).
- An approval made in an outside travel-claim system can be recorded as
  aggountant's approval when the program's computation matches it
  (section 8).
- The three-year rule is strict (section 3).
- Attaching an already sealed voucher to a claim stays in the product;
  moving one home's own file format is that home's work, and older claim
  history goes through the phased `migrate` skill (section 10).
- The fiscal store and the personal-data store may each get one one-way
  migration, in the slices that need them (Consequences).

He then answered the nine questions this text had left open:

- A travel day across the year boundary gets the lower of the two years'
  amounts; a free meal is one meal whatever its price, and a company-paid
  own meal is deducted, not refused; rounding is once per claim; leisure
  combined with a trip is computed from the times the user states
  (section 2).
- Claim payables get their own account in the next chart edition, and
  retention is decided when deletion is built (section 5).
- Every company has an "awaiting approval" step, acknowledged at once in a
  one-person company (section 5).
- The offset date is the Incomes Register payment date (section 6).
- Travel advances are booked by hand in the first version (section 11).

What was left to this ADR is how the claim is stored, computed, approved and
followed afterwards.

## Decision

**The program owns the claim from the first trip to the seal. The agent
enters trips and receipts through the CLI; the program computes every
scheduled allowance from the trip and refuses what the law or its sources do
not let it compute. One approval, bound to the previewed content, freezes
the claim as a document and seals the vouchers that carry its hash. Nothing
is written into a claim after that: payment, the Incomes Register report and
corrections are separate facts, and the claim's state is derived from them
every time it is asked.**

Each part answers one of the three failures above. Field lists, command
names, storage paths, tables, error codes and the content of the document
are owned by the implementation issues listed at the end.

### 1. The program owns the draft

The draft lives in the data home as intake under ADR-5, outside git, and is
written only through the CLI. Each write validates the typed fields and
answers with the questions still open. The readable claim, Markdown
included, is a view the program renders on demand; it is never stored or
edited. Free text has its place as the trip's note.

The product owner had sketched a trip as Markdown with front matter and
lines as YAML written by the agent, and chose the program-owned draft
instead: with agent-written files the program would see the content only at
preview, the format would live in the agent's instructions, and an approved
file would stay writable, which is what failed in the file-based practice.

A claim has one claimant, who is a person in the personal-data register,
employee or not. The register gains two dated facts that are never inferred:
the person's relationship to the company and whether they are a part-owner
(an Incomes Register field). A contractor's costs belong on the contractor's
own invoice, so a contractor is not a claimant.

### 2. The program computes from the trip; a human supplies what is interpretation

The allowance rules, written out, are short. The difficulty is in the
inputs. So the boundary is drawn at the input, not at the rule:

- **Computed, never typed in:** mileage, travel days, domestic and foreign
  per diem, the halving by free meals, the meal and night travel allowances,
  the deduction of a company-paid meal, the Incomes Register lines, the
  postings and the rebilling cost base. The company may pay less than the
  computed amount; more is refused.
- **Asked and recorded with source and date, never inferred:** the regular
  workplace, whether a meal was a free meal in the legal sense, what part of
  the time was work, the receipt kind and whether a cost is entertainment,
  whether a cost is rebilled, and whether each drive really happened. The
  agent fills in the free and company-paid meals; an empty answer is not
  "none", because a missing answer would make every per diem too large. The
  claimant declares the meals true when approving.
- **Refused:** an amount above the scheduled maximum; a trip with a
  statutory field missing; a trip overlapping another of the same claimant;
  a receipt already booked; and the situations the Tax Administration's
  decision and guidance do not settle, which the calculator issue lists. A
  refusal names its reason with a machine-readable code and blocks approval,
  not the draft.

Where the sources leave the computation a choice, it is settled as follows:

- **A travel day across the year boundary** gets the lower of the two
  years' amounts.
- **A free meal is one meal whatever its price**, and the per diem is
  halved by the number of free meals. A meal the traveller buys and the
  company pays, for example with the company's card, is not a free meal:
  the tax-free per diem is the per diem minus the meal's cost, but never
  less than half the per diem
  ([Tax Administration guidance VH/630/00.01.00/2026](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48013/tyomatkakustannusten-korvaukset-verotuksessa7/),
  8.3.3 and example 60). An expensive meal therefore ends at the same half
  as a free meal and is not refused.
- **Rounding** is done once per claim, half away from zero.
- **Leisure combined with a trip** is computed from the work time and the
  travel times the user states (the same guidance, 8.3.2): the number of per
  diems comes from that duration, and free meals and, abroad, the country
  are asked for each computed per diem day, not by the clock. It is not
  refused in the first version.

The yearly maximums are a rate table bundled with the program, one edition
per year with the decision it comes from, chosen by the date of the trip
and never by the payment date. A missing year refuses; nothing is
extrapolated. The table must be updated every autumn before the first
January trip is computed.

The Finnish rules are one module of the Domain. The claim, its receipt
lines, approval and status contain nothing Finnish. No plug-in layer or
country parameter is built now; when another country arrives, the split is
made along this line.

Receipt lines get the VAT treatment `capture-receipt` gives the same
receipt, decided and sealed at approval.

### 3. Regular workplace: a dated register and a sealed snapshot

The register is in the personal-data store and is append-only with two
times: when a fact holds, and when the program learned it. A correction is a
new entry that supersedes the old one. Without the second time, a backdated
correction would look as if the approver had known it all along.

Each trip carries a snapshot of the register for its start date, frozen at
approval, because the nature of a place is judged by the circumstances when
the trip is made. When the register is later corrected backwards, the
program lists the sealed claims whose snapshot differs. It changes none of
them; each is corrected as a new version if the user decides so.

Without an entry valid on the trip's date the program computes no tax-free
travel allowance and no travel costs of that trip. Receipt lines outside a
trip are unaffected. A trip to the regular workplace is refused, and a
secondary workplace gets travel and lodging costs only. The agent asks in
plain words and proposes an entry with its basis; only the user's
confirmation is written. The program asks again only with a reason, such as
a pattern of trips that contradicts the entry.

For the three-year rule (TVL 72 a §) the program counts the time at a
customer site, the gaps and the last tax-free day. Whether the site is the
person's principal place of work is a human's dated assessment, because it
depends on working days the claims do not show. The rule is strict: past the
last tax-free day the program refuses, as it refuses an amount above the
maximum, and on a site that calls for the assessment an unrecorded one
blocks approval. The draft can still be written in both cases.

### 4. Evidence that the trips happened

Every trip and line records when it was recorded and from what source: the
user's statement, an imported record, or a reconstruction the user confirmed
drive by drive. A trip recorded clearly after it ended is marked as such in
the preview and the document; it is allowed and not hidden. A trip or drive
may carry evidence (tickets, parking receipts, a meeting invitation), sealed
with the claim. The agent may propose drives from a calendar, but a drive
enters the draft only when the user confirms it.

### 5. Approval is the seal

The user sees a preview, and the approval is bound to exactly that content;
a changed draft refuses. In one transaction the approval writes the frozen
claim document to the artifact store and seals:

- one voucher per receipt line, dated on the receipt date, so that VAT
  follows the receipt and not the claim, and so that the same receipt cannot
  be booked both here and by `capture-receipt`;
- one voucher per calendar month in which a trip ends, holding that month's
  scheduled allowances (accrual, KPL 2:3).

Claim payables get their own account in the next chart edition (proposed
`2741` Kululaskuvelat). Until a home is on that edition they are booked on
`2740` Muut lyhytaikaiset velat with other payables, and what each claim
owes is derived from the links between its vouchers (section 6).

Every voucher carries the document's hash as its attachment. The seal
therefore covers the claim and, through the hashes in it, the receipts. The
document is the travel claim as a voucher: it names the claimant and holds
the trips, their workplace snapshots, the computation with the rate table
used, and the approval. It is kept outside the hash chain so that its
personal data can be erased when retention ends without breaking the chain.
How long it is kept is decided when deletion is built, likely about ten
years for the data an Incomes Register correction may rely on; nothing is
deleted before then.

Each approval entry says which kind it is. An **own approval** records the
approver's name, role and time, and optionally evidence such as an approving
e-mail. A **recorded outside approval** is described in section 8.
Self-approval is allowed and shown as such.

Every company records who approves its claims, and the step "awaiting
approval by X" is in the first version in every company. In a company with
several people, the claim the claimant submits waits for that approver, and
the wait ends with one of its closing commands: the approver approves or
rejects, or the claimant withdraws the claim. In a one-person company the
step exists but is acknowledged at once: the claimant's approval is recorded
visibly as self-approval, with no separate wait. A second approver is a
second entry in the same approval. No "approved once, waiting for the
second" state is stored, because nobody would own it. The claimant's statement that the
receiptless costs and the meals are declared truthfully is part of the
approval. The program cannot verify who is at the keyboard; it records who
the operator states approved.

The approver is asked when the company's first claim is submitted, never in
`init`, onboarding or a normal month. A company with no claims records
nothing.

A claim may well come after its month is closed, since a claim is not tied
to a month. The allowances of a trip in a month already closed are booked in
the first open period of the same fiscal year, as a correction is, and the
document shows the trip's own dates. A receipt line dated in a closed period
is treated as `capture-receipt` treats the same receipt. A trip across the fiscal-year end is
carried by the year close's accrual (siirtovelka), which lists trips with no
sealed claim.

### 6. Afterwards: facts, and a state derived from them

A claim's later steps are decided by others: the user pays from the bank,
the Incomes Register answers, a customer pays a rebilled invoice. The product
rule is that **a Run exists only for the program's own irreversible outside
act**, such as a filing to an authority or a payment. A Run that waited for
someone else's act would sit open for weeks and show unfinished work after
the matter was settled elsewhere, which is how VAT runs were left hanging.
So there is no claim Run. The Incomes Register filing stays the `file-wage`
Run it is. The rule is new: the first claim slice writes it into
`crates/AGENTS.md`.

- **Payment** is recognised in the bank import, which links the payment
  voucher to the claims it settles. A payment made another way is recorded
  with its evidence. aggountant still pays nothing and keeps no payables
  ledger ("Vouchers, not ledgers of receivables", `crates/AGENTS.md`): what a
  claim is owed is derived from the links between its vouchers.
- **Payment by offset** against a receivable from the claimant, such as a
  travel advance (section 11), is a settlement like a bank payment, and the
  offset date is the Incomes Register payment date
  ([Tax Administration, VH/5655/05.00.00/2026](https://vero.fi/tulorekisteri/yritykset-ja-organisaatiot/yksityiskohtaiset-tulorekisterin-ohjeet/64028/tietojen-ilmoittaminen-tulorekisteriin-luontoisedut-ja-kustannusten-korvaukset12/),
  ch. 2.1). When the offset and the bank payment of the difference fall in
  different months, the source does not say which date the report uses, so
  the guidance is to date the offset on the day the difference is paid.
- **The Incomes Register report** is built from sealed claim documents and
  the payment date, not retyped. One per diem amount carries every per diem
  type it contains, as the register's guidance describes; `file-wage`
  changes to allow this. A month's payments to one claimant may be combined
  into one report, and a payment belongs to exactly one report. The deadline
  shown is the statutory one.
- **State** is computed on request, with the deadline and the next command,
  for one claim or for everything unfinished, including a claim awaiting its
  approver (section 5). Every unfinished state has a
  command that ends it truthfully; the states and their closing commands are
  listed in `expense-claim-status`.

The period close treats a draft with content in the period as unresolved
intake under ADR-5. An approved claim that is unpaid or unreported is a
warning, so the close is never held hostage to a payment or a report.

### 7. Corrections

A sealed voucher and a filed report are never changed. A correction is a new
version of the claim: a draft copied from the sealed version, approved the
same way, sealing correction vouchers for the difference in the first open
period. Cancelling is a correction to empty content.

If the corrected version was already reported, the Incomes Register follows
the Tax Administration's guidance on corrections: a reporting error is fixed
by a replacement report, and an underpayment is a new report for the day the
difference is paid. An amount actually overpaid becomes taxable pay, which
the product cannot report before it has payroll. The program books the
correction and a receivable from the claimant, and the claim shows the
register correction as owed outside aggountant until its evidence is
recorded.

### 8. Rebilling and outside systems

A trip and a receipt line may carry an opaque rebilling reference, and the
trip's reference covers its scheduled allowances. The program computes the
cost base per reference. Whether, at what price and at which VAT rate the
customer is invoiced belongs to billing. Rebilling is never a step of the
claim: a customer's payment does not show the claimant was paid, and
approving a claim does not authorise an invoice.

When a claim comes from an outside travel-claim system, its steps (submitted,
approved by whom and when) are stored as facts with their evidence and shown
in the preview and the document. They drive no state of their own.

aggountant's approval can then be the recording of the approval made in the
outside system. Such an entry names the outside approver, the time, the
system and its reference, and separately who recorded it. The program
records an outside approval only when its own computation matches the
content approved outside. A difference is either fixed at the source and
imported again, or the claim is approved in aggountant as an own approval.
There is no separate approval workflow for outside systems: it is the same
approval, which binds the sealed content either way.

### 9. Words

The product uses the law's words (varsinainen työpaikka, toissijainen
työpaikka, erityinen työntekemispaikka, matkavuorokausi, ilmainen ateria)
and may add a plain explanation in parentheses, for example "erityinen
työntekemispaikka (tilapäinen paikka, kuten asiakkaan luona)". The wording
lives in the user guide's glossary.

### 10. A home that already has claims

Vouchers sealed for earlier claims are not resealed. The product can attach
an already sealed voucher to a claim identity as it is, and its sealed
attachment remains the claim.

Moving one home's own file format into drafts is not a product command; it
is done on that home's side. Claim history from another program or practice
in general goes through the phased `migrate` skill (issue
`migrate-skill-phased`): old claims go to the archive with their vouchers,
and what is carried forward is the claims open at the cutover, the
reimbursements not yet reported, each claimant's regular workplace and the
earlier work at customer sites that the three-year rule needs.

### 11. Travel advances are booked by hand in the first version

The program does not handle travel advances (matkaennakko) in the first
version. They are booked by hand with today's commands, and the agent
guidance says how:

- The advance paid is a receivable from the person, `1730` Muut
  lyhytaikaiset saamiset (in the next chart edition its own account,
  proposed `1740` Matkaennakot), without VAT.
- The claim is always booked gross as a payable, as if there were no
  advance.
- The offset of the advance against the claim is its own voucher, debit
  the payable and credit the receivable, dated on the day the difference is
  paid or returned (section 6).
- The difference comes from the bank statement either way: the company's
  payment against the payable, the person's return against the receivable.
- The advance itself is never reported to the Incomes Register. The claim's
  mileage (311) and per diem (331) are reported in full, whether they were
  paid from the bank or by offset.
- The agent warns about an advance still open a month after the trip ended,
  and for a shareholder always before 31 December. A company may record its
  own practice in the company's knowledge.

Programmatic support comes after the first version: the chart edition with
the advance and claim-payable accounts, and an advance slice, which needs
payment matching to settle one claim in parts (offset and bank).

## Consequences

- Two companies get the same per diem for the same trip, and a release that
  fixes a rule fixes it everywhere.
- The file the agent reads is either the live draft or the sealed document.
  There is no edited copy of an approved claim.
- A claimant with no regular-workplace entry cannot be reimbursed tax-free
  until one is confirmed. That is one question per person, and it is
  deliberate: a wrong answer costs a correction voucher, a replacement
  report and, for an owner, possibly a hidden dividend.
- The fiscal store gets link tables and the personal-data store the
  workplace register, each through a one-way migration. Each lands only in
  the slice that needs it, with a backup taken first and the migrated copy
  compared with the original.
- Trips add routes, times and the home as a workplace to stores that are
  still unencrypted and have no deletion path. That work is not part of this
  ADR.

**Companies that do not use claims.** Nothing is asked or required until the
first trip or claim is recorded. When no claim draft or claim exists, the
period close and the year close behave exactly as before this ADR. The
regular-workplace question is asked only for a claimant. The store
migrations add empty tables and change no existing data or command output.
The rate table is read only when a claim is computed. The skills mention
claims only where the user is doing one.

## Rejected alternatives

- **Agent-written files.** Section 1.
- **Everything in the fiscal store.** Daily draft edits would be fiscal
  writes, and personal data would sit in a store built never to delete.
- **One Run from capture to report.** Section 6.
- **One payable voucher for the whole claim.** Receipt VAT would move to the
  claim's period, and the duplicate check would not see the receipts.
- **Regular workplace as a field of the employee record, as company
  knowledge, or only on each trip.** A field has no history; company
  knowledge is prose in git and may hold no personal data; per trip means
  the same question every trip or silent copying.
- **An outside approval taken as is.** If the program sealed content it had
  not computed to the same result, the seal would bind amounts nobody in
  aggountant checked. Section 8.

## Implementation issues

Filed unlaned for triage. Each owns the detail of its part, and its
dependencies are written in it.

| Issue | Owns |
|---|---|
| `travel-allowance-rates-and-calculator` | The rate table, the computation, the refusals and the cases the sources leave open with their defaults |
| `claimant-workplace-register` | The register, its basis codes, the relationship and part-owner facts, the workplace refusals |
| `three-year-site-clock` | The site clock, gaps and the principal-site assessment |
| `expense-claim-draft` | Draft storage and its change log, trip and line commands, open questions, evidence, plausibility checks, the rendered view |
| `expense-claim-approve-seal` | The claim document, own and recorded outside approvals, the vouchers and links, closed periods, the payable role, verification |
| `expense-claim-status` | The derived states and their closing commands, the open-claim list, the period-close checks |
| `expense-claim-payment-matching` | Settlement in the bank import and by other means |
| `wage-report-multiple-allowance-codes`, `wage-report-from-claim`, `travel-report-statutory-deadline` | The Incomes Register report from claims, its references and its deadline |
| `expense-claim-correction` | Versions, cancellation and the register follow-up |
| `rebilling-reference` | How the reference reaches the postings |
| `expense-claim-external-import` | Claims and lifecycle facts from an outside system |
| `expense-claim-legacy-migration` | Attaching earlier sealed vouchers to a claim identity |

`employee-pii-entry` (an optional tax card) is needed before a claimant who
only travels can be recorded. `migrate-skill-phased` carries the
travel-claim part of moving a company in.
