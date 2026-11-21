# Assessment of the agent-surface-small review

Assessed against the fix commit that follows `a41dfc6`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `configure --replace` dropped the employer_report block | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX (confirmed, data loss in the profile) — applied |
| F2 | vat_code help gave the wrong formula for margin codes | CONFIRMED | RARE | IMPROVES | NONE | HIGH | FIX (help an agent books from; silently wrong VAT leg) — applied |
| V1 | readiness did not load the signing key | CONFIRMED | RARE | NEUTRAL | MINOR | MED | FIX (status would claim ready for a bundle every send refuses) — applied |

FIX: 3   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: `--replace` is the repair path for a broken profile, which is exactly
  when an agent reaches for it; losing the employer's separate report settings
  then breaks the monthly report. Fixed by keeping a parseable block and
  diffing against the stored file.
- F2: margin-scheme sales are uncommon among the companies using this, but a
  help text that gives the wrong formula is silently wrong VAT the first time
  it is followed. Fixed in the shared paragraph.
- V1: pre-existing, found by the verifier while checking the gate. The
  repository's own VAT test fixture carries an EC key that the VAT channel
  accepts and Tulorekisteri signing refuses, so a bundle reused from VAT is
  not automatically usable here. Fixed because it is small and makes `tulorekisteri status`
  truthful; the gate only became stricter.

No confirmed finding remains open; no follow-up issue was filed.

## Simplicity

- Reused: the upgrade planner (`GitMasterDataRefStore::upgrade` dry-run) decides
  whether `MASTERDATA_CORRUPT` names `home upgrade`, so the hint cannot promise
  a file the upgrade would refuse; `parse_vat_code`, the EU-acquisition guard
  (now a per-code function both callers use), `eu_sale_kind` and the dated rate
  table decide what `voucher vat-codes` lists, so the listing cannot drift from
  what a seal accepts; `vero_fi::merge_patch` / `diff_leaves` and
  `write_wage_profile` for `tulorekisteri configure`; one credential path
  resolution (`tulorekisteri::credential_path`) for status, configure and the
  `file-wage` broker.
- Repetition removed: the readiness requirements are stated once
  (`profile_blockers`, `credential_blockers`) and used by both the offline
  report and the send-time gate; the signing key load is one function used by
  signing and readiness; `MarginCategory` parsing goes through `ALL` /
  `segment()` instead of a second string table.
- Remaining repetition, justified: `file-wage`'s broker keeps its own
  path checks (`MISSING_CREDENTIAL`, `INVALID_CREDENTIAL_PATH`,
  `CREDENTIAL_IN_ACCOUNTING_HOME`) because those error codes are existing API
  that tests and agents branch on.
- Left out on purpose: a separate blockers page in the guide (each blocker's
  message says what to do, and the `TULOREKISTERI_NOT_READY` entry explains
  owners); a chart-aware `vat-codes` (the mistag guard depends on the account,
  so `voucher validate` stays the place for it); extending `vero-fi status`
  instead of a `tulorekisteri` group (the profiles, credentials and
  requirements differ).
