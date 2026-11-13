# Review: boundary opening voucher (`/llm-review`, critical rigor)

- Finder: `gpt-6-astra`. Verifier: `claude-fable-5-2`. Model calls: 2.
- Rigor: critical, because the change decides when one fiscal year's closing
  position may open the next. Getting that wrong produces silently wrong books.
- Scope: commit `813a8cb` (`fiscal_year_data` / `boundary_cutover` in
  `crates/ledger/src/lib.rs`, tests, guide, skill). The verifier also reviewed the
  follow-up fixes in `b492dec`.
- Risk class: data integrity.

## F1 (must-fix, confirmed, fixed): a backdated voucher changes a sealed year's carried position

- Location: `crates/ledger/src/store/sqlite_fiscal.rs` `commit` (period gate),
  `crates/ledger/src/lib.rs` `fiscal_year_data` (prior fold).
- Claim: once 2026 is sealed on top of a 2025 cut-over, a new voucher dated
  2025-12-31 is still accepted, because the seal gate only checks the month and
  quarter period rows and 2025's periods were never closed. `fiscal_year_data(2027)`
  folds every earlier voucher, so 2027's opening changes silently while naming
  sealed 2026 as its source.
- Evidence: the seal code checks only `month_containing` and `containing` period
  rows. The same hole existed before this change for any never-closed year before a
  sealed one, but the cut-over makes it the normal shape of a home.
- Fix: the seal refuses a voucher dated in or before a Sealed fiscal year
  (`PERIOD_SEALED`, audited). Test: the end of
  `a_boundary_started_year_closes_and_opens_the_next_from_the_sealed_year`.

## F2 (should-fix, confirmed, fixed): reopening a period erases the "kept here" signal

- Location: `crates/ledger/src/lib.rs` `boundary_cutover`.
- Claim: eligibility looked only at the current period state and `vat_filed`. A
  2025 quarter that was closed and then reopened read as never kept here.
- Fix: `head_seq > 0` (any recorded transition) counts as kept here. Test:
  `a_prior_year_with_a_period_closed_in_aggountant_still_refuses`.

## F3 (medium, verifier, confirmed, fixed): a late period close makes a sealed first year irreproducible

- Location: `sqlite_fiscal.rs` `transition_period`.
- Claim: with 2026 Sealed and 2025 never closed, `begin_close(2025-Q4)` was still
  allowed. After F2 that makes `boundary_cutover` return `None` for good, so
  reports for 2026 refuse. Recovering by sealing 2025 changes 2026's opening source
  and therefore its `statements_hash`. The amounts stay the same; the failure is
  loud, not silent.
- Fix: period transitions refuse a period of a year at or before a Sealed year
  (the VAT self-edge stays exempt).

## F4 (low, verifier, confirmed, fixed): attributing an opening voucher to an earlier year

- Location: `sqlite_fiscal.rs` `attribute_adjustment`.
- Claim: the guard checked only `[effective_year, posting_year]`, so after 2026 was
  sealed an opening voucher could still be attributed to 2024, which permanently
  disqualifies the cut-over. Today this is library-only (no CLI caller).
- Fix: the guard covers every year from `effective_year` onwards. The now unused
  `AttributionRequest::posting_year` field was removed.

## Verifier checks with no finding

(A) blocks no legitimate flow. The verifier checked idempotent replay of an
already-sealed voucher (replays before the gate), close-year adjustments (posted
in the next year), close-period with `ClosingAuthority`, non-calendar years and
`define_fiscal_year`, and the full workspace suite (780 passed) plus a throwaway
probe. Behaviour change: a voucher dated inside a Sealed year is now rejected by
the year gate, so the rejection target is `fiscal:date:` rather than
`fiscal:period:`, with the same `PERIOD_SEALED` code. Nothing matches on the
target.

## Dropped

None.

## Coverage

Read: `fiscal_year_data`, `boundary_cutover`, the seal commit path, period
transitions, attribution, and the domain `validate_opening_context` (live period
inside the year; the target chart equals the active chart, which holds because
the source uses the active chart version). Outside scope: the planned git mirror
and the close-year Run's own persisted state.
