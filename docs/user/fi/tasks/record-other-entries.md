# Kirjaa myyntilasku, palkka tai muu erikoiskirjaus

Myyntilaskut, palkkatositteet, avaavat saldot ja korjaukset eivät tule
tiliotteelta eivätkä kuittireitin kautta. Agentti valmistelee niistä
tositeluonnoksen, jonka hyväksyt ennen sinetöintiä.

## Ennen kuin aloitat

- Sinulla on kirjauksen lähde: myyntilasku, palkanlaskentapalvelun
  palkkalaskelma tai -yhteenveto, vanhan järjestelmän saldoluettelo tai
  korjattavan tositteen numero.
- Kuukausi, jolle kirjataan, on vielä avoin.

## Näin teet sen

1. **Pyydä agenttia ja anna lähde.**

   > Kirjaa myyntilasku 2026-0042 Asiakas Oy:lle, 2 480,00 € sis. ALV 25,5 %.
   > Lasku on kansiossa myyntilaskut.

   Palkat pyydät esimerkiksi näin:

   > Kirjaa maaliskuun palkat palkanlaskennan yhteenvedosta.

2. **Agentti valmistelee luonnoksen.** Se arkistoi lähdetiedoston, kokoaa
   tositteen viennit ja tarkistaa luonnoksen tallentamatta mitään: debet ja
   kredit täsmäävät, tilit ovat tilikartassa ja liite on tallessa. Jos
   ALV-tilin viennillä ei ole ALV-merkintää missään tositteen rivissä,
   tarkistus varoittaa koodilla `VAT_POSTING_UNTAGGED` mutta ei estä
   sinetöintiä.

3. **Tarkista ja hyväksy.** Agentti näyttää päivän, kuvauksen, jokaisen
   viennin tilin ja summan sekä liitteen. Katso, että summat vastaavat lähdettä
   ja että myynnin ALV on omalla tilillään. Kuittireitistä poiketen ohjelma ei
   tässä laske ALV-rivejä, joten tarkista ne huolella.

   Kysy myös, onko tositteella ALV-merkintä. ALV-ilmoitus lasketaan
   merkityistä vienneistä eikä tilinumeroista, joten
   [merkitsemätön ALV jää ilmoituksesta pois](../concepts/vat-treatment.md).
   Merkintä on veron perusteen rivillä, myynnissä siis myyntitilin rivillä
   (esimerkiksi `sales-domestic-standard`).

   Merkitsemätön vienti näkyy vielä
   [ALV-ilmoituksen esikatselussa ennen kuukauden sulkua](close-month.md#ennen-kuin-aloitat).
   Jos se löytyy vasta sulun jälkeen, ilmoitus on tehtävä OmaVerossa käsin
   korjatuin luvuin.

4. **Agentti sinetöi tositteen.** Kun asiakas myöhemmin maksaa laskun, agentti
   kirjaa tiliotteen rivin myyntisaamisia vastaan.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant artifact attach --file myyntilasku-2026-0042.pdf
aggountant voucher vat-codes --document-date 2026-03-15
aggountant voucher draft < luonnos.json
aggountant voucher validate < sinetointi.json
aggountant voucher seal < sinetointi.json
```

`voucher vat-codes --document-date` listaa tositteen päivänä sallitut
ALV-merkinnät.

Agentti tarkistaa komennolla `voucher validate` saman tiedoston, jonka se
sinetöi. Silloin tarkistus hylkää myös suljetulle kuukaudelle päivätyn
tositteen ja kertoo, onko tiedoston `natural_key` jo sinetöinyt tositteen. Jos
sisältö on sama, tarkistus varoittaa ja uusi sinetöinti palauttaa olemassa
olevan tositteen. Jos sisältö on eri, tarkistus hylkää tiedoston.
</details>

## Valmis, kun

- Agentti kertoo sinetöidyn tositteen numeron.
- Myyntilasku näkyy kuukausiraportin liikevaihdossa ja myyntisaamisissa.

## Rajoitukset

- Ohjelma
  [ei laske myynnin ALV:tä](../reference/status.md#pankin-tiliote)
  luonnoksesta eikä tiliotteelta.
- aggountantilla
  [ei voi laskea, maksaa eikä ilmoittaa rahapalkkoja](../reference/status.md#palkat),
  joten agentti kirjaa palkkatositteen palkanlaskentapalvelun tiedoista.
- Korjaukselle ei ole omaa komentoa. Se on tavallinen erikoiskirjaus, ja
  [Korjaa virheellinen tosite](correct-voucher.md) kertoo, miten se tehdään.

## Katso myös

- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md), jossa
  kirjataan avaavat saldot
- [Mitä on valmista ja mitä ei](../reference/status.md)
