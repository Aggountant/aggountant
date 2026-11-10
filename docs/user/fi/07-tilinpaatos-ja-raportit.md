# 7. Tilinpäätös, raportit ja tilikauden tila

## Kuukausiraportti

Kuukausiraportissa ovat kuukauden tuloslaskelma, tilikauden alusta kertynyt
tulos ja kuukauden lopun tase:

```sh
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html
```

- `--format html` tekee selaimessa avattavan raportin, jonka voi tulostaa
  PDF:ksi selaimen tulostustoiminnolla. `--format native-json` tekee saman
  sisällön koneluettavana.
- Ohjelma ei koskaan kirjoita olemassa olevan tiedoston päälle
  (`OUTPUT_EXISTS`). Anna uudelle raportille uusi nimi.
- Raportti lasketaan sinetöidyistä tositteista. Se voidaan tehdä myös
  kuukaudelle, jota ei ole vielä suljettu. Silloin se näyttää tähän mennessä
  kirjatun tilanteen.
- Raportissa lukee aina "completeness unverified". Ohjelma ei voi tietää, onko
  kaikki aineisto kirjattu. Sen arvioit sinä ja agentti lähteitä vasten.
  Valitsimella `--history-start` voit kertoa, mistä päivästä kirjanpitohistoria
  on aggountantissa, ja valitsimella `--limitation` voit lisätä raporttiin
  rajoitusta koskevan huomautuksen, esimerkiksi "Avaavat saldot vanhasta
  järjestelmästä".

Raportti on myös hyvä tarkistus ennen kauden sulkua: pankkitilin saldon pitää
vastata tiliotteen loppusaldoa ja taseen pitää täsmätä.

## Kausien ja tilikauden tila

```sh
aggountant period show --period 2026-03
aggountant fiscal-year show --year 2026
```

Kauden tila on `open`, `closing` tai `sealed` (luku 4), ja tulosteesta näkyy, onko
kauden ALV-ilmoitus tehty. Tilikauden tila on `open` tai `sealed`. Sinetöidyllä
tilikaudella on myös tilinpäätöslaskelmien tarkiste.

Tositeketjun eheyden voi tarkistaa milloin tahansa:

```sh
aggountant voucher verify
```

Yksittäisen tositteen näet numerolla (`aggountant voucher show 12`). Tositteita
voi hakea kirjauspäivän tai liitteen mukaan (`aggountant voucher list
--posting-from 2026-03-01 --posting-to 2026-03-31`).

## Tilinpäätös

Ohjelmassa on tilinpäätösajo (`close-year`). Kun vuoden kaikki kuukaudet on
suljettu, se laskee poistot ja jaksotukset, muodostaa tuloslaskelman ja taseen,
käy läpi neljä hyväksyntää (oikaisukirjaukset, tase-erittelyt, liitetiedot ja
laskelmat) ja sinetöi lopuksi tilikauden.

**Älä vielä tee oikeaa tilinpäätöstä tällä ajolla.** Tässä versiossa siinä on
kaksi puutetta, jotka tekevät tuloksesta väärän tai hyväksynnästä sokean:

- **Vuoden lopun poistot ja jaksotukset puuttuvat suljettavan vuoden
  laskelmista.** Ajo kirjaa ne seuraavan vuoden ensimmäiselle avoimelle kaudelle,
  koska vuoden kuukaudet on jo suljettu. Niitä ei kuitenkaan vielä kohdisteta
  takaisin suljettavan vuoden tilinpäätökseen. Tuloslaskelma ja tase eivät siis
  sisällä vuoden poistoja.
- **Hyväksyntävaiheet eivät näytä laskelmia.** Komennon tuloste kertoo vain
  vaiheiden tilan. Hyväksyisit siis tuloslaskelman ja taseen näkemättä niitä.

Tilikauden sinetöinti on peruuttamaton. Tee tilinpäätös toistaiseksi
tilitoimiston kanssa tai käsin. Kuukausiraporteista ja sinetöidyistä tositteista
saat sille aineiston. Myös virallisen tilinpäätösasiakirjan laatiminen ja
allekirjoitus, ilmoitus kaupparekisteriin ja yhtiön veroilmoitus ovat
aggountantin ulkopuolisia tehtäviä.
