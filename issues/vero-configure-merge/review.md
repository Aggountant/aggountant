# Review: vero-configure-merge

- Skill: `/llm-review`, rigor **routine** with a high-risk surface: the change
  rewrites the stored Vero filing profile, which carries the verification
  evidence production filing depends on. Losing it silently blocks or misstates
  production readiness.
- Scope: commit `a1c891b` against its parent (`crates/cli/src/vero_fi.rs`,
  `crates/cli/src/main.rs`, `crates/cli/src/input_help.rs`,
  `crates/cli/tests/vero_fi_cli.rs`, user guide), with
  `crates/broker/src/vero_filing.rs` attached.
- Finder: `gpt-6-astra`, one call. Verifier: `claude-fable-5-2`, one call,
  asked the focused false-negative question for the high-risk boundary (can
  stored evidence or attestations be lost or altered without `--replace`, or
  a valid profile left invalid) against the code with the finder's fixes
  applied. Two model calls in total.
- Risk class: stored onboarding evidence (data integrity of the filing
  profile).

## F1 (should-fix): an unknown field set to `null` was accepted silently

- Location: `crates/cli/src/vero_fi.rs`, `merge_patch` (a `null` member is
  removed before the merged profile reaches `deny_unknown_fields`).
- Claim: `{"attestations": {"production_permission_activ": null}}` succeeded
  as a no-op. An agent clearing an attestation with a misspelt name would be
  told it succeeded while the attestation stayed asserted.
- Evidence: finder reproduced it with the built CLI; confirmed locally by the
  regression test, which failed before the fix.
- Fix: `reject_unknown_nulls` checks every `null` member against
  `PROFILE_FIELDS` before the merge; a unit test pins `PROFILE_FIELDS` to the
  fields a fully populated `VeroFilingProfile` serialises, so a new profile
  field cannot be missed. Test `a_misspelt_field_cleared_with_null_is_refused`.

## F2 (should-fix): a credential path change was missing from `changes`

- Location: `crates/cli/src/main.rs`, the `vero-fi configure` handler.
- Claim: `changes` is documented as every changed field, but a new
  `credential_path` appeared only in `would` (dry run) and not at all in a
  real run.
- Evidence: local read; the profile diff runs after `credential_path` is
  taken out of the input.
- Fix: the handler adds a `credential_path` entry (before/after) when the
  supplied path differs from the stored one, captured before the platform
  config is rewritten. Test `a_new_credential_path_is_listed_among_the_changes`
  covers dry run, real run and an unchanged path.

## Verifier result

No case found where evidence or attestations are lost or altered without
`--replace`, or a valid stored profile becomes invalid. It built the binary
and probed: `{"attestations": null}` and `{"attestations": []}` are refused
with evidence stored; an `{}` patch over a `+03:00` instant normalises the
spelling with `changes: []`; dry run and write give the same `changes`.

Boundaries it noted outside the criterion (assessed in `assessment.md`):

- V1: `{"attestations": [true]}` (any non-object) replaces the attestations
  object and serde reads a sequence, clearing non-evidence attestations.
  Listed in `changes`; cannot pass production readiness or touch evidence; the
  old configure accepted the same form.
- V2: `production_verification_completed` is set by `verify-portal` but is not
  guarded; an explicit `false` changes it.
- V3: `"credential_path": null` is ignored while the help said null clears a
  field.
- V4: the platform config is written before the profile; an I/O failure on
  the profile write leaves the new credential path stored. Predates the diff.

## Dropped candidates

None from the finder; both candidates were confirmed.

## Coverage

Read: the whole diff, `vero_filing.rs` (profile type, validation, write),
the verify commands' profile writes in `main.rs`, the existing CLI tests and
`stdin_input_help`. Tested: `crates/cli/tests/vero_fi_cli.rs` (12 tests,
including an onboarded production profile with both evidence records and a
millisecond instant), the `input_help` unit tests, and `scripts/ci-local.sh`.
Not covered: concurrent writers (single-user scope).
