# Assessment of the filing-output-markers review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Class list steered toward a 0 % domestic voucher | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX the wording — applied; SPIN-OFF the Domain rule (`capture-domestic-input-eu-class`) |
| F2 | Doubled `VAT_CODE:` on other Domain refusals | CONFIRMED | OCCASIONAL | BETTER | NONE | HIGH | FIX — applied |

FIX: 2   FIX (with care): 0   SPIN-OFF: 1   DISCUSS: 0   DROP: 0

- F1: the wording is this slice's; changed so the message informs rather than
  instructs. Whether `domestic-input` may carry an EU or zero-rated class is a
  data-integrity rule in the Domain expansion, outside this slice's files, so
  it is filed unlaned for triage rather than half-fixed in `process`.
- F2: the issue's complaint was the repeated code; fixing `map_domain` removes
  it for every Domain refusal on the capture paths, not just the unknown class.

## Simplicity

- Reused: the existing dated rate table (one new public reader,
  `vat_rate_classes`, instead of a second list of classes); the existing
  capture error family (`CAPTURE_*`) and its CLI classification; the existing
  `build_scripted_broker_path`, the one place a scripted broker is built, as
  the single point that marks the seam.
- Repetition: the seam message is one constant used for the JSON field, the
  error field and the stderr warning. The sensitivity reason is one constant on
  the one preview struct, so `preview` and `start --dry-run` both carry it.
- Left out: no per-command seam flags, no stderr JSONL log channel for one
  warning (JSON callers get the field), no filtering of classes per treatment
  in `process` (that rule belongs in the Domain, see the spin-off).
