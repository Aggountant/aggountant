---
created: 2026-11-10
updated: 2026-11-10
type: task
status: open
priority: high
lane: camt-identity
collision: [crates/model, crates/wire, crates/ledger, crates/cli/src/main.rs]
---

# Atomic camt import with event identity across overlapping exports

## Problem

The same bank transaction can appear in several camt exports, for example when
two yearly exports overlap at the year boundary. Each voucher's canonical bytes
currently include the whole XML attachment, so an unchanged transaction seen in
a second export produces a different request hash, and the next batch can be
rejected even though nothing changed. A batch can also seal its first
transactions before a later conflict aborts the command, leaving a half-written
import.

The opposite case must stay loud: if the bank has changed a transaction's facts,
or the user has changed how it is booked, the import must stop rather than
silently keep or replace the earlier voucher.

## Design

Keep four things separate:

| Item | Meaning | Use |
|---|---|---|
| Event key | Bank's unique entry reference + account | Find the same transaction in another export. |
| Source fingerprint | Versioned hash of the bank's transaction facts before booking | Detect a transaction the bank has changed. |
| Projection hash | Hash of accounts, amounts, VAT treatment and dates, without the XML container | Detect a changed booking proposal. |
| Source observation | Event key + hash of the XML artifact it was seen in | Record every original export a transaction appeared in. |

The ledger gets one atomic bank-transaction seal operation that writes the
voucher and the source observation in the same transaction:

- unknown event key → seal the voucher and its first observation;
- same fingerprint and projection hash → record only the new observation and
  return `duplicate_source_observation`;
- different fingerprint → stop with `SOURCE_FACTS_CHANGED`;
- same fingerprint, different projection hash → stop with
  `POSTING_PROJECTION_CHANGED`.

The original XML stays as an artifact, but it is not part of any voucher's
canonical identity. The existing `request_hash` keeps protecting the integrity
of the seal request and is not loosened.

`import camt seal` preflights the whole batch (parse, assignment, conflicts)
before writing. If any transaction conflicts, nothing is written; an accepted
batch is written in one ledger transaction.

Out of scope: migration of existing homes or old vouchers (homes created before
this change are rebuilt), general reversal handling (a changed bank fact stops
at a visible conflict), and automatic account or VAT assignment.

## Acceptance criteria

- [ ] Two synthetic yearly exports that share four transactions produce one voucher per unique transaction and four `duplicate_source_observation` results.
- [ ] A changed amount, date or bank status stops the batch before any write.
- [ ] The same transaction with a different assignment stops the batch before any write.
- [ ] The link between each XML artifact and the transactions seen in it can be inspected.
- [ ] Batch counters separate new vouchers, duplicate observations and conflicts.
- [ ] A separate simplicity review is recorded beside the issue.
