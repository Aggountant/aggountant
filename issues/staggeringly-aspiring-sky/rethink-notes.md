# Rethink notes (2026-11-13)

What the original was protecting: the rule that a filing whose outcome is
uncertain is resolved by asking the authority, never resent. It found a place
where a probably-accepted filing is recorded as a definitive rejection.

Checked against the current source. Every code-path claim held:
- the `200` arm in both FileVATReturn adapters;
- the broker's `Malformed` → `MALFORMED` mapping;
- `HTTP_2xx` for other 2xx;
- `deny_unknown_fields` on `FileVatReceipt`;
- the `OUTCOME_UNKNOWN`-only guard behind `complete-manual`;
- `status.rs` treating `MALFORMED`/`HTTP_2` as `outcome_unknown`.

Changed:
- The issue now opens with what goes wrong for the operator, so the product
  owner can judge it without reading code. The code paths follow for the
  implementer. The empty `## Description` heading is gone.
- Added things the implementer could not easily see:
  - fix it in the adapters, because the broker's `Malformed` arm also carries
    errors raised before anything is sent;
  - `status.rs` must keep its `MALFORMED`/`HTTP_2` rule for Runs journalled
    before the fix;
  - the `complete-manual` guard should cover those older codes too, ideally
    through one predicate shared with `status.rs`.
- The fix is stated as a decision: a 2xx is never a rejection, and
  `deny_unknown_fields` is dropped from the receipt. The original said
  "consider relaxing". The issue now lists acceptance criteria; the original
  had only a test hint. The criteria say which test source gives which case.
  The mock cannot produce an extra field, but the scripted adapter can.
- Title: "200" → "2xx", "definitive rejection" → "rejection".

Corrected: the "possible duplicate filing" effect was attributed to this case.
A throwaway test (since reverted) showed that a new Run for the same period also
re-sends after a genuine `OUTCOME_UNKNOWN` (timeout). The broker deliberately
does not short-circuit `Unknown`, and no Run checks another Run's unresolved
filing. So this fix alone does not stop a duplicate filing. That gap needs a
decision on when re-filing becomes allowed, so it is filed separately as
`vat-filing-resent-by-new-run-after-unknown` (unlaned intake). The issue says
this plainly.

Left untouched: frontmatter, lane and collision. The shared-predicate
suggestion would touch `crates/process/src/file_vat/status.rs`, which the
collision tokens do not name. Whoever schedules this may want to add it.
