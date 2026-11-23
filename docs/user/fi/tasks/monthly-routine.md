# Kuukauden kirjanpito alusta loppuun

Kuukauden kirjanpito etenee joka kuukausi samassa järjestyksessä, ja
jokaisella vaiheella on oma sivunsa. Jos aineisto on koossa, tavallinen
kuukausi on kirjattu, suljettu ja ilmoitettu yhdessä istunnossa. Jos yritys
sulkee kirjanpidon [neljännesvuosittain](../concepts/periods.md#tilikausi-ja-kaudet),
sulku ja ALV-ilmoitus (vaiheet 7 ja 8) tehdään neljänneksen viimeisen
kuukauden jälkeen.

## Ennen kuin aloitat

- Yrityksesi kirjanpito on
  [otettu käyttöön](../start/set-up-company.md).
- Kuukauden tiliote on saatavilla camt.053- tai camt.052-tiedostona, ja kuitit
  ovat paikassa, jonka kodin `AGENTS.md` kertoo.

## Näin teet sen

1. **Pyydä agenttia hoitamaan koko kuukausi.**

   > Kirjataan maaliskuu.

   Agentti käyttää `capture-receipt`-skilliä. Se tekee itse sen, mihin lähteet
   riittävät, ja tuo kysymykset ja hyväksyttävät tositteet sinulle koottuina
   eikä kuitti kerrallaan.

2. **Agentti kirjaa kuitit ja ostolaskut**, jokaisen
   [omana tositteenaan](record-receipts.md). Yrityksen tililtä maksettu osto
   kirjataan ostovelaksi ja omalla rahallasi maksettu velaksi sinulle.

3. **Agentti lukee tiliotteen**, etsii kuitit ja
   [ehdottaa jokaiselle tapahtumalle kirjauksen](import-bank-statement.md).
   Kuitin maksurivi kuittaa kuitin ostovelan. Sinä hyväksyt listan.

4. **Agentti kirjaa muut tositteet**, kuten
   [myyntilaskut, palkkatositteet ja korjaukset](record-other-entries.md).

5. **Käy läpi avoimet asiat.** Agentti kertoo, mitkä tapahtumat jäivät
   avoimiksi ja mitä niistä puuttuu, esimerkiksi kuitti, jota ei löytynyt.
   Hanki puuttuva tai päätä, että asia kirjataan myöhemmin. Agentti poistaa
   [kodin `TODO.md`-tiedostosta](../concepts/company-home.md#kodin-todomd)
   kuukauden aikana ratkenneet kohdat ja lisää sen, mikä jäi odottamaan,
   joten seuraava kuukausi alkaa siitä.

6. **Tarkista kuukausi.** Katso [kuukausiraportista](monthly-report.md), että
   pankkitilin saldo on sama kuin tiliotteen loppusaldo ja että ostovelkoina
   on vain maksamattomia laskuja.

7. **Pyydä agenttia [sulkemaan kuukausi](close-month.md)**, kun se on valmis.

8. **Tee ALV-ilmoitus** suljetun kuukauden luvuista joko
   [OmaVerossa](file-vat-omavero.md) tai
   [suoraan Verohallinnolle](file-vat-direct.md). Maksa vero itse.

9. **Anna [työnantajan erillisilmoitus](file-employer-report.md)
   Tulorekisteriin**, jos yritys on työnantajarekisterissä. Se annetaan joka
   kuukausi, myös ilman maksuja.

## Valmis, kun

- Kuukauden tila on `sealed` ja `vat_filed true`. Pyydä agenttia näyttämään
  kuukauden tila.
- ALV on maksettu OmaVeron viitteellä, jos maksettavaa oli.

## Katso myös

- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Harjoittele ensimmäinen kuukausi](../start/practice-month.md)
