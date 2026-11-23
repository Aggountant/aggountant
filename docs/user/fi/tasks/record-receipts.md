# Kirjaa kuitit ja ostolaskut

Tavallinen ostokuitti tai -lasku kirjataan omana tositteenaan, ja alkuperäinen
tiedosto liitetään siihen. Kuitti kirjaa kulun ja sen ALV:n. Jos osto maksetaan
yrityksen tililtä, maksu kirjataan myöhemmin tiliotteelta. Siksi kuitti
kirjataan velaksi, jonka
[tiliotteen rivi kuittaa](#kun-osto-näkyy-myös-tiliotteella).

## Ennen kuin aloitat

- Kuitti tai lasku on tiedostona (PDF, kuva tai muu), tai agentti löytää sen
  kodin `AGENTS.md`-tiedostossa kuvatusta paikasta.
- Tiedät, miten osto maksettiin: yrityksen tililtä, yrityksen kortilla vai
  omalla rahallasi.

## Näin teet sen

1. **Pyydä agenttia.**

   > Kirjaa lasku lasku-ohjelmistolisenssi-2026-03.pdf. Se maksetaan
   > yrityksen tililtä.

2. **Agentti arkistoi tiedoston.** Ohjelma tunnistaa saman tiedoston
   myöhemmin, joten agentti huomaa, jos kuitti on jo kirjattu.

3. **Agentti esikatselee kirjauksen.** Se pitää kuitista luetut tiedot
   (summa, kuitin päivä, kuvaus) erillään omasta ehdotuksestaan (kulutili,
   vastatili, ALV-käsittely). Ohjelma muodostaa ALV-rivit kuitin päivänä
   voimassa olleella verokannalla. Esikatselu ei tallenna mitään. Jos
   [ALV-käsittelyn](../concepts/vat-treatment.md) ratkaiseva tieto puuttuu,
   agentti kysyy sen sinulta.

4. **Tarkista ja hyväksy.** Agentti näyttää kuitista luetut tiedot,
   ehdotuksensa ja syntyvät kirjausrivit. Esimerkiksi ohjelmistolisenssi
   124,00 € (sis. ALV 25,5 %), joka maksetaan yrityksen tililtä:

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

   Pankkitili 1910 ei ole kuitin vastatili, koska pankkitilille kirjataan
   vain tiliotteen rivit. Näin sen saldo vastaa tiliotetta.

5. **Agentti sinetöi tositteen** juuri sellaisena kuin sen näit. Jos jokin
   muuttui välissä, ohjelma kieltäytyy, ja agentti näyttää uuden esikatselun.
   Jos sama kuitti kirjataan uudelleen, ohjelma palauttaa saman tositteen
   (`replayed=true`) eikä tee uutta.

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

Ohjelma vastaa `sealed receipt voucher #N (replayed=false)`.

## Kun osto näkyy myös tiliotteella

Yrityksen tililtä tai kortilla maksettu osto näkyy sekä kuitissa että
tiliotteella, ja kumpikin kirjataan kerran. Kuitista kirjataan kulu: summa
menee kulutilille ja ALV:lle ostovelkaa 2700 vastaan, ja kuitti on tositteen
liitteenä. Tiliotteen rivistä kirjataan maksu: tiliotteen tuonnissa rivi
kirjataan ostovelkaa 2700 vastaan ilman ALV:tä, ja se kuittaa kuitin velan.

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

Kun [katsot kuun lopussa, miten kuukausi meni](monthly-report.md), tarkista,
että pankkitilin saldo vastaa tiliotteen loppusaldoa ja että ostovelkojen
saldo on vain vielä maksamattomien laskujen summa. Jos sama osto on kirjattu
tiliotteen riviltä kuluksi ja lisäksi kuitista ostovelaksi, pankkitilin saldo
täsmää, mutta ostovelkaan jää kuitin summa.

[Harjoituskuukaudessa](../start/practice-month.md) käyt tämän läpi oikealla
ohjelmalla.

## Kun kuittia ei löydy

Kun tiliotteella on maksu, jolle ei löydy kuittia, agentti etsii sitä kodin
`AGENTS.md`-tiedostossa kuvatuista paikoista. aggountantin oma automaattinen
kuittihaku ei ole vielä käytössä, mutta agentti voi etsiä omilla
työkaluillaan sieltä, minne olet antanut sille pääsyn. Jos kuitti ei löydy,
agentti nimeää tapahtuman avoimeksi ja kertoo, mitä puuttuu. Hanki silloin
kuitti myyjältä tai verkkokaupan tililtä ja anna se agentille.

## Rajoitukset

- Näin kirjataan kotimaiset ostot, käännetyn verovelvollisuuden ostot ja
  verottomat ostot. Osittaista vähennysoikeutta tai maahantuontia ohjelma ei
  tässä käsittele.
- Poikkeuksellisen kuitin ulkopuolinen konsultointi ei ole käytössä. Agentti
  kysyy sinulta tai kirjanpitäjältäsi.
- Ohjelma ei yhdistä kuittia ja tiliotteen riviä toisiinsa. Ne kohtaavat
  tilillä 2700 tai 2740, ja tilin saldo kertoo, jäikö jotain kuittaamatta.

Rajoitukset on kuvattu tarkemmin sivulla
[Mitä on valmista ja mitä ei](../reference/status.md#kuitit-ja-kirjaukset).

## Jos jokin menee pieleen

- `CAPTURE_PREPARATION_CHANGED`: jokin muuttui esikatselun jälkeen. Agentti
  tekee uuden esikatselun, ja sinä hyväksyt uudelleen.
- `PERIOD_CLOSING`: kuukauden sulku odottaa hyväksyntääsi. Jos kuitti kuuluu
  tälle kuukaudelle, [hylkää sulku](close-month.md) ja kirjaa kuitti. Muuten
  kirjaa se seuraavalle avoimelle kuukaudelle.
- `PERIOD_SEALED`: kuukausi on suljettu. Kirjaa seuraavalle avoimelle
  kuukaudelle.
- Muut koodit selitetään sivulla [Virhekoodit](../reference/error-codes.md).

## Katso myös

- [Miten ALV-käsittely päätetään](../concepts/vat-treatment.md)
- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Tuo pankin tiliote](import-bank-statement.md)
