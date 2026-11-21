# Assessment of the capture-domestic-input-eu-class review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | suggested partial treatment omits its deductibility | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX — applied |
| R1 | a replay of an entry sealed with such a pair before the change is refused | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | DROP (Rule 1b: RARE, no readability gain) |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

- F1: an agent following the message for a partial treatment would get a parse
  error. Fixed in the same landing, with a test.
- R1: needs a retry of the exact same batch spanning the upgrade, over an entry
  that was sealed wrong. The result is a loud refusal that leads to correcting
  the wrong voucher, not wrong books. Making replay skip the check would put a
  second, check-free path in front of the seal. Not filed.

## Simplicity

- One rule, one place: `check_treatment_rate_class` runs in
  `resolved_treatment_percent`, the step both public expansion entry points
  already share, so capture-receipt, bank import and the ledger's seal-time
  re-expansion all get it without their own copies. Nothing filters in
  `process` or `import`.
- The rule is by class, not by percentage: two closed lists over the dated
  table, with a test that every table class is in exactly one. That keeps a
  dated 0 % domestic rate (passenger transport in 2023) valid and forces a
  decision when a class is added.
- The non-EU services slot restriction stays where it was
  (`vat_rate_class_field`); the new check does not repeat it.
- capture-receipt's `CAPTURE_VAT_CLASS_UNKNOWN` message now lists
  `PURCHASE_RATE_CLASSES` from the domain rather than every class, so the
  process layer does not keep its own list.
- Deliberately left out: changing `vat-rates.json` (its hash is sealed into
  every voucher), mapping every capture domain error to its own code (other
  codes stay `INVALID_ANSWER` for compatibility), and the legacy tagged
  `purchase-zero-rated` path, which the bank import supports on purpose.
