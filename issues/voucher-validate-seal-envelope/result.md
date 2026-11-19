# Result: voucher-validate-seal-envelope

`voucher validate` now accepts the file `voucher seal` accepts:

```
aggountant --json voucher validate < seal.json   # {"draft":{…},"natural_key":"…","actor":"…"}
aggountant --json voucher seal < seal.json
```

A bare draft keeps working exactly as before.

## What was built

- `crates/cli/src/main.rs`: `voucher validate` reads stdin once and picks the
  shape. A top-level `draft`, `natural_key` or `actor` means the seal
  envelope (a bare draft's fields are closed, so the two never overlap); it is
  parsed as `SealInput` and the draft goes through `build_draft`, both exactly
  as in `seal`, so a broken envelope gets seal's error and the domain gates
  (EU-sale buyer VAT-ID, reverse-charge structure) apply. A bare draft is read
  as `VoucherDraft` as before. `--help` for `validate` documents both shapes
  with the same example envelope as `seal --help`; `seal --help` says to
  dry-run the same file with `validate`.
- `crates/ledger/src/lib.rs`: `Ledger::validate_seal(draft, natural_key)`
  runs `validate`, then what `commit` checks, in its order: a key that already
  sealed the same request is the warning `NATURAL_KEY_ALREADY_SEALED` (with
  `voucher_number`; seal replays it); a key bound to different content, or a
  key the importer bound to a source event, is refused with
  `IDEMPOTENCY_KEY_REUSE`; an unused key is checked against the fiscal
  calendar and period state (`PERIOD_CLOSING`, `PERIOD_SEALED`,
  undefined fiscal year). Nothing is written, not even a rejection record.
  The warning type is `ValidationWarning` (was `UntaggedVatWarning`), whose
  `posting_index`/`account` are omitted when absent; the existing
  `VAT_POSTING_UNTAGGED` JSON is unchanged.
- `crates/ledger/src/store`: `read_voucher_by_natural_key` and
  `posting_gate_refusal` (read-only). The calendar and period gate was moved
  out of `SqliteFiscalStore::commit` into one function that `commit` and the
  dry-run share.
- Guide (fi, en): the command blocks in `start/set-up-company.md`,
  `tasks/correct-voucher.md` and `tasks/record-other-entries.md` validate the
  seal file; `record-other-entries` says to validate the file you seal.
  Skill `capture-receipt` says the same.

## Acceptance

- Envelope validates, then warns after sealing, refuses changed content with
  seal's code, accepts a new key, actor does not matter, bare draft still
  validates, help examples match:
  `crates/cli/tests/voucher_input_help.rs`
  `validate_accepts_the_seal_envelope_it_documents_and_checks_its_natural_key`.
- Broken envelopes (missing `draft`/`natural_key`, unknown field) give the
  same `INVALID_INPUT` in `validate` and `seal`; an EU sale without buyer
  VAT-ID gives the same domain error in both:
  `validate_reads_a_broken_envelope_as_seal_does`.
- Replay, conflict, source-bound conflict, no rejection written:
  `crates/ledger/tests/seal.rs`
  `validate_seal_predicts_replay_and_conflict_without_writing`.
- Closing and Sealed periods refused, replay into a closed period still a
  warning, bare `validate` unchanged:
  `crates/ledger/tests/period.rs`
  `validate_seal_sees_the_period_gate_and_replays_like_seal`.

## Decisions for later review

- **An already-used key with the same content is a warning, different
  content is an error.** The issue suggested a warning; a dry-run that says
  `ok: true` for input seal will certainly refuse would mislead, so the
  conflicting case refuses with seal's own code.
- **Only the envelope sees period state.** The envelope is a seal request;
  adding period gating to bare-draft validation would change an existing
  command's results without being asked.

## Gates

`scripts/ci-local.sh` green (fmt, clippy, cargo-deny, build and test on
stable and 1.89, user guide) after the review fixes.

## Review

`review.md` (`/llm-review`, routine; `gpt-6-astra` finder,
`claude-fable-5-2` verifier) and `assessment.md`: two confirmed findings, both
fixed in this slice. No follow-up issue was filed.
