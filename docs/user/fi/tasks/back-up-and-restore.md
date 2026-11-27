# Varmuuskopioi ja palauta kirjanpito

Kirjanpidon tärkein osa, sinetöidyt tositteet ja arkistoidut liitteet, ei ole
gitissä, joten koko yrityskoti varmuuskopioidaan erikseen. Varmuuskopio on
tavallinen kopio kotihakemistosta, koska aggountantissa
[ei ole varmuuskopio- tai vientikomentoa](../reference/status.md#yrityskoti-ja-käyttöönotto).

## Ennen kuin aloitat

- Mikään aggountant-komento ei ole käynnissä, koska kesken kirjoituksen
  otettu kopio tositetietokannasta voi olla rikki. Pyydä agenttia odottamaan,
  kunnes kopio on otettu.
- Kopiolle on paikka muualla kuin siellä, missä koti on, esimerkiksi ulkoinen
  levy tai salattu pilvitallennus, jota vain sinä käytät.

## Ota varmuuskopio

Ota kopio ainakin jokaisen kuukauden sulun jälkeen ja
[ennen jokaista päivitystä](upgrade.md).

1. **Kopioi koko kotihakemisto** itse tai pyydä agenttia:

   > Ota kirjanpitokodista varmuuskopio levylle /Volumes/Varmuus.

   Kopioon kuuluvat myös gitistä rajatut tiedostot: tositetietokanta
   (`fiscal.sqlite` ja sen rinnalla olevat `fiscal.sqlite-`-alkuiset
   tiedostot), liitteet (`artifacts/`), henkilötiedot
   (`masterdata/pii.sqlite`) ja kululaskujen luonnokset
   (`intake/expense-claims/`).

2. **Säilytä kopiot** kirjanpitolain
   [säilytysajan](../start/move-from-another-program.md#mitä-kerätään):
   tilinpäätös ja kirjanpidot vähintään 10 vuotta ja tositteet vähintään
   kuusi vuotta. Kopiossa on henkilötietoja, joten pidä se vain omassa
   käytössäsi.

## Palauta varmuuskopio

1. **Siirrä vaurioitunut koti talteen.** Älä poista sitä, koska siitä voi
   vielä olla apua.
2. **Kopioi varmuuskopio** kodin paikalle.
3. **Pyydä agenttia tarkistamaan tositeketju.**

   > Tarkista tositeketju.

4. **Tarkista, että kaikki on mukana.** Agentti varmistaa ensin, että
   käytössä on palautettu koti, ja näyttää sitten kausien tilan ja viimeisen
   tositteen numeron. Jos kopio on vanhempi kuin viimeiset kirjaukset, kirjaa
   puuttuvat tapahtumat uudelleen [kuiteista](record-receipts.md) ja
   [tiliotteista](import-bank-statement.md). Jos
   varmuuskopion jälkeen on lähetetty viranomaisilmoitus, kerro siitä
   agentille, ennen kuin se lähettää mitään uudelleen.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant voucher verify
aggountant period show --period 2026-03
aggountant config show
```
</details>

## Valmis, kun

`aggountant voucher verify` vastaa esimerkiksi:

```
chain ok — 42 vouchers, head #42
```

## Jos jokin menee pieleen

- Jos [ketjun tarkistus epäonnistuu](../troubleshooting.md#ketjun-tarkistus-epäonnistuu)
  palautuksen jälkeenkin, kokeile vanhempaa kopiota.

## Katso myös

- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
- [Päivitä aggountant](upgrade.md)
