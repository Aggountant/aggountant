---
created: 2026-11-12
updated: 2026-11-12
type: bug
status: open
priority: high
provenance: other
provenance_detail: agent observation while verifying the Finnish user guide against the binary
source_ref: taskfleet:01m77aregmn19w8kh2w2zetxwf/observation:untagged-vat-posting
originating_run: 01m77aregmn19w8kh2w2zetxwf
originating_run_kind: spinoff
lane: vat-calculation
lane_seq: 0
collision: [crates/domain/src/vat/code.rs, crates/domain/src/vat/guards.rs]
preparation: ready
---

# Untagged VAT posting on a VAT account is silently left out of the VAT return

## Description

## Problem

A voucher sealed through `voucher draft` / `voucher seal` can put output VAT on
the output-VAT account (2939 Suoritettava arvonlisävero) without a `vat_code`
on that posting. Nothing refuses it, and the VAT return derived from the
period ignores it, because `DerivedVatFigures` sums only VAT-coded postings
(`crates/domain/src/vat/code.rs`). The return then under-reports VAT with no
warning.

Reproduced with the fictional Esimerkki Oy against 0.1.0 in a throwaway home:

1. `init`, then seal one sales voucher dated 2026-03-05: 1700 debit 2 480,00 €,
   3000 credit 1 976,10 €, 2939 credit 503,90 €, no `vat_code` on any posting.
   `voucher validate` says `ok`, `voucher seal` seals it as #1.
2. Close 2026-03 (`run start close-period`, `run answer … approve`).
3. `file-vat preview --period 2026-03` reports `sales_vat` 0.00, `net_vat`
   0.00, `payment_amount` 0.00, fields 307 and 308 both 0.00, although the
   ledger shows 503,90 € payable on 2939.

## Impact

Sales and other special entries go through the draft path, prepared by the
agent; the `capture-receipt` skill does not tell it to tag VAT legs with a
`vat_code`. A missed tag files a VAT return that is too low, which is silently
wrong bookkeeping reaching the Tax Administration. The month-end bank check
does not catch it, because the bank balance is right.

The user guide now tells the owner to check that VAT legs carry a VAT code
(`docs/user/fi/tasks/record-other-entries.md`), but a guard in code or a
period-level reconciliation would catch what a reader misses.

## Expected

One of: refuse sealing a posting to an account with a VAT role but no
`vat_code` (or require an explicit "not VAT-reportable" marker), or make
`file-vat preview` compare the VAT accounts' period movement with the derived
figures and refuse or warn on a difference. Which one is a product decision.
The invariant that VAT treatment is never inferred from an account number
stays; the check only detects the disagreement.
