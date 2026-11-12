# Kaudet, sulku ja tilikausi

Kirjanpito suljetaan pala kerrallaan: ensin kuukausi, lopulta koko tilikausi.
Suljettuun kauteen ei voi enää kirjata. Tämä sivu kertoo, mitä kaudet ovat,
missä tiloissa ne voivat olla ja miksi avoin asia voi estää sulun.

## Tilikausi ja kaudet

aggountant käsittelee tilikauden kalenterivuotena. Vuosi suljetaan
kausittain, joko kuukausittain (`2026-03`) tai neljännesvuosittain
(`2026-Q1`), mutta samana vuonna ei voi käyttää molempia. Kuukausi on
suositeltu ja koeteltu vaihtoehto. Kalenterivuodesta poikkeavaa tilikautta ei
vielä tueta.

ALV-ilmoitus tehdään samalle kaudelle, jolla kirjanpito suljetaan: kuukaudelle
tai neljännekselle. Suoraa ALV-ilmoittamista on koeteltu kuukausikausilla.

## Kauden tilat

Kauden tila näkyy aina, kun agentti näyttää sen:

```
period "2026-03" — "sealed" (…, vat_filed false)
```

| Tila | Mitä se tarkoittaa | Voiko kirjata |
|---|---|---|
| `open` (avoin) | kautta ei ole suljettu | kyllä |
| `closing` (suljettavana) | sulku on aloitettu ja odottaa hyväksyntääsi | ei |
| `sealed` (suljettu) | kausi on lukittu pysyvästi | ei |

`vat_filed` kertoo, onko kauden ALV-ilmoitus tehty. ALV-ilmoituksen luvut
lasketaan vain suljetusta kaudesta, koska vasta silloin ne eivät voi enää
muuttua.

Kausi etenee vain yhteen suuntaan. Sulkua ei voi perua, eikä tässä versiossa
ole komentoa, jolla kauden saisi takaisin avoimeksi. Jos sulku hylätään, kausi
jää pysyvästi tilaan `closing`
([Mitä on valmista ja mitä ei](../reference/status.md#hylätty-sulku)).

## Miksi suljettuun kauteen ei kirjata

Kun kuukauden luvut on raportoitu ja ALV ilmoitettu, niiden on pysyttävä
samoina. Jos suljettuun kuukauteen voisi lisätä tositteen, ilmoitettu ALV ja
kirjanpito eivät enää vastaisi toisiaan. Siksi myöhemmin huomattu puute tai
virhe kirjataan seuraavalle avoimelle kaudelle, kuten kirjanpidossa on aina
tehty: [Korjaa virheellinen tosite](../tasks/correct-voucher.md).

## Velvoitteet

Velvoite on avoin asia määräpäivineen, esimerkiksi puuttuva ajopäiväkirja.
Avoin velvoite estää kautensa sulun (`OPEN_OBLIGATION`), jotta asia ei unohdu
sulun alle. Velvoite merkitään ratkaistuksi vasta, kun asia on oikeasti
hoidettu.

Tässä versiossa velvoitteita syntyy vain poikkeuksellisten kuittien
konsultointireitin kautta, eikä sitä reittiä voi vielä käyttää. Tavallisessa
käytössä velvoitelista on siksi tyhjä.

## Tilikauden tila

Tilikauden tila on `open` tai `sealed`. Sinetöidyllä tilikaudella on myös
tilinpäätöslaskelmien tarkiste. Tilikauden sinetöi tilinpäätösajo, jota ei
vielä voi käyttää oikeaan tilinpäätökseen, joten tavallisesti tilikausi on
tilassa `open` ([Valmistele tilinpäätös](../tasks/year-end.md)).

## Mitä tämä tarkoittaa sinulle

- Sulje kuukausi vasta, kun se on valmis: pankkitili täsmää tiliotteeseen ja
  avoimet asiat on käsitelty.
- Jos huomaat puutteen sulun alettua, hyväksy sulku ja kirjaa puute
  seuraavalle kuukaudelle. Älä hylkää sulkua.
- Pysy samassa kausijaossa koko vuoden. Kuukausi on suositeltu.

## Katso myös

- [Sulje kuukausi](../tasks/close-month.md)
- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](filings.md)
