# crates/ledger — rules the code does not show

Read `crates/AGENTS.md` first. This crate is the only write path to the books,
so its rules are the ones whose violation produces silently wrong bookkeeping.

## Where the truth lives

- **Fiscal SQLite is the single authority** for the gapless voucher counter, the
  chain head, the sealed canonical voucher bytes, the rejection log and the
  period state. Every read surface is a derived projection: the planned git
  mirror (its outbox is already enqueued in the seal transaction; the projector
  is [ADR-3](../../docs/site/adr-3-fiscal-store.md) work), the audit archive and
  the analytics replica are never consulted for numbering, reads, correctness or
  recovery, and when one diverges it is rebuilt from SQLite. Do not split
  authority ("SQLite owns the number, git owns the content"): that is a
  distributed transaction with no atomic commit, and every crash would leave an
  ambiguous state.
- **One `BEGIN IMMEDIATE` transaction** does check-idempotency → validate period
  and authority → allocate number → build canonical wire → insert voucher →
  advance counter and chain head with compare-and-swap → record idempotency.
  Success is returned only after `COMMIT`. A number is never reserved outside
  that transaction, which is what keeps numbering gapless across crashes. The
  `UNIQUE` constraints and CAS predicates are there to turn a logic bug into a
  loud failure; do not relax them to make a test pass.
- **The data home must be on a local filesystem.** SQLite WAL locking and
  `fsync` guarantees do not hold on NFS, SMB, cloud-synced or network-mounted
  paths. Never back up by copying the database file while it is open; use
  SQLite's backup API or copy a home nothing is using.
- **Masterdata is split by data-protection class.** Reference data (chart of
  accounts, account kinds, content-hashed `chart_version`) is plaintext git,
  diffable and reviewable. Personal data (HETU, IBAN, tax cards, employee
  identity) lives in a separate encrypted SQLite store so it stays erasable;
  it never goes into git or the fiscal store. Artifacts are encrypted,
  content-addressed files, not git and not SQLite blobs.
- **Chart role metadata is frozen once a voucher is sealed against its
  `chart_version`.** A role change is a new chart edition; never reinterpret old
  sealed vouchers under the same version.

## Refusals

- **Never return `Refused` unless the rejection record committed.** If writing the
  audit record itself fails, the outcome is `AuditUnavailable`, never a refusal
  that left no trace.
- The rejection log stores no raw personal data or secrets: request hash, reason
  code, a safe actor id, the target, a timestamp and the previous rejection
  hash (the log is itself hash-chained).

## Sealed means immutable

- A sealed voucher is never edited. An error found after sealing is fixed with a
  new, linked correction voucher through the same seal path, and in the user
  surface it is called a correction, never an edit. (The dedicated
  `voucher correct` verb is still to be built under
  [ADR-5](../../docs/site/adr-5-seal-timing.md).) If the affected period's
  VAT return was already filed, the correction also needs a VAT replacement
  return; correcting the books does not correct the filing.
- **Period state is authoritative in fiscal and co-transacted with the seal.**
  `Open → Closing → Sealed` transitions are CAS updates of the period row under
  the same writer lock as `voucher seal`, so no voucher can slip into a period
  the close believes is `Closing`. Each transition appends to a hash-chained
  transition log that `voucher verify` walks. The process-side Run record is
  reconciled to fiscal after a crash, never the other way round.
- **Reopen is the one exception and only before the VAT return is filed.** The
  `Sealed → Open/Closing` edge requires `ClosingAuthority`, is a logged CAS
  transition, and is refused once the period's return is filed; from then on
  the only path is a correction voucher in the next open period.

## Fiscal years

- **Length is set by kirjanpitolaki 1:4** (1336/1997, Finlex
  <https://www.finlex.fi/fi/lainsaadanto/1997/1336>): "Tilikausi on 12
  kuukautta. Toimintaa aloitettaessa tai lopetettaessa taikka tilinpäätöksen
  ajankohtaa muutettaessa tilikausi saa olla tätä lyhyempi tai pitempi,
  kuitenkin enintään 18 kuukautta." There is no minimum; aggountant accepts
  1–18 whole months and cannot check the reason, so it does not try.
- **A fiscal year is labelled by the calendar year it ends in**, and that `i32`
  is what the year row, transition log, attributions, opening sources and
  statements store. A calendar year's label is the year itself, which is why
  homes from before fiscal-year definitions needed no migration. Labels must
  stay unique and increasing, so two fiscal years ending in one calendar year
  are refused rather than numbered; labels can skip a value after an 18-month
  year, so the previous year comes from the calendar, never `year - 1`.
- **The definitions live in the fiscal store, not in git masterdata.** They
  decide which sealed vouchers a sealed year covers, so they are written under
  the seal's writer lock, append-only, and a definition that would move a
  sealed voucher to another year or change the dates of a year with recorded
  state is refused. Map a date to its year only through `FiscalCalendar`
  (`model`); never derive a fiscal year from `PeriodId::year()` or a date's
  calendar year. Monthly and quarterly periods, VAT periods and the payroll tax
  year stay calendar-based by law.
