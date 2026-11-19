# ADR-9 rethink: what was taken out

ADR-9 was rewritten on 2026-11-19 so that it can be read and decided on. No
decision changed: jiff 1.x with `std` and `serde` and no time-zone database,
the type mapping, the instant type in `model` that keeps formats 1–3
byte-identical, `model::Date` as the one strict date parser, the hand-written
Finnish summer-time rule, the probe evidence, the migration plan and both
rejected alternatives with their reasons all stay. The original is in git
history (`git log -p -- docs/site/adr-9-date-time-library.md`).

## Cut or shortened

- **Authors line:** the method detail (stable and MSRV builds, crates.io
  metadata, RustSec database, "no panel: the evidence settled it") is reduced
  to the kinds of sources.
- **Reference counts:** 78 references, the per-crate counts and "63 uses" of
  the instant type are cut; "43 files across six crates" stays.
- **chrono API names in the usage list** (`NaiveDate`, `inner()`, `Utc::now()`,
  `to_rfc3339_opts(Secs, true)`, `parse_from_rfc3339`, `FromStr`, "18 files use
  `NaiveDate` directly"): restated facts that live in the code. The usage is
  described by what it does; the mapping table keeps the type names.
- **Exact summer-time rule** (UTC+2/+3, last Sundays of March and October,
  01:00 UTC): lives in `domain::bank_days` and its tests.
- **Probe table:** exact sub-second sample values, the `serde-human-readable`
  feature name, the month counts tested, crate MSRVs (1.85, 1.88), licence
  names and "no build scripts" in the table are cut; the result of every row
  stays. `time` rejecting `+0000` and `:60` is cut since `time` was rejected.
- **"Arithmetic and comparisons go through the inner `Timestamp`"**:
  implementation detail for the slice.
- **Named direct date parses** (`transcript_630`, `file_vat::status`, YTJ's
  `Option<NaiveDate>` fields) and "one strict parser instead of four lenient
  ones": shortened to "the few places that still parse dates directly"; the
  implementer finds them by searching for chrono.
- **`tzdb-bundle-always` feature name** and "under time it would mean a
  third-party tz crate": the feature name is jiff's documentation; the time
  point is already in the rejection of `time`.
- **jiff release history** (46 releases in 0.2, the listed API items) and
  **time's MSRV history** (1.81 → 1.88): shortened to the conclusion.
- **Leniency on input** appeared both in Context and in Consequences; it is
  now stated once, with Consequences pointing back.
- **Migration plan:** "each step builds and passes the full suite before the
  next one starts" is cut as what any implementer does; "remove this ADR page
  and its `glasspad.yaml` entry" and "run `scripts/ci-local.sh`" are cut
  because `docs/AGENTS.md` and the root `AGENTS.md` already require them;
  "`model` has no `AGENTS.md` of its own" is cut as a fact visible in the
  tree. Record type names in step 0 are given as plain descriptions. "Existing
  tests must pass without edits; a changed expected string is a format break"
  is kept, stated as the reason for the order rather than as a rule.
