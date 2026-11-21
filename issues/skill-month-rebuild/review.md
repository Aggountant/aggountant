# Review: `skills/capture-receipt/SKILL.md`

`/llm-skill-review`, 2026-11-21, routine rigor: one external model (`gpt-6-astra`), lenses executor and edge-case, with `skills/aggountant-init/SOUL.md` attached as the shared context every skill is read with. The file reviewed was the 1,149-word draft; the fixes the assessment accepted were applied afterwards, so line numbers refer to the draft. The model's output is reproduced unedited.

## Executor lens

I read both files, the relevant CLI input help, and the filing handoff in `vero-fi`. I treated command syntax, ordinary accounting judgment, and SOUL’s approval/company-knowledge rules as supplied—not missing instructions.

These are the remaining execution ambiguities, in workflow order.

### 1. Which purchases may use a manually prepared voucher?
**`skills/capture-receipt/SKILL.md:42–46, 123–131`**

> “Itse `voucher draft` -riveiksi kootut ALV-viennit eivät kanna niitä”

But later:

> “ALV-ilmoitus näkee tällaisen tositteen veron vain `vat_code`-merkinnästä”

The first passage appears to prohibit manual VAT entries categorically; the second explains how manual purchase entries become visible to the return. When `capture-receipt record` rejects a purchase, I cannot tell whether a correctly tagged manual voucher is an authorized alternative or a prohibited workaround.

**Clarify:** distinguish ordinary purchases that must use `record`, legitimate manual corrections/special entries, and unsupported treatments that must remain unresolved. Qualify the first claim rather than implying that all manual VAT entries lack the necessary information.

### 2. What source identity must survive subsequent sessions?
**`skills/capture-receipt/SKILL.md:35–38, 68–70`**

> “`natural_key` johdetaan lähteestä, jotta sama kuitti tunnistetaan myöhemmin.”

Neither this nor the command help defines the identity convention. Supplier/date, invoice number, and attachment hash all satisfy “derived from the source,” but behave differently when two receipts share a date, the same invoice arrives as another PDF, or one source requires multiple vouchers.

Attachment lookup establishes that particular bytes were used; it does not necessarily establish that the economic transaction is new.

**Clarify:** use a durable company convention for document identity and reuse existing keys. Explain how to distinguish multiple documents and multiple postings from one document. This is a substantive duplicate-prevention decision, not missing syntax.

### 3. A receipt found after direct expense booking has no completion path
**`skills/capture-receipt/SKILL.md:90–94`**

> “Jos on, kuittia ei kirjata uudelleen, vaan kerro käyttäjälle, että kulu on jo kirjanpidossa ilman kuittia liitteenä.”

Avoiding a duplicate is clear. What completes this item is not. Merely telling the user leaves the newly found source disconnected from its existing voucher. The original direct expense entry might also have omitted or misclassified VAT.

**Clarify:** archive and identify the source against the existing voucher in the available durable record, inspect the existing treatment, and distinguish “evidence found; amounts already correct” from “correction required.” Do not imply that notification alone resolves both cases.

### 4. “No separate document” can mean two different things
**`skills/capture-receipt/SKILL.md:94–96`**

> “Rivi, jolla ei ole erillistä tositetta … kirjataan tuonnissa suoraan kulutilille.”

Does this mean a separate document is **not expected**, or merely **has not been supplied**? The latter conflicts with the instruction to put payments awaiting receipts against `2700`.

**Clarify:** direct expense booking applies when the statement itself is the sufficient source and no separate expense voucher is expected—not simply when the receipt folder contains nothing.

### 5. Unknown bank entries lack a defined stopping boundary
**`skills/capture-receipt/SKILL.md:102–104`**

> “avoin rivi kirjataan sille tilille, jolla se pysyy näkyvissä … ei jätetä pois.”

The purchase-without-receipt example supplies an account, but an unidentified incoming transfer could be a customer payment, loan, owner contribution, or refund. “Where it remains visible” does not determine a defensible account.

SOUL tells me to ask when evidence is insufficient, but this passage appears to require importing every row regardless.

**Clarify:** whether the entire import waits for classification, or whether an explicitly agreed temporary account is permissible. Do not let the all-rows requirement become permission to invent the economic nature of a transaction.

### 6. The VAT-payment instruction excludes its necessary counter-entry literally
**`skills/capture-receipt/SKILL.md:113–114`**

> “ALV-tilille yksin, ilman muita tase-eriä samalla tositteella”

A bank payment necessarily includes another balance-sheet account: the bank. I must infer an unstated exception. A combined tax payment also leaves me choosing between representing the actual components and obeying “VAT alone.”

The handoff already gives a more precise recognition rule: `skills/vero-fi/SKILL.md:165–167` permits VAT accounts, designated settlement accounts, and certain small interest/late-fee entries.

**Clarify:** explicitly allow the bank/settlement counter-entry and defer to that recognition rule. State what to do when a payment contains other components rather than forcing them onto a VAT account.

### 7. Statement validation is temporally ambiguous and rejects a legitimate empty month
**`skills/capture-receipt/SKILL.md:115–119`**

> “Katso, että rivejä tuli ja että ne ovat oikealta tililtä ja kuukaudelta.”

This follows the sealing instructions, and the next paragraph explicitly discusses checking after import. I could interpret it as a post-seal check, when a wrong-account or wrong-period import is already permanent.

“Ensure rows appeared” also gives no distinction between a genuinely inactive account and a parser that missed transactions.

**Clarify:** inspect account, period coverage, and expected transactions from the source and parsed output **before sealing**. Accept zero transactions only when the source independently supports that result.

### 8. Two matching totals do not establish that the month is correct
**`skills/capture-receipt/SKILL.md:142–149`**

> “Kaksi lukua kertoo, onko kuukausi kirjattu oikein”

> “`2700`:n saldo on vielä maksamattomien laskujen summa.”

The text supplies no criterion for identifying offsetting unmatched items.

For example:

- Purchase A’s receipt credits `2700` by €100, but its bank payment is incorrectly booked directly as an expense.
- Purchase B’s €100 bank payment debits `2700`, but its receipt remains unrecorded.

The bank reconciles and `2700` is zero, despite a duplicated expense and a missing receipt. The stated checks cannot distinguish this from correct completion.

**Clarify:** these are necessary summary checks, not sufficient ones. Explain the remaining payable/clearing balance through identifiable invoices and payments, allowing partial or aggregate settlements. This is the highest-value reconciliation correction.

### 9. Outstanding items are both permitted and grounds for rejecting closure
**`skills/capture-receipt/SKILL.md:17–20, 162–171`**

> “jokainen tapahtuma … tai nimetty avoimeksi kodin `TODO.md`-tiedostoon”

> “käyttäjä on päättänyt, mitä `TODO.md`:n avoimille kohdille tehdään”

But also:

> “Hylkäys … on oikea vastaus, kun jotain vielä puuttuu.”

Can I close with a missing receipt, an unsupported EU purchase, or an unexplained reconciliation difference after the user chooses to defer it? Or must any missing item block closure?

The approval requirement answers **who decides**, but not what the proposed disposition must establish.

**Clarify:** distinguish an ordinary outstanding payment, an unresolved bookkeeping discrepancy, and an unresolved tax treatment. State which can remain pending and what must be resolved before offering closure. Otherwise agents will apply materially different closure standards.

### 10. The later-period correction instruction conflicts with the filing handoff
**`skills/capture-receipt/SKILL.md:156–158`**

> “Myöhemmin löytynyt virhe korjataan tositteella seuraavalle avoimelle kuukaudelle.”

Read literally, this includes adding missing VAT metadata through a later-period correction. However, `skills/vero-fi/SKILL.md:157–159` expressly says that VAT corrected manually for the original period must not be tagged again in a later period, because it would be reported twice.

**Clarify:** distinguish correcting the bookkeeping record from correcting the affected VAT return. Route tax-affecting closed-period corrections through the filing guidance before preparing a later-period voucher.

### 11. Monthly bookkeeping is silently treated as monthly VAT filing
**`skills/capture-receipt/SKILL.md:160–161, 175–177`**

> “se siirtää kuukauden heti suljettavaksi”

> “Suljetusta kuukaudesta annetaan ALV-ilmoitus”

The skill does not establish that the company’s VAT reporting period is monthly. Reading company knowledge might reveal a different period, but the instructions give no branch for it.

This matters before closure, not merely during filing: `crates/ledger/src/lib.rs:1864–1884` rejects filing authority when an overlapping period is already non-open or filed. Thus independently sealing months can obstruct a subsequent quarterly filing path.

**Clarify:** either declare this closure-and-filing workflow monthly-VAT-only, or supply the supported path for other reporting periods. Check that prerequisite before sealing a month.

**Overall:** the main problem is not insufficient procedural detail. It is contradictory boundaries: manual versus supported VAT handling, unresolved versus closable work, and bookkeeping months versus filing periods. Resolve those distinctions rather than expanding the skill into a general accounting manual.

## Edge-case lens

Reviewed both files and the initialization skill they reference. **The main gaps are recovery after interrupted writes and distinguishing a genuinely empty month from missing input.** Missing command syntax is not a finding.

References below use `capture` = `skills/capture-receipt/SKILL.md` and `SOUL` = `skills/aggountant-init/SOUL.md`.

| Failure / edge case | Coverage | Evidence and actionable assessment |
|---|---|---|
| **Empty input** | **Addresses weakly** | `capture:115–116` explicitly warns that empty statements can import successfully, but “check that rows arrived” does not distinguish an inactive account from an empty export or missing statement. Add: establish that the statement covers the requested period; zero transactions can be valid, but absent material is not proof of no activity. |
| **Malformed or wrong input** | **Addresses weakly** | CLI help/error handling is covered by `SOUL:85–90`; uncertain receipt treatment is covered by `capture:51–65`. However, account/month checking at `capture:115–116` reads as a check after import, when sealing is irreversible. Explicitly place identity, period and parsed-row checks **before sealing**, using the source statement and parse output. Ordinary schema errors need no additional prose. |
| **Dirty repository state** | **Addresses weakly** | `SOUL:52–55` protects existing TODO content, and `SOUL:66–68` requires related changes to be committed together. Neither explains what happens when unrelated edits are already present. A short shared instruction to preserve existing changes and commit only task-owned changes would address this; a clean-worktree requirement or automatic stash/reset would be excessive. |
| **Missing prerequisites** | **Addresses weakly** | `SOUL:44–62` provides home discovery, required context and initialization routing. `capture:59–65` explicitly handles missing EU-goods accounts. But `capture:80–84` prescribes numeric accounts while `skills/aggountant-init/SKILL.md`, “Yrityksen perustiedot,” permits retaining an existing chart. State that these numbers describe the default chart and require confirmation of the equivalent accounts in the active chart. An existing number with a different meaning is more dangerous than a missing number that fails validation. |
| **Interruption mid-run** | **Addresses weakly** | Attachment lookup and stable receipt keys (`capture:35–38,67–70`), statement deduplication (`106–109`) and period-state inspection (`169–171`) provide useful recovery primitives. There is no explicit recovery instruction for interruption after `run start close-period` has blocked further entries, or after manual `voucher seal` may have succeeded. Add: inspect persisted vouchers and the existing run before retrying; continue or reject that run rather than starting the workflow afresh. Do not infer “nothing happened” from missing output. |
| **Concurrent invocation** | **Ignores** | Preview hashes (`capture:67–68`) detect certain changed proposals; they do not establish general coordination between agents. However, simultaneous writers are outside the repository’s stated v1 scope (`AGENTS.md`, “Single user, v1”). Record this as an unsupported operating condition, **not a reason to add locking or concurrency machinery to this skill**. |
| **Partial completion** | **Addresses weakly** | Unresolved business items are handled well through TODO and the closing decision (`capture:17–20,51–65,160–171`). Technical partial completion is less explicit: some receipts may be sealed while later ones fail, or sealing may succeed before notes/source handling finishes. Add one recovery principle: retain completed vouchers, reconcile remaining work against stored state, and resume only unfinished items. Do not replay an entire manual-voucher batch merely because the batch did not finish. |
| **Resource exhaustion: disk, API limits** | **Ignores** | Generic error interpretation exists (`SOUL:85–90`), but there is no recovery guidance for a failed write or unavailable upstream source. A shared sentence is sufficient: stop affected writes, preserve source material, report incomplete work, and inspect durable state before retrying. Do not prescribe automatic archive deletion or blind retries. Rate-limit handling belongs primarily to whichever source-fetching or filing skill makes the API calls. |
| **Unauthorized or destructive prerequisites** | **Addresses** | Scope of authority and escalation are explicit (`SOUL:3–5,28–31`); irreversible actions require informed approval (`74–78`). Closing’s immediate blocking effect is disclosed (`capture:160–171`), and missing EU-goods support is not bypassed (`59–65`). Read `next_command` guidance (`SOUL:89–90`) under those existing authority limits—not as permission to execute every suggested repair. No duplicated approval section is needed. |

**Recommended priority:** clarify pre-seal statement checks, add one compact interrupted/partial-write recovery paragraph, and qualify the default-chart account numbers. Put generic Git and resource-failure guidance in SOUL rather than expanding every task skill.

These are instruction-coverage findings, not claims that the CLI lacks validation, atomicity or recovery support.
