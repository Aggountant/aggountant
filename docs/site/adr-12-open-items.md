# ADR-12 — Open items: what is still owed is derived from sealed postings and settlement links, never kept as a ledger

- **Status:** Proposed
- **Date:** 2026-11-28
- **Authors:** technical-decision worker for issue `open-items-model`
  (taskfleet run `01m8gcdm7et7akjt1egj1dcmdm`), from research kept outside
  this repository; the review and its assessment are in
  `issues/open-items-model/`. Decider: Jari Mustonen.
- **Refines:** [ADR-11](adr-11-expense-claim) (section 6: the claim's
  settlement link becomes kind-agnostic; section 8: a scheduled payment is an
  outside fact that refines a state and drives none of its own).

---

## Context

Between period closes aggountant cannot say what is still open. A production
deployment reported the case that matters: sales invoices and expense-claim
payments are made in the bank's own tools, so the books learn about them only
at the month close, from the statement. A claim approved mid-month and
scheduled for payment in the bank looked unpaid at the close and was nearly
paid a second time.

Three facts shape the answer.

- **A bank statement carries only booked transactions.** The Finnish
  statement guide says camt.053 and camt.054 "may only include BOOK
  transactions"; pending ones (PDNG) can appear only in the camt.052 account
  report, and banks differ in what they put there
  ([Finanssiala, ISO 20022 Account Statement Guide 2020](https://www.finanssiala.fi/wp-content/uploads/2021/03/ISO-20022-Account-Statement-Guide-2020.pdf)).
  A scheduled payment is therefore invisible to the import until it is paid.
- **A payment proposal is not a voucher.** KILA's general guidance lists
  "maksuliikenneohjelmiston maksuehdotukset, maksuluettelot yms. erittelyt"
  among documents that are not vouchers in the sense of the Accounting Act,
  unless they carry required information found in no voucher, attachment or
  correspondence
  ([KILA, yleisohje kirjanpidon menetelmistä ja aineistoista 20.4.2021](https://kirjanpitolautakunta.fi/documents/8208007/11087193/final%202021-04-20%20KILA-menetelm%C3%A4ohje.pdf/9db9c781-f4e0-957e-508e-7500451bae46?t=1619681624950),
  5.7). A scheduled payment can be recorded as a dated fact, never booked.
- **The law asks for no receivables ledger.** A sub-ledger (osakirjanpito) is
  one from which the general ledger receives totals
  ([KPL 2:2.3](https://www.finlex.fi/fi/lainsaadanto/1997/1336#chp_2__sec_2));
  KILA adds that a system whose items reach the general ledger one entry at a
  time is "pääkirjanpidon osa-alue", not a sub-ledger (1.4.1). Payables and
  receivables must be continuously ascertainable when books are kept on a
  payment basis during the year
  ([KPL 2:3](https://www.finlex.fi/fi/lainsaadanto/1997/1336#chp_2__sec_3)),
  which nothing requires to happen inside the bookkeeping program.

In the product, `crates/AGENTS.md` keeps sales and purchases as vouchers and
puts invoicing, receivables and payables ledgers, collection and payment
batches out of scope. ADR-11 already derives what a claim is owed from links
between its vouchers. What it does not yet say is where those links live,
whether they are claim-specific, and how a scheduled payment is recorded.
That has to be settled before `expense-claim-approve-seal` runs: it makes the
claim's only allowed fiscal-store migration, and claim-specific tables would
force a second, data-copying migration over production homes when sales and
purchase invoices follow.

## Decision

### 1. An open item is derived, never stored as a balance

An **open item** is a sealed payable or receivable posting that a later
posting has not yet fully settled: an approved expense claim, and later an
issued sales invoice, a purchase invoice or a declared tax. Its outstanding
amount is computed every time from the postings that create it and the
settlement links against it, read as known at a given moment. No balance,
"paid" flag or status is stored anywhere. This is the same rule ADR-11 set
for claims, extended to every kind.

Each item is still a voucher of its own in the general ledger, so in KILA's
terms the open-item view is a part of the general ledger, not a sub-ledger.

The rule in `crates/AGENTS.md` keeps its content and gets a wording that no
longer reads as forbidding the view. Proposed text, written into
`crates/AGENTS.md` by the slice that creates the tables:

> **Vouchers, not ledgers of receivables.** Sales and purchases are recorded
> as vouchers. aggountant keeps no receivables or payables ledger of its own:
> what is still owed is derived each time from sealed postings and the
> settlement links between them, and no balance, paid flag or status is
> stored. Invoicing, dunning and debt collection, and initiating payments or
> payment batches are out of scope: they belong to the systems where invoices
> are issued and payments leave.

### 2. Identity and links live in the fiscal store, outside the hash chain

The identity of an item and its links are append-only tables in the fiscal
store, beside the vouchers and outside their hash preimage, as the bank
transaction's identity (`bank_event`) already is. They cannot go inside the
voucher: a sealed voucher never changes, so nothing could be attached to an
old one, and the settlement link only exists once the payment is sealed. The
voucher's `natural_key` is not the identity either: it is one per voucher, its
format is free, it cannot be given afterwards or corrected, and an item can
span several vouchers.

Four tables, kind-agnostic. Column names are the implementing slice's.

| Table | One row says |
|---|---|
| **open item** | an item exists: a program-assigned opaque id, its kind (`expense_claim` now; `sales_invoice`, `purchase_invoice`, `tax`, `claimant_receivable` reserved), the kind's own key unique within the kind (a claim's `claim_id`), and identifiers (payment reference in its own indexed column; invoice number, due date, source system and its reference) |
| **item origin** | this sealed posting, named by voucher number and posting index, creates or changes the item |
| **item settlement** | this sealed posting settles this amount of the item, with a kind (`bank`, `offset`, `other_account`, `credit_note`, `credit_loss`) and its evidence (bank event key, reference read, evidence hash) |
| **item fact** | a dated outside fact about the item; the first kind is `payment_scheduled` (section 4) |

The settled amount is stored because one transfer can pay two items and the
split cannot be recovered afterwards; it is an allocation decided once, not a
balance. A write is refused when the allocations against one posting would
exceed it. An offset voucher settles both sides through two rows: the claim
through its payable posting, a receivable item (a travel advance, an
overpayment) through its receivable posting.

Kinds and codes are checked by the program, not by a `CHECK` constraint, so a
new kind needs no table rebuild. The opaque id keeps whatever the kind's own
key contains (a claim id built from an `employee_id`) out of the other tables.

Every row carries when it was recorded and by whom. A row is corrected only
by a new row that names the row it supersedes (unique) and a reason, or by a
retraction, as in the claimant workplace register. Triggers keep the tables
append-only.

The tables are not hash-chained. `adjustment_attribution` is chained because
it moves figures between fiscal years; these tables change no reported
figure, like `bank_event` and `idempotency`. Their integrity is checked by
`claim verify` (later a generic verify): every named posting exists, is on the
kind's role account, and no posting is over-allocated. `voucher verify` does
not read them.

### 3. One kind-agnostic link from the first claim on

`expense-claim-approve-seal` creates all four tables, empty, in its single
fiscal migration, beside the claim's own version table. Approval writes the
claim's open-item row (first version only) and one origin row per sealed
payable posting in the seal transaction. A correction version adds its
difference voucher's payable posting to the same item, so a claim stays one
item across versions. The claim's own tables hold no payment, status or
balance columns.

`natural_key` stays the idempotency key. A seal may build its key from the
item (`claim:<id>:v<n>:<part>`), but no code reads an item out of a key: old
vouchers with other key formats and vouchers attached later would be missed.

Attaching an item to an already sealed voucher writes the same rows pointing
at the existing voucher and posting, and seals nothing. That is how
`expense-claim-legacy-migration` attaches earlier claim vouchers, and how a
sales or purchase invoice booked by hand later gets its identity.

There is one implementation, in `ledger`. Claim commands (`claim settle`,
`claim status`, the schedule commands) are names over it for claims, so the
skills and the guide speak of claims. A generic `open-item` command group is
added only with the first non-claim kind.

### 4. "Payment scheduled" is a fact with two times, not a state

A scheduled payment is an `item fact` of kind `payment_scheduled`: the date
the payment is scheduled for, when it was scheduled at the source if known,
an optional amount (the whole outstanding amount when absent), the source
(the user, or a named outside system and its reference; later possibly a
bank report), and optional evidence. The row's own recording time is the
second time, so "what was known when the month was closed" can be answered
later.

- A schedule cancelled at the bank is a new row superseding the old one and
  marking it cancelled; it was true until then.
- A schedule recorded wrong (wrong date, wrong claim) is superseded with the
  right content and a reason, or retracted.
- Recording, moving or cancelling a schedule needs no open period, because it
  books nothing.

The fact never settles an item; only a sealed posting does. It refines
"Approved, unpaid" and drives no state of its own, as ADR-11 section 8 says of
outside facts:

- **Approved, payment scheduled for *D*:** a live schedule, and the company's
  imported statements do not yet cover *D* plus the bank import's date
  tolerance (`DATE_TOLERANCE_DAYS`, seven days). The claim is not on the
  pay-now list, and the close warns with these words instead of "unpaid".
- **Scheduled payment not seen in the bank:** the imported statements cover
  *D* plus the tolerance and no settlement exists. The claim does not return
  to the pay-now list by itself: if the money left another way, that return
  is exactly the double payment this ADR prevents. The state asks for a
  decision and each answer is a command: import the missing statement, cancel
  the schedule (back to unpaid), reschedule, or record the payment made
  another way with its evidence.

Staleness is measured against the coverage of imported statements, not the
calendar. Measured by the calendar, the warning would fire at every close
whose next statement is not yet in, and would stop meaning anything.

The pay-now list is the claims in "Approved, unpaid" with no live schedule.
Recording a payment made another way (`claim settle`) while a schedule is
live succeeds but warns that the bank will still pay on *D* unless the
schedule is cancelled there.

The schedule is not part of the claim document or the approval: it is
recorded after the seal.

### 5. A second payment of a settled item is refused where it is booked

A bank import resolution can say "this transaction settles items X and Y
with amounts a and b". The outstanding amounts are recomputed inside the
import's write transaction, under the same writer lock, before the links are
written; the preview computes the same without writing. The check belongs in
the transaction because two imports, or an import and `claim settle`, could
otherwise both see the item open.

- A settlement of an item with nothing outstanding, or above what is
  outstanding, refuses the whole batch, which is logged like any batch
  conflict. Nothing is written.
- The refusal names every live settlement of the item (voucher number, date,
  amount, bank reference) and the next step: if the money really went twice,
  book the transaction as a receivable from the payee.
- Re-importing a transaction already booked with the same settlement is an
  observation. A different settlement stops the batch; changing a link is a
  correction (section 2), never a replay.

The likelier double payment comes from a first payment that was never
linked: booked as an ordinary posting on the claims payable account, the
claim still looks unpaid and returns to the pay-now list. Whether the import
stops on that is open question 7.

A wrong link is corrected by superseding it, and the books do not change,
since both items are on the same account. If the wrong link already fed an
Incomes Register payment date, the claim status shows the conflict and the
report is corrected on `expense-claim-correction`'s path, never resent by
itself. A posting on the wrong account is still fixed by a correction
voucher; a link correction never replaces one.

### 6. Companies without open items see nothing

The migration creates empty tables and changes no existing row. Where the
bank import and the period and year closes would look at items, they first
ask one indexed existence query and skip the rest when there are none; the
import adds no second pass over the vouchers it already decodes. Every
question and warning comes from an item the company itself created.
Each slice carries a regression test on a synthetic home without items: the
import, close and year-close previews are byte-identical to the build before
it, and the migrated and unmigrated copies give the same outputs.

### 7. Out of scope, by name

Invoicing (drafting, numbering, sending), reminders, interest and debt
collection; initiating payments or payment batches; a stored balance, paid
flag or status; closing an item from a schedule or an outside system's
status; settling an ambiguous match without a human approving it; driving a
bank's or outside system's user interface; continuous mirroring of an outside
invoice list; and an open item as an obligation that blocks a close. The
first two are the outside systems' work, the next three would give the
product a second truth that drifts from the vouchers, and the last would hold
the close hostage to someone else's payment, which ADR-11 already rejected
for claims.

## Assumptions that depend on bank sources

These are not yet checked against what banks and their AI connectors provide
(follow-up research J2, not done). None changes the tables; each changes how
much is automatic.

- Scheduled payments do not reach aggountant from the bank, so the schedule
  comes from the user or an agent reading the bank. If a connector exposes
  them, the fact's source is that system and its read time.
- Outgoing payments carry the payer's structured reference often enough to
  recognise tax payments (`camt-creditor-reference`); matching claim payments
  relies on amount, date and the counterparty name.
- The statement gives the counterparty's name, not their account.
- Importing a partial month and later the full month works with the bank's
  exports, so statements can be imported weekly.

## Open questions for the product owner

Each has a recommendation; none changes section 2's tables.

1. **Scope.** This ADR decides the model for every kind. Should the first
   version build only expense claims, with sales and purchase invoices as
   later slices, or plan both now? *Recommendation:* claims first; the invoice
   slices are filed but wait for this answer and question 2.
2. **Where open sales invoices are watched weekly.** The bank's invoicing
   tool, with aggountant reconciling at the close, or aggountant, which then
   needs invoices booked when sent? *Recommendation:* the invoicing system
   stays primary; aggountant supports both and shows only what the books know,
   and a company that wants the weekly view in aggountant books invoices when
   sent.
3. **Evidence for a scheduled payment.** *Recommendation:* the source is
   always recorded, evidence is optional. A wrong schedule turns into
   "not seen in the bank" as soon as a statement covers its date, so
   mandatory evidence adds work without adding protection.
4. **Claim warnings in the close's approved content.** If they are, a
   schedule recorded between the close preview and its approval needs a new
   preview; if not, the approval does not bind what the approver was warned
   about. *Recommendation:* include them, serialised only when present, so a
   home without claims keeps the same content hash.
5. **Partial payment in the first version.** A partial settlement link is
   easy in the tables, but a claim paid in two parts gets two Incomes
   Register payment dates, and no source says how its lines split between the
   two reports. *Recommendation:* refuse the partial link with guidance, so
   the statement can still be imported: book the paid part as a receivable
   from the claimant, recorded as a `claimant_receivable` item that names the
   claim (as in question 6), and offset it when the rest is paid, so the
   offset date is the one payment date. While that receivable is open, the
   claim's status and the pay-now list show the claim's outstanding amount
   less the receivable, never the full amount; otherwise the part already
   paid would be offered again. Partial links come with the travel-advance
   slice.
6. **Overpayment as a claim state.** *Recommendation:* yes. The second
   payment, booked as a receivable from the claimant, becomes an item of kind
   `claimant_receivable` that names the claim, and the claim shows "overpaid"
   until the return or an offset settles it. A balance on the receivables
   account alone is left for the agent to explain at every close.
7. **Stopping the import on an ordinary posting that reduces the claims
   payable while a matching claim is open.** *Recommendation:* yes, in the way
   `BANK_ENTRY_POSSIBLY_BOOKED` stops: only in a company with open claims,
   resolved by settling the claim or naming the claim the transaction does
   not settle.
8. **Integrity of the links.** *Recommendation:* append-only triggers and
   verify, no hash chain (section 2). A chain costs a few columns and one
   verify step in the same migration if wanted.

## Consequences

- `expense-claim-approve-seal` creates four more empty tables and writes two
  more kinds of row in its seal transaction. Later kinds need no migration.
- The pay-now list stops offering a claim whose payment is scheduled, which
  removes the reported near-double-payment, and a second payment of a settled
  claim cannot be booked as its settlement.
- What is open can be asked between closes, but it is only as current as the
  statements imported. A company that imports monthly sees a schedule go
  stale only at the next close.
- The fiscal store gets data that `voucher verify` does not cover; `claim
  verify` does.

## Rejected alternatives

- **No product change; the agent remembers schedules.** The reported case is
  exactly that failing.
- **An own receivables and payables ledger** with invoice objects, statuses
  and balances, synchronised with invoicing systems. It is a second truth that
  drifts from the vouchers, and it would ask questions on every sales voucher.
- **Mirroring an outside system's invoice list.** It ages between reads and
  binds the product to one provider.
- **The identity inside the voucher or in `natural_key`.** Section 2.
- **Claim-specific link tables now.** Sales and purchase invoices would need
  a second migration that copies production claims into new tables.

## Implementation slices

Filed unlaned for triage, or existing issues with a note on what this ADR
changes in them. Each owns its details and states its dependencies.

| Issue | What this ADR gives it |
|---|---|
| `camt-creditor-reference` | (in progress, unchanged) The structured reference as a passthrough field and tax-payment recognition |
| `expense-claim-approve-seal` | The four tables in its one migration, the item and origin rows at approval, verify of them, the rule wording in `crates/AGENTS.md` |
| `expense-claim-status` | "Approved, unpaid" as the only pay-now state; room for the schedule refinements |
| `expense-claim-payment-matching` | Settlement through the generic link, the in-transaction refusal naming the earlier payment, re-import comparison, the warning of a live schedule; the partial-payment and overpayment receivable items and the claim's amount net of them (questions 5 and 6); question 7 |
| `expense-claim-payment-schedule` (new) | Recording, moving, cancelling and retracting the schedule fact; the scheduled and not-seen refinements, the pay-now list and close warnings; questions 3 and 4 |
| `open-item-link-correction` (new) | Superseding or retracting a wrong settlement link or identifier, with the over-allocation and conflict checks |
| `expense-claim-correction` | Correction versions add origin rows to the same item |
| `expense-claim-legacy-migration` | Attaching writes item, origin and settlement rows |
| `expense-claim-external-import` | An outside system's "payment sent" is the same schedule fact with that system as source |
| `month-batch-approval` | The monthly manifest carries no schedule facts or settlement proposals |
| `tax-account-rollforward` | Taxes become items of kind `tax` through this model, so there is one list, not two |
| `invoice-open-items` (new) | Sales and purchase invoice items, at seal or attached later, and an open-item list; waits for questions 1 and 2 |
| `open-item-reference-matching` (new) | Settlement candidates in the bank import by reference, then amount and counterparty, never settled without approval |
| `close-open-item-balance-check` (new) | The close warns when a role account's balance differs from the outstanding total of its identified items |
