# Siirrä kirjanpito toisesta ohjelmasta

Kun yritys siirtyy aggountantiin toisesta kirjanpito-ohjelmasta, aggountant
aloittaa vaihtopäivän avaavista saldoista. Vanhan järjestelmän historia
viedään vanhasta ohjelmasta ja pankista arkistoksi, jota säilytät lain
vaatiman ajan. Kerää aineisto ennen kuin luovut vanhasta ohjelmasta, koska sen
jälkeen sitä ei välttämättä enää saa. Agentti käyttää keruuseen
`migrate-books`-skilliä.

## Ennen kuin aloitat

- Olet päättänyt vaihtopäivän eli viimeisen päivän, jonka vanha järjestelmä
  kattaa. Tilikauden vaihde on yksinkertaisin. Päiväyksen säännöt ovat sivulla
  [Ota yrityksesi kirjanpito käyttöön](set-up-company.md).
- Pääset vanhaan ohjelmaan ja pankkiin omilla tunnuksillasi, ja tilaus on
  voimassa, kunnes arkisto on tarkistettu.
- Jos tilitoimisto pitää kirjanpitoasi vanhassa ohjelmassa, olet sopinut sen
  kanssa, kuka vie aineiston. Kaikki viennit eivät välttämättä näy sinun
  käyttöoikeuksillasi.

## Mitä kerätään

Kirjanpitolain mukaan tilinpäätös, toimintakertomus (jos sellainen on
laadittu), kirjanpidot (päiväkirja ja pääkirja), tililuettelo ja luettelo
kirjanpidoista ja aineistoista säilytetään vähintään 10 vuotta tilikauden
päättymisestä. Tositteet, liiketapahtumia koskeva kirjeenvaihto ja muu
kirjanpitoaineisto säilytetään vähintään kuusi vuotta sen vuoden lopusta,
jonka aikana tilikausi päättyi (kirjanpitolaki 2 luku 10 §). Kerää siksi
jokainen tilikausi, jonka säilytysaika on vielä kesken.

| Aineisto | Mihin sitä tarvitaan |
|---|---|
| tililuettelo (tilinumerot, nimet, ALV-koodit) | vanhojen tilien muuntamiseen aggountantin tilikartalle avaavissa saldoissa ja arkistoon |
| saldoluettelo tai tase vaihtopäivältä, kesken tilikauden myös tuloslaskelma | avaavien saldojen tositteeseen |
| päiväkirja ja pääkirja jokaiselta tilikaudelta | arkistoon ja sen tarkistamiseen, että tositteet ja summat täsmäävät |
| tilinpäätökset, toimintakertomukset ja tase-erittelyt | arkistoon ja seuraavan tilinpäätöksen vertailutiedoiksi |
| tositteet ja niiden kaikki liitteet | arkistoon |
| ostolaskut ja myyntilaskut kaikissa tiloissa, myös avoimet | avoimien laskujen selvittämiseen vaihtopäivänä ja arkistoon |
| kommentit ja muistiinpanot tositteissa ja laskuissa | arkistoon, koska ne kertovat usein, miksi jokin kirjattiin niin kuin kirjattiin |
| pankin tiliotteet sähköisenä (camt.053 tai camt.052) ja PDF-muodossa | vaihtopäivän jälkeiset tuodaan aggountantiin, aiemmat menevät arkistoon |
| annetut ALV-ilmoitukset ja muut viranomaisilmoitukset | arkistoon, vaikka näet ne myöhemmin myös OmaVerosta |

## Näin teet sen

1. **Kirjaudu vanhaan ohjelmaan ja pankkiin itse** ja hoida tunnistautuminen
   (MFA). Älä anna agentille salasanaasi. Evästeitä, kirjautumistunnisteita
   ja kirjautumisen sisältäviä linkkejä ei kopioi tiedostoihin,
   muistiinpanoihin eikä keskusteluun kumpikaan teistä. Jos istunto vanhenee,
   kirjaudu uudelleen.

2. **Pyydä agenttia tekemään luettelo.**

   > Siirrämme Esimerkki Oy:n kirjanpidon aggountantiin. Vanha järjestelmä
   > kattaa 31.12.2025 asti. Tehdään ensin luettelo kaikesta, mitä vanhasta
   > ohjelmasta ja pankista pitää viedä.

   Agentti kirjoittaa luettelon odotetusta aineistosta: tilikaudet, raportit,
   tositesarjat ja niiden numerot, laskujen määrät, tiliotteiden jaksot ja
   liitteet. Luvut se ottaa vanhasta järjestelmästä, esimerkiksi
   tositeluettelon viimeisestä numerosta.

   Lue luettelo läpi. Sinä tiedät, onko yrityksellä ollut muita pankkitilejä
   tai aiempi ohjelma, joita agentti ei vanhasta ohjelmasta näe.

3. **Agentti vie aineiston.** Se käyttää ensisijaisesti ohjelman omaa vientiä
   tai rajapintaa. Sen, mitä saa vain selaimesta, agentti voi hakea
   kirjautuneesta selaimestasi, jos sen ympäristössä on selaimen ohjaus.
   Muuten lataat tiedostot itse ja annat ne agentille. aggountantissa
   itsessään ei ole työkaluja vanhan ohjelman tai pankin käyttöön. Agentti
   vain lukee vanhaa järjestelmää: se vie ja lataa, mutta ei tallenna,
   hyväksy, muuta eikä poista mitään.

4. **Agentti tarkistaa tiedostot ja kokoaa arkiston.** Se avaa jokaisen
   tiedoston ja katsoo sisällöstä, että yritys ja ajanjakso ovat oikeat.
   Kirjanpidoista se tarkistaa, että jokaisen tositteen debet ja kredit
   täsmäävät ja että vuosisummat vastaavat tilinpäätöstä. Alkuperäiset viennit
   agentti säilyttää muuttamattomina ja kirjaa jokaisen tiedoston
   SHA-256-tiivisteen `SHA256SUMS`-tiedostoon, josta näkee myöhemmin, että
   tiedosto on ennallaan.

5. **Päätä, missä arkisto säilytetään.** Säilytä se gitin ulkopuolella, koska
   se on oikeaa talousdataa ja suurta, eikä gitin historiasta saa mitään
   poistettua. Pidä se
   [varmuuskopioinnin piirissä](../tasks/back-up-and-restore.md). Agentti
   kopioi arkiston toiseen paikkaan, tarkistaa kummankin kopion tiivisteitä
   vasten ja kirjaa kodin `AGENTS.md`-tiedostoon, missä arkisto on. Silloin
   se löytää vanhan tositteen, kun kysyt sitä.

6. **Käy läpi, mitä jäi puuttumaan.** Agentti näyttää luettelosta kohdat,
   joita se ei saanut, ja erot, jotka se löysi tarkistuksessa. Pyydä
   puuttuvat tilitoimistolta tai ohjelman pääkäyttäjältä. Pidä vanhan
   ohjelman tilaus voimassa, kunnes jokainen kohta on tallessa tai olet
   hyväksynyt, että se jää puuttumaan.

7. **Ota kirjanpito käyttöön.** Saldoluettelosta ja tililuettelosta tehdään
   [avaavien saldojen tosite](set-up-company.md). Vaihtopäivän jälkeiset
   tiliotteet [tuodaan kuten joka kuukausi](../tasks/import-bank-statement.md),
   ja kuitit ja laskut
   [kirjataan tavalliseen tapaan](../tasks/record-receipts.md).

   Tuo ensimmäinen tiliote vaihtopäivää seuraavasta päivästä alkaen. Aiemmat
   tapahtumat ovat jo avaavissa saldoissa, ja ohjelma vaatii kirjauksen
   jokaiselle tiliotteen tapahtumalle, joten vaihtopäivän ylittävää
   tiliotetta ei voi tuoda kirjaamatta samoja tapahtumia toiseen kertaan.

   Ennen vaihtopäivää päivätty lasku, joka maksetaan vasta sen jälkeen, voi
   olla jo avaavissa saldoissa ostovelkana. Agentti tarkistaa sen
   saldoluettelosta ja avoimien laskujen luettelosta, ettei sama lasku tule
   kirjatuksi kahdesti.

<details>
<summary>Komennot, joita agentti ajaa</summary>

Arkiston agentti tarkistaa tavallisilla työkaluilla, ei aggountantilla:

```sh
cd arkisto-esimerkki-oy
sha256sum -c SHA256SUMS
```

Käyttöönoton komennot ovat sivuilla
[Ota yrityksesi kirjanpito käyttöön](set-up-company.md) ja
[Tuo pankin tiliote](../tasks/import-bank-statement.md).
</details>

## Valmis, kun

- Luettelon jokainen kohta on tallessa tai merkitty puuttuvaksi syineen.
- `sha256sum -c SHA256SUMS` hyväksyy jokaisen tiedoston kummassakin kopiossa.
- Avaavien saldojen tosite
  [täsmää saldoluetteloon](set-up-company.md#valmis-kun).
- Kodin `AGENTS.md` kertoo, missä vanhan järjestelmän arkisto on.

## Ohjelmakohtaisia huomioita

Agentin skillissä on havaintoja Brioxin ja Holvin vienneistä vuodelta 2026.
Sinun osaltasi niistä seuraa tämä:

### Briox

- Arkiston ja liitettyjen asiakirjojen vienti
  ([Arkiston vienti/lataaminen](https://help.briox.fi/hc/fi/articles/360029163831-Arkiston-vienti-lataaminen))
  ja tositteiden CSV-vienti eivät välttämättä näy sinun käyttöoikeuksillasi.
  Pyydä ne silloin tilitoimistolta tai pääkäyttäjältä. Ilman arkiston vientiä
  liitteet saa vain yksi kerrallaan selaimesta, eikä kuvaliitteen
  katseluversio välttämättä ole alun perin ladattu tiedosto.
- SIE4-vientiä emme löytäneet.

### Holvi

- Saatavilla olevat raportit riippuvat hinnoittelupaketista
  ([Holvin kirjanpitoraportit ja viennit](https://support.holvi.com/hc/fi/articles/115000984169-Holvin-kirjanpitoraportit-ja-viennit)).
- Holvin sähköinen tiliote (camt.052) käy aggountantiin sellaisenaan.
- Istunto vanhenee lyhyen käyttämättömyyden jälkeen. Kirjaudu silloin
  uudelleen, koska sivun päivitys ei korvaa kirjautumista.

## Rajoitukset

- Vanhan järjestelmän tositteita ja tilikarttaa ei voi tuoda aggountantiin,
  ei myöskään SIE4-tiedostona:
  [Yrityskoti ja käyttöönotto](../reference/status.md#yrityskoti-ja-käyttöönotto).
- Tiliotteista on koeteltu vain Holvin tiliote:
  [Pankin tiliote](../reference/status.md#pankin-tiliote).

## Jos jokin menee pieleen

- Jos palvelu epää viennin tai muun toiminnon, agentti kirjaa sen luetteloon
  puuttuvana eikä kokeile kiertoreittiä. Pyydä oikeus tai vienti ohjelman
  pääkäyttäjältä tai tilitoimistolta.
- Jos tiedosto ei aukea tai on lyhyempi kuin pitäisi, lataus keskeytyi.
  Agentti hakee sen uudelleen ja päivittää tiivisteen.
- Jos pankkitilin saldo ei täsmää ensimmäisen kuukauden jälkeen, katso
  [Pankkitilin saldo ei täsmää tiliotteeseen](../troubleshooting.md#pankkitilin-saldo-ei-täsmää-tiliotteeseen).

## Katso myös

- [Ota yrityksesi kirjanpito käyttöön](set-up-company.md)
- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
- [Mitä on valmista ja mitä ei](../reference/status.md)
