# A company home written by the `main` binary before fiscal-year definitions

Written by `aggountant` built from commit 3a177c6, whose code is `main` at 505aa12 (fiscal store schema v6), so
`tests/existing_home.rs` can prove a home from before the
`non-calendar-fiscal-year` change opens, migrates and keeps reading as calendar
years with no user action. Synthetic data only: `init`, three vouchers
(2026-01-05, 2026-01-20, 2026-02-10) and a sealed close of `2026-01`.

Tests copy this directory to a temporary home before opening it; never open it
in place, because opening migrates the database.
