# Result: vero-configure-merge

`aggountant vero-fi configure` merges its stdin JSON into the stored
`vero-filing.json` instead of replacing it.

- The input is a JSON merge patch (RFC 7386): a field left out keeps its
  stored value, `null` clears it, members of `attestations` merge one by one.
  A whole profile is still accepted, so existing callers keep working; sending
  the stored profile back changes nothing. `schema_version` and `mode` are
  required only when there is no stored profile.
- The evidence the verify commands record (`attestations.test_verification`,
  `attestations.portal_verification`) cannot be cleared or changed by a merge
  input: `VERO_EVIDENCE_WOULD_BE_DROPPED`, nothing written.
- `--replace` writes the input as the whole profile (the old behaviour); it is
  also the way past `VERO_PROFILE_INVALID` when the stored file is broken.
- The output has `merged` and `changes` (every changed leaf field with
  `before`/`after`, plus `credential_path` when it changes), in both `--dry-run`
  and real runs.
- A `null` on an unknown field name is refused like any unknown field.

Compatibility: every valid stored profile is read through the same type as
before and written back with identical values; tested with an onboarded
production profile carrying both evidence records. One deliberate semantic
change: an input that omits a field, or sends `"attestations": {}`, no longer
resets it.

Help (`vero-fi configure --help`) and the user guide (`set-up-vero-api`,
en/fi) are updated. The `vero-fi` skill's warning about whole-profile
replacement is now obsolete; it is left for the skill rebuild, which owns
`skills/` at the moment.

Review and assessment: `review.md`, `assessment.md`.
