---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: high
lane: wage-reporting
lane_seq: 30
collision: [crates/cli/src/main.rs, crates/ledger/src/employee.rs]
related: ['@employee-store-plaintext-help']
---

# Safe personal-data entry and masked display for employee records

## Problem

The employee store has no safe path for entering personal data:

1. `employee add` reads only `EmployeeState` JSON on stdin, so the personal
   identity code and IBAN have to be typed into a file, a heredoc or a chat with
   an agent. Users end up writing their own hidden-input wrapper scripts.
2. `employee show` prints name, identity code and IBAN in clear text; there is
   no masked view for checking that a record exists or that the IBAN ends in
   the right digits.
3. `tax_card.withholding_pct` is mandatory even for a payee who only receives
   tax-free travel reimbursements through `file-wage`. The user has to invent
   `0`, a misleading placeholder that a later real payroll could silently use.

## Expected

- `employee add --interactive` (TTY only) prompts for identity code and IBAN
  with hidden input, validates format (identity-code checksum, IBAN mod-97),
  and prints only non-personal confirmation. Stdin JSON stays for automation.
- `employee show` masks the identity code and IBAN by default (for example
  `FI42…72`, birth-date part hidden); `--reveal` shows clear text.
- `tax_card` is optional; `run-payroll` refuses an employee without a tax card
  instead of assuming one. `file-wage` does not need it.

## Acceptance criteria

- [ ] Interactive hidden-input add exists and never echoes personal data.
- [ ] Masked default for `employee show`, explicit reveal flag.
- [ ] A missing tax card blocks payroll with a clear error; travel-only reporting works without it.
