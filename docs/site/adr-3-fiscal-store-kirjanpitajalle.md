# ADR-3 kirjanpitäjälle — tarkastettava arkisto ja kirjanpidon kyselykäyttö

Tämä on **yhteenveto** ADR-3:n päätöksestä kirjanpidon ammattilaiselle: mitä
päätettiin ja miksi, kirjanpitäjän termein. Varsinainen päätös perusteluineen on
`adr/ADR-3-fiscal-store.md`; tämä sivu ei päätä mitään uutta.

## Mistä tässä on kyse

Tositteet ovat jo nyt lukittu **muuttumattomaan ketjuun** (jokainen tosite sidottu
edelliseen; muutos katkaisee ketjun ja ohjelma huomaa sen — ks. ADR-1/ADR-2). Tämä
on **parempi kuin markkinan yleinen taso**: valtaosa suomalaisohjelmista (esim.
Kitsas, Tilitin) lukitsee vain tilikauden ja laskee tilinpäätösarkistolle yhden
SHA256-tiivisteen kerran vuodessa — ja niiden oma arkistovarmenne myöntää, että
tiiviste suojaa **vain jos varmennesivu säilytetään luotettavasti ohjelman
ulkopuolella**.

Kaksi asiaa jäi kuitenkin ratkaisematta, ja ADR-3 ratkaisee ne:

1. Ketjusta ei vielä saa **kannettavaa, tarkastajan tarkistettavaa arkistoa**.
2. Kirjanpitodataa ei pääse **kysymään** joustavasti (AI-avustaja ei pysty
   tekemään vapaita analyysikyselyjä).

## 1. Tarkastettava tilikausiarkisto (`fiscal archive`)

Ohjelma osaa tuottaa **itsenäisen tilikausiarkiston**, jonka tarkastaja voi
todentaa **ilman aggountant-ohjelmaa**. Arkisto sisältää:

- **tositteet** koneen alkuperäisessä, ihmisluettavassa muodossa (sama muoto, josta
  muuttumattomuusketju lasketaan);
- **hylätyt kirjaukset** (miksi ohjelma kieltäytyi) ja kauden sinetit;
- **käytetyn tilikartan ja laskentasäännöt sisällytettynä arkistoon** (ei pelkkänä
  viittauksena) — niin että vuosien päästä näkee, minkä tilikartan mukaan tosite on
  kirjattu, vaikka tilikarttaa olisi myöhemmin muutettu;
- **tilikauden arkistossa myös luettavat kirjanpitokirjat** — päiväkirja, pääkirja,
  tase-erittelyt, tilinpäätös ja liitetiedot **PDF:nä**. Pelkkä nippu
  konetiedostoja ei ole tilikausiarkisto; laki (KPA 2:8) vaatii aineiston
  saatettavuuden selväkieliseen muotoon **ilman riippuvuutta yksittäisestä
  ohjelmasta**, koko säilytysajan.

Arkistolle lasketaan **SHA256-varmenne** (kuten Kitsasin ARKISTOVARMENNE), ja lisäksi:

- **jokainen arkisto viittaa edellisen arkiston varmenteeseen** — näin tilikaudet
  ketjuuntuvat toisiinsa vuosikymmenten yli. Tätä Kitsasissa ei ole: se paljastaa,
  jos jokin kausi on jälkikäteen lisätty tai poistettu.
- **varmenteen talletus ohjelman ulkopuolelle on pakollinen työvaihe** (tuloste
  tilitoimistolle, sähköposti kirjanpitäjälle tai talletus asiakkaan omaan
  varastoon). Juuri tämä tekee varmenteesta luotettavan — sama malli kuin Kitsasissa,
  mutta nyt **nimenomaisena pakollisena vaiheena**, ei toiveena.

Lisäsuojat, porrastettuna (rehellinen turvataso kerrottu):

- **Pakollinen perustaso:** SHA256-varmenne + ulkoinen talletus. Tarkistettavissa
  pelkällä `sha256sum` -komennolla, ei avaimia.
- **Suositeltu:** arkiston **sähköinen allekirjoitus** omalla allekirjoitusavaimella.
  Todistaa alkuperän — **mutta ei suojaa**, jos joku saa haltuunsa sekä koneen että
  allekirjoitusavaimen (tämä sanotaan suoraan).
- **Suositeltu jo v1:stä:** **luotettava aikaleima** (RFC 3161) ulkoiselta
  palvelulta. Tämä on ainoa keino, joka suojaa myös tilanteessa, jossa
  kirjanpitovelvollinen allekirjoittaa itse oman kirjanpitonsa. Yksi verkkokutsu
  vuodessa, ei pysyvää avainta.

**Mitä "tarkastettavissa ilman ohjelmaa" tarkoittaa käytännössä:** tarkastaja voi
tarkistaa arkiston tavallisilla, vuosikymmeniä muuttumattomilla työkaluilla
(`sha256sum`, tarvittaessa allekirjoituksen tarkistus), lukea tositteet ja
kirjanpitokirjat suoraan, ja kävellä muuttumattomuusketjun läpi dokumentoidulla
ohjeella. Ohjelman mukana tulee myös `aggountant verify-archive` -komento tämän
helpottamiseksi, mutta se **ei ole pakollinen** tarkistukselle.

## 2. Kirjanpidon kyselykäyttö (`fiscal analytics`)

AI-avustajan pitää voida tehdä **vapaita analyysikyselyjä** kirjanpitoon (raportit,
täsmäytykset, "paljonko myyntiä maaliskuussa"). Tätä varten tehdään **erillinen,
vain-luku -tietokanta**, joka on **koostettu sinetöidyistä tositteista** —
kertakäyttöinen kopio, ei koskaan alkuperäinen. Ratkaisevaa kirjanpitäjän kannalta:

- Kyselyt **eivät voi rakenteellisesti muuttaa tositteita** — avustaja ei koskaan
  edes näe alkuperäistä tositekantaa, vain kopion. Muuttumattomuus säilyy, vaikka
  avustaja tekisi minkälaisen kyselyn tahansa.
- Kopio on **rakennettu kyselyä varten**: rahamäärät täsmällisinä kokonaislukuina
  (ei pyöristyviä liukulukuja), tilin nimi ja laji sekä ALV valmiiksi mukana — niin
  että avustaja saa oikeita lukuja eikä joudu arvaamaan.
- Kopio kertoo aina, **mihin hetkeen se perustuu** (viimeinen mukaan otettu tosite),
  joten vanhentunut kopio on havaittavissa, ei hiljaa harhaanjohtava.

## Mitä tämä tarkoittaa arjessa

- **Tarkastaja saa tutun, kannettavan arkiston**, jonka voi todentaa itse — ja
  tarkemmin kuin markkinan yleinen taso (tositekohtaisesti, ei vain vuositasolla).
- **Laki täyttyy** aineiston säilyvyyden ja selväkielisyyden osalta (KPA 2:8;
  tositteet/kirjat 6 v, tasekirja 10 v).
- **Kirjanpitoa voi kysyä vapaasti** ilman että kyselyt voivat vahingoittaa
  kirjauksia.
- **Yksi käytännön vastuu jää ihmiselle:** varmenteen talletus ohjelman ulkopuolelle
  on pakollinen — ohjelma muistuttaa ja vaatii sen, mutta se pitää oikeasti tehdä.

## Liittyvät

- Varsinainen päätös: `adr/ADR-3-fiscal-store.md`.
- Tausta: **ADR-1** (mihin tositteet tallennetaan) ja **ADR-2** (tositteen tavumuoto
  ja hash-ketju).
- Vertailuaineisto: `benchmark.md` (Kitsas/Tilitin, arkistovarmenne).
