# ADR-5 — Seal timing: a first-class pre-seal draft window and the correction surface

- **Status:** Accepted (2026-08-04)
- **Date:** 2026-08-04
- **Authors:** aggountant design-v2. Recommendation synthesized from a five-lens
  expert panel — product/usability (agent-ergonomics) and workflow/domain-accounting
  weighted heaviest, plus architect, security/seal-invariant, maintainability —
  applied to the seal-timing survey, the S1 sealed-voucher evidence, the
  ADR-1/ADR-2/ADR-3 substrate, and the Kitsas/Tilitin benchmark. Product owner:
  Jari Mustonen.
- **Refines:** builds on **ADR-1** (fiscal = SQLite authoritative + git-native
  reference masterdata, single Ledger write-port, fail-closed PII), **ADR-2**
  (canonical wire / hash chain — a sealed voucher is immutable bytes), and **ADR-3**
  (derived read surfaces — reporting nets corrections). Resolves the one editing
  ergonomics question the seal-timing survey deferred
  (`seal-timing-research.md §4.1`, koonti **K-14**): under settled seal-on-record
  (**D-05/D-06**, chain-vs-lock **not** reopened), how the pre-seal editable phase
  works and how corrections are surfaced, so seal-on-record does not cost usability.
  Resolves open item **O-29**.

---

## Context

Seal-on-record is settled (D-05/D-06): every voucher is hash-chain-sealed the
moment it is recorded, a sealed voucher is immutable, and corrections are new
sealed vouchers (*oikaisutosite*), never edits. The survey established that this is
**lawful** — both seal-on-record and the market's edit-until-lock satisfy the same
two binding KPL requirements (the *kirjausketju* holds at all times, KPL 2:6; every
correction's link to the original is verifiable, KPL 2:5), and immutability
(KPL 2:7) bites at *tilinpäätöksen laatiminen*. The purely-legal question is
answered. **This ADR turns on usability and realistic agent-assisted workflow, not
statute.**

The survey located the entire felt cost of seal-on-record in one place: the
**pre-seal draft window** (O-29). It did not decide it. The open question:

- **Option A — thin draft.** Seal ≈ immediately; the editable phase exists only for
  the seconds/minutes of assembly; every post-seal fix is a correction voucher.
- **Option B — first-class pre-seal draft window.** A durable, listable
  "received / in-progress" stage a voucher can sit in for hours/days, freely
  revised before it is sealed, plus an O-29 accepted-change log for pre-seal audit
  parity with Kitsas `Tositeloki`.
- A principled hybrid.

(Option C — adopt the market's edit-until-lock model — is out of scope; it reverses
D-06 and is only the survey's comparator.)

Three ground facts constrain the decision:

- **The draft object already exists, but has no home.** S1 ships
  `voucher draft → validate → seal` (`spike-notes.md`): `voucher draft` returns a
  canonical `VoucherDraft` value, `validate` runs a Ledger dry-run, `seal` consumes
  it. The draft is *conceptually* first-class in the model
  (`crates/model::VoucherDraft`) but *physically* transient — it lives on stdout, in
  a tempfile, or in the agent's context. The interim `capture-receipt` skill shows
  the friction: it must attach the artifact and assemble the voucher *after* the
  confirmation, so the confirmation binds to extracted details rather than stored
  bytes, and a cancelled run can orphan an artifact.
- **The sealed invariants are structurally isolated from anything a draft can
  touch.** Gaplessness comes from allocating the voucher `number` *inside* the
  `BEGIN IMMEDIATE` seal transaction; chain integrity from the CAS on the chain head
  inside that same transaction (S1 proved both under 8×20 concurrency). A draft has
  **no number, no hash, no chain position** until seal. What the draft looked like
  before is irrelevant to the seal's atomicity.
- **A tamper-evident append-only log already exists.** The O-25 rejection log is a
  hash-chained, append-only log the ledger already keeps and `voucher verify`
  already walks (its own chain, separate from the voucher chain — `spike-notes.md`).

The real small-Oy workflow (survey §3) puts editing pressure in two places: the
high-frequency, low-stakes mid-period fix, and the low-frequency, deliberate
pre-close reclass. The decisive test is whether **the agent can absorb the
correction mechanics so the human issues one intention ("fix that")**, and where
that abstraction honestly leaks. Walking the occasions shows only *one* of them —
capture-before-clarity — is helped by a draft window at all; every post-seal
correction is identical under A and B and is carried entirely by agent absorption +
transparent reporting.

---

## Decision

**Adopt Option B, refined into a bounded hybrid.** aggountant gains a first-class
pre-seal draft window, realized as a durable **intake / work-item stage** that is
explicitly *not* a booking, with a **thin commit boundary** (human approval seals
immediately — there is never an "approved-but-unsealed" queue) and a **per-skill
seal cadence** (a clear receipt still seals in one intention; ambiguous captures and
`hunt-receipts` batches persist as intake). The seal remains the single, atomic,
irreversible commit; the S1 seal transaction is unchanged. Corrections after seal
are always new, linked, sealed vouchers surfaced through a first-class
`voucher correct` verb whose transparency line is **"absorb the mechanics, surface
the fact."** A **must-seal policy** enforced by the Ledger keeps drafts from
becoming an off-chain place to park real liabilities. The **O-29 accepted-change
log is adopted**, lightweight but tamper-evident, reusing the O-25 substrate.

This decides the workflow/architecture; it ships **no `crates/` or `skills/`
code**. Build work is a follow-up spinoff (see Related).

### 1. Thin vs. first-class → a first-class window, as *intake*, not a mutable voucher

The durable pre-seal object is an **intake / work-item**, not a mutable
`VoucherBody`. It holds captured artifact reference(s), the agent's proposed
postings/classification, unresolved questions, the target accounting period, and
provenance. It is explicitly **unbooked** and carries **no voucher number, no hash,
no chain position, and no effect on any balance, VAT figure, report, filing, or
period-completeness computation**. At approval the agent compiles the *current*
intake candidate into the existing transient `VoucherDraft`, runs `validate`, and
`seal`s — the same seal path S1 already has.

Calling it *intake* rather than "voucher draft" is deliberate and load-bearing. A
durable object named "draft voucher" drifts toward "almost booked": users start
expecting it in "expected VAT," reports grow an *include-drafts* boolean, the
close logic reasons over it — that is how a second mutable ledger begins, and it is
precisely the Kitsas mutable-`KIRJANPIDOSSA` window that seal-on-record exists to
avoid. Intake is *saapunut / käsittelyssä oleva aineisto* (received / in-clarification
material) — a legitimate accounting stage that is not yet a *kirjaus*. Acceptable
status words: `received`, `needs clarification`, `ready for approval`. Forbidden:
`booked draft`, `preliminary voucher`, or any label implying a balance already
includes it.

The **thin commit boundary** is the other half. Intake covers occasions where facts
are genuinely unresolved (category/VAT unclear, evidence missing, personal/business
allocation open). It must **not** hold a determinate, approved booking waiting to be
sealed — approval *is* the commit point, and sealing follows immediately. Per-skill
cadence follows from this: `capture-receipt` on a clear, already-paid receipt seals
in one intention (the thin path); on an ambiguous one it persists intake and pauses;
`hunt-receipts` (A5) always produces intake because batch review needs the pause.
The **mechanism** is unified (one intake stage behind one write-port); the **feel**
is calibrated per skill. This also gives the already-decided A5 `hunt-receipts` Run
a clean, typed home for its per-transaction match candidates instead of an
`[T→integration]` scratch namespace — it *removes* an under-defined namespace rather
than adding one.

Why not thin-only (A): it leaves the agent's capture-before-clarity flow with no
home, forcing either a premature seal-with-guess (which then needs a correction
voucher — permanent noise in the chain) or off-system limbo (worse than a visible,
listed intake item). Why not an unbounded durable voucher (naïve B): it re-creates
the mutable ledger the whole model rejects. The bounded intake hybrid takes B's
ergonomics and A's commit discipline.

### 2. The edit/correction surface and its transparency line

**Pre-seal (intake): a draft-revise loop, transparent by nature.** The user (via the
agent) revises an intake item freely; it is visibly "not in the books." The verbs
the agent compiles to, and a user may invoke directly:

```
intake list | show <id> | revise <id> …| discard <id> --reason … | prepare-voucher <id>
```

`prepare-voucher` emits the transient `VoucherDraft`; then the existing
`voucher validate` / `voucher seal`. The agent may hide the intermediate JSON; it
must **never** imply the item is booked, and every ledger/report view must state
whether unresolved intake exists for the period.

**Post-seal: a first-class `voucher correct` verb.** A sealed voucher is immutable;
there is **no** `voucher edit`. The correction primitive:

```
voucher correct <voucher-number> --mode reclass|reverse|adjust --reason …
```

`reclass` (wrong account/VAT), `reverse` (duplicate, personal, invalid), `adjust`
(amount/partial). It builds a transient correction `VoucherDraft`, structurally
linked to the original (`corrects: <number>` + reason in provenance), and seals it
through the *same* seal path. The agent invokes it from natural language.

**The transparency line — "absorb the mechanics, surface the fact."** The agent may
absorb the debit/credit construction (the user says "that lunch was personal, fix
it," not "reverse 7790 and debit shareholder loan"). The agent must **never** hide
that a correction voucher exists. Concretely, before sealing a correction the agent
shows a preview of the shape:

> This will **not** edit voucher #123. It creates correction voucher #171 linked to
> #123. Net effect: 7690 −124.00 EUR, 7790 +124.00 EUR. Seal the correction?

and afterward names the result ("Correction voucher #171 sealed; corrects #123").
This is the honest line between the two failure modes the product owner named: a
correction that is **too invisible** ("don't worry, I fixed the receipt") fractures
the user's mental model the moment they open the *päiväkirja* and see two vouchers,
and quietly weakens the audit-trust story to below Kitsas; one that is **too
ceremonious** (forcing the user to hand-construct reversal postings for every typo)
loses to Kitsas on daily friction. Seamless *action*, transparent *record*. The word
**"edit" is forbidden** in the user surface for anything sealed; "correct" / "issue
a correction voucher" / "reverse" are the vocabulary.

**Where the abstraction honestly leaks (and must):**

- **On the statutory journal.** The user *will* see two vouchers in the *päiväkirja*
  / *pääkirja* / audit archive. That is correct and required (KPL 2:5/2:6). Reporting
  nets it in the *effective* view (§3); the *audit* view retains both.
- **After a VAT filing.** "Fix that" then yields **two** consequences — a new
  accounting correction voucher **and** a possible amended VAT return / later-period
  OmaVero adjustment. The agent must surface both and must **not** imply that
  correcting the ledger corrected the filed return. Seal-on-record makes the lawful
  path (an adjusting entry) the *only* path here, which is a fit, not a cost.

### 3. The must-seal policy (drafts cannot park liabilities off-chain)

Must-seal is a **Ledger-enforced precondition**, not a skill convention — modelled
as unresolved-intake *obligations/blockers* in the same shape as D1's "avoimet
obligationit blokkaavat sulun." An intake item must be **sealed or explicitly
dispositioned** before it can influence — or be omitted from — any figure that
leaves the system:

- **Period close.** `close-period` blocks while period-relevant intake, unmatched
  known bank events, or known unrecorded liabilities remain unresolved; the refusal
  lists the blockers with stable IDs.
- **VAT filing.** `file-vat` already requires `PeriodState = Sealed` (scenario B2);
  VAT is computed from **sealed vouchers only**. No intake plausibly belonging to the
  VAT period may remain unresolved at filing.
- **Year close.** `close-year` blocks on unresolved year-relevant intake;
  pre-close reclassifications are sealed before `ApproveStatements`.
- **Any filed/published figure.** If an amount contributes to an externally
  provided return or statement, its accounting treatment is already sealed.
- **Statutory recording deadline (KPL 2:4)** and **known cash/bank event at
  cutoff.** A missing category or evidence must **not** license omitting a known
  economic event from closed books. The event is sealed with a justified provisional
  treatment (a suspense/clearing booking or non-deductible-VAT treatment) and
  corrected later — *"VAT unclear" is never a reason to omit a known bank
  transaction.* (The S1 five-account chart cannot yet express suspense/clearing; this
  is flagged as O-34.)

Explicit disposition (`discard --reason …`, defer-to-next-period-with-basis, "not
accounting material", "no economic event") is **auditable and reasoned**; a bare
"ignore" is not sufficient. Stale intake escalates: items past a review horizon
require re-review before seal, and are surfaced, never silently dropped. Drafts are
always listed, so nothing is parked hidden.

### 4. The O-29 accepted-change log — adopted, tamper-evident, lightweight

Because intake is durable, its accepted revisions need an audit trail for parity
with Kitsas `Tositeloki` and — more sharply for an AI-first ledger — so the
**autonomous agent cannot silently rewrite pre-seal history** either. **O-29 is
adopted**, with this integrity and scope:

- **Its own append-only hash chain**, `SHA256(prev ‖ canonical(event))`, written
  **inside the single Ledger write-port** and **walked by `voucher verify`** —
  exactly the O-25 rejection-log mechanism, which already works this way. It is a
  **separate** chain from the voucher chain (a draft is a pre-booking; it is not part
  of the *kirjausketju* until sealed), so draft writes do not serialize on the seal
  chain head, yet an attacker or hallucinated agent cannot alter draft history
  without breaking a verifiable hash chain. This is preventive integrity, matching
  the guarantee D-06 chose over Kitsas's externalised trust.
- **Reuses a generic namespaced substrate.** O-25 is refactored to a generic
  append-only audit log keyed by `namespace` (`rejection` | `intake_change` | …), so
  O-29 reuses the storage/writer/verify machinery without duplicating it and without
  overloading the rejection log's semantics.
- **Logs accepted revisions, not every agent thought.** Recorded: intake
  create/revise/status-transition/disposition/seal, actor (human | agent | import),
  timestamp, `run_id` if any, change reason/source, before/after **content hash or
  compact diff**, artifact/bank-transaction refs, the human-approval binding, and the
  final voucher number/hash or disposal reason. **Not** recorded: unsaved
  alternatives, private model deliberation, or verbatim raw OCR/free-text (kept in
  the encrypted artifact/intake store, referenced by hash — the ADR-1 fail-closed PII
  posture applies, and the log must not become a permanent PII trap; see O-35).
- **Bound to the sealed result.** The final accepted-revision hash and the
  approval-payload hash are stamped into the sealed voucher's provenance (a draft
  ancestry pointer: `draft_id` + `draft_change_log_head`), so an auditor can prove
  which approved proposal became which voucher.

**Human approval binds to content.** The intake candidate presented for approval is
content-addressed; approval binds to that hash via the already-decided **Preview
gate `bind-payload-hash`** (O-23). Any later revision requires a fresh approval — the
agent cannot silently change what the human approved.

### 5. Reporting nets corrections gracefully (ADR-3)

A two-voucher correction renders, by default, as a clean **"corrected to X"** in the
effective/business view, with drill-down to both sealed facts; the statutory
*päiväkirja* / *pääkirja* / audit archive **retain every voucher and posting**
unchanged. Correction linkage is **structured data captured at seal time**
(`corrects` provenance), never a report-time heuristic over free-text descriptions,
and the original is **never** rewritten or suppressed in any statutory export. This
is an ADR-3 read-surface obligation; the ADR fixes only that the linkage is sealed
structured data so the netting is exact.

---

## Consequences

### Positive

- **Seal-on-record stops costing usability.** The agent's realistic
  capture-before-clarity flow gets a first-class, visible home; the ambiguous-receipt
  and `hunt-receipts` cases no longer force a premature seal-with-guess or off-system
  limbo.
- **Zero blast radius on the sealed invariants.** Gaplessness and chain integrity
  come from the seal transaction, which is untouched. A durable intake stage changes
  what the *input* to `seal()` looks like; it changes nothing about `seal()`. Thin
  drafts are not architecturally safer — both share the exact same seal transaction.
- **The single write-gate is strengthened, not weakened.** Pre-seal state that would
  otherwise accumulate in skill memory, tempfiles, or an `[T→integration]` namespace
  now lives in one named, typed, hash-chain-verifiable place behind the Ledger port.
- **Corrections are agent-easy and audit-honest at once.** One human intention →
  agent-built correction voucher → named result. The lawful adjusting-entry path is
  the only path, so the higher-stakes moments (post-filing, pre-close) have no
  foot-gun. The low-stakes cases teach the two-voucher model *before* the user meets
  the high-stakes case where it is legally mandatory.
- **Cheap integrity via reuse.** O-29 rides the existing O-25 hash-chained append-only
  log (namespaced); the `Cipher` seam already covers masterdata-style encryption for
  intake; provenance already carries `chart_version`/`policy_set` and gains
  `draft_id`/`draft_change_log_head`.

### Negative / costs (accepted)

- **A durable intake subsystem is real, ongoing surface.** Intake schema + lifecycle
  states, `intake` CRUD, must-seal precondition queries at close/filing, the O-29
  namespaced log, reporting netting, and a battery of contract tests (intake is not
  numbered / not chained / not archived-as-record / not counted / cannot authorise a
  filing; sealing from intake allocates a number only inside the atomic seal
  transaction; an abandoned item leaves no partial voucher). Deferred to the spinoff,
  not free.
- **Must-seal is cross-cutting.** `close-period`, `close-year`, `file-vat` each gain a
  "no unresolved period-relevant intake" precondition. Bounded (it mirrors the
  existing obligation-blocker pattern) but touches every closing Run.
- **Intake is a PII surface.** Raw OCR/vendor/free-text lives there; it must be
  encrypted like masterdata, and the O-29 log must record structural diffs/hashes,
  not verbatim free text. A retention/erasure policy for abandoned intake is required
  (O-35) — a hash-chained log is in tension with unrestricted draft erasure.
- **The provisional-treatment path needs chart support S1 lacks.** Sealing a known
  event at cutoff with a suspense/clearing booking needs accounts the S1 five-account
  stub does not have (O-34).
- **Reports gain a completeness dimension.** Every period view must state whether
  unresolved intake exists — a small permanent addition to the read surface.

### Rejected alternatives

**Option A — thin draft only — REJECTED.** It leaves capture-before-clarity homeless:
the agent must seal a guess (permanent correction-voucher noise in the chain) or push
the item off-system. It does not even reduce the *most common* correction event
(§3 step 2 mid-period fixes are post-seal and identical under A and B). Its sound
core — that a determinate, approved booking should seal immediately — is **kept** as
the thin commit boundary (§1).

**Option C — adopt edit-until-lock — OUT OF SCOPE / REJECTED.** Reverses D-05/D-06,
re-litigates a settled question, trades preventive+continuous tamper-evidence for
detective+point-in-time with externalised trust, and is the worst fit for an
autonomous agent writer (silent edits become possible). The survey showed the felt
"misfit" maps onto A↔B (the draft window), not B↔C (chain vs. lock).

**Unbounded durable "voucher draft" as a mutable booking — REJECTED.** A durable
object with posting-ready accounting semantics that can sit for days becomes a second
mutable ledger: reports grow an *include-drafts* axis, close reasons over it,
analytics wants to expose it. That is the Kitsas mutable window by the back door. The
intake framing + thin commit boundary + must-seal policy is exactly what prevents it.

**A silent "edit" that hides the correction voucher — REJECTED.** Too invisible: the
user does not realise a second sealed voucher exists, the mental model fractures at
the journal, and the audit-trust story falls below Kitsas. Violates the transparency
requirement.

**A mandatory manual correction ceremony — REJECTED.** Forcing the user to
hand-construct reversal postings for every typo is the friction that loses to Kitsas
daily. The agent must absorb the mechanics; only the *fact* must surface.

**No O-29 log — REJECTED.** A durable, agent-mutable pre-seal window with no
accepted-change trail has no audit parity with `Tositeloki` and, worse, lets the
autonomous agent silently rewrite pre-seal history. The window's cost of admission is
its log.

**Full-chain-weight draft logging (draft numbering / interleave draft events into the
fiscal voucher chain / archive-grade canonical draft bytes / immutable snapshot per
keystroke) — REJECTED as over-engineered.** A draft is a pre-booking, not a fiscal
event; giving it the sealed chain's full weight multiplies cost and confuses the
mental model, and interleaving draft writes into the seal chain head serializes agent
interactions against sealing. A separate append-only hash chain (O-25 shape) gives
sufficient tamper-evidence.

**A purely in-memory, conversation-scoped draft (security's fallback hybrid) —
REJECTED.** It captures single-session editing but evaporates across sessions, which
defeats the multi-day capture-before-clarity case and the `hunt-receipts` Run's
durable cursors — the two flows that motivate the window at all. Durability is the
requirement; its safety is met by the integrity set in §4 instead.

### Biggest risk

**Intake drifting into a shadow ledger, or the O-29 log becoming a PII trap.** The
technical shape is sound, but its safety rests on discipline the tooling must
enforce, not assume: intake must be invisible to *every* derived read surface
(analytics replica, archive, statutory reports) except an explicit "inbox" view —
enforced by contract tests, since the analytics source of truth is the sealed store;
the must-seal preconditions must be Ledger-enforced so a skill cannot bypass them by
"just sealing quickly first"; and the O-29 log must record structural diffs/hashes
with raw PII kept in the encrypted store, so a permanent append-only log does not
accumulate free-text personal data. A secondary risk is a "fast seal from intake"
path that skips the standard seal transaction — forbidden; `prepare-voucher` only
emits a `VoucherDraft`, and the one seal path allocates the number.

---

## Review-process lenses

Five lenses decided this ADR — product/usability and workflow/domain weighted
heaviest per the product owner. Below is one faithful summary per lens
(**summarising the reasoning behind the decision as it stands, not re-deciding**).
Four converged on a durable pre-seal window bounded so it is not a second ledger;
the security lens dissented toward thin-only and was resolved (not overruled) by
meeting its full integrity set.

- **Product / usability (agent-ergonomics, load-bearing).** Held that thin-only is a
  straitjacket for asynchronous, agent-driven capture — a receipt arrives today, the
  bank line tomorrow, the VAT code next week — and that the user's mental model is an
  "inbox" of work-in-progress. Favoured the durable window and named the transparency
  crux as *assisted ceremony*: the agent absorbs the KPL correction mechanics but
  narrates the correction-voucher reality, because a too-seamless "I fixed it"
  fractures trust the moment the journal shows two vouchers. Its non-negotiables —
  durable inbox, state-aware edit surface, hard must-seal at close/filing, adopt O-29
  — are adopted verbatim.

- **Workflow / domain-accounting (load-bearing).** Reframed the durable object as
  *saapunut / käsittelyssä oleva aineisto* — legitimate received material, not a
  booking — and insisted on a **thin commit boundary** (no "approved-but-unsealed"
  queue) so it never becomes editable-booked `KIRJANPIDOSSA`. Set the honest line:
  *"hide the debit/credit construction, but never hide the existence of the
  correction voucher,"* and required that a post-filing fix surface both its
  consequences (accounting correction + amended return). Warned that the real hazard
  is omitting *known* events for want of classification, mandating a provisional
  seal-then-correct at cutoff. Proposed scenario additions (A9/A10/B4/D7, extend D1),
  carried as wrap-up recommendations. This lens's intake framing is the spine of §1.

- **Architect.** Established the load-bearing structural fact: gaplessness and chain
  integrity come from the seal transaction, so a durable draft table has **zero**
  blast radius on the sealed invariants as long as the seal path is the only one that
  turns a draft into a numbered, chained voucher. Showed the draft object already
  exists as a value with no home, that O-29 is the O-25 mechanism pointed at a new
  event stream, and that the window *strengthens* the single write-gate by giving
  ad-hoc pre-seal state a typed home. Endorsed "unified mechanism, per-skill seal
  cadence." Required draft state be PII-encrypted and drafts never reach a derived
  read surface.

- **Security / seal-invariant (dissent, resolved).** Preferred thin-only (or an
  in-memory conversation-scoped draft) to keep tamper-evidence preventive and
  unblurred, warning that a durable mutable store lets an autonomous agent mutate data
  before any chain trace exists. It conceded the call to the product owner *provided*
  a strict integrity set: the draft store lives behind the single write-port (no
  scratchpad DB); every accepted mutation is a hash-chained O-29 event walked by
  `verify`; must-seal invariants are hard and Ledger-enforced with stale-draft
  escalation; the presented candidate is content-addressed so a post-approval change
  forces re-confirmation; and "edit" is banned for sealed vouchers. **Resolution:**
  the durable window is adopted because agent-ergonomics is weighted heaviest and the
  already-decided A5 `hunt-receipts` Run makes durable pre-seal state indispensable —
  but *every* security non-negotiable is honoured (§4, and the Preview-gate
  `bind-payload-hash` binding from O-23 is exactly the content-address-on-approval it
  required). The lens is deferred-to on the integrity mechanism and overruled only on
  its thin-only preference.

- **Maintainability.** Judged full first-class *fiscal voucher* drafts too much
  permanent surface — a durable draft-CRUD table would grow an *include-drafts*
  boolean across every report/close/archive path (a second accounting system).
  Favoured the hybrid: thin fiscal drafts + a narrower durable **intake/work-item**
  layer, a lightweight O-29 on a **generic namespaced** O-25 substrate (separate from
  the rejection namespace), and a single `voucher correct <id> --mode
  reclass|reverse|adjust` builder over the existing validate/seal path. Rejected
  full-chain-weight draft logging as over-engineered for a pre-booking. Required
  contract tests that intake can never leak into a figure. These constraints shape
  §1, §3 and §4 directly.

---

## Related

- **ADR-1 (store)** — single Ledger write-port (intake lives behind it, not in a
  scratchpad DB); the `Cipher` seam encrypts intake as it does masterdata; fail-closed
  PII is why the O-29 log records hashes/diffs, not verbatim free text.
- **ADR-2 (wire format)** — a sealed voucher is immutable canonical bytes; an intake
  item is *not* wire and must never travel the strict `stored_bytes == reencoded_bytes`
  read path as-if-sealed.
- **ADR-3 (fiscal store)** — reporting nets corrections ("corrected to X") over the
  sealed store; intake is excluded from every derived read surface by construction.
- **ADR-4 (cost-centre)** — a correction/reclass voucher can re-target `dimensions`,
  so per-dimension P&L nets corrections the same way.
- `decisions.md` — **D-05/D-06** (seal-on-record settled), **O-21** (corrections to a
  sealed period are adjusting entries in the next open period — this ADR's post-seal
  surface is the how), **O-23** (Preview gate `bind-payload-hash` = the approval
  content-binding), **O-25** (the append-only hash-chained log substrate O-29 reuses),
  **D-08** (this decision), **O-29** (resolved by this ADR), new open items **O-34/O-35**.
- `seal-timing-research.md §3–§6` — the survey this ADR is the forward decision for;
  `s1-sealed-voucher/spike-notes.md` — the seal transaction + O-25 log this builds on;
  `scenarios.md` A1/A2/A5, B2, D1/D2 — capture, hunt, VAT filing, close.
- **Follow-up code work (spinoff, not this commit):** the durable intake stage +
  `intake` verbs; the `voucher correct` verb + `corrects` provenance; the generic
  namespaced O-25/O-29 audit log + `verify` extension; the must-seal preconditions in
  `close-period`/`close-year`/`file-vat`; reporting correction-netting (ADR-3); the
  contract-test battery; provisional-treatment chart accounts (O-34).

---

*Dated 2026-08-04: initial decision. This ADR decides seal timing — the pre-seal
intake window, the correction surface and its transparency line, the must-seal
policy, and the O-29 log — as workflow/architecture only; the intake stage,
`voucher correct` verb, O-29 log, must-seal preconditions and reporting netting are
recorded as follow-up implementation work, and no `crates/`/`skills/` code
accompanies this ADR. See git history for provenance.*
