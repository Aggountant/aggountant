# Review: bank-row-for-recorded-receipt (guide, skill, practice month)

- Finder: gpt-6-astra (1 call). Verifier: claude-fable-5-2 (1 call, batched survivors). Model calls: 2.
- Rigor: routine. Risk class: user guide and agent skill for live bookkeeping; a wrong
  convention leads to double-booked purchases in sealed books, but the change is prose,
  examples and the practice-month script, not ledger code.
- Scope: the working-tree diff against `c80ae67` (skills/capture-receipt/SKILL.md,
  docs/user/{fi,en} record-receipts, import-bank-statement, practice-month, status,
  troubleshooting, monthly-report, monthly-routine, for-bookkeepers, glossary,
  docs/user/practice/{README.md,run-practice-month.sh}, CHANGELOG.md).

## F1 (should-fix): duplicate-expense check names a command that shows no postings

- Location: `skills/capture-receipt/SKILL.md` ("Vastatili" section, the sentence telling
  the agent to check with `voucher list` whether the payment row was already sealed as an
  expense).
- Claim: `voucher list` returns `VoucherSummary` rows (`crates/ledger/src/voucher_read.rs`:
  number, hash, dates, attachment_ref, match_status) with no accounts or amounts, so it
  cannot show that a bank row was booked to an expense account.
- Evidence: local read of `VoucherSummary` and `aggountant voucher list --help`; verifier
  traced `list_vouchers` and the CLI handler and confirmed only `voucher show` carries the
  body. Verdict: survives.
- Impact: an agent following the skill finds nothing, records the receipt against 2700 and
  the purchase is expensed twice.
- Fix: list candidates by posting date around the payment, narrow when `truncated`, and
  read postings with `voucher show <number>`.

## F2 (should-fix): Finnish troubleshooting describes the symptom wrongly

- Location: `docs/user/fi/troubleshooting.md`, "Pankkitilin saldo ei täsmää tiliotteeseen".
- Claim: the Finnish text says the leftover payable is "summa, jota kukaan ei ole
  laskuttanut" (an amount nobody invoiced); in the double-booking case the supplier did
  invoice it. English says "an amount no supplier is owed".
- Evidence: local read of both pages; verifier confirmed. Verdict: survives.
- Impact: a Finnish reader finds the matching invoice and dismisses the very defect.
- Fix: "summa, jota yritys ei ole kenellekään velkaa".

## Dropped candidates

None; the finder raised two and both survived.

## Coverage

Read: the full diff, the issue's decision, docs/user/AGENTS.md. Tested: the practice month
runs against a freshly built binary with the new convention (`scripts/check-user-guide.sh`:
language pairs, links, site build, practice month all pass), and the report shows 2700 at
zero after the bank row clears the invoice. Outside scope: ledger and import code (not
changed). Observed separately and filed as `report-negative-zero-balance`: the report
prints the cleared 2700 as `−0,00`.
