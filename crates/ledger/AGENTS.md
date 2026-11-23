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
  sealed vouchers under the same version. Adding a reverse-charge target for a
  scope the chart had none for reinterprets nothing and is the one exception
  (see the upgrade rule below). The rule protects filed figures and
  VAT treatment, so it governs the roles those come from
  (`vat-account-roles.json`). The settlement roles in
  `settlement-account-roles.json` (bank and tax accounts) are detection
  evidence for the VAT reconciliation only: they enter no figure and no
  treatment, and were added to the already used `fi-general-2026` edition on
  purpose. Adding them makes the reconciliation of earlier periods stricter,
  so `file-vat preview` of a filed period can now show an untagged correction
  it used to take for a payment; that is the intended discovery. A home without
  the file opens, and the reconciliation then fails closed: no payment is a
  settlement. Never fall back to recognising payments by voucher shape alone.
- **Existing homes take in new masterdata only by addition.** `home upgrade`
  creates missing bundled snapshot and role files and never rewrites the active
  chart or a file the company may have approved, because that masterdata is
  what its sealed vouchers resolve against. A binary that needs something an
  older home cannot get by addition needs a decision, not a rewrite in the
  upgrade. So new kinds of masterdata go into a new file (as the settlement
  roles did), never as a new field of a file every home already has.
  The one sanctioned in-place change: a bundled `vat-account-roles.json` may
  gain a reverse-charge target for a scope its chart had none for (no voucher
  can have been sealed with that scope, so nothing is reinterpreted; the
  `eu-goods` target on `fi-general-2026` came this way). Whoever changes the
  bundled role bytes copies the previous file, byte for byte as `init` wrote
  it, into `data/retired-vat-account-roles/` and lists it in
  `RETIRED_VAT_ROLE_FILES`; the upgrade then replaces a home's file only while
  it is exactly such a retired file and the bundled table passes
  `only_adds_reverse_charge_targets_to`. Any other change to the bundled roles
  is a new chart edition. Without the retired entry every existing home would
  report the file `differs` and never get the target.

## Bank-statement imports

- **A bank transaction is booked once, whatever export it arrives in.** Its
  event key is the voucher's natural key (bank reference and account). The
  statement file stays the voucher's attachment and part of its request hash,
  but whether a second sighting books the same thing is decided only by the
  source fingerprint and the projection hash (`bank_event.rs`); a match records
  an observation of the new file, anything else stops the batch. Never relax a
  conflict into a replay or a second voucher: a changed bank fact or booking is
  for the user to resolve, by a correction voucher if the books were wrong.
- **The batch is the import's only write path** and one transaction: no
  voucher of a statement is sealed unless every transaction resolves.
- **Vouchers sealed before bank events have their identity read back** from
  the statement they retained (the CLI parses it, since the ledger cannot
  depend on `import`) and backfilled in the next import's transaction. One
  whose transaction cannot be read back is never compared by guess and never
  booked again: a batch that meets its key stops with
  `SOURCE_IDENTITY_UNKNOWN`.
- **A voucher sealed outside the import may already book a new entry.**
  Generic `voucher seal` leaves no event key, so before sealing a new entry
  the batch searches every voucher not bound to a bank event (and not under
  the import's own key prefix) for a bank reference in its natural key or
  description, or the same amount on the same side of the same bank account
  within `DATE_TOLERANCE_DAYS`, and stops on any candidate
  (`BANK_ENTRY_POSSIBLY_BOOKED`). A false stop is the intended price; do not
  narrow the match to avoid one. The user resolves it explicitly: a binding
  (`BankEventBooking::Existing`) whose voucher's bank leg passes the same
  match, or `not_booked_by` naming the voucher. A binding writes an extra
  `idempotency` row for the event key pointing at the existing voucher plus
  the `bank_event`, so nothing can seal that key again; the voucher and its
  hash are untouched and `voucher verify` does not read `idempotency`. One
  voucher binds at most one event (`bank_event.voucher_number` is unique).

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
- **Which VAT return a correction belongs to is sealed into the voucher.**
  `corrects_vat_period` on a voucher posted in a later period puts its VAT in
  the named period's return and takes it out of the return of the period it is
  posted in; every VAT derivation selects vouchers through
  `VoucherBody::in_vat_period`, never by posting date alone, or the amount
  would be reported twice or not at all. It is part of the hashed body, set
  only at the seal and only for a period whose return is filed, and there is
  deliberately no way to attribute a voucher afterwards: by then the posting
  period's own return may have counted it. A voucher without it stays in its
  posting period's return, which is also the lawful route for a financially
  minor error. Bookkeeping reports and statements go by posting date and do
  not read it.
- **A period's VAT filings are a sequence, and the last is in force.** The
  first return and each replacement are `Sealed→Sealed` events of the period's
  transition chain; nothing else may follow a filing. A replacement is taken
  with `ReplacementAuthority`, which names how many filings it replaces and is
  checked in the CAS, so the first-filing path can never record a second
  return and a replacement prepared against an older state is refused. Earlier
  receipts are never overwritten. A binary older than replacement returns
  reports a home with two filing events as `HASH_CHAIN_BROKEN`; binaries only
  move forward.
- **Period state is authoritative in fiscal and co-transacted with the seal.**
  `Open → Closing → Sealed` transitions are CAS updates of the period row under
  the same writer lock as `voucher seal`, so no voucher can slip into a period
  the close believes is `Closing`. Each transition appends to a hash-chained
  transition log that `voucher verify` walks. The process-side Run record is
  reconciled to fiscal after a crash, never the other way round.
- **A rejected close returns the period with `Closing → Open`.** Only the run
  that owns the Closing period takes it, with its `ClosingAuthority`, fenced in
  the CAS like the seal. Nothing in the period is discarded: vouchers sealed
  while Closing stay sealed in the Open period. A binary older than this edge
  reports a home containing it as `HASH_CHAIN_BROKEN`; binaries only move
  forward.
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
- **A year opens from an unsealed prior year only at a boundary cut-over.**
  `fiscal_year_data` requires the prior fiscal year to be Sealed before it
  carries its closing position forward, because an open year can still change.
  The one exception is a company that moves to aggountant at the turn of its
  fiscal year: when every prior voucher posts balance-sheet accounts only on
  the prior year's last day and no period up to then was closed or filed here,
  those vouchers become an `ExternalCutover` opening balance. Do not widen the
  test (to other dates, result accounts or kept years) without a decision;
  that would let a real year's activity reach the next year unfrozen.
- **The fixed-asset register is part of the year seal.** The assets a close
  depreciated or carried, with their opening and closing book values, are
  written to the fiscal store in the transaction that seals the year, one row
  per year, and never afterwards. The next close opens each carried asset at
  that closing value, and `seal_year` refuses a register that does not
  continue the previous year's, so a carried asset's opening book value is
  never typed, and a retyped one that disagrees cannot reach a sealed year. A
  value is typed only when an asset enters: a new acquisition, the first
  close of a home, or an asset removed for a year (depreciated by hand, or a
  year that is not 12 months) and given again; nothing checks that last one
  against the year it left. It lives
  here and not in masterdata, company knowledge or `archive/<year>/` because
  it decides the next year's depreciation: the archive holds a copy for the
  reader and is never read back. No row means "no register" (a year sealed
  before registers, or kept in another system), and the close after it starts
  one from its input; an empty register is a row. The register is not in the
  year's transition hash, so `voucher verify` does not cover it; binding it
  there changes the hash preimage and needs a decision.
- **Nothing enters a year at or before a Sealed fiscal year.** A sealed year
  has carried forward the closing position of every earlier year, including a
  year kept in the old system that was never sealed here. So the seal refuses
  a voucher dated in or before a Sealed year, period transitions refuse
  periods of such years, and attribution refuses a target year at or before
  one. Without this a backdated voucher would change the next year's opening
  silently, or turn a sealed year's cut-over into a refusal.
