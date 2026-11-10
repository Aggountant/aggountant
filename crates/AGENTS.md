# crates/ — rules the code does not show

The crates are the description of what aggountant does. This file and the
`AGENTS.md` in each crate hold only what reading the code would not tell you:
the boundaries a change must not cross and the reasons for them. They replace
the architecture, decision and ADR pages that were written while the system was
designed; those pages predate this repository's public history.

Many code comments still cite `candidate.md §N`, `ADR-N` or `D-NN`. A pending
ADR is in `docs/site/`; any other cited page no longer exists, and the rule it
carried is in these `AGENTS.md` files. When you edit such a comment, write the
reason inline instead of the citation.

## Layers and the one write path

Dependencies point down. `model` depends on nothing; `wire` and `domain` only on
`model`; `ledger` on those three; `broker` (the edge: adapters, outbox, vault)
and `import` sit beside `ledger`; `process` builds on `ledger` and `broker`; the
`cli` binary is on top. The direction is the design: an upward or sideways
dependency is a design change, not a refactor. `crates/cli/tests/layering.rs`
enforces the allowed edges and keeps the store ports inside `ledger`. `vero-vat-api-mock` is a test
and local-development double, never a production path.

- **`ledger` is the only way to change the books.** The CLI primitive
  (`voucher seal`) and a workflow effect reach the same Ledger service. The
  store traits are `pub(crate)` inside `ledger` so nothing else can write
  records directly; keep them that way. `process` writes through the Ledger,
  never to a store.
- **`domain` is deterministic law and never guesses.** Pure functions, no I/O.
  When a fact is missing or a treatment is ambiguous, the Domain refuses with a
  machine-readable code; interpreting the situation is the agent's work. A user
  approval does not override the law (for example, non-deductible VAT stays
  non-deductible however it is approved).
- **The CLI makes no domain decisions.** It is one thin primitive per command
  with `--json`, structured stdin input instead of long flag lists, no
  interactive prompts, and `error_code` / `error_kind` / `message` on every
  failure so a skill can branch on the code (`/ai-first-cli-canon`).
- **Skills compose, the program enforces.** A skill must not keep its own copy
  of a calculation, validation or state machine. Skills are built on one shared
  bookkeeping-assistant role (`skills/aggountant-init/SOUL.md`) plus a short task
  description per skill; the installer ships the role with every skill instead
  of copying it.

## Runs (process)

- **aggountant owns the Run, the agent drives it.** The Run's durable state, its
  journal and every ledger mutation are written by the program; the agent
  proposes, answers gates and triggers, but never mutates state itself. A Run's
  status is derived from its effects' states; there is no separate status field
  to keep in sync.
- **A Run does not start another Run.** A heavy stateful sub-process is its own
  Run (`close-year` does not open `close-period` runs; a close that needs a
  fresh receipt hunt runs `hunt-receipts` separately). Composing Runs needs a
  decision first, because shared state, partial-failure propagation and
  cancellation are not designed.
- **Effects are not transactional with each other.** A partial success parks
  the Run in `NeedsAttention`; it is never reported as plain failure (which
  would hide an irreversible success) or as success with warnings.
- **An external call whose result is unknown is `OutcomeUnknown`, not failed.**
  The intent is recorded in the outbox before the network call, and an unknown
  outcome is reconciled from the remote side, never answered with a blind
  resend. A retry of the same payload keeps its effect key; a changed payload
  gets a new one.
- **An approval binds the bytes that leave.** A preview gate stores the hash of
  the approved payload and the transmitted bytes must hash to it.
- **`run_id` is not authority.** Writes into a `Closing` period are accepted only
  against the in-memory `ClosingAuthority` capability (no `Serialize`, no public
  constructor); a `--run-id` argument is never enough.
- An open obligation blocks `close-period` for its period; closing a Run never
  drops obligations.

## Scope that is decided

- **One trusted user, one company per data home.** Several companies are kept in
  separate repositories and homes. Do not add a `company_id`, home binding or
  company switcher: a single experiment did not justify an identity subsystem,
  and legal company identity is modelled only when a concrete function (for
  example an authority report) needs it. Multi-user concerns go to
  `issues/multi-user-considerations` (root `AGENTS.md`).
- **Company knowledge is prose the agent judges.** A company fact or practice is
  a statement, a prose scope and a dated source, with its revisions kept in the
  same record. The store does not decide relevance through a supplier, VAT or
  usage-share schema; the agent weighs source, date and the company's current
  situation. Writing a fact is an ordinary authorised operation, not a Run.
  Advice from the consult path never becomes a fact on its own; it goes through
  its own review gate. The reasons for a single voucher belong on the voucher.
- **Vouchers, not ledgers of receivables.** Sales and purchases are recorded as
  vouchers. Invoicing, accounts receivable and payable, debt collection and
  payment batches are out of scope: they are integration work against outside
  systems, not an extension of the bookkeeping core.
- **The consult escalation is a later-version concern.** The `consult-llm`
  escalation and its disclosure boundary exist and are tested, but with one
  frontier model interpreting and computing in the same session they add
  little, so they are not to grow onto the default path. The decision was to
  put them behind a default-off setting; that setting is not built, and
  building it waits for a real need.
