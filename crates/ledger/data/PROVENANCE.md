# Provenance & licensing — `fi-general-2026.json`

> Due-diligence audit of the bundled 98-account Finnish general chart of accounts
> (`crates/ledger/data/fi-general-2026.json`), the default reference masterdata
> shipped by the chart-init slice (commit `e24fa15`). Issue:
> `chart-provenance-rapko`. Audited 2026-08-04.

## Conclusion — CLEAN (author-derived; no re-derivation required)

The bundled chart is an **author-derived, minimal conventional Finnish general
chart**. No GPL/copyleft compilation (Kitsas `tilit.json`, Tilitin `.jar`
templates, or any Odoo module) was lifted verbatim. **No re-derivation was
needed.** The [Raportointikoodisto (RAPKO)](#2-rapko--the-defensible-public-basis)
standard stands as the cite-able *public basis* going forward.

## 1. Origin — author-derived, not copied

Evidence that the 98 accounts were hand-authored, not imported from a specific
source:

1. **Single authored commit.** The file entered the repo fully formed in one
   commit (`e24fa15`, "feat(masterdata): aggountant init + git-native reference
   masterdata") by the repo author. There is **no import tooling, no source-file
   attribution, and no vendored `LICENSE`** anywhere near it. The commit message
   describes it as an authored deliverable ("Real 98-account Finnish general
   chart, a single maintainable data file").
2. **A deliberately minimal representative subset.** A full RAPKO code list or a
   full Kitsas/Tilitin default chart runs to *hundreds* of rows with a heading
   hierarchy. This file is a curated ~98-account teaching/default subset — the
   shape of hand curation, not a bulk copy.
3. **Pragmatic non-standard entries.** `"1920": "Pankkitili 2"` is a convenience
   duplicate that appears in **no** published standard verbatim — a tell of hand
   authoring.
4. **Numbers follow a public *convention*, not one file.** The numbering is the
   widely-shared Finnish SME convention (Liikekirjuri-style: `1910 Pankkitili`,
   `3000 Myynti`, `4000 Ostot`, `1763 Arvonlisäverosaaminen`,
   `2939 Suoritettava arvonlisävero`). These account numbers are **public
   facts/conventions shared across many charts**; convergence on them is not
   evidence of copying any one GPL file. Kitsas's own `tilit.json` default uses a
   *different, richer* scheme (heading nodes `Otsikko`, letter-coded account
   types) — **none of which is present here.**
5. **Only the three factual fields.** Each account carries `id`, `name`, `kind`
   and nothing else. The **distinguishing curated content** of the GPL
   compilations (VAT-code columns, depreciation policy, KPA statement-formula
   mappings, Swedish names) is **absent**. What remains is the factual layer that
   copyright does not reach.

This matches the legal position already recorded in
`issues/benchmark-kitupiikki/analysis.md` §3.1: *"A chart of accounts is mostly
facts (account numbers, Finnish/Swedish names, KPA mapping), not a copyrightable
program — but a specific curated compilation (Kitsas's `tilit.json`, Tilitin's
templates) is a GPL work. Do not lift those verbatim."* We did not.

## 2. RAPKO — the defensible public basis

**Raportointikoodisto (RAPKO)** — the *talousraportoinnin koodisto* / standardized
financial-reporting code list — is the cite-able public standard for a Finnish
default chart + statutory-reporting mapping. It is a **standard (facts published
for interoperability), not a GPL compilation.**

- **Business reporting code** (yrityksen raportointikoodisto): developed 2009 in
  the Real-Time Economy (Reaaliaikatalous) programme; historically curated by
  Tili-instituuttisäätiö / **Suomen Taloushallintoliitto** with the authorities;
  it is the basis for the Finnish **SBR/XBRL** financial-statement taxonomy.
- **Publication of record.** The standardized financial-reporting code lists are
  published on the national **Reference Data registry**
  (`koodistot.suomi.fi`, registry `sbr-fi-code-lists` — *"Standardisoidun
  talousraportoinnin koodit ja luokitukset"*) and on the open-data portal
  **`avoindata.suomi.fi`**, maintained by **Valtiokonttori** (State Treasury)
  under the reporting board (*raportointilautakunta*). Editions are dated and
  versioned (fiscal-year packages; the FY2027 package was updated 2026-06-02).

### Terms of use

The code lists are distributed through Finland's **official open-data channels**
(`avoindata.suomi.fi` + `koodistot.suomi.fi`). Public-administration open data on
these channels carries, by policy, a **Creative Commons Attribution 4.0
(CC BY 4.0)** or **CC0 1.0** licence — i.e. free to implement and derive from with
attribution. (The interoperability-platform *tooling* is EUPL; that is the
software, not the data.)

> **Open item (see issue `discussion_items`):** we did **not** machine-confirm an
> explicit CC BY 4.0 licence *stamp* on the specific SME RAPKO dataset — only that
> it is published on channels where CC BY 4.0 is the standing policy. This does
> **not** affect the clean conclusion above, because aggountant ships **author-derived
> convention facts, not RAPKO-derived bytes.** *If* aggountant ever ingests RAPKO
> bytes directly (e.g. to generate the KPA statement-formula mapping), confirm the
> stamped licence on that dataset first and record attribution.

### Sources

- Valtiokonttori — talousraportoinnin koodistot ja taksonomiat:
  <https://www.valtiokonttori.fi/palvelut/julkishallinnon-palvelut/kuntatalouden-palvelut/talousraportoinnin-koodistot-ja-taksonomiat/>
- Suomi.fi Reference Data registry (RAPKO / SBR code lists):
  <https://koodistot.suomi.fi/registry;registryCode=sbr-fi-code-lists>
- Valtiokonttori — "Talousraportoinnin koodiston käyttö helpottaa pk-yritysten
  viranomaisraportointia":
  <https://www.valtiokonttori.fi/uutinen/talousraportoinnin-koodiston-kaytto-helpottaa-pk-yritysten-viranomaisraportointia/>
- Suomen Taloushallintoliitto — "Rakenteista tilinpäätöstä kehittämässä":
  <https://taloushallintoliitto.fi/rakenteista-tilinpaatosta-kehittamassa/>
- Suomi.fi-avoindata — avoimen datan lisenssit (CC BY 4.0 / CC0 policy):
  <https://avoindata.suomi.fi/fi/tietoa-avoimesta-datasta/lisenssit>
- Tilisanomat — "Raportointikoodisto on talousammattilaisen työkalu":
  <https://tilisanomat.fi/kirjanpito-ja-tilinpaatos/raportointikoodisto-on-talousammattilaisen-tyokalu>

## 3. PII classification — confirmed, still correct

The fail-closed PII boundary (ADR-1 §Masterdata) still holds and is still correct:

- **Positive whitelist, enforced.** Reference masterdata is a *positive* whitelist
  of known-safe fields — top-level `{chart, chart_date, accounts}`, account fields
  `{id, name, kind}` — enforced by `reference_schema_is_a_positive_non_pii_whitelist`
  (fails closed on any unclassified field) and `#[serde(deny_unknown_fields)]` on
  the read path. The new **`chart_date`** field was classified here as a non-PII
  edition marker (an ISO date) and added to the whitelist explicitly.
- **Write-time + CI canary.** `first_pii_token` refuses HETU/IBAN-shaped tokens to
  plaintext at write time; `no_pii_token_in_the_git_bytes` scans the actual git
  bytes. The bundled chart is clean (all 98 names are generic accounting terms).
- **Account names are NOT automatically non-PII — and this is handled correctly.**
  An account *name* can carry a person's name (a sole trader's named equity/loan
  account, e.g. "Oma pääoma, <person>"). The current canary detects HETU/IBAN
  *shapes* only; it does **not** detect a free-text personal name. This is the
  **known, documented deferred seam** (`git_masterdata.rs` module docs): a
  user-added custom account with a personal name "would need an explicit
  classification gate before it is safe as plaintext — that gate is future work."
  For the **bundled standard chart** the classification is correct (no personal
  data), and the schema-whitelist test fails closed on growth. **No regression;
  the named-custom-account case remains a deferred gate, as designed.**

## 4. Human version metadata

`chart_date` (ISO `YYYY-MM-DD`, the edition's effective date — `2026-01-01` for
the FY2026 edition) was added alongside the content-hashed `chart_version` so a
person can tell which edition a home carries **without** hashing bytes. It
participates in the content hash, so an *accidental* edit of either the human
`chart_date` or the machine `chart_version` without the other trips the `open()`
drift check (an internal-consistency guard). It is **validated at parse time** as
a real ISO calendar date (`validate_chart_date`) — since it feeds the permanent
content address, a free-text value is refused, so the marker cannot become a typo
or a personal-data channel. The content-hash mechanism is otherwise unchanged.

## 5. Review outcome (`/llm-review`, 4 models)

A 4-model review (Gemini 3.1-pro, GPT-5.6, Opus-4-7, DeepSeek-v4) ran on the diff;
full report + triage in `history/review-chart-provenance-rapko.md`. Two load-bearing
findings, both dispositioned:

- **Unvalidated hashed `chart_date` → FIXED** (`validate_chart_date` at the parse
  funnel; rejection + drift tests added).
- **Strict-schema hard break for pre-`chart_date` homes → ACCEPTED, no migration.**
  Adding a required field under `deny_unknown_fields` (and hashing it) means an
  old on-disk `chart-of-accounts.json` fails `open()`. **Deliberate:** this is
  pre-1.0 with *no deployed homes* — the prior `chart_version` itself only landed
  in `e24fa15` and has never shipped, so there is nothing to migrate. A V1/V2
  hash-versioning + migration path would be permanent complexity on a
  legally-critical hash for zero real homes; it is explicitly declined here. **If**
  homes are ever deployed before a further chart-schema change, that change must
  carry a real migration (schema-version field or dual-decode) — the seam is noted
  for whoever ships first.
