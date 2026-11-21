# Result

chrono is gone. ADR-9 is built: dates and times run on `jiff` 1.0.1
(default features off, `std` and `serde`, no time-zone database), and every
stored or sent timestamp keeps the bytes the chrono build wrote.

- `model::Instant` wraps `jiff::Timestamp`. `Display` writes
  `2026-09-13T12:00:00.123+00:00`, serde writes the same with `Z`,
  `to_whole_seconds_z` writes `2026-09-13T12:00:00Z`; fractions have 0, 3, 6
  or 9 digits as chrono's did. It reads any RFC 3339 string as UTC.
  `sealed_at`, fiscal-store and journal `ts`, the outbox, the Tulorekisteri
  XML `<Timestamp>`, request JSON, Vero evidence, certificate validity, the
  VAT authority-check note and the run lease all go through it.
- `model::Date` wraps `jiff::civil::Date` and is the one strict
  `YYYY-MM-DD` parser. YTJ register dates, transcript 630 dates, the receipt
  date check and `--as-of` now parse through it, so `--as-of 2026-1-5` is
  refused where chrono accepted it.
- `PeriodId::last_date` replaces two copies of the period-end computation
  (Vero filing period, GetVATPeriods preflight).
- Finnish local time is still the hand-written EU rule in
  `domain::bank_days`.
- Certificate messages keep chrono's `2026-09-13 12:00:00 UTC` text.
- `deny.toml` no longer ignores RUSTSEC-2026-0543; `chrono` and
  `iana-time-zone` left `Cargo.lock`, `jiff` and `jiff-core` came in.
- The rule the code cannot show is in `crates/AGENTS.md` under "Dates and
  times". The ADR-9 page and its manifest entry are removed.

Proof that formats did not move: the golden tests were committed on chrono
first (`c23cfb4`) and their expected strings are unchanged since then:
formats 1–3 and read-back over 72 instants × 12 sub-second values
(`crates/model/tests/golden/instants.tsv`), strict dates, Finnish local time,
the employer-report delivery reference and wage proposal hash for whole and
fractional instants, Tulorekisteri request JSON and XML, Vero evidence,
certificate validity, transcript import records, register dates, the
authority-check note, `sealed_at`'s form, `FilingYear`. Every existing test
also passes unchanged, including the home written by an older chrono build
(`fiscal_year_cli`, which opens it, verifies the chain and seals on) and the
signed Tulorekisteri XML fixtures. `scripts/ci-local.sh` is green on stable
and 1.89.

Input is slightly more lenient where ADR-9 says so (for example `12:00Z`
without seconds in `--timestamp`); a leap second `:60` now reads as `:59`.
Neither occurs in data aggountant writes.

Not done here: the musl and macOS release targets were not cross-built;
jiff is pure Rust with no build script, so the release build should not be
affected, but the first release after this change is the real check.

Simplicity: one instant type and one date parser carry every format rule;
no compatibility layer for chrono was kept. The time-zone database feature
was left out deliberately.

Review and assessment: `review.md`, `assessment.md` (one finding, fixed: the
XSD conformance test had become lenient about `xs:dateTime`).
