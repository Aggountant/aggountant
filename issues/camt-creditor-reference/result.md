# Result: camt-creditor-reference

`import camt parse` and the `import camt seal --dry-run` preview show each
transaction's structured creditor reference as `creditor_reference`, read
from the first `RmtInf/Strd/CdtrRefInf/Ref`:

- `value`: the reference as the bank gave it (trimmed);
- `kind`: `domestic` (only digits), `rf` (starts with RF, any case) or
  `other`;
- `valid`: `PaymentReference` accepts the canonical form (check digit and
  length); always false for `other`;
- `canonical`: present when valid, without spaces or leading zeros and with
  uppercase RF. Banks may zero-pad a domestic reference to 20 digits, so
  references are compared by this.

An invalid or unknown reference is shown, never refused or dropped. The
check-digit rules are `PaymentReference`'s; there is no second parser.

## Tax reference recognition

A money-out entry whose `canonical` reference equals one of the company's
stored references in `masterdata/tax-payments.json` carries
`tax_reference_match`: `kind` (`general`, `self_assessed`, `prepayment` or
`payment_arrangement`), `period_identified: false` and a message saying the
reference does not identify the period. The comparison is
`TaxPaymentDetails::reference_kind` in `process` and runs in the CLI at
display time; `import` and `ledger` do not read tax data and nothing is
stored. Without stored details nothing is marked; without an initialised home
`parse` marks nothing; an unreadable details file marks nothing and adds a
top-level `warnings` entry instead of failing (review F1).

The issue named the self-assessed reference. All four stored references are
compared, because a limited company normally pays every tax, VAT included,
with the general reference and has no self-assessed one; matching only
`self_assessed` would miss exactly those companies. The kind says which
reference matched, so the agent still knows whether the tax is determined.

## Identity

The reference is outside the event key, source fingerprint v1, the default
description, the posting projection and the preparation hash. Verified by
sealing the new fixture with the `main` build (eb4a86b) and previewing,
re-importing and re-exporting it with this build: five duplicate
observations, no new vouchers. The `main` build's fingerprints and
descriptions are pinned in `crates/import/tests/creditor_reference.rs`. No
store migration.

## Tests

- `crates/import/tests/creditor_reference.rs`: golden output
  (`reference-camt053-sample`: domestic, RF, zero-padded, wrong check digit,
  other); classification incl. spaces, lowercase rf and bad lengths; same
  identity with and without `Strd`; pinned identity of the previous build;
  `Strd` without `CdtrRefInf` gives no field.
- The existing golden test for a file without the elements passes unchanged.
- `crates/process/src/tax_payments.rs`: `reference_kind` for every stored
  kind and no match.
- `crates/cli/tests/import_camt.rs`: re-import with references over entries
  sealed without them is duplicate observations; recognition with and without
  stored details (preparation hash unchanged, money-in not marked, the rest of
  the parse output unchanged); parse without a home; unreadable details warn.
- Guide (fi/en) bank-statement page and the `capture-receipt` skill describe
  the field and the tax match.
- `scripts/ci-local.sh` passed.

Review: `/llm-review` (one finder call, `gpt-6-astra`) and assessment in
this directory; the one finding is fixed.
