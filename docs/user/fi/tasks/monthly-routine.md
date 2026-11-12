# Kuukauden kirjanpito alusta loppuun

Kuukauden kirjanpito etenee joka kuukausi samassa järjestyksessä. Tämä sivu
näyttää järjestyksen ja vie kunkin vaiheen omalle sivulle. Tavallinen kuukausi
on kirjattu, suljettu ja ilmoitettu yhden istunnon aikana, jos aineisto on
koossa.

## Ennen kuin aloitat

- Yrityksesi kirjanpito on otettu käyttöön
  ([Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md)).
- Kuukauden tiliote on saatavilla camt.053- tai camt.052-tiedostona, ja kuitit
  ovat paikassa, jonka kodin `AGENTS.md` kertoo.

## Näin teet sen

1. **Pyydä agenttia hoitamaan koko kuukausi.**

   > Kirjataan maaliskuu.

   Agentti käyttää `capture-receipt`-skilliä. Se tekee itse sen, mihin lähteet
   riittävät, ja tuo sinulle kysymykset ja hyväksyttävät tositteet koottuina,
   ei kuitti kerrallaan.

2. **Tiliote.** Agentti lukee tiliotteen, etsii kuitit ja ehdottaa jokaiselle
   tapahtumalle kirjauksen. Sinä hyväksyt listan:
   [Tuo pankin tiliote](import-bank-statement.md).

3. **Kuitit ja laskut**, joita ei kirjata tiliotteelta, kuten omalla kortilla
   maksetut ostot: [Kirjaa kuitit ja ostolaskut](record-receipts.md).

4. **Muut kirjaukset**, kuten myyntilaskut, palkkatositteet ja korjaukset:
   [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](record-other-entries.md).

5. **Avoimet asiat.** Agentti kertoo, mitkä tapahtumat jäivät avoimiksi ja
   mitä niistä puuttuu, esimerkiksi kuitti, jota ei löytynyt. Hanki puuttuva
   tai päätä, että asia kirjataan myöhemmin.

6. **Tarkistus.** Katso kuukausiraportista, että pankkitilin saldo vastaa
   tiliotteen loppusaldoa: [Katso, miten kuukausi meni](monthly-report.md).

7. **Sulku**, kun kuukausi on valmis: [Sulje kuukausi](close-month.md).

8. **ALV-ilmoitus** suljetun kuukauden luvuista:
   [OmaVerossa](file-vat-omavero.md) tai
   [suoraan Verohallinnolle](file-vat-direct.md). Maksa vero itse.

## Valmis, kun

- Kuukauden tila on `sealed` ja `vat_filed true`. Pyydä agenttia näyttämään
  kuukauden tila.
- ALV on maksettu OmaVeron viitteellä, jos maksettavaa oli.

## Katso myös

- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Harjoittele ensimmäinen kuukausi](../start/practice-month.md)
