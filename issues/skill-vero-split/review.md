# Review: skill-vero-split

`/llm-skill-review`, routine rigor, one model (`gpt-6-astra`), 2026-11-21. Target: the six files of
`skills/vero-fi/` and `skills/tulorekisteri/` as first written (before the fixes in
`assessment.md`). Lenses: executor on each skill, cross-skill over both with
`skills/AGENTS.md`, `SOUL.md` and the `e-services` files attached. The reviewer was told
that flags, formats, states and codes are deliberately left to `--help` and the output.
Raw model output follows, unedited.

## Executor lens: vero-fi


I found four material ambiguities. I read all eight files and checked the relevant CLI/process code and linked Tulorekisteri instructions. The binary was not on `PATH`, so command behavior below is source-verified, not execution-tested.

### 1. An empty OmaVero view is both insufficient and sufficient evidence for manual refiling
**High — `skills/vero-fi/hairiot.md:37–40`**

> “Näyttö on esimerkiksi Verohallinnon vastaus tai OmaVeron näkymä, josta käy ilmi, ettei aiempaa ilmoitusta ole.”

But `skills/vero-fi/SKILL.md:75–76` says:

> “tyhjä OmaVero ei todista, ettei ilmoitusta ole.”

**Executor ambiguity:** After a timeout, I see no return in OmaVero. Does that screenshot satisfy the manual-filing exception, or must I continue waiting? The incident reference supplies no criterion distinguishing an authoritative absence determination from the delayed empty view explicitly rejected by the main skill.

**Risk:** The agent could help prepare a second filing and then treat a screenshot as sufficient resolution evidence. The program does not adjudicate that evidence: `crates/process/src/file_vat.rs:243–271` checks that a reference is supplied, and `:1169–1183` validates its syntax—not its meaning.

**Recommendation:** Remove the unqualified empty-view example. State that delayed nonvisibility is not resolution evidence; manual fallback requires an actual resolution of the previous attempt, not merely an archived observation of absence.

### 2. The corrected-manual branch points back to instructions that restore the known-wrong figures
**High — `skills/vero-fi/SKILL.md:35–38,93–99`**

> “käyttäjä antaa kauden ilmoituksen OmaVerossa korjatuin luvuin (alla)”

The referenced section then says:

> “esikatselun numeroidut kentät ovat ilmoitettavat luvut.”

**Executor ambiguity:** When preview has omitted VAT, should I fill the form with preview values, or with a separately corrected set? Who establishes that corrected set, and which numbers should the user approve?

**Risk:** Following the linked manual procedure literally submits the very underreported amounts that triggered the exception. SOUL’s approval rule does not settle which proposal is correct.

**Recommendation:** Qualify the manual section: preview supplies filing values only when no unresolved content discrepancy exists. For this exception, establish and show the complete corrected field set before form filling, with the differences from preview explained and approved. Preserve that actual submitted set with the filing evidence; do not imply that the unchanged preview represents it.

### 3. “Do not bypass a blocker” conflicts with the instructed manual fallback
**Medium — `skills/e-services/SKILL.md:22–27`; `skills/vero-fi/kayttoonotto.md:4–5`**

> “kun ohjelma estää lähetyksen … selaimella ei tehdä samaa ilmoitusta sen ohi.”

Yet setup says:

> “Siihen asti ilmoitus annetaan käsin”

And the VAT skill explicitly directs a manually corrected filing when VAT is missing from the sealed period.

**Executor ambiguity:** Does “ohjelma estää lähetyksen” include unavailable credentials, an incomplete setup, and reconciliation failures—or only blockers that make filing itself unsafe? An explicit request to fill a form does not clearly resolve the contradiction, because the browser prohibition is categorical.

**Risk:** Different agents will either refuse a legitimate manual route or infer an excessively broad exception permitting bypass of unresolved-delivery or approval safeguards.

**Recommendation:** Distinguish **an unavailable automated channel** from **an unresolved filing or content/approval problem**. Manual filing may replace the former; it must not evade the latter. Link the corrected-figures exception to its required resolution rather than making agents invent the distinction.

### 4. Browser corrections are promised a recording path that rejects already-filed VAT periods
**Medium — `skills/e-services/SKILL.md:26–30`; `skills/vero-fi/SKILL.md:19–21`**

> “käsin annettu ilmoitus tai korjaus, jota aggountant ei tuota”

followed by:

> “Tulos kirjataan aggountantin manuaalisen ilmoituksen polulla (`complete-manual`)”

**Executor ambiguity:** After the user corrects an already-filed VAT return in OmaVero, how do I record that correction and keep later agents from treating the original filing as current?

**Observed implementation:** `complete_manual_vat` acquires filing authority (`crates/process/src/file_vat.rs:265`). `acquire_filing_authority` rejects an already-filed period (`crates/ledger/src/lib.rs:1859–1896`). Thus this is not merely an omitted flag discoverable through help: the stated recording route does not support this case.

**Risk:** The correction is already external and irreversible when recording fails; the local record still describes the original filing.

**Recommendation:** Separate initial manual completion from subsequent corrections. Until a correction-recording operation exists, explicitly require archiving the correction evidence and recording its relationship to the original filing in the company’s continuity records; do not claim `complete-manual` records it.

**Otherwise:** The approval boundary, browser submit boundary, normal uncertain-send reconciliation, and certificate activation approvals are clear. The linked Tulorekisteri catch-up instructions supply the report-630 parameters and company-confirmation requirement, so their absence from the browser reference is not a finding.

## Cross-skill lens: vero-fi and tulorekisteri


## Findings

I read all supplied files, plus the linked `skills/e-services/selain.md`, `skills/aggountant-init/SKILL.md`, and relevant CLI/process implementations. No files were changed. This is a contract review, not independent verification of Finnish tax rules.

### 1. High: the incident reference permits evidence that the main skill explicitly says is insufficient

**Locations:** `skills/vero-fi/hairiot.md:37–40`; `skills/vero-fi/SKILL.md:73–82,99–101`.

The reference allows manual filing:

> “kun suoran lähetyksen lopputulos on yhä auki”

and gives this example of resolution evidence:

> “OmaVeron näkymä, josta käy ilmi, ettei aiempaa ilmoitusta ole.”

But the body explicitly says:

> “tyhjä OmaVero ei todista, ettei ilmoitusta ole”

and prohibits manual filing until the previous outcome is resolved.

**Failure:** after a timeout, an agent could treat an empty OmaVero view as sufficient evidence, assist with another filing, and record it using `--remote-status-evidence-ref`. The program does not validate the evidence’s substance: `crates/process/src/file_vat.rs:243–263` checks the reference and its presence.

**Recommendation:** distinguish an *unresolved local run* from an *unresolved authority outcome*. Permit recording a manual filing only after the latter is definitively resolved. Explicitly exclude an empty OmaVero view or a single “not found” response as proof of non-delivery.

### 2. High: `complete-manual` is presented as recording corrections, but can silently preserve the original filing

**Locations:** `skills/e-services/SKILL.md:26–30`; `skills/tulorekisteri/SKILL.md:83–86`.

The browser skill includes a correction:

> “korjaus, jota aggountant ei tuota”

then promises:

> “Tulos kirjataan … (`complete-manual`), jotta kirjanpito tietää, mitä viranomaiselle on annettu.”

The Tulorekisteri body similarly routes corrected travel reports to the service and then instructs recording their references with `complete-manual`.

**Actual implementation:**

- `crates/process/src/file_wage.rs:403–411`: if the run already has a successful/reconciled effect, manual completion returns the existing run **without recording the supplied correction references**.
- `crates/ledger/src/lib.rs:2016–2023`: VAT manual completion after an earlier manual filing can return the existing filing unchanged; after an API filing, it rejects the different filing hash.
- `crates/process/src/file_employer_report.rs:847–852`: an already accepted employer report rejects manual completion.

**Failure:** a browser correction can succeed while the agent receives an apparently successful command result retaining the original evidence. The advertised “bookkeeping knows what was filed” postcondition is false.

**Recommendation:** separate initial/manual completion from recording a later correction. Until a supported correction-recording operation exists, require preserving the correction receipt and explicitly reporting that aggountant still represents the original filing. Do not describe `complete-manual` as a universal correction recorder.

### 3. Medium: importing a report reference does not establish the prerequisites for a safe replacement

**Locations:** `skills/tulorekisteri/kiinnikurominen.md:46–52`; `skills/tulorekisteri/SKILL.md:69–70`.

The catch-up reference says:

> “se kirjataan … `complete-manual`, ja sen jälkeen korvaava onnistuu.”

The body says:

> “Eläkevakuutustietoja sillä ei voi muuttaa”

However, manual completion records references/version, **not the previous report’s contents or pension-insurance details**. External reports have `report: None` (`crates/process/src/file_employer_report.rs:305–314`). Replacement planning then falls back to current profile insurance data, and the comparison against previous insurance details runs only when previous report content exists (`:523–564`).

**Risk:** the suggested handoff lacks the information needed to preserve the original insurance details. It can construct a replacement violating the body’s own rule. Whether the authority rejects that replacement was not established here.

**Recommendation:** remove the unconditional promise. For externally filed reports, require verifying the previous report’s complete relevant contents; otherwise keep the correction in the service. A stronger implementation would refuse API replacement when the required previous content is unknown.

## Other explicit contract mismatches

| Location | Mismatch | Recommendation |
|---|---|---|
| `skills/e-services/SKILL.md:46`; `skills/e-services/tulorekisteri.md:14–15` | “`tulorekisteri` kertoo [rajaukset]” points to another skill, but its body delegates those parameters to `kiinnikurominen.md`. Also, installing only `e-services` does not install `tulorekisteri`: `crates/cli/src/skill.rs:83–86` requires only `aggountant-init`. | Link directly to the parameter-owning reference and guarantee availability through dependencies, or explicitly name its `skill print` retrieval. The binary can retrieve it, so this is a handoff/packaging defect—not proof that the information is unavailable. |
| `skills/e-services/SKILL.md:32` versus `skills/tulorekisteri/kiinnikurominen.md:48–50` | “Älä poimi … **tunnisteita** … sivulta” literally prohibits extracting identifiers, while catch-up requires obtaining the report identifier from the service. | Narrow the prohibition to authentication/session identifiers; distinguish permitted filing receipt identifiers. |
| `skills/tulorekisteri/kiinnikurominen.md:20–21` versus `:48–55` | “**ainoa vaihe**, jossa käyttäjän pitää kirjautua sinne” contradicts the same reference’s later requirements to retrieve report references and inspect conflicting months. | Say this is the normal catch-up flow’s browser step; corrections and conflicts may require another visit. This is an expectation mismatch, not a filing-safety defect. |

## Explicit reference coverage

| Referring files | Referenced contracts inspected |
|---|---|
| All three `SKILL.md` bodies | `aggountant-init/SOUL.md`: company context, exact-proposal approval, immutable actions, uncertain-outcome handling, CLI discovery. |
| `SOUL.md` | `aggountant-init/SKILL.md`: create or supplement missing company instructions and pending-work notes; preserve existing setup. |
| `e-services/SKILL.md` | `omavero.md`, `tulorekisteri.md`, `selain.md`; also the named `tulorekisteri` task skill. |
| All three e-services references | Their parent `SKILL.md`: observations confer no authority; browser submission remains the user’s action. |
| `vero-fi/SKILL.md` | `hairiot.md`, `kayttoonotto.md`, `e-services/SKILL.md`, `e-services/omavero.md`, `tulorekisteri/SKILL.md`. |
| VAT references | Their parent `SKILL.md`, including its no-resubmission and approval rules. |
| `tulorekisteri/SKILL.md` | `kayttoonotto.md`, `kiinnikurominen.md`, `e-services/SKILL.md`. |
| Tulorekisteri references | Their parent skill, each other, `vero-fi/kayttoonotto.md` for credentials, and `e-services/tulorekisteri.md` for report retrieval. |

The company-local `AGENTS.md`, `TODO.md`, credential files and `.aggountant/tulorekisteri.json` are runtime artifacts, not additional bundled instruction contracts. I found no further material mismatch in the shared approval rules, credential handoff, or browser submit/confirmation boundary.

## Executor lens: tulorekisteri


I read all nine files and inspected the relevant CLI help definitions and implementations. The binary was not on `PATH`, so this is a static review, not an execution test. I treated SOUL’s approval rules and the program’s discoverable syntax as supplied safeguards.

## Material execution ambiguities

### 1. An external report reference is presented as sufficient for a safe replacement

**`skills/tulorekisteri/kiinnikurominen.md:46–52`**

> “se kirjataan komennolla `aggountant file-wage employer-report complete-manual`, ja sen jälkeen korvaava onnistuu.”

**Unanswered question:** Does the existing report contain information that the replacement must preserve but aggountant cannot reconstruct?

This is a concrete implementation gap. External completion stores a receipt, not the original report content (`crates/process/src/file_employer_report.rs:314, 863`). Replacement planning consequently falls back to the current profile’s insurance information and skips comparisons against the original report (`:523–568`).

Obtaining a reference enables addressing a replacement; it does not establish that the proposed replacement preserves the right information. The user could approve the visible correction without realizing that other fields differ.

**Recommendation:** Make this alternative conditional on checking the original report’s complete contents against what aggountant can reproduce. Otherwise route the correction through the e-service. Do not promise that recording the reference alone makes the replacement safe.

### 2. Historical months silently inherit today’s pension-insurance settings

**`skills/tulorekisteri/kayttoonotto.md:64–67`**

> “Yhteyshenkilö ja eläkevakuutukset ovat käyttäjän antamia tietoja.”

**`skills/tulorekisteri/kiinnikurominen.md:43–44`**

> “Käy ne läpi vanhimmasta alkaen, kukin omalla esikatselullaan ja hyväksynnällään”

**Unanswered question:** Are the supplied insurance policies—and the assertion that no insurance obligation exists—valid for each historical month, or only now?

The implementation uses one profile’s insurance list when constructing historical reports; it has no month-specific selection (`crates/process/src/file_employer_report.rs:525–542`). Thus, a company now employing only a YEL-insured owner can produce historical “Ei palkanmaksua” reports using an empty list even if its historical situation differed.

**Recommendation:** At catch-up, explicitly establish the insurance information applicable to the historical months. If it differs from the profile and cannot be represented safely, use the e-service rather than treating current settings as historical evidence.

### 3. The occasional-employer branch stops before explaining how to fulfil an actual obligation

**`skills/tulorekisteri/kayttoonotto.md:57–59`**

> “Satunnaiselle työnantajalle lohkoa ei tehdä.”

**`skills/tulorekisteri/SKILL.md:54–55`**

> “Satunnainen työnantaja, joka ei ole rekisterissä, ei anna ilmoitusta kuukausilta, joilta se ei maksanut mitään.”

**Unanswered question:** What should I do when that employer *has* paid reportable travel reimbursements?

Official guidance requires an occasional employer’s separate report for applicable payment months, including reportable reimbursements. ([vero.fi](https://www.vero.fi/tulorekisteri/yritykset-ja-organisaatiot/yksityiskohtaiset-tulorekisterin-ohjeet/63595/tietojen-ilmoittaminen-tulorekisteriin-ty%C3%B6nantajan-erillisilmoitus9/)) But the subsequent skill workflow uses commands requiring the omitted `employer_report` profile (`crates/process/src/file_employer_report.rs:264–275`).

The CLI failure is loud if attempted; the dangerous interpretation is that no profile means no separate-report work.

**Recommendation:** State the positive branch explicitly: these payment months still require reporting, but this configuration does not support the automated monthly workflow; the user files through the e-service. Do not invent a registration date to enable the command.

### 4. A successful import is treated as proof that remaining months are genuinely unreported

**`skills/tulorekisteri/kiinnikurominen.md:43`**

> “Tuonnin jälkeen `missing` näyttää todella puuttuvat kuukaudet.”

**`skills/e-services/tulorekisteri.md:26–27`**

> “Valmis raportti ladataan Tehdyt tilaukset -sivulta zip-pakettina.”

**Unanswered question:** Which completed report is sufficiently current to establish absence?

The instructions correctly specify the company confirmation, language and reception-time filter. They do not establish that a supplied or downloaded report reflects the latest external filings. An older, otherwise valid report can omit a subsequently filed month.

The importer exposes coverage and import time (`crates/process/src/employer_report_history.rs:454–478`); `missing` lists locally unaccounted-for months, not independently verified absences at the authority (`crates/process/src/file_employer_report.rs:889–927`).

**Recommendation:** Qualify the claim: remaining months are candidates until the imported reports cover the intended history and are known to include subsequent external filings. Match the downloaded job to the requested report; refresh or investigate when snapshot currency is uncertain.

### 5. “The register’s month” is ambiguous after deregistration and re-registration

**`skills/tulorekisteri/kayttoonotto.md:60–61`**

> “Ehdota silloin rekisterin kuukautta”

**`skills/tulorekisteri/kiinnikurominen.md:25`**

> “kohdekausi rekisteröinnin koko ajalta”

**Unanswered question:** Which registration period governs catch-up when the company has several periods or registration has ended?

The registration command’s suggested start derives from the **current** registration period (`crates/process/src/employer_report_history.rs:197–208`). Meanwhile, `missing` walks continuously from one configured start month (`crates/process/src/file_employer_report.rs:899–919`).

Choosing the current start can hide older outstanding months; choosing the earliest start can introduce months in registration gaps. User approval of a date does not explain this representational limitation.

**Recommendation:** Identify interrupted or ended registration as a separate case. Do not describe the single-start monthly list as the complete historical obligation list; reconcile those periods explicitly before filing.

### 6. Manual correction recording can report success without recording the correction

**`skills/tulorekisteri/SKILL.md:83–86`**

> “Hylätty tai korjattava matkakorvausilmoitus hoidetaan asiointipalvelussa”  
> “Asiointipalvelussa annettu ilmoitus kirjataan viitteineen `complete-manual`-komennolla.”

**`skills/e-services/SKILL.md:26–30`** makes the same general promise for manual corrections.

**Unanswered question:** How do I record a correction to an already accepted filing, rather than merely complete its original submission?

For travel reports, `complete_wage_manual` returns the existing successful run without recording the supplied correction references (`crates/process/src/file_wage.rs:405–412`). For employer reports, an already accepted month is refused (`crates/process/src/file_employer_report.rs:848–853`).

The travel case is particularly dangerous: successful command completion can be mistaken for successful correction recording, leaving stale local evidence for later decisions.

**Recommendation:** Distinguish initial manual completion from post-acceptance correction. Until correction recording is supported, archive the correction evidence and explicitly record the local-state limitation; do not claim `complete-manual` updated it.

### 7. Travel-payment identity does not establish whether another system already reported the payment

**`skills/tulorekisteri/SKILL.md:36–38`**

> “`source_reference` yksilöi maksun pysyvästi.”

**Unanswered question:** Before creating the first local report for an older expense payment, how do I establish whether the previous payroll provider or the user already reported it?

The skill anticipates this history problem for employer reports, but not for travel reports. The local identity is derived from the supplied reference, employee and payment date (`crates/domain/src/wage_report.rs:119–137`). It cannot establish the absence of an external report using another reference.

**Recommendation:** For historical travel payments or a reporting-system handover, require evidence or user confirmation of prior reporting status before creating a new report. Reuse an existing report’s correction route when one exists; do not treat a new local reference as proof of a new reporting obligation.

## Additional factual defect

### 8. The travel-reimbursement deadline is incorrect

**`skills/tulorekisteri/SKILL.md:28`**

> “Määräpäivä on viisi päivää maksupäivästä.”

For the tax-free reimbursements this skill handles, official guidance specifies the **fifth day of the following month**, not five days after payment. ([vero.fi](https://vero.fi/tulorekisteri/yritykset-ja-organisaatiot/suorituksen-maksajat/palkkatiedot/kustannusten-korvaukset/))

This is not an ambiguity and normally causes premature urgency rather than a wrong filing. However, consulting the program will not resolve it: the same rule appears in `crates/cli/src/input_help.rs:449–450` and `crates/domain/src/wage_report.rs:115–117`.

**Recommendation:** Correct the domain calculation, help and skill together.

The highest-priority changes are **external replacement eligibility, historical insurance applicability, and honest recording of manual corrections**. Those are places where the current instructions can appear fully followed while producing an incorrect submission or misleading evidence of what was filed.
