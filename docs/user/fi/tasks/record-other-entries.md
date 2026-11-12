# Kirjaa myyntilasku, palkka tai muu erikoiskirjaus

Kaikkea ei kirjata tiliotteelta tai kuittireitin kautta. Myyntilaskut,
palkkatositteet, avaavat saldot ja korjaukset agentti valmistelee
tositeluonnoksena, jonka sinä hyväksyt ennen sinetöintiä.

## Ennen kuin aloitat

- Sinulla on kirjauksen lähde: myyntilasku, palkanlaskentapalvelun
  palkkalaskelma tai -yhteenveto, vanhan järjestelmän saldoluettelo tai
  korjattavan tositteen numero.
- Kuukausi, jolle kirjataan, on vielä avoin.

## Näin teet sen

1. **Pyydä agenttia ja anna lähde.**

   > Kirjaa myyntilasku 2026-0042 Asiakas Oy:lle, 2 480,00 € sis. ALV 25,5 %.
   > Lasku on kansiossa myyntilaskut.

   Palkoista:

   > Kirjaa maaliskuun palkat palkanlaskennan yhteenvedosta.

2. **Agentti valmistelee luonnoksen.** Se arkistoi lähdetiedoston, kokoaa
   tositteen viennit ja tarkistaa luonnoksen: debet ja kredit täsmäävät,
   tilit ovat tilikartassa ja liite on tallessa. Tarkistus ei tallenna mitään.

3. **Tarkista ja hyväksy.** Agentti näyttää päivän, kuvauksen, jokaisen
   viennin tilin ja summan sekä liitteen. Katso, että summat vastaavat lähdettä
   ja että myynnin ALV on omalla tilillään. Kuittireitistä poiketen ohjelma ei
   tässä laske ALV-rivejä itse, joten tarkista ne erityisen huolella. Kysy
   myös, onko ALV-vienneillä ALV-merkintä: ALV-ilmoitus lasketaan merkityistä
   vienneistä, ei tilinumeroista, joten merkitsemätön ALV puuttuisi
   ilmoituksesta ([Miten ALV-käsittely päätetään](../concepts/vat-treatment.md)).

4. **Agentti sinetöi tositteen.**

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant artifact attach --file myyntilasku-2026-0042.pdf
aggountant voucher draft < luonnos.json
aggountant voucher validate < luonnos.json
aggountant voucher seal < sinetointi.json
```
</details>

## Valmis, kun

- Tosite on sinetöity, ja agentti kertoo sen numeron.
- Myyntilasku näkyy kuukausiraportin liikevaihdossa ja myyntisaamisissa. Kun
  asiakas maksaa, tiliotteen rivi kirjataan myyntisaamisia vastaan.

## Rajoitukset

- Ohjelma ei laske myynnin ALV:tä luonnoksesta eikä tiliotteelta. Agentti
  kokoaa ALV-viennit itse.
- Rahapalkkoja ei lasketa, makseta eikä ilmoiteta aggountantilla. Palkkatosite
  kirjataan palkanlaskentapalvelun tiedoista.
- Omaa korjauskomentoa ei ole. Korjaus on tavallinen erikoiskirjaus:
  [Korjaa virheellinen tosite](correct-voucher.md).

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md).

## Katso myös

- [Korjaa virheellinen tosite](correct-voucher.md)
- [Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)
- [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) (avaavat saldot)
