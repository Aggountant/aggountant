# Result: wage-report-onboarding-docs

## Record hash without argv

`file-wage approve`, `file-wage correction submit` and
`file-wage employer-report submit` take `--record-hash-file PATH` as an
alternative to `--record-hash HASH` (exactly one is required; `--record-hash`
keeps working). `-` reads the hash on stdin, except for
`correction submit --action replace`, whose stdin is the report: there `-` is
refused with `INVALID_INPUT` and a path is used. Surrounding whitespace (the
newline `jq -r` adds) is ignored; anything other than one token is refused.
No error repeats what was read.

The documented safe flow captures the hash from the preview the user reviews
into a private `mktemp` file outside the home, shows the preview without it,
and passes the file to the approval:

```sh
aggountant --json file-wage preview --timestamp T < report.json \
  | tee >(jq -r .record_hash > HASH_FILE) | jq 'del(.record_hash)'
aggountant file-wage approve wr-… --record-hash-file HASH_FILE
```

Reading the hash from the stored run was not built: it would approve without
naming what the user saw (see `assessment.md`, Simplicity).

## Onboarding and production facts

`skills/tulorekisteri/kayttoonotto.md` lists the four interface choices on the
replacement-application form, names Palkkatiedot (Web service) as the one
`file-wage` needs, keeps Vero API ticked, and names the confirmation's
"Palkkatiedot tuottaja (Web service)" and "Vero API (Web service)".
`skills/tulorekisteri/SKILL.md` records the synchronous production acceptance
of a travel report (dated, no company data) and the safe hash flow. The user
guide's travel-allowance page (en, fi) follows. The `vero-fi` skill only
points to `tulorekisteri` and needed no change.

## Verification

`crates/cli/tests/file_wage_cli.rs` covers stdin, file, empty, multi-token and
missing-file input, both and neither form, and the refused `-` on a
replacement. `scripts/ci-local.sh` passes. Review and assessment:
`review.md`, `assessment.md`.
