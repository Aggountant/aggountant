# Assessment of the employer-report-configure review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Non-JSON output hid the before/after | CONFIRMED | COMMON | NEUTRAL | NONE | HIGH | FIX — applied |
| V1 | An invalid existing block could not be repaired by configure | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX — applied |
| V2 | The profile was read twice | CONFIRMED | RARE | IMPROVES | NONE | HIGH | FIX — applied with V1 |
| — | Symlink, file mode, directory fsync, canonical re-layout | CONFIRMED | RARE | NEUTRAL | NONE | MEDIUM | DROP |

FIX: 3   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

- F1: the guide's commands are shown without `--json`, so the human line is
  what a person reading along sees. Fixed with a configure-specific line;
  tested.
- V1: the command exists to replace hand edits of this block, and a broken
  hand edit is exactly the case it would meet. `with_employer_report` now
  requires the rest of the profile to be valid (the spliced result is parsed
  as `WageProfile`, which also refuses a duplicated key) but replaces an
  invalid block; `before` is the block as written, valid or not. Tested in the
  broker unit test (repair, invalid rest refused, duplicate refused).
- V2: fixed by V1's shape: one read, the splice returns the parsed new
  profile (Business ID for YTJ) and the old block.
- Dropped side effects: the profile is a plain file written by agents in a
  git-versioned home; none of them changes content, and a power loss leaves
  the old file. The profile holds a credentials path, not the secret, so the
  mode is not a confidentiality concern.

No confirmed data-integrity finding remains open; no follow-up issue filed.

## Simplicity

- Reused: `compare_registration` and the YTJ error mapping (factored into
  `register_entries`, now shared by `registration` and `configure`, instead of
  a second copy), `FilingContactName`/`FilingContactPhone` parsing, the
  `EmployerReportProfile` type as the stdin shape, the vero-fi configure
  conventions (`--dry-run`, `would`).
- Repetition removed: the pension-insurance shape rule moved from
  `EmployerSeparateReport::validate` to `PensionInsurance::validate`, which
  the report, the profile validation and the CLI flag parser all call.
- Left out: partial updates (changing one field keeps the others), keeping
  the file's indentation style for the inserted block, and a JSON-patch
  abstraction; the splice handles exactly one top-level member.
