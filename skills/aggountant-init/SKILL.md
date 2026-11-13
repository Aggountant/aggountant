---
name: aggountant-init
description: >-
  Ota yrityksen kirjanpito käyttöön aggountantissa tai jatka keskeneräistä
  käyttöönottoa: uusi kirjanpitokoti, yrityksen perustiedot, siirto aiemmasta
  järjestelmästä ja avaavat saldot, aineistolähteiden kuvaus.
cli_version: "0.1.0"
schema_version: 1
---

# Kirjanpidon käyttöönotto

Lue ensin [SOUL.md](SOUL.md), joka kuvaa roolisi kirjanpitoavustajana ja
`aggountant`-työvälineesi.

Käyttöönotto on valmis, kun yrityksen kirjanpitoa voi jatkaa aggountantissa ilman että
kukaan selvittää enää, mikä yritys tämä on, mistä sen tilikausi ja saldot alkavat ja
mistä aineisto tulee. Sama skilli palvelee sekä uuden yrityksen käyttöönottoa että
keskeneräisen jatkamista: katso ensin, mitä kodissa jo on (`aggountant config show`,
`chart list`, `voucher verify`, `knowledge company list`, kodin `AGENTS.md`), ja jatka
siitä.

## Tekninen alustus on pieni osa työtä

`aggountant init` luo kirjanpitokodin tallennusrakenteet, oletustilikartan ja
`.gitignore`-tiedoston, joka rajaa tositetietokannan, liiteblobit ja PII-varaston pois
versionhallinnasta. Se ei kysy eikä tallenna yrityksen nimeä, Y-tunnusta, tilikautta,
katkaisupäivää tai aiemman järjestelmän tietoja, eikä se alusta git-repositoriota.
Kodin sijainnin ja sen versionhallinnan sovit käyttäjän kanssa; kodin etsintäjärjestys
on `aggountant --help`-ohjeessa. Komento kieltäytyy ajamasta olemassa olevaan kotiin,
joten se ei ole tapa korjata tai päivittää vanhaa kotia. Jos uudempi binääri odottaa
masterdataa, jota vanha koti ei sisällä, erillistä päivityskomentoa ei toistaiseksi ole;
kerro tilanne käyttäjälle sen sijaan että kopioisit tiedostoja käsin.

Kaikki muu käyttöönoton tieto kerätään lähdeaineistosta ja käyttäjältä.

## Mitä valmiiseen lähtötilanteeseen kuuluu

**Yrityksen perustiedot** tallennetaan `knowledge company`-tietueiksi lähteineen:
virallinen nimi ja Y-tunnus, tilikausi, mistä päivästä aggountant-kirjanpito alkaa ja
mihin aiempi järjestelmä päättyy, sekä tilikarttapäätös (jatketaanko aiempaa tilikarttaa
vai siirrytäänkö aggountantin oletustilikarttaan). Nämä ovat juuri sitä tietoa, jota
kaikki myöhemmät skillit lukevat ensin, ja rekisteriote tai aiemman järjestelmän
raportti on niille parempi lähde kuin muistinvarainen vastaus.

**Tilikausi ohjelmalle**, jos se ei ole kalenterivuosi. Kalenterivuosi on oletus eikä
vaadi mitään. Muuten määrittele ennen ensimmäistä tositetta se tilikausi, johon
ensimmäinen tosite osuu: `aggountant fiscal-year define --start <alku> --end <loppu>`.
Alku on kuukauden ensimmäinen ja loppu kuukauden viimeinen päivä, ja pituus on
enintään 18 kuukautta (kirjanpitolaki 1:4). Seuraavat tilikaudet jatkuvat 12 kuukauden
mittaisina. Tilikausi tunnistetaan sen kalenterivuoden mukaan, jona se päättyy:
1.7.2026–30.6.2027 on tilikausi 2027. Ensimmäistä määriteltyä tilikautta edeltävälle
päivälle ei ole tilikautta, ellei se ala 1.1., joten tilikauden alussa siirtyvän
yrityksen avaava tosite päivätään tilikauden ensimmäiselle päivälle (siinä on silloin
vain tasetilejä). Määrittely on pysyvä, ja ohjelma kieltäytyy määrittelystä, joka
siirtäisi jo sinetöidyn tositteen toiselle tilikaudelle.

**Avaavat saldot**, kun yritys siirtyy toisesta järjestelmästä. Ne kirjataan tavallisena
sinetöitynä tositteena `voucher seal`-polkua pitkin, lähdesaldoluettelo liitteenä.
Muunnos aiemman järjestelmän tileiltä aggountantin tileille kannattaa kirjata omaksi
tiedostoksi kotiin, jossa jokainen lähdetili esiintyy kerran suuntineen ja summineen ja
loppusumma täsmää lähderaporttiin: usea lähdetili yhdistyy usein yhdeksi tiliksi, ja
erittely katoaa muuten. Erityisesti verotilin ja osakasvelkojen yhdistäminen samalle
tilille tekee myöhemmän täsmäytyksen vaikeaksi, joten pidä ne erillään jos tilikartta
sen sallii.

Avaavan tositteen päivämäärä on päättyvän järjestelmän viimeinen päivä, ei uuden
kirjanpidon ensimmäinen. Jos tilikauden aiemmat tulostilit tuodaan mukaan ja tosite
päivätään ensimmäiselle uudelle päivälle, koko aiempi tulos näkyy ensimmäisen kuukauden
kuukausituloksena.

- **Kesken tilikauden** tosite osuu samalle tilikaudelle kuin uudet tapahtumat, ja siinä
  on tasetilien lisäksi tilikauden tulostilit.
- **Tilikauden vaihteessa** (vanha järjestelmä kattaa tilikauden loppuun asti) tosite
  päivätään edellisen tilikauden viimeiselle päivälle, ja siinä on vain tasetilejä:
  edellisen tilikauden tulos kirjataan omaan pääomaan, ei tulostileille. Ohjelma käyttää
  sitä uuden tilikauden avaavina saldoina (`external_cutover` raportin lähteissä) eikä
  vaadi vanhaa tilikautta suljetuksi. Ehto on tiukka: jos edellisellä tilikaudella on
  tulostilejä, aiempia päiviä, vanhempia tilikausia tai aggountantissa suljettu kausi,
  uuden tilikauden raportit kieltäytyvät koodilla `PERIOD_NOT_SEALED`, eikä sinetöityä
  tositetta voi poistaa. Tarkista siksi tilit ja päivä ennen sinetöintiä.
- **Tilikauden alussa, kun se ei ala 1.1. ja on ensimmäinen määritelty**, tosite
  päivätään tilikauden ensimmäiselle päivälle (yllä). Sinetöityä tositetta ei poisteta, joten virhe korjataan
vastakirjauksella ja uudella tositteella, ja ketjuun jää kolme tositetta yhden sijaan.
Aja avauksen jälkeen `report statements` ensimmäiselle uudelle kuukaudelle ja katso, että
sen kuukausitulos on nolla ennen uusia tapahtumia, tilikauden alusta kertynyt tulos ja tase
vastaavat lähderaporttia ja pankkitilin saldo vastaa tiliotetta.

**Katkaisuhetken avoimet asiat**, jotka eivät näy saldoluettelosta. Kysy nämä
käyttäjältä käyttöönotossa, koska myöhemmin ne paljastuvat vasta täsmäytysvirheinä:

- Onko yhtiön laskuja maksettu osakkaan omalta tililtä tai kortilta? Pankista puuttuva
  maksu ei tarkoita maksamatonta laskua, ja vastatili on silloin velka osakkaalle.
- Onko taseessa verotiliä, vanhaa verovelkaa tai maksujärjestely? Jos on, pyydä
  katkaisuvuoden kaikki OmaVero-tapahtumaotteet kerralla. Maksujärjestelyn saldo ei ole
  yhden pääkirjatilin saldo, ja korot on usein kirjattu aiemmassa järjestelmässä kahteen
  kertaan tai jaksottamatta; se selviää vain katkeamattomasta oteketjusta.
- Onko avoimia myyntilaskuja tai ostovelkoja, joita saldoluettelo ei erittele?

Nämä ovat käyttäjän päätettäviä asioita. Kokoa ne yhdeksi kokonaiskuvaksi ehdotuksineen
sen sijaan että kysyisit joka erää erikseen, ja kirjaa vahvistetut tulkinnat
`knowledge company`-tietueiksi, jotta niitä ei kysytä uudelleen kuukausittain.

## Yrityksen aineistolähteet: `<AGGOUNTANT_HOME>/AGENTS.md`

Aggountant ei määrää, mistä kuitit tulevat tai minne käsitelty aineisto viedään.
Yritys kuvaa oman käytäntönsä kotinsa `AGENTS.md`-tiedostoon, ja `capture-receipt`
lukee sen ennen kuin paikantaa, kopioi tai siirtää mitään. Ilman tiedostoa myöhempi
agentti joko kysyy käyttäjältä joka kerta tai arvaa kansion, ja arvaus siirtelee
tiedostoja paikasta, jota kukaan ei tarkoittanut. Tiedosto on proosaa agentille, ei
konfiguraatiota, jota ohjelma jäsentäisi.

Jos tiedosto on olemassa, lue ja säilytä se. Jos ei, kirjoita se käyttäjän kanssa
yrityksen omilla sanoilla. Sen pitää vastata ainakin näihin: minne saapuvat kuitit ja
laskut päätyvät (sähköposti, pilvikansio, paikallinen polku, vastuuhenkilö); missä
kulu- ja matkalaskujen luonnokset ovat; missä ajokilometrit kirjataan ja mitä niistä
kirjataan (verottaja edellyttää ajopäiväkirjalta vähintään ajon alkamis- ja
päättymisajankohdan, tarkoituksen, lähtö- ja päätepisteen, matkamittarin lukemat ajon
alussa ja lopussa, kilometrit ja auton käyttäjän, tarvittaessa reitin ja korvaukseen
vaikuttavat lisätiedot); ja minne käsitelty alkuperäisaineisto arkistoidaan
ja milloin sen saa siirtää pois lähteestä. Älä ehdota yritykselle pilvipalvelua tai
kansiorakennetta, jota sillä ei ole.

Hyvä hetki on myös sopia, mikä kodissa kuuluu gitiin: raaka tositemateriaali ja
henkilötiedot eivät, johdettu ja tarkastettava metatieto (mappingit, luettelot,
yritystieto, valmistelut) kyllä. `init` kirjoittaa vain ohjelman omien tiedostojen
rajan; yrityksen omat kansiot lisätään `.gitignore`-tiedostoon itse. Sovittu raja
`AGENTS.md`-tiedostossa säästää jokaisen uuden aineistotyypin kohdalla saman päätöksen.

Pidä `AGENTS.md`-tiedostossa vain toimintatavat ja reitit; yrityksen faktat ja
kirjanpitokäytännöt ovat `knowledge company`-tietueissa, jossa niillä on lähde,
soveltamisala ja historia. Salasanoja, tunnuksia tai tokeneita ei kirjoiteta
kumpaankaan: koti on git-versioitua selkotekstiä ja keskusteluloki päätyy usein
talteen.

## Vero.fi

Jos yritys haluaa lähettää ALV-ilmoitukset suoraan Verolle, jatka `vero-fi`-skillillä.
Se kuvaa valmiuden, profiilin ja salaisuuksien käsittelyn; portaalin sopimus-, lupa- ja
varmennetoimet ovat käyttäjän omia oikeustoimia. Käyttöönotossa riittää, että tämä
on tiedossa ja että suoraa yhteyttä ei oteta käyttöön ennen kuin valmius on todettu.
