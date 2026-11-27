# Review: wage-report-multiple-allowance-codes

- Skill: `/llm-review`, rigor **critical**: the change alters the record sent
  to the Incomes Register and the serialized form of the report whose hash
  binds an approval. A changed single-code record or hash would strand open
  runs or send something other than what was approved.
- Scope: commit `6ec2461` against `main` (`crates/domain/src/wage_report.rs`,
  `crates/broker/src/tulorekisteri_xml.rs`, tests, `input_help.rs`, guide,
  skill, CHANGELOG), with `crates/process/src/file_wage.rs`,
  `crates/broker/src/tulorekisteri.rs` and the vendored
  `WageReportsToIRTypes.xsd` attached.
- Finder: `gpt-6-astra`, one call (diff plus the requirements: XSD shape,
  legacy input, byte identity of single-code JSON and XML, code validation).
  Verifier: `claude-fable-5-2`, one focused false-negative call on the
  persisted-state boundary, because the finder came back clean on a
  high-risk surface. Two model calls in total.
- Risk class: authority filing output and approval binding.

## Findings

No confirmed findings.

The finder answered "No findings." The verifier answered "No such path" for
both directions asked (state written by `main` read and re-serialized by the
branch; state written by the branch round-tripped by the branch). It built
probe binaries against `main` and the branch and compared, over 75
legacy-shaped reports, the `proposal_hash` preimage, the broker envelope body
behind `sent_hash`, `wage.json`-shaped state, `correction_hash` and the XML
behind `record_hash`: all byte-identical. 284 branch-shaped reports (every
permutation of subsets of 1–4, duplicates, empty, out of range) round-tripped
to identical bytes, and the same set in any order gave the same bytes.

## Observations next to the question (not defects)

- Downgrade: a `main` build refuses state holding a multi-type line
  (`allowance_codes` is unknown to it). One-type lines stay readable by
  `main`. Recorded in the result; the conductor installs forward only.
- The process-level golden test `proposal_hash_is_reproducible` pinned a
  311-only report. Taken: it now also pins a 331 line in both input shapes
  against the hash computed by the `main` build
  (`ed9edb7d…a76d`).

## Dropped candidates

None (the finder proposed none).

## Coverage

Read and tested locally: the serde boundary (`WireLine` try_from/into,
`present` null refusal, `deny_unknown_fields`), validation of codes, the XML
writer, XSD validation of a new record and a replacement with codes 2 and 3,
the CLI preview and its refusals, and a cross-build check: a home created and
a run started by the `main` binary, then preview (byte-identical output
including `record_hash`), `file-wage start` idempotency (accepted unchanged;
a changed amount still refused) and `wage.json` unchanged under the branch
binary. Not exercised: a live submission to the register (no test
connection in this slice); the record is checked against the vendored XSD.
