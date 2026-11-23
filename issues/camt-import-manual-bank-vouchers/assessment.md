# Assessment of the camt-import-manual-bank-vouchers review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Binding one payment hid the voucher's other payments | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX — applied (ec5b5d2), superseded by F2 |
| F2 | The posting count still missed duplicates (repeated binding, transfer legs) | CONFIRMED | OCCASIONAL | BETTER | MINOR | HIGH | FIX — applied |
| R1 | Own-account transfer booked twice across two imported accounts | CONFIRMED | OCCASIONAL | NEUTRAL | MAJOR | HIGH | SPIN-OFF — filed `camt-transfer-between-imported-accounts` |

FIX: 2   FIX (with care): 0   SPIN-OFF: 1   DISCUSS: 0   DROP: 0

- F1/F2 are data integrity: each is a silent second voucher for money that
  moved once. Likelihood OCCASIONAL: hand vouchers booking several card
  payments are a common manual practice, and overlapping exports with a
  repeated binding are the documented path. The final rule is simpler than
  either count (one boolean picks the message; nothing is skipped), at the
  cost of a false stop for a second identical payment within 7 days of a
  bound one, which `not_booked_by` answers truthfully.
- R1 predates the change and needs several bank events per voucher
  (`bank_event.voucher_number` is unique in v9; dropping it is a table
  rebuild, not an additive migration), so it is a design decision, not a
  fix inside this slice.

No confirmed data-integrity finding introduced by this slice remains open.

## Simplicity

- Reused: the existing bank batch (one transaction, preview and seal through
  the same `resolve_bank_batch`), the hash-bound dry-run/approve flow and its
  preparation hash, `bank_event` and `bank_source_observation` as they are in
  v9, and the `idempotency` table for the binding (an extra key row pointing
  at the existing voucher). No migration, no new table, no new subcommand.
- One rule, one place: `BankEntryFacts::booked_by` decides both "this voucher
  may book the entry" and "this binding is valid", so the candidate search and
  the binding check cannot drift. `DATE_TOLERANCE_DAYS` is the one constant;
  the help states it and a unit test pins the help to it.
- Left out on purpose: several bank events per voucher (R1), auto-binding a
  single obvious candidate (the user decides), receipt matching and
  auto-assignment (out of scope).
- Preparation-hash fields for bindings and `not_booked_by` are written only
  when present, so an input with assignments alone keeps its hash.
