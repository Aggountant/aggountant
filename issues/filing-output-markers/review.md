# Review: filing-output-markers

- Skill: `/llm-review`, rigor **routine**: the change adds output fields and one
  error code; it does not change what is filed or sealed.
- Scope: commit `c90d0bf` (`crates/cli/src/main.rs`, `crates/cli/src/wage.rs`,
  `crates/process/src/{file_wage,capture_receipt,error,lib}.rs`,
  `crates/domain/src/vat/code.rs`, tests, guide fi/en).
- Finder: `claude-fable-5-2`, one call (the diff plus `main.rs`). It ran the
  built binary against a disposable home for the capture paths.
- Verifier: not called. Both candidates were reproduced locally by running the
  binary (outputs below), which settles them more firmly than a second model
  could. One model call in total.

## F1 (should-fix): the refusal's class list steered toward a 0 % domestic voucher

- Location: `crates/process/src/capture_receipt.rs`, `known_rate_class`; the
  `CAPTURE_VAT_CLASS_UNKNOWN` rows in `docs/user/{fi,en}/reference/error-codes.md`.
- Claim: the message said "use one of" every class in the rate table and the
  guide's hint said to pick from that list, but `domestic-input` with
  `eu-goods`, `eu-services` or `zero-rated` is accepted and seals at 0 % with
  no VAT legs, so following the hint could hide a reverse-charge purchase.
- Evidence: local run, `domestic-input` + `eu-goods` dry run returns
  `projection_valid: true`, `resolved_percent: "0.0"`, `generated_vat_legs: []`.
  The acceptance predates this change; the wording that pointed at it is new.
- Fix: the message says "known classes: …", the hint says to choose the class
  matching the receipt's VAT treatment. The underlying acceptance is a Domain
  rule and filed as `capture-domestic-input-eu-class`.

## F2 (should-fix): the doubled `VAT_CODE:` remained on other Domain refusals

- Location: `crates/process/src/capture_receipt.rs`, `map_domain`.
- Claim: `map_domain` prefixed `e.error_code()` to a `Display` that already
  starts with it, so a known class with no dated edition still gave
  `INVALID_ANSWER: VAT_CODE: VAT_CODE: …`; the new doc comment also named the
  wrong code.
- Evidence: local run with `domestic-standard` and `document_date` 2012-07-15.
- Fix: `map_domain` uses the Domain text as is; comment corrected; CLI test
  asserts `VAT_CODE` appears once.

## Checked and found sound (finder and local)

- Every scripted broker is built through `build_scripted_broker_path`, so no
  command can use the seam without the `test_seam` marker; `file-wage` never
  uses the seam.
- The error path stays one JSON line on stderr; the plain warning is written
  only on success without `--json`.
- The class pre-check rejects nothing that previously succeeded: an unknown
  class always failed in `resolve_vat_rate`.
- `sensitive.record_hash` is additive and serialize-only; it is not in stored
  state or in any hash.

Dropped candidates: none.
