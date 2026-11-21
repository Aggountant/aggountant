---
created: 2026-11-10
updated: 2026-11-21
type: feature
status: done
priority: normal
lane: unlaned
collision: [crates/domain/src/vat/expand.rs, docs/user]
closed: 2026-11-21
---

# Migration guide for moving books from another program

## Description

Narrowed by the product owner on 2026-11-21. The original idea was to
bring browser-automation helpers for other accounting and banking
programs into this repository (`helpers/<program>/`). That was dropped:
the helpers would work on one platform and one browser only, they break
whenever the vendor changes its interface, and the useful part is the
knowledge, not the code. A vendor's own export or API is the better
route where one exists.

What this issue delivers instead:

1. A user-guide page (`docs/user/fi` and `docs/user/en`) on moving books
   from another program into aggountant: what to collect (chart, opening
   balances, vouchers and their attachments, bank statements, filed
   returns), the rules for collecting it (read-only use of the old system,
   the user signs in and handles MFA, no cookies or tokens extracted, raw
   data kept outside git, completeness checked against an inventory and
   SHA-256 checksums), how it enters aggountant (SIE4 chart import, camt.053
   bank import, capture-receipt for attachments, opening-balance cutover),
   and short per-program notes for at least one bookkeeping program and one
   bank, using their own exports where they exist and naming what is only
   reachable through the browser.
2. Vendor-specific explanations moved out of core code comments (for
   example `crates/domain/src/vat/expand.rs`): where a comment explains a
   source program's chart rather than aggountant behaviour, it is
   rewritten neutrally or moved into the guide.

## Acceptance Criteria

- [x] Guide page in both languages, linked from the user-guide index and
      the onboarding pages where moving from another program comes up.
- [x] Core comments no longer depend on one vendor's chart.
- [x] No browser-automation code added.

## Resolution

### 2026-11-21T19:31:57Z · @issuectl

Guide page in both languages, vendor-neutral VAT comments; see result.md and review.md.
