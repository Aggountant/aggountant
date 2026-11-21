# Review: migrate-from-chrono (ADR-9, chrono → jiff)

- **Target:** `git diff 5fcece7..04247ec` (golden tests, migration, deny/AGENTS/ADR removal), 3 100 lines without the golden TSV, `Cargo.lock` and the removed ADR page.
- **Finder:** gpt-6-astra (other provider than the implementing agent). **Verifier:** claude-fable-5-2. **Model calls:** 2.
- **Rigor:** critical. Qualifying reason: timestamps in sealed vouchers, hash chains and authority payloads; a format drift is silently wrong bookkeeping and cannot be undone once written.
- **Risk class:** data integrity (stored and sent formats), parsing of previously written data.

## F1 — should-fix: the XSD conformance test accepts timestamps that are not xs:dateTime

- **Location:** `crates/broker/tests/payer_summary_schema.rs:246` (`builtin("dateTime", v)`).
- **Claim:** the check moved from chrono `DateTime::parse_from_rfc3339` to `Instant::parse` (jiff), which also accepts `2026-10-02T09:30Z` (no seconds), `+03` offsets, `,5` fractions and `[UTC]` annotations. `PayerSummaryReportsToIRTypes.xsd:42` declares `Timestamp` as plain `xs:dateTime`, so this helper is the only gate.
- **Evidence:** finder ran the compiled parser; the verifier built a differential harness (chrono 0.4.45 vs the worktree's `Instant`) and confirmed eight inputs chrono rejected and jiff accepts. `the_validator_rejects_records_the_schema_forbids` had no timestamp case.
- **Impact:** weakened test only; every generated `<Timestamp>` goes through `Instant`'s `Display`, which always writes `HH:MM:SS` (pinned by the golden tests).
- **Fix:** a test-local lexical check `YYYY-MM-DDThh:mm:ss[.f+](Z|±hh:mm)` before `Instant::parse`, plus two rejection cases (no seconds, `+03`).

## Unresolved uncertainty

None.

## Dropped / not findings (2)

- **Leap-second input folds `:60` to `:59`** (verifier, question 2). chrono kept `23:59:60` as a leap second and wrote it back as `:60`; jiff folds it to `:59`. Reachable only if a user typed `--timestamp …:60Z` into `file-wage` or the employer report under the chrono build; no clock or authority value produces it. ADR-9 records the fold as accepted. Not a finding.
- **`Instant::from_unix` refuses nanos ≥ 1e9** (chrono used them for leap seconds). Every caller passes 0. Unreachable.

## Coverage

- Verifier's differential harness, sampled not exhaustive: 13.2 M `(secs, nanos)` pairs 1900–2100 (negative epoch, day boundaries, 19 nano patterns) comparing `Display`, serde, `to_whole_seconds_z`, `Debug`, `unix_seconds` and re-parsing all three chrono forms — zero differences. 10.6 M instants through old vs new `finnish_local_time` / `one_bank_day_after` (20-minute grid plus probes around every switch, year end, holidays) and all 73 413 dates through `start_of_finnish_day`, `is_finnish_bank_day`, `next_finnish_bank_day` — zero differences.
- Host checks: every `Display` use of `Instant` enumerated by compiling with `Display` removed (only intended format-1 sites); golden expected strings unchanged since the chrono commit (`git diff c23cfb4 HEAD -- crates/model/tests/golden` empty; other golden diffs only change type annotations); `scripts/ci-local.sh` green on stable and 1.89, including `cargo deny` without the RUSTSEC-2026-0543 ignore.
- Outside scope: musl and macOS release cross-builds (jiff and jiff-core are pure Rust with no build script; not cross-built here).
