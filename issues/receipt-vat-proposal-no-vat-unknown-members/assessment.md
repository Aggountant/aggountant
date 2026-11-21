# Assessment of the receipt-vat-proposal-no-vat-unknown-members review

The review produced no findings, so there is nothing to triage.

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

The verifier's note that a duplicate `type` key is not refused once the
input has gone through `serde_json::Value` is not a member drop and reaches
no production path (stdin is parsed with `from_str`). Not filed.

## Simplicity

- Reuses: serde's `deny_unknown_fields` and the existing `deny_members`
  helper. No new types; `NoVat` stays a unit variant, so no call site changes.
- Repetition removed: the helper had one private copy in `domain`; it now has
  one owner in `aggountant_model::de`, used by both enums, instead of a second
  copy in `process`. `model` is the bottom crate, so any later user
  (including `VatExpansionTreatment`) can reach it without a new dependency.
- Left out on purpose: tightening `VatExpansionTreatment::NoVat`. It is read
  from sealed vouchers, and refusing members there needs a check that no
  stored record carries them. Also left out: the same serde gap on non-VAT
  unit variants in `domain` (`EmployerReportContent::NoWagesPayable`,
  `EmployerReportAction::New`); reported as a spin-off proposal, not changed
  here.
