# Korjaa virheellinen tosite

Sinetöityä tositetta ei muokata eikä poisteta. Virhe korjataan uudella
tositteella, joka viittaa alkuperäiseen, ja molemmat jäävät kirjanpitoon,
kuten kirjanpitolaki edellyttää.

## Ennen kuin aloitat

- Tiedät, mikä tosite on väärin. Jos et tiedä numeroa, pyydä agenttia
  etsimään tosite päivän, summan tai liitteen perusteella.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tosite 12 on kirjattu toimistokuluihin, mutta se on palvelinvuokra.
   > Korjaa se.

2. **Agentti valmistelee korjaustositteen** erikoiskirjauksena ja viittaa sen
   kuvauksessa korjattavaan tositteeseen. Korjaus kirjataan avoimelle
   kuukaudelle. Jos virheellisen tositteen kuukausi on jo suljettu, korjaus
   menee seuraavalle avoimelle kuukaudelle. Korjauksen muoto riippuu
   virheestä:
   - Jos tili tai ALV-käsittely on väärä, agentti siirtää summan oikealle
     tilille.
   - Jos tapahtuma on kirjattu kahdesti tai se ei kuulu yritykselle, agentti
     kumoaa alkuperäisen vastakirjauksella.
   - Jos summa on väärä, agentti kirjaa erotuksen.

3. **Tarkista ja hyväksy.** Katso, että korjaus viittaa oikeaan tositteeseen,
   että sen jälkeen summa on oikealla tilillä ja että ALV-viennit korjautuvat
   samalla. Jos ALV-käsittely muuttuu, korjaus vaikuttaa sen kuukauden
   ALV-ilmoitukseen, jolle korjaus kirjataan.

   Kirjanpidon korjaus ei korjaa jo annettua ALV-ilmoitusta. Jos virhe koskee
   kuukautta, jonka ALV-ilmoitus on annettu, kerro se agentille ennen
   sinetöintiä, koska agentti merkitsee sen tositteeseen eikä merkintää voi
   lisätä jälkikäteen. Anna sitten korvaava ilmoitus kohdan
   [Korjaa jo annettu ilmoitus](file-vat-direct.md#korjaa-jo-annettu-ilmoitus)
   mukaan.

4. **Agentti sinetöi korjauksen.**

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant voucher show 12
aggountant voucher draft < korjaus.json
aggountant voucher validate < korjaus-sinetointi.json
aggountant voucher seal < korjaus-sinetointi.json
```

Pyydä agenttia näyttämään tositteen viennit, koska `voucher show` näyttää
ilman `--json`-valitsinta vain tositteen numeron ja tunnisteen.
</details>

## Valmis, kun

- Korjaustosite on sinetöity, ja sen kuvaus kertoo, mitä tositetta se korjaa.
- Kuukausiraportissa summa on oikealla tilillä.

## Rajoitukset

- Omaa korjauskomentoa ei ole, joten agentti kirjaa korjauksen tavallisena
  erikoiskirjauksena.

## Katso myös

- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](record-other-entries.md)
- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
