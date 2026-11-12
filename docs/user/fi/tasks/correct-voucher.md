# Korjaa virheellinen tosite

Sinetöityä tositetta ei muokata eikä poisteta. Virhe korjataan uudella
korjaustositteella, joka viittaa alkuperäiseen, ja molemmat jäävät
kirjanpitoon. Kirjanpitolaki edellyttää juuri tätä.

## Ennen kuin aloitat

- Tiedät, mikä tosite on väärin. Jos et tiedä numeroa, pyydä agenttia
  etsimään tosite päivän, summan tai liitteen perusteella.
- Korjaus kirjataan avoimelle kuukaudelle. Jos virheellisen tositteen kuukausi
  on jo suljettu, korjaus menee seuraavalle avoimelle kuukaudelle.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tosite 12 on kirjattu toimistokuluihin, mutta se on palvelinvuokra.
   > Korjaa se.

2. **Agentti valmistelee korjaustositteen** erikoiskirjauksena ja viittaa sen
   kuvauksessa korjattavaan tositteeseen. Korjauksen muoto riippuu virheestä:
   - **väärä tili tai ALV-käsittely**: tosite, joka siirtää summan oikealle
     tilille
   - **kahdesti kirjattu tai yritykselle kuulumaton tapahtuma**: vastakirjaus,
     joka kumoaa alkuperäisen
   - **väärä summa**: erotuksen kirjaus.

3. **Tarkista ja hyväksy.** Katso, että korjaus viittaa oikeaan tositteeseen,
   että sen jälkeen summa on oikealla tilillä ja että ALV-viennit korjautuvat
   samalla. Jos ALV-käsittely muuttuu, korjaus vaikuttaa sen kuukauden
   ALV-ilmoitukseen, jolle korjaus kirjataan.

4. **Agentti sinetöi korjauksen.**

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant voucher show 12
aggountant voucher draft < korjaus.json
aggountant voucher validate < korjaus.json
aggountant voucher seal < korjaus-sinetointi.json
```

`voucher show` näyttää ilman `--json`-valitsinta vain tositteen numeron ja
tunnisteen. Pyydä siksi agenttia näyttämään tositteen viennit.
</details>

## Valmis, kun

- Korjaustosite on sinetöity, ja sen kuvaus kertoo, mitä tositetta se korjaa.
- Kuukausiraportissa summa on oikealla tilillä.

## Rajoitukset

- Omaa korjauskomentoa ei ole. Korjaus on tavallinen erikoiskirjaus.
- Jos virhe koskee jo ilmoitettua ALV-kuukautta ja haluat korjata sen
  korvaavalla ilmoituksella, tee se OmaVerossa. aggountant ei tee korvaavia
  ilmoituksia.

## Katso myös

- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](record-other-entries.md)
- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
