---
created: 2026-11-24
updated: 2026-11-24
type: feature
reporter: jari
status: untriaged
priority: normal
provenance: agent:homebase-wrapup
source_ref: agent:homebase-wrapup/reporter:jari/id:wrapup-20261124-run-list-fields
---

# run list --json: include workflow, period and status; filter to open ru…

## Description

run list --json: include workflow, period and status; filter to open runs

`aggountant run list --json` returns only run ids. Finding which runs are open (awaiting_gate, running, needs_attention) takes one `run show` per run; on Itsellesi's home that was ~20 calls to find two awaiting-payment VAT runs and four stale July/August runs (24.11.2026).

Requested: each entry carries workflow, period, derived status and open gate id; `--open` (or `--status`) filters. Status stays derived from effect states as `run show` does.

Reported by jari via agent:homebase-wrapup.
