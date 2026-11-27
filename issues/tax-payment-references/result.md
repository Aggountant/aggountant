# Result: tax-payment-references

The company's Tax Administration payment details are typed master data, and
`file-vat` payment proposals take the account and the VAT reference from them.

```
aggountant tax-payments set [--dry-run] < tax-payments.json
aggountant tax-payments show
```

## What was built

- `crates/model/src/payment_reference.rs`: `PaymentReference`, a domestic
  reference (4–20 digits, 7-3-1 check digit, no leading zeros; Finanssiala,
  *Kotimaisen viitenumeron rakenne*) or an RF reference (ISO 11649, mod 97;
  Finanssiala, *Kansainvälisen viitteen käyttöönotto*). Serde runs the same
  validation.
- `crates/process/src/tax_payments.rs`: `masterdata/tax-payments.json`
  (`schema_version` 1; `payee_iban`, `general_reference`,
  `self_assessed_reference`, `prepayment_reference`,
  `payment_arrangement_reference`, all optional). Rules from vero.fi *Tili- ja
  viitenumerot yritys- ja yhteisöasiakkaille* (read 2026-11-27):
  - the payee is a Finnish IBAN with a valid checksum (vero.fi does not
    publish the accounts and they are not per tax type, so their identity is
    not checked);
  - general and tax-specific references are not stored together
    (`TAX_PAYMENT_REFERENCES_MIXED`), because the Tax Administration asks a
    company to pay all taxes one way;
  - the VAT reference is the general one when stored, otherwise the
    self-assessed one, never the payment arrangement reference (arrangement
    taxes cannot be paid with the general reference);
  - the Danske Bank account (`FI56…`, replaced 1 December 2026, closed at the
    end of May 2027) is a warning, `TAX_PAYEE_IBAN_RETIRING`.
- `file-vat start` / `replace`: the engine resolves the account (stdin
  `payee_iban` optional; a different one is `TAX_PAYEE_IBAN_MISMATCH`; none at
  all is `TAX_PAYEE_IBAN_MISSING`) and the reference. A first return without a
  VAT reference is refused before a Run exists
  (`TAX_PAYMENT_REFERENCE_MISSING`); a replacement pays nothing and does not
  need one. New Runs add `reference` and `reference_kind` to the payment
  proposal; `run show`, the journal reason and the closed month's
  `file-vat preview` (`payee_iban`, `payment_reference`, instruction) name
  them. Approving the filing of a payable Run whose account or reference was
  corrected since its start is refused (`TAX_PAYMENT_DETAILS_CHANGED`).
- Runs stored by earlier builds have no `reference` in their proposal: they
  hash, read and show exactly as before (test
  `a_run_stored_without_a_payment_reference_reads_unchanged`).
- No store migration: the file is new plaintext under `masterdata/`.
- Guide: new section "Tallenna Verohallinnon maksutiedot kerran" / "Store the
  Tax Administration payment details once" in `tasks/file-vat-direct.md`,
  error codes, commands, company-home layout, the OmaVero page; `vero-fi`
  skill.

## Review

`/llm-review` (finder `gpt-6-astra`, verifier `claude-fable-5-2`, two calls)
and its assessment are in `review.md` and `assessment.md`; all three confirmed
findings were fixed.

## Tests

`scripts/ci-local.sh` passes (fmt, clippy, deny, stable and MSRV build and
test, user-guide gate with site build and practice month).
