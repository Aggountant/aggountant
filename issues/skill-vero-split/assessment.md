# Assessment: skill-vero-split

`/assess-findings` on `review.md`, 2026-11-21. Every FIX below is applied in this change;
the two SPIN-OFF rows are filed as `filing-correction-unrecordable` and
`employer-replacement-external-unchecked`. Simplicity: no fix added a section; each is
one to four sentences where the agent meets the case, and four findings were dropped
because their fix would have been text in another skill or for a case nobody has seen.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | An empty OmaVero view is offered as evidence for manual refiling [^F1] | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX |
| F2 | Missing-tax branch points to a section that says the preview figures are the ones to file [^F2] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F3 | e-services "selain ei ole varakanava" reads as forbidding the manual route vero-fi directs [^F3] | CONFIRMED | RARE | WORSENS | NONE | MED | DROP (Rule 1b: RARE, no readability gain) |
| F4 | Skills promise complete-manual records an e-service correction [^F4] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F5 | A correction made in the e-service cannot be recorded; file-wage complete-manual succeeds without recording it [^F5] | CONFIRMED | OCCASIONAL | NEUTRAL | MODERATE | HIGH | SPIN-OFF |
| F6 | "sen jälkeen korvaava onnistuu" promises a safe replacement after recording an external reference [^F6] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F7 | Replacement of an externally filed month skips the pension-insurance check [^F7] | CONFIRMED | OCCASIONAL | NEUTRAL | MODERATE | MED | SPIN-OFF |
| F8 | e-services names the tulorekisteri skill without requiring it [^F8] | CONFIRMED | RARE | NEUTRAL | MINOR | MED | DROP (Rule 1b: RARE, no readability gain) |
| F9 | "Älä poimi … tunnisteita" could forbid reading a report reference [^F9] | CONFIRMED | RARE | NEUTRAL | NONE | LOW | DROP (Rule 1b: RARE, no readability gain) |
| F10 | "ainoa vaihe" contradicted by later visits to the e-service [^F10] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F11 | Catch-up months inherit today's pension-insurance settings [^F11] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F12 | No instruction for an occasional employer that did pay something [^F12] | UNABLE_TO_VERIFY | OCCASIONAL | NEUTRAL | NONE | LOW | FIX |
| F13 | "todella puuttuvat kuukaudet" overstates what an imported transcript proves [^F13] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX |
| F14 | Several registration periods are not representable by one start month [^F14] | UNABLE_TO_VERIFY | RARE | WORSENS | MODERATE | LOW | DROP (Rule 1b: RARE, no readability gain) |
| F15 | A travel payment made before aggountant may already be reported [^F15] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX |
| F16 | Deadline stated as five days from payment without saying it is the program's cautious date [^F16] | CONFIRMED | REGULAR | NEUTRAL | NONE | MED | FIX |

[^F1]: review lines 15, 88 (gpt-6-astra); `skills/vero-fi/hairiot.md`. Both sentences were in the first draft; the example was the writer's own addition. `complete_manual_vat` checks only that a reference is given. Fixed in hairiot.md.
[^F2]: review lines 30 (gpt-6-astra); `skills/vero-fi/SKILL.md`. Read both passages; literal reading files the under-reported figures. Fixed: corrected fields are computed from the vouchers and shown against the preview; the manual section excepts the missing-tax case.
[^F3]: review lines 45 (gpt-6-astra); `skills/e-services/SKILL.md`. vero-fi states the route explicitly and it is the user who files, which e-services allows ("ilmoituksen, jonka käyttäjä antaa itse"). The fix is text in another skill for a misreading the task skill already settles.
[^F4]: review lines 62, 110, 266 (gpt-6-astra); `skills/tulorekisteri/SKILL.md`. `crates/process/src/file_wage.rs`: `complete_wage_manual` returns `self.show(run_id)` when the Run already has EffectSucceeded/EffectReconciled. Fixed in both skills; e-services wording left (its own file).
[^F5]: review lines 62, 110, 266 (gpt-6-astra); `crates/process/src/file_wage.rs`. Code read as above; `acquire_filing_authority` refuses a filed VAT period. Corrections in the e-service are the documented route for travel reports, so the case is ordinary use.
[^F6]: review lines 134, 182 (gpt-6-astra); `skills/tulorekisteri/kiinnikurominen.md`. `file_employer_report.rs`: `previous = current.and_then(|r| r.report.as_ref())` is None for `Filing::External`, so insurances come from the profile and the REPLACEMENT_NOT_ALLOWED comparison is skipped. Fixed in kiinnikurominen.md.
[^F7]: review lines 134, 182 (gpt-6-astra); `crates/process/src/file_employer_report.rs`. Same code as above. Whether the register rejects or accepts such a replacement was not tested.
[^F8]: review lines 156 (gpt-6-astra); `skills/e-services/SKILL.md`. The mention is prose, not a link; the route into e-services is from a task skill, and both filing skills require it. Requiring task skills from e-services would invert the dependency.
[^F9]: review lines 157 (gpt-6-astra); `skills/e-services/SKILL.md`. The sentence lists cookies, identifiers and session ids together and the user, not the agent, fetches the reference. Not this slice's file.
[^F10]: review lines 158 (gpt-6-astra); `skills/tulorekisteri/kiinnikurominen.md`. Same file names two later visits. Fixed ("Tavallisesti").
[^F11]: review lines 196 (gpt-6-astra); `skills/tulorekisteri/kiinnikurominen.md`. `plan` takes `profile.pension_insurances` for every month. Fixed with one paragraph in kiinnikurominen.md.
[^F12]: review lines 212 (gpt-6-astra); `skills/tulorekisteri/SKILL.md`. The gap in the text is real and the commands need `employer_report`; the reviewer's statement of the legal obligation was not verified here, so the fix does not assert it.
[^F13]: review lines 230 (gpt-6-astra); `skills/tulorekisteri/kiinnikurominen.md`. `missing` lists locally unaccounted months. Fixed.
[^F14]: review lines 248 (gpt-6-astra); `skills/tulorekisteri/kayttoonotto.md`. No occurrence; `registration` reports `register_ended` and the comparison is shown to the user. A skill paragraph for it would be speculative.
[^F15]: review lines 283 (gpt-6-astra); `skills/tulorekisteri/SKILL.md`. Identity is local (`wage_report.rs`). Fixed with one sentence.
[^F16]: review lines 297 (gpt-6-astra); `skills/tulorekisteri/SKILL.md`. The guide (`tasks/report-travel-allowances.md`) calls payment date + 5 a deliberately cautious deadline; the program is not wrong to show it. The statutory rule was not verified. Fixed wording only.

FIX: 10   FIX (with care): 0   SPIN-OFF: 2   DISCUSS: 0   DROP: 4

### F5 A correction made in the e-service cannot be recorded; file-wage complete-manual succeeds without recording it

- **Milloin tämä näkyy käyttäjälle** — kun jo hyväksytty matkakorvausilmoitus tai ALV-ilmoitus korjataan viranomaisen asiointipalvelussa, mikä on korjausten ainoa reitti.
- **Miten se näkyy** — agentti kirjaa korjauksen viitteet `file-wage complete-manual` -komennolla, komento onnistuu, mutta `run show` näyttää edelleen alkuperäisen ilmoituksen. ALV:ssa ja erillisilmoituksessa komento kieltäytyy.
- **Miksi sillä on väliä** — kirjanpito väittää viranomaiselle annetuksi jotain muuta kuin siellä on, ja myöhempi päätös tehdään vanhan tiedon varassa. Matkakorvauksissa virhe on hiljainen.
- **Miksi tämä vaatii oman suunnittelunsa** — korjauksen kirjaaminen koskee kolmea ilmoituslajia ja sitä, mikä ilmoitus on "voimassa oleva"; erillisilmoituksen mitätöinnistä on jo oma issuensa, ja ratkaisun pitää olla niille yhteinen.

### F7 Replacement of an externally filed month skips the pension-insurance check

- **Milloin tämä näkyy käyttäjälle** — kun aiemman palkanlaskijan antamalle kuukaudelle on kirjattu ilmoitusviite ja sille lähetetään korvaava erillisilmoitus.
- **Miten se näkyy** — korvaavaan tulevat profiilin nykyiset eläkevakuutustiedot, eikä ohjelma vertaa niitä alkuperäiseen, koska se ei tunne alkuperäisen sisältöä.
- **Miksi sillä on väliä** — Tulorekisterin sääntö on, että korvaavassa on samat eläkevakuutustiedot. Jos ne eroavat, ilmoitus hylätään tai rekisteriin jää väärä vakuutustieto ilman, että kukaan päätti muuttaa sitä.
- **Miksi tämä vaatii oman suunnittelunsa** — ohjelman pitää joko kieltäytyä, kun aiempi sisältö on tuntematon, tai saada alkuperäiset tiedot käyttäjältä; kumpikin muuttaa `complete-manual`- ja `--replace`-polkujen sopimusta.
