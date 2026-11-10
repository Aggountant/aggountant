# ADR-3 — Fiscal store: portable audit archive + analytical read surface

- **Status:** Accepted (2026-08-04)
- **Date:** 2026-08-04
- **Authors:** aggountant design-v2. Recommendation synthesized from a five-lens
  multi-model panel (architect, security, data/analytics, auditability/compliance,
  maintainability — `gemini-3.1-pro-preview`, `gpt-5.6-sol`, `deepseek-v4-pro`,
  `claude-opus-4-7`) run via `consult-llm`, applied to the S1/M1 evidence and the
  Kitsas/Tilitin benchmark. Product owner: Jari Mustonen.
- **Refines:** builds on **ADR-1** (fiscal = SQLite authoritative, git = derived
  mirror, single write-gate) and **ADR-2** (canonical wire format / hash chain).
  Resolves the two open items raised in the 2026-08-04 data-model industry-proofing
  stint — the portable audit-archive artifact (`benchmark.md §3.2`, koonti
  **K-10…K-12, K-14, K-15**) and the analytical SQL read surface (**K-13**).

---

## Context

S1/M1 proved the fiscal substrate: every voucher carries `H(prev_hash ‖
canonical_wire)` over the ADR-2 owned RFC 8785 JCS profile, gapless numbering via
`BEGIN IMMEDIATE` + CAS holds under genuine multi-connection concurrency (8×20
seals → gapless `{1..160}`, `voucher verify` re-walks the chain), and SQLite is the
single source of truth (ADR-1). The chain-vs-lock immutability question is
**settled** and this ADR does **not** reopen it.

Two gaps remain, both surfaced by the benchmark and the stint, and both belong to
the *storage architecture* rather than to any single Run:

**1. No portable, auditor-verifiable archive artifact.** The continuous chain is
*above* the Finnish market baseline — Kitsas/Tilitin ship only a
mutable-until-locked period plus a one-shot year-end SHA256 archive digest at
**year** granularity, and Kitsas's own `ARKISTOVARMENNE` certificate admits the
digest certifies immutability *only if the certificate page is preserved and signed
outside the program* (trust externalized; no voucher-to-voucher or year-to-year
chain). What the chain does **not** yet give — and what Kitsas gives cheaply — is a
**self-contained artifact an auditor can verify without aggountant installed**. The S1
git audit mirror that would underpin this is **stubbed** (`spike-notes.md` flag #2:
the `git_projection_outbox` row is written inside the seal transaction, but no
projector drains it and no `fiscal verify` lockstep SQLite↔mirror exists). This is
also a statutory anchor: **KPA 2:8** requires the books be renderable to
*selväkieliseen kirjalliseen muotoon* for the whole KPL retention period **without
dependence on a specific program**.

**2. No analytical read surface.** Fiscal is authoritative in SQLite, but the sealed
store is a write-gated transactional core, not a query surface. The product
requirement (2026-08-04): an AI agent must be able to run **ad-hoc analytical SQL**
against the books — for reports, cross-checks, exploration — **without going through
CLI verbs**, against a *clean, stable, documented, read-only* schema. The agent
generates **arbitrary** SQL; the read path must be **provably read-only** with
respect to the sealed vouchers, chain and counter — no mutation path, no way for a
hostile or hallucinated query to touch fiscal truth.

Constraints inherited and not re-opened: single-tenant, embedded/local, one small
Finnish Oy, low write volume, correctness legally critical (KPL). Consistency with
ADR-1 (SQLite fiscal, git-native non-fiscal, **single authority with derived read
projections**) and ADR-2 (wire format) is required.

---

## Decision

**Pair the proven per-voucher chain with (1) an exportable, digest-certified,
optionally-signed period/year *audit archive* built directly from the authoritative
fiscal SQLite store, and (2) a separate, derived, read-only *analytics replica*
exposing a stable documented SQL-view contract. Both are derived read projections of
the single SQLite authority — siblings of the git mirror — and neither can write the
sealed store.** This ADR decides the architecture; **it does not implement it** (see
`## Consequences` → follow-up).

The unifying principle, extending ADR-1: **the sealed fiscal SQLite has exactly one
job — the write-gated seal/chain/counter/rejection-log transaction. Every read
surface (the git mirror for browsing, the audit archive for hand-off/retention, the
analytics replica for SQL) is a derived, disposable projection of that one
authority. No read surface is ever authoritative, and no read surface can mutate the
authority.**

### Facet 1 — the audit archive (`fiscal archive`)

**Canonical format — reuse the ADR-2 wire.** The archive's **authoritative bytes are
the exact ADR-2 canonical voucher JSON**, copied byte-for-byte out of fiscal SQLite
(`canonical_bytes`) — they are already the chain preimage, already human-readable
and `git diff`-able, and require **no new serialization code** (the archive format
can never drift from the chain format). PDF/CBOR/bespoke authoritative formats are
rejected (see Rejected alternatives).

**Built from the authority, after verification.** `fiscal archive` reads a
consistent snapshot of the authoritative fiscal SQLite, **re-verifies the chain on
the fly**, and requires **`fiscal verify` (SQLite↔mirror lockstep) to pass first**.
It **never** treats the derived git mirror as an authoritative source. An archive
built while the mirror is diverged is a reconciliation landmine.

**Archive contents (a self-contained *tilikausiarkisto*).** A period/year archive is
a directory (or deterministic tar/zip) containing:

- the authoritative canonical voucher JSON for the period (chain preimages);
- the period/year **seal records** and the **rejection log (O-25)** for the period,
  as canonical hash-chained JSON (part of the *kirjausketju* / internal-control
  story);
- the **resolved masterdata embedded as content** — the chart of accounts version,
  KPA-formula policy and VAT-rate policy that the archived vouchers stamped,
  content-hashed and matching each voucher's `chart_version`/`policy_set`. Embedding
  (not referencing) is mandatory: ADR-1 keeps this masterdata in *git*, which can be
  rebased/rewritten, so long-horizon statement reproducibility must not depend on
  external git history surviving;
- for a **year-close** archive, a **required** human-readable rendition of the
  statutory books — *päiväkirja, pääkirja, tase-erittelyt, tilinpäätös,
  liitetiedot* — in a durable format (PDF/A or self-contained HTML). This rendition
  is *derived* (the JSON remains the authoritative chain preimage) but **required to
  exist**: a bag of JSON files is not a *tilikausiarkisto* a Finnish tilintarkastaja
  will accept (KPA 2:8);
- a `MANIFEST.sha256` (one `<hex>  <path>` line per archive file, `sha256sum`
  format) over a **safe bundle profile** — fixed ASCII deterministic paths only; no
  absolute paths, `..` traversal, symlinks/hardlinks, duplicate/case-colliding
  names, control characters, or unbounded expansion;
- an `ARCHIVE.json` metadata record carrying the archive format version, install
  identifier, period/year identity, the chain-head hash, and the **previous
  archive's digest** (period-to-period chaining — see below);
- the **archive digest** — `SHA256(domain_separator ‖ canonical_manifest_bytes)`,
  computed with a **non-circular** construction (the digest, signature and any
  convenience public-key file are **not** inputs to the manifest they certify);
- a `VERIFY.md` documenting the verification procedure and citing the statute
  (KPL 2:6 *kirjausketju*, KPA 2:8 *selväkielisyys*).

**Period-to-period meta-chain.** Each archive's `ARCHIVE.json` embeds the previous
period/year archive's digest, forming a **coarse, externally-verifiable meta-chain
over the tilikaudet** that wraps the fine per-voucher micro-chain. This detects
retro insertion/removal at period granularity and survives aggountant being
uninstalled — Kitsas has no equivalent. It is tamper-evident across periods **only
when at least one prior digest is independently retained** (see the deposit step).
Period archives are **lightweight** (chain-head snapshot + embedded masterdata +
digest [+ signature/timestamp]); **year-close archives are mandatory and full**
(rendered statutory books + complete voucher set + rejection log). Both chain via
`previous_archive_digest`.

**Certification — tiered, with an honest threat model.** The archive is certified in
tiers; the mandatory floor needs no persistent key.

- **Tier 0 — embedded SHA256 sigill (MANDATORY, key-free).** The archive digest +
  the internal per-voucher chain + the period-to-period meta-chain give integrity
  and internal consistency verifiable with `sha256sum -c` alone. Paired with a
  **mandatory external digest-deposit step** — the `fiscal archive` procedure emits
  an `ARKISTOVARMENNE`-equivalent certificate page and the operator deposits the
  digest **outside** the installation (print, email to the accountant, and/or commit
  to a customer-owned external repo). This is exactly the Kitsas trust model, made
  an **explicit operational step** rather than an implicit hope, and it is where
  tamper-evidence against a privileged local attacker actually lives at this tier.
- **Tier 1 — detached signature (RECOMMENDED).** A detached **Ed25519** signature
  (minisign/signify) over a domain-separated signed statement (archive format
  version, install id, period identity, chain head, previous-archive digest,
  manifest digest), produced with a **dedicated archive-signing key that is NOT the
  `age` encryption identity and NOT any artifact/PII/seal key**. It authenticates the
  archive **only to a verifier who has pinned the public key/fingerprint through an
  independent channel** — the public key bundled in the archive is *convenience
  metadata, explicitly untrusted*. Honest bound: a same-host signature does **not**
  survive compromise of the machine *and* the key (an attacker who can rewrite fiscal
  state and read the key can re-sign a forged archive). The full signing-key
  lifecycle (generation, storage, rotation, compromise response, revocation) is
  deferred to the **key-lifecycle ADR** that ADR-1 already calls for; until that
  lands, Tier 1 is optional and Tier 0 + deposit is the shipping floor.
- **Tier 2 — trusted timestamp (OPTIONAL, RECOMMENDED FROM v1).** An **RFC 3161**
  timestamp token over the archive digest from a public TSA (the token and the TSA
  CA chain archived alongside it). It needs **no persistent private key**, is a
  self-contained artifact with no ongoing dependency, and is the one mechanism that
  defends against **wholesale local history-replacement after anchoring** — precisely
  the threat for a single-person Oy where the *kirjanpitovelvollinen* signs their own
  books. This is genuine differentiation over Kitsas.

**"Auditor-verifiable outside the program" — concretely.** An auditor with **no
aggountant** can, using only shell + coreutils that have been stable for decades:
(1) `sha256sum -c MANIFEST.sha256` — every archive file matches; (2) recompute the
archive digest over the manifest and compare it to the deposited digest / signed
statement / timestamp token; (3) optionally `minisign -V` the detached signature
against an independently-pinned public key; (4) re-walk the per-voucher chain by
recomputing `SHA256(prev ‖ canonical_wire)` — a documented ~1-page algorithm in
`VERIFY.md`, executable for convenience by `aggountant verify-archive` but **not
dependent on it**; (5) read the vouchers and rendered books directly (they are
human-readable JSON + PDF/HTML). The durable floor — `sha256sum -c` over
human-readable files — needs no bundled script and satisfies KPA 2:8; richer
verification has both a documented spec and a aggountant convenience command. The
"verifiable outside the program" claim is tested by an **annual re-verification of a
multi-year-old archive on a fresh minimal machine that has never run aggountant**.

### Facet 2 — the analytics replica (`fiscal analytics`)

**A separate, derived, read-only analytics database — not the live fiscal file.**
The analytical read surface is a **physically separate SQLite database**, a
**rebuildable projection** of the sealed store (a sibling of the git mirror, another
derived read projection of the one authority). It is **disposable, never
authoritative**, and carries a `built_from_chain_head` watermark (head hash + latest
voucher number) plus a source-integrity-check timestamp and an
`analytics_schema_version`. The **sealed fiscal store is never opened by the
analytics/agent path** — the read-only guarantee is by **construction** (the agent
never holds a handle to the authoritative file), not by configuration.

**Full rebuild on demand.** At this write volume the replica is rebuilt by a
**deterministic full snapshot** (read fiscal SQLite → build the denormalized read DB
→ atomically publish the immutable snapshot), **not** by an incremental
outbox/CDC cursor. This removes the entire class of staleness-drift and
cursor-recovery bugs; incremental projection may be added later if volume ever
warrants it, reusing the existing projection outbox. A rebuild that cannot **fully**
denormalize a voucher (e.g. an unresolvable `chart_version`, corrupt canonical wire)
is a **hard failure** — the prior valid snapshot stays in place; **a partial
snapshot is never published**.

**A denormalized, agent-legible star schema behind stable views.** Because the
replica is physically separate, its schema is decoupled from the fiscal internal
layout and is designed for high-accuracy agent SQL:

- one **fact row per posting**, with voucher/date/account/VAT/period columns
  inlined — the hardest analytical query becomes a flat `GROUP BY` / `SUM`, no CTEs;
- **amounts stored as exact integers** (minor units, with explicit scale where the
  domain allows sub-cent amounts) with a clear **sign convention** and a companion
  `is_debit` boolean — **never** float and **never** decimal strings as the arithmetic
  representation (ADR-2's decimals-as-strings are for the wire/chain; scale
  differences that ADR-2 preserves are numerically identical here and irrelevant to
  aggregation);
- **chart fields denormalized at build time** — account name, **account-kind
  classification** (asset/liability/equity/revenue/expense + refinement, so the agent
  computes "total revenue" without reverse-engineering account-number ranges),
  VAT code + rate + resolved net/VAT amounts — resolved from the *pinned*
  `chart_version` so the agent **never** reaches into git masterdata or the fiscal DB;
- a **positive-classification whitelist** on exposed fields (fail-closed, consistent
  with ADR-1) so PII / sensitive free-text is not silently surfaced into the
  analytics surface;
- a `_meta`/provenance view returned alongside results carrying the watermark, so any
  report generated from the replica is pinnable to a specific fiscal state.

**The stable contract is a set of documented, versioned SQL VIEWS** (`v_posting`,
`v_voucher`, `v_account_balance`, `v_period`, `v_vat_summary`, …) discoverable by
standard SQLite introspection; the physical tables underneath may evolve between
`analytics_schema_version`s while the views hold.

**Enforcement stack (defense in depth, "provably read-only").** Separation alone is
not a boundary; the read-only guarantee is enforced by construction plus:

- the agent opens **only** the immutable published snapshot via
  `SQLITE_OPEN_READONLY` and is **never given the fiscal DB path or a filename/URI** —
  a query broker selects the replica by internal handle;
- `sqlite3_set_authorizer` **denies** `ATTACH`/`DETACH`, every write/DDL/writable
  pragma, extension loading (`load_extension` disabled), output-database operations,
  and any non-allowlisted UDF/virtual table; `query_only` is set as resettable
  defense-in-depth, explicitly **not** the boundary;
- the snapshot is **published immutably** — a privileged builder stages, verifies the
  chain-head + schema, checkpoints away the WAL, closes, then atomically publishes
  into a directory the analytics principal cannot modify;
- **recommended deployment** gives the analytics/agent path an OS capability with no
  write/traverse access to the fiscal directory (a distinct principal or sandbox);
- hostile-query bounds (SQL size/statement/depth limits, a progress-handler VM-step
  and wall-clock budget, memory and temp-storage caps) protect availability.

Honest bound, recorded: on a **minimal single-user install without OS uid
separation**, the guarantee rests on the agent never receiving the fiscal path +
the authorizer + `SQLITE_OPEN_READONLY` (defense in depth) rather than on a hard OS
boundary; the physical replica is what makes the stronger OS-capability separation
**achievable** when wanted, and it still dominates same-file views on availability
(no WAL-checkpoint starvation from long agent queries), schema quality, and keeping
the fiscal DB lean.

---

## Consequences

### Positive

- **One canonical representation from edge to archive.** The archive reuses the ADR-2
  wire bytes verbatim — zero new serialization, no format drift over a decade, and
  the auditor's ground truth is the same human-readable JSON the chain hashes.
- **A portable artifact auditors already recognize, strictly stronger than Kitsas.**
  Digest-certified, `sha256sum`-verifiable *tilikausiarkisto* with rendered statutory
  books (KPA 2:8 satisfied), plus a per-voucher chain that answers "was voucher 47
  altered after seal?" precisely (Kitsas answers only per year), plus a
  decade-scale period-to-period meta-chain Kitsas lacks, plus optional trusted
  timestamping that closes the very gap Kitsas's own certificate admits.
- **The read-only analytical surface is read-only by construction.** A separate
  disposable replica means "could the agent modify the books via a query?" is
  answerable *"no, by construction of separate files"* rather than *"no, provided no
  future pragma/schema change re-opens the surface."* Long agent queries cannot
  starve the fiscal WAL; the fiscal DB stays lean (an ADR-1 value).
- **Agent answer quality is designed in.** A denormalized star schema with exact
  integer amounts, a sign convention, and embedded account-kind/VAT fields minimizes
  the hallucination/misinterpretation surface on a legally-critical dataset.
- **Architecturally coherent.** The archive and the replica are siblings of the git
  mirror — all derived read projections of the single SQLite authority — so ADR-1's
  single-authority invariant is *reinforced*, not stretched.

### Negative / costs (accepted)

- **Two more derived projections to build and keep consistent.** The archive command,
  the (still-stubbed) git mirror + `fiscal verify`, and the analytics rebuild each
  read the authority and must agree; a reconciliation check must prove archive bytes
  match the mirror bytes for a period. Mitigated by full-rebuild-on-demand (no CDC)
  and by building all three from the same authoritative snapshot.
- **A required year-close PDF rendering path.** Mandating rendered statutory books
  adds a rendering dependency (e.g. `typst` or a minimal HTML→PDF path — engine
  choice is an implementation detail). Accepted because a JSON-only archive is not a
  *tilikausiarkisto*.
- **A signing-key lifecycle enters the operator's world.** Tier 1 adds a *dedicated*
  Ed25519 identity distinct from the ADR-1 `age`/artifact/PII keys — a third
  cryptographic lifecycle. Capped by making Tier 1 *recommended, not mandatory*,
  keeping the mandatory floor (Tier 0 digest + deposit) and the highest-value hedge
  (Tier 2 RFC 3161 timestamp) **key-free**, and deferring the full lifecycle to the
  key-lifecycle ADR.
- **The analytics enforcement stack is real code.** Authorizer allowlist, immutable
  snapshot publication, resource bounds and (recommended) OS capability separation
  must be built and regression-tested across SQLite upgrades and platforms — cost
  SQLite does not give for free.
- **Archive confidentiality is a separate concern.** An archive may contain personal
  data (descriptions, counterparties, attachments, bank details); signing is not
  encryption. Secure delivery/handling is a policy matter and must not let transport
  encryption replace the plaintext manifest/signature semantics an auditor needs
  after authorized decryption.

### Rejected alternatives

**Facet 1 — PDF (or bespoke/CBOR) as the archive's *authoritative* format —
REJECTED.** PDF is not the chain preimage, not byte-reproducible, and not the ADR-2
canonical form; making it authoritative would create a second fiscal representation
that can drift from the chain. PDF/HTML is kept as a *required derived rendition*
(KPA 2:8) but never as the authoritative bytes. Bespoke/CBOR formats are rejected for
the same reason ADR-2 rejected them: they forfeit `git diff`-ability and re-introduce
serialization surface on the legally-critical path.

**Facet 1 — embedded digest only, no signature and no timestamp (pure Kitsas parity)
— REJECTED as the ceiling (kept as the mandatory floor).** The digest alone
externalizes trust exactly as Kitsas's certificate admits. It is the right
*mandatory floor* (key-free, `sha256sum`-verifiable, KPA-compliant), but the ADR adds
the recommended dedicated-key signature and the recommended RFC 3161 timestamp
because the single-Oy "signed-my-own-books" threat is real and the timestamp closes
it at near-zero cost.

**Facet 1 — reuse the ADR-1 `age` encryption identity as the signing key —
REJECTED.** Crypto hygiene: a signing key must be distinct from an encryption
identity, and from artifact/PII/seal keys, to bound blast radius. The maintainability
pull to avoid a third key ecosystem is answered instead by keeping the mandatory and
highest-value tiers key-free, not by overloading the `age` identity.

**Facet 1 — mandatory external anchoring / notary (RFC 3161 required) — REJECTED as
mandatory (adopted as recommended).** Requiring an online TSA at every archive
conflicts with offline/embedded operation. It is promoted to *optional-but-recommended
from v1* (not "deferred"), and the ADR states plainly that it is the mechanism needed
to resist wholesale local replacement by an attacker who controls both the data and
the signing environment.

**Facet 2 — stable views + `SQLITE_OPEN_READONLY` on the *live fiscal file* (no
replica) — REJECTED.** The maintainability lens's strongest option: `SQLITE_OPEN_READONLY`
is a kernel `O_RDONLY` fd, "free," no projector. Rejected because (1) it is *not*
"provably read-only under arbitrary agent SQL" in a same-uid embedded context — the
agent can open a *second* writable handle to the same path, so the guarantee needs
capability restriction the replica makes achievable and same-file access defeats;
(2) a long or pathological agent query on the live DB holds a read snapshot that
**starves WAL checkpointing**, growing the WAL unbounded and threatening the fiscal
*write* path's availability; (3) a genuinely useful analytical surface needs a
denormalized, exact-integer, chart-enriched schema you would never put in the
transactional store — materializing it there bloats fiscal backup/VACUUM and violates
ADR-1's keep-lean value. Full-rebuild-on-demand defuses the projector-rot concern the
dissent raised. Recorded as the panel's one formal dissent; resolved in favor of the
replica (defer-to-security on the safety conflict) with the honest degraded-mode
caveat above.

**Facet 2 — `PRAGMA query_only` as the read-only mechanism — REJECTED.** Resettable
by any `PRAGMA query_only=OFF` the agent may emit; kept only as defense-in-depth,
never as the boundary.

**Facet 2 — raw physical-table access (no view contract) — REJECTED.** Couples agent
SQL to the internal schema, which evolves for Ledger-internal reasons; the stable
versioned views are the contract.

**Facet 2 — a second analytical engine (DuckDB/DataFusion) — REJECTED.** Unjustified
at a handful of vouchers a day; a second embedded engine adds dependency and
contract-test surface for a scaling problem that does not exist — the same reasoning
ADR-1 used to reject RocksDB. SQLite is mandated for the replica too, preserving the
small footprint.

### Biggest risk

**A published-but-diverged read projection mistaken for truth, and the
external-deposit / key-pinning discipline not actually performed.** The technical
protocol can be flawless and the audit story still fail if: an archive is produced
while the git mirror is diverged (mitigated: `fiscal verify` must pass first, and an
archive↔mirror byte-reconciliation check); the analytics replica silently goes stale
and an agent reports on incomplete data (mitigated: full rebuild per session +
mandatory watermark in every result); the mandatory digest-deposit is skipped, so
Tier 0's tamper-evidence against a local attacker never materializes (mitigated: the
`fiscal archive` procedure should fail-closed / require confirmation of the deposit,
not merely print an instruction); or the signing public key is never independently
pinned, so the bundled key authenticates a forged archive (mitigated: `VERIFY.md`
labels the bundled key untrusted and documents out-of-band pinning). The archive and
replica are both **read-only against the authority** — they never weaken ADR-1's
single-write-gate invariant — but their *value* depends on operational discipline
that the tooling must enforce, not assume.

---

## Review-process lenses

A five-lens multi-model panel decided this ADR. Below is one faithful summary per
lens — **summarising the reasoning behind the decision as it stands, not
re-deciding**. Four lenses converged; the maintainability lens dissented on the
Facet-2 replica and is recorded with its resolution.

- **Architect.** Confirmed both surfaces as derived read projections of the single
  SQLite authority (the archive and replica are siblings of the git mirror; build
  the archive from SQLite, never the mirror). Broke the Facet-2 tie decisively on
  **WAL-checkpoint starvation**: a long or pathological agent query on the live
  fiscal DB blocks WAL truncation and threatens the write path, so a physically
  separate replica is an availability necessity, not just hygiene. Endorsed the
  period-to-period digest as an elegant macro-chain wrapping the per-voucher
  micro-chain, and required independent projection cursors for the two consumers.

- **Security.** Load-bearing here. Established the honest threat model: a same-host
  signing key does **not** protect against a privileged local attacker (rewrite chain
  → re-sign), and a bundled public key is convenience, not authenticity — verification
  requires an independently pinned key/fingerprint. Mandated a **dedicated** Ed25519
  key (never the `age` encryption identity), a non-circular domain-separated digest,
  and a safe bundle profile (fixed ASCII paths, no traversal/symlinks). On Facet 2:
  separation *alone* proves nothing; "provably read-only under arbitrary SQL" needs
  `sqlite3_set_authorizer` (deny `ATTACH`/writes/`load_extension`/dangerous pragmas/
  non-allowlisted UDFs), the agent never holding the fiscal path, immutable snapshot
  publication, resource bounds, and — for a hard boundary — OS capability separation.
  The replica is what makes that capability restriction *achievable*; hence it beats
  same-file views once the enforcement stack is in place.

- **Data / analytics.** A good read surface is load-bearing for agent answer quality:
  the replica + a **denormalized star schema** (one fact row per posting) beats views
  over raw fiscal tables (repeated joins, decimal-string parsing, reaching into git
  masterdata). Non-negotiables folded in: **exact integer amounts** with a sign
  convention (never float/decimal-string for arithmetic), chart fields + account-kind
  + resolved VAT **embedded at build time** from the pinned `chart_version`, a
  **hard-fail-never-partial** rebuild, a measurable `built_from_chain_head` watermark,
  and documented versioned views as the contract. Endorsed full-rebuild-on-demand at
  this volume over incremental CDC.

- **Auditability / compliance.** Anchored the archive in statute: **KPA 2:8**
  (*selväkielinen*, no dependence on a specific program) is why `sha256sum -c` over
  human-readable files is the durable floor, and why a JSON-only archive is *not* a
  *tilikausiarkisto* — rendered statutory books (*päiväkirja/pääkirja/tase-erittelyt/
  tilinpäätös/liitetiedot*) are **required** year-close content. Required embedding
  the resolved masterdata as content (git can be rebased), including the rejection
  log (O-25), and mandating the period-to-period meta-chain as the decade-scale
  externally-verifiable artifact. Promoted **RFC 3161 timestamping** from "deferred"
  to optional-recommended-from-v1 (cheap insurance for the "signed-my-own-books" Oy
  threat), and made the Kitsas trust model an **explicit external digest-deposit
  step**. Flagged the signing-**key lifecycle**, not the algorithm, as the real
  long-horizon risk, and the retention mapping (6-year vouchers/books, 10-year
  tasekirja).

- **Maintainability (dissent, resolved).** Argued the separate replica is
  over-engineering at a handful of vouchers a day, that `SQLITE_OPEN_READONLY` is a
  free kernel `O_RDONLY` guarantee, and against adding a third key ecosystem or a
  10-year-fragile verify script. **Resolution:** the dissent's O_RDONLY point is true
  for one fd but does not deliver "provably read-only under arbitrary agent SQL" in a
  same-uid context (the agent can reopen the path writable) — defer-to-security on the
  safety conflict; and three independent wins (WAL isolation, schema quality,
  keep-lean) go unrebutted. Its legitimate concerns **were adopted**: full-rebuild-on-
  demand (no rotting CDC projector), the mandatory certification floor kept **key-free**
  (Tier 0 digest + deposit; Tier 2 timestamp needs no persistent key), reuse of the
  ADR-2 wire encoder for the archive (no new serialization), and `sha256sum -c` + a
  built-in `aggountant verify-archive` as the verification path rather than a fragile
  bundled shell script. The residual dissent is preserved as the honest degraded-mode
  caveat for a minimal single-user install.

---

## Related

- **ADR-1 (store)** — fiscal = SQLite authoritative, git = derived append-only
  mirror, single write-gate = Ledger, git-native reference masterdata, keep-fiscal-DB-
  lean. This ADR adds two more derived read projections (archive, analytics replica)
  under the same single-authority rule, and depends on ADR-1's stubbed git mirror +
  `fiscal verify` being finished. The signing-**key-lifecycle ADR** ADR-1 calls for is
  the home of Tier-1 key mechanics.
- **ADR-2 (wire format)** — the archive's authoritative bytes *are* the ADR-2
  canonical wire; the per-voucher chain rewalk in `VERIFY.md` recomputes
  `SHA256(prev ‖ canonical_wire)`.
- `benchmark.md §3.2/§3.4`, `benchmark-koonti.md` **K-10…K-15** (archive artifact,
  analytical SQL surface, pre-seal change log, immutability spectrum);
  `issues/benchmark-kitupiikki/analysis.md` (Kitsas `Arkistoija`/`ARKISTOVARMENNE`).
- `s1-sealed-voucher/spike-notes.md` flags #1 (crash-injection/restore matrix) and #2
  (git mirror projector + `fiscal verify` stubbed) — both are preconditions this ADR's
  implementation depends on.
- `decisions.md` **D-06** (this decision) and open items **O-27…O-29**.
- **Deliberately not decided here** (implementation / other ADRs): the PDF-rendering
  engine; whether `fiscal archive` becomes a Run (EffectState/provenance) vs a
  synchronous CLI verb (process/S2 design); the exact analytics amount encoding for
  sub-cent scale; the full signing-key lifecycle (key-lifecycle ADR); the pre-seal
  accepted-change log (K-14, deferred).

---

*Dated 2026-08-04: initial decision. This ADR decides architecture only — building
the `fiscal archive` command, finishing the git audit mirror + `fiscal verify`, and
building the `fiscal analytics` replica are recorded as a follow-up implementation
spinoff; no `crates/` code changes accompany this ADR.*
