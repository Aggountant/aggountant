# Result: README plain intro

The README now opens with one plain sentence ("keep a Finnish limited
company's books by talking to an AI agent"), a short paragraph on the
division of work (you hand over material, the agent prepares, you approve,
aggountant records and files), then a worked month for Esimerkki Oy, an
honest status, installation, getting started, links to both guide languages,
and a short "Under the hood" section for developers.

## Separation (rethink-document)

The issue text is Jari's word on what the README must say. The previous README
came from the initial public snapshot (one commit, agent-written), so every
statement in it was sorted on its merits against the binary and `docs/user/`.

| Statement in the old README | Decision | Reason |
|---|---|---|
| Bookkeeping for a Finnish limited company with an AI agent | Kept, rewritten as the opening sentence | The core of what a newcomer needs |
| kirjanpito / ALV / tilinpäätös list in the first paragraph | Reworded | Finnish terms on the first line are jargon for a newcomer; tilinpäätös is not usable yet, so leading with it overclaimed |
| "payroll reports to Tulorekisteri" | Narrowed to tax-free travel allowances | Guide ch. 6: wage payroll is not usable for real books; only travel allowances reach Tulorekisteri today |
| Skills as the agent's "operating manual", strict validation, `--json`, explicit approval steps | Approval kept up front in plain words; the rest moved to "Under the hood" | Issue: implementation vocabulary belongs further down |
| Status: 0.x, one company's live books, one user, rough edges, changing formats | Kept | Honest status the issue asks for |
| "a closed period and an accepted VAT return" | Dropped | Detail of one real company's books; "keeps the live books of one company" carries the point |
| "skills and user guide written in Finnish first" | Kept for skills; guide now linked in both languages | The English guide exists |
| One user, one machine, one company per home, not hosted | Kept in plain words | Product invariant a newcomer must know |
| How it works: company home is a directory, SQLite + content hashes, content-addressed artifact store | Condensed into "Under the hood"; storage engine detail moved to the guide (ch. 2) | Developer-level detail |
| camt.053/camt.052, Holvi | Kept inside the month walkthrough | Concrete: tells the reader which bank file to fetch |
| VAT directly to Vero.fi with own software registration and certificate | Kept in the month and in getting started | A real precondition the user must arrange |
| Uncertain submission reconciled, not resent | Kept in "Under the hood" | Distinctive safety property, but not first-screen material |
| Wage reports to Tulorekisteri | Dropped (see above) | Not true for real use today |
| `aggountant init --home ./accounting` in getting started | Dropped from the README | The `aggountant-init` skill runs it; the reader talks to the agent. The command remains in guide ch. 2 |
| Pointer to `docs/DISTRIBUTION.md` | Dropped | Maintainer material, not for a newcomer |
| Contributing section | Kept unchanged | Still accurate |
| Prebuilt-binary sentence and installer one-liner | Kept inside the install marker region, with the statement that no release is published yet; from-source command moved first | `gh api repos/Aggountant/aggountant/releases` returns 0 releases and the repo has no tags, so the one-liner currently fails |

Added: the "A month with aggountant" walkthrough, built from guide ch. 4, 5
and 7 (the €124.00 licence example is the guide's own), and the not-ready list
(wages, receipt search, year-end close) from the guide's status section.

## Verification

- Every fenced command (`aggountant skill list`, `aggountant skill install`,
  `aggountant --help`, the `cargo install --git` line) checked against a
  debug build of this tree (`aggountant 0.1.0`): `skill install` default
  target is `~/.claude/skills` and `--target` exists; `voucher verify` exists.
- Platform claims match the contract's `distributions[].platforms`.
- `shipshape audit`: see the closing report for before/after; no new README
  gap.

## Facts for the guide redesign (not edited here)

- Guide ch. 2 says "There are no ready-made installation packages yet" and
  installs with `cargo install --locked --path crates/cli`. That is true
  today, but it will be stale the moment the first release is cut, and the
  README's install region (owned by shipshape) will then lead with the
  installer. Whoever cuts the first release should update both: the README
  install region (move the shell installer and Releases page first, drop
  "No release has been published yet") and guide ch. 2.
- The binary's top-level help still says "S1 sealed-voucher light path" and
  describes `employee` as an "encrypted store"; guide ch. 6 already flags the
  encryption mismatch. Not a README concern, noted for completeness.

## Decisions for later review

- **Opening sentence names "an AI agent", not a product.** The skills install
  into Claude Code's directory by default; the README says so in getting
  started and avoids one-runtime phrasing in the opening (`shipshape audit`
  flags "Claude Code skill" wording).
- **Contents list kept** (shipshape `mvp` tier asks for one) but placed after
  the intro so the first screen is the plain description.
- **"Under the hood" section kept** rather than moving all implementation
  detail to the guide: an evaluating developer reads the README, and three
  sentences about the CLI, the voucher chain and filing safety answer their
  first question without sending them into a user guide written for owners.
- **No `aggountant init` in the README.** Onboarding is the agent's job via
  the `aggountant-init` skill; showing the bare command invited readers to
  skip onboarding.
- **Install region rewritten honestly** instead of leaving the generated
  installer-first order, because no release exists. This is the one deviation
  from shipshape-readme's install ordering, and it reverts on the first
  release (see above).
- **The restaurant-bill question in the walkthrough is illustrative**, not a
  documented skill behaviour; it shows the kind of owner-only question the
  guide (ch. 3) describes.
