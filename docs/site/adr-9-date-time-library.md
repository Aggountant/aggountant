# ADR-9 — Date and time library: replace chrono with jiff

- **Status:** Proposed
- **Date:** 2026-11-19
- **Authors:** technical-decision worker for issue `migrate-from-chrono`.
  Throwaway probes compiled against the three crates on stable and on the
  MSRV (1.89), the code read crate by crate, and the crates' changelogs,
  crates.io metadata and the RustSec database. No panel: the evidence
  settled it. Decider: Jari Mustonen.
- **Refines:** no earlier ADR. It is bound by the sealed-voucher invariant in
  `crates/AGENTS.md` (a sealed record is never rewritten) and by the rule that
  authority payloads keep their exact format.

---

## Context

RUSTSEC-2026-0543 (2026-09-14) marks `chrono` unmaintained. Every version is
affected and there is no patched version. `deny.toml` ignores the notice for
now. The advisory recommends `jiff` or `time`.

### What aggountant uses chrono for

There are 78 references in 43 files across six crates: `model` 8, `domain` 20,
`ledger` 5, `broker` 14, `process` 20 and `cli` 11. `wire`, the crate that
seals, does not use chrono. The usage is narrow:

- **Civil dates** (`NaiveDate`): fiscal years, VAT and report periods,
  deadlines, bank days. `model::Date` already wraps one with a strict
  `YYYY-MM-DD` parser (parse, then reformat and compare), but `inner()`
  exposes the chrono type, and 18 files use `NaiveDate` directly. The date
  logic needs construction, weekday, next and previous day, adding days,
  the difference in days, and **adding months** in one place
  (`model::fiscal_year::add_months`, clamped to the end of the month).
- **UTC instants** (`DateTime<Utc>`, 63 uses): `Utc::now()`, epoch-second
  conversion (`Clock`, x509 validity), comparisons, and 24-hour arithmetic.
- **Finnish local time.** `domain::bank_days::finnish_local_time` implements
  the EU summer-time rule by hand (UTC+2, UTC+3 from the last Sunday of March
  to the last Sunday of October, switching at 01:00 UTC). It uses no time-zone
  database, no `chrono-tz` and no `Local`. The `clock` feature still pulls in
  `iana-time-zone`, which we never call.
- **The formats we emit.** These are the bytes that have to stay identical:
  1. `to_rfc3339()`: `2026-09-13T12:00:00.123456789+00:00`. chrono's "AutoSi"
     fraction has 0, 3, 6 or 9 digits, and the offset is written as `+00:00`.
     This format is used for `sealed_at` in voucher provenance, the journal and
     fiscal-store `ts`, the broker outbox `ts`, the Tulorekisteri XML
     `<Timestamp>`, and session expiry.
  2. serde `DateTime<Utc>`: the same AutoSi fraction, ending in `Z`
     (`imported_at` in transcript records, `observed_at` in Vero onboarding
     evidence, `timestamp` in Tulorekisteri requests, certificate metadata).
  3. `to_rfc3339_opts(Secs, true)`: `2026-09-13T12:00:00Z` (the
     authority-check note in VAT recovery).
  4. Dates `%Y-%m-%d` (also the serde form of `NaiveDate`), plus
     `%Y-%m-%d %H:%M` and `%Y` in messages and the Vero `FilingYear`.
- **Parsing:** `parse_from_rfc3339` when reading journals and notes,
  `FromStr` for clap `--timestamp` arguments and serde `Deserialize`, and
  `%Y-%m-%d` for dates (including the YTJ API's `registrationDate`).

All persisted timestamps are stored as **strings**. `sealed_at` is a `String`
that is formatted once, when the record is written. No sealed record is ever
deserialised into a chrono value and serialised again before hashing. A
migration therefore cannot change the hash of an existing record. What it can
change is the format of **newly written** values, and whether old values
still parse.

### What the probes showed

The probes covered 72 instants (epoch, 2000-02-29, present-day values and
2099-12-31) × 12 sub-second values (0, 1 ns, 10 ns, 100 ms, …, 123 456 780 ns,
999 999 999 ns) and compared each candidate's output with chrono 0.4.45's:

| Check | jiff 1.0.1 | time 0.3.55 |
|---|---|---|
| Library's own RFC 3339 output equals chrono's | **no**: fewest digits needed (`.1Z` for chrono's `.100Z`); 36/72 differ | **no**: same minimal-digit behaviour, 36/72 differ; default serde is a tuple `[1970,1,0,…]`, 72/72 differ |
| A small helper (pick 0/3/6/9 digits, write `+00:00` or `Z`) reproduces formats 1–3 | 0 mismatches | 0 mismatches |
| Parses every string chrono wrote (formats 1–3) back to the same instant | yes | yes |
| `YYYY-MM-DD` strict without our reformat check | no (`FromStr` takes `2026-01-05T00:00`; `strptime` takes `2026-1-5`) | no (takes `+2026-01-05`) |
| Date serde output equals chrono's `NaiveDate` | yes | only with `serde-human-readable` |
| Add months, clamped, 1999–2040 × {1,2,3,11,12,13,24} months (107 387 cases) | built in, 0 mismatches | no API; an 8-line hand clamp, 0 mismatches |
| Europe/Helsinki tzdb vs our hand-written rule, 1997–2037, every 30 min ±1 s (2.16 M instants) | 0 mismatches | no time-zone support |
| Builds and runs on Rust 1.89 | yes (MSRV 1.85) | yes (MSRV 1.88) |
| `cargo deny check licenses advisories` with our `deny.toml` | pass (Unlicense OR MIT) | pass (MIT OR Apache-2.0) |
| New crates in our graph | `jiff`, `jiff-core` (no build scripts, no platform code with default features off) | **none**: `time` is already present through `x509-parser` and pinned `>=0.3.47` in `broker` |

Parser leniency for input (clap, serde) differs from chrono's at the edges.
jiff also accepts `12:00Z` without seconds and `+03` offsets, rejects more
than 9 fractional digits, and folds a leap second `:60` to `:59`. time rejects
`+0000` (which chrono's `FromStr` accepted) and `:60`. aggountant never writes
any of these, so stored data reads back either way.

Neither crate reproduces our bytes on its own, and both reproduce them
exactly with the same small helper. The serialisation lens therefore does not
separate them. What separates them is API fit and the failure modes when that
helper is forgotten.

## Decision

**Replace chrono with `jiff` 1.x** (`default-features = false`,
`features = ["std", "serde"]`, minimum `1.0.1`). No time-zone database is
enabled. Type mapping:

| chrono | jiff |
|---|---|
| `NaiveDate` | `jiff::civil::Date` |
| `DateTime<Utc>` | `jiff::Timestamp` |
| `NaiveDateTime` | `jiff::civil::DateTime` |
| `Duration` on instants | `jiff::SignedDuration` (24-hour days stay explicit as `from_hours(24 * n)`) |
| `Duration::days` / `succ_opt` / `pred_opt` on dates | `Date::checked_add(Span)` / `tomorrow()` / `yesterday()` |
| `checked_add_months(Months)` | `Date::checked_add(Span::new().months(n))` (same end-of-month clamp) |

The rules that make "byte-identical" hold live in **one place in `model`**:

1. **An instant newtype in `model`** wraps `jiff::Timestamp`. Its `Display`
   is format 1 (AutoSi with `+00:00`) and its `Serialize` is format 2
   (AutoSi with `Z`). It has a named method for format 3. `FromStr` and
   `Deserialize` accept RFC 3339 and normalise to UTC. Every `DateTime<Utc>`
   field and every `to_rfc3339` call site moves to it, so nothing can reach
   jiff's own minimal-digit `Display` or serde by accident. Arithmetic and
   comparisons go through the inner `Timestamp`.
2. **`model::Date` stays the strict date type.** Its inner type becomes
   `jiff::civil::Date`, and it keeps the reformat-and-compare check, because
   neither candidate's parser is strict on its own. The remaining direct date
   parses (`transcript_630`, `file_vat::status`, YTJ's `Option<NaiveDate>`
   fields) switch to `model::Date::parse` or a serde path through it, so one
   strict parser remains instead of four lenient ones.
3. **Finnish local time stays the hand-written EU rule** in
   `domain::bank_days`. It matches the tzdb exactly for 1997–2037, needs no
   tzdb on a static musl binary, and is pinned by its own tests. If the rule
   ever changes in law, jiff can supply Europe/Helsinki from a bundled tzdb
   (`tzdb-bundle-always`) as a contained change. Under time it would mean a
   third-party tz crate.

## Consequences

- RUSTSEC-2026-0543 leaves `deny.toml`. `chrono` and `iana-time-zone` (with
  its platform crates, such as CoreFoundation bindings on macOS) leave the
  graph. `jiff` and `jiff-core` come in: pure Rust with no build script, so
  the musl and macOS arm64 cargo-dist builds are unaffected. The musl targets
  were not cross-built in this run (only the host target is installed). This
  is inferred from the crates having no build script and, with the default
  features off, no platform-specific code.
- `time` stays in the graph regardless, through `x509-parser`. Our own code
  does not use it, and `broker`'s `time = "0.3.47"` line remains only as the
  RUSTSEC-2026-0009 floor.
- The 1.x line is new: 1.0.0 shipped on 2026-11-15, and 1.0.1 on 2026-11-18
  fixed a timestamp-parsing bounds bug. The API this migration uses (`Date`,
  `Timestamp`, `SignedDuration`, `Span`, `strtime`, `DateTimePrinter`) was in
  the 0.2 line for its 46 releases. The risk is ordinary patch churn, not
  design churn. Cargo.lock pins the version and Dependabot proposes patches.
- jiff's leap-second folding and wider input acceptance apply to clap
  `--timestamp` and serde input. That makes acceptance wider, never narrower,
  for anything chrono accepted except more than 9 fractional digits.

### Rejected: `time`

Its one concrete advantage is real: zero new crates, and its maintenance and
MSRV already bind us through `x509-parser`. It lost on the rest:

- **Silent-format failure mode.** The default serde of its date-time types is a
  numeric tuple, and `Display` prints `2026-09-10 0:26:40.0 +00`. A field
  that misses its `#[serde(with = …)]` or a stray `format!("{ts}")` emits
  something no parser of ours reads, and the compiler says nothing. jiff's
  defaults are at least valid RFC 3339 and ISO 8601. The newtype guards both
  candidates, but this project's worst failure is silently wrong bookkeeping,
  so a safer default is worth two small crates.
- **Missing pieces we use.** It has no month arithmetic (fiscal years need it,
  so we would write a hand clamp) and no `FromStr` (every clap timestamp
  argument would need a `value_parser` function). It needs four feature flags
  (`formatting`, `parsing`, `macros`, `serde`) and compile-time format
  descriptions for what is now one-line parsing. It has no time-zone path at
  all if the Finnish rule ever has to come from a tzdb.
- **Stability signal.** It has been at 0.3 since 2021 with no 1.0 commitment,
  and its MSRV moves in patch releases (1.81 → 1.88 within 0.3.4x). We are
  exposed to that already through `x509-parser`. Choosing it as our own API
  would not add the exposure, but it does not argue for it either.

### Rejected: stay on chrono (the null option)

chrono 0.4.45 works today, and our usage avoids its historical problem area
(local time zones). The cost of staying is that no one fixes the next bug,
toolchain break or advisory. A future unmaintained-crate advisory becomes a
second permanent ignore in `deny.toml`. Every new date feature (and dates are
the core of periods and deadlines) gets written against a dead API. The
migration is bounded (43 files, one formatting helper, probes showing exact
byte reproduction), so deferring it only makes it larger.

No third candidate was found. `hifitime` targets scientific time scales, and
I found no maintained fork of `chrono`.

## Migration plan (sketch)

One slice, in dependency order. Each step builds and passes the full suite
before the next one starts.

0. **Golden tests first, on chrono.** Add a table test in `model` that pins
   formats 1–3 and dates for the probe's instant × sub-second matrix to
   literal strings. Add round-trip tests for the serde record types that carry
   instants (`TranscriptImportRecord`, `TestVerificationEvidence`,
   `PortalVerificationEvidence`, `WageRequest`, `EmployerReportRequest`): the
   literal JSON written by the chrono build deserialises and serialises back
   to the same bytes. Commit these before any library change so they prove
   the migration rather than describe it.
1. **`model`:** the instant newtype, `Date` on `jiff::civil::Date`, and
   `fiscal_year` month arithmetic. The golden tests pass unchanged.
2. **`domain`:** `bank_days` (keeps the EU rule; its tests pass unchanged) and
   the wage-report date validation.
3. **`ledger`:** `sealed_at` and fiscal-store `ts` through the newtype. The
   existing seal and hash-chain tests, plus a test that opens a fiscal store
   written by the chrono build and verifies its chain, cover the invariant.
4. **`broker`:** the Tulorekisteri XML `<Timestamp>` (covered by the
   payer-summary XSD test and the signed XML fixtures), Vero filing evidence and certificate validity, the YTJ
   dates, and the outbox `ts`.
5. **`process`:** the journal `ts`, VAT recovery notes (format 3), transcript
   630, employer-report history, and session expiry. Journals written by the
   chrono build replay unchanged.
6. **`cli`:** clap `--timestamp` and `--as-of` parsing through the newtype and
   `model::Date`, and `FilingYear`.
7. Remove `chrono` from the workspace and the RUSTSEC-2026-0543 ignore from
   `deny.toml`, run `scripts/ci-local.sh` (stable and 1.89, cargo-deny), and
   remove this ADR page and its `glasspad.yaml` entry. Move the one rule the
   code cannot show ("instants are written in chrono's AutoSi RFC 3339 form;
   never use the library's own `Display` or serde for a persisted or sent
   value") into `crates/AGENTS.md` (`model` has no `AGENTS.md` of its own).

Existing tests must pass without edits to their expected strings. A test
whose expected bytes change is a format break, not a test to update.
