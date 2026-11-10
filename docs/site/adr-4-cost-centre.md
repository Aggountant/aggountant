# ADR-4 — Cost-centre / kohdennus dimensional tagging on the posting

- **Status:** Accepted (2026-08-04)
- **Date:** 2026-08-04
- **Authors:** aggountant design-v2. Recommendation synthesized from a five-lens
  expert panel (architect, maintainability, data-model, domain/accounting,
  security/seal-invariant), applied to the S1 sealed-voucher evidence, the
  ADR-1/ADR-2/ADR-3 substrate, and the Kitsas/Tilitin benchmark. Product owner:
  Jari Mustonen.
- **Refines:** builds on **ADR-1** (fiscal = SQLite authoritative + git-native
  reference masterdata, single write-gate), **ADR-2** (canonical wire format /
  hash chain — a dimensional tag is *sealed content*), and **ADR-3** (analytical
  read surface — per-dimension P&L is a first-class consumer). Resolves the one
  genuinely missing primitive the benchmark surfaced
  (`issues/benchmark-kitupiikki/analysis.md §3.3`, `benchmark.md` row *Cost
  centres / kohdennus*, koonti follow-up `cost-centre-kohdennus`): decide the
  `Posting` dimensional-tag shape **before the type ossifies** (F1/F7).

---

## Context

The S1 sealed-voucher slice has landed. `Posting` and `VoucherBody`
(`crates/model/src/voucher.rs`) are real, hash-chained, sealed record types:
each `VoucherBody` carries `wire_version = 1`, `schema_version = 1`, its bytes
are the ADR-2 canonical RFC 8785 JCS profile, and the chain hash
`SHA256(prev_hash ‖ canonical_wire)` (ADR-1) is verified against the **stored
original bytes** with a strict decode→validate→re-encode→byte-equality read path
(ADR-2 §Decision.4). Both structs are `#[serde(deny_unknown_fields)]`. A
`Posting` today is `{account, amount, side, vat_code?}`.

The Kitsas/Tilitin benchmark flagged **cost-centre / kohdennus dimensional
tagging** as *"the one genuinely missing primitive"*
(`analysis.md §3.3`): Kitsas models a `Kohdennus` (a self-referential
cost-centre/project tree, `tyyppi CHECK IN (0,1,2,3)`) and every `Vienti`
(posting) can target one via a single `kohdennus` foreign-key column. Per-project
/ per-cost-centre P&L is a pervasive small-Oy need. It is **absent from aggountant's
F1–F16 node inventory** and the benchmark warned it is **costly to retrofit into
`Posting`/`Voucher` after they ossify**.

The decision must be taken now, while the `Posting` shape is still cheap to
extend, and it is constrained on three sides:

- **The tag is sealed content.** A dimensional tag on a posting is part of the
  accounting fact (which project this expense belongs to) and must be
  tamper-evident exactly like `account` and `amount`. Whatever shape is chosen
  must have a **deterministic canonical serialization** under the ADR-2 encoder
  and must not break the S1 seal/chain invariants or the existing wire-v1 golden
  fixtures.
- **The sealed schema is immutable and versioned; masterdata is not.** ADR-1
  keeps reference masterdata (chart of accounts, `chart_version`) as git-native,
  content-hashed, human-diffable files that evolve without touching the sealed
  chain. A dimensional taxonomy is the same *kind* of data.
- **PII fails closed (ADR-1).** A cost-centre or project *name* is free text and
  can carry a person's name; it is **not** automatically safe to place in
  immutable sealed bytes.

The decisive framing is the cost of **not** reserving the shape now versus later.
The fiscal chain is append-only and immutable: a field can never be added to
already-sealed vouchers. If the field is deferred, the day dimensions are needed
the ledger acquires a **permanent schema discontinuity** — a `schema_version`
bump splitting the record set into pre-dimension vouchers that structurally
cannot carry a tag and post-dimension vouchers that can — which every downstream
consumer (analytics replica, reports, verification) must special-case forever. If
the field is reserved now as *optional and absent-by-default*, every voucher from
genesis uniformly supports a dimension, untagged postings serialize
byte-for-byte as they do today, and the discontinuity never happens.

---

## Decision

**The core `Posting` carries an optional dimensional tag from day one: a small,
bounded key/value map of validated opaque codes — `dimensions` — where the key
names the dimension (e.g. `kustannuspaikka`, `projekti`) and the value is an
opaque reference into a git-native dimension registry. The map is sealed content;
the dimension universe it references (valid keys, valid values, hierarchy,
display labels, rollups) lives in reference masterdata and evolves without ever
touching the sealed schema.** The tag lives on the **posting**, not the voucher;
the ADR reserves the shape and mandates the field, and defers the registry,
validation and reporting machinery to F7.

### Where it lives — the posting, and only the posting

The tag is added to `Posting` (alongside `account`, `amount`, `side`,
`vat_code`), not to `VoucherBody`. Per-project P&L requires **posting-level**
granularity: a single voucher routinely splits a cost across cost centres (one
supplier invoice covering two projects), so a voucher-level tag is too coarse and
would be ambiguous when postings disagree. Kitsas confirms the granularity
(`Vienti.kohdennus`). A voucher-level *default* — "tag every posting in this
voucher with X unless overridden" — is an **edge/CLI ergonomics** convenience
that expands to per-posting tags **before** sealing; it is never itself sealed
content.

### The shape — a bounded key/value map of opaque codes

`Posting` gains one field:

```rust
#[serde(default, skip_serializing_if = "Dimensions::is_empty")]
dimensions: Dimensions,     // newtype over BTreeMap<DimensionKey, DimensionValue>
```

- **`DimensionKey` and `DimensionValue` are validating newtypes** in the Model
  layer, in the same family as `AccountId` and `VatCode`: a strict, documented
  grammar of opaque codes (a bounded character set — ASCII alphanumerics plus a
  small separator set — a length cap, and no leading/trailing/whitespace
  ambiguity). They are **codes, never free text**: a cost-centre's human name /
  label lives in the registry (masterdata), never in the sealed posting (ADR-1
  fail-closed classification). `serde` deserialize runs the constructor
  validation, so an out-of-grammar or PII-shaped value is unrepresentable on the
  read path.
- **`Dimensions` is a `BTreeMap<DimensionKey, DimensionValue>`** — ADR-2
  §Decision.2 explicitly permits a string-keyed `BTreeMap` in the wire format
  (unlike an arbitrary/generic map): the encoder sorts keys lexicographically by
  UTF-16 code unit (RFC 8785), the strict decoder rejects duplicate keys, and the
  Model layer bounds the entry count. There is therefore a single canonical
  serialization and no insertion-order or duplicate-key ambiguity.
- **The key/value shape subsumes the simpler shapes at no extra canonical-form
  cost.** A business tagging only a cost centre writes `{"kustannuspaikka":
  "MYYNTI"}`; one needing orthogonal dimensions on the same posting writes
  `{"kustannuspaikka": "MYYNTI", "projekti": "P-2026-01"}`. Crucially, **adding a
  new dimension type never bumps the wire schema** — the set of valid keys is a
  masterdata concern, not a sealed-schema concern. This is the one shape that is
  future-proof against the retrofit cost the whole decision exists to avoid.

### Backward-compatibility and the canonical form — the crux

The field is **optional and absent-by-default**, and an empty map serializes to
*nothing* (`skip_serializing_if`), so:

- **No `wire_version` bump; no golden breakage.** An untagged posting produces
  byte-identical canonical wire to today's, because the `dimensions` key is
  omitted entirely. Existing wire-v1 fixtures re-encode identically; the ADR-2
  `stored_bytes == reencoded_bytes` check holds for every historical voucher
  (absent → `default` empty → skipped → identical). Adding the optional field is
  a `schema_version = 1` addition, not a canonicalization change, so it does not
  trip ADR-2 §Decision.5 (which reserves a new `wire_version` for changes to
  *canonicalization or the profile*).
- **Exactly one canonical form for "untagged."** An explicit empty
  `"dimensions": {}` is **forbidden** on the strict read path — an untagged
  posting is represented only by the absence of the key, never by a present-empty
  map, so there is no `{}`-vs-absent ambiguity in the sealed bytes.
- **Reserve the shape now; defer the machinery.** Reserving the field is cheap
  (a validated newtype + an optional posting field). The dimension registry, the
  record-time validation that a posting's dimension refs exist in the pinned
  registry, and the reporting/rollup path are **F7 work** and are not built by
  this ADR. Until the registry lands, the CLI simply does not populate the field,
  so genesis vouchers stay byte-identical to current output while the schema is
  already dimension-ready.

### The dimension registry — git-native reference masterdata

The **meaning** of a dimension code — the set of valid keys, the valid values per
key, whether values form a hierarchy (a cost centre with sub-centres), display
labels, active/archived state, and any "this account requires a kohdennus" rule —
lives in **git-native reference masterdata** (ADR-1), content-hashed and
human-diffable exactly like the chart of accounts. This is the clean separation
the decision turns on:

> **Sealed = the fact** (this posting is tagged with code X). **Masterdata = what
> X means and how X rolls up.** The immutable chain records a stable opaque
> reference; the evolvable git registry gives it meaning.

Kitsas's self-referential `Kohdennus` **tree** is therefore modelled in the
*registry*, not in the sealed posting: hierarchy and rollup are a
masterdata/reporting concern, and putting tree structure into immutable fiscal
bytes would couple the sealed record to an evolving org chart. At record time the
Ledger validates a posting's dimension refs against the **pinned** registry
version (the same resolve-then-pin discipline ADR-1 uses for `chart_version`, no
atomic cross-store transaction), and the resolved registry version is stamped
into provenance so historical per-project reports reproduce (see O-31).

### The analytical read surface (ADR-3)

The ADR-3 analytics replica is the primary consumer. Its denormalized star schema
(one fact row per posting) exposes the posting's dimensions so per-dimension P&L
is a flat `GROUP BY` — either as explicit columns for the common keys or via a
posting→dimension bridge, resolved and labelled at build time from the pinned
registry (so the agent never reaches into git masterdata or the sealed store).
The exact exposure is an ADR-3 implementation detail (O-32); this ADR only fixes
that the dimensions are sealed on the posting and are therefore available to
every derived read projection.

---

## Consequences

### Positive

- **The retrofit cost the benchmark warned about is eliminated, cheaply.** The
  sealed `Posting` shape is dimension-ready from genesis with no `wire_version`
  bump and no golden breakage, so the ledger never acquires a pre/post-dimension
  schema discontinuity.
- **One shape, no future schema bumps for new dimension types.** A new dimension
  (osasto, kohde, …) is a masterdata registry addition, not a sealed-schema
  change — the key/value map absorbs it. Orthogonal dimensions (cost-centre *and*
  project on the same posting) work without any special case.
- **Coherent with the substrate.** Sealed opaque references + evolvable git-native
  registry mirrors ADR-1's `chart_version` pattern exactly; the string-keyed
  `BTreeMap` is precisely the wire shape ADR-2 already permits; per-dimension P&L
  is a natural ADR-3 read projection. No new architectural primitive.
- **PII stays out of the immutable chain.** Opaque codes in the sealed posting,
  human labels in erasable/editable masterdata — the ADR-1 fail-closed rule is
  honoured on the one surface where a mistake is unerasable.
- **Accounting-correct granularity.** Posting-level tagging supports the real
  small-Oy workflow (split one invoice across projects) that a voucher-level tag
  could not.

### Negative / costs (accepted)

- **`Posting` gains a field and the Model layer gains two newtypes.** A one-time,
  small `crates/model` change (recorded as a follow-up spinoff — this ADR ships no
  `crates/` code). The hand-written `Deserialize` shadow structs already in
  `voucher.rs` must route the new field through the validating constructor, and a
  strict read-path rule must reject a present-empty `dimensions: {}`.
- **A dimension registry is new masterdata to design and govern (F7).** Its model
  (flat vs. tree, valid-values, per-account requirement rules, versioning/pinning)
  is real work — deferred, not free (O-30). Until it lands the field is reserved
  but unpopulated.
- **Provenance surface grows.** For reproducible per-dimension reports the
  resolved registry version should be pinned into provenance alongside
  `chart_version` (O-31, resolve with O-19/O-26).
- **Analytics must denormalize dimensions.** The ADR-3 replica gains dimension
  columns / a bridge and rollup logic (O-32) — a real but bounded read-surface
  cost, not a new engine.
- **A bounded-but-flexible map needs guardrails.** The Model layer must cap the
  entry count and key/value length and enforce the code grammar, so a hostile or
  buggy input cannot smuggle unbounded or PII-shaped content into sealed bytes.

### Rejected alternatives

**Defer entirely — reserve nothing now — REJECTED.** This is the benchmark's
explicit warning realized: adding `dimensions` after real sealed vouchers exist
forces a `schema_version` bump and a **permanent** heterogeneous ledger (some
postings structurally cannot carry a tag), which analytics, reporting and
verification must special-case for the whole KPL retention horizon. Reserving an
optional, absent-by-default field now is nearly free (byte-identical untagged
output, no golden breakage) and strictly dominates deferral. The *machinery* is
deferred; the *shape* is not.

**A single free dimension (one `cost_centre` slot) — REJECTED.** Matches Kitsas's
single `kohdennus` column and is the 80% case, but it cannot carry orthogonal
dimensions (cost-centre **and** project) on one posting without a later schema
bump — reintroducing exactly the retrofit cost. The key/value map subsumes the
single-dimension case at no additional canonical-form cost, so the narrower shape
buys nothing.

**Fixed typed slots (`kustannuspaikka` + `projekti` fields) — REJECTED.**
Hardcodes the dimension taxonomy into the immutable sealed schema; adding a third
dimension type later is a `schema_version` bump. The whole point is to keep the
dimension *set* in evolvable masterdata, which the open map does and fixed slots
do not.

**A self-referential `Kohdennus` tree in the sealed posting (Kitsas literal) —
REJECTED.** Hierarchy and rollup are a masterdata/reporting concern; embedding
tree structure (parent refs, `tyyppi`) into immutable fiscal bytes couples the
sealed record to an evolving org chart and re-encodes reporting logic into the
chain. The posting carries a flat opaque reference; the tree lives in the
git-native registry, where it can be reorganised without rewriting history.

**A free-text dimension label in the posting — REJECTED.** A cost-centre or
project name is free text that can carry a person's name; ADR-1's fail-closed
classification forbids unclassified free text in surfaces that must stay
PII-clean, and sealed bytes are the one place a PII leak is effectively
unerasable. The sealed posting carries an opaque code; the label lives in
masterdata.

**Voucher-level tagging instead of posting-level — REJECTED.** Too coarse for
per-project P&L (a voucher can split a cost across cost centres) and ambiguous
when postings would disagree. Kitsas tags the posting (`Vienti.kohdennus`). A
voucher-level *default* survives only as pre-seal edge convenience that expands to
per-posting tags.

**Keep dimensions in an unsealed side-store — REJECTED.** The dimensional tag is
part of the accounting fact and must be tamper-evident like `account`/`amount`;
an unsealed side-store lets "which project did this expense belong to" be altered
after seal with no chain evidence. Sealing the tag is the point, and the
optional-empty-skipped map makes sealing it free for untagged postings.

### Biggest risk

**A misclassified dimension value putting PII into the immutable chain, or the
registry-vs-seal boundary being blurred.** The technical shape is sound, but its
safety rests on discipline the tooling must enforce, not assume: the code grammar
and entry/length bounds must fail closed (a canary test that no PII-shaped or
free-text value reaches sealed bytes), the CLI must not accept a human label
where a code is expected, and the registry (mutable, git) must never be mistaken
for authoritative fiscal state (the sealed reference is authoritative; the
registry only gives it meaning). A secondary risk is the empty-map canonical rule
being missed in the read path, admitting a `{}`-vs-absent ambiguity — closed by
the strict-decoder rule and a golden fixture for both forms.

---

## Review-process lenses

Five lenses decided this ADR. Below is one faithful summary per lens —
**summarising the reasoning behind the decision as it stands, not re-deciding**.
All five converged on *reserve now, posting-level, key/value map of opaque codes,
registry in masterdata*; there was no lens tie.

- **Architect.** Anchored the decision on decoupling the **sealed posting field**
  (which ossifies and must be pinned now) from the **dimension masterdata model**
  (which can evolve freely). The posting carries a stable opaque reference; the
  registry gives it meaning — the same single-authority-plus-derived pattern ADR-1
  uses for `chart_version` and git-native reference data. No new architectural
  primitive is introduced; the tag rides the substrate ADR-1/2/3 already fix.

- **Maintainability.** Weighed reserve-now against defer and found reserve-now
  strictly cheaper: an optional, absent-by-default field costs a validated newtype
  and one struct field, while deferral costs a permanent schema discontinuity that
  every consumer special-cases forever. Insisted the key/value map is the one
  shape that never needs a further `schema_version` bump for a new dimension type,
  and that the *machinery* (registry, validation, rollups) be deferred to F7 so
  the ADR reserves only the shape. Required the read-path guardrails (bounded
  entries/length, reject present-empty map) to keep the flexible map from becoming
  a maintenance hazard.

- **Data-model (load-bearing).** Confirmed the retrofit-cost and canonical-form
  mechanics that decide the ADR: the field is posting-granular; a string-keyed
  `BTreeMap` is exactly the wire shape ADR-2 §Decision.2 permits (sorted keys, no
  duplicate/insertion-order ambiguity); optional-and-skipped-when-empty yields
  **byte-identical** wire for untagged postings, so there is **no `wire_version`
  bump and no golden breakage** and the ADR-2 byte-equality read check holds for
  every historical voucher. Denormalises cleanly into the ADR-3 star schema for a
  flat per-dimension `GROUP BY`.

- **Domain / accounting.** Asked whether a small Oy needs Kitsas's tree or just
  one free dimension, and answered: the real need is per-project / per-cost-centre
  P&L at posting granularity (split one invoice across projects), not deep
  hierarchy in the core. One free dimension is the 80% case, but the key/value map
  covers orthogonal cost-centre-and-project tagging at no extra cost, and
  hierarchy/rollup belongs in the registry + reporting, not the sealed fact.
  Endorsed posting-level over voucher-level, matching `Vienti.kohdennus`.

- **Security / seal-invariant.** Established that the tag is **sealed content** —
  immutable, chain-protected, tamper-evident like `account`/`amount` — and must
  therefore carry only opaque codes, never free-text labels (ADR-1 fail-closed
  PII rule; sealed bytes are unerasable). Required a strict code grammar and
  bounded entry count/length enforced at construction, a strict-read-path rejection
  of a present-empty `dimensions: {}` so there is exactly one canonical form for
  "untagged," and a canary test that no PII-shaped value reaches the wire. Raised
  no dissent: sealing the tag is precisely what makes per-project attribution
  audit-grade.

---

## Related

- **ADR-1 (store)** — git-native, content-hashed reference masterdata + the
  resolve-then-pin `chart_version` discipline is the model for the dimension
  registry; ADR-1's fail-closed PII classification is why the sealed posting
  carries opaque codes, not labels.
- **ADR-2 (wire format)** — the tag is sealed content; the string-keyed `BTreeMap`
  is the permitted wire shape, and the optional-empty-skipped field preserves the
  canonical form (no `wire_version` bump).
- **ADR-3 (fiscal store)** — the analytics replica denormalises the dimensions for
  per-dimension P&L; this ADR makes the tag available to that read projection.
- `issues/benchmark-kitupiikki/analysis.md §3.3`, `benchmark.md` (*Cost centres /
  kohdennus* row), koonti follow-up `cost-centre-kohdennus` — the source of the
  requirement; Kitsas `Kohdennus` (`kitsas/sqlite/luo.sql`) as the reference data
  shape (facts, not code).
- `crates/model/src/voucher.rs` — the real `Posting`/`VoucherBody` this decision
  extends; `crates/model/src/newtype.rs` — the newtype family `DimensionKey`/
  `DimensionValue` join.
- `decisions.md` **D-07** (this decision) and open items **O-30…O-32**.
- **Follow-up code work (spinoff, not this commit):** add `dimensions` to
  `Posting` + the `DimensionKey`/`DimensionValue` newtypes in `crates/model`; the
  registry masterdata + record-time validation + provenance pinning + analytics
  denormalisation are F7 implementation work.

---

*Dated 2026-08-04: initial decision. This ADR decides the sealed `Posting`
dimensional-tag shape only — adding the field to `crates/model`, building the
dimension registry, its validation and provenance pinning, and the ADR-3 analytics
exposure are recorded as follow-up implementation work; no `crates/` code changes
accompany this ADR.*
