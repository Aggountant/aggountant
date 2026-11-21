# Result: voucher-vat-code-help

- `voucher draft --help` and `voucher seal --help` carry one shared `vat_code`
  paragraph: the code goes on the VAT base line, never on the VAT account
  line; how the return derives the VAT (normal and margin formulas) and that
  the VAT line is booked at that amount; the grammar
  (`<sales|purchase>-<rate class>`, `-deduct<NN>` on purchase codes,
  `sales-margin-<category>`); what is refused (`purchase-eu-*`, `-deduct` on a
  sales code, `purchase-margin-*`).
- New read-only, offline command:

  ```
  aggountant --json voucher vat-codes --document-date 2026-01-15
  ```

  lists every code the seal accepts on that document date with its rate class,
  percent in force, scheme, whether `buyer_vat_id` is needed and whether a
  `-deduct<NN>` suffix is accepted. It is `aggountant_domain::vat_codes_on`,
  which decides validity with the same `parse_vat_code`, EU-acquisition guard
  and dated rate table the seal uses. A chart's per-account mistag guard is
  left to `voucher validate`.
- `skills/capture-receipt/SKILL.md` keeps only the reason (the return sees the
  VAT only through the code) and points to the help and the listing; the guide
  (`record-other-entries`, `commands`, both languages) names the command.
- Tests: `vat_codes_on_lists_what_a_seal_accepts_on_that_date` (domain),
  `voucher_vat_codes_cli.rs` (CLI), the existing INPUT-section tests.
