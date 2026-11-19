# Assessment: bank-row-for-recorded-receipt

Source review: `history/review-bank-row-for-recorded-receipt.md`. HEAD `c80ae6781617` plus the uncommitted slice.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Duplicate-expense check names `voucher list`, which shows no postings (`skills/capture-receipt/SKILL.md`)[^F1] | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX (Rule 5: confirmed, real impact, local fix) |
| F2 | Finnish troubleshooting says the leftover payable was never invoiced (`docs/user/fi/troubleshooting.md`)[^F2] | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX (Rule 5: confirmed, real impact, local fix) |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

[^F1]: Review section "F1 (should-fix)". Evidence: VoucherSummary in crates/ledger/src/voucher_read.rs has number, hash, dates, attachment_ref, match_status only; voucher list --help confirms; verifier traced the CLI handler.
[^F2]: Review section "F2 (should-fix)". Evidence: Read both pages; Finnish 'jota kukaan ei ole laskuttanut' vs English 'an amount no supplier is owed'.

Both FIX rows were applied in the same slice before merge (see the diff of the commit carrying this file). No follow-up issue staged: nothing remains open.
