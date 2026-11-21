# Review: `skills/aggountant-init/SKILL.md`

`/llm-skill-review`, 2026-11-21, routine rigor: one external model (`gpt-6-astra`), lenses executor and design-fidelity (design intent: this issue's "Wanted" and `skills/AGENTS.md`), with `SOUL.md` attached as the file the skill relies on. The file reviewed was the 903-word draft; the fixes the assessment accepted were applied afterwards, so line numbers refer to that draft. The model's output is reproduced unedited.

## Executor lens

I read all three supplied files and checked relevant CLI help definitions and reporting code. These are execution ambiguities in **`skills/aggountant-init/SKILL.md`**, ordered by consequence—not requests to duplicate `--help` or SOUL.

### 1. Retaining the previous chart is offered without an executable path
**Lines 28–30, 65–69**

> “jatketaanko aiempaa tilikarttaa vai siirrytäänkö oletustilikarttaan”

If the user chooses their existing chart, what am I agreeing to preserve: account numbers, classifications, reporting detail, or merely the source-to-target mapping?

This is not just missing command syntax. The inspected CLI exposes only `ChartCmd::List` (`crates/cli/src/main.rs:711–717`), while the skill says `init` installs the default chart and later prohibits manual repairs. I cannot establish how the offered alternative becomes operational.

**Recommendation:** Distinguish retaining a chart already active in an existing home from importing an external chart. Offer the latter only if supported; otherwise describe mapping into the supported chart and identify any lost distinctions before approval.

### 2. The first fiscal year and opening date depend circularly on each other
**Lines 34–38, 54–63**

> “määrittele … se tilikausi, johon ensimmäinen tosite osuu”

> “kun uusi tilikausi on ensimmäinen määritelty eikä ala 1.1.”

For a migration on 1 July, I can follow the ordinary rule and date the opening 30 June, then define the preceding fiscal year. Alternatively, I can define the new year first, making 30 June unavailable, and follow the exception to date the opening 1 July.

Both readings satisfy different parts of the instructions. Because the definition is permanent, this is not harmless discretion.

**Recommendation:** State what determines whether the preceding fiscal year should exist in the home. Choose that boundary before deriving the opening-voucher date.

### 3. A mid-month migration cannot satisfy the stated zero-month-result check
**Lines 47–53, 73–75**

> “Päivä on päättyvän järjestelmän viimeinen päivä”

> “kuukauden tulos on nolla ennen uusia tapahtumia”

Suppose new bookkeeping starts on 15 October. The prescribed opening is dated 14 October and includes accumulated income accounts. Those accounts therefore enter October’s result; moving the opening back one day does not move it outside the month.

The implementation confirms that monthly reports select vouchers by their posting month (`crates/ledger/src/reports.rs:429–444`). I must guess whether mid-month starts are unsupported, whether “ensimmäinen uusi kuukausi” means November, or whether the zero-result criterion should be abandoned.

**Recommendation:** Either explicitly require a month-boundary migration or describe the expected first-month report limitation for a mid-month start. Do not change the economically correct cutoff merely to obtain a zero result.

### 4. Balance-changing discoveries come after opening approval, without a readiness distinction
**Lines 42–45, 78–93, 118–120**

> “näytä käyttäjälle tilit ja päivä ennen sinetöintiä”

> “korot on aiemmassa järjestelmässä usein kirjattu kahteen kertaan tai jaksottamatta”

> “Kirjaa sinne heti se, mikä käyttöönotosta jäi odottamaan.”

Walking through the sections in order, I seal the opening before asking questions that may establish that its balances are wrong. If statements or payable details are missing, the skill does not distinguish an acceptable TODO from an unresolved issue that makes the opening proposal premature.

SOUL supplies approval and uncertainty handling, but not this migration-specific distinction. User approval alone does not establish whether the proposal represents verified balances or a provisional import.

**Recommendation:** Put these inquiries before opening approval and distinguish missing supporting detail from uncertainty affecting opening amounts. State whether provisional openings are intended at all.

### 5. The year-to-date comparison contradicts the year-boundary branch
**Lines 54–56, 73–76**

> “siinä on vain tasetilejä: edellisen tilikauden tulos on omassa pääomassa”

> “tilikauden alusta kertynyt tulos ja tase vastaavat lähderaporttia”

At a fiscal-year boundary, the source report may show the previous year’s nonzero result. The new year contains no imported income accounts under the preceding instruction. Its year-to-date result therefore should not equal that source result.

I have to invent which report, period, and figures “vastaavat lähderaporttia” refers to. The same blanket test also needs qualification when resuming setup after new transactions already exist.

**Recommendation:** Separate the checks: mid-year imported YTD result versus new-year zero result, plus balance-sheet reconciliation after the stated equity transfer. For resumed setup, distinguish opening balances from subsequent activity.

### 6. The cutoff record appears to assign one date to two different boundaries
**Lines 28–29**

> “päivä josta aggountant-kirjanpito alkaa ja johon aiempi järjestelmä päättyy”

Literally this requests one date on which both systems include activity. Later instructions distinguish the old system’s last day from the new system’s first day—and sometimes place the opening voucher on the former.

I must decide whether this record denotes an inclusive transaction boundary, an abstract handover date, or the opening-voucher date. That ambiguity propagates because the skill says all subsequent work reads these records first.

**Recommendation:** Name separately the last transaction date covered by the previous system, the first transaction date handled in aggountant, and the derived opening-voucher date.

### 7. “Matching the total” does not define a useful mapping check
**Lines 65–68**

> “jokainen lähdetili on siinä kerran summineen ja loppusumma täsmää lähderaporttiin”

Which total: signed net balance, debit and credit totals separately, or totals grouped by destination account? A balanced source and a balanced but incorrectly mapped import can both have a net total of zero.

The instruction also leaves unclear how the mapping file demonstrates that its destination totals equal the actual opening voucher—not merely the source report.

**Recommendation:** Require source-account coverage and destination-account reconciliation to the proposed voucher. Specify the totals being compared rather than relying on “loppusumma.”

## Design-fidelity lens

I found **three violations and two plausible intentional refinements**. Paths below are repository-relative; `SKILL.md` means `skills/aggountant-init/SKILL.md`.

## Violations

### 1. A fiscal-year exception was lost, changing the preserved setup knowledge

**Design:** “set-up knowledge as it is, the opening voucher cases made clear” (`issues/skill-init-rebuild/item.md:28–29`).

**Skill:** “Ensimmäistä määriteltyä tilikautta edeltävillä päivillä ei ole tilikautta lainkaan, eikä niille voi kirjata.” (`SKILL.md:37–38`).

This is now unconditional. The previous text explicitly qualified it with **“ellei se ala 1.1.”** That exception still exists in the implementation: dates before the first defined year resolve to calendar years when that definition starts January 1 (`crates/model/src/fiscal_year.rs:221–229`).

Consequently, a first defined fiscal year starting January 1 but ending outside December can incorrectly appear to prohibit the preceding December 31 opening voucher. The third opening-voucher case still correctly includes “eikä ala 1.1.” (`SKILL.md:61`), making the draft internally inconsistent.

**Recommendation:** Restore the January 1 qualification, preferably keeping the explanation beside the opening-date exception rather than stating an unconditional rule earlier.

### 2. Shared house rules remain duplicated—and TODO gains a mandatory structure

**Design:** “`TODO.md`/`AGENTS.md` created here with the rules left to SOUL” (`item.md:29`); “Task skills rely on it and do not repeat it” (`skills/AGENTS.md:39–40`).

**Skill:**
- “`AGENTS.md` vastaa siihen, mitä kuukauden työ muuten kysyisi tai arvaisi joka kerta”, followed by source, archive and Git-content rules (`SKILL.md:101–113`).
- “`TODO.md` saa kolme osiota: mitä muilta odotetaan … mitä kukin avoin kuukausi tarvitsee ennen sulkua, ja lakisääteiset määräajat” (`SKILL.md:118–119`).

SOUL already owns these content categories (`SOUL.md:48–55`). The TODO sentence additionally turns categories into a prescribed three-section layout, which SOUL does not require. This is particularly awkward for an existing, user-maintained TODO with another useful structure.

**Recommendation:** Keep creation, preservation of existing files, and recording setup leftovers here. Refer to SOUL for content categories; do not mandate reorganizing existing TODO files. Distinguish the genuinely setup-specific questions below from these repeated rules.

### 3. `home upgrade` still contains a maintenance procedure, not just its trigger

**Design:** “`home upgrade` is not set-up at all” and “`home upgrade` reduced to when to run it” (`item.md:22,30`).

**Skill:** “aja `aggountant home upgrade --dry-run` ja näytä käyttäjälle, mitä se lisäisi. Jos se ohittaa jotain tai koti ei sen jälkeenkään aukea, kerro se käyttäjälle sellaisenaan. Tiedostoja ei korjata käsin…” (`SKILL.md:131–134`).

The draft retains preview/reporting instructions, failure handling, a repair prohibition and a chart-hash implementation explanation. That exceeds the expressly requested trigger-only treatment.

It also leaves an incomplete miniature procedure: “sen jälkeenkään” follows a dry run, which cannot repair the home. The command’s help already explains preview versus application and predicted opening results (`crates/cli/src/home.rs:14–25`).

**Recommendation:** Reduce this to when to reach for `home upgrade`, relying on help/output. If the manual-repair warning is indispensable knowledge absent from help, retain it explicitly as a justified exception rather than silently preserving the runbook.

## INTENTIONAL DRIFT — plausible refinements needing acknowledgement

### 4. Some retained AGENTS content is setup discovery, not a shared house rule

**Design:** “rules left to SOUL” (`item.md:29`).

**Skill:** “missä kulu- ja matkalaskujen luonnokset ovat” and “missä ajokilometrit kirjataan”, followed by mileage-record details (`SKILL.md:105–108`).

Unlike the duplicated categories above, these are concrete questions needed to establish company practices; SOUL does not provide them.

**Recommendation:** Retain these as setup discovery and acknowledge that distinction. Removing the entire checklist mechanically would discard knowledge the issue otherwise says to preserve.

### 5. The fiscal-year paragraph retains a consequence-oriented explanation

**Design:** “Weight to remove: … `fiscal-year define` paragraphs repeat their `--help`” (`item.md:21–22`).

**Skill:** “määrittele … ennen kuin sinetöit mitään: määrittely on pysyvä” (`SKILL.md:34–37`).

This still overlaps help, but the draft removes syntax and detailed input constraints while preserving **when to act and why order matters**—expressly permitted by `skills/AGENTS.md:19–22`.

**Recommendation:** Accept this narrowed explanation as intentional refinement, correcting finding 1 rather than deleting all fiscal-year guidance.

The approximately 900-word length is reasonably close to “about 800.” I found no separate, supported regression in approval requirements, handling user answers, or cut-over questions: those remain covered by the draft together with SOUL.
