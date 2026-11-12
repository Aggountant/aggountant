# Reference masterdata (git-native, non-PII)

These files are **aggountant's git-native reference masterdata** (ADR-1 §Masterdata).
They are plaintext on purpose: commit them, `git diff` them, `git blame` them,
PR-review changes to the chart of accounts.

- `chart-of-accounts.json` — the chart of accounts (account number, name, kind),
  one account per line for clean diffs, plus a `chart_date` edition marker (ISO
  date) so you can tell which edition this is at a glance.
- `chart_version` — a content hash over the active chart; each sealed voucher pins the
  exact `chart_version` it was recorded against. It is derived from chart content.
- `vat-account-roles.json` — reviewed VAT account roles keyed by the exact chart version.
  Adoption fails closed unless the active chart has complete, chart-valid role metadata.
- `chart-snapshots/<sha256>.json` — immutable copies used to resolve each voucher under
  the account kinds it actually sealed. Never overwrite a snapshot; chart evolution
  creates another content-addressed file.

**No PII here.** Personal data (HETU, IBAN, tax cards, employee identity) never
lives in git — it goes in a separate, *erasable* encrypted store (GDPR). Do not
add personal names or identifiers to this tree.
