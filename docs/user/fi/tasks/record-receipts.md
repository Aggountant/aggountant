# Kirjaa kuitit ja ostolaskut

Tavallinen ostokuitti tai -lasku kirjataan omana tositteenaan, ja alkuperäinen
tiedosto liitetään siihen. Kuitti kirjaa kulun ja sen ALV:n. Jos sama osto
maksetaan yrityksen tililtä, maksu kirjataan myöhemmin tiliotteelta, joten
kuitti kirjataan velaksi, jonka tiliotteen rivi kuittaa: [Kun osto näkyy myös
tiliotteella](#kun-osto-näkyy-myös-tiliotteella).

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
   kirjausrivit. Esimerkiksi ohjelmistolisenssi 124,00 € (sis. ALV 25,5 %),
   joka maksetaan yrityksen tililtä:

   | Tili | Debet | Kredit |
   |---|---|---|
   | 7690 Atk-laite- ja ohjelmistokulut | 98,80 € | |
   | 1763 Arvonlisäverosaaminen | 25,20 € | |
   | 2700 Ostovelat | | 124,00 € |

   Katso, että summa ja päivä vastaavat kuittia, kulu kuuluu yritykselle,
   tili ja ALV-käsittely ovat oikeat ja vastatili vastaa sitä, miten osto
   maksettiin:

   | Miten osto maksettiin | Vastatili |
   |---|---|
   | yrityksen tililtä tai yrityksen kortilla | 2700 Ostovelat |
   | omalla rahallasi | 2740 Muut lyhytaikaiset velat (yrityksen velka sinulle) |
   | käteisellä yrityksen kassasta | 1900 Käteisvarat |

   Pankkitili 1910 ei ole kuitin vastatili. Pankkitilille kirjataan vain
   tiliotteen rivit, jotta sen saldo vastaa tiliotetta.

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

## Kun osto näkyy myös tiliotteella

Yrityksen tililtä tai kortilla maksettu osto näkyy kahdessa lähteessä:
kuitissa ja tiliotteella. Kumpikin kirjataan kerran, ja kumpikin kirjaa eri
asian:

- **Kuitti kirjaa kulun.** Se kirjataan kulutilille ja ALV:lle ostovelkaa
  2700 vastaan, ja kuitti on tositteen liitteenä.
- **Tiliotteen rivi kirjaa maksun.** Tiliotteen tuonnissa rivi kirjataan
  ostovelkaa 2700 vastaan ilman ALV:tä. Se kuittaa kuitin velan.

Jos maksoit oston omalla rahallasi ja yritys maksaa sen sinulle takaisin,
toimitaan samoin tilillä 2740: kuitti kirjataan velaksi sinulle, ja
tiliotteen takaisinmaksurivi kuittaa sen.

Esimerkiksi edellä kirjattu 124,00 euron ohjelmistolisenssi maksetaan
tililtä kolme päivää myöhemmin. Tiliotteen rivi kirjataan näin:

| Tili | Debet | Kredit |
|---|---|---|
| 2700 Ostovelat | 124,00 € | |
| 1910 Pankkitili | | 124,00 € |

Kahden tositteen jälkeen kulu ja ALV-saaminen ovat kirjanpidossa kerran,
pankkitili on vähentynyt maksun verran ja ostovelan saldo on palannut
nollaan. Järjestyksellä ei ole väliä: jos tiliote tuodaan ennen kuin kuitti
on löytynyt, rivi kirjataan silti tilille 2700, ja myöhemmin kirjattu kuitti
kuittaa sen.

Kuun lopussa tarkistetaan kaksi asiaa
([Katso, miten kuukausi meni](monthly-report.md)):

- pankkitilin saldo vastaa tiliotteen loppusaldoa, ja
- ostovelkojen saldo on vain vielä maksamattomien laskujen summa. Jos sama
  osto on kirjattu tiliotteen riviltä kuluksi ja lisäksi kuitista
  ostovelaksi, pankkitilin saldo täsmää, mutta ostovelkaan jää kuitin summa.

Harjoituskuukausi käy tämän läpi oikealla ohjelmalla:
[Harjoittele ensimmäinen kuukausi](../start/practice-month.md).

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
- Ohjelma ei yhdistä kuittia ja tiliotteen riviä toisiinsa. Ne kohtaavat
  tilillä 2700 tai 2740, ja tilin saldo kertoo, jäikö jotain kuittaamatta.

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
