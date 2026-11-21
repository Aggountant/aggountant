# Result: distinctly-tired-hearing

Set-up no longer offers keeping the previous system's chart of accounts.

- `skills/aggountant-init/SKILL.md`: the chart decision is gone from the
  basic details; the mapping paragraph says the books always use the default
  chart and the previous accounts are mapped onto it in the opening voucher,
  that the agent tells the user which distinctions merge, and that a merged
  tax account and shareholder debt need an agreed separation. 935 → 929 words.
- `docs/user/{fi,en}/start/set-up-company.md`: step 3 no longer asks about
  the chart; step 4 explains the mapping and the lost distinctions; a
  limitation says the old chart cannot be imported. The stale "no separate
  upgrade command" limitation now links to the upgrade page.
- `docs/user/{fi,en}/tasks/upgrade.md`: "a company's own chart" became "the
  home's chart of accounts" in the `unavailable` limitation.
- `docs/user/{fi,en}/reference/status.md`: entry "A company's own chart of
  accounts cannot be imported" pointing at `sie4-chart-import`.

Not changed: `tasks/file-vat-omavero.md` (en) still mentions marking
settlement accounts "with the company's own chart"; it is outside this
issue's named files and describes a home whose chart lacks settlement roles,
which can still occur for older homes.

Verified: `scripts/ci-local.sh` passed; `scripts/check-user-guide.sh` passed
after the final edits. Review: `review.md`, assessment: `assessment.md`.
