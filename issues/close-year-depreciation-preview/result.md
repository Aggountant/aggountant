# Result: close-year-depreciation-preview

`aggountant close-year start --dry-run` takes the same stdin JSON as the start
and writes nothing: no Run, no state, no voucher.

It returns `depreciation` (method, `rate_percent` and amount per asset),
`asset_register` (opening book value, depreciation and closing book value per
asset, and whether the asset is carried or from the input),
`opening_register_year`, `accruals`, `corrections`, `adjustment_period`,
`blockers`, `would_start` and a `message`.

Unlike the start, it does not stop at the first blocker: open months, open
obligations and unresolved intake are listed in `blockers` while the figures
are still computed, so the depreciation can be agreed with whoever prepares the
financial statements before the months are closed. A malformed input (unknown
field, unknown asset class, duplicate id, a figure that disagrees with the
register) is an error, as it is for the start.

The start and the dry run share the same functions for the blockers, the asset
plan, the adjustment period and the state, so they cannot compute different
figures. `close-year start --help` documents the flag and the output fields,
and the close-year skill tells the agent to run it first.

Built together with `fixed-asset-register`; the review and its assessment are
in `issues/fixed-asset-register/`.
