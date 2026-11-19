# Result

A zero statement balance now prints as `0.00` (`0,00` in HTML and PDF)
whatever side the credit-normal sign flip came from.

- `aggountant_domain::statement_amount_str` is the one formatter for statement
  and report amounts: it rounds to cents and clears the sign of any zero. The
  close-year statements, tase-erittelyt and the monthly report (native JSON
  and HTML) all use it; the ledger's separate `decimal_string` is gone.
- The defect was wider than the monthly report: close-year statement rows that
  summed to zero through a credit-normal account could also carry `-0.00`.
- `scripts/render-statement-pdf.py` renders a saved `-0.00` as `0,00`, for
  reports saved before this fix.
- Tests: `cleared_credit_normal_accounts_print_zero_without_a_sign`
  (ledger, the practice-month payables case plus a reversed sale, JSON and
  HTML) and `a_cleared_credit_normal_account_closes_at_unsigned_zero`
  (domain, close-year statements). Both fail on the old code.

Compatibility: the statement bytes, and so `statements_hash`, change only for
a year that has a zero-summing credit-normal row. Sealed years are not
recomputed, so they are unaffected. A close-year Run that computed its
statements before the upgrade and is approved after it is refused ("the
statements changed after approval was bound"); reject it and start a new
close.

Review and assessment: `review.md`, `assessment.md`.
