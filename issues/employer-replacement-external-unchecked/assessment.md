# Assessment of the employer-replacement-external-unchecked review

Assessed against the slice's working tree before commit; both fixes are in the
same commit as the slice.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `--previous-pension-insurance` help promised a refusal that applies only to "No wages payable" | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX — applied |
| F2 | Pension-insurance comparison was order-sensitive; a stated list in another order was refused with advice to cancel | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: the production case is exactly a EUR 0 replacement, where the stated
  insurances are carried forward and never compared with the profile. The help
  now says which payment type is refused, and the preview warns, naming both
  lists, when the stated insurances differ from the current settings. A typo is
  then visible before approval instead of being sent unremarked.
- F2: the stated side is now typed by a person from the e-service, so list
  order is no longer controlled. The comparison is a multiset comparison
  (`same_insurances`), and when the original's insurances were stated the
  refusal names both lists and asks to check the statement before cancelling
  anything. Tested with two policies in reversed order and with a real
  difference.

No confirmed data-integrity finding remains open. No follow-up issue was filed.

## Simplicity

- Reused: the planner's existing replacement path and its two checks
  (`EMPLOYER_REPORT_UNCHANGED`, `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED`); the
  stated list simply takes the place of the stored report's list, so there is
  still one comparison. The CLI reuses `parse_pension_insurance` and the
  `--pension-insurance` / `--no-pension-insurance` shape of `configure`.
- Left out on purpose: storing the stated insurances in the `complete-manual`
  record. That would change the external record's identity hash and state
  schema for existing data; the replacement Run already stores the report it
  sent, so after one replacement aggountant knows the previous version itself
  (and refuses a needless statement with
  `EMPLOYER_REPORT_PREVIOUS_STATED_NEEDLESSLY`).
- Left out: fetching the original from the register. No interface returns it:
  transcripts 600/630 and record 312 carry only sums of employer's separate
  reports, and only the e-service "Submitted reports" view shows a report.
