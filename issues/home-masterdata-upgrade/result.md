# Result

An existing company home takes in the masterdata a newer binary needs with
`aggountant home upgrade --dry-run` and then `--apply`, with no copying of
files by hand. A home written the way an older `init` wrote it (no
`chart-snapshots/`, no `vat-account-roles.json`), which the current binary
refuses with `MASTERDATA_CORRUPT`, opens after the upgrade and its sealed
vouchers verify. The active chart and every existing file stay byte-identical.

## Decisions for later review

- **Command shape: `home upgrade` with a required `--dry-run` or `--apply`.**
  The alternative was a bare `home upgrade` that applies by default, with
  `--dry-run` as an option (the canon's usual shape). I made the mode required
  because the command writes into a production home and the user guide tells
  the agent to plan first. `home` is a new top-level group, so it does not
  collide with `init` (which refuses an existing home on purpose) or `chart`.
- **The upgrade only creates files, never rewrites them.** A file whose bytes
  differ from the bundled one is reported as `skipped` with reason `differs`
  and left alone, even when its meaning is the same (for example the same roles
  with other formatting). It could be company-approved masterdata, and a byte
  rule is the one the user can check.
- **Role metadata is added only when the bundled table activates the home's
  own active chart.** For a company chart without roles the role file is
  reported `unavailable` and not written. Writing roles for a chart the home
  does not use would not make the home open and would put foreign metadata in
  the company's tree.
- **New roles are not merged into an existing role file.** If a later binary
  bundles a new chart edition, its snapshot is added, but its roles stay out of
  a role file that already exists, because merging would rewrite that file. A
  home only needs roles for its active chart, and switching charts is a
  separate decision. The `differs` detail names the bundled chart versions the
  existing file lacks.
- **A snapshot of the active chart is added only if the active chart is
  bundled.** A company chart's snapshot is not created from
  `chart-of-accounts.json`. The issue asks only for bundled snapshots, and
  `open` already resolves the active chart from the chart file.
- **An unreadable active chart or existing snapshot refuses the whole
  upgrade**, and nothing is written. The upgrade does not repair anything.
- **Audit record: none in the home.** The masterdata is git-native and its
  README tells the user to commit it. The created files show up in
  `git status`, and the guide tells the user to commit them, so git history is
  the record. The JSON output (`created`, `existed`, `skipped`) reports the
  run. A log file in the home would be a second record with no reader.
- **Exit code 0 when the upgrade ran**, even if `masterdata_opens` is `false`
  (for example when roles are `unavailable`). The upgrade did everything it
  safely could, and the next command that opens the home fails with its own
  code. Errors before planning (home not initialized, corrupt chart) exit 1, and
  I/O errors exit 2, as elsewhere.
- **No-clobber writes**: a temporary file plus `hard_link`, which fails if the
  target exists. `rename` would replace a file that appeared in the meantime.

## Gates

- `scripts/ci-local.sh`: green (fmt, clippy, cargo-deny, build and test on stable
  and 1.89, user guide pairs and links, site build, practice month).
- Review: `review.md` and `assessment.md` beside this file. There was one
  confirmed should-fix finding (stray temporary file on a write error), and it
  is fixed. No data-integrity findings.
