# Assessment of the cli-stdin-input-help review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | capture-receipt start example sealed VAT the return leaves out | CONFIRMED | COMMON | NEUTRAL | NONE | HIGH | FIX — applied |
| F2 | employee help promised an encrypted store | CONFIRMED | COMMON | NEUTRAL | NONE | HIGH | FIX — applied |
| F3 | unknown fields inside vat_treatment are ignored | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX (help) here, SPIN-OFF (code) — `vat-treatment-deny-unknown-fields` |
| F4 | wrong error code for a mismatching acknowledgement | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 3   FIX (with care): 0   SPIN-OFF: 1 (F3 code half)   DISCUSS: 0   DROP: 0

- F1 is the one that matters most: the help is meant to replace the skills'
  copies, and an agent copying the example would seal a deduction the VAT
  return omits. Fixed and covered by an end-to-end test through
  `file-vat preview`.
- F2 also closes the open bug `employee-store-plaintext-help`, whose two
  acceptance criteria (truthful protection, `employee_id`) are both met.
- F3 is data integrity, so it is not dropped: the help now tells the truth and
  the code fix is filed unlaned with the reproduction. It is a one-line serde
  change in `crates/domain`, outside this worker's file set.

## Simplicity

- Reused: the existing `after_help` mechanism and the voucher INPUT format; the
  examples are checked against the very types the handlers parse, so there is
  no second schema.
- Removed repetition: all INPUT texts are in one module (`input_help.rs`); the
  voucher texts moved there unchanged, the knowledge `EXAMPLE` texts became
  INPUT sections, and the inline JSON shape in the employer-report configure
  description was replaced by the INPUT section. The Vero transport note is a
  macro so `file-vat start` includes it without a copy.
- Left out: generating help from the serde types (no schema crate, and the
  field meanings are prose anyway), and runtime checks tying `read_stdin_json`
  to a help section; the tree-walk test covers that at build time.
