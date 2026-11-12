# Asenna aggountant ja skillit

Ennen kuin agentti voi pitää kirjanpitoa, koneellesi asennetaan kaksi asiaa:
aggountant-ohjelma ja sen mukana tulevat skillit, eli suomenkieliset
toimintaohjeet, joista agentti oppii toimimaan kirjanpitäjänä. Asennus tehdään
kerran ja uudelleen aina, kun päivität ohjelman.

## Ennen kuin aloitat

- Rust-työkalut, versio 1.89 tai uudempi. Asennusohje on osoitteessa
  <https://rustup.rs>. Valmiita asennuspaketteja ei vielä ole, joten
  aggountant käännetään lähdekoodista.
- Agentti, joka osaa lukea skillejä, esimerkiksi Claude Code.

## Näin teet sen

1. **Asenna ohjelma.** Tämän vaiheen voit tehdä itse päätteessä tai pyytää
   agenttia tekemään sen:

   > Asenna aggountant lähdekoodista GitHubista.

   Kääntäminen kestää muutamia minuutteja.

2. **Tarkista, että ohjelma löytyy.** `aggountant --version` kertoo version.
   `aggountant --help` listaa kaikki komennot, ja jokaisella komennolla on oma
   ohjeensa. Agentti lukee näitä ohjeita itse, joten sinun ei tarvitse
   opetella komentoja.

3. **Asenna skillit.** Pyydä agenttia:

   > Asenna aggountantin skillit.

   Skillejä on kolme:

   | Skilli | Mihin agentti käyttää sitä |
   |---|---|
   | `aggountant-init` | yrityskodin käyttöönotto tai keskeneräisen käyttöönoton jatkaminen |
   | `capture-receipt` | kuittien, tositteiden ja kuukauden tapahtumien käsittely |
   | `vero-fi` | suoran ALV-ilmoittamisen käyttöönotto, ALV-ilmoitus ja Tulorekisterin matkakorvaukset |

   Oletuksena skillit menevät Claude Coden skillihakemistoon
   `~/.claude/skills/`. Muuta agenttia varten anna sen skillihakemisto
   valitsimella `--target`.

4. **Käynnistä agentti uudelleen**, jotta se lukee uudet skillit.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill list
aggountant skill install
aggountant skill install --target <agentin skillihakemisto>
aggountant skill print capture-receipt
```

Jos lähdekoodi on jo koneellasi, asennus tehdään sen juuresta komennolla
`cargo install --locked --path crates/cli`. `skill print` näyttää skillin
sisällön asentamatta mitään. Kun asennat pelkän `capture-receipt`-skillin,
mukaan tulee myös `aggountant-init`, koska niillä on yhteinen roolikuvaus.
</details>

## Valmis, kun

- `aggountant --version` näyttää version, esimerkiksi `aggountant 0.1.0`.
- `aggountant skill list` listaa kolme skilliä, ja ne löytyvät agentin
  skillihakemistosta.

Skillit on kirjoitettu suomeksi, mutta voit puhua agentille myös englanniksi.

## Seuraavaksi

[Harjoittele ensimmäinen kuukausi](practice-month.md) erillisessä
harjoituskodissa, tai jos olet jo tehnyt sen,
[ota yrityksesi kirjanpito käyttöön](set-up-company.md).

## Katso myös

- [Päivitä aggountant](../tasks/upgrade.md)
- [Sinä, agentti ja aggountant](../concepts/division-of-work.md)
