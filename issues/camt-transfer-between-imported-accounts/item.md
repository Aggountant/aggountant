---
created: 2026-11-23
updated: 2026-11-23
type: bug
status: untriaged
priority: normal
provenance: ai-review
source_ref: taskfleet:01m84vp4m5465em0twa01qgnc0/review-finding:transfer-two-imported-accounts
review_source: ai-review
originating_run: 01m84vp4m5465em0twa01qgnc0
originating_run_kind: spinoff
review_target: diff 06be3de..HEAD (camt-import-manual-bank-vouchers)
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
labels:
- ai-review-model:claude-fable-5-2
---

# camt import books an own-account transfer twice when both accounts' statements are imported

## Description

## Problem

`import camt053 seal` can book an own-account transfer twice when the company
imports the statements of both bank accounts and the agent books the transfer
directly between the two bank ledger accounts.

Synthetic reproduction (Esimerkki Oy, accounts 1910 current and 1920 savings,
fictional IBANs):

1. Import the 1910 statement. Its entry `REF-OUT` (500.00 EUR out) is
   assigned `contra_postings: [{account: 1920, amount: 500.00}]`, so the
   import seals Dr 1920 500 / Cr 1910 500 under event key
   `bank-import:holvi:<iban-1910>:REF-OUT`.
2. Import the 1920 statement. Its entry `REF-IN` (500.00 EUR in) has a
   different event key, so it is new; an assignment that books it against
   1910 seals a second voucher. The same 500.00 moved once and is booked
   twice; no conflict is raised.

The candidate search added for `camt-import-manual-bank-vouchers`
(`possible_bookings` in `crates/ledger/src/bank_event.rs`) deliberately
skips every voucher under the import's own key prefix, because such a
voucher books the transaction its key names. For a transfer that is only
half true: it also books the other account's leg. Including import vouchers
of another account in the search would stop the batch, but there would be
no truthful way out, because `bank_event.voucher_number` is unique (fiscal
schema v9) and a voucher can be bound to one bank transaction only. The same
limit leaves a hand voucher that books several payments (for example a
month's card payments in one voucher) with only one bindable payment; the
others must be reversed by correction vouchers before the import books them.

## What a fix needs

A decision on recording several bank events per voucher (which posting each
event stands for), which is a schema change beyond an additive table, and
then candidate search over import vouchers of other accounts. Until then the
skill could steer the agent to book transfers through a clearing account so
each statement books only its own leg.

Found by the verifier while reviewing `camt-import-manual-bank-vouchers`;
pre-existing, not introduced by that change.
