---
created: 2026-11-24
updated: 2026-11-24
type: feature
reporter: jari
status: untriaged
priority: normal
provenance: agent:homebase-wrapup
source_ref: agent:homebase-wrapup/reporter:jari/id:wrapup-20261124-home-location
---

# Home records which machine holds its sealed data; other checkouts are t…

## Description

Home records which machine holds its sealed data; other checkouts are told where to go

A company home's sealed data (fiscal.sqlite, artifacts, outbox) is kept on exactly one machine, while the repository is cloned to others: Itsellesi moved from Haapa to Trude on 2026-11-24, leaving a frozen rollback copy on Haapa; Ergona's books live only on Trude, other founders have checkouts without data. Today this is only prose in each repo's AGENTS.md. An agent starting in the wrong checkout either finds no binary/home (`HOME_NOT_INITIALIZED`) or, worse, a stale copy it could write to (split-brain).

Observed 24.11.: a session started in Trude's itsellesi clone (no store then) and had to discover by hand that the home was on Haapa.

Requested: a tracked marker in the home (e.g. in `.aggountant/`) naming the canonical host and path, written by `init`/a `home relocate` command. On any other host, commands that read or write the fiscal store refuse with a distinct code (e.g. `HOME_ELSEWHERE`) whose hint names the host, path and an ssh command; a frozen former copy refuses writes the same way. Read-only `--help`/`explain` keep working.

Reported by jari via agent:homebase-wrapup.
