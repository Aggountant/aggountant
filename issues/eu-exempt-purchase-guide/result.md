# Result

The guide and the refusal message now say how to record a service bought from
another EU state whose supply the Finnish VAT Act exempts, for example an
insurance or financial service. It is recorded as `{"type":"no-vat"}`, not as a
reverse charge, and it is not reported under "Palveluostot muista EU-maista".
Verohallinto's VAT return instructions (updated 1 Jan 2026) list exempt
general-rule services under the box's *Tässä ei ilmoiteta*, and section 9 of
detailed guidance 48679 describes the box as services on which the buyer owes
Finnish VAT. So the issue's premise holds and no escalation was needed.

Goods are different, which the issue did not anticipate. The same instructions
say an exempt intra-Community acquisition of goods is reported under
"Tavaraostot muista EU-maista". No treatment models that acquisition. The
guide says so and refers the user to their bookkeeper, and no message advises
`no-vat` for goods.

- `crates/domain/src/vat/expand.rs`, `check_treatment_rate_class`: message text
  only. `reverse-charge`/`reverse-charge-partial` with `zero-rated` and scope
  `eu-services` now points at `{"type":"no-vat"}` with no other member. It
  also says that an invoice without VAT is still a reverse charge unless the VAT
  Act exempts the supply. Scopes `eu-goods` and `non-eu-services` keep the
  "not a modelled reverse-charge case" wording. The `domestic-input` +
  `zero-rated` message names the exempt EU service as a `no-vat` case and
  excludes the exempt goods acquisition. Which purchases are refused and which
  are accepted is unchanged.
- Tests (`tests_expand.rs`, `treatment_class_refusal_names_the_treatment_that_fits`):
  the services scope (full and partial) advises `no-vat`, the goods scope does
  not, and the domestic message carries the goods exclusion.
- User guide, fi/en: the VAT treatment concept page has a paragraph on exempt
  EU services, with links to both Verohallinto sources, and one on exempt goods
  acquisitions. The `VAT_TREATMENT_CLASS_MISMATCH` row in the error-code guide,
  which is also the bundled hint, names both cases.

Review: `review.md` (finder gpt-6-astra, verifier claude-fable-5-2; one
must-fix on the goods scope fixed before landing). `scripts/ci-local.sh`
passed.
