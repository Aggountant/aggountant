---
name: capture-receipt
description: >-
  Käsittele käyttäjän antama aineisto yrityksen kirjanpitoon: yksittäinen kuitti,
  joukko tositteita tai kuukauden tapahtumat. Kerro, mitä jää avoimeksi.
cli_version: "0.1.0"
schema_version: 1
---

# Aineiston käsittely kirjanpitoon

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi.

Käyttäjä antaa sinulle aineistoa: yhden kuitin, nipun tositteita tai kuukauden
tapahtumat. Hyvä lopputulos on, että jokainen tapahtuma on joko sinetöity tositteeksi
lähdeaineisto liitettynä tai nimetty avoimeksi niin, että käyttäjä tietää mitä siltä
puuttuu. Kerro lopuksi mitä käsittelit, mitä kokonaisuudesta pitäisi ymmärtää ja mikä
tarvitsee vielä huomiota.

## Mistä aineisto tulee ja minne se menee

Yritys kuvaa omat lähteensä ja käsitellyn aineiston reitin kirjanpitokodin
`AGENTS.md`-tiedostossa (`<AGGOUNTANT_HOME>/AGENTS.md`). Se voi osoittaa sähköpostiin,
pilvikansioon, paikalliseen inboxiin tai vastuuhenkilöön, ja se kertoo myös milloin
alkuperäisen saa siirtää pois. Lue se ennen kuin paikannat, kopioit tai siirrät mitään:
aggountant ei määrää hakemistorakennetta, joten ilman tätä tiedostoa et voi tietää
yrityksen käytäntöä. Jos tiedostoa ei ole eikä käyttäjä ole antanut aineistoa tai
lähdettä suoraan, kysy mistä aineisto haetaan. Oletettu kansio tai pilvipalvelu
johtaisi väärän aineiston käsittelyyn tai tiedostojen siirtelyyn paikasta, jota kukaan
ei tarkoittanut.

Tiedosto on proosaa toimintatavoista. Yrityksen vahvistetut faktat ja
kirjanpitokäytännöt ovat `knowledge/company`-tiedossa (`aggountant knowledge company
list` ja `search`); tarkista sieltä ennen kuin selvität saman asian uudelleen. Salaisuudet
eivät kuulu kumpaankaan.

Aiempi kirjanpito on käytettävissäsi: `aggountant voucher list` (mm. `--attachment-ref`
paljastaa, onko sama lähdetiedosto jo kirjattu), `aggountant voucher show` ja tilit
`aggountant chart list`. Lue niistä, miten yritys on käsitellyt vastaavat tapahtumat
aiemmin.

## Tavallinen ostokuitti

Tavallisen ostokuitin reitti on `aggountant capture-receipt record`. Domain johtaa
tasapainoisen kirjauksen ALV-jalkoineen aktiivisen tilikartan ALV-rooleista ja päivätystä
verokantataulukosta, ja sinetöity käsittely kantaa ALV-ilmoituksen tarvitsemat tiedot.
Jos kokoat ALV-jalat itse `voucher draft`-riveiksi, tämä kaikki jää tekemättä ja
ilmoitus rakentuu arvauksen päälle.

Arkistoi lähdetiedosto ensin komennolla `aggountant artifact attach --file <tiedosto>`.
Se palauttaa sha256-viitteen, joka annetaan syötteessä `source.attachment_ref`-kenttänä;
näin tosite kantaa todisteensa ja sama tiedosto tunnistetaan myöhemmin.

Komennon `--help` ei kuvaa stdin-JSONia, joten muoto on tässä. Kuitista luetut faktat
ovat `source`-objektissa, oma käsittelyehdotuksesi `proposal`-objektissa. Erottelu on
sama kuin SOUL.md:n vaatimus pitää lähteestä todettu ja oma tulkinta erillään, ja
esikatselun tiiviste lasketaan niistä erikseen.

```json
{
  "natural_key": "receipt:stable-source-id",
  "actor": "agent",
  "source": {
    "amount": {"amount": "125.50", "currency": "EUR"},
    "document_date": "2026-07-15",
    "description": "Palvelu",
    "attachment_ref": "<artifact attach -komennon sha256>"
  },
  "proposal": {
    "posting_date": "2026-07-16",
    "net_account": "7690",
    "contra_account": "1910",
    "vat": {"type": "domestic-input", "rate_class": "domestic-standard"}
  }
}
```

Tuntemattomat kentät hylätään. `source.document_date` on kuitin tai laskun päivä, ei
pankin kirjauspäivä: verokanta valitaan tästä päivästä, ja Suomen kannat ovat muuttuneet
(yleinen kanta 25,5 % 1.9.2024 alkaen, alennettu kanta 2026 alussa). `natural_key` on
lähteestä johdettu pysyvä tunniste. `actor` kertoo kuka valmisteli kirjauksen.

`proposal.vat` on aina nimenomainen väite, koska ALV on kirjanpidon ja ilmoituksen
rahaa. Vaihtoehdot:

- `domestic-input` + `rate_class` (lähdesumma on brutto). Kantaluokat ovat ohjelman
  verokantataulukon nimiä: tavalliset ovat `domestic-standard`, `domestic-reduced-1` ja
  `domestic-reduced-2`, ja taulukossa on lisäksi erityisluokkia (mm.
  `domestic-electricity`, `domestic-passenger-transport`, `zero-rated`); tuntemattoman
  luokan ohjelma hylkää päivättynä virheenä.
- `reverse-charge` + `rate_class` + `scope`, jossa scope on `eu-services`, `eu-goods` tai
  `non-eu-services` (lähdesumma on netto). Ohjelma kirjaa laskennallisen veron tilikartan
  scopen mukaiseen ALV-rooliparille. Jos pari puuttuu aktiivisesta tilikartasta, ohjelma
  kieltäytyy; se on oikea lopputulos, koska kovakoodatut tilit rikkoisivat tilikartan
  omistajuuden. Maahantuodut tavarat eivät ole tämän polun käsittely.
- `no-vat` vain, kun verottomuus on lähteestä tai yrityksen käytännöistä vahvistettu.
- `{"type":"clarification-required","question":"..."}`, kun ratkaiseva fakta puuttuu.
- `{"type":"unsupported","treatment":"...","reason":"..."}`, kun käsittely on tämän polun
  ulkopuolella, esimerkiksi osittainen vähennysoikeus.

Kaksi viimeistä ovat dataa, ei epäonnistumista: ne tuottavat ohjeellisen virheen ja
kertovat käyttäjälle täsmälleen, mikä pitää selvittää. Tuntemattoman ALV:n merkitseminen
`no-vat`iksi olisi hiljaa väärää kirjanpitoa, jota kukaan ei huomaa ennen ilmoitusta.

### Esikatselu, hyväksyntä, sinetöinti

```sh
aggountant --json capture-receipt record --dry-run < receipt.json > preview.json
```

Esikatselu ei kirjoita mitään eikä varaa tositenumeroa, joten sitä voi ajaa vapaasti.
Se palauttaa `draft`-kirjauksen ja `preparation_hash`-tiivisteen, joka sitoo yhteen
avaimen ja toimijan, lähdefaktat, ehdotuksen, tilikartan version, policyn ja luonnoksen.
`projection_valid` ja `ledger_preflight_valid` kertovat vain, että tämä projektio kelpaa
nyt.

Sinetöity tosite on pysyvä: se saa numeron hash-ketjussa, ja virhe korjataan
korjauskirjauksella, ei muokkaamalla. Siksi käyttäjä hyväksyy ennen sinetöintiä
kokonaisuuden, jonka hän pystyy arvioimaan: lähdefaktat, ehdotetun käsittelyn ja
syntyvän kirjauksen. Kun hyväksyntä on saatu, anna esikatselun tiiviste sellaisenaan:

```sh
hash=$(jq -er '.preparation_hash' preview.json)
aggountant --json capture-receipt record \
  --approved-preparation-hash "$hash" < receipt.json
```

Jos mikä tahansa tiivisteen osa on muuttunut välissä, ohjelma kieltäytyy koodilla
`CAPTURE_PREPARATION_CHANGED`. Tarkoitus on, ettei käyttäjä koskaan hyväksy yhtä ja saa
toista: tee uusi esikatselu ja pyydä hyväksyntä uudelleen. Saman syötteen uusinta samalla
`natural_key`-arvolla on turvallinen ja palauttaa olemassa olevan tositteen
(`replayed: true`); sama avain eri sisällöllä kieltäytyy. Avaimen vaihtaminen virheen
kiertämiseksi tuottaisi kahdennetun tositteen.

## Muut reitit

- Poikkeuksellinen kuitti, jonka käsittelyyn tarvitset ulkopuolista konsultaatiota, kulkee
  `capture-receipt start/answer/resume` -Runin kautta. Rajan yli menevät vain
  hyväksytyt faktat, ja käyttäjä hyväksyy paljastuksen erikseen
  (`ApproveConsultDisclosure`); raakalähde ei mene ulos.
- Erikoiskirjaukset, joita record-polku ei tue, valmistellaan komennoilla
  `aggountant voucher draft`, `validate` ja `seal`. Sama hyväksyntäperiaate koskee niitä.
- Pankin tiliotteet (CAMT.053/.052) käsittelee `aggountant import camt053`.
- Kuukauden kirjattu tilanne: `aggountant report statements`.
- Kun toimeksianto jatkuu ALV-ilmoitukseen, vaihda `vero-fi`-skilliin. Ilmoitus saa
  oman hyväksyntänsä ja veron maksu on käyttäjän erillinen tehtävä.

## Käyttäjän aika

Kuukauden aineistossa on kymmeniä tapahtumia. Ratkaise itse kaikki, mihin lähde, aiempi
kirjanpito ja yrityksen tieto riittävät, ja kokoa kysymykset yhteen sen sijaan että
keskeytät joka kuitista. Kysymys kannattaa esittää, kun vaihtoehdot eroavat tavalla,
josta käyttäjä välittää (tili, ALV-käsittely, kuuluuko kulu yritykselle) ja et voi
päätellä hänen valintaansa. Hyväksyntä sen sijaan pyydetään aina ennen sinetöintiä, ja
sen on oltava sellaisessa muodossa, että käyttäjä oikeasti näkee mitä hyväksyy;
usean tositteen erässä se tarkoittaa jokaisen rivin olennaiset tiedot, ei vain lukumäärää.
