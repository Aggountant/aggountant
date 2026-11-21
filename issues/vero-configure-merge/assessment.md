# Assessment of the vero-configure-merge review

Assessed against the commit that lands this slice (review fixes included).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Unknown field set to `null` accepted silently | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX (confirmed, can leave an attestation asserted) — applied |
| F2 | Credential path change missing from `changes` | CONFIRMED | OCCASIONAL | IMPROVES | MINOR | HIGH | FIX (documented output was incomplete) — applied |
| V1 | Non-object `attestations` replaces the object | CONFIRMED | RARE | NEUTRAL | MINOR | MEDIUM | DROP (listed in `changes`, cannot reach production or evidence, predates the slice) |
| V2 | `production_verification_completed` not guarded | CONFIRMED | RARE | NEUTRAL | MINOR | MEDIUM | DROP (a typed attestation the user can restate; guarding it would block a legitimate withdrawal) |
| V3 | Help implied `credential_path: null` clears it | CONFIRMED | RARE | IMPROVES | NONE | HIGH | FIX — help says absent or null leaves the path unchanged |
| V4 | Platform config written before the profile | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | DROP (predates the slice; stored profile stays valid, rerun repairs) |

FIX: 3   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 3

No confirmed data-integrity finding remains open. No follow-up issue was
filed.

## Simplicity

- Reused: `VeroFilingProfile` and its `deny_unknown_fields` serde derive as
  the one validation of field names and values (the merge result is
  deserialised, not a second hand-written schema), `validate()`,
  `write_vero_profile`, the readiness evaluation and the existing `would` list.
- Removed: `VeroConfigureInput`, a second copy of the profile's fields in
  `main.rs`, and its `profile()` conversion.
- Remaining repetition: `PROFILE_FIELDS` names the profile's fields once more,
  needed only because a `null` deletion never reaches serde. A unit test ties
  it to the type's serialisation, so drift fails the build.
- Left out: a general patch framework, per-field flags, guarding the typed
  attestations, and a separate `diff` command; `--replace` is the one escape
  hatch and keeps the old whole-profile semantics for recovery.
