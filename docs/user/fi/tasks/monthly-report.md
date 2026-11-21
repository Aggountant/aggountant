# Katso, miten kuukausi meni

Kuukausiraportissa ovat kuukauden tuloslaskelma, tilikauden alusta kertynyt
tulos ja kuukauden lopun tase. Raportti kertoo, miten yrityksellä menee, ja se
on paras tarkistus ennen kuukauden sulkua.

## Ennen kuin aloitat

- Kuukauden tapahtumat on kirjattu. Raportin voi tehdä myös kesken kuukauden,
  jolloin se näyttää siihen mennessä kirjatun tilanteen.
- Uuden tilikauden kuukaudelle edellinen tilikausi on sinetöity
  tilinpäätösajolla. Siihen asti raportti torjutaan, katso alta.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tee maaliskuusta kuukausiraportti HTML-muodossa.

2. **Agentti tekee raportin** sinetöidyistä tositteista ja kertoo, mihin
   tiedostoon se tallentui. Avaa tiedosto selaimessa. Selaimen
   tulostustoiminnolla saat siitä PDF:n.

3. **Tarkista raportista:**
   - **Pankkitili** on sama kuin tiliotteen loppusaldo. Jos se poikkeaa, jokin
     tapahtuma puuttuu tai on kirjattu kahdesti.
   - **Ostovelat** ovat vain vielä maksamattomien laskujen summa. Jos niihin
     jää jo maksettu summa, sama osto on todennäköisesti kirjattu sekä
     kuitista että tiliotteen riviltä kuluksi
     ([Kun osto näkyy myös tiliotteella](record-receipts.md#kun-osto-näkyy-myös-tiliotteella)).
   - **Taseen loppusummat** ovat samat kummallakin puolella.
   - **Kuukauden tulos** näyttää järkevältä. Jos jokin kulu tai tuotto on
     odottamattoman suuri, pyydä agenttia näyttämään sen tositteet.

Raportin alussa lukee "Kattavuus: varmistamaton". Ohjelma ei voi tietää, onko
kaikki aineisto kirjattu. Sen arvioit sinä ja agentti lähteitä vasten.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html
```

`--format native-json` tekee saman sisällön koneluettavana. `--history-start`
kertoo, mistä päivästä kirjanpitohistoria on aggountantissa, ja `--limitation`
lisää raporttiin huomautuksen, esimerkiksi "Avaavat saldot vanhasta
järjestelmästä". Ohjelma ei koskaan kirjoita olemassa olevan tiedoston päälle,
joten uusi raportti saa uuden nimen. Ilman `--output`-valitsinta raportti
tulostetaan tiedoston sijaan, JSON-muodossa ellei `--format html` ole annettu;
agentti käyttää tätä, kun se haluaa nopeasti katsoa kuukauden tilanteen.
</details>

## Valmis, kun

- Ohjelma vastaa `wrote html report for 2026-03 to …`, ja pankkitilin saldo
  vastaa tiliotetta.

## Jos jokin menee pieleen

- **`OUTPUT_EXISTS`.** Samanniminen tiedosto on jo olemassa. Anna uusi nimi.
- **`PERIOD_NOT_SEALED` uuden tilikauden kuukaudelle.** Tase alkaa
  edellisen tilikauden loppusaldoista, ja ne ovat lopulliset vasta, kun se
  tilikausi on sinetöity. Raportti odottaa siksi, kunnes edellisen vuoden
  tilinpäätösajo on sinetöinyt sen ([Valmistele tilinpäätös](year-end.md)).
  Kirjanpidossa ei ole vikaa: sillä välin kirjaat, suljet ja ilmoitat ALV:n
  uuden vuoden kuukausille tavalliseen tapaan; kuukauden sulku näyttää vain
  tämän vuoden tapahtumat ja sanoo sen. Kalenterivuoden yrityksessä tämä
  tulee vastaan joka tammikuu, kunnes tilinpäätösajo on tehty. Se ei koske
  ensimmäistä aggountantissa pidettävää vuotta, joka avattiin avaavilla
  saldoilla ([Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md)).
- **Pankkitilin saldo ei täsmää.** [Kun jokin menee vikaan](../troubleshooting.md#pankkitilin-saldo-ei-täsmää-tiliotteeseen)

## Katso myös

- [Sulje kuukausi](close-month.md)
- [Valmistele tilinpäätös](year-end.md)
