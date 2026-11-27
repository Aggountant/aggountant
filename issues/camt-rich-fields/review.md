# Review: camt-rich-fields

- Skill: `/llm-review`, rigor **routine**. The change touches the bank-import
  parser whose output becomes sealed vouchers, but it adds no write path and no
  store migration; the identity invariants were tested directly (see Coverage).
- Scope: the slice's working diff against `a642c1a`
  (`crates/import/src/camt.rs`, `crates/import/src/lib.rs`, the CLI help line,
  tests and fixtures, guide pages, the `capture-receipt` skill, CHANGELOG),
  with post-change `camt.rs`, `lib.rs` and the CLI caller `bank_import.rs`.
- Finder: `gpt-6-astra`, one call. Verifier: not called. The single surviving
  candidate was confirmed against the code and fixed in a way that holds under
  either party-role convention, so a verifier verdict could not change the
  outcome. One model call in total.
- Risk class: bookkeeping source data (an entry's identity, direction and
  amount; voucher description).

## F1 (should-fix): a reversal's counterparty became the account owner

- Location: `crates/import/src/camt.rs`, the `counterparty` selection in
  `parse_entry`.
- Claim: for a credit that reverses an outgoing payment, the code read the
  debtor first (the role for money in) and fell back to the creditor only when
  the debtor was missing. ISO 20022 camt.05x usage keeps the original
  transaction's roles on a reversal, so a bank that lists both parties names
  the account owner as debtor, and the reversal's counterparty and default
  voucher description would name the company itself.
- Evidence: local read of the selection; the slice's own fixture listed only
  the creditor and so masked the branch.
- Impact: a wrong party in the default description of a reversal voucher and
  in the evidence the agent matches the original by. Amount, direction and
  identity unaffected; no existing import affected (reversals were refused
  before this slice).
- Fix: a reversal reads the role opposite its own direction first and falls
  back to its direction's role (a bank that swaps roles). Ordinary entries are
  unchanged. The reversal fixture now lists both parties, and
  `a_reversal_whose_bank_swaps_the_roles_still_names_the_other_party` covers
  the fallback.

## Dropped candidates

None; the finder proposed one candidate.

## Coverage

- Identity across builds: the `main` build (`a642c1a`) imported
  `rich-camt053-sample.xml` (card entry with `BkTxCd`, transfer with `Chrgs`,
  USD purchase with `InstdAmt`, returned payment with `RtrInf`) into a fresh
  home; this build re-imported the same file and a re-export of it into that
  home: 4 duplicate observations, 0 new vouchers, 0 conflicts, identical
  source fingerprints and drafts (only the attachment of the re-export
  differs). The same for the Holvi camt.052 sample, which carries `BkTxCd`.
  The fingerprints and descriptions the `main` build computed are pinned in
  `crates/import/tests/rich_fields.rs`.
- Files without the elements: the Holvi camt.053 sample's parse output from
  the `main` build is a golden file and matches byte for byte.
- Residual risk: the passthrough readers are lenient by design (an unreadable
  element is left out, never refused), so a bank's unusual shape shows as a
  missing field, not an error. Charge records of camt.053.001.02 and of later
  versions were tested; no real bank file other than Holvi's has been tried.
