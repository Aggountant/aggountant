# Review of eservice-skill-bundle (slice 1 of ADR-10)

- **Finder:** gpt-6-astra (`--task review`), one follow-up after its context request.
- **Verifier:** claude-fable-5-2, one batched refutation call.
- **Model calls:** 3. **Rigor:** routine; the skill text governs when an agent
  may touch a filing form, so the boundary wording was checked against ADR-10
  and its original revision.
- **Scope:** the slice diff: `crates/cli/src/skill.rs` (installer, packaging
  tests), `crates/cli/tests/skill_cli.rs`, `crates/cli/src/main.rs`,
  `skills/e-services/*`, `skills/vero-fi/SKILL.md`, `skills/AGENTS.md`, the
  install pages (fi, en).

## R1 — should-fix: the vero-fi manual route and omavero.md disagreed about filling a VAT return

- **Location:** `skills/vero-fi/SKILL.md` "Manuaalinen reitti";
  `skills/e-services/omavero.md` `omavero-lomakekentta-syotto`;
  `skills/e-services/SKILL.md` "Tieto ei ole lupa".
- **Claim (finder):** letting the agent fill the OmaVero VAT form contradicts
  ADR-10's "the browser is never a fallback channel for a filing aggountant
  can make".
- **Evidence:** the verifier refuted the contradiction with the ADR: its
  exception names "a manual return", recorded through the manual-filing path,
  and the proposal's original wording ("a form the user files through the
  portal (a manual return, a correction aggountant does not produce)") makes
  "does not produce" qualify only the correction. What survived is an
  internal inconsistency: vero-fi permitted the fill, while the linked
  omavero entry and SKILL.md allowed it only "when aggountant does not produce
  the same return" and said "if a command exists, use it, not the browser".
  The verifier also noted the real risk: "direct route unavailable" includes
  a period blocked by an unresolved API submission, where a browser fill would
  bypass `VAT_FILING_UNRESOLVED`.
- **Fix:** SKILL.md, omavero.md and vero-fi now say the same thing: a return
  the user files by hand may be filled on their explicit request; never past a
  blocker, an unresolved submission or a pending approval; recorded with
  `complete-manual`. vero-fi forbids the fill while the period has an
  unresolved API submission.

## R2 — should-fix: the submit-confirmation entry put the stop at the OK dialog

- **Location:** `skills/e-services/omavero.md` `omavero-lahetys-vahvistus`.
- **Claim (verifier, adjacent):** "pysähdy sen kohdalla" at the confirmation
  implies the agent may press the submit button itself; ADR-10 makes the final
  submit the user's act, and the safety then rests on every form showing the
  dialog.
- **Fix:** stop before the submit button; the submit and its confirmation are
  both the user's.

## R3 — uncertain / decider: seed dates are not independent witness dates

- **Location:** every entry's *Havaittu* / *Vahvistettu* line.
- **Claim:** letter entries reuse the dates of ADR-10's example entry; the
  form, submit and browser entries carry the ADR's date (2026-11-19); the
  report 630 entries carry the date its page steps entered `vero-fi`
  (2026-11-18). ADR-10 says dates move only on evidence and has no
  "unverified seed" state.
- **Assessment:** the observations come from production use in that window,
  so the error is days to weeks, and the freshness rule's 180-day horizon is
  not materially misled. The verifier's sharper point applies to slice 2:
  "a local fact newer than *Vahvistettu* wins" would let a seed dated a few
  days late outrank a real earlier contradiction. Left to the decider; see
  `result.md`.

## Dropped

None beyond R1's refuted ADR-contradiction framing. The finder and the
verifier found no installer or test defect.

## Coverage

Read: the whole diff, ADR-10, the e-services files. Tested: `ci-local`,
mutation checks (dropping `e-services` from vero-fi's requirements fails the
link and requirement tests; an invalid date fails the header test). Not
covered: whether the OmaVero behaviour still holds at the live pages, which
only someone at the page can check.
