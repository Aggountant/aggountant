# Katso, miten kuukausi meni

Kuukausiraportissa ovat kuukauden tuloslaskelma, tilikauden alusta kertynyt
tulos ja kuukauden lopun tase. Raportti kertoo, miten yrityksellä menee, ja se
on paras tarkistus ennen kuukauden sulkua.

## Ennen kuin aloitat

- Kuukauden tapahtumat on kirjattu. Raportin voi tehdä myös kesken kuukauden,
  jolloin se näyttää siihen mennessä kirjatun tilanteen.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tee maaliskuusta kuukausiraportti HTML-muodossa.

2. **Agentti tekee raportin** sinetöidyistä tositteista ja kertoo, mihin
   tiedostoon se tallentui. Avaa tiedosto selaimessa. Selaimen
   tulostustoiminnolla saat siitä PDF:n.

3. **Tarkista raportista:**
   - **Pankkitili** on sama kuin tiliotteen loppusaldo. Jos se poikkeaa, jokin
     tapahtuma puuttuu tai on kirjattu kahdesti.
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
joten uusi raportti saa uuden nimen.
</details>

## Valmis, kun

- Ohjelma vastaa `wrote html report for 2026-03 to …`, ja pankkitilin saldo
  vastaa tiliotetta.

## Jos jokin menee pieleen

- **`OUTPUT_EXISTS`.** Samanniminen tiedosto on jo olemassa. Anna uusi nimi.
- **Pankkitilin saldo ei täsmää.** [Kun jokin menee vikaan](../troubleshooting.md#pankkitilin-saldo-ei-täsmää-tiliotteeseen)

## Katso myös

- [Sulje kuukausi](close-month.md)
- [Valmistele tilinpäätös](year-end.md)
