# Result

`VatTreatment` now refuses any member that does not belong to its variant.
`import camt053 seal` answers INVALID_INPUT and seals nothing when, for
example, a `deductibility` is written on `domestic-input`.

- `crates/domain/src/vat/expand.rs`: `deny_unknown_fields` on the enum.
  Serde does not apply it to the unit variant `no-vat`, so that variant refuses
  members through a variant-level `deserialize_with = "deny_members"`. The Rust
  API is unchanged.
- `crates/cli/src/input_help.rs`: the "ignored, not refused" sentence now says
  that a member which does not belong to the type is refused.
- Tests: `treatment_refuses_members_of_another_variant_for_every_variant`
  covers every variant. The `type` tag still selects the variant, serialization
  round-trips, and foreign, duplicate and unknown members are refused.
  `deductibility_misplaced_on_a_full_treatment_is_refused_before_sealing` is
  the end-to-end CLI case: INVALID_INPUT and no fiscal vouchers.
- Stored data is unaffected. `VatTreatment` is deserialized only from the
  import assignment on stdin, and sealed vouchers store `VatExpansionTreatment`.
  A saved stdin assignment file with surplus members is now refused when
  resubmitted, which is the intended behaviour.
- Same landing, separate commit: the bundled skills' `cli_version` was bumped to
  0.2.0. Release v0.2.0 had left it at 0.1.0, which failed
  `scripts/ci-local.sh` on main.

Review and assessment: `review.md` and `assessment.md`. Spin-off:
`receipt-vat-proposal-no-vat-unknown-members`.
