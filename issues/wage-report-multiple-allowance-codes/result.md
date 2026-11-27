# Result: wage-report-multiple-allowance-codes

A 331 daily-allowance line of `file-wage` carries every per diem type its
amount contains, and the record sends one 331 transaction with an
`AllowanceCode` for each:

```json
{"income_type": "331", "amount": "79.00", "allowance_codes": [2, 3]}
```

```xml
<DailyAllowance><AllowanceCode>2</AllowanceCode><AllowanceCode>3</AllowanceCode></DailyAllowance>
```

## Primary source

- Verohallinto, *Tietojen ilmoittaminen tulorekisteriin: luontoisedut ja
  kustannusten korvaukset*, VH/5655/05.00.00/2026, antopäivä 30.10.2026,
  luku 2.1.3 Päiväraha: tulolajilla 331 ilmoitetaan verovapaiden
  päivärahojen ja ateriakorvausten yhteismäärä, ja Päivärahan tyyppi
  -tiedolle "arvoja voi valita useita samalle summalle". Esimerkki 37
  (luku 2.1.4) shows one 331 row with types Ateriakorvaus and Kokopäiväraha.
  A halved per diem keeps its own type.
- Vendored schema `crates/broker/schemas/tulorekisteri/2025/WageReportsToIRTypes.xsd`:
  `DailyAllowance` is a sequence of `AllowanceCode`, `minOccurs="1"
  maxOccurs="unbounded"`.

## What changed

- `crates/domain/src/wage_report.rs`: `TravelLine::DailyAllowance` holds
  `allowance_codes: AllowanceCodes`, kept in ascending order so one set has
  one serialization and one record. The JSON shape lives in a private
  `WireLine`: a 331 line takes `allowance_codes` or the earlier
  `allowance_code` (exactly one; `null` is refused), and a one-type line is
  written back as `allowance_code`. `validate` refuses an empty set, a
  duplicate and a code outside 1–4.
- `crates/broker/src/tulorekisteri_xml.rs`: one `AllowanceCode` element per
  type.
- `file-wage` INPUT help, the guide page "Ilmoita matkakorvaukset
  Tulorekisteriin" / "Report travel allowances to Tulorekisteri" (fi/en),
  `skills/tulorekisteri/SKILL.md` and the CHANGELOG.
- The calculator's `incomes_register_lines` (`claim compute`) are accepted by
  `file-wage` as they are; a test in `crates/domain/tests/travel_fi.rs` reads
  them as `TravelLine`s and writes them back unchanged.

## Compatibility

- No store migration; no schema version changes.
- Single-code reports are byte-identical: the serialized report (so the
  stored `proposal_hash`, the outbox `sent_hash` and `correction_hash` hold),
  and the sent XML (so a `record_hash` from an earlier preview still
  approves). Pinned by a line-level test, by the process golden hash computed
  with the `main` build, and checked across builds: a home and a run created
  by the `main` binary gave identical preview output under this build, its
  `file-wage start` was accepted unchanged and its `wage.json` was untouched.
- Downgrade: a build before this one refuses a report holding a multi-type
  line. One-type reports remain readable by it.

## Acceptance criteria

- [x] XML for a 331 with codes 2 and 3 validates against the vendored XSD
  (new report and replacement; `crates/broker/tests/payer_summary_schema.rs`).
- [x] Old `wage.json` states and old stdin input deserialize (unit tests,
  golden hash, cross-build check).
- [x] Guide (fi/en) and `skills/tulorekisteri/SKILL.md` updated.

## Review

`/llm-review` at critical rigor (finder `gpt-6-astra`, verifier
`claude-fable-5-2`), no findings; `/assess-findings` took the one test
suggestion. See `review.md` and `assessment.md`.
