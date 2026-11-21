# Review: `skills/aggountant-init/SOUL.md`

`/llm-skill-review`, 2026-11-21, routine rigor: one external model (`gpt-6-astra`), lenses executor and design-fidelity (design intent: this issue's "Wanted" and `skills/AGENTS.md`). The file reviewed was the 672-word draft; the fixes the assessment accepted were applied afterwards. The model's output is reproduced unedited.

## Executor lens

I read `SOUL.md` and the setup, receipt-capture, and filing skills to avoid flagging details already supplied by task instructions. Four ambiguities remain, ordered by consequence. All references below are to `skills/aggountant-init/SOUL.md`.

#### 1. What exactly am I asking the user to approve when closing a period?
**Lines 73–79:** “Suljettu kausi lukitsee kirjauksensa.” / “näytä ehdotus sellaisena kuin se tallentuu tai lähtee.”

For a voucher or filing, the task skills identify the proposed content to display. For closure, I must choose that content myself: the period identifier, financial statements, included vouchers, unresolved items, or some combination.

This is consequential because the user approves a lock on existing bookkeeping, not merely a new record. Showing “close October” and showing October’s balances and outstanding issues are materially different approvals. The inspected task skills do not supply an equivalent closure presentation.

**Clarify:** Identify where the closure-specific approval content is defined. This belongs in the relevant task instruction rather than a detailed closure procedure in SOUL.

#### 2. Does the program override only recorded status, or also an unresolved business concern?
**Lines 52–60:** “mitä kukin avoin kuukausi tarvitsee ennen sulkua” / “ristiriidassa ohjelma on oikeassa” / “ohjelman oma lista velvoitteista, jotka estävät kauden sulun.”

Suppose TODO says a month still needs a receipt, but the program reports that month closed. I know the recorded state is closed; I do **not** know that the missing-receipt concern was resolved. The instruction does not distinguish correcting the stale status description from deleting the unresolved concern.

Similarly, an empty program obligation list does not establish that TODO’s human-recorded prerequisites have been satisfied. The setup skill explicitly says receipt-related outstanding work is not represented by those obligations.

**Clarify:** The program is authoritative about recorded state, not documentary completeness. An unresolved TODO concern survives a state correction and must be reconciled rather than discarded.

#### 3. Must I read the company’s Git rules before changing or committing its records?
**Lines 47–50:** “mikä kodista kuuluu gitiin” / “Lue se ennen kuin haet tai siirrät aineistoa”

**Lines 67–69:** “Kun koti on git-repositorio, commitoi näiden tiedostojen muutokset samalla kertaa kuin työ, josta ne syntyivät”

The required reading point is before retrieving or moving source material. But a task can update company knowledge, resolve a TODO item, or complete a filing without retrieving or moving any material. It then reaches the commit instruction without necessarily having read the company-specific version-control boundary.

I must infer whether reading `AGENTS.md` is actually mandatory at session start, only before material handling, or additionally before committing.

**Clarify:** Require reading the applicable house rules before the first affected action, including record changes and Git operations—not solely source-material handling.

#### 4. When does my interpretation become a reusable company practice?
**Lines 17–22:** “Erotat lähteistä todetut asiat, käyttäjän kertomat tiedot ja omat tulkintasi.” / “Säilytä yrityksen tietovarannossa työssä saatu fakta tai käytäntö”

The text distinguishes evidence, user statements, and interpretations, but does not explain which interpretations qualify for promotion into persistent company knowledge.

For example, several historical vouchers suggest a recurring treatment. May I save that as the company’s practice, save only the observation that those vouchers used it, or seek confirmation first? Recording “practice X” with historical vouchers as its source can make an inference look like an agreed policy to the next agent.

The setup skill mentions *confirmed interpretations*, but SOUL does not establish that distinction for other tasks.

**Clarify:** Distinguish an observed historical pattern from an established practice when saving it. This need not require approval for every factual memory; it requires preserving the interpretation’s status, not merely its source.

## Design-fidelity lens

The rewrite largely follows the design: the role is unchanged, the command catalogue is removed, and every explicitly requested topic is present. I found **two wording-level scope drifts and two reasonable extensions to acknowledge**—not a structural mismatch.

Paths below are repository-relative; **SOUL** means `skills/aggountant-init/SOUL.md`.

#### 1. Uncertain submission becomes a stop-everything instruction

**Design:** `skills/AGENTS.md:30–32` asks for the principle “**a filing is never resent on a guess**.” The issue also preserves “**the role as it is**” (`issues/skill-soul-rebuild/item.md:28`), including continuing other work where possible.

**SOUL, lines 81–82:**
> “Jos et tiedä, menikö lähetys perille, selvitä se ohjelmalta **ennen muuta**.”

“Before anything else” is broader than “before resubmitting.” Read literally, it suspends unrelated bookkeeping until submission uncertainty is resolved, whereas the preserved role says:

> “viet muuta työtä eteenpäin mahdollisuuksien mukaan.” — lines 30–31

**Recommendation:** Scope the restriction to another submission: establish the outcome before sending again. Keep unrelated work possible. This is an instruction ambiguity, not evidence that an agent has actually stalled.

#### 2. Four categories of shared working knowledge become an exhaustive storage inventory

**Design:** `issues/skill-soul-rebuild/item.md:28–31` specifies:
> “the house rules once (`AGENTS.md` routes and practices, `TODO.md` pending work, `knowledge company` sourced facts, `obligation` the program's own list …)”

**SOUL, lines 44–45:**
> “Istunnosta toiseen säilyvällä tiedolla on siinä **neljä paikkaa**”

This says information persisting between sessions has four locations, not merely that these four locations have distinct responsibilities. That overstates the design and conflicts with the preserved role’s other durable destinations:

> “Yksittäisen tapahtuman perustelut kuuluvat tositteelle ja alkuperäinen aineisto lähdearkistoon.” — lines 22–23

**Recommendation:** Introduce the list as the division of responsibility for shared instructions, pending work, company knowledge and program obligations—not as all persistent storage. A short wording change suffices.

#### 3. INTENTIONAL DRIFT: secret-storage rules acquire a custody rule

**Design:** `issues/skill-soul-rebuild/item.md:30`:
> “secrets in none of them”

**SOUL, lines 65–67:**
> “eivät kuulu mihinkään näistä **eivätkä keskusteluun**”  
> “**Käyttäjä säilyttää ne itse kodin ulkopuolella.**”

Keeping secrets out of conversation is a sensible extension, with an explicit reason: conversation logs persist. Assigning storage exclusively to the user is a further policy decision; prohibiting these four destinations does not itself prohibit an authorized agent from configuring an external credential store.

**Recommendation:** Retain the conversation restriction. Acknowledge whether user-only custody is intended; if the intended boundary concerns storage location rather than who operates it, remove “itse.” I am not asserting that an existing credential workflow is broken.

#### 4. INTENTIONAL DRIFT: the length target was relaxed materially

**Design:** `issues/skill-soul-rebuild/item.md:27–28`:
> “at about **500 words**: the role as it is”

**SOUL:** The complete file is **672 whitespace-delimited words**. There is no individual conflicting phrase here; the discrepancy is its aggregate length.

With the required 279-word role retained, approximately 393 words remain, versus approximately 221 under the original target. That is a meaningful departure, but not a reason to damage the preserved role or remove required subjects.

**Recommendation:** Explicitly accept roughly 670 words, or tighten only the new sections. Treat this as a sizing reconciliation, not a correctness defect.

#### What I would not flag

- `--help`, `--json`, `error_code`, `message`, and `next_command` are explicitly requested; mentioning them does not violate “the program describes itself.”
- Prior approval for sealing, closing and filing—and showing changed proposals again—implements the specified approval boundary.
- The uncertain-submission principle belongs here; only “before anything else” over-expands it.
- Linking to initialization rather than embedding setup instructions respects the scope.
- Copies remaining in task skills are explicitly deferred to their own rebuild slices.
