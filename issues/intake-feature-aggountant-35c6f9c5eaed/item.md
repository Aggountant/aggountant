---
created: 2026-11-24
updated: 2026-11-24
type: feature
reporter: jari
status: untriaged
priority: normal
provenance: agent:wrap-up
source_ref: agent:wrap-up/reporter:jari/id:wrapup-20261124-home-location
---

# Home records which machine holds its sealed data; other checkouts are t…

## Description

Home records which machine holds its sealed data; other checkouts are told where to go

A company home's sealed data (fiscal.sqlite, artifacts, outbox) is kept on exactly one machine, while the repository is cloned to others: a company home that moves from one machine to another leaves a frozen rollback copy behind, and other people working on the same company repository have checkouts without data. Today this is only prose in each repo's AGENTS.md. An agent starting in the wrong checkout either finds no binary/home (`HOME_NOT_INITIALIZED`) or, worse, a stale copy it could write to (split-brain).

Observed 24.11.: a session started in a clone without a store and had to discover by hand which machine held the home.

Requested: a tracked marker in the home (e.g. in `.aggountant/`) naming the canonical host and path, written by `init`/a `home relocate` command. On any other host, commands that read or write the fiscal store refuse with a distinct code (e.g. `HOME_ELSEWHERE`) whose hint names the host, path and an ssh command; a frozen former copy refuses writes the same way. Read-only `--help`/`explain` keep working.

Reported by jari via agent:wrap-up.
