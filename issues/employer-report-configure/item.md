---
created: 2026-11-19
updated: 2026-11-19
type: improvement
status: open
priority: normal
preparation: ready
lane: wage-reporting
lane_seq: 0
collision: [crates/cli/src/wage.rs, crates/process/src/file_employer_report.rs]
---

# employer-report: a configure command for the employer_report profile, with the YTJ registration check

## Description

The monthly employer's separate report needs an `employer_report` block in
`.aggountant/tulorekisteri.json` (`registered_from`, `contact_name`,
`contact_phone`, `pension_insurances[]`), but no command writes it. An agent
has to edit the JSON by hand, and nothing checks the values until a later
`missing` or `preview` fails or, worse, silently uses a wrong
`registered_from`. `file-wage employer-report registration` already knows
the employer-register start from YTJ; the setup step does not use it.

Example (Esimerkki Oy, fictional): the agent writes
`"registered_from": "2026-07"` by hand; YTJ says the company has been in the
employer register since 2020-02, and 77 months are hidden from `missing`
until someone runs `registration`.

## Wanted

`file-wage employer-report configure`, in the style of
`vero-fi configure` (validate and report readiness, `--dry-run` writes
nothing):

- inputs: `--registered-from`, `--contact-name`, `--contact-phone`,
  `--pension-insurance PROVIDER_CODE:POLICY_NUMBER` (repeatable) or
  `--no-pension-insurance` (explicit, so an empty list is never a default);
  or the same as stdin JSON;
- runs the `registration` YTJ check and refuses a `registered_from` that
  disagrees with the register unless `--accept-register-mismatch` is given
  (with the comparison in the output); a YTJ failure is reported and the
  write needs `--offline` explicitly;
- validates the phone format, provider codes and policy-number shape the
  report schema requires;
- writes only the `employer_report` block, keeping the rest of the profile
  byte-for-byte, and prints the before/after;
- `--help` with examples; the `vero-fi` skill and the guide use the command
  instead of telling the agent to edit JSON.
