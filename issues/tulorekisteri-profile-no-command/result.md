# Result: tulorekisteri-profile-no-command

```
aggountant tulorekisteri status
aggountant tulorekisteri configure [--dry-run] [--replace] < settings.json
```

- `tulorekisteri configure` writes `.aggountant/tulorekisteri.json`
  (`schema_version`, `mode` disabled|production, `business_id`,
  `reporting_permission_active`, `credentials_file`) like `vero-fi configure`:
  a JSON merge patch onto the stored profile, `changes` before/after,
  `--dry-run` with `would`, `--replace` for a broken file. It refuses
  `employer_report` and keeps the stored block (also with `--replace`), and
  writes production only when the channel is ready, naming each blocker
  (`TULOREKISTERI_NOT_READY`). The INPUT section describes the file.
- `tulorekisteri status` is the offline readiness report: profile path,
  mode, Business ID, the permission statement, credential path, certificate
  validity, `employer_report_configured`, and every blocker with a code, an
  owner (`human`/`agent`) and what to do (`TULOREKISTERI_PROFILE_MISSING`,
  `_DISABLED`, `_REPORTING_PERMISSION_UNATTESTED`, `_CREDENTIAL_PATH_MISSING`,
  `_CREDENTIAL_IN_ACCOUNTING_HOME`, `_CREDENTIAL_BUSINESS_ID_MISMATCH`,
  `_CERTIFICATE_EXPIRED`, ...).
- `check_readiness` (`crates/broker/src/tulorekisteri.rs`), the send-time
  gate, uses the same requirement functions and now names every unmet one
  instead of one sentence; it refuses on the same conditions as before plus a
  record-signing key that cannot be loaded.
- `aggountant explain TULOREKISTERI_NOT_READY` has an entry (both languages).
- Removed: the profile JSON example and the "Ohjelma ei erittele" paragraph
  from `skills/tulorekisteri/kayttoonotto.md`; the "no command" lines in
  `reference/status.md` and `tasks/report-travel-allowances.md` (fi, en).
- Tests: `readiness_is_itemised_with_owners`, `readiness_fails_closed`
  (broker), `tulorekisteri_cli.rs` (CLI).
