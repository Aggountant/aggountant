# Result

Delivered as narrowed on 2026-11-21.

- New user-guide page `start/move-from-another-program.md` in Finnish and
  English: what to collect (with the Kirjanpitolaki 2:10 § retention periods),
  the collection rules (inventory first, user signs in and handles MFA, no
  cookies or tokens copied, old system read-only, refused functions treated as
  boundaries, file type and period read from content, originals kept
  unchanged, SHA-256 manifest checked on two copies, archive outside git but
  inside backups), how material enters aggountant (opening balances via
  set-up, camt import from the day after cutover, receipts as usual), and
  notes for Briox and Holvi.
- Linked from both front pages ("Aloita tästä" / "Start here"), from the
  prerequisites of `set-up-company` and from the `historical-import` line of
  `reference/status.md`. `docs/user/AGENTS.md` lists the page under `start/`.
- The guide says plainly that aggountant has no SIE4 or chart import and no
  historical voucher import; those stay with `sie4-chart-import` and
  `historical-import`.
- Core comments no longer depend on one vendor's chart:
  `crates/domain/src/vat/expand.rs` and `roles.rs` describe the account pairs
  as legacy-expander or chart-snapshot metadata; the role tests use a neutral
  "imported chart" fixture name; the model test's source chart version is
  neutral; two dangling references to a removed design page in
  `crates/import` were dropped. No behaviour change; tests pass.
- No browser-automation code was added.

Review: `review.md` in this directory.
