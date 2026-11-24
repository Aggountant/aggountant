# Review: ADR-11 expense claim (docs/site/adr-11-expense-claim.md)

- **Finder:** gpt-6-astra (`--task review`), one call.
- **Verifier:** claude-fable-5-2, one batched call on the five candidates not
  already settled by a primary source (C2, C3, C6, C7, C8).
- **Model calls:** 2.
- **Rigor:** routine. The ADR is a design document; nothing is executed. Its
  risk class is data integrity (silently wrong tax treatment or Incomes
  Register reporting would follow from a wrong rule), which is why the
  verifier was used and why three legal claims were checked against the
  primary text.
- **Scope:** the ADR text as first written (Proposed, 2026-11-24), against the
  cited law and guidance and against the product's code and binding rules:
  `crates/AGENTS.md`, `crates/ledger/AGENTS.md`, `crates/domain/AGENTS.md`,
  ADR-5, `crates/domain/src/wage_report.rs`,
  `crates/broker/src/tulorekisteri_xml.rs`, the vendored
  `WageReportsToIRTypes.xsd`, `crates/ledger/src/employee.rs`,
  `crates/process/src/file_wage.rs`, `crates/process/src/capture_receipt.rs`.

All eight candidates were confirmed. The ADR was amended for each before
merge (see the assessment).

## F1 (must-fix): overpaid allowance corrected by a replacement 311/331 report

- **Location:** ADR §7 *Corrections*, "overpaid amounts by a replacement
  report on the same `source_reference`".
- **Claim:** money actually paid in excess cannot simply be removed from 331.
- **Evidence:** Tax Administration, *Correcting data in the Incomes Register*
  (VH/7200/05.00.00/2025), example 23: of a per diem paid for five days with
  entitlement to three, the entitled part stays 331 and the excess is
  reported as taxable time-rate pay marked unjust enrichment (code 201); the
  same guidance says an excess over the confirmed values "can change into a
  different income type (such as wages)". Read 2026-11-24 on vero.fi.
  `TravelLine` (`crates/domain/src/wage_report.rs:28-41`) can express only
  311 and 331.
- **Impact:** an overpayment would vanish from the register while the money
  stayed paid.
- **Fix:** separate reporting errors (replacement) from actual overpayments,
  which become a manual Incomes Register obligation with a closing command
  (`file-wage correction complete-manual`) until payroll exists.

## F2 (must-fix): duplicate guards refuse the correction draft itself

- **Location:** ADR §4 refusals, "overlapping trips or the same travel day
  twice ... across all drafts and sealed claims"; §7 `claim correct` copies
  version *n*.
- **Evidence:** no exclusion of the predecessor in the text. Verifier: the
  trip half survives; the receipt half is already scoped ("another claim").
- **Impact:** every correction that keeps a trip would be refused.
- **Fix:** check against other open drafts and the versions in force of other
  claims; a correction is checked without its predecessor; superseded and
  cancelled versions no longer count.

## F3 (must-fix): a settlement can be reported twice; a top-up can collide

- **Location:** ADR §9 monthly combination and `source_reference` rule; §7
  underpaid difference.
- **Evidence:** `WageReport::identity_hash` covers business ID,
  `source_reference`, `employee_id` and payment date
  (`crates/domain/src/wage_report.rs:118-131`); `start_wage`
  (`crates/process/src/file_wage.rs:100-119`) refuses a different proposal
  under one identity and opens a new Run for a new identity. A claim reported
  alone and later included in a combined report gets a second identity.
- **Impact:** a silent duplicate report to the Incomes Register; a blocked
  same-day top-up.
- **Fix:** a settlement belongs to exactly one report (listed in the
  `file-wage` state, refused by `from-claim` when already reported);
  supplementary payments get `<claim_id>-p<n>`.

## F4 (must-fix): meal allowance eligibility inputs missing

- **Location:** ADR §1 trip facts; §4 "the meal allowance amount and its
  exclusion with per diem".
- **Evidence:** decision 970/2025, 14 §: "Ateriakorvauksen maksaminen
  edellyttää, että työmatkasta ei suoriteta päivärahaa ja että palkansaajalla
  ei työn vuoksi ole mahdollisuutta ruokailutauon aikana aterioida
  tavanomaisella ruokailupaikallaan" (read from the decision PDF on Finlex);
  ohje 8.6 on the lunch benefit.
- **Impact:** a tax-free meal allowance computed for a trip that does not
  qualify.
- **Fix:** typed, confirmed trip facts (usual eating place out of reach,
  lunch benefit usable) and a refusal while unanswered.

## F5 (must-fix): secondary workplace lodging commute not excluded

- **Location:** ADR §4, "a secondary workplace gets travel and lodging costs
  only".
- **Evidence:** decision 970/2025, 4 § 2 mom: "matkustamiskustannusten
  korvaukseen oikeuttavana matkana ei pidetä toissijaisen työpaikan sijainnin
  vuoksi tarpeellisen erillisen majoittumispaikan ja toissijaisen työpaikan
  välistä matkaa."
- **Impact:** tax-free mileage for an excluded journey.
- **Fix:** a start-place role for lodging kept for work and the explicit
  exclusion.

## F6 (must-fix): "foreign receipt carries no deduction" as a blanket rule

- **Location:** ADR §4 VAT paragraph.
- **Evidence:** `ReceiptVatProposal::ReverseCharge`
  (`crates/process/src/capture_receipt.rs:144-147`) exists and carries both
  VAT legs; the ADR claims to follow `capture-receipt`.
- **Impact:** a reverse-charge purchase paid by the claimant sealed as no VAT,
  omitting output VAT from the return.
- **Fix:** only foreign VAT charged by a foreign seller is non-deductible in
  the Finnish return; the seller's country alone decides nothing; reverse
  charge stays available.

## F7 (should-fix): ADR-5 accepted-change log not addressed

- **Location:** ADR §2 draft row and "does not cover ... the draft's history".
- **Evidence:** ADR-5 §4 adopts an accepted-change log for intake and stamps
  `draft_change_log_head` into sealed provenance. Verifier: no mechanism is
  broken today (the log is not built; `crates/process/src/intake.rs:1-5`),
  but the ADR is silent on whether claim drafts inherit it, and a discard
  reason has no home once the draft is removed.
- **Fix:** state the inheritance; the document records the draft's last
  revision hash; a discard keeps its reason in that log.

## F8 (should-fix): "a refusal leaves no trace" contradicts the rejection log

- **Location:** ADR §7, "A refusal before an irreversible step leaves no
  trace".
- **Evidence:** `crates/ledger/AGENTS.md` *Refusals*: never return `Refused`
  unless the rejection record committed; the bank batch, named as the
  precedent, records refusals.
- **Fix:** a refusal leaves no claim state; a refused approval keeps the
  ledger's rejection record.

## Unresolved uncertainty

None carried. The open product questions are in the ADR's own section.

## Dropped candidates

None (0 of 8). Two were narrowed by the verifier: F2's receipt half and F3's
same-day collision (a loud refusal, not wrong data).

## Coverage and residual risk

Read and checked: the ADR, the files listed in *Scope*, decision 970/2025
4 §, 11 §, 12 §, 14 §, 17 §, 19 § (PDF text), the Incomes Register correction
guidance (examples 7, 15, 17, 23). Not checked against primary text in this
review: the foreign-trip return-day rules (13 §, ohje 8.4), the three-year
rule details (ohje 5.3), and AVL 209 f §; the ADR relies on the research
reports for them. The 2027 amounts are not restated in the ADR.
