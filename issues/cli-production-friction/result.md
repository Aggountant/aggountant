# Result: cli-production-friction

All three frictions are fixed in `crates/cli`, and existing inputs and JSON
shapes still work.

- **`knowledge company update|retire <ID>`**: the fact id can be given
  positionally, with `--id` (but not both), or as `id` in the stdin JSON. If
  the JSON id and an argument are both given they must match
  (`FACT_ID_MISMATCH`). With no id anywhere the command fails with
  `FACT_ID_REQUIRED`. Stdin is still parsed strictly: unknown fields and
  duplicate keys are refused. `--help` for `add`, `update` and `retire` shows
  an example.
- **`chart list`** text output: the header line is unchanged and is followed
  by one `number<TAB>name<TAB>kind` line per account. `--json` is unchanged.
- **`report statements`**: when `--output` is omitted or `-`, the report goes
  to stdout and `--format` defaults to `native-json`. In text mode stdout
  carries the document itself, byte-identical to the file form. With `--json`
  stdout is the usual summary object with `"output":"-"` and the document text
  in `content`. File output still requires `--format`
  (`FORMAT_REQUIRED` otherwise) and still never overwrites a file. `--help`
  shows the quick form.

Docs: monthly-report task page (fi/en), the `knowledge company` commands in
SOUL, and the CHANGELOG `[Unreleased]` section.

Tests: `company_knowledge_cli.rs` (id sources, mismatch, missing id,
duplicate keys, help examples), `monthly_reports.rs` (stdout default, `-`,
html, `--json` envelope, file shape unchanged, help), `light_path.rs` (chart
text lists every account). `scripts/ci-local.sh` passes.

Review: `review.md`, `assessment.md` (one confirmed finding, fixed).
