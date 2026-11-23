# Kaudet, sulku ja tilikausi

Kirjanpito suljetaan pala kerrallaan: ensin kuukausi, lopulta koko tilikausi.
Suljettuun kauteen ei voi enää kirjata, ja avoin asia voi estää sulun.

## Tilikausi ja kaudet

Tilikausi on kalenterivuosi, ellei sille määritellä muuta. Yritys, jonka
tilikausi on esimerkiksi 1.7.–30.6., määrittelee sen kerran kirjanpidon
käyttöönotossa, ja seuraavat tilikaudet jatkuvat sen mukaan. Ensimmäinen
tilikausi tai tilikausi, jona tilikautta muutetaan, voi olla 12 kuukautta
lyhyempi tai pidempi, kuitenkin enintään 18 kuukautta (kirjanpitolaki 1:4).

Tilikausi nimetään sen kalenterivuoden mukaan, jona se päättyy:
1.7.2026–30.6.2027 on tilikausi 2027. Kaksi tilikautta ei voi päättyä samana
kalenterivuonna. Jos muutat tilikautta, käytä siksi yhtä pidempää
siirtymätilikautta.

Vuosi suljetaan kausittain. Kalenterivuoden tilikausi suljetaan joko
kuukausittain (`2026-03`) tai neljännesvuosittain (`2026-Q1`), mutta samana
vuonna ei voi käyttää molempia. Kuukausi on suositeltu ja koeteltu vaihtoehto.
Kalenterivuodesta poikkeava tilikausi suljetaan aina kuukausittain.

ALV-ilmoitus tehdään samalle kaudelle, jolla kirjanpito suljetaan, eli
kuukaudelle tai neljännekselle. Suoraa ALV-ilmoittamista on koeteltu
kuukausikausilla.

## Kauden tilat

Kun agentti näyttää kauden, sen tila on rivillä toisena:

```
period "2026-03" — "sealed" (…, vat_filed false)
```

| Tila | Mitä se tarkoittaa | Voiko kirjata |
|---|---|---|
| `open` (avoin) | kautta ei ole suljettu | kyllä |
| `closing` (suljettavana) | sulku on aloitettu ja odottaa, hyväksytkö vai hylkäätkö sen | ei |
| `sealed` (suljettu) | kausi on lukittu pysyvästi | ei |

`vat_filed` kertoo, onko kauden ALV-ilmoitus tehty. ALV-ilmoituksen luvut
lasketaan vain suljetusta kaudesta, koska vasta silloin ne eivät voi enää
muuttua.

Kausi etenee tilasta `open` tilaan `closing` ja siitä tilaan `sealed`. Kun
sulku odottaa hyväksyntääsi, voit vielä hylätä sen. Hylkäys tarkoittaa, ettei
kuukausi ole vielä valmis, ja se palauttaa kauden tilaan `open`. Sulun aikana
tehdyt kirjaukset säilyvät, eikä mitään poisteta. Hyväksyttyä sulkua ei voi
perua, eikä suljettua kautta avata uudelleen.

## Miksi suljettuun kauteen ei kirjata

Kun kuukauden luvut on raportoitu ja ALV ilmoitettu, niiden on pysyttävä
samoina. Jos suljettuun kuukauteen lisätään tosite, ilmoitettu ALV ja
kirjanpito eivät enää vastaa toisiaan. Siksi myöhemmin huomattu puute tai
virhe [kirjataan seuraavalle avoimelle kaudelle](../tasks/correct-voucher.md),
kuten kirjanpidossa on aina tehty.

## Velvoitteet

Velvoite on avoin asia määräpäivineen, esimerkiksi puuttuva ajopäiväkirja.
Avoin velvoite estää kautensa sulun (`OPEN_OBLIGATION`), jotta asia ei unohdu
sulun alle. Velvoite merkitään ratkaistuksi vasta, kun asia on oikeasti
hoidettu.

Tässä versiossa velvoitteita syntyy vain poikkeuksellisten kuittien
konsultointireitin kautta, eikä sitä reittiä voi vielä käyttää. Tavallisessa
käytössä velvoitelista on siksi tyhjä.

## Tilikauden tila

Tilikauden tila on `open` tai `sealed`. Tilikauden sinetöi
[tilinpäätösajo](../tasks/year-end.md). `aggountant fiscal-year show` näyttää
tilan lisäksi tilikauden ensimmäisen ja viimeisen päivän ja pituuden
kuukausina, ja sinetöidystä tilikaudesta myös tilinpäätöslaskelmien
tarkisteen.

## Mitä tämä tarkoittaa sinulle

- Sulje kuukausi vasta, kun se on valmis: pankkitili täsmää tiliotteeseen ja
  avoimet asiat on käsitelty.
- Jos huomaat puutteen sulun alettua, hylkää sulku, kirjaa puute ja aloita
  sulku uudelleen.
- Pysy samassa kausijaossa koko vuoden.

## Katso myös

- [Sulje kuukausi](../tasks/close-month.md)
- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](filings.md)
