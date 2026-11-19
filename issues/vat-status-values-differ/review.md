# Review: vat-status-values-differ

- Skill: `/llm-review`, rigor routine with a high-risk surface: the comparison
  decides whether `file-vat reconcile` records a filing of unknown outcome as
  filed. A false match records another return as ours; a false mismatch is
  safe but sends the user to OmaVero for nothing.
- Scope: the working-tree diff on top of `f46a5b0`
  (`crates/broker/src/vero_vat_api.rs`, `crates/process/src/file_vat/status.rs`,
  `crates/process/src/file_vat/recovery.rs`, the Vero API mock, tests, guide,
  skill, `crates/broker/AGENTS.md`), with the FileVATReturn/v2 request and
  GetFiledVATReturn/v2 response schemas from the Vero API developer portal.
- Finder: `gpt-6-astra`, one call. Verifier: `claude-fable-5-2`, one call: no
  finder candidate was left unfixed, so it got the focused false-negative
  question for the high-risk boundary ("is there an answer that compares equal
  although an amount differs?"). Two model calls in total.

## F1 (must-fix): non-zero VAT relief was ignored

- Location: `crates/broker/src/vero_vat_api.rs`,
  `FiledVatReturn::value_differences`, the `DeductibleTax` branch.
- Claim: GetFiledVATReturn/v2 nests `DeductibleVAT` beside
  `FileVATRelief`, `TurnoverForVATRelief`, `TaxForVATRelief` and
  `AmountOfVATRelief` (returned for periods before 2025). Only `DeductibleVAT`
  was read, so a return with relief compared equal to an approved request
  without it. The old comparison had the same blind spot.
- Evidence: local read of the branch and the schema; aggountant never sends
  `VATRelief` (`api_field` has no destination for it), and no period floor
  keeps 2024 periods out of `file-vat`.
- Fix: the other `DeductibleTax` keys and the top-level `VATOnCashBasis` are
  differences unless they carry nothing (null, false, zero). Tested in
  `an_amount_the_comparison_cannot_read_never_matches`.

## F2 (should-fix, from the verifier): amounts outside the read keys were unseen

- Location: same function; it read only the three group keys,
  `DeductibleTax` and `VATOnCashBasis`.
- Claim: an amount under a top-level key the comparison does not read (a
  renamed or new group) was invisible. For a no-activity request any answer
  reshaped that way matched. This contradicted the rule written into
  `crates/broker/AGENTS.md` in the same change.
- Evidence: the verifier ran the cases through the built broker; local read
  confirms nothing iterated the remaining top-level keys. The answer schema
  fixes the key names, so this needs a deviation from the published schema.
- Fix: every top-level key outside the read set that holds a non-zero number
  or numeric string at any depth is a difference; text such as an identifier
  or a date is not. Tested with a renamed group plus a `BusinessId`.

## Dropped (4)

- Net tax `TaxPayableOrNegativeTaxThatQualifiesForRefund` not compared: it is
  derived from the compared amounts, the request carries no field 308, and
  comparing it would restate the domain's 308 rule in the broker. An answer
  whose net disagrees with its own amounts is internally inconsistent.
- Fractions beyond 28 decimals round in `Decimal::from_str`: not a realistic
  answer.
- Duplicate JSON keys (last wins in serde_json): not a plausible answer.
- `SubmittedDate` compared lexically in `processed_differences`: unchanged
  behaviour, not touched by this diff, and the authority returns ISO dates; it
  matters only for an older return with identical amounts.

## Coverage

Read: the whole diff, `value_differences` and its helpers, `judge`,
`reconcile_with_authority`, `classify`, the mock's `filed_response_for`, both
schemas. Tested: broker unit tests for every structural variant in the issue
and each difference case, status and reconcile end to end against the mock,
`scripts/ci-local.sh` green. Outside scope: the real authority's answers for
the two production periods (the conductor runs `status` against them).
