# Asenna aggountant ja skillit

Koneellesi asennetaan kaksi asiaa: aggountant-ohjelma ja sen mukana tulevat
skillit, eli suomenkieliset toimintaohjeet, joista agentti oppii toimimaan
kirjanpitäjänä. Asennus tehdään kerran ja uudelleen aina, kun päivität
ohjelman.

## Ennen kuin aloitat

- Mac, jossa on Applen oma suoritin (arm64), tai Linux-kone (arm64 tai
  x86_64). Näille on valmis asennuspaketti. Intel-Macilla aggountant
  käännetään lähdekoodista, ja siihen tarvitset Rust-työkalut, version 1.89
  tai uudemman, joiden asennusohje on osoitteessa <https://rustup.rs>.
  Windowsia ei ole testattu.
- Agentti, joka osaa lukea skillejä, esimerkiksi Claude Code.

## Näin teet sen

1. **Asenna ohjelma.** Voit tehdä tämän itse päätteessä tai pyytää agenttia:

   > Asenna aggountant.

   Asennusohjelma hakee GitHubista uusimman julkaisun ja valitsee siitä
   koneellesi sopivan paketin. Jos pakettia ei ole, pyydä agenttia asentamaan
   aggountant lähdekoodista. Kääntäminen kestää muutamia minuutteja.

2. **Tarkista, että ohjelma löytyy.** `aggountant --version` kertoo version.
   Ohjelma asentuu hakemistoon `~/.cargo/bin`, ja asennusohjelma lisää sen
   komentopolkuun. Jos komentoa ei löydy, avaa uusi pääte.
   `aggountant --help` listaa kaikki komennot, ja jokaisella komennolla on oma
   ohjeensa. Agentti lukee ne itse, joten sinun ei tarvitse opetella
   komentoja.

3. **Asenna skillit.** Pyydä agenttia:

   > Asenna aggountantin skillit.

   Agentti saa nämä skillit:

   | Skilli | Mihin agentti käyttää sitä |
   |---|---|
   | `aggountant-init` | yrityskodin käyttöönotto tai keskeneräisen käyttöönoton jatkaminen |
   | `migrate-books` | kirjanpidon siirto toisesta ohjelmasta: vanhan aineiston vienti, tarkistus ja arkistointi |
   | `capture-receipt` | kuukauden kirjanpito: kuitit, tiliote, erikoiskirjaukset, täsmäytys ja kuukauden sulku |
   | `vero-fi` | ALV-ilmoitus Verohallinnolle ja suoran ALV-ilmoittamisen käyttöönotto |
   | `tulorekisteri` | Tulorekisterin matkakorvausilmoitus ja työnantajan erillisilmoitus |
   | `close-year` | tilikauden päättäminen: tilinpäätöksen kirjaukset, hyväksynnät ja tilikauden sinetöinti |
   | `e-services` | OmaVeron ja Tulorekisterin asiointipalvelun käyttö selaimessasi: miten sivut toimivat, päivättyinä havaintoina |

   Oletuksena skillit menevät Claude Coden skillihakemistoon
   `~/.claude/skills/`. Muuta agenttia varten anna sen skillihakemisto
   valitsimella `--target`. Skillit on kirjoitettu suomeksi, mutta voit puhua
   agentille myös englanniksi.

4. **Käynnistä agentti uudelleen**, jotta se lukee uudet skillit.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill list
aggountant skill install
aggountant skill install --target <agentin skillihakemisto>
aggountant skill print capture-receipt
aggountant skill print aggountant-init --file SOUL.md
```

Ensimmäinen komento asentaa valmiin paketin.

`cargo install` kääntää ohjelman lähdekoodista, ja sitä tarvitaan vain, kun
valmista pakettia ei ole. Jos lähdekoodi on jo koneellasi, sama tehdään sen
juuresta komennolla `cargo install --locked --path crates/cli`.

Kun asennat pelkän `capture-receipt`-skillin, mukaan tulee myös
`aggountant-init`, koska niillä on yhteinen roolikuvaus. `vero-fi` ja
`tulorekisteri` tuovat lisäksi toisensa ja `e-services`-skillin, jonka
varassa ne tekevät viranomaisten verkkopalveluissa tehtävät vaiheet.

`skill print` näyttää skillin sisällön asentamatta mitään. `--file` näyttää
skillin muut tiedostot, kuten yhteisen roolikuvauksen `SOUL.md`.
</details>

## Valmis, kun

- `aggountant --version` näyttää versionumeron.
- `aggountant skill list` listaa seitsemän skilliä, ja ne löytyvät agentin
  skillihakemistosta.

## Seuraavaksi

[Harjoittele ensimmäinen kuukausi](practice-month.md) erillisessä
harjoituskodissa, tai jos olet jo tehnyt sen,
[ota yrityksesi kirjanpito käyttöön](set-up-company.md).

## Katso myös

- [Päivitä aggountant](../tasks/upgrade.md)
- [Sinä, agentti ja aggountant](../concepts/division-of-work.md)
