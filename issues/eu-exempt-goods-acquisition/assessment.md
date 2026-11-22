# Assessment of the eu-exempt-goods-acquisition review

Assessed against the landed commit "vat: book an exempt intra-Community goods
acquisition with zero tax" (R1 and the R2 mitigation are folded into it).

| # | Item | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| R1 | Doc comment said 305 does not move | CONFIRMED | ALWAYS | IMPROVES | NONE | HIGH | FIX — applied |
| R2 | Taxable goods mistyped as `zero-rated` now seal | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MEDIUM | FIX_WITH_CARE — mitigated in text, not in code |
| R3 | Chart without an `eu-goods` target refuses the exempt booking | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | DROP |

FIX: 1   FIX (with care): 1   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

- R1: wording; the comment now says 305 is reported as zero and 307 does not
  move.
- R2: the issue asks for exactly this booking, and the Domain has no input
  that separates an exempt acquisition from a mistyped taxable one; a second
  confirmation member would be the same claim written twice. The mitigation is
  where the choice is made: INPUT help for capture-receipt and the bank import
  and the concept page (fi/en) reserve it for a confirmed exemption and say an
  invoice without VAT does not make the acquisition exempt; the comment on the
  constant records the trade-off. The services scopes keep refusing
  `zero-rated`, so the mistake stays caught there.
- R3: consistent with every other EU goods booking on such a chart, and the
  message names the fix. Making the account lookup lazy would change the
  verified arithmetic path for a case no source we use produces.

No confirmed data-integrity finding remains open. No follow-up issue filed.

## Simplicity

- Reused: the existing `reverse-charge` treatment and `eu-goods` scope, the
  existing `zero-rated` rate class and its 0 % edition, and the unchanged
  filing projection, which already files a `reverse-charge`/`eu-goods`
  source amount in 312 with its (here zero) tax in 305. No new treatment,
  scope, class, stored variant or migration; vouchers of this shape sealed
  before the class check are byte-identical to new ones.
- Repetition: the booking string has one owner, `EXEMPT_EU_GOODS_ACQUISITION`,
  quoted by every refusal; a test pins that it parses to the admitted
  treatment. The admission is one match arm in `check_treatment_rate_class`.
- Left out: a dedicated `exempt-eu-goods` treatment or scope (would add a
  stored variant, verifier and projection arms for no difference in the
  return), and lazy chart-target lookup (R3).
- `no-vat` members: one attribute using the existing
  `aggountant_model::de::deny_members`, the helper the domain's `VatTreatment`
  already uses.
