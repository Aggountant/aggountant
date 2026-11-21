[model:gpt-6-astra] [thread_id:01a1fd03-bb78-7510-ac84-700c807c1c87]

I read both files and checked the relevant CLI help definitions and close-year implementation. I treated SOUL.md’s professional judgment, approval rules, and command discovery as supplied context—not omissions to repeat in this skill.

These are the execution ambiguities I found, in workflow order. References below are to `skills/close-year/SKILL.md`.

### 1. When the user prepares the statements, where does the comparison document come from?
**Lines 28–30, 87–88**

> “Luvut tulevat tilinpäätöksen tekijältä, joka on tilitoimisto, kirjanpitäjä tai käyttäjä itse.”

> “Vertaa tuloslaskelmaa ja tasetta rivi riviltä tekijän tilinpäätökseen ennen kuin pyydät hyväksyntää.”

With an external accountant’s completed statements, this is clear. When the user prepares them, they may expect this run to produce the figures they will put into the document. I must decide whether to stop before starting, proceed to the statements gate and supply its figures, or require an independently prepared document.

**Clarify:** Whether the run’s preview may supply the figures for the externally assembled document, provided that document is finalized and compared before sealing.

### 2. Accruals may already have settled before this run starts
**Lines 35, 60–62**

> “Uuden vuoden kirjaaminen, kuukausien sulku ja ALV-ilmoitukset jatkuvat sillä välin tavallisesti.”

> “Kun jaksotettu lasku tai maksu aikanaan tulee, se kirjataan jaksotustiliä vasten, ei uudelleen kuluksi.”

This handles a future invoice, but not one already booked during the permitted wait. If it was booked as a new-year expense before the old-year accrual was agreed, following this skill now can leave both expense entries unless I notice and arrange the new-year correction.

I also have to decide when to write the TODO entry: immediately after identifying the accrual, after its voucher is sealed, or only after the entire year closes.

**Clarify:** Check whether each accrual has already settled; reconcile any existing new-year booking. Record outstanding settlement work when the accrual becomes permanent, including when the overall run is subsequently rejected.

### 3. Missing VAT filings: warning or prerequisite?
**Lines 80–81**

> “ALV-velka ilmoitettua. Ajo ei tarkista, että vuoden ALV-ilmoitukset on annettu.”

I understand that the tool does not check submission status, but not what decision I must make after checking it myself. If a return is missing, should I suspend year-end closing, hand off the filing and return afterward, or proceed when the preparer confirms the liability?

This is not a request for filing instructions here; the missing criterion is whether the finding blocks this workflow.

**Clarify:** State the intended consequence of missing or unresolved VAT filings, without implying that the CLI enforces it.

### 4. Manual depreciation conflicts with the notes approval criterion
**Lines 53–54, 84–86**

> “Jos yritys poistaa vähemmän tai muulla suunnitelmalla, jätä hyödyke pois hyödykkeistä ja anna sen poisto oikaisuna.”

> “Käyttäjä vahvistaa, että ne ovat samat kuin tilinpäätösasiakirjassa.”

Following the first instruction means manually depreciated assets do not contribute to the generated depreciation notes. The implementation confirms this: `crates/process/src/close_year.rs:1277–1300` builds those notes exclusively from calculated depreciation lines.

At the notes gate, I must guess whether “samat” means:
- every generated note agrees with the document, although the document contains additional methods; or
- the generated notes must cover all depreciation in the document.

The latter cannot generally be satisfied using the prescribed manual-correction route.

**Clarify:** Generated notes cover automatic depreciation only; explicitly account for manual depreciation in the external document, and define approval as agreement within the generated notes’ limited scope.

### 5. “Line by line” needs a rule for different presentation formats
**Lines 87–88**

> “Vertaa tuloslaskelmaa ja tasetta rivi riviltä tekijän tilinpäätökseen”

If the preparer groups accounts differently or displays rounded figures, literal row equality may be impossible despite matching underlying books. I must choose whether to reject, reconcile the grouping, or accept apparent rounding differences.

**Clarify:** Permit an explicit reconciliation of presentation differences, while requiring unexplained numerical differences to be resolved before approval. This does not need a detailed comparison algorithm.

### 6. Recovery does not distinguish unsealed mistakes from permanent ones
**Lines 48–50, 94–97**

> “Ajon, joka ei ole sinetöinyt mitään, voi hylätä jälkiä jättämättä.”

> “Kun vaiheessa löytyy virhe, hylkää vaihe ja käynnistä uusi ajo, jonka syötteessä ovat samat hyödykkeet, jaksotukset ja oikaisut samoin summin sekä virheen korjaava uusi oikaisu.”

Suppose the initial depreciation preview is wrong and nothing has been sealed. The first passage suggests discarding and correcting the input; the second instructs me to preserve the wrong input and add a compensating correction.

The ambiguity also occurs when only some corrections have been sealed: must I preserve erroneous entries that remain entirely unsealed?

**Clarify:** Preserve and counter-correct permanent entries; directly fix entries that have never been sealed. This avoids unnecessarily turning a preview mistake into permanent bookkeeping.

### 7. Restart identity is underspecified
**Lines 95–97**

> “samat hyödykkeet, jaksotukset ja oikaisut samoin summin”

Does “same” require the original identifiers and complete entry definitions, or merely economically equivalent entries with matching amounts?

This matters operationally: `crates/process/src/close_year.rs:1097–1123` derives reuse keys from asset, accrual, and correction IDs. Reconstructing equivalent input with fresh IDs is not equivalent recovery. Help documents the ID fields, but not this recovery contract.

**Clarify:** Reuse the original input—including identifiers and original posting details—for already sealed entries; append new corrections rather than reconstructing those entries from memory.

### 8. A changed ledger is not necessarily a mistaken entry
**Lines 94–98**

> “Kun vaiheessa löytyy virhe, hylkää vaihe ja käynnistä uusi ajo”

> “ja kieltäytyy, jos kirjanpito on muuttunut vaiheen avautumisen jälkeen.”

The skill identifies stale approval content but does not distinguish it from incorrect bookkeeping. If legitimate intervening activity invalidates a gate, I must decide whether the recovery needs a new correction or merely a fresh run and renewed review.

**Clarify:** A stale gate requires refreshed review; add a correction only if reconciliation identifies an actual accounting error.

**Highest-value fixes:** distinguish sealed from unsealed recovery, explain manual-depreciation notes, and cover accruals already settled during the wait. Those affect the accounting outcome, rather than just making execution smoother.
[model:gpt-6-astra] [thread_id:01a1fd03-bb64-7777-a2d9-7b20243b2e3c]

I reviewed both supplied files and the relevant CLI/process implementation. Ratings below include coverage inherited from `SOUL.md` and command help; missing flag documentation is not a defect.

Paths abbreviated below:
- **Skill:** `skills/close-year/SKILL.md`
- **SOUL:** `skills/aggountant-init/SOUL.md`

### 1. Empty or malformed input — **Addresses it**
**Evidence:** Skill:44–46:
> “Siksi kaikki, mikä kuuluu päättyneelle vuodelle, annetaan käynnistyksen syötteessä; kesken ajon sitä ei voi täydentää.”

SOUL:85–90 delegates syntax to command help and documents structured errors. The actual help distinguishes the required year from optional adjustment lists (`crates/cli/src/input_help.rs:233–249`).

**Assessment:** Empty stdin and malformed fields belong to CLI validation. Empty adjustment lists are distinct from missing financial evidence; the skill requires agreement with the preparer and comparison against final figures.

**Recommendation:** No additional JSON instructions needed.

### 2. Dirty repository state — **Addresses it weakly**
**Evidence:** Skill:106–108:
> “Anna ne tekijälle, jos hän tarvitsee ne, ja commitoi koti.”

SOUL:48–54 delegates tracked-file policy to the home’s instructions and protects user edits to `TODO.md`, but neither document addresses unrelated staged changes or an unresolved merge.

**Risk:** A blanket commit could incorporate unrelated work. Trying to obtain a clean checkout through reset/checkout could discard bookkeeping or user notes.

**Recommendation:** Put one shared rule in SOUL: inspect existing changes, preserve them, and commit only the intended accounting work. Do not require a pristine repository before every operation.

### 3. Missing prerequisites — **Addresses it weakly**
**Evidence:** Skill:29–30:
> “Sovi hänen kanssaan hyödykkeet, jaksotukset ja oikaisut ennen ajon käynnistämistä, ja vie ajo loppuun vasta, kun tekijän luvut ovat lopulliset.”

Skill:80–81:
> “Ajo ei tarkista, että vuoden ALV-ilmoitukset on annettu.”

**Assessment:** Closed months, final figures, asset opening balances and missing home instructions receive meaningful coverage. However, the VAT statement identifies a missing tool check without explicitly telling the agent to establish filing status. The adjustment destination is described at lines 40–42, but no-open-period recovery is left to tool errors.

**Recommendation:** Explicitly establish outstanding VAT filings before final approval, resolving or surfacing them without automatically submitting anything. Let help/errors handle mechanical prerequisites rather than duplicating them.

### 4. Interruption mid-run — **Addresses it weakly**
**Evidence:** Skill:71–73:
> “`close-year show` näyttää avoimen vaiheen sisällön juuri sellaisena kuin hyväksyntä sen sitoo; näytä se käyttäjälle, älä omaa yhteenvetoasi muistista.”

This supports reinspection, but the skill never distinguishes an interrupted run from a rejected run.

**Risk:** An agent may follow the restart instructions at lines 93–98 after a timeout, even though approvals or vouchers already persisted.

**Recommendation:** Add a short instruction to recover the existing run’s identity and inspect its state before restarting. Use its resume operation for interrupted automation. This operation already exists and can perform previously approved writes (`crates/process/src/close_year.rs:656–689`); it is not merely a read.

### 5. Concurrent invocation — **Addresses it weakly; largely outside v1 scope**
**Evidence:** Skill:97–98:
> “Ohjelma nimeää puuttuvat kirjaukset, jos uusi ajo ei kata niitä, ja kieltäytyy, jos kirjanpito on muuttunut vaiheen avautumisen jälkeen.”

**Assessment:** Stale accounting state is covered, but duplicate active runs and actual simultaneous execution are not. Repository instructions explicitly exclude concurrent-process operation from v1.

**Recommendation:** Do not add locking procedures to this skill. Cover the supported, sequential equivalent: inspect and resume an existing unfinished run rather than casually starting another. Any genuine concurrency requirement belongs in the deferred multi-user register.

### 6. Partial completion — **Addresses it weakly overall; rejection recovery is well covered**
**Evidence:** Skill:93–97:
> “Vaiheen hylkääminen päättää ajon, mutta sen jo sinetöimät tositteet jäävät kirjanpitoon.”

> “Väärin sinetöityä vientiä ei jätetä pois, vaan se perutaan vastaoikaisulla.”

**Assessment:** This correctly rejects rollback assumptions. Two narrower gaps remain:
- Replacement runs must preserve original identifiers, not merely amounts. Replay uses natural keys (`crates/process/src/close_year.rs:1033–1090`).
- Sealing, archive creation, knowledge updates, TODO updates and Git commit are separate completion outcomes.

**Recommendation:** Say to reuse the original input unchanged—including identifiers—and add correcting entries. After sealing, verify outstanding deliverables and complete only those; do not repeat the financial close because a commit or note update failed.

### 7. Disk exhaustion and API limits — **Ignores resource-specific recovery**
**Evidence:** Skill:106–107:
> “Ajo kirjoittaa hyväksytyt laskelmat, tase-erittelyt ja liitetiedot kotiin hakemistoon `archive/<vuosi>/`.”

**Material finding:** Archive writing is best-effort: its error is discarded after sealing (`crates/process/src/close_year.rs:885–893`). Consequently, successful sealing does **not** establish that the promised archive exists.

**Recommendation:** Verify the archive before delivery or declaring completion. If missing, report “year sealed, archive incomplete”; do not imply rollback or blindly restart. Separately, the implementation should expose archive failure or a supported repair route.

API-rate-limit procedures would add little value here: the described close workflow does not submit authority filings.

### 8. Unauthorized or destructive prerequisites — **Addresses it**
**Evidence:** Skill:66–67:
> “Se on sinetöinti, joten näytä oikaisu käyttäjälle ja pyydä hyväksyntä ennen komentoa.”

Skill:88–89:
> “Hyväksyntä sinetöi tilikauden, eikä sitä voi perua.”

SOUL:5–6 limits action to delegated authority; SOUL:76–78 requires advance approval of irreversible actions and renewed presentation when proposals change.

**Assessment:** These rules also constrain prerequisite actions. SOUL’s preference for `next_command` does not grant additional authority.

**Recommendation:** No repeated approval framework needed. If clarifying SOUL, explicitly state that suggested recovery commands remain subject to the same authorization rules.

**Priority:** Fix the archive-success assumption first, then clarify interrupted-run recovery and preservation of original input identifiers. The other gaps mostly belong in shared operating guidance, not a longer year-close checklist.
