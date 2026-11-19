# Result

Implemented decision (a). One convention, stated in the guide and the skill:
a receipt or purchase invoice that will also be paid from the bank account is
recorded against 2700 Ostovelat (2740 when the owner paid and is repaid), and
the bank-statement row is assigned to that account without VAT to clear it.
The receipt records the expense and VAT, the bank row records the payment.
Cash purchases keep a direct contra (1900); 1910 is never a receipt's contra.

- `skills/capture-receipt/SKILL.md`: the record example uses contra 2700; a new
  "Vastatili" section gives the table, the clearing assignment JSON for
  `import camt053 seal`, the order-independence, how to check that a payment
  row was not already expensed (`voucher list` by date, then `voucher show`),
  and the month-end check of 2700.
- Guide (fi, en): `record-receipts` holds the convention and the worked example
  ("Kun osto näkyy myös tiliotteella"); `import-bank-statement`,
  `monthly-routine`, `monthly-report`, `status`, `troubleshooting`,
  `for-bookkeepers` and the glossary point to it instead of the old "ask the
  agent" limitation. The month-end check adds "trade payables are only unpaid
  invoices", because a receipt booked to 2700 plus a bank row booked to expense
  leaves the bank balance matching.
- Practice month (fi, en, script): the Pilvipalvelu invoice is now recorded as
  voucher 1 against 2700 and the statement row clears it; the office receipt is
  voucher 4. Figures are unchanged (result −123.50, bank 2 376.00, totals
  2 407.50, VAT 307 31.50). The script also asserts 2700 is zero. It runs in
  CI against the built binary.
- CHANGELOG: Unreleased/Changed entry.

Verification: `scripts/ci-local.sh` passes (fmt, clippy, deny, build and test
on stable and MSRV, user-guide gate including the practice month).

Review: `review.md` (finder gpt-6-astra, verifier claude-fable-5-2), two
should-fix findings, both confirmed and fixed (`assessment.md`).

Follow-up filed (unlaned): `report-negative-zero-balance`: the monthly report
prints the cleared 2700 as `−0,00`, which the practice month now shows.
