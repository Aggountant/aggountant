# Assessment: skill-close-year review

Source: `history/review-skill-close-year.md` (one model, gpt-6-astra; lenses executor and edge-case). HEAD `072cab6`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | User-prepared statements: where do the comparison figures come from [Executor 1] | CONFIRMED | REGULAR | NEUTRAL | NONE | HIGH | FIX |
| F2 | Accrual already booked as a new-year expense before the run [Executor 2] | CONFIRMED | REGULAR | NEUTRAL | NONE | HIGH | FIX |
| F3 | Missing VAT filings: warning or prerequisite [Executor 3, Edge 3] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F4 | Manual depreciation is absent from the generated notes [Executor 4] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F5 | 'Line by line' comparison with different presentation [Executor 5] | CONFIRMED | REGULAR | NEUTRAL | NONE | MED | FIX |
| F6 | Recovery does not separate unsealed from sealed mistakes [Executor 6] | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX |
| F7 | Restart must reuse the original identifiers [Executor 7, Edge 6] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F8 | A stale gate is not necessarily an error [Executor 8] | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b: RARE, no readability gain) |
| F9 | Dirty repository when committing the home [Edge 2] | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b: RARE, no readability gain) |
| F10 | Interrupted run is not distinguished from a rejected one [Edge 4, Edge 5] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F11 | Post-seal deliverables are separate completion outcomes [Edge 6] | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b: RARE, no readability gain) |
| F12 | Archive write error is discarded after the year seal [Edge 7] | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b: RARE, no readability gain) |

FIX: 8   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 4

## Evidence

- **F1** When the user prepares the tilinpäätös, the skill only describes comparing against a preparer's finished figures. Evidence: Skill text named only an outside preparer's figures; small companies often prepare their own. Fix: One sentence: the run's statements are then the figures, and the document is finished from them before the last approval.
- **F2** The accrual paragraph covers a future invoice, not one already expensed in the new year while the close waited; both entries would stand. Evidence: The close happens months after year end and new-year bookkeeping continues meanwhile (verified in a scratch home: vouchers seal and months close with the prior year open). Fix: Tell the agent to check before start and correct the new-year entry; write TODO once sealed.
- **F3** The skill says the run does not check VAT filings but not what to do about a missing one. Evidence: start_close_year checks sealed subperiods, obligations and intake only; no vat_filed check. Fix: Name `file-vat status` and say a missing return is filed before the year is sealed.
- **F4** Depreciation given as a correction does not appear in the liitetiedot gate, so 'same as the document' cannot hold. Evidence: build_liitetiedot builds notes only from depreciation_lines(state). Fix: Say the notes cover program-computed depreciation only and the user confirms no contradiction.
- **F5** A preparer's grouping or rounding makes literal row equality impossible. Evidence: The run's statement rows are the program's own KPA row policy; an accountant's layout will differ. Fix: Allow explained presentation differences; unexplained differences are resolved first.
- **F6** Text told the agent to keep every item and add a counter-correction even when nothing was sealed. Evidence: Only sealed vouchers carry natural keys in the books; assert_no_unplanned_close_year_vouchers checks those alone. Fix: Unsealed items are corrected in the input; sealed ones are kept and reversed.
- **F7** 'Same items with the same amounts' omits that reuse is keyed on asset, accrual and correction ids. Evidence: Natural keys are closeyear:<year>:dep|acc|corr:<id>; a new id would seal a second voucher and the old one would block as unplanned. Fix: Say same id, accounts and amount, and keep the start input.
- **F8** Skill does not distinguish a changed ledger from wrong bookkeeping. Evidence: The program's refusal already says to reject and start a new run; a capable agent adds a correction only for an error. Rule 1b.
- **F9** 'commitoi koti' could sweep in unrelated changes. Evidence: Commit practice for the home is SOUL's house rule, outside this file; ordinary git judgment. Rule 1b.
- **F10** After a timeout an agent may follow the restart instructions although approvals persisted; resume is not a read. Evidence: resume_close_year forward-drives sealing after an approved gate and runs SealYear after an approved ApproveStatements. Fix: One short paragraph: resume, and that resume completes what was approved, including the year seal.
- **F11** Sealing, archive, knowledge, TODO and commit can each fail separately. Evidence: Start refuses a sealed year, so the close cannot be repeated; the rest is ordinary follow-through. Rule 1b.
- **F12** run_seal_year ignores the result of write_archive_stub, so a sealed year may have no archive and nothing reports it. Evidence: `let _ = self.write_archive_stub(run_id, &state);` after seal_year succeeds. A skill sentence for a disk failure is hardening (Rule 1b); the code behaviour is noted as a product gap in result.md.

All eight FIX rows were applied to `skills/close-year/SKILL.md` in the same change. The review's remaining points (malformed input, authorisation) were rated as already addressed and are not findings. No issue command was staged: F12 is a code observation with a rare trigger and is carried as a product-gap proposal in `result.md`.
