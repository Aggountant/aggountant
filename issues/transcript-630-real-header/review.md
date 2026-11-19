# Review: transcript-630-real-header

`/llm-review`, routine rigor (the change only renames expected header and type
strings; a mismatch is refused with nothing stored, so it is not a
data-loss or filing boundary).

- Finder: `gpt-6-astra` (`--task review`), one call, with the working-tree
  diff of `crates/process/src/transcript_630.rs`, both transcript fixtures,
  the guide pages, the error-code tables and `skills/vero-fi/SKILL.md`, plus
  the full parser source.
- Verifier: not called. The finder returned no candidates and the surface is
  not high-risk.
- Model calls: 1.

## Confirmed findings

None. The finder answered "No findings." after inspecting the parser, its
import caller (`employer_report_history.rs`), the integration tests, and the
fixtures' column counts and range totals.

## Unresolved uncertainty

The two substitute-payer earnings types are pluralised by analogy; only
"Palkkatietoilmoitukset: Maksajan omat suoritukset" was seen in a real file.
If the e-service writes them differently, such a file is refused with
`TRANSCRIPT_INVALID_ROW` and nothing is stored. Not a defect, recorded in
`result.md`.

## Dropped candidates

None (zero proposed).

## Coverage

Read: the whole diff and parser. Tested locally: `scripts/ci-local.sh`
(fmt, clippy, cargo-deny, build and test on stable and 1.89, guide) green.
Outside scope: the dry run on the real files, which are private (left to the
conductor).
