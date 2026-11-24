# Assessment: ADR-11 expense claim review

Source review: `history/review-adr-11-expense-claim.md` (finder gpt-6-astra, verifier claude-fable-5-2). HEAD `189ab91` (the ADR was uncommitted; fixes applied in the same working tree).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Overpaid allowance corrected by a 311/331 replacement [^F1] | CONFIRMED | OCCASIONAL | IMPROVES | MINOR | HIGH | FIX (Rule 5: confirmed, readability improves, minor reach) |
| F2 | Duplicate guards refuse the correction draft [^F2] | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX (Rule 5: confirmed, readability improves, minor reach) |
| F3 | Settlement reported twice; supplementary payment collides [^F3] | CONFIRMED | OCCASIONAL | IMPROVES | MINOR | HIGH | FIX (Rule 5: confirmed, readability improves, minor reach) |
| F4 | Meal allowance eligibility inputs missing [^F4] | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX (Rule 5: confirmed, readability improves, minor reach) |
| F5 | Secondary workplace lodging commute not excluded [^F5] | CONFIRMED | RARE | IMPROVES | NONE | HIGH | FIX (Rule 5: confirmed, readability improves, minor reach) |
| F6 | Foreign receipt as blanket no-deduction [^F6] | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX (Rule 5: confirmed, readability improves, minor reach) |
| F7 | ADR-5 accepted-change log not addressed [^F7] | CONFIRMED | REGULAR | IMPROVES | NONE | MED | FIX (Rule 5: confirmed, readability improves, minor reach) |
| F8 | Refusal leaves no trace contradicts rejection log [^F8] | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX (Rule 5: confirmed, readability improves, minor reach) |

[^F1]: review line 23, §7 Corrections. Evidence: Correcting data in the Incomes Register, example 23; TravelLine only 311/331. Applied to the ADR before merge.
[^F2]: review line 42, §4 refusals. Evidence: ADR text had no lineage exclusion; verifier confirmed trip half. Applied to the ADR before merge.
[^F3]: review line 54, §9 monthly combination. Evidence: wage_report.rs identity_hash; file_wage.rs start_wage. Applied to the ADR before merge.
[^F4]: review line 70, §1 trip facts, §4. Evidence: Decision 970/2025 14 § text. Applied to the ADR before merge.
[^F5]: review line 84, §4 refusals. Evidence: Decision 970/2025 4 § 2 mom text; fix is one clause and makes the rule list complete. Applied to the ADR before merge.
[^F6]: review line 96, §4 VAT. Evidence: capture_receipt.rs ReceiptVatProposal::ReverseCharge. Applied to the ADR before merge.
[^F7]: review line 108, §2 draft row. Evidence: ADR-5 §4; intake.rs says the full stage is later. Applied to the ADR before merge.
[^F8]: review line 119, §7. Evidence: crates/ledger/AGENTS.md Refusals; bank batch records refusals. Applied to the ADR before merge.

FIX: 8   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

F5 is RARE, but its fix is one clause that makes the list of statutory exclusions complete, so it improves rather than adds complexity (Rule 1b does not apply).

All eight were applied by hand to `docs/site/adr-11-expense-claim.md` and its companions (applicability `manual`: prose in a design document). No follow-up issue was staged: every finding is resolved in the ADR itself.
