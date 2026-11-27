# Result: camt-rich-fields

`import camt parse` now passes through, as typed optional fields of each
transaction, the source data the bank gives beyond amount, dates, parties and
remittance:

- `transaction_code`: `BkTxCd` of the entry, else of its transaction details;
  `domain` (`domain` / `family` / `sub_family`, all three or none) and
  `proprietary` (`code`, `issuer`).
- `reversal`: present for `RvslInd` true (`kind: "reversal"`) or `RtrInf`
  without it (`kind: "return"`); `reason` (`{"code": ...}` or
  `{"proprietary": ...}`), `additional_info`, `original_transaction_code`
  (`RtrInf/OrgnlBkTxCd`) and `end_to_end_id`, the link to the original.
- `charges`: `total` (`TtlChrgsAndTaxAmt`) and `records` (`amount`,
  `direction`, `included_in_amount` from `ChrgInclInd`, `type`, `bearer`),
  read from both the camt.053.001.02 layout (one `Chrgs` per charge) and the
  later one (`Chrgs/Rcrd`), on the entry or else its transaction details.
- `instructed_amount`: `AmtDtls/InstdAmt/Amt` as money in its own currency.

## Reversal semantics (decided)

A reversal is no longer refused. It is a booked entry in its own `CdtDbtInd`
direction (ISO 20022: a reversed debit is reported as a credit) under its own
reference; the agent assigns it the original payment's accounts, so the
projection's sides undo the original. A reversal reusing the original's
reference is refused within one file (duplicate key) and stops as
`SOURCE_FACTS_CHANGED` against an original already in the books; it is never
dropped. Its counterparty is read from the role opposite its direction (the
reversal keeps the original's roles), falling back to the direction's role.
The parent run chose this option (A) over keeping the refusal.

## Identity

The new fields are outside the event key, source fingerprint v1 and the
default description, and an unreadable passthrough element is left out rather
than refused, so every file the previous build accepted reads back to the
same identity. Verified by importing a fixture with all the elements with the
`main` build and re-importing it (and a re-export) with this build: duplicate
observations only. The `main` build's fingerprints are pinned in
`crates/import/tests/rich_fields.rs`. No store migration.

## Tests

- `crates/import/tests/rich_fields.rs`: golden files for a card entry,
  charges, a foreign-currency entry and a returned payment
  (`rich-camt053-sample`), a payment and its reversal with return reason
  (`reversal-camt052-sample`), and the unchanged Holvi camt.053 parse; pinned
  identity; later-version charge records; proprietary reason; malformed
  elements left out.
- `crates/cli/tests/import_camt.rs`: payment and reversal seal as two
  vouchers that cancel out; a reversal reusing a booked reference stops with
  `SOURCE_FACTS_CHANGED`; a rich re-export is a duplicate observation.
- `scripts/ci-local.sh` passed.

Review: `/llm-review` (one finder call) and assessment in this directory;
the one finding is fixed.
