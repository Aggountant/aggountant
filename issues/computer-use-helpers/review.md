# Review: migration guide and vendor-neutral VAT comments

- Scope: the working-tree diff of this slice: the new guide page in `docs/user/fi/start/` and
  `docs/user/en/start/`, its links from both front pages, `set-up-company`
  and `status`, the `docs/user/AGENTS.md` structure row, and comment and
  test-name changes in `crates/domain/src/vat/{expand,roles,tests_roles}.rs`,
  `crates/model/src/lib.rs` and `crates/import/src/{lib,camt053}.rs`.
- Method: `/llm-review`, routine rigor (documentation plus comment-only
  code change; no data, filing or migration path changes). Finder
  `gpt-6-astra`, one call. No verifier call: the single candidate was
  settled against the primary source directly (below).
- Risk class: user-facing guidance on keeping statutory records; no
  behaviour change.

## Confirmed finding

### should-fix: management report missing from the retention summary

- Location: `docs/user/fi/start/move-from-another-program.md`, "Mitä
  kerätään"; the same section in `en/`.
- Claim: Kirjanpitolaki 2:10 § lists toimintakertomus in the ten-year group
  and liiketapahtumia koskeva kirjeenvaihto in the six-year group; the page
  left out both.
- Evidence: the current consolidated text on Finlex (1336/1997, 2 luku
  10 §), read during this review: "Tilinpäätös, toimintakertomus,
  kirjanpidot, tililuettelo sekä luettelo kirjanpidoista ja aineistoista on
  säilytettävä vähintään 10 vuotta tilikauden päättymisestä …"; "…
  tilikauden tositteet, liiketapahtumia koskeva kirjeenvaihto sekä muu …
  kirjanpitoaineisto on säilytettävä vähintään kuusi vuotta sen vuoden
  lopusta, jonka aikana tilikausi on päättynyt …".
- Fix applied: both items added in both languages, toimintakertomus marked
  "where one was prepared"; the collection table row names management
  reports. Severity lowered from the finder's must-fix: a small company
  rarely has a management report, and the page already said to collect
  every year's financial statements.

## Other factual checks

- Retention periods (10 / 6 years and their starting points) checked
  against Finlex as above.
- Holvi's report set (PDF/Excel statement, camt.052, päiväkirja, pääkirja;
  plan-dependent): Holvi's support article "Holvin kirjanpitoraportit ja
  viennit", via search summary; the page links it.
- Briox archive export: the help article "Arkiston vienti/lataaminen"
  exists (search result listing); its body was behind a bot check and could
  not be read, so the page claims only that the export exists and links it,
  not its format.
- aggountant behaviour: `aggountant --help`, `import camt053 --help`,
  `capture-receipt --help` and `chart --help` on this branch's build. There
  is no SIE4 or chart import, which the page states as a limitation; the
  statement-import rule that every confirmed entry needs a booking is quoted
  from `tasks/import-bank-statement.md`.
- `scripts/check-user-guide.sh`: language pairs, links, site build and the
  practice month pass.

## Dropped candidates

None besides the above; the finder reported no other findings.

## Not covered

Wording quality of the English translation beyond fact parity; whether
the Briox menu names are current for every plan (the page says the notes
date from 2026 and should be checked against the live service).
