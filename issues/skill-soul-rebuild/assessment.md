# Assessment: review of `skills/aggountant-init/SOUL.md`

Review: `/llm-skill-review`, lenses executor and design-fidelity, one model (`gpt-6-astra`), 2026-11-21. Raw output is in `review.md` beside this file.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | What the user approves at a month close [executor 1] | CONFIRMED | REGULAR | NEUTRAL | MODERATE | HIGH | SPIN-OFF |
| F2 | "The program is right" could delete an unresolved TODO concern [executor 2] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | MED | FIX (applied) |
| F3 | AGENTS.md was to be read only before fetching or moving material [executor 3] | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX (applied) |
| F4 | When an interpretation becomes a company practice [executor 4] | CONFIRMED | RARE | WORSENS | NONE | LOW | DROP (Rule 1d: worsens readability, not regular) |
| F5 | "Before anything else" halts unrelated work on an uncertain filing [design-fidelity 1] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX (applied) |
| F6 | "Four places" read as all persistent storage [design-fidelity 2] | CONFIRMED | RARE | IMPROVES | NONE | MED | FIX (applied) |
| F7 | Secrets: user-only custody goes beyond "secrets in none of them" [design-fidelity 3] | INCORRECT | - | - | - | HIGH | DROP (Rule 1a: incorrect) |
| F8 | 672 words against "about 500" [design-fidelity 4] | CONFIRMED | REGULAR | NEUTRAL | NONE | HIGH | DISCUSS |

FIX: 4   FIX (with care): 0   SPIN-OFF: 1   DISCUSS: 1   DROP: 2

## Evidence

- **F1** SOUL says to show the proposal as it will be stored, but for a period close neither SOUL nor the program says what that content is. Ran `run start close-period --period 2026-07` on a synthetic home: the answer has the ApproveClose gate with only a snapshot_hash. Passes the filing gate as an observed occurrence; filed as `virtually-tidy-owl`. The same defect was fixed for close-year in `close-year-gates-show-content`.
- **F2** TODO bullet said that in a conflict the program is right, without limiting that to recorded state; an agent could drop a still-missing receipt because the month shows closed. The sentence as written covered the whole file. Reworded: "onko kausi suljettu tai ilmoitus annettu, luetaan ohjelmalta."
- **F3** The reading point was tied to material handling, though the file also carries practices and the git boundary that the commit rule depends on. Two separate reading instructions became one sentence in the section lead; the text got shorter.
- **F4** The role does not say which interpretations may be saved as practice. The role already asks for claim, scope and source and to keep sourced facts, user statements and interpretations apart, so an inferred pattern saved honestly names its vouchers as the source. The issue keeps the role as it is; a rule here adds text without an observed failure.
- **F5** "selvitä se ohjelmalta ennen muuta" is broader than the design principle (never resend on a guess) and contradicts the role, which carries other work forward. Reworded to "ennen kuin lähetät uudelleen".
- **F6** The lead said information that persists between sessions has four places, while the role also names the voucher and the source archive. Lead now reads "Tositteiden ja lähdearkiston lisäksi siinä on neljä paikkaa sille, mitä seuraavan istunnon pitää tietää."
- **F7** SOUL adds that secrets stay out of the conversation and that the user keeps them outside the home. Not a drift: both come from the copies SOUL replaces. vero-fi: "Käyttäjä laatii ja tallentaa sen itse", "Salaisuuksia ei pyydetä keskusteluun"; aggountant-init: "keskusteluloki päätyy usein talteen". User custody is the intended rule.
- **F8** The file is about 670 words; the role (279, kept verbatim) leaves about 220 for everything else under the target. Sizing decision, not a defect. The brief for this slice asks that the house rules be complete and precise enough for three task skills to delete their copies; those copies total about 590 words and are replaced by about 230.

## Write-ups

### F1 What the user approves at a month close

- **Milloin tämä näkyy käyttäjälle** — joka kuukausi, kun agentti pyytää lupaa sulkea kuun.
- **Miten se näkyy** — ohjelman hyväksyntäportti antaa pelkän tiivisteen, joten agentti päättää itse, mitä näyttää: yksi näyttää tuloslaskelman, toinen kysyy vain "suljetaanko maaliskuu".
- **Miksi sillä on väliä** — sulku lukitsee kirjaukset, ja käyttäjä hyväksyy sen näkemättä, mitä lukitaan.
- **Miksi tämä vaatii oman suunnittelunsa** — sisältö kuuluu ohjelman porttiin (kuten `close-year`issa jo on), ja se on prosessi- ja CLI-kerroksen muutos, ei SOULin lause. Siihen asti kuukauden skilli kertoo, mitä agentti näyttää. Kirjattu issueksi `virtually-tidy-owl`.

### F8 672 words against "about 500"

- Ei suoraa käyttäjävaikutusta — perustelu: kyse on tiedoston mitasta, ei virheestä.
- **Mistä päätös on kyse** — issue pyytää noin 500 sanaa ja roolin sellaisenaan (279 sanaa). Loppuun jäisi noin 220 sanaa neljälle paikalle, salaisuuksille, commit-säännölle, pysyville teoille ja työkalun lukemiselle. Tämän slicen ohje pyytää lisäksi, että pelisäännöt kantavat sen, kun kolme tehtäväskilliä poistaa omat kopionsa (yhteensä noin 590 sanaa). Valitsin kattavuuden: uusi osa on 396 sanaa. Jos 500 on kova raja, lyhennettävää on lähinnä `TODO.md`-kohdassa ja työkaluosan toisessa kappaleessa, mutta silloin tehtäväskillit joutuvat kantamaan osan säännöistä itse.

