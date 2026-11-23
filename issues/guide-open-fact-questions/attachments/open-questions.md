=== concepts/division-of-work
- Should the owner-facing guide name `SOUL.md` at all? `docs/user/AGENTS.md`
  says to name internal mechanisms only when the user meets them on screen. I
  kept it because it is a true fact on the page and a curious owner can open
  the file, but the sentence works without the file name.
- The page says permanent acts "kuten" (such as) sealing, month close and
  filing. SOUL.md says there are exactly three. If the list is complete, the
  page could say so; I left "kuten" because sealing-and-approval.md is the home
  of that fact.
=== tasks/close-month
1. Step 4 lists "the month's open obligations" in what the user approves,
   although an open obligation blocks the close from starting. Should the page
   drop it from the list (always empty in practice), or does it stay because
   the content really has the field?
2. For `PERIOD_CLOSING`, should the page tell the user they can reject the
   pending close and record in the same month, instead of only "record in the
   next month"? I did not add it, as it is not on the page today.
=== start/is-it-for-you
1. How many companies keep real books with aggountant: "muutaman" (guide) or
   one (`OSS-RELEASE.md`)? The page states "a few" in its second paragraph.
2. Should the guide name the version ("0.2") in running text, or only say it
   is an early 0.x version whose commands and file formats may change?
3. "Kokeile ilman riskiä" is the page's call to action but sits last, after
   responsibility and limitations. Should it move up, or is last right for a
   reader who has just decided?
=== start-install
- The installer puts the binary in `~/.cargo/bin` (or `$CARGO_HOME/bin`) and
  edits the shell profile, so `aggountant` may not be found until a new
  terminal is opened. I left this out because it is a new fact. Should step
  2 say "open a new terminal if the command is not found"?
- Is Windows a supported platform at all? If a source build is not known to
  work there, the page should say macOS and Linux only instead of sending
  Windows users to `cargo install`.
- The README mentions SHA-256 checksums for each release. Left out as
  detail the owner does not act on; say so if it should be on the page.
=== reference-status
- Is Windows actually supported from source? If not, the Asennus section
  should say Windows is not supported instead of "käännetään lähdekoodista".
- Has 0.2.0 been released, or is the workspace ahead of the latest release?
  The page says 0.2; if the published binaries are still 0.1.x, a reader who
  installs with the shell installer gets an older version than the page
  describes.
- Should each limitation keep a scannable bold lead on this one page (see
  style-guide feedback)?
=== README
- Is "aggountantin käyttäjän opas" / "aggountant user guide" the title you
  want as the site's page title for `/guide/`, or plain "Käyttäjän opas" /
  "User guide"?
- "sillä pidetään jo muutaman yrityksen oikeaa kirjanpitoa": still true and
  still wanted on the front page as well as on status and is-it-for-you? It is
  now in three places.
Addendum to section 4: `site/src/components/Landing.astro` lines 62 and 116 do
say "Versio 0.1" / "Version 0.1". That is the site's landing page, the most
visible place the stale version appears.
=== start-early-users
1. The page publishes a personal email address. It was there before and I
   kept it. Is that the address you want on the public site, or should the
   page point only to GitHub?
2. Is "usually within a few days" the response time you want to promise? The
   original said "within days, not weeks".
3. "Sovitaan tapaaminen": is a meeting offered to anyone who is considering
   aggountant, as the page says, or only once they have described their
   company? The page does not say how to ask for one beyond "Ota yhteyttä".
=== tasks-monthly-routine
- The page speaks only of months, but `concepts/periods.md` says a company can
  close quarterly (`2026-Q1`). Should the routine page say in one sentence
  that a quarterly VAT filer closes and files per quarter (steps 7–8), with a
  link to periods? Not added, since it is not on the page today.
- Step 2 now says "Agentti kirjaa kuitit ja ostolaskut". Approval is stated in
  step 1 ("hyväksyttävät tositteet") and step 3. Is that enough, or should
  each recording step repeat that you approve?
- Step 8 says "Maksa vero itse" and "Valmis, kun" repeats the payment with the
  OmaVero reference. I kept both (one is the action, the other the check).
=== concepts-sealing-and-approval
- In `bank_import.rs` the approved preparation hash is optional ("so existing
  callers keep working"); without it the import seals with no binding check.
  The page says the import "works the same way" as receipts. Is the hash
  always passed in the agent's skill flow, so the statement holds for users,
  or should the page say the binding applies when the agent passes the hash?
- The dropped reassurance ("a mistake in a sealed voucher is not a disaster")
  was a repeat, but it was also the page's only comforting line. Restore it if
  you want that tone in the closing list.
=== concepts-periods
- What does "Sulun aikana tehdyt kirjaukset säilyvät" refer to, given that a
  period in `closing` does not accept entries? If the close records nothing
  itself, the sentence should go and "mitään ei poisteta" is enough.
- Should run states live on this page (as `AGENTS.md` says), or is that line
  stale?
- "Suoraa ALV-ilmoittamista on koeteltu kuukausikausilla": does it mean
  quarterly direct filing is untested, or unsupported? The reader with a
  quarterly VAT period cannot tell what to do.
=== concepts-filings
- Does the uncertain-outcome flow for a Tulorekisteri travel report end the
  same way as VAT (asked again, closed as not filed after repeated "not
  found"), or does it stay open until the user settles it? The page is now
  explicit that the two-answer rule is VAT's but says nothing about how a
  Tulorekisteri run gets out of the state.
- Should the page list all bank holidays or drop the list? A sample of five
  of eleven invites the reader to guess about 1 May or Independence Day.
=== concepts-vat-treatment
- Is this concept page the right home for the exempt EU service and exempt
  intra-Community acquisition rules with their Verohallinto citations, or
  should they be a short section in `tasks/record-receipts.md` (or stay only
  in the error-code row), leaving the concept page with one sentence and a
  link? Half the page is now a case most owners never meet.
- "Verotonta käsittelyä käytetään vain, kun verottomuus on varmistettu": who
  confirms it, the owner or the bookkeeper? The page does not say, and the
  closing list sends unusual purchases to the bookkeeper. Saying "kysy
  kirjanpitäjältäsi" there would be a new instruction, so I left it.
=== concepts-company-home
- The heading says "Miten ohjelma löytää kodin" while the English says "How
  aggountant finds the home", and the page mixes "ohjelma" and "aggountant".
  Should the guide pick one? I left the heading (it has inbound anchors) and
  used "aggountant" where the sentence opens with the actor.
- The section "Yksi koti, yksi yritys" is now two sentences. It could merge
  into the intro, but that removes a heading in both languages. Keep it?
=== concepts-for-bookkeepers
1. Should this page tell the bookkeeper that year-end entries are recorded in
   the next year's first open period and allocated to the ended year? It is
   the detail a bookkeeper reading the voucher list would trip on, but it
   was not on the page, so I did not add it.
2. On this page "sinä" is the bookkeeper, not the owner. That matches the
   original, but the "Mitä aineistoa saat" section tells the bookkeeper to
   ask the owner or the agent. Is the page also meant for a bookkeeper who
   runs the agent themself? If so that line should say how.
=== tasks-record-receipts
- Should the example prompt and the worked posting table describe the same
  purchase (same payment method), or is showing both cases on purpose?
- `PERIOD_CLOSING` on this page says only "kirjaa seuraavalle avoimelle
  kuukaudelle"; the error-code table also offers "hyväksy sulku". Is leaving
  that out here intended?
=== tasks-import-bank-statement
1. After a successful import, can a transaction be "nimetty avoimeksi"
   without being recorded? If "open" always means "recorded to 2700 and
   waiting for its receipt", the "Valmis, kun" line should say that instead of
   "joko kirjattu tai … avoimeksi".
2. Should this page name `SOURCE_IDENTITY_UNKNOWN`, `MISSING_ASSIGNMENT` and
   `ACCOUNT_NOT_FOUND`, since the error-code table sends readers here?
=== tasks-correct-voucher
- The page says a correction for a closed month goes to the next open month.
  The code also allows reopening a closed month before its VAT return is
  filed. Should this page mention reopening as an alternative (with a link to
  `concepts/periods.md`), or is the correction voucher the only route you
  want users to take? I added nothing.
- Step 3 tells the user to tell the agent about a filed VAT month "before
  sealing" without saying why (the marking cannot be added afterwards; the
  reason is on file-vat-direct). Is one clause of reason wanted here, or is
  the link enough?
=== tasks-upgrade
- Does rerunning the shell installer cleanly replace a binary that was first
  installed with `cargo install` (same directory)? If not, the page needs a
  sentence for users who started on the source build.
- Is there a limitation entry in `reference/status.md` that "Rajoitukset"
  here should link to? The page keeps the limitation in full with an
  issue-tracker link, which is against "one fact, one home", but status.md has
  no upgrade entry to link to.
=== tasks-report-travel-allowances
- **Is "pääomaosakkuus" the right Finnish term for `joint_owner`?** The code
  maps it to Tulorekisteri's IncomeEarnerType 2, which the Incomes Register
  calls "Osaomistaja", and the skill says it is not the unemployment-law
  "osaomistajuus". The page, the glossary and the skill all say
  "pääomaosakkuus". If the user sees "Osaomistaja" in the e-service, the
  guide should probably name that word too.
- **"Hakemuksen voi hyväksyä heti": who approves?** It can be read as "you
  can approve (sign) the application right away" or "the application is
  granted at once". I left the sentence as it was in both languages.
- Should the user be told in step 2 that the agent asks for the earning
  period too? The skill says the program does not infer it from the claim;
  the page only says so for shareholding, so I added nothing.
=== tasks-file-employer-report
- Should the command example show `--record-hash-file` instead of
  `--record-hash`? `--help` says the hash on the command line ends up in
  shell history and prefers the file form, though for this report it also says
  "`--record-hash` is fine" because the hash holds no personal data.
- Should the page name the error codes for the situations it already
  describes (wages month, YTJ down, replacement not allowed), as it does for
  two others? That would be adding content, so I left it.
- Setup has no explicit approval step on the page: the agent "shows" the
  settings and the example command is a `--dry-run`. Does the agent wait for
  the user's yes before the real write? If so, step 3 should say "Tarkista ja
  hyväksy".
=== tasks-year-end
1. Should step 6 say that the first approval already puts the depreciation
   and accrual vouchers permanently in the books, and that each correction
   is approved separately before it? The help says so; the page does not,
   and a user who rejects at step 2 to 4 would otherwise expect nothing to
   have been recorded. I did not add it because the brief forbids new facts.
2. "Ennen kuin aloitat" requires "ALV ilmoitettu" for every month and step 1
   requires `vat_filed true`. Does `close-year start` enforce the VAT filing
   or only that months are sealed? The help says "whose months are all
   sealed". If it is not enforced, the page should say it is the user's
   check.
=== tasks-file-vat-direct
- Should the task page say that the replacement return is untested against
  production, or is the link to status.md enough?
- Should the monthly-periods limitation say outright that quarterly periods
  are less tested?
=== tasks-file-vat-omavero
- Should the reconciliation mechanism (settlement accounts, what is left out)
  stay on this task page at all, or move to `concepts/` or `status.md`? It is
  the longest thing on the page and the direct-filing page needs it equally.
- Step 4 says "aggountant ei maksa mitään" and the command note says the
  command "ei lähetä eikä maksa mitään". I kept both because a user of the
  manual route may assume otherwise; say if one is enough.
- `masterdata/reference/settlement-account-roles.json` is named for users with
  their own chart. Does the user (or the agent) really edit that file by hand,
  or is there a command the page should name instead?
=== tasks-set-up-vero-api
- Prerequisite 2 says "Vero API -sopimus tai -hakemus". Is a submitted
  application enough to go on, or must the agreement be in force? The code's
  field is `api_agreement_active`, which suggests the latter.
- Prerequisite 5 is passive: "tuotantokäyttö aktivoidaan portaalissa". Does
  the user activate it, or does the Tax Administration? Step 4 treats it as
  something the user did. I kept the passive because I could not tell.
- "Kun varmenne vaihtuu", step 3: "ne portaalitoimet, jotka koskevat uutta
  varmennetta". Which ones? The readiness check binds only the test evidence
  and the activation to the fingerprint, so the four attestations stay true
  after a renewal; the step may be unnecessary or needs naming.
- The intro promises "useita odotusaikoja". Is there a rough figure (days,
  weeks) worth giving, so "hyvissä ajoin" means something?
=== tasks-record-other-entries
- Should the page name `natural_key` at all? It is in the folded command
  notes only, and the user never types it, but it is what
  `NATURAL_KEY_ALREADY_SEALED` refers to. I kept it.
- The page covers four kinds of entry (sales invoice, payroll, opening
  balances, corrections) but its steps and "Valmis, kun" are written almost
  entirely for the sales invoice. Corrections and opening balances have their
  own pages; payroll has only the example prompt and one limitation. Is that
  enough for payroll, or does a payroll voucher need its own checks (what to
  compare against the payroll summary)? That would be new content, so I did
  not add it.
=== tasks-monthly-report
1. Should the `PERIOD_NOT_SEALED` item keep an explicit "kirjanpidossa ei ole
   vikaa"? I removed it as a denial, but an owner who sees an error every
   January may want to be told so directly.
2. Should the page say that a company whose first fiscal year in aggountant
   has no earlier year at all also does not meet this error? `--help` says so;
   the page is silent.
=== tasks-back-up-and-restore
1. Why must no aggountant command be running during the copy? The page gives
   no reason. If the reason is that a copy taken mid-write can be an
   inconsistent database, one clause would make the user take the condition
   seriously; I did not add it because it is not on the page.
2. Step 4 of the restore says the agent shows period states and the last
   voucher number, but the command block also lists `config show`, which no
   step explains. Should the step say that the agent first confirms the
   restored home is the one in use, or should `config show` leave the block?
3. "Kirjaa puuttuvat tapahtumat uudelleen lähteistään" is vague for an owner.
   Is "kuiteista ja tiliotteista" (through the normal receipt and bank
   statement tasks, with links) what is meant?
4. "Säilytä kopio kirjanpitolain säilytysajan": should the page name the
   periods (ten years for the books, six for vouchers) or link to where the
   guide states them? And should it say that one old copy is not enough,
   i.e. how often to back up (for example before every upgrade and after
   every closed month)? Today the page gives no rhythm at all.
=== start-set-up-company
1. For a brand-new company, what should "Valmis, kun" say about the first
   month's report? As written, the middle check only makes sense after a
   migration. I did not add a condition because it would be a new statement.
2. The skill tells the agent to suggest moving at a month boundary, since a
   mid-month opening voucher distorts the first month's result anyway. The
   page does not mention it. Should the user be told to pick a month
   boundary in "Ennen kuin aloitat"?
3. The skill has the agent ask for all OmaVero transaction statements of the
   cut-over year when there is a tax-account balance or payment arrangement.
   Step 5 only says the agent asks. Worth a prerequisite for migrating
   companies, or is that agent business?
=== start-move-from-another-program
1. The whole procedure on this page (inventory, read-only export, checksums,
   second copy, no credentials in files) is described as what the agent does,
   but no bundled skill tells the agent to do it. Is the page meant as
   instructions the user gives the agent, or should a skill carry it? As it
   stands the page promises agent behaviour that depends on the agent having
   read this page.
2. Step 1 has the agent take the figures for the inventory "from the old
   system" before step 2, where the user signs in. Should signing in come
   first, or does the user supply the figures for the inventory?
3. "SIE4-vientiä emme löytäneet" (Briox): keep the "we", or state it
   impersonally ("SIE4-vientiä ei ole")? The second is a stronger claim than
   what was observed, so I kept the first.
=== troubleshooting
- "Älä myöskään vaihda maksun tunnistetta tai maksupäivää" (Tulorekisteri
  rejection): `tasks/report-travel-allowances.md` adds "kiertääksesi
  hylkäyksen". Without that reason the sentence here is hard to understand.
  Should it carry the reason, or go, since the task page is its home?
- "Aiemmat versiot saattoivat jättää väärään paikkaan tyhjän kodin" and
  "sulku hylättiin aiemmalla versiolla": are there still users on homes
  touched by those versions? If not, both sentences can go.
- The test-seam section (`AGGOUNTANT_BROKER_SCRIPT`) describes a
  developer's situation. Does an owner ever meet it, or does it belong in a
  contributor document with only the error-code row left in the guide?
=== reference-glossary
- Should glossary entries link to the page that is the fact's home? It would
  also let several entries shrink to the definition alone.
- "Katso myös" lists Komennot and Virhekoodit. Would a concept page be the
  more useful third link?
- Should "Velvoite" stay while obligations cannot arise in normal use, or say
  so in one clause?
=== reference-commands
- Should the commands table follow the calendar (as now) or `--help` order as
  the template says?
- "Katso myös" has Sanasto and Virhekoodit. `status.md` is linked in the body
  and would be the more useful third link; add it?
- Is `close-year` meant to be presented as fully in use? `tasks/year-end.md`
  treats it so, and I followed that.
=== reference-vero-fi-blockers
- `VERO_PRODUCTION_VERIFICATION_UNATTESTED` is owned by `human` in the code and
  sits under "Sinun esteesi", but the setup page says the program records the
  tests as passed after a successful `verify-portal` run, which the agent
  starts. So the user's part is giving permission for the run. Should the cell
  say what the user does ("anna agentille lupa ajaa hyväksyntätestit"), or
  should the owner in the code be `agent`? As written ("hyväksyntätestejä ei
  ole kirjattu hyväksytyiksi") the reader cannot tell what to do.
- Should the page say that blockers can appear in stages (section 3)? It
  explains a real on-screen surprise, but it is a new fact on the page.
- The tables say what each blocker means but not what to do about it. For most
  codes the meaning implies the action; for the certificate-mismatch pair the
  action is the "Kun varmenne vaihtuu" section, linked only under the
  warnings. A third column would be a content change, so I left it.
=== start-practice-month
- The tutorial has no `<details>` command notes, although `AGENTS.md` says
  commands sit in a folded block under the steps. The page names four commands
  inline (`init`, `config show`, `report statements`, `file-vat preview`,
  `voucher verify`). Is the tutorial exempt, or should it get a block listing
  what the agent ran per step? I added nothing, since that would be new
  content.
- Step 7 says "jos suora ilmoittaminen on käytössä" without a link to
  `reference/status.md` or the Vero API setup page. Should the tutorial link
  it, or stay link-free there to keep one path?
=== reference-error-codes
- Should the `hint` for `VAT_TREATMENT_CLASS_MISMATCH` keep the return-box
  details (dropped here in favour of the concept page), given the agent reads
  the hint and may not follow the link?
- In the "what is done" column, should every row name the actor ("agentti …"
  / "sinä …"), or is a bare imperative fine when either may act? It decides
  whether some forty rows change.
- The first paragraph says the table holds "the most common" codes. Is that
  the intent, or should it hold every code a user can meet (for example
  `PROCESS_STORE`, which three rows mention as the `error_code` actually
  shown)?
