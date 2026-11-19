# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While the version is 0.x, a minor version bump may contain breaking changes.

<!-- oss-changelog:unreleased-start -->
## [Unreleased]

### Changed

- `knowledge company update` and `retire` take the fact id positionally as
  well as with `--id`, or as `id` in the stdin JSON (which must agree with an
  argument given alongside it).
- `chart list` text output lists the accounts (number, name, kind), one per
  line.
- `report statements` prints the report to stdout when `--output` is omitted
  or `-`, defaulting to `--format native-json`; with `--json` the summary
  object carries the document in `content`.

<!-- oss-changelog:unreleased-end -->

## [0.1.0] - 2026-11-19

### Added

- First public release. The `aggountant` CLI and its bundled agent skills
  cover a Finnish limited company's bookkeeping: company home setup, bank
  statement import (camt.053/camt.052), receipt capture, period close, VAT
  returns filed directly to Vero.fi, Tulorekisteri travel-expense reports and
  monthly employer's separate reports, and the year-end close.
- Prebuilt binaries for macOS arm64 and Linux arm64/x86_64 (static musl) on
  GitHub Releases, with a shell installer.

[0.1.0]: https://github.com/aggountant/aggountant/releases/tag/v0.1.0
