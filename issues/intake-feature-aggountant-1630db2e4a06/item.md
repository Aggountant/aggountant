---
created: 2026-11-24
updated: 2026-11-24
type: feature
reporter: jari
status: untriaged
priority: normal
provenance: agent:wrap-up
source_ref: agent:wrap-up/reporter:jari/id:wrapup-20261124-run-list-fields
---

# run list --json: include workflow, period and status; filter to open ru…

## Description

run list --json: include workflow, period and status; filter to open runs

`aggountant run list --json` returns only run ids. Finding which runs are open (awaiting_gate, running, needs_attention) takes one `run show` per run; in a real home that was about 20 calls to find two VAT runs awaiting payment and four stale runs from older builds (24.11.2026).

Requested: each entry carries workflow, period, derived status and open gate id; `--open` (or `--status`) filters. Status stays derived from effect states as `run show` does.

Reported by jari via agent:wrap-up.
