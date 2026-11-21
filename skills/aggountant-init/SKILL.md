---
name: aggountant-init
description: >-
  Ota yrityksen kirjanpito käyttöön aggountantissa tai jatka keskeneräistä
  käyttöönottoa: uusi kirjanpitokoti, yrityksen perustiedot, tilikausi, siirto
  aiemmasta järjestelmästä ja avaavat saldot, kodin AGENTS.md ja TODO.md.
cli_version: "0.1.0"
schema_version: 1
---

# Kirjanpidon käyttöönotto

Lue ensin [SOUL.md](SOUL.md): roolisi, kirjanpitokodin pelisäännöt, se mikä tässä
työssä on pysyvää, ja miten työkalua luetaan.

Käyttöönotto on valmis, kun kirjanpitoa voi jatkaa ilman että kukaan selvittää enää,
mikä yritys tämä on, mistä sen tilikausi ja saldot alkavat ja mistä aineisto tulee.
Keskeneräistä käyttöönottoa jatketaan samasta: katso ensin, mitä kodissa jo on.

`aggountant init` on tästä pieni osa. Se luo kodin tallennusrakenteet, oletustilikartan
ja `.gitignore`-tiedoston, joka rajaa ohjelman omat tietokannat ja liitteet pois
versionhallinnasta. Yrityksestä se ei kysy mitään, eikä se alusta git-repositoriota:
kodin sijainti ja versionhallinta sovitaan käyttäjän kanssa. Kaikki muu kerätään
lähdeaineistosta ja käyttäjältä.

## Perustiedot ja tilikausi

Tallenna `knowledge company`-tietueiksi virallinen nimi ja Y-tunnus, tilikausi, aiemman
järjestelmän viimeinen päivä ja aggountant-kirjanpidon ensimmäinen. Kaikki myöhempi työ
lukee nämä ensin, joten rekisteriote tai aiemman järjestelmän raportti on parempi lähde
kuin muistinvarainen vastaus.

Ohjelma olettaa tilikaudeksi kalenterivuoden. Jos se on muu, määrittele
`aggountant fiscal-year define`lla se tilikausi, johon ensimmäinen tosite osuu, ennen
kuin sinetöit mitään: määrittely on pysyvä, eikä sitä voi enää tehdä tavalla, joka
siirtäisi sinetöidyn tositteen toiselle tilikaudelle. Ensimmäistä määriteltyä tilikautta
edeltävillä päivillä ei ole tilikautta lainkaan, ellei se ala 1.1., eikä niille voi
kirjata. Määrittely ratkaisee siis myös sen, mille päivälle avaava tosite voi tulla.

## Avaavat saldot

Toisesta järjestelmästä siirtyvän yrityksen saldot kirjataan tavallisena sinetöitynä
tositteena, lähdesaldoluettelo liitteenä. Sinetöinti on pysyvä, ja virheellinen avaus
jättää ketjuun kolme tositetta yhden sijaan. Kysy siksi ensin se, mitä saldoluettelo ei
kerro (alla), koska vastaukset voivat muuttaa saldoja, ja näytä käyttäjälle tilit ja
päivä ennen sinetöintiä.

Päivä on päättyvän järjestelmän viimeinen päivä, ei uuden kirjanpidon ensimmäinen.
Ensimmäiselle uudelle päivälle päivätty avaus toisi mukanaan tuodut tulostilit
ensimmäisen kuukauden tulokseen. Kesken kuukauden siirryttäessä niin käy joka
tapauksessa, joten ehdota siirtymää kuun vaihteeseen. Se, mitä tositteeseen kuuluu,
riippuu siirtymän hetkestä:

- **Kesken tilikauden.** Tosite osuu samalle tilikaudelle kuin uudet tapahtumat, ja
  siinä on tasetilien lisäksi tilikauden alusta kertyneet tulostilit.
- **Vuodenvaihteessa**, kun tilikausi on kalenterivuosi ja aiempi järjestelmä kattaa
  edellisen vuoden loppuun. Tosite päivätään edellisen tilikauden viimeiselle päivälle,
  ja siinä on vain tasetilejä: edellisen tilikauden tulos on omassa pääomassa. Ohjelma avaa uuden
  tilikauden näistä saldoista vaatimatta vanhaa tilikautta suljetuksi, mutta vain jos
  edellisellä tilikaudella ei ole mitään muuta: ei tulostilejä, ei muita päiviä, ei
  aggountantissa suljettua kautta eikä vanhempia tilikausia. Muuten uuden tilikauden
  raportit kieltäytyvät koodilla `PERIOD_NOT_SEALED`.
- **Tilikauden vaihteessa, kun tilikausi ei ala 1.1.** Määrittele ensimmäiseksi uusi
  tilikausi. Edellistä päivää ei silloin ole ohjelmalle olemassa, ja tosite päivätään
  uuden tilikauden ensimmäiselle päivälle. Siinä on vain tasetilejä, joten kuukauden
  tulos ei vääristy, eikä edellisen kohdan ehtoa tarvita.

Kirjanpito käyttää aina oletustilikarttaa, ja aiemman järjestelmän tilit muunnetaan sille
avaavassa tositteessa. Kirjaa muunnos omaksi tiedostokseen kotiin niin, että jokainen
lähdetili on siinä kerran summineen ja summat täsmäävät sekä lähderaporttiin että
avaavaan tositteeseen; muuten erittely katoaa. Kerro käyttäjälle, mitkä erottelut
yhdistyvät. Jos verotili ja osakasvelat osuisivat samalle tilille, sovi niiden erottelu:
yhdistettyinä niitä ei saa myöhemmin täsmäytettyä.
ALV-tilien avaava saldo on aiempien kausien veroa ja näkyy ensimmäisen ALV-ilmoituksen täsmäytyksessä
merkitsemättömänä vientinä; `vero-fi`-skilli kertoo, miten se kuitataan.

Avauksen jälkeen `aggountant report statements` ensimmäiselle uudelle kuukaudelle
näyttää, onnistuiko se: kuukauden tulos on nolla ennen uusia tapahtumia, tilikauden
alusta kertynyt tulos on lähderaportin mukainen (tilikauden vaihteessa nolla), tase
vastaa lähderaporttia ja pankkitilin saldo tiliotetta.

## Mitä saldoluettelo ei kerro

Nämä selviävät vain kysymällä, ja kysymättä ne paljastuvat kuukausia myöhemmin
täsmäytysvirheinä:

- Onko yhtiön laskuja maksettu osakkaan omalta tililtä tai kortilta? Pankista puuttuva
  maksu ei silloin tarkoita maksamatonta laskua, ja vastatili on velka osakkaalle.
- Onko taseessa verotiliä, vanhaa verovelkaa tai maksujärjestely? Pyydä silloin
  katkaisuvuoden kaikki OmaVeron tapahtumaotteet kerralla. Maksujärjestelyn saldo ei ole
  yhden pääkirjatilin saldo, ja korot on aiemmassa järjestelmässä usein kirjattu kahteen
  kertaan tai jaksottamatta; se näkyy vain katkeamattomasta oteketjusta.
- Onko avoimia myyntilaskuja tai ostovelkoja, joita saldoluettelo ei erittele?

Tulkinnat ovat käyttäjän päätettäviä. Kokoa ne yhdeksi kokonaiskuvaksi ehdotuksineen,
ja tallenna vahvistetut `knowledge company`-tietueiksi, jotta samaa ei kysytä joka
kuukausi.

## Kodin `AGENTS.md` ja `TODO.md`

SOUL kertoo, mitä näihin kuuluu ja miten niitä pidetään yllä. Käyttöönotossa ne
perustetaan kodin juureen käyttäjän kanssa ja yrityksen omin sanoin. Olemassa olevaa
tiedostoa täydennetään.

`AGENTS.md` vastaa siihen, mitä kuukauden työ muuten kysyisi tai arvaisi joka kerta:

- minne saapuvat kuitit ja laskut päätyvät (sähköposti, pilvikansio, paikallinen polku)
  ja kuka niistä vastaa;
- missä kulu- ja matkalaskujen luonnokset ovat;
- missä ajokilometrit kirjataan. Verohallinto edellyttää ajopäiväkirjalta ajon alkamis-
  ja päättymisajan, tarkoituksen, lähtö- ja päätepisteen, matkamittarin lukemat alussa
  ja lopussa, kilometrit ja auton käyttäjän, joten sovi, että ne kirjataan;
- minne käsitelty alkuperäinen arkistoidaan ja milloin sen saa siirtää pois lähteestä;
- mikä kodista kuuluu gitiin. Raaka tositemateriaali ja henkilötiedot eivät kuulu,
  johdettu ja tarkastettava tieto (tilimuunnokset, luettelot, valmistelut) kuuluu. `init`
  rajasi vain ohjelman omat tiedostot, joten yrityksen omat kansiot lisätään
  `.gitignore`-tiedostoon itse.

Kuvaa se käytäntö, joka yrityksellä on; pilvipalvelua tai kansiorakennetta, jota sillä
ei ole, ei kannata ehdottaa.

Uuteen `TODO.md`-tiedostoon riittää osio kullekin SOULin nimeämälle asialle. Kirjaa
sinne heti se, mikä käyttöönotosta jäi odottamaan. Esimerkki Oy:n tiedostossa voisi
lukea, että perustajan omalta tililtään maksaman rekisteröintimaksun kuitti puuttuu,
lokakuuta ei suljeta ennen sitä, ja joulukuun ALV-ilmoitus on annettava 12.2. mennessä.

## Myöhemmin

Suora ilmoittaminen Verolle otetaan käyttöön `vero-fi`-skillillä, kun yritys sitä
haluaa; käyttöönotto ei edellytä sitä.

`init` kieltäytyy olemassa olevasta kodista, joten vanhaa kotia ei korjata sillä. Kun
aggountant on päivitetty, tai koti antaa virheen `MASTERDATA_CORRUPT`, koti tarvitsee
`aggountant home upgrade`n; sen ohje kertoo loput. Se, minkä päivitys jättää
lisäämättä, kerrotaan käyttäjälle eikä korjata käsin: tilikartan versio on sen
sisällöstä laskettu tiiviste, johon sinetöidyt tositteet viittaavat.
