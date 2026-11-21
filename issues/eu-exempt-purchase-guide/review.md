# Review: eu-exempt-purchase-guide

`/llm-review`, routine rigor (user-guide prose and one error message; no
behaviour change, but the text gives tax advice, so every tax claim was checked
against Verohallinto's pages). Two model calls: finder `gpt-6-astra`, verifier
`claude-fable-5-2`.

## Primary sources checked (2026-11-21)

- Verohallinto, "Näin ilmoitat arvonlisäveron" / "Instructions for completing
  VAT returns" (vero.fi, updated 1.1.2026). "Palveluostot muista EU-maista":
  services on which VAT is paid to Finland under the reverse charge; *Tässä ei
  ilmoiteta* lists "yleissäännön mukaisia palveluja, joiden myynti on
  veroton". "Tavaraostot muista EU-maista": "Ilmoita myös verottomat
  yhteisöhankinnat."
- Verohallinto detailed guidance 48679, Palvelujen ulkomaankaupan
  arvonlisäverotus (VH/4150/00.01.00/2021), section 9: the EU services box
  holds services on which the buyer is liable for Finnish VAT.

The issue's premise holds. Nothing in the sources requires an exempt EU service
purchase to be reported anywhere in the return, so the issue did not need
escalating. The goods side differs (see finding 1).

## Findings and what was done

1. **Must-fix, confirmed (finder).** The first draft of the message for
   `reverse-charge` + `zero-rated` advised `no-vat` for every scope. An exempt
   intra-Community acquisition of goods is still reported in "Tavaraostot muista
   EU-maista" (checked on vero.fi), so for `eu-goods` that advice would drop a
   reportable amount. Fixed: only scope `eu-services` advises `no-vat`.
   `eu-goods` and `non-eu-services` keep the earlier "not a modelled
   reverse-charge case" wording. A test pins that the goods scope does not
   mention `no-vat`.
2. **Should-fix, confirmed (verifier).** The general "no Finnish VAT → `no-vat`"
   sentence in the `domestic-input` + `zero-rated` message and in the
   error-code rows also covered exempt goods acquisitions. Fixed: the message
   and both rows now carve out the exempt intra-Community goods acquisition,
   and the concept page has a paragraph on it.
3. **Should-fix, confirmed (verifier).** "Exempt" was not tied to Finnish law,
   and a 0 % reverse-charge invoice reads as "veroton". Fixed: the message and
   prose say the Finnish VAT Act exempts the supply and that an invoice without
   VAT does not by itself make a purchase exempt. The message also says
   "with no other member" so that a partial treatment does not carry its
   deductibility member into `no-vat`. A partial-treatment test was added.
4. **Nits, applied.** The rows now name the box instead of "the VAT return".
   "Section 9 limits" was changed to "describes". The English page title is
   used as the link text. "accountant" became "bookkeeper" and
   "nollaverokantaluokka" became "nollakantaluokka" for consistency.

Dropped: none. Not verified against Finlex: the AVL section numbers for
financial and insurance services. The text names no section numbers, only that
the VAT Act exempts these services.

## Simplicity

The change reuses the existing refusal and its match. It adds one arm, for the
only scope where the advice differs, and no new code path. The rule is written
once in the message, once on the concept page and once in the error-code row,
which is the hint the binary bundles. Those are the three homes the user-guide
rules ask for.
