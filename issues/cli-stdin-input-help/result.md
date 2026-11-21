# Result: cli-stdin-input-help

Every command that reads JSON on stdin shows its input in an INPUT section of
`--help` (both `-h` and `--help`): field meanings, required and optional
fields, the error codes the fields lead to, and one synthetic example with the
command line that runs it.

## Commands

Already had one: `voucher draft|validate|seal`. New: `capture-receipt record`,
`capture-receipt start`, `hunt-receipts start`, `close-year start`,
`employee add`, `run-payroll start`, `import camt053 seal` (the per-entry
assignment shape), `vero-fi configure`, `file-vat start` (keeps the Vero
transport note), `file-wage preview`, `file-wage start`,
`file-wage employer-report configure`, `knowledge company add|update|retire`.

## Where

- `crates/cli/src/input_help.rs` holds every INPUT text and the unit tests:
  - `every_stdin_command_has_an_input_section_and_only_those_do` walks the
    clap tree; a listed stdin command without INPUT, an INPUT on an unlisted
    command, or a command whose description mentions stdin but is not listed
    fails it.
  - `every_input_example_parses_as_its_command_reads_it` parses each example
    into the type its handler reads (with `validate` where the handler runs it
    before any I/O) and checks the help shows the run command.
- `crates/cli/tests/stdin_input_help.rs` runs the offline examples end to end,
  and runs the `capture-receipt start` example through sealing and a period
  close to show its VAT reaches `file-vat preview`.

## Also fixed

- `employee` help no longer claims an encrypted store and names `employee_id`
  (closes `employee-store-plaintext-help`); the user guide's notes about the
  wrong help are updated.

## Filed

- `vat-treatment-deny-unknown-fields` (unlaned): `VatTreatment` ignores
  unknown members, so a `deductibility` on a full type is silently dropped on
  bank import. The help warns about it until the code is fixed.

Review and assessment: `review.md`, `assessment.md`.
