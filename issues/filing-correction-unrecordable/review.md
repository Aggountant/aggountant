# Review: filing-correction-unrecordable

- Skill: `/llm-review` on commit `6157604` against `d61b3c5`. The four findings below were fixed in the following commit; `assessment.md` has the triage.

- Finder: claude-fable-5-2. Verifier: gpt-6-astra (one batched refutation call). Model calls: 2.
- Rigor: routine. Risk class: high (irreversible filing to an authority, PII), so the verifier was used.
- Scope: `crates/process/src/file_wage_correction.rs`, `file_wage.rs`, `authority.rs`, `file_employer_report.rs`, `crates/broker/src/tulorekisteri.rs`, `tulorekisteri_xml.rs`, `crates/domain/src/wage_report.rs`, `crates/cli/src/wage.rs`.

No must-fix finding: no path was found that sends a correction twice, sends after an unknown outcome, or sends bytes other than the approved record. Four should-fix findings, each confirmed by reading the code and by the verifier.

## Finding 1: Re-recording unversioned e-service evidence rewrites an existing Run (should-fix, high confidence)

- Location: `crates/process/src/file_wage_correction.rs:805` (version check), `:841` (unconditional `write_state` and journal appends), `:857` (`external_run_id`).
- Claim: recording a replacement from e-service evidence without `--report-version` twice (a retry, or a second e-service replacement) returns success, lands on the same Run id, overwrites its state with a new sequence and appends a second `RunStarted`/`EffectPlanned`/`EffectReconciled`.
- Evidence: local read: the newer-version check is skipped when either version is `None`; `write_state` renames over the file; nothing checks that the Run exists. Verifier: survives (`lib.rs:1054` lease permits existing directories; fold accepts the duplicate events). A cancellation repeat is refused as already cancelled, so this is the replacement case.
- Impact: a filing disappears from the report's history and `run show wc-…` reads `running`; the command reports success.
- Fix: refuse before `write_state` when the Run's state already exists.

## Finding 2: `run show` of an earlier correction Run still says accepted after a later correction (should-fix, high confidence)

- Location: `crates/process/src/authority.rs:170`.
- Claim: `incomes_register_accepted` reports `Superseded` only for workflow `file-wage`; a replacement Run that a later replacement or cancellation superseded still shows `accepted` with its receipt and no `next_command`.
- Evidence: local read; verifier: survives, no other supersession path.
- Impact: the same stale-receipt presentation the change removes for the report's own Run.
- Fix: for a correction Run, report `Superseded` unless it is the latest accepted filing of its report and nothing is in doubt.

## Finding 3: A sent replacement settled by e-service evidence is treated as known content (should-fix, medium confidence)

- Location: `crates/process/src/file_wage_correction.rs:319-329`, settle path `:757-793`.
- Claim: when a sent replacement's outcome is unknown and e-service evidence settles it, the evidence proves a newer version exists, not that it has the sent content. Status still sets `report_is_current = true`.
- Evidence: local read: receipt source is not consulted; verifier: survives.
- Impact: if the user replaced the report in the e-service with other content, the next preview shows the unsent content as the register's, without the warning, and a replacement to that content is refused as unchanged.
- Fix: `report_is_current` is false when an accepted sent replacement's receipt source is `e_service`.

## Finding 4: A replacement does not check the income earner's identity code against the one first sent (should-fix, low-to-medium confidence)

- Location: `crates/process/src/file_wage_correction.rs:422-430`, `:466-473`; `crates/broker/AGENTS.md:81-84`.
- Claim: the replacement check compares `employee_id`, not the personal identity code read fresh from the ledger. After the stored code is changed, a replacement with an otherwise allowed change goes out naming another person under the same `IRReportId`. The new AGENTS.md sentence says the process refuses this before sending.
- Evidence: local read; verifier: survives with the qualification that it does not establish the authority would accept it. The register's schema document says the income earner identifiers "should match that of the report's previous version", which reads as a processing rule the register enforces.
- Impact: at best a loud register rejection; at worst a permanent report naming another person.
- Fix: compare against the first report's outbox `sent_hash` (rebuild the first envelope with the current code); refuse on a difference, warn when the first report was not sent through aggountant.

## Unresolved uncertainty

- The `DeliveryDataType` echoed in processing feedback for a cancellation record is taken to be 105 (the record's own type). The processing-feedback schema document confirms `ItemVersion` = cancelled version + 1 and `IRItemId` = report reference for type 105; it does not show a sample response. If the register echoed another type, an accepted cancellation would be journalled as unknown (never as a second send) and need e-service evidence.
- A correction parked as unknown can be settled by e-service evidence only when the corrected version was recorded; otherwise only the status query settles it. `file-wage employer-report` has the same rule.

## Dropped candidates

None: the finder returned four candidates and all four survived.

## Coverage

Read: the files in scope, the new tests, the vendored XSDs for element order. The reviewers read the code and did not run the tests; the workspace tests and clippy were green before the review. Outside scope: VAT replacement returns (no code in this diff), the employer's separate report cancellation.
