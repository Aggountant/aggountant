---
schema_version: 2
status: approved
maturity: mvp
ecosystems: [rust]
targets:
  - {ecosystem: rust, package: aggountant, registry: gh-releases, adapter: cargo-dist}
distributions:
  - adapter: cargo-dist
    installers: [shell]
versioning: zerover
changelog: {mode: curated, source: issuectl-trailers}
release: {model: gated, layout: single}
conventional_commits: false
contribution_provenance: dco
provenance_level: none
dependency_bot: dependabot
health_badges: [ci, license]
license: MIT
docs_site: none
---

> **Approved 2026-11-10.** The decisions below are Jari Mustonen's, made for the
> publication under `aggountant/aggountant`. Refresh with `/shipshape-init`; a change to a dial is the
> maintainer's call.

## Rationale
- **maturity: mvp** — `shipshape facts` reports CI, one SemVer tag (`v0.1.0`), two recent committers and `inferred_maturity: mvp`. The CLI runs the books of one real company, but it has one maintainer and no outside users yet, so it stays below production.
- **ecosystems: [rust]** — a Rust workspace rooted at `Cargo.toml`; the shipped product is the `aggountant` binary from `crates/cli`, with the skills in `skills/` bundled into it.
- **targets: gh-releases via cargo-dist only** — prebuilt binaries are attached to GitHub Releases of `aggountant/aggountant` by the tag-triggered `release.yml` that cargo-dist generates. No crates.io publication (maintainer decision): every member crate keeps `publish = false`, which also keeps the normalizer from expanding a crates.io target.
- **distributions: cargo-dist, shell installer** — the maintained default platforms (`aarch64-apple-darwin`, `aarch64-unknown-linux-musl`, `x86_64-unknown-linux-musl`) are used by omitting `platforms`. No Homebrew tap: none has been decided, and adding one later means a `homebrew` target, a `homebrew_tap` slug and an existing tap repository.
- **versioning: zerover** — 0.x versions by decision; breaking changes bump the minor version.
- **changelog: curated + issuectl-trailers** — single maintainer, `issues/` exists and commits carry `Refs-Issue` trailers, so release notes are written by hand from issue and commit context.
- **release.model: gated, layout: single** — a human cuts every release; all crates inherit one workspace version from `[workspace.package]`.
- **contribution_provenance: dco** — outside contributions sign off under the Developer Certificate of Origin (`git commit -s`); no CLA.
- **provenance_level: none** — cargo-dist's `github-attestations` adds build attestations to the release assets, but no SLSA level is claimed in the contract at mvp.
- **dependency_bot: dependabot** — the mvp default; the hosted CI that reviews its pull requests runs only in `aggountant/aggountant`.
- **health_badges: [ci, license]** — no registry target, so no registry badge.
- **license: MIT** — `[workspace.package]` declares `license = "MIT"`; copyright holder Jari Mustonen.
- **docs_site: none** — the user guide is Markdown under `docs/user/`, built into an Astro Starlight site at https://aggountant.dev by `site/` and `.github/workflows/pages.yml`. Shipshape's `docs_site` enum has no Starlight value, so the contract says `none`.

## Release notes
- **Hosted CI and releases run only in `aggountant/aggountant`.** Jobs carry `if: github.repository == 'aggountant/aggountant'`, so a fork or a private mirror spends no Actions minutes; local gates run through `scripts/ci-local.sh`.
- **A pushed tag publishes binaries.** cargo-dist's `release.yml` builds and attaches the three platform archives and the shell installer as soon as a `v*` tag reaches the public repository. A published GitHub Release can be deleted, but downloaded binaries cannot be recalled, so cut only from a green `main`.
- **No crates.io.** If that ever changes, remove `publish = false` from the crates to publish, add explicit crates.io targets here, and review crates.io permanence before the first upload.
