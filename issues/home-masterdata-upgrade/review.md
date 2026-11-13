# Review: home masterdata upgrade

- Target: the implementation commit of this issue (`home upgrade`, the
  `GitMasterDataRefStore::upgrade` store path and the `open` refactor into
  shared readers).
- Finder: gpt-6-astra (one call, `--task review`). Verifier: claude-fable-5-2
  (one focused false-negative call). Two model calls in total.
- Rigor: critical. The command writes into a production company's reference
  masterdata, which sealed vouchers resolve against.

## Finder

No findings. It compared the old and new `open` and found the same checks,
error codes and order: active chart, stored version, role file, snapshots,
role activation.

## Verifier (focused probe of the write boundary)

The question was whether any realistic on-disk state (older layout, a company
chart and role file, crash leftovers, an empty or missing `chart-snapshots/`, a
snapshot with the right name but other formatting) lets apply modify, replace or
delete an existing file, change the active chart, or write in a dry-run. No
such path was found.

### F1 (should-fix): a failed temporary write leaves a stray file

- Location: `crates/ledger/src/store/git_masterdata.rs`, `create_new_file`.
- Claim: `write_file_synced(&temporary, bytes)?` returned before the cleanup,
  so a write or fsync error (a full disk, say) left a possibly partial
  `.<name>.upgrade.<pid>.<n>` beside the target.
- Evidence: checked locally. The early return skipped `remove_file`. The
  leftover does not break `open` or a re-run: `read_snapshots` reads only
  `*.json`, and nothing reads dotfiles in `reference/`. It could still be
  committed by mistake.
- Fix: remove the temporary file when the write fails. A crash between the
  write and the link can still leave the file behind; that window comes with
  the temp-plus-link approach.

## Dropped

None.

## Coverage

Read: the whole diff, `open` before and after, `init`, the CLI wiring and the
tests. Tested: the ledger and CLI upgrade tests and the full `ci-local.sh`.
Not covered: filesystems without hard links. The home has to be on a local
filesystem anyway (`crates/ledger/AGENTS.md`); there the upgrade fails cleanly
and writes nothing.
