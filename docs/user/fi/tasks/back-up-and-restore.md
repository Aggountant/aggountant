# Varmuuskopioi ja palauta kirjanpito

Kirjanpidon tärkein osa, sinetöidyt tositteet ja arkistoidut liitteet, ei ole
gitissä. Siksi koko yrityskoti varmuuskopioidaan erikseen. Tällä sivulla teet
varmuuskopion ja palautat sen tarvittaessa.

## Ennen kuin aloitat

- Mikään aggountant-komento ei ole käynnissä. Pyydä agenttia odottamaan, kunnes
  kopio on otettu.
- Sinulla on varmuuskopiolle paikka toisessa paikassa kuin alkuperäinen, esimerkiksi
  ulkoinen levy tai salattu pilvitallennus, jota vain sinä käytät.

## Ota varmuuskopio

1. **Kopioi koko kotihakemisto**, myös gitistä rajatut tiedostot:
   tositetietokanta (`fiscal.sqlite` ja sen rinnalla olevat
   `fiscal.sqlite-`-alkuiset tiedostot), liitteet (`artifacts/`) ja
   henkilötiedot (`masterdata/pii.sqlite`). Voit tehdä sen itse tai pyytää
   agenttia:

   > Ota kirjanpitokodista varmuuskopio levylle /Volumes/Varmuus.

2. **Säilytä kopio** kirjanpitolain säilytysajan. Kopiossa on henkilötietoja,
   joten pidä se vain omassa käytössäsi.

aggountantissa ei ole omaa varmuuskopio- tai vientikomentoa. Kopio on
tavallinen hakemiston kopio.

## Palauta varmuuskopio

1. **Siirrä vaurioitunut koti talteen**, älä poista sitä: siitä voi vielä olla
   apua.
2. **Kopioi varmuuskopio** kodin paikalle.
3. **Pyydä agenttia tarkistamaan tositeketju.**

   > Tarkista tositeketju.

4. **Tarkista, että kaikki on mukana.** Agentti näyttää kausien tilan ja
   viimeisen tositteen numeron. Jos kopio on vanhempi kuin viimeiset
   kirjaukset, kirjaa puuttuvat tapahtumat uudelleen lähteistään. Jos
   varmuuskopion jälkeen on lähetetty viranomaisilmoitus, kerro siitä
   agentille ennen kuin mitään lähetetään uudelleen.

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

- **Ketjun tarkistus epäonnistuu palautuksen jälkeenkin.** Kokeile vanhempaa
  kopiota: [Kun jokin menee vikaan](../troubleshooting.md#ketjun-tarkistus-epäonnistuu).

## Katso myös

- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
- [Päivitä aggountant](upgrade.md)
