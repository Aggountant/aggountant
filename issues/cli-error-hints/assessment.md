# Assessment of the cli-error-hints review

The review (`review.md`) returned no findings, so there is nothing to triage.

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

No follow-up issue was filed.

## Simplicity

- Reuses: the guide's existing error-code table as the only source, bundled
  with `include_str!` the way `skill.rs` bundles the skills; the single
  `emit_error` path, so every failing command gets the hint with one change.
- Removes repetition: the skills can now drop their own code explanations
  (left to the skill rebuild; `skills/` was out of scope here).
- Leaves out: no generated Rust table or build script (parsing ~50 rows at
  error time is trivial and keeps one artefact), no hints for clap usage
  errors or for warnings inside successful output, no bundling of the
  Vero.fi blocker page (an unknown `VERO_…` code points to it instead).
