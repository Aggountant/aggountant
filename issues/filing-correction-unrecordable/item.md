---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: fixed
priority: high
provenance: ai-review
source_ref: taskfleet:01m7yj1dwxfz2vr4z5h867tt8t/review-finding:sha1:d8cfb3cf331def517313168ed7b13c00e28ecadd
review_source: ai-review
originating_run: 01m7yj1dwxfz2vr4z5h867tt8t
originating_run_kind: spinoff
labels:
- ai-review-model:gpt-6-astra
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_target: skills/vero-fi and skills/tulorekisteri (skill-vero-split draft)
review_confidence: HIGH
review_severity: High
lane: wage-reporting
lane_seq: 1
preparation: ready
closed: 2026-11-21
closed_by: taskfleet-worker
---

# A correction made in the e-service cannot be recorded; file-wage complete-manual succeeds without recording it

## Description

## Problem

A correction the user makes in an authority's e-service to a filing aggountant
already holds as accepted cannot be recorded, and for travel earnings reports
the attempt succeeds silently.

- `file-wage complete-manual <run> --ir-delivery-id … --ir-report-id …` on a
  Run whose send already succeeded returns the Run's existing state with exit
  code 0 (`complete_wage_manual` in `crates/process/src/file_wage.rs` returns
  `self.show(run_id)` when the journal has `EffectSucceeded` or
  `EffectReconciled`). The references given are discarded. Nothing in the
  output says so.
- `file-vat complete-manual` for a period that is already `vat_filed` is
  refused (`acquire_filing_authority`). That is loud, but there is still no way
  to record the corrected return.
- The employer's separate report has the same gap for a cancellation; that one
  is tracked in `employer-report-record-cancellation`.

## When it happens

Ordinary use. aggountant does not send a replacement or cancellation for a
travel earnings report, nor a replacement VAT return, so the user guide and the
skills send the user to the Incomes Register e-service or OmaVero for every
correction. Example (Esimerkki Oy, fictional): a kilometre allowance of
120.00 € is reported through `file-wage` and accepted; the kilometres were
wrong; the user gives a replacement report in the e-service and the agent runs
`file-wage complete-manual` with the new references. The command succeeds and
`run show` still shows the original report as the receipt.

## Impact

The books claim the authority holds something it no longer holds, and the agent
has been told the recording worked. Later decisions (a further correction, the
employer's separate report for the month, an audit trail) use the stale
receipt.

## Direction

At least: `file-wage complete-manual` refuses on an already accepted Run with a
code and a message that says the correction was not recorded. Better: one way to
record "this accepted filing was corrected in the e-service" with evidence,
shared by VAT, the travel report and the employer's separate report (see
`employer-report-record-cancellation`).

Until then `skills/tulorekisteri/SKILL.md` ("Jo hyväksytyn ilmoituksen korjausta
ohjelma ei kirjaa, vaikka komento onnistuu…") and `skills/vero-fi/SKILL.md`
("eikä ohjelma kirjaa korjausta…") tell the agent to archive the receipt and
note it in the home; remove those sentences when the program records or refuses.
The sentence in `skills/e-services/SKILL.md` that says a correction is recorded
with `complete-manual` needs the same change.

## Decision (2026-11-21, Jari Mustonen)

Correcting an accepted filing in the e-service is not the product's way. A
correction goes through aggountant, like the original filing: previewed,
approved by the user, sent through the API, receipted and recorded. The
e-service stays a fallback for what the API cannot do, not the documented
route.

This issue therefore covers:

1. **Never silent.** `file-wage complete-manual` on an already accepted Run
   refuses with its own code and a message that nothing was recorded (and why),
   instead of exit 0. Same principle for any other path that today accepts
   and discards evidence.
2. **Corrections through aggountant.** Find, from the authorities' API
   documentation, what each channel supports: a replacement Tulorekisteri
   earnings payment report (travel allowances) and its cancellation; a
   corrected (replacement) VAT return through Vero API (`FileVATReturn` for an
   already filed period). Build the correction as its own previewed and
   approved action wherever the API supports it, with the same outbox,
   reconciliation and "never resend on a guess" rules as the original. If one
   channel needs more than one slice, land the other channel and file the rest
   as a follow-up issue with the API facts.
3. **Guidance.** The skills and the guide stop sending the user to the
   e-service for corrections the program can make; where the API cannot, they
   say so and say how the program records the e-service fallback.

The employer's separate report cancellation stays in
`employer-report-record-cancellation`, but should follow the same decision.

## Resolution

### 2026-11-21T10:45:56Z · @taskfleet-worker

file-wage complete-manual refuses on an accepted report (WAGE_REPORT_ALREADY_ACCEPTED); the travel report's replacement and cancellation go through file-wage correction with preview, approval, outbox and reconcile-only recovery; an e-service correction is recorded from evidence. VAT replacement return is filed as vat-replacement-return. See result.md.
