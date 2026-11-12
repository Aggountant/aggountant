# Kirjaa kuitit ja ostolaskut

Tavallinen ostokuitti tai -lasku kirjataan omana tositteenaan, ja alkuperäinen
tiedosto liitetään siihen. Tätä reittiä käytetään erityisesti ostoihin, jotka
eivät näy yrityksen tiliotteella, kuten omalla kortilla maksettuihin.

## Ennen kuin aloitat

- Kuitti tai lasku on tiedostona (PDF, kuva tai muu), tai agentti löytää sen
  kodin `AGENTS.md`-tiedostossa kuvatusta paikasta.
- Tiedät, miten osto maksettiin: yrityksen tililtä, yrityksen kortilla vai
  omalla rahallasi.

## Näin teet sen

1. **Pyydä agenttia.**

   > Kirjaa kuitti kuitti-toimistotarvike-2026-03.pdf. Maksoin sen omalla
   > kortillani.

2. **Agentti arkistoi tiedoston.** Ohjelma tallentaa sen ja tunnistaa saman
   tiedoston myöhemmin, joten saman kuitin kahdentuminen huomataan.

3. **Agentti esikatselee kirjauksen.** Se erottaa kuitista luetut tiedot
   (summa, kuitin päivä, kuvaus) omasta ehdotuksestaan (kulutili, maksutili,
   ALV-käsittely). Ohjelma muodostaa ALV-rivit verokantataulukosta kuitin
   päivän mukaan. Esikatselu ei tallenna mitään. Jos ratkaiseva tieto puuttuu,
   agentti ei arvaa vaan kysyy: [Miten ALV-käsittely päätetään](../concepts/vat-treatment.md).

4. **Tarkista ja hyväksy.** Agentti näyttää lähdetiedot, ehdotuksen ja syntyvät
   kirjausrivit. Esimerkiksi ohjelmistolisenssi 124,00 € (sis. ALV 25,5 %)
   yrityksen tililtä maksettuna:

   | Tili | Debet | Kredit |
   |---|---|---|
   | 7690 Atk-laite- ja ohjelmistokulut | 98,80 € | |
   | 1763 Arvonlisäverosaaminen | 25,20 € | |
   | 1910 Pankkitili | | 124,00 € |

   Katso, että summa ja päivä vastaavat kuittia, kulu kuuluu yritykselle,
   tili ja ALV-käsittely ovat oikeat ja maksutili vastaa sitä, miten osto
   maksettiin. Omalla rahalla maksettu osto kirjataan yrityksen velaksi
   sinulle (2740 Muut lyhytaikaiset velat).

5. **Agentti sinetöi tositteen** juuri sellaisena kuin sen näit. Jos jokin
   muuttui välissä, ohjelma kieltäytyy, ja agentti näyttää uuden esikatselun.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant artifact attach --file kuitti.pdf
aggountant voucher list --attachment-ref <tunniste>
aggountant capture-receipt record --dry-run < kuitti.json
aggountant capture-receipt record --approved-preparation-hash <tiiviste> < kuitti.json
```
</details>

## Valmis, kun

- Ohjelma vastaa `sealed receipt voucher #N (replayed=false)`.
- Sama kuitti uudelleen kirjattuna palauttaa saman tositteen
  (`replayed=true`) eikä tee uutta.

## Kun kuittia ei löydy

Kun tiliotteella on maksu, jolle ei löydy kuittia, agentti etsii sitä kodin
`AGENTS.md`-tiedostossa kuvatuista paikoista. Jos kuitti ei löydy, agentti
nimeää tapahtuman avoimeksi ja kertoo, mitä puuttuu. Hanki kuitti myyjältä tai
verkkokaupan tililtä ja anna se agentille. aggountantin oma automaattinen
kuittihaku ei ole vielä käytössä, mutta agentti voi etsiä omilla
työkaluillaan sieltä, minne olet antanut sille pääsyn.

## Rajoitukset

- Reitti käsittelee kotimaiset ostot, käännetyn verovelvollisuuden ostot ja
  verottomat ostot. Osittaista vähennysoikeutta tai maahantuontia se ei
  käsittele.
- Poikkeuksellisen kuitin ulkopuolinen konsultointi ei ole käytössä. Agentti
  kysyy sinulta tai kirjanpitäjältäsi.
- Jos kuitti maksettiin yrityksen tililtä, sama maksu on myös tiliotteella, ja
  tiliotteen rivi on kirjattava tuonnissa. Varo kahdenkertaista kirjausta:
  [Tuo pankin tiliote](import-bank-statement.md#rajoitukset).

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#kuitit-ja-kirjaukset).

## Jos jokin menee pieleen

- **`CAPTURE_PREPARATION_CHANGED`.** Jokin muuttui esikatselun jälkeen. Agentti
  tekee uuden esikatselun, ja sinä hyväksyt uudelleen.
- **`PERIOD_CLOSING` tai `PERIOD_SEALED`.** Kuukausi on suljettu tai sulku on
  kesken. Kirjaa seuraavalle avoimelle kuukaudelle.
- Muut koodit: [Virhekoodit](../reference/error-codes.md).

## Katso myös

- [Miten ALV-käsittely päätetään](../concepts/vat-treatment.md)
- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Tuo pankin tiliote](import-bank-statement.md)
