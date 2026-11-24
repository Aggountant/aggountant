---
created: 2026-11-24
updated: 2026-11-24
type: feature
reporter: jari
status: untriaged
priority: normal
provenance: agent:homebase-wrapup
source_ref: agent:homebase-wrapup/reporter:jari/id:wrapup-20261124-per-home-credentials
---

# Vero credential selection is user-wide; bind it to the company home

## Description

Vero credential selection is user-wide; bind it to the company home

The Vero/Incomes Register credential path comes from `AGGOUNTANT_VERO_CREDENTIALS_FILE` or the user-wide `~/.config/aggountant/config.json` (`vero_credentials_file`). One Linux account on Trude now holds two company homes (Itsellesi with a production certificate, Ergona without). Putting Itsellesi's path in the user-wide config would make Ergona's home see Itsellesi's certificate; the profile's activation fingerprint attestation probably stops a filing, but the selection itself is wrong.

Workaround in place: Itsellesi's repo-local `.aggountant-test/activate` exports the env var; no user-wide config exists.

Requested: the credential reference lives with the home (e.g. the vero-filing profile names a credential id resolved under a platform-local store), or the user config maps business id -> credential and status refuses a certificate whose business id/fingerprint does not match the home's profile, with a clear code.

Reported by jari via agent:homebase-wrapup.
