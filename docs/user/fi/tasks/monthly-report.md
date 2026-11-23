# Katso, miten kuukausi meni

Kuukausiraportissa ovat kuukauden tuloslaskelma, tilikauden alusta kertynyt
tulos ja kuukauden lopun tase. Tarkista siitä kuukausi ennen kuin suljet sen.

## Ennen kuin aloitat

- Kuukauden tapahtumat on kirjattu. Raportin voi tehdä myös kesken kuukauden,
  jolloin se näyttää siihen mennessä kirjatun tilanteen.
- Jos kuukausi kuuluu uuteen tilikauteen, edellinen tilikausi on sinetöity
  tilinpäätösajolla. Ennen sitä ohjelma ei tee raporttia
  ([`PERIOD_NOT_SEALED`](#jos-jokin-menee-pieleen)).

## Näin teet sen

1. **Pyydä agenttia.**

   > Tee maaliskuusta kuukausiraportti HTML-muodossa.

2. **Agentti tekee raportin** sinetöidyistä tositteista ja kertoo, mihin
   tiedostoon se tallentui. Avaa tiedosto selaimessa. Selaimen
   tulostustoiminnolla saat siitä PDF:n.

3. **Tarkista raportti.**
   - Pankkitilin saldo on sama kuin tiliotteen loppusaldo. Jos se poikkeaa,
     jokin tapahtuma puuttuu tai on kirjattu kahdesti.
   - Ostoveloissa on vain vielä maksamattomien laskujen summa. Jos niihin jää
     jo maksettu summa, sama osto on todennäköisesti kirjattu kuluksi sekä
     kuitista että tiliotteen riviltä. Katso
     [Kun osto näkyy myös tiliotteella](record-receipts.md#kun-osto-näkyy-myös-tiliotteella).
   - Taseen loppusummat ovat samat kummallakin puolella.
   - Kuukauden tulos näyttää järkevältä. Jos jokin kulu tai tuotto on
     odottamattoman suuri, pyydä agenttia näyttämään sen tositteet.

Raportin alussa lukee "Kattavuus: varmistamaton", koska ohjelma ei voi tietää,
onko kaikki aineisto kirjattu. Sen arvioit sinä ja agentti lähteitä vasten.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html
```

`--format native-json` tekee saman sisällön koneluettavana. `--history-start`
kertoo, mistä päivästä kirjanpitohistoria on aggountantissa, ja `--limitation`
lisää raporttiin huomautuksen, esimerkiksi "Avaavat saldot vanhasta
järjestelmästä".

Ohjelma ei kirjoita olemassa olevan tiedoston päälle, joten uusi raportti saa
uuden nimen. Ilman `--output`-valitsinta ohjelma tulostaa raportin eikä
tallenna sitä, JSON-muodossa ellei `--format html` ole annettu. Agentti käyttää
tätä, kun se haluaa nopeasti katsoa kuukauden tilanteen.
</details>

## Valmis, kun

- Ohjelma vastaa `wrote html report for 2026-03 to …`, ja pankkitilin saldo
  vastaa tiliotetta.

## Jos jokin menee pieleen

- `OUTPUT_EXISTS`: samanniminen tiedosto on jo olemassa. Anna uusi nimi.
- `PERIOD_NOT_SEALED` uuden tilikauden kuukaudelle: edellistä tilikautta ei
  ole vielä sinetöity. Tase alkaa edellisen tilikauden loppusaldoista, ja ne
  ovat lopulliset vasta, kun tilinpäätösajo on sinetöinyt sen tilikauden
  ([Valmistele tilinpäätös](year-end.md)). Kalenterivuoden yrityksessä tämä
  tulee vastaan joka tammikuu, kunnes tilinpäätösajo on tehty. Sillä välin
  kirjaat, suljet ja ilmoitat ALV:n uuden vuoden kuukausilta tavalliseen
  tapaan. Kuukauden sulku näyttää silloin vain tämän vuoden tapahtumat ja
  sanoo sen. Virhe ei koske ensimmäistä aggountantissa pidettävää vuotta, joka
  avattiin [avaavilla saldoilla](../start/set-up-company.md).
- Jos pankkitilin saldo ei täsmää, katso
  [Pankkitilin saldo ei täsmää tiliotteeseen](../troubleshooting.md#pankkitilin-saldo-ei-täsmää-tiliotteeseen).

## Katso myös

- [Sulje kuukausi](close-month.md)
- [Valmistele tilinpäätös](year-end.md)
