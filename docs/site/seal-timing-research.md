# Seal-on-record vs. edit-until-lock — workflow fit for a small Finnish Oy

> **What this is.** A sourced *survey*, **not a decision and not an ADR**. It weighs
> how aggountant's chosen **seal-on-record** model (every voucher hash-chain-sealed the
> moment it is recorded; corrections are separate *oikaisutositteet*, never edits) fits
> the real day-to-day bookkeeping of a single small Finnish Oy, against the
> market-standard **edit-until-locked + change-log** model (Kitsas et al.). It
> **recommends nothing to change**; it gives the product owner the evidence to later
> decide — or not — via a separate `/worktree-technical-decision`.
>
> **The question (product owner, 2026-08-04).** Intuition: bookkeeping feels "open" and
> editable through the year and is only "locked" at tilinpäätös; aggountant instead seals
> on record. Is seal-on-record a workflow *misfit*, or is the intuition incomplete?
>
> **Method.** Statute + KILA guidance (Finlex / kirjanpitolautakunta.fi / KILA
> menetelmäohje), the landed Kitsas/FI-OSS benchmark (`benchmark.md`, `benchmark-koonti.md`),
> the S1 spike evidence (`s1-sealed-voucher/spike-notes.md`), the fiscal-store decisions
> (**ADR-1**, **ADR-3**, `decisions.md` D-06/D-07, open items O-29/O-33), and public
> vendor docs (Kitsas, Tilitin, Procountor/Netvisor). Full source list in §7.
>
> **Bottom line up front.** The product owner's intuition is **half right, and the half
> that is right is the important half.** Finnish law's *immutability* prohibition genuinely
> does not bite until **tilinpäätöksen laatiminen** (KPL 2:7) — so the year *is* legally
> "open" for content change. But two other requirements bind the whole time — the
> **kirjausketju** (KPL 2:6) and the **verifiability of every correction** (KPL 2:5) — so
> *silent, traceless* editing is **not** what the law contemplates even before the close.
> The market satisfies these by **editing but logging** (Kitsas `Tositeloki`); aggountant
> satisfies the *same* legal requirements by **never mutating** (correction vouchers).
> **Both are lawful routes.** They differ only in the pre-close mutation discipline, and
> the honest cost of seal-on-record is concentrated in exactly one place: the *pre-seal
> draft window* (open item **O-29**) and the ergonomics of "post a correction" vs. "fix
> the original." Nothing in the norm forces aggountant to change; the fit question is a UX
> and audit-parity judgement, not a legality one.

---

## 1. What the Finnish norm actually requires

Four statutory anchors in **Kirjanpitolaki (KPL, 1336/1997)** chapter 2 govern this
question, plus KILA's method guidance. The 2016 reform (HE 89/2015) renumbered and
reworded parts of chapter 2; current wording is cited below.

**KPL 2:5 — Tosite (the voucher).** "Kirjauksen on perustuttava päivättyyn ja
numeroituun tositteeseen, joka todentaa liiketapahtuman." Corrections get explicit
treatment: *"Oikaisu- ja siirtokirjauksen todentava tosite on asianmukaisesti
varmennettava,"* and a lighter route — *"korjausmerkintä saadaan tehdä ilman erillistä
tositetta edellyttäen, että alkuperäisen kirjauksen ja korjausmerkinnän yhteys on
vaikeuksitta todennettavissa."* So the law's native correction primitive is **an adjusting
entry whose link to the original is traceable** — conceptually the *oikaisutosite*, not a
silent overwrite ([Finlex 1336/1997](https://www.finlex.fi/fi/laki/ajantasa/1997/19971336);
statutory text also mirrored in [Kirjanpitolaki 1336/1997, pohjois-pohjanmaa.fi PDF](https://www.pohjois-pohjanmaa.fi/wp-content/uploads/2020/08/Kirjanpitolaki-1336-1997.pdf)).

**KPL 2:6 — Kirjausketju (the audit trail).** Current wording: *"Kirjanpito on
järjestettävä niin, että liiketapahtumien, tositteiden ja kirjausten yhteys mahdollisten
osakirjanpitojen kautta pääkirjanpitoon ja siitä tilinpäätökseen on vaikeuksitta
todettavissa **kumpaankin suuntaan**."* The trail must hold **at all times and in both
directions** — this is not gated on the close ([Taloushallintoliitto — Aukoton
kirjausketju / audit trail](https://taloushallintoliitto.fi/tietopankki/kirjanpidon-abc/tositteet-kirjanpidon-pohjana/);
[Grant Thornton — KILAn menetelmäyleisohje](https://www.grantthornton.fi/ajankohtaista/kirjanpitolautankunnan-uusi-yleisohje-kirjanpidon-menetelmista-ja-aineistoista/)).
Taloushallintoliitto notes **aukoton juokseva numerointi** is a *good way* to secure the
trail's completeness — **"vaikka numeroinnin perustana voidaan käyttää myös muuta
systemaattista ja yksilöivää numerointitapaa."** I.e. gapless running numbering is
**endorsed practice, not a hard statutory mandate**.

**KPL 2:7 — muuttamattomuus (the immutability bite point).** This is the section the
owner's intuition is really about. Current text: *"Tositteen, kirjanpidon tai muun
kirjanpitoaineiston sisältöä ei saa muuttaa tai poistaa **tilinpäätöksen laatimisen
jälkeen**."* It extends the prohibition to the moment the material is otherwise
committed: also no change *"sen jälkeen, kun … kirjanpitovelvollinen on julkistanut tai
antanut ulkopuolisen käyttöön aineistosta laatimansa välitilinpäätöksen tai muun
vastaavan asiakirjan."* A **format** change is allowed *"jos se on tarpeen käsittelyn,
siirron tai säilytyksen vuoksi"* — but then *"on varmistettava **sisällön ja
kirjausketjun säilyminen**."* The older/continuing companion rule: kirjanpitomerkinnät
tehdään *"selvästi ja pysyvästi,"* and a *"kirjausta ei saa poistaa eikä tehdä
epäselväksi."* (Finlex 1336/1997; wording confirmed in vendor guidance quoting the statute,
e.g. [Kitsas — tilikauden muokkaaminen](https://kitsas.fi/docs/tilikaudet/muokkaa/).)

**KPL 2:4 — timing.** Cash transactions **viipymättä päiväkohtaisesti**; other entries
may be booked monthly (or similar) **within four months** of period end. So the norm
already assumes a *lag* between the event and its final booking — the "open window" is
real and legally sized.

**Retention (KPL 2:10).** Accounting records/vouchers are kept **six years** from the end
of the year the period closed (peripheral to seal-timing, noted for completeness).

**KILA method guidance.** The [KILA yleisohje kirjanpidon menetelmistä ja aineistoista
(20.4.2021)](https://www.edilex.fi/kilaohje/20210420) is the operative interpretation for
machine-kept books: it lets the whole chain be **digital**, and requires **completeness of
machine-stored records to be demonstrable** (e.g. transaction counts + totals stored per
booking period and reconciled) and the **kirjausketju preserved** across any processing
or format change. It is the guidance layer where "changes must stay traceable" is
spelled out for software; it does **not** mandate any specific mechanism (no hash chain,
no particular lock model).

### 1.1 The precise answer to "is silent editing before lock permitted?"

Distinguish three layers:

- **Statute (KPL 2:7):** the *immutability prohibition* activates at **tilinpäätöksen
  laatiminen** (or earlier publication/filing). Before that, the law does **not** forbid
  changing a booked entry's content. → *The intuition is correct here.*
- **Statute (KPL 2:6 + 2:5):** but the **kirjausketju must hold at all times**, and any
  correction's **link to the original must be verifiable**. A change that leaves *no*
  trace erodes 2:6/2:5. → *"Silent" (traceless) editing is not what the norm
  contemplates, even mid-year.*
- **Practice / KILA:** the market resolves the two by **editing openly but recording the
  change** (a change log) so the trail survives, or by **not mutating at all** (adjusting
  entries). Both preserve the ketju; the difference is bookkeeping technique, not
  legality.

So: **editing before the close is permitted; *silent* editing is at best tolerated and at
worst a 2:6 gap — and no serious tool relies on it.** aggountant's seal-on-record and the
market's edit-until-lock are **two lawful implementations of the same two requirements**
(unbroken trail + post-close immutability). aggountant's continuous hash chain and gapless
numbering are **above the statutory floor**, not demanded by it — matching `benchmark.md
§3.2`'s reading.

---

## 2. How real tools behave

The benchmark placed the market on a five-point immutability spectrum
(`benchmark.md §3.2`, koonti **K-15**); this section extends it with the pre-close *edit
and correction* behaviour, which is what the workflow question turns on.

| Tool | Pre-close editability | How the trail is preserved | Lock / close | Placement |
|---|---|---|---|---|
| **Tilitin** (thelineva, GPLv3) | **Fully mutable**, no period lock at all | External discipline + backups only — the [user guide](https://helineva.net/tilitin/ohjeet/) documents no lock, no archive, no change log | none in-app | spectrum **(a)** trust-the-file |
| **Kitsas / Kitupiikki** (GPLv3) | Mutable while the period is open; vouchers move through a state machine (`SAAPUNUT → TARKASTETTU → HYVÄKSYTTY → KIRJANPIDOSSA`) | **`Tositeloki`** change-log table records who changed which voucher when, with the prior `data` snapshot (`benchmark.md §1.2`) | period **lukitseminen**; year-end **`Arkistoija`** freezes to HTML/PDF + a **SHA256 sigill** (`ARKISTOVARMENNE`) | spectrum **(b)+(c)** |
| **Procountor** (cloud) | Voucher has a status; a saved memo voucher sits **"In progress"** and stays editable; postings of an approved voucher can be returned to In-progress to void/fix; view shows **created + last-modified timestamps** ([Procountor — Uusi muistiotosite](https://support.procountor.fi/hc/fi/articles/360000254937-Uusi-muistiotosite)) | status model + modification timestamps + system change history | month-end / period lock ([Netvisor — month-end closing](https://support.netvisor.fi/en/support/solutions/articles/77000466437-month-end-closing-and-account-reconciliation)) | spectrum **(b)** |
| **Netvisor** (cloud) | Edit-until-confirmed; invoices/vouchers are **approved/verified** then posted | approval workflow + audit history | month-end closing / lock | spectrum **(b)** |
| **aggountant** (this project) | **No mutation of a sealed voucher** — a *draft* is assembled, validated, then **sealed on record**; corrections are separate *oikaisutositteet* | **continuous per-voucher hash chain** `H(prev‖canonical)` + gapless CAS numbering, `voucher verify` re-walks it any time (`spike-notes.md`); rejection log (O-25) | period/year **seals**; git audit mirror + `fiscal archive` digest (ADR-3) | spectrum **(d)(+e)** |

**Two market patterns, one legal target.** Kitsas is the reference *edit-until-lock +
change-log* implementation: it *does* let a bookkeeper open a locked period again *"vain,
jos tilinpäätöstä laadittaessa on vielä tarpeus palata tekemään kirjauksia"* and warns
the KPL 2:7 prohibition applies once the close/filing is done ([Kitsas
docs](https://kitsas.fi/docs/tilikaudet/muokkaa/)). Tilitin sits one notch *weaker* than
aggountant's opposite pole — a fully mutable file with **no** in-app lock, log, or archive —
and is nonetheless a long-used FI small-business tool, which tells us the market's
*effective* floor for a tiny entity is low, carried by backups and the bookkeeper's
discipline rather than by the software. Procountor/Netvisor sit where Kitsas does:
edit-until-confirmed/locked, with an approval status model and system-level change
history.

**Reading:** the entire mainstream market is at **(b)/(c)** — mutable until lock, trail
preserved by a *log* and a *point-in-time year digest*. aggountant at **(d)/(e)** is
deliberately stricter. So seal-on-record is not "wrong for the workflow"; it is **above
the workflow's market baseline**, and the fit question is whether that extra strictness
costs more in daily friction than it returns in audit strength for *this* user.

---

## 3. The actual small-Oy workflow

A realistic single-Oy cycle (entrepreneur or part-time bookkeeper, agent-assisted):

1. **During the month** — receipts, invoices, bank events get booked, often in batches,
   within the KPL 2:4 window (cash immediately; the rest within four months). Data is
   incomplete: a receipt arrives before its category is clear; a bank line lacks a
   counterparty; VAT code is a guess pending the invoice PDF.
2. **A week or two later** — an error surfaces: wrong account, wrong VAT code, a
   duplicate, a receipt that turns out to be personal. This is the **single most common
   correction event**, and it lands *inside* the open period.
3. **VAT filing (ALV)** — filed per *verokausi* (monthly, quarterly, or yearly by
   turnover) to OmaVero, due the 12th of the second month after the period. Filing a
   period **commits** those numbers outward (a KPL 2:7 trigger-adjacent event: the data is
   given to an authority). Errors found *after* filing follow the OmaVero correction
   rules (small net errors carried to a later period; larger ones via an *oikaisuilmoitus*
   / corrected return for the original period).
4. **Accountant / tilintarkastaja review before tilinpäätös** — a pass over the year;
   reclassifications and accruals get posted; then the year is closed and (2:7) frozen.

**Where edits actually happen:** overwhelmingly at step 2 (open-period fixes) and step 4
(pre-close reclass). Step 2 is high-frequency and low-stakes; step 4 is low-frequency and
deliberate. The design tension is entirely about **step 2**: how disruptive is "post a
correction voucher" versus "edit the original," dozens of times a year, for small fixes?

**Edit-the-original (market):** the bookkeeper opens voucher #123, changes the account,
saves; `Tositeloki` records the diff. One object, mentally simple ("I fixed the receipt"),
and the running reports show the corrected state directly. Cost: the *as-booked* history
exists only inside the log, in the same mutable DB (`benchmark.md §1.2` — detective, not
preventive).

**Post-a-correction (seal-on-record):** the original #123 stays sealed; a new
*oikaisutosite* #171 reverses/adjusts it. Two objects, and the account balance is the net
of both. This is **exactly the KPL 2:5 native primitive** (*oikaisukirjaus*, link
verifiable) and **exactly how a post-filing VAT correction must legally work anyway** —
but for a trivial mid-month typo it is more ceremony than a human expects, and naïve
reports show two lines where the user thinks of one. Whether that ceremony is acceptable
depends heavily on how well the **agent** hides it (see §5) — the agent can generate the
oikaisutosite from "that receipt was wrong, fix it" so the *human* still issues one
intention.

---

## 4. The two models compared on the axes that matter

| Axis | Seal-on-record (aggountant) | Edit-until-lock + log (market) |
|---|---|---|
| **Audit strength / tamper-evidence** | **Preventive + continuous.** Any retro change breaks the chain; `voucher verify` detects it at any moment with no out-of-band artefact. Above statutory floor. | **Detective + point-in-time.** The `Tositeloki` shows *accepted* changes but lives in the same mutable store; the year digest only protects *if the certificate is preserved/signed outside the tool* (`benchmark.md §1.2`). Meets the floor; trust externalised. |
| **UX friction (mid-period fixes)** | Higher *per correction* (new voucher, net-of-two balances) unless the agent collapses it to one intention. Draft window softens it (see below). | Lower — edit the object in place; running reports show the fixed state directly. This is the market's main ergonomic win. |
| **VAT-period corrections** | **Better fit after filing.** A filed period is committed; the lawful correction is an adjusting entry, which is *native* here. No "reopen and silently edit a filed period." | Convenient *before* filing (edit original); *after* filing must also add an adjusting entry or reopen — so the edit-in-place advantage evaporates exactly where stakes rise. |
| **Pre-seal draft window (O-29 / K-14)** | The **pressure point.** If the seal is immediate, the "editable" phase is the *draft* (validate → seal), and O-29 asks whether accepted pre-seal changes need their own log for parity with `Tositeloki`. If the draft is a real multi-day holding area (à la Kitsas `SAAPUNUT/TARKASTETTU`), that log likely *is* needed. | The whole open period *is* the editable window, and `Tositeloki` already logs it. No separate concept. |
| **Reversibility of mistakes** | Every state is recoverable *forward* (correction vouchers); nothing is ever lost, but nothing is ever silently undone either. | An edit can quietly replace the prior state; recovery relies on reading the log / backups. |

**Synthesis of the axes.** Seal-on-record dominates on *audit strength* and on
*post-filing VAT correctness*, and ties or wins on *reversibility as a record*. It loses
on *raw mid-period friction*, and it concentrates a genuine open design question in the
**pre-seal draft window**. The market model wins *only* on everyday edit convenience — a
real but agent-addressable cost — and pays for it with a weaker, externalised trust model
that aggountant deliberately rejected (`ADR-1`, `ADR-3`, D-06).

### 4.1 The pre-seal draft window is the crux (O-29)

S1 already ships a draft stage: `voucher draft → voucher validate → voucher seal`
(`spike-notes.md`). So seal-on-record does **not** mean "no editable phase" — it means the
editable phase is **pre-seal and un-booked**, and the seal is the *commit*. Two shapes are
possible, and they change the whole fit verdict:

- **Thin draft (seal ≈ immediately).** The draft exists only for the seconds/minutes of
  assembly; nothing sits half-booked. Closest to the current S1 behaviour. Pro: maximum
  tamper-evidence, minimal ambiguity. Con: every human "oops" after the seal is a
  correction voucher — the §3 friction.
- **Explicit pre-seal window (a real "received / in-progress" stage).** A voucher can be
  captured, revised, and reviewed for days *before* it is sealed into the chain — the
  agent's `capture-receipt`/`hunt-receipts` staging naturally wants this. This restores
  the market's "edit freely, then commit" ergonomics *without* mutating sealed truth. Con:
  it re-creates exactly the mutable window Kitsas logs with `Tositeloki`, so **O-29's
  pre-seal accepted-change log becomes necessary for audit parity** (the pre-seal window
  otherwise has no accepted-change trail).

This is the single most decision-relevant finding of the survey: **the "misfit" the owner
senses is not seal-on-record per se — it is the absence (so far) of a deliberately
designed pre-seal window and its change-log.** O-29/K-14 already parks this as
*"lykätään, päätetään harkiten … todennäköisesti tarpeen audit-pariteetin vuoksi"*
(`decisions.md` O-29). The survey supports that instinct.

---

## 5. Where seal-on-record genuinely helps vs. hurts *this* user

**Genuinely helps** (single small Oy, agent-driven, correctness legally critical):

- **The agent can't quietly rewrite history.** The whole point of an AI-first ledger is
  that an autonomous agent proposes bookings. A model where the agent (or a hallucinated
  retry) could silently *edit* a booked voucher is far more dangerous than one where every
  change is an append-only, chain-verifiable event. Seal-on-record turns "did the agent
  alter last month?" into a question answerable **by construction** — the same argument
  ADR-3 makes for the read-only analytics replica.
- **Post-filing VAT and pre-close reclass are correction-shaped anyway.** The two
  higher-stakes edit moments (§3 steps 3–4) *must* be adjusting entries under the norm.
  Seal-on-record makes the lawful path the *only* path, removing a foot-gun.
- **Auditor story is stronger, not just equal.** Continuous verification + the ADR-3
  archive digest gives the tilintarkastaja a portable artefact that beats the Kitsas
  externalised-trust digest (`benchmark.md §3.2`, D-06).

**Genuinely hurts:**

- **Mid-period micro-corrections feel heavy** unless the agent fully absorbs the
  correction-voucher mechanics into one human intention. This is a UX obligation on the
  Skill layer, not a substrate flaw — but it is real, and it is where a human coming from
  Kitsas will feel the difference first.
- **No designed pre-seal window yet.** Until O-29 is decided, the agent's natural
  "capture now, finalise later" flow has nowhere clean to live: either everything seals
  instantly (friction), or drafts linger with no accepted-change trail (audit gap). This
  is the concrete gap to close, and it is a *feature* decision, not a reversal of the
  chain.
- **Reporting must net corrections gracefully.** Two-voucher corrections must render as a
  clean "corrected to X," or users will perceive clutter. An ADR-3 analytics/reporting
  concern, not a sealing concern.

**Net:** for this exact user, seal-on-record is a **good fit on the axes that carry the
legal and agent-safety risk**, and a **worse fit on everyday edit ergonomics** — and the
ergonomic gap is closable by (a) agent UX that hides the correction-voucher mechanics and
(b) a deliberate pre-seal draft window (O-29). Nothing in the survey argues for
abandoning the chain; the evidence points at *completing* the model, not replacing it.

---

## 6. Options space (described, not chosen)

Three coherent positions. This survey **does not pick one** — that is a later
`/worktree-technical-decision` if the owner wants it (see spinoff note).

**Option A — Seal-on-record as-is (thin draft).** Keep S1's behaviour: assemble, validate,
seal immediately; corrections are always oikaisutositteet.
- *Pros:* maximum, continuous tamper-evidence; simplest mental/threat model; the lawful
  correction path is the only path; least new code.
- *Cons:* highest mid-period friction; leans entirely on agent UX to feel natural; the
  "capture now, finalise later" flow has no home; a human from Kitsas feels the change
  immediately.

**Option B — Seal-on-record + an explicit pre-seal draft window (with O-29 log).** Add a
first-class *un-booked draft / "received / in-progress"* stage where a voucher can be
freely revised and reviewed before it is sealed; log accepted pre-seal changes for parity
with `Tositeloki`; the seal remains the immutable commit.
- *Pros:* recovers the market's "edit freely, then commit" ergonomics **without** mutating
  sealed truth; gives the agent's capture/hunt flows a clean home; closes the O-29 audit
  gap; keeps the chain's guarantees fully intact post-seal.
- *Cons:* re-introduces a mutable pre-seal window that now needs its *own* change-log and
  contract tests (more surface); a policy line must define when a draft *must* seal
  (e.g. before a VAT filing, before period close) so drafts can't be used to park real
  liabilities off-chain.

**Option C — Adopt the market's edit-until-lock + change-log model.** Replace seal-on-record
with mutable-until-period-lock vouchers plus a `Tositeloki`-style accepted-change log and a
year-end archive digest.
- *Pros:* lowest daily friction; maximal familiarity for a bookkeeper; provably "enough"
  for the FI market (Kitsas/Procountor/Netvisor all ship it).
- *Cons:* **reverses D-05/D-06 and re-litigates a settled question** (`ADR-3` §Context:
  chain-vs-lock is *settled*); trades preventive+continuous tamper-evidence for
  detective+point-in-time with externalised trust; worst fit for an *autonomous agent*
  writer (silent edits become possible); throws away S1/M1's already-built and validated
  chain substrate. Included for completeness; it is the option the accepted architecture
  deliberately rejected.

**Observation (not a recommendation):** Options A and B both *keep* the chain; they differ
only in whether the pre-seal window is thin or first-class. The survey's evidence
(§4.1, §5) is that the owner's felt "misfit" maps onto the A↔B axis — the draft window —
far more than onto the B↔C axis (chain vs. lock). If any decision is worth taking, it is
**A vs. B (O-29)**, not a reopening of C.

---

## 7. Sources

**Statute & official guidance (facts / freely citable):**
- Kirjanpitolaki 1336/1997, current consolidated text — [Finlex](https://www.finlex.fi/fi/laki/ajantasa/1997/19971336); statutory chapter-2 text also at [pohjois-pohjanmaa.fi PDF](https://www.pohjois-pohjanmaa.fi/wp-content/uploads/2020/08/Kirjanpitolaki-1336-1997.pdf) (pre-2016 numbering) and [Edilex 1336/1997](https://www.edilex.fi/lainsaadanto/19971336). Cited: 2:4 (kirjausaika), 2:5 (tosite, oikaisu-/korjausmerkintä), 2:6 (kirjausketju, kumpaankin suuntaan), 2:7 (muuttamattomuus tilinpäätöksen laatimisen jälkeen), 2:10 (säilytysaika).
- KILA — Yleisohje kirjanpidon menetelmistä ja aineistoista, 20.4.2021 — [Edilex kilaohje 20210420](https://www.edilex.fi/kilaohje/20210420); summary at [Grant Thornton](https://www.grantthornton.fi/ajankohtaista/kirjanpitolautankunnan-uusi-yleisohje-kirjanpidon-menetelmista-ja-aineistoista/) and [Suomen Tilintarkastajat](https://tilintarkastajat.fi/uutiset/uusi-yleisohje-kirjanpidon-menetelmista-ja-aineistoista/).
- Suomen Taloushallintoliitto — [Aukoton kirjausketju eli audit trail](https://taloushallintoliitto.fi/blog/tietopankki/kirjanpidon-abc/aukoton-kirjausketju-eli-audit-trail/) and [Tositteet kirjanpidon pohjana](https://taloushallintoliitto.fi/tietopankki/kirjanpidon-abc/tositteet-kirjanpidon-pohjana/) (juokseva numerointi as *a* good practice, not a mandate).

**Vendor behaviour (reference only; Kitsas/Tilitin are GPLv3 — studied, not copied):**
- Kitsas — [Tilikauden muokkaaminen / lukitseminen](https://kitsas.fi/docs/tilikaudet/muokkaa/) (period lock, reopen caveat, KPL 2:7 quote); voucher state machine + `Tositeloki` per `benchmark.md §1.2`.
- Tilitin — [Käyttöohjeet (helineva.net)](https://helineva.net/tilitin/ohjeet/) (no lock / no archive / no change log — fully mutable, tositelaji number ranges).
- Procountor — [Uusi muistiotosite](https://support.procountor.fi/hc/fi/articles/360000254937-Uusi-muistiotosite) (voucher status "In progress", created/modified timestamps).
- Netvisor — [Month-end closing and account reconciliation](https://support.netvisor.fi/en/support/solutions/articles/77000466437-month-end-closing-and-account-reconciliation) (period close/lock).

**aggountant internal (evidence base):**
- `benchmark` §1.2, §3.2 (immutability spectrum, Kitsas archive digest); `benchmark-koonti` K-10…K-15 (removed pages, in git history under `docs/site/`).
- `adr-1-store` (landed and removed; its rules are in `crates/ledger/AGENTS.md`), [`adr-3-fiscal-store`](adr-3-fiscal-store) (chain-vs-lock settled; archive + analytics projections); `decisions` D-06/D-07, **O-29** (pre-seal accepted-change log), O-33 (removed page, in git history).
- `issues/s1-sealed-voucher/spike-notes.md` (M1: gapless CAS + chain integrity proven under concurrency; git mirror + crash-injection stubbed).

---

## 8. Spinoff proposal (flag, not an action)

This survey is **research, not a decision.** If the product owner wants a forward call, the
decision-relevant question it surfaces is **not** "chain vs. lock" (settled, D-06) but
**"thin draft vs. explicit pre-seal draft window + O-29 change-log"** (Option A vs. B). A
`/worktree-technical-decision` scoped to **O-29 / K-14 — the pre-seal accepted-change log
and whether seal-on-record should gain a first-class pre-seal editing window** would be the
right vehicle. Filed here as a proposal only; no ADR is authored and no decision is
recorded by this work.

*Dated 2026-08-04. Research survey for the product owner; supersedes nothing, decides
nothing. See git history for provenance.*
