# Assessment of the wage-report-multiple-allowance-codes review

Assessed against the branch head (the golden-hash test is in the closing
commit).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| O1 | Process golden hash lacked a 331 line | CONFIRMED | n/a | BETTER | NONE | HIGH | FIX (test only) — applied |
| O2 | `main` cannot read a multi-type line (downgrade) | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | DROP — inherent to a new input shape; installs go forward, and one-type lines stay readable |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

No defect was found by either model; both rows are observations the verifier
made beside its answer. No follow-up issue was filed.

## Simplicity

- Reused: the existing `WageReport`/`TravelLine` types, the existing XML
  writer and its XSD test harness, the existing `file-wage` commands. No new
  command, flag, store or migration.
- One place for the wire shape: `WireLine` and its two conversions in
  `wage_report.rs` are the only code that knows about `allowance_code`
  versus `allowance_codes`; the rest of the code sees `AllowanceCodes`.
  Ordering is decided once, in `AllowanceCodes::new`.
- Code validation (non-empty, distinct, 1–4) stays in `WageReport::validate`,
  where the single-code check was, so the error path is unchanged.
- Left out: the fifth per diem type in the register's guidance (tax-exempt
  reimbursements related to work abroad); nothing in aggountant produces it
  and the code list it needs was not in scope. Also left out: merging several
  331 lines of one report into one, since the guidance allows separate lines.
