# Assessment: `skills/capture-receipt/SKILL.md` review

Source: `issues/skill-month-rebuild/review.md` (`/llm-skill-review`, one model, `gpt-6-astra`). Assessed with `/assess-findings` at `072cab6` plus the uncommitted draft. Every row is a claim about the instruction text; evidence is from the draft, the other skills and the binary built from this worktree, run in a synthetic company home.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Manual VAT legs: forbidden early, explained later | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX |
| F2 | No convention for `natural_key` | CONFIRMED | RARE | WORSENS | NONE | LOW | DROP (Rule 1b: rare, no readability gain) |
| F3 | Receipt found after its bank line was booked as expense has no completion path | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F4 | 'No separate voucher' reads as 'no receipt supplied' | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX |
| F5 | Unknown bank line: 'every line needs an assignment' invites inventing its nature | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F6 | VAT payment 'alone, without other balance-sheet items' excludes the bank leg | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX |
| F7 | Statement checks read as post-seal; a truly empty month is rejected | CONFIRMED | REGULAR | IMPROVES | NONE | HIGH | FIX |
| F8 | Two balances are necessary, not sufficient | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F9 | Open items both allowed at close and a reason to reject | CONFIRMED | REGULAR | NEUTRAL | NONE | MED | FIX |
| F10 | Later-month correction vs a VAT error of a closed month | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F11 | Month close assumes a monthly VAT period | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F12 | Dirty repository state | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b: rare, no readability gain) |
| F13 | Account numbers are the default chart's | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F14 | Interrupted close has no recovery hint | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F15 | Concurrent invocation | CONFIRMED | COSMIC_RAY | WORSENS | NONE | HIGH | DROP (Rule 1b: rare, no readability gain) |
| F16 | Technical partial completion of a batch | CONFIRMED | OCCASIONAL | WORSENS | NONE | MED | DROP (Rule 1d: worsens readability, not regular) |
| F17 | Resource exhaustion | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b: rare, no readability gain) |

FIX: 12   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 5

## Evidence and sources

- **F1** (Executor lens #1). The draft says VAT legs built with `voucher draft` do not carry the return's data, then explains how a manual voucher's VAT reaches the return through `vat_code`; an agent cannot tell whether a tagged manual voucher is an allowed alternative when `record` refuses. *Evidence:* Both sentences are in the draft (Kuitit ja ostolaskut; Erikoiskirjaukset). Ran `voucher validate` with `purchase-eu-goods`: refused with VAT_CODE, so the manual route is not a general fallback. *Fix:* Scope the first sentence to an ordinary purchase and say in the special-entries section that it is not a route for a purchase `record` refuses.
- **F2** (Executor lens #2). `natural_key` is 'derived from the source' with no convention for documents sharing a date or the same invoice arriving as another file. *Evidence:* True that no convention is given. `capture-receipt record --help` already says the key is stable and source-derived; the duplicate check the skill relies on is the attachment lookup and the payment-date check. A fixed convention would be company practice (it belongs in `knowledge company`), and the text would grow for an unusual combination.
- **F3** (Executor lens #3). The draft only says to tell the user; the found receipt stays disconnected and the direct booking's VAT may be wrong. *Evidence:* Draft: 'kuittia ei kirjata uudelleen, vaan kerro käyttäjälle'. A sealed voucher cannot take an attachment afterwards (no such command in `voucher --help`), so archive plus compare is what remains. *Fix:* Say to archive the receipt, compare the sealed voucher's treatment with it, and correct with a new voucher if they differ.
- **F4** (Executor lens #4). 'Rivi, jolla ei ole erillistä tositetta' can mean not expected or not yet supplied; the latter contradicts booking an unreceipted payment to 2700. *Evidence:* Quoted phrase is in the Vastatili section; the Tiliote section tells the opposite for a payment still waiting for its receipt. *Fix:* Reword to a line whose only source is the statement itself.
- **F5** (Executor lens #5). An unidentified incoming transfer has no defensible account, yet the draft says every line is assigned and none left out. *Evidence:* Ran `import camt053 seal` with one assignment missing: MISSING_ASSIGNMENT refuses the whole statement, so the only choices are to wait or to guess; the draft named only the payment-without-receipt case. *Fix:* Say that a line whose nature is unknown is asked about before the import; the import waits.
- **F6** (Executor lens #6). A VAT payment necessarily has the bank account as its other leg; the sentence literally forbids it. *Evidence:* Draft sentence quoted. `crates/domain/src/vat/reconcile.rs` treats a reduction as a settlement when the other postings are VAT-role or settlement-role (bank, tax) accounts or small interest lines; `vero-fi` states the same. *Fix:* Say the other leg is only the bank account, and point to `vero-fi` for the exact rule.
- **F7** (Executor lens #7; Edge-case lens rows 'Empty input' and 'Malformed or wrong input'). 'Katso, että rivejä tuli…' follows the sealing text, so the account and month check can be read as happening after the permanent import; zero rows may be legitimate. *Evidence:* Ran the sample statement for March into a home working on July: imported without complaint. `parse` output carries `account_iban` and booking dates, so the check is possible before `seal`. *Fix:* Move the check to the `parse` output before sealing and say what zero rows means.
- **F8** (Executor lens #8). 'Kaksi lukua kertoo, onko kuukausi kirjattu oikein': a double-booked expense and a missing receipt of the same amount cancel on 2700. *Evidence:* The offsetting example is arithmetically right; the guide's `reference/status.md` confirms the program does not match a receipt to its payment line. *Fix:* Call them the two checks that catch the usual mistakes and say the payable balance is explained invoice by invoice.
- **F9** (Executor lens #9). The draft lets open items stay in TODO.md but says rejecting is right 'when something is still missing'. *Evidence:* Both phrases are in the draft's Sulku section and opening paragraph. *Fix:* Say who decides and what the user is told: an open item stays out of this month's figures and return; reject is for when the user wants to enter something in this month.
- **F10** (Executor lens #10). 'Myöhemmin löytynyt virhe korjataan tositteella seuraavalle avoimelle kuukaudelle' would, read literally, re-tag VAT that `vero-fi` says must be corrected in OmaVero and not tagged again. *Evidence:* `skills/vero-fi/SKILL.md`: 'käyttäjä ilmoittaa kauden OmaVerossa korjatuin luvuin, eikä samaa veroa kirjata enää merkinnällä myöhemmälle kaudelle, koska se ilmoitettaisiin kahdesti'. *Fix:* Add that an error touching the closed month's VAT is handled as `vero-fi` says before a correcting voucher.
- **F11** (Executor lens #11). The skill closes months without checking the company's VAT period; closing a month blocks quarter closes for the year. *Evidence:* Ran in a synthetic home: after sealing 2026-03, `run start close-period --period 2026-Q2` refuses with PERIOD_TRANSITION_CONFLICT 'calendar year 2026 cannot mix active month and quarter coverage'. *Fix:* Say the period closed is the VAT period and that months and quarters cannot be mixed within a year.
- **F12** (Edge-case lens row 'Dirty repository state'). Neither file says what to do with unrelated uncommitted edits in the company home. *Evidence:* Not this file's subject: SOUL owns the git house rule, and committing only one's own changes is what a capable agent does without being told.
- **F13** (Edge-case lens row 'Missing prerequisites'). The contra-account table gives 2700, 2740, 1900, 1910 as fixed numbers, but a company may keep its own chart where a number means something else. *Evidence:* `skills/aggountant-init/SKILL.md` lets a company keep an existing chart. A number that exists with another meaning validates and posts silently. *Fix:* One clause: the numbers are the default chart's; use the active chart's equivalents.
- **F14** (Edge-case lens row 'Interruption mid-run'). After `run start close-period` the month is locked; a new session finding it in closing is not told the run still waits. *Evidence:* Ran: a second `run start close-period` on a closing month fails with PERIOD_TRANSITION_CONFLICT and no `next_command`; `period show` returns `owning_run_id`. *Fix:* Say that `period show` names the run a closing month waits on and that it is answered, not restarted.
- **F15** (Edge-case lens row 'Concurrent invocation'). No coordination between simultaneous agents. *Evidence:* aggountant v1 is one user or agent on one machine (`crates/AGENTS.md`). No concrete defect is named, so nothing goes to the multi-user register either.
- **F16** (Edge-case lens row 'Partial completion'). Some vouchers may be sealed when a later one fails; the skill gives no resume principle. *Evidence:* The program already makes a retry safe and says so in its help and output: the same `natural_key` and content replays (`replayed: true`), a re-imported statement reports `replayed_count`, and `voucher validate` warns when a key is already sealed. Restating that is what `skills/AGENTS.md` asks to keep out.
- **F17** (Edge-case lens row 'Resource exhaustion: disk, API limits'). No guidance for a failed write or an unavailable source. *Evidence:* Generic operating advice, not knowledge about this task; the month's commands are local and report failures with `error_code` and `message`, which SOUL tells the agent to read.

## Outcome

All twelve FIX rows were applied to the skill by hand in the same change (the
tree was not clean, so `--apply` was not used). No row needed an issue of its
own: every accepted finding was a wording defect in the draft.

## Simplicity

The rebuild reuses what the program and SOUL already say instead of adding to
them: command formats come from `--help`, the house rules and the approval
principle from `SOUL.md`. It removes the skill's copies of both (the receipt
JSON, the VAT option list, the assignment example, the `TODO.md` and
`knowledge company` section, the hash walkthrough). What it adds is the part no
other source carries: the bank import, reconciliation and the close. Left out
on purpose: a reference file (nothing in the month is rare enough to move out),
a rename, and any recovery or concurrency procedure the review asked for that
the program's replay keys already cover.
