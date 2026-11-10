# Contributing to aggountant

aggountant is a command-line tool and a set of agent skills for running a
Finnish limited company's bookkeeping with an AI agent. Bug reports, questions
and pull requests are welcome. This page covers what an outside contributor
needs; the rules for working in the code are in [AGENTS.md](AGENTS.md) and in
the `AGENTS.md` files under `crates/`.

## Reporting issues

- **Bugs and feature requests:** open a
  [GitHub issue](https://github.com/aggountant/aggountant/issues/new/choose)
  with the bug or feature form. Include the `aggountant --version` output, the
  command you ran and its `--json` output. Never paste real bookkeeping data,
  bank statements, personal identity codes or Vero.fi credentials; a minimal
  made-up example is enough.
- **Questions and ideas:** use
  [GitHub Discussions](https://github.com/aggountant/aggountant/discussions).
- **Security vulnerabilities:** do not open a public issue. Follow
  [SECURITY.md](SECURITY.md).

The maintainers track accepted work in an in-repository tracker (`issues/`,
managed with `issuectl`). When your report is accepted it is mirrored there,
and the GitHub issue may be closed with a pointer to the internal item; it is
not being dismissed.

## Development setup

You need Rust (the minimum supported version is 1.89, declared as
`rust-version` in `Cargo.toml`) and both the stable and the 1.89.0 toolchains
for the full gate:

```sh
rustup toolchain install stable 1.89.0 --profile minimal --component clippy,rustfmt
cargo build --workspace --locked
```

## The green gate

Every pull request must pass the same gate the maintainers run locally:

```sh
scripts/ci-local.sh
```

It runs rustfmt, clippy with warnings denied, `cargo-deny` (advisories, bans,
licenses, sources; it downloads a pinned, checksum-verified `cargo-deny` if
one is not installed), and a locked build and test of the whole workspace on
stable and on 1.89.0. GitHub Actions runs the same checks for each pull
request, with build and test on both Linux and macOS.

Tests use synthetic data only. A fixture never contains a real company's
figures, counterparties, account numbers or personal data.

## Pull requests

- Branch from `main` and open the pull request against `main`. Keep one
  logical change per pull request.
- A change to a command's behaviour updates the user guide under `docs/user/`
  in the same pull request.
- Bookkeeping correctness comes first. If a change could alter a sealed
  voucher, a VAT computation or what is sent to an authority, say so in the
  description and explain how you verified it.

## Commit messages

The log uses a short `area: summary` subject in the imperative, for example
`cli: reject an empty voucher description`, followed by a body that explains
why. Maintainers add a `Refs-Issue: @<slug>` or `Fixes-Issue: @<slug>`
trailer for the internal tracker; you do not need to.

## Changelog

`CHANGELOG.md` is curated by the maintainers from merged changes and their
tracker items. You do not need to edit it; describe the user-visible effect in
the pull request and it will be written up at release time.

## Sign-off (DCO)

Contributions are accepted under the
[Developer Certificate of Origin](https://developercertificate.org/). Sign off
each commit to certify that you wrote it or otherwise have the right to submit
it under the project's license:

```sh
git commit -s
```

This adds a `Signed-off-by: Your Name <you@example.com>` trailer. A pull
request with unsigned commits cannot be merged until they are signed off
(`git rebase --signoff main` fixes an existing branch).

## Licensing

By contributing, you agree that your contributions are licensed under the
project's MIT license. See [LICENSE](LICENSE).
