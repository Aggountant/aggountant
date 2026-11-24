---
name: migrate-books
description: >-
  Siirrä yrityksen kirjanpito toisesta kirjanpito-ohjelmasta aggountantiin: luetteloi,
  mitä vanhassa ohjelmassa ja pankissa on, vie aineisto muuttamatta siellä mitään,
  tarkista ja arkistoi se tiivistein ja luovuta saldot käyttöönottoon. Avaa ennen kuin
  haet mitään vanhasta ohjelmasta. Mukana havainnot Brioxista ja Holvista.
cli_version: "0.3.0"
schema_version: 1
---

# Kirjanpidon siirto toisesta ohjelmasta

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi (`aggountant skill print aggountant-init --file SOUL.md`).

aggountant ei tuo vanhan järjestelmän tositteita eikä tilikarttaa. Uusi kirjanpito
alkaa vaihtopäivän avaavista saldoista, ja kaikki sitä edeltävä jää arkistoksi, jota
käyttäjä säilyttää kirjanpitovelvollisena. Tämä skilli on sen arkiston kerääminen.
Avaavat saldot ja muu käyttöönotto ovat skillissä
[aggountant-init](../aggountant-init/SKILL.md).

Keruu tehdään, kun käyttäjä vielä pääsee vanhaan ohjelmaan. Tilauksen päätyttyä
aineistoa ei välttämättä enää saa, ja puute huomataan vasta, kun joku vuosia myöhemmin
kysyy vanhaa tositetta. aggountantissa ei ole komentoja vanhan ohjelman tai pankin
käyttöön: teet tämän työn ympäristösi omilla työkaluilla.

## Vanhaa järjestelmää vain luetaan

Viet ja lataat. Et tallenna, hyväksy, muuta etkä poista siellä mitään, et myöskään
sivulla, joka näyttää pelkältä kommenttikentältä. Vanha ohjelma on yhä yrityksen
kirjanpito niiltä kausilta, usein tilitoimiston hoitama, eikä siellä tehtyä muutosta
näe eikä saa takaisin täältä käsin.

Käyttäjä kirjautuu vanhaan ohjelmaan ja pankkiin itse ja hoitaa tunnistautumisen.
SOULin sääntö salaisuuksista koskee tässä myös selaimen evästeitä, kirjautumistunnisteita
ja osoitteita, joissa kirjautuminen kulkee mukana: niitä ei kopioida tiedostoihin,
luetteloon eikä keskusteluun. Kun istunto vanhenee, pyydä käyttäjää kirjautumaan
uudelleen.

Kun palvelu epää viennin tai muun toiminnon, kirjaa kohta luetteloon puuttuvaksi
syineen äläkä etsi kiertoreittiä. Epäys on käyttöoikeus, jonka antaa ohjelman
pääkäyttäjä tai tilitoimisto, ja käyttäjä pyytää sen heiltä. Jos tilitoimisto pitää
kirjanpitoa vanhassa ohjelmassa, kysy jo alussa, kuka aineiston vie: kaikki viennit
eivät näy käyttäjän oikeuksilla.

## Luettelo ensin

Kirjoita ennen ensimmäistäkään vientiä luettelo siitä, mitä pitäisi tulla: tilikaudet,
raportit kultakin, tositesarjat ja niiden numerovälit, laskujen määrät tiloittain,
tiliotteiden jaksot ja liitteet. Ota luvut vanhasta järjestelmästä itsestään,
esimerkiksi tositeluettelon viimeisestä numerosta. Ilman odotettua määrää et erota
täyttä vientiä vajaasta, ja vajaa näyttää levyllä yhtä valmiilta.

Luettelo on työn tila. Keruu on valmis, kun jokainen kohta on tallessa ja tarkistettu
tai merkitty puuttuvaksi syineen, ja käyttäjä on nähnyt puuttuvat. Pidä luettelo
arkiston rinnalla, niin keskeytynyttä keruuta voi jatkaa siitä.

## Mitä kerätään

Tilinpäätös, toimintakertomus, kirjanpidot (päiväkirja ja pääkirja), tililuettelo ja
luettelo kirjanpidoista ja aineistoista säilytetään vähintään 10 vuotta tilikauden
päättymisestä. Tositteet, liiketapahtumia koskeva kirjeenvaihto ja muu
kirjanpitoaineisto säilytetään vähintään kuusi vuotta sen vuoden lopusta, jonka aikana
tilikausi päättyi (kirjanpitolaki 2 luku 10 §). Kerää siis jokainen tilikausi, jonka
säilytysaika on kesken, ei vain viimeisintä.

- Tililuettelo tilinumeroineen, nimineen ja ALV-koodeineen. Se on avaavien saldojen
  tilimuunnoksen pohja.
- Saldoluettelo tai tase vaihtopäivältä, ja kesken tilikauden siirryttäessä myös
  tuloslaskelma. Avaava tosite tehdään näistä, ja saldoluettelo on sen liite.
- Päiväkirja ja pääkirja jokaiselta tilikaudelta.
- Tilinpäätökset, toimintakertomukset ja tase-erittelyt. Viimeisin on seuraavan
  tilinpäätöksen vertailutieto.
- Tositteet ja niiden kaikki liitteet.
- Osto- ja myyntilaskut kaikissa tiloissa. Vaihtopäivänä avoimet tarvitaan
  käyttöönotossa, koska saldoluettelo ei erittele ostovelkoja eikä myyntisaamisia.
- Tositteiden ja laskujen kommentit ja muistiinpanot. Niissä lukee usein, miksi jokin
  kirjattiin niin kuin kirjattiin, eivätkä ne tule raporttien mukana.
- Pankin tiliotteet sähköisenä (camt.053 tai camt.052) ja PDF-muodossa. Vaihtopäivän
  jälkeiset tuodaan aggountantiin, aiemmat menevät arkistoon.
- Annetut ALV-ilmoitukset ja muut viranomaisilmoitukset.

## Vienti

Käytä ensin ohjelman omaa vientiä tai rajapintaa: ne toimivat samoin, vaikka palvelun
käyttöliittymä muuttuu, ja antavat alkuperäisen tiedoston. Selaimesta haetaan vain se,
mitä muuten ei saa, ja vain jos ympäristössäsi on selaimen ohjaus ja käyttäjä on
kirjautunut siihen selaimeen. Muuten käyttäjä lataa tiedostot itse ja sinä tarkistat
ne.

Ohjelmien luettelot rajaavat oletuksena usein lyhyeen aikaväliin tai yhteen tilaan.
Katso rajaukset ennen vientiä ja vertaa osumamäärää luetteloosi.

Havainnot yksittäisistä ohjelmista ovat omissa tiedostoissaan: [briox.md](briox.md) ja
[holvi.md](holvi.md). Avaa se, jota yritys käyttää. Muusta ohjelmasta lue sen oma
vientiohje, ja kerro käyttäjälle, että toimit ilman aiempia havaintoja.

## Jokainen tiedosto tarkistetaan

Avaa jokainen tiedosto. Keskeytynyt lataus jättää vajaan tiedoston, joka näyttää
hakemistossa samalta kuin ehjä.

Lue yritys ja ajanjakso tiedoston sisällöstä: tiedostonimen päiväys voi olla
vientihetki, ja väärän tilikauden raportti on muuten uskottavan näköinen. Lue myös
tiedostomuoto sisällöstä, sillä XLS-niminen tiedosto voi olla XLSX.

Kirjanpidoista tarkista, että jokaisen tositteen debet ja kredit täsmäävät ja että
tilikauden summat vastaavat tilinpäätöstä. Laske tositteet ja laskut ja vertaa
luetteloon. Ero on löydös, jonka kerrot käyttäjälle; vanhan kirjanpidon virheitä ei
korjata arkistossa.

## Arkisto

Alkuperäiset viennit säilyvät tavu tavulta sellaisina kuin ne tulivat. Niistä johdetut
taulukot ja tarkistusraportit menevät omaan kansioonsa, jotta alkuperäiseen voi aina
palata ja jotta kukaan ei myöhemmin erehdy pitämään johdettua tiedostoa lähteenä.

Kirjaa jokaisesta tiedostosta koko, SHA-256-tiiviste, mistä se tuli ja milloin se
haettiin, ja kokoa tiivisteet arkiston juureen `SHA256SUMS`-tiedostoon. Merkitse
luettelon kohta valmiiksi vasta, kun sen tiedostot vastaavat tiivisteitä. Kopioi
arkisto lopuksi toiseen paikkaan ja tarkista kummankin kopion jokainen tiedosto
tiivisteitä vasten (`sha256sum -c SHA256SUMS`): tarkistamaton kopio ei ole varmuuskopio.

Arkisto ei kuulu gitiin. Se on oikeaa talousdataa henkilötietoineen ja suurta, eikä
gitin historiasta saa mitään pois. Sovi käyttäjän kanssa paikka, joka on hänen
varmuuskopiointinsa piirissä, ja kirjaa se kodin `AGENTS.md`-tiedostoon. Sieltä
seuraava istunto löytää vanhan tositteen, kun käyttäjä kysyy sitä.

## Luovutus käyttöönottoon

`aggountant-init` tekee saldoluettelosta ja tililuettelosta avaavan tositteen ja kertoo,
mille päivälle se tulee. Arkistosta siihen siirtyy kolme asiaa:

- Vaihtopäivän saldoluettelo ja tililuettelo sekä vaihtopäivänä avoimet laskut.
- Ensimmäinen tuotava tiliote alkaa vaihtopäivää seuraavasta päivästä. Aiemmat
  tapahtumat ovat jo avaavissa saldoissa, ja `aggountant import camt` vaatii
  kirjauksen tiliotteen jokaiselle tapahtumalle, joten vaihtopäivän ylittävä tiliote
  kirjaisi samat tapahtumat toiseen kertaan. Pyydä pankista uusi rajaus.
- Ennen vaihtopäivää päivätty lasku, joka maksetaan vasta sen jälkeen, voi olla jo
  avaavissa saldoissa ostovelkana. Tarkista se saldoluettelosta ja avoimien laskujen
  luettelosta ennen kuin kirjaat laskun kuluksi; muuten sama kulu on kirjattu kahdesti,
  kerran kummassakin järjestelmässä.

Kun keruu jää kesken, kirjaa puuttuvat kohdat kodin `TODO.md`-tiedostoon ja kerro
käyttäjälle, ettei vanhan ohjelman tilausta kannata päättää ennen kuin ne on saatu.
