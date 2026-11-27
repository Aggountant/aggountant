---
name: aggountant-init
description: >-
  Ota yrityksen kirjanpito käyttöön aggountantissa tai jatka keskeneräistä
  käyttöönottoa: uusi kirjanpitokoti, yrityksen perustiedot, tilikausi, siirto
  aiemmasta järjestelmästä ja avaavat saldot, kodin AGENTS.md ja TODO.md.
cli_version: "0.3.0"
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

## Valmiuslista

Käytännössä valmis tarkoittaa, että alla olevat seitsemän kohtaa ovat valmiita tai eivät
koske yritystä. Lista on näkymä siihen, mitä kodissa jo on, ei erillinen tila: älä
perusta sille omaa tiedostoa. Katso jokaisen istunnon alussa kodin `AGENTS.md` ja
`TODO.md` sekä ohjelmalta `aggountant config show`, `knowledge company list`,
`fiscal-year list`, `voucher list` katkaisun ympäriltä ja `period show` ensimmäiselle
kuukaudelle, ja näytä käyttäjälle kohta kohdalta, mikä on valmis, mikä puuttuu ja mikä
ei koske tätä yritystä. Valmista kohtaa ei aloiteta alusta, eikä tallennettua tietoa
kysytä uudelleen; jos se näyttää väärältä, kerro miksi ja päivitä se SOULin mukaan.

1. **Tunnistetiedot**: virallinen nimi ja Y-tunnus `knowledge company`-tietueina
   lähteineen.
2. **Tilikausi**: tallennettu tietueena, ja jos se ei ole kalenterivuosi, myös
   määritelty ohjelmaan.
3. **Katkaisu**: aggountant-kirjanpidon ensimmäinen päivä ja toisesta järjestelmästä
   siirtyvällä yrityksellä aiemman järjestelmän viimeinen päivä.
4. **Aineiston kattavuus**: jokaisen pankkitilin tiliotteet katkaisusta alkaen, kuittien
   ja laskujen reitti kodin `AGENTS.md`-tiedostossa ja vastaukset kohtaan
   [Mitä saldoluettelo ei kerro](#mitä-saldoluettelo-ei-kerro).
5. **Tilimuunnos ja avaus**: vain toisesta järjestelmästä siirtyvälle yritykselle.
   Muunnostiedosto on kodissa ja avaava tosite sinetöity.
6. **Täsmäytykset**: ensimmäisen kuukauden `aggountant report statements` vastaa
   lähderaporttia ja pankkitilin saldo tiliotetta. Siirtyvän yrityksen ehdot ovat
   [avaavan tositteen ohjeessa](avaavat-saldot.md#onnistuiko-avaus).
7. **Ensimmäinen sulku**: ensimmäinen kuukausi on suljettu `capture-receipt`-skillin
   mukaan, eli `period show` näyttää sen sinetöidyksi.

## Yksi aineistopyyntö

Kun lista on käyty läpi ensimmäisen kerran, kokoa kaikki, mitä käyttäjän pitää
toimittaa, yhdeksi pyynnöksi. Kerralla saatu aineisto säästää käyttäjän kymmeneltä
erilliseltä kierrokselta, joista jokainen pysäyttää työn. Räätälöi pyyntö yritykselle:
nimeä sen omat pankkitilit ja kaudet, pyydä vain sitä, mikä koskee sitä (OmaVeron
otteita vain, jos taseessa on verotiliä tai maksujärjestely; palkka-aineistoa vain,
jos sillä on palkansaajia), ja kerro kunkin kohdan kohdalla, miksi sitä tarvitaan ja
missä muodossa se on paras. Esimerkki on tiedostossa
[aineistopyynto.md](aineistopyynto.md).

Kirjaa pyyntö kodin `TODO.md`-tiedostoon muilta odotettavaksi. Jos käyttöönotto oli jo
kesken, kun lista otettiin käyttöön, pyyntö kattaa vain sen, mitä ei ole toimitettu, ja
täydentää `TODO.md`:n jo odottavia kohtia toistamatta niitä. Jatkoistunnossa pyydä vain
sitä, mikä pyynnöstä yhä puuttuu. Jos myöhemmin paljastuu jotain, mitä pyyntö ei
kattanut, kerro käyttäjälle, miksi sitä ei voinut nähdä ennalta.

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

Koskee vain toisesta järjestelmästä siirtyvää yritystä; uudella yrityksellä ei ole
avaavia saldoja. Saldot kirjataan tavallisena sinetöitynä tositteena,
lähdesaldoluettelo liitteenä. Vanhan järjestelmän aineiston kerääminen arkistoksi on
`migrate-books`-skillissä; tee se ennen kuin käyttäjä luopuu vanhasta ohjelmasta.
Sinetöinti on pysyvä, ja virheellinen avaus jättää ketjuun kolme tositetta yhden
sijaan. Kysy siksi ensin se, mitä saldoluettelo ei kerro (alla), koska vastaukset
voivat muuttaa saldoja, ja näytä käyttäjälle tilit ja päivä ennen sinetöintiä.
Tositteen päivä ja sisältö riippuvat siirtymän hetkestä, ja aiemman järjestelmän tilit
muunnetaan oletustilikartalle; molemmat ovat tiedostossa
[avaavat-saldot.md](avaavat-saldot.md).

## Mitä saldoluettelo ei kerro

Nämä selviävät vain kysymällä, ja kysymättä ne paljastuvat kuukausia myöhemmin
täsmäytysvirheinä. Kysy ne myös uudelta yritykseltä, jolla saldoluetteloa ei ole:
perustamisvaiheen kulut maksetaan usein omalta tililtä.

- Onko yhtiön laskuja maksettu osakkaan omalta tililtä tai kortilta? Pankista puuttuva
  maksu ei silloin tarkoita maksamatonta laskua, ja vastatili on velka osakkaalle.
- Onko taseessa verotiliä, vanhaa verovelkaa tai maksujärjestely? Pyydä silloin
  katkaisuvuoden kaikki OmaVeron tapahtumaotteet kerralla. Maksujärjestelyn saldo ei ole
  yhden pääkirjatilin saldo, ja korot on aiemmassa järjestelmässä usein kirjattu kahteen
  kertaan tai jaksottamatta; se näkyy vain katkeamattomasta oteketjusta.
- Onko avoimia myyntilaskuja tai ostovelkoja, joita saldoluettelo ei erittele?
- Käytetäänkö yrityksen toimintaan omaa autoa, kotia tai muuta yksityistä omaisuutta?
  Käyttöosuus ja sen peruste ovat pysyviä tietoja, joita jokainen kuukausi tarvitsee.

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
aggountant on päivitetty, koti tarvitsee `aggountant home upgrade`n; sen ohje kertoo
loput. Se, minkä päivitys jättää lisäämättä, kerrotaan käyttäjälle eikä korjata käsin:
tilikartan versio on sen sisällöstä laskettu tiiviste, johon sinetöidyt tositteet
viittaavat.
