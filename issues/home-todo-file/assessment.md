# Assessment of the home-todo-file review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| R1 | "Same commit" assumes a commit that does not exist | CONFIRMED | COMMON | IMPROVES | NONE | HIGH | FIX — applied |
| R2 | Example VAT period the program cannot represent | CONFIRMED | OCCASIONAL | IMPROVES | NONE | MEDIUM | FIX — applied |
| R3 | Deadline dropped while filing unprocessed | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MEDIUM | FIX — applied |
| R4 | Resume path for existing files | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MEDIUM | FIX — applied |

FIX: 4   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- R1: every session hits this wording; homes without git are allowed. The
  issue itself said "same commit", so the requirement's wording was
  corrected rather than followed literally.
- R2–R4: one-sentence text fixes, each removing a way an agent could
  misexecute the guidance.

No follow-up issue was filed.

## Simplicity

- Reused: the existing home `AGENTS.md` pattern (company-owned prose the
  program does not parse) and the existing guide pages; no new page.
- One home per rule: the full convention (three sections, what goes where)
  is stated once in `aggountant-init`; `capture-receipt` and `vero-fi` give
  only the part their work touches and point back to it. In the guide the
  description lives in `concepts/company-home.md`; the setup and monthly
  pages give a sentence and link there.
- Left out: any command, template file or parsed format for `TODO.md`
  (out of scope by the issue), and a way to raise an `obligation` by hand.
