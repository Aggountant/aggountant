---
created: 2026-11-10
updated: 2026-11-10
type: bug
status: open
priority: high
lane: vat-filing
lane_seq: 10
collision: [crates/process/src/file_vat.rs]
---

# file-vat preview tells the user aggountant cannot file

## Problem

`file-vat preview` returns a fixed `filing_instruction`:

> Enter these numbered VAT fields manually in OmaVero, or use an independently
> prepared Ilmoitin.fi file; aggountant does not create or upload that file.

(`crates/process/src/file_vat.rs`). Since the Vero API route exists, this is
wrong whenever Vero.fi readiness is configured: the same preview can be fed to
`file-vat start` and approved for direct filing. An agent reading the preview is
told the opposite of what the tool can do.

## Expected

The instruction describes the actual channels: with Vero.fi configured, point to
`file-vat start` and the `ApproveFiling` gate; otherwise describe the manual
route (`complete-manual`). Preview stays read-only and offline, so it may say
that readiness was not checked instead of calling the network.

## Acceptance criteria

- [ ] Preview text no longer claims aggountant cannot file.
- [ ] The text distinguishes the API route from the manual route without network access.
- [ ] Tests updated.
