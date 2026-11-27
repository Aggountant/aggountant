---
name: migrate
description: >-
  Siirrä yritys toisesta ohjelmasta aggountantiin alue kerrallaan, kullakin
  oma vaihtopäivä: kirjanpito, ALV-ilmoitukset, Tulorekisteri-ilmoitukset ja
  matka- ja kululaskut. Luetteloi, vie vanhasta ohjelmasta vain lukien,
  tarkista ja arkistoi tiivistein, ja kirjaa mitä siirrettiin mistä päivästä,
  jotta seuraava alue jatkuu siitä. Avaa ennen kuin haet mitään vanhasta
  ohjelmasta tai kun yritys siirtää seuraavan alueen. Mukana havainnot
  Brioxista ja Holvista.
cli_version: "0.3.0"
schema_version: 1
---

# Siirto toisesta ohjelmasta

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi (`aggountant skill print aggountant-init --file SOUL.md`).

Yritys siirtyy harvoin kerralla. Usein kirjanpito siirtyy ensin ja palkanlaskijan
Tulorekisteri-ilmoitukset tai kululaskut myöhemmin, kukin omana päivänään. Siksi siirto
tehdään alue kerrallaan. aggountant ei tuo vanhan ohjelman tositteita, tilikarttaa eikä
ilmoituksia: kunkin alueen vaihtopäivää edeltävä aineisto jää arkistoksi, jota käyttäjä
säilyttää, ja aggountantiin siirtyy vain se, mikä on vaihtopäivänä kesken. aggountantissa
ei ole komentoja vanhan ohjelman tai pankin käyttöön: keräät aineiston ympäristösi omilla
työkaluilla, ja se tehdään, kun käyttäjä vielä pääsee vanhaan ohjelmaan. Tilauksen
päätyttyä aineistoa ei välttämättä enää saa, ja puute huomataan vasta, kun joku vuosia
myöhemmin kysyy vanhaa tositetta.

## Alueet

| Alue | Mitä siirtyy | Edellyttää | Ohje |
|---|---|---|---|
| Kirjanpito | avaavat saldot, avoimet laskut, ensimmäinen tiliote | — | [kirjanpito.md](kirjanpito.md) |
| ALV | ensimmäinen kausi, jonka aggountant ilmoittaa | kirjanpito | [alv.md](alv.md) |
| Tulorekisteri | tulonsaajat, muualla ilmoitetut kuukaudet, ilmoittamattomat maksut | tulonsaajat työntekijärekisterissä | [tulorekisteri.md](tulorekisteri.md) |
| Matka- ja kululaskut | avoimet laskut, ilmoittamattomat korvaukset, työpaikkatiedot | kirjanpito, Tulorekisteri, työntekijärekisteri | [kululaskut.md](kululaskut.md) |

Riippuvuudet seuraavat siitä, mistä ohjelma lukee: ALV-ilmoitus lasketaan aggountantin
sinetöidyistä tositteista, Tulorekisteri-ilmoitus ja kululasku hakevat henkilön
työntekijärekisteristä, ja kululaskun maksu ja ilmoittamaton korvaus kulkevat
kirjanpidon ja Tulorekisterin kautta. Muuten järjestys on vapaa.

Siirrä vain alueet, joita yrityksellä on ja joita käyttäjä on siirtämässä. Päättele ne
aineistosta ja käyttäjän pyynnöstä: palkanmaksu ja matkakorvaukset näkyvät vanhan
kirjanpidon tileillä ja tositesarjoissa. Alue, jota yrityksellä ei ole, ohitetaan
kokonaan: sen ohjetta ei avata eikä sen kysymyksiä esitetä. Yritys, jolla ei ole
kululaskuja, ei kuule niistä mitään. Avaa alueen ohje vasta, kun aloitat sen alueen.

## Mitä siirrettiin ja mistä päivästä

Seuraava alue voi siirtyä kuukausia myöhemmin toisessa istunnossa, joka tietää vain sen,
minkä tämä kirjasi. Kun alue on siirretty, tallenna siitä `knowledge company`-tietue
(esimerkiksi tunnisteella `siirto-kirjanpito`): vanha ohjelma ja sen viimeinen päivä,
aggountantin ensimmäinen päivä, mitä siirrettiin ja missä alueen arkisto on. Tämä on
fakta, josta myöhempi työ lukee, mitkä kaudet ja kuukaudet ovat aggountantin. Alueen
puuttuvat kohdat ja vielä siirtämättömät alueet kuuluvat kodin `TODO.md`-tiedostoon,
koska ne ovat odottavaa työtä. Kun seuraava alue alkaa, lue ensin nämä.

Vaihtopäivä on alueen vanhan järjestelmän viimeinen päivä. Kuukauden tai kauden vaihde
on lähes aina oikea, koska ilmoitukset annetaan kuukausittain tai kausittain, ja kesken
kauden vaihtaessa sama kausi olisi kahden ilmoittajan.

## Vanhaa järjestelmää vain luetaan

Viet ja lataat. Et tallenna, hyväksy, muuta etkä poista siellä mitään, et myöskään
sivulla, joka näyttää pelkältä kommenttikentältä. Vanha ohjelma on yhä yrityksen
kirjanpito ja ilmoitusten lähde niiltä kausilta, usein tilitoimiston tai palkanlaskijan
hoitama, eikä siellä tehtyä muutosta näe eikä saa takaisin täältä käsin.

Käyttäjä kirjautuu vanhaan ohjelmaan ja pankkiin itse ja hoitaa tunnistautumisen.
SOULin sääntö salaisuuksista koskee tässä myös selaimen evästeitä, kirjautumistunnisteita
ja osoitteita, joissa kirjautuminen kulkee mukana: niitä ei kopioida tiedostoihin,
luetteloon eikä keskusteluun. Kun istunto vanhenee, pyydä käyttäjää kirjautumaan
uudelleen.

Kun palvelu epää viennin tai muun toiminnon, kirjaa kohta luetteloon puuttuvaksi
syineen äläkä etsi kiertoreittiä. Epäys on käyttöoikeus, jonka antaa ohjelman
pääkäyttäjä, tilitoimisto tai palkanlaskija, ja käyttäjä pyytää sen heiltä. Jos joku
muu hoitaa aluetta vanhassa ohjelmassa, kysy jo alussa, kuka aineiston vie: kaikki
viennit eivät näy käyttäjän oikeuksilla.

## Luettelo ensin

Kirjoita ennen ensimmäistäkään vientiä luettelo siitä, mitä alueelta pitäisi tulla, ja
ota määrät vanhasta järjestelmästä itsestään, esimerkiksi tositeluettelon viimeisestä
numerosta. Ilman odotettua määrää et erota täyttä vientiä vajaasta, ja vajaa näyttää
levyllä yhtä valmiilta. Luettelo on työn tila: alue on kerätty, kun jokainen kohta on
tallessa ja tarkistettu tai merkitty puuttuvaksi syineen ja käyttäjä on nähnyt puuttuvat.
Pidä luettelo arkiston rinnalla, niin keskeytynyttä keruuta voi jatkaa siitä.

## Vienti ja tarkistus

Käytä ensin ohjelman omaa vientiä tai rajapintaa: ne toimivat samoin, vaikka
käyttöliittymä muuttuu, ja antavat alkuperäisen tiedoston. Selaimesta haetaan vain se,
mitä muuten ei saa, ja vain jos ympäristössäsi on selaimen ohjaus ja käyttäjä on
kirjautunut siihen selaimeen. Muuten käyttäjä lataa tiedostot ja sinä tarkistat ne.
Luettelot rajaavat oletuksena usein lyhyeen aikaväliin tai yhteen tilaan: katso rajaukset
ja vertaa osumamäärää luetteloosi.

Havainnot yksittäisistä ohjelmista ovat tiedostoissa [briox.md](briox.md) ja
[holvi.md](holvi.md). Muusta ohjelmasta lue sen oma vientiohje ja kerro käyttäjälle,
että toimit ilman aiempia havaintoja.

Avaa jokainen tiedosto: keskeytynyt lataus jättää vajaan tiedoston, joka näyttää
hakemistossa ehjältä. Lue yritys, ajanjakso ja tiedostomuoto sisällöstä, ei nimestä:
nimen päiväys voi olla vientihetki, väärän kauden raportti on muuten uskottava, ja
XLS-niminen tiedosto voi olla XLSX. Ero luetteloon on löydös, jonka kerrot käyttäjälle;
vanhan aineiston virheitä ei korjata arkistossa.

## Arkisto

Alkuperäiset viennit säilyvät tavu tavulta sellaisina kuin ne tulivat. Niistä johdetut
taulukot ja tarkistusraportit menevät omaan kansioonsa, jotta alkuperäiseen voi aina
palata eikä kukaan erehdy pitämään johdettua tiedostoa lähteenä.

Kirjaa jokaisesta tiedostosta koko, SHA-256-tiiviste, mistä se tuli ja milloin se
haettiin, ja kokoa tiivisteet arkiston juureen `SHA256SUMS`-tiedostoon. Merkitse
luettelon kohta valmiiksi vasta, kun sen tiedostot vastaavat tiivisteitä. Kopioi arkisto
lopuksi toiseen paikkaan ja tarkista kummankin kopion jokainen tiedosto tiivisteitä
vasten (`sha256sum -c SHA256SUMS`): tarkistamaton kopio ei ole varmuuskopio.

Arkisto ei kuulu gitiin. Se on oikeaa talousdataa henkilötietoineen ja suurta, eikä
gitin historiasta saa mitään pois. Sovi käyttäjän kanssa paikka, joka on hänen
varmuuskopiointinsa piirissä, ja kirjaa se kodin `AGENTS.md`-tiedostoon, josta seuraava
istunto löytää vanhan tositteen tai ilmoituksen. Myöhempi alue voi käyttää samaa
arkistoa omassa kansiossaan.

Kun keruu jää kesken, kirjaa puuttuvat kohdat `TODO.md`-tiedostoon ja kerro käyttäjälle,
ettei vanhan ohjelman tilausta kannata päättää ennen kuin ne on saatu.
