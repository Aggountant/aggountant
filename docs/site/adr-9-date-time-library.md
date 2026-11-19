# ADR-9 — Date and time library: replace chrono with jiff

- **Status:** Proposed
- **Date:** 2026-11-19
- **Authors:** technical-decision worker for issue `migrate-from-chrono`, from
  probes against the candidate crates, a crate-by-crate reading of our code,
  and the crates' changelogs and advisories. Decider: Jari Mustonen.
- **Refines:** no earlier ADR. It is bound by the sealed-voucher invariant in
  `crates/AGENTS.md` and by the rule that authority payloads keep their exact
  format.

---

## Context

RUSTSEC-2026-0543 (2026-09-14) marks `chrono` unmaintained, every version,
with no fix coming. `deny.toml` ignores the notice for now. The advisory
recommends `jiff` or `time`.

### What we use chrono for

chrono appears in 43 files across six crates (`model`, `domain`, `ledger`,
`broker`, `process`, `cli`). `wire`, the crate that seals, does not use it.
The use is narrow:

- **Calendar dates** for fiscal years, VAT and report periods, deadlines and
  bank days: building a date, weekday, next and previous day, adding days,
  the difference in days, and in one place adding months clamped to the end
  of the month (`model::fiscal_year`). `model::Date` already wraps chrono's
  date with a strict `YYYY-MM-DD` parser, but many files still use chrono's
  date type directly.
- **UTC instants**: the current time, epoch seconds (the clock, certificate
  validity), comparisons and 24-hour arithmetic.
- **Finnish local time**, which `domain::bank_days` computes with a
  hand-written EU summer-time rule. No time-zone database is used.

The part that matters is the **text we write**. Timestamps are stored and sent
as strings, and these formats must stay byte-for-byte the same:

1. `2026-09-13T12:00:00.123456789+00:00`: chrono's RFC 3339 output, with 0, 3,
   6 or 9 fractional digits and the offset as `+00:00`. Used for `sealed_at`
   in voucher provenance, journal and fiscal-store `ts`, the broker outbox,
   the Tulorekisteri XML `<Timestamp>` and session expiry.
2. The same with `Z` instead of `+00:00`: chrono's serde form, used in
   transcript import records, Vero onboarding evidence, Tulorekisteri request
   JSON and certificate metadata.
3. `2026-09-13T12:00:00Z`, whole seconds: the authority-check note in VAT
   recovery.
4. Dates as `YYYY-MM-DD`, plus `%Y-%m-%d %H:%M` and `%Y` in messages and the
   Vero `FilingYear`.

We also parse: RFC 3339 when reading journals and notes, `--timestamp`
arguments, serde input, and dates (including YTJ's `registrationDate`).

A sealed value such as `sealed_at` is formatted once, when the record is
written, and never read back into a chrono value and reformatted before
hashing. So a migration cannot change the hash of an existing record. What it
can change is how **new** values are written and whether old ones still
parse.

### What the probes showed

The probes compared each candidate with chrono 0.4.45 on 72 instants (from
the epoch through 2000-02-29 to 2099-12-31) × 12 sub-second values:

| Check | jiff 1.0.1 | time 0.3.55 |
|---|---|---|
| Library's own RFC 3339 output equals chrono's | **no**: minimal digits (`.1Z` for chrono's `.100Z`), 36/72 differ | **no**: same, 36/72 differ; default serde is a numeric tuple, 72/72 differ |
| A small helper (pick 0/3/6/9 digits, write `+00:00` or `Z`) reproduces formats 1–3 | 0 mismatches | 0 mismatches |
| Reads back every string chrono wrote to the same instant | yes | yes |
| Strict `YYYY-MM-DD` without our own check | no (accepts `2026-01-05T00:00`, `2026-1-5`) | no (accepts `+2026-01-05`) |
| Date serde equals chrono's | yes | only with an extra feature |
| Add months with end-of-month clamp, 107 387 cases | built in, 0 mismatches | no API; 8-line hand clamp, 0 mismatches |
| Europe/Helsinki time-zone data vs our hand-written rule, 1997–2037 every 30 min (2.16 M instants) | 0 mismatches | no time-zone support |
| Builds on our MSRV 1.89 | yes | yes |
| `cargo deny` licenses and advisories | pass | pass |
| New crates in our dependency graph | `jiff`, `jiff-core` | none: `time` is already there via `x509-parser` |

On input the candidates are more lenient than chrono at the edges: jiff also
accepts `12:00Z` without seconds and `+03` offsets, folds a leap second `:60`
to `:59`, and rejects more than 9 fractional digits. aggountant never writes
any of these, so stored data reads back with either.

Neither library reproduces our bytes by itself, and both do exactly with the
same small helper. Formats therefore do not separate them. What does is how
well the API fits our use and what happens when someone bypasses the helper.

## Decision

**Replace chrono with `jiff` 1.x** (at least 1.0.1, default features off,
features `std` and `serde`, no time-zone database).

| chrono | jiff |
|---|---|
| `NaiveDate` | `jiff::civil::Date` |
| `DateTime<Utc>` | `jiff::Timestamp` |
| `NaiveDateTime` | `jiff::civil::DateTime` |
| `Duration` on instants | `jiff::SignedDuration` (a day stays an explicit 24 hours) |
| adding days, next and previous day | `Date::checked_add(Span)`, `tomorrow()`, `yesterday()` |
| `checked_add_months` | `Date::checked_add(Span::new().months(n))`, same clamp |

Byte-identical output is kept in one place, in `model`:

1. **An instant type in `model`** wraps `jiff::Timestamp` and writes formats 1
   (`Display`), 2 (serde) and 3 (a named method), and reads any RFC 3339
   string as UTC. Every chrono instant and every RFC 3339 formatting call
   moves to it, so jiff's own minimal-digit output never reaches a stored or
   sent value.
2. **`model::Date` is the one strict date parser.** It wraps
   `jiff::civil::Date` and keeps its parse-then-reformat check, because
   neither library is strict alone. The few places that still parse dates
   directly move to it.
3. **Finnish local time stays the hand-written EU rule.** It matches the
   time-zone database exactly for 1997–2037 and needs no database in a static
   musl binary. If the law changes, jiff can bundle Europe/Helsinki as a
   contained change.

## Consequences

- The RUSTSEC-2026-0543 ignore leaves `deny.toml`. `chrono` and
  `iana-time-zone` (pulled in by chrono but never called, with its platform
  crates) leave the graph. `jiff` and `jiff-core` come in: pure Rust with no
  build script, so the musl and macOS release builds should be unaffected.
  The musl targets were not cross-built during the probes; this is inferred.
- `time` stays in the graph through `x509-parser`. `broker` keeps its
  `time = "0.3.47"` line only as the RUSTSEC-2026-0009 floor.
- jiff 1.0 is four days old (1.0.1 fixed a parsing bounds bug), but the API we
  use was stable through the long 0.2 line. The risk is patch churn, which
  `Cargo.lock` and Dependabot handle.
- `--timestamp` and serde input accept slightly more than before (see the
  leniency note above); the one narrowing is more than 9 fractional digits,
  which nothing writes.

### Rejected: `time`

Its real advantage is that it adds no crates; we already depend on it through
`x509-parser`. It lost on the rest:

- **Its defaults fail silently.** Default serde is a numeric tuple and
  `Display` prints `2026-09-10 0:26:40.0 +00`. A field that misses its serde
  attribute or a stray `format!` writes something none of our parsers read,
  and the compiler does not notice. jiff's defaults are at least valid
  RFC 3339. The instant type guards against this with either library, but
  silently wrong bookkeeping is this project's worst failure, so the safer
  default is worth two small crates.
- **It lacks things we use.** No month arithmetic (we would hand-write the
  clamp), no `FromStr` (every `--timestamp` argument needs its own parser),
  more feature flags and format descriptions for what is one line in jiff,
  and no time-zone path if the Finnish rule ever has to come from a database.
- **Stability.** It has been 0.3 since 2021 with no 1.0 plan, and its MSRV
  moves in patch releases. We already live with that through `x509-parser`,
  but it does not argue for choosing it.

### Rejected: stay on chrono

chrono works today and our use avoids its old trouble spot, local time
zones. But no one will fix its next bug, toolchain break or advisory; the
`deny.toml` ignore becomes permanent and every new date feature is written
against a dead API. The migration is bounded and proven possible by the
probes, so waiting only makes it larger.

No third candidate was found: `hifitime` targets scientific time scales, and no
maintained chrono fork was found.

## Migration plan (sketch)

One slice, crate by crate in dependency order. The point of the order is that
existing tests keep their expected strings: a test whose expected bytes change
has found a format break.

0. **Golden tests first, still on chrono**, committed before any library
   change so they prove the migration rather than describe it: formats 1–3
   and dates pinned as literal strings over the probe's instant × sub-second
   matrix, and round trips for the serde records that carry instants
   (transcript import, Vero test and portal verification evidence,
   Tulorekisteri wage and employer-report requests).
1. **`model`**: the instant type, `model::Date` on jiff, fiscal-year month
   arithmetic.
2. **`domain`**: `bank_days` (keeping the EU rule) and wage-report date
   checks.
3. **`ledger`**: `sealed_at` and fiscal-store `ts`, plus a test that opens a
   fiscal store written by the chrono build and verifies its chain.
4. **`broker`**: the Tulorekisteri `<Timestamp>` (covered by the XSD test and
   signed XML fixtures), Vero evidence, certificate validity, YTJ dates and
   the outbox.
5. **`process`**: journal `ts`, VAT recovery notes, transcript 630,
   employer-report history and session expiry; journals written by the chrono
   build replay unchanged.
6. **`cli`**: `--timestamp`, `--as-of` and `FilingYear`.
7. Remove chrono and the `deny.toml` ignore. The rule the code cannot show
   goes into `crates/AGENTS.md`: instants are written in chrono's RFC 3339
   form through the `model` type, never through the library's own `Display`
   or serde.
