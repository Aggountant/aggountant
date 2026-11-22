# Result: read-commands-create-home

Only `init` creates a company home now. Every other command that opens a home
first checks that the resolved path holds one, and otherwise stops with
`HOME_NOT_INITIALIZED`, exit 1, writing nothing. The message and `details`
name the path (made absolute, so a wrong working directory shows) and how it
was chosen:

```json
{"error_code":"HOME_NOT_INITIALIZED","error_kind":"user",
 "details":{"home":"/work/accounting","source":"default","marker_path":null},
 "message":"no initialized aggountant home at /work/accounting, chosen by the ./accounting fallback, used when no --home, $AGGOUNTANT_HOME, configured home or .aggountant marker applies; nothing was written. ..."}
```

`source` uses the same values as `config show` (`flag`, `env`, `file`,
`discovered`, `default`); for `file` the message names the configuration
file, for `discovered` the marker.

## What changed

- `crates/cli/src/main.rs`: `existing_home(cli)` is the one gate; every
  home-opening command goes through it (directly or via
  `open_existing_ledger(cli)`), including write commands (seal, import,
  filing, payroll, receipts, knowledge, `vero-fi`/`tulorekisteri configure`,
  `home upgrade`) and the camt import commands in `bank_import.rs`.
  `reports.rs` gets its home from the same gate. `home_path` remains for
  `init` alone. Commands that never open a home (`config`, `explain`,
  `skill`, `voucher draft`, `voucher vat-codes`, `import camt053 parse`) are
  unchanged.
- No command other than `init` needed to create a home: the tests that ran
  `knowledge`, `vero-fi configure`, `tulorekisteri configure` or
  `file-wage employer-report configure` in an empty directory did so only for
  convenience, and the guide and skills always run `init` first.
- A home is `fiscal.sqlite` plus `masterdata/reference/`. `artifacts/` and
  `masterdata/pii.sqlite` are not required (the earlier voucher-read
  preflight required them); a home copied without its empty `artifacts/`
  still opens, and a clone of a company repository without its voucher
  database is refused instead of being opened as empty books.
- Error code: `HOME_NOT_INITIALIZED` already existed for `voucher show/list`
  and is kept. Commands that used to fail with `MASTERDATA_NOT_INITIALIZED`
  in a non-home directory (`chart list`, `home upgrade`, `employee list`, ...)
  now answer `HOME_NOT_INITIALIZED`. The code is now in the error-code table
  (fi, en), so `explain` and the error `hint` know it.
- `--home` help says only `init` creates a home. The guide's troubleshooting,
  set-up and company-home pages describe the refusal; the limitation entry in
  `reference/status.md` is removed.

## Tests

- `crates/cli/tests/home_required_cli.rs`: 24 read commands and 9 write
  commands in an empty directory via the `./accounting` fallback leave the
  directory empty and name the fallback; the fallback from inside a home
  creates no `accounting/accounting`; `--home`, `$AGGOUNTANT_HOME`, a
  configured home and a bare `.aggountant` marker are named as the source;
  the cloned-home and missing-`artifacts/` cases.
- Existing tests that relied on a home being created now call `init` first
  (`tests/common::initialized_home`).
- `scripts/ci-local.sh` passes.

Review: `review.md` (`/llm-review`, no findings) and `assessment.md`.
