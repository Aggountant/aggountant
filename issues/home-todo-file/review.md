# Review: home-todo-file

- Skill: `/llm-review`, rigor **routine**, risk class low: skill text and
  user guide only, no program behaviour changes. The risk is an agent
  misreading where pending work goes, or a guide claim that contradicts the
  program.
- Scope: the working-tree diff of `skills/{aggountant-init,capture-receipt,vero-fi}/SKILL.md`
  and `docs/user/{fi,en}/{concepts/company-home,start/set-up-company,tasks/monthly-routine}.md`,
  with the full init and capture-receipt skills and `concepts/periods.md` as
  context.
- Finder: `claude-fable-5-2`, one call. Verifier: none. All four candidates
  were textual claims that could be settled by reading the cited lines and
  the program (`.gitignore` writer, `PeriodKind`, `ObligationCmd`), so a
  second model would have added nothing. One model call in total.

## R1 (should-fix): "same commit as the entry" assumes a commit that does not exist

- Location: `skills/aggountant-init/SKILL.md` (AGENTS.md pointer and the
  "Mikä menee minne" list), `skills/capture-receipt/SKILL.md` (Odottava työ),
  `skills/vero-fi/SKILL.md` (Määräajat).
- Claim: sealing a voucher changes nothing git tracks (`fiscal.sqlite` and
  `artifacts/` are ignored), and the home need not be a git repository at
  all (`init` creates none). The wording could make an agent defer the edit
  to a commit that never comes, or run `git init` unasked.
- Evidence: finder ran a scratch home; locally confirmed the `.gitignore`
  writer and the init skill's "ei alusta git-repositoriota".
- Fix: the file is updated as soon as the voucher is sealed or the act done;
  in a git home the change is committed with the home's other changes.

## R2 (should-fix): example VAT period the program cannot represent

- Location: `skills/aggountant-init/SKILL.md`, Esimerkki Oy example
  ("kauden 1.11.–31.12. ALV-ilmoitus … 28.2.").
- Claim: an annual filer's partial first period, not labelled as such;
  aggountant's VAT periods are months or quarters.
- Fix: example changed to the December return due 12 February.

## R3 (should-fix): vero-fi drops the deadline while the filing is unprocessed

- Location: `skills/vero-fi/SKILL.md`, Määräajat.
- Claim: the deadline was removed on Vero's receipt, but the same skill says
  a receipt is not processing and to escalate "jos eräpäivä painaa"; the
  follow-up item carried no due date.
- Fix: the deadline goes when the filing is processed (`file-vat status`
  `processed`) or recorded with `complete-manual`; the follow-up item keeps
  the run id, the due date and the recheck time.

## R4 (should-fix): resume path for existing files

- Location: `skills/aggountant-init/SKILL.md`, `skills/capture-receipt/SKILL.md`.
- Claim: init said "create" with no clause for an existing, possibly
  user-edited `TODO.md`; adding the pointer to an existing `AGENTS.md` was
  not stated as compatible with "lue ja säilytä"; capture-receipt created a
  missing `TODO.md` without the pointer.
- Fix: init reads and supplements an existing file and only adds the pointer
  to an existing `AGENTS.md`; capture-receipt adds the pointer when it
  creates the file.

## Dropped

None. The finder's lower-confidence aside (no program source for the next
period's due date) is left as is: the skill says "jos se on tiedossa".

## Coverage

Checked and holding: the new anchors (`#kodin-todomd`, `#the-homes-todomd`,
`#velvoitteet`, `#obligations`), fi/en heading parity and facts, the
obligation claims (only `list` and `resolve` exist; an open one blocks the
close), and that installing `capture-receipt` alone also installs
`aggountant-init`, so its cross-reference resolves. `scripts/check-user-guide.sh`
and the skill bundle tests pass after the fixes.
