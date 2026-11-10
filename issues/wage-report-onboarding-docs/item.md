---
created: 2026-11-10
updated: 2026-11-10
type: task
status: open
priority: high
lane: wage-reporting
lane_seq: 10
collision: [skills/vero-fi/SKILL.md, crates/cli/src/wage.rs]
---

# Tulorekisteri onboarding facts and safe record-hash input for file-wage

## Problem

The `vero-fi` skill's Tulorekisteri section (`skills/vero-fi/SKILL.md`) is
vaguer than the actual onboarding and first use:

- In the certificate service the replacement application is made under
  Tuotantovarmenteet → organisation → Näytä tiedot → **Tee korvaava hakemus**.
  The form lists the Tulorekisteri interfaces as Palkkatiedot (SFTP / Web
  service) and Etuustiedot (SFTP / Web service); the right choice for
  `file-wage` is **Palkkatiedot (Web service)**, and the existing Vero API
  selection must stay ticked. The confirmation lists the granted rights as
  "Vero API (Web service)" and "Palkkatiedot tuottaja (Web service)". The
  application can be approved immediately, and no new certificate is needed.
- A `file-wage` delivery of travel reimbursements (income types 311 and 331)
  has been accepted synchronously in production: delivery status 3, the item
  among the valid items as version 1, with delivery and item ids returned. The
  skill still describes the channel as untried in production.
- `file-wage approve` takes the `record_hash` as a command-line argument, so it
  ends up in shell history and agent transcripts. The skill does not say how to
  avoid that.

## Expected

The skill describes the actual UI path and labels, states that production
delivery works, and gives safe hash handling. `file-wage approve` reads the hash
from stdin or from the stored preview, so it never has to appear in argv.

## Acceptance criteria

- [ ] Skill text matches the certificate-service flow and labels above.
- [ ] Production use described without personal data.
- [ ] `file-wage approve` can take the hash without argv (stdin or stored preview), or the plan records why not.
