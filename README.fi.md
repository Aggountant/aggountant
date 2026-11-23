<img src="site/public/logo.svg" alt="" width="56" height="56">

# aggountant

<!-- shipshape-readme:badges-start -->
[![CI](https://github.com/aggountant/aggountant/actions/workflows/ci.yml/badge.svg)](https://github.com/aggountant/aggountant/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
<!-- shipshape-readme:badges-end -->

*[In English](README.md)*

aggountant on avoimen lähdekoodin työkalu tekoälyagenteille. Sen avulla agentti hoitaa pienen suomalaisen osakeyhtiön lakisääteisen kirjanpidon ja tarjoaa taloushallinnon neuvontaa, raportointia ja analyysejä.

Tositteiden käsittely, kaksinkertainen kirjanpito, viranomaisilmoitukset ja muut lakisääteiset rutiinit pitää pystyä automatisoimaan. Silloin kirjanpito on jatkuvasti ajan tasalla, ja yrittäjällä on selvä, reaaliaikainen tieto yrityksensä taloudesta heti käytettävissään.

aggountant on jo käytössä muutaman suomalaisen yrityksen kirjanpidossa. Projekti on silti varhaisessa vaiheessa, eikä kaikkea suunniteltua ole vielä toteutettu.

**Käyttöopas:** [aggountant.dev](https://aggountant.dev) ·
[suomeksi](docs/user/fi/README.md) ·
[in English](docs/user/en/README.md) ·
[Tule varhaiseksi käyttäjäksi](docs/user/fi/start/early-users.md)

- [Elämä aggountantin kanssa](#elämä-aggountantin-kanssa)
- [Miksi työ tuntuu kevyeltä](#miksi-työ-tuntuu-kevyeltä)
- [Luotettavuus](#luotettavuus)
- [Tilanne](#tilanne)
- [Tavoitteet](#tavoitteet)
- [Asennus](#asennus)
- [Käytön aloitus](#käytön-aloitus)
- [Osallistu](#osallistu)

## Elämä aggountantin kanssa

Kun hoidat kirjanpitoa aggountantilla, tilitoimiston kanssa riittää tuntiveloitteinen sopimus, jolla ostat asiantuntija-apua, kun tarvitset.

Pienessä yrityksessä tavallisen kuukauden kirjanpito vie vartista puoleen tuntiin, esimerkiksi heti seuraavan kuukauden ensimmäisenä päivänä.

Pyydät agenttia käsittelemään edellisen kuukauden. Kun agenttisi on kytketty pankkiin ja sähköpostiin, se hakee tiliotteen ja tilitapahtumat liitteineen ja etsii loput tositteet sähköpostista sekä kansiosta, johon tallennat esimerkiksi valokuvat paperikuiteista. Jos jotain puuttuu, agentti kertoo sen ja auttaa hankkimaan puuttuvan aineiston. Kun tiedät, että aineisto käsitellään automaattisesti, voit järjestää asiat niin, että tositteet päätyvät oikeaan paikkaan jo syntyessään, ja puuttuvia jää vähän.

Agentti voi käyttää myös sopimuksia, tuntikirjauksia ja kalenteria. Niistä syntyvät asiakaslaskut sekä matka- ja kululaskut, ja agentti käsittelee niiden vaikutukset kirjanpitoon.

Kun aineisto on koossa, agentti ehdottaa kirjaukset ja aggountant tarkistaa ne. Hyväksyt tositteet, ja agentti ehdottaa kuukauden sulkemista. Voit käydä aineiston läpi niin tarkasti kuin haluat, mutta tavallisesti siinä ei ole mitään erityistä selvitettävää.

Suljetusta kuukaudesta aggountant laskee arvonlisäveroilmoituksen ja lähettää sen hyväksynnälläsi suoraan Verohallinnolle. Matkakulukorvaukset se ilmoittaa Tulorekisteriin. Agenttisi voi valmistella ALV-maksun ja kulukorvaukset pankkiisi, esimerkiksi Holviin, jossa hyväksyt maksut mobiilisovelluksessa. aggountant itse ei maksa mitään.

Kaikki ei aina valmistu ensimmäisenä päivänä, koska joskus odotetaan palveluntarjoajan raporttia tai puuttuvaa tositetta. Kun aineisto myöhemmin tulee, pyydät agenttia jatkamaan siitä, mihin jäitte. Sinun tehtäväksesi jää lähinnä kokonaisuuden tarkistaminen, maksujen hyväksyminen ja joskus yksittäisen tositteen toimittaminen.

Pankki-, sähköposti- ja laskutusyhteydet ovat agenttisi omia työkaluja, eivät osa aggountantia. aggountant lukee tiliotteen camt.053- tai camt.052-tiedostona, ja agentti hakee aineiston sieltä, mihin annat sille pääsyn.

## Miksi työ tuntuu kevyeltä

Perinteinen kirjanpito on rakennettu sen ympärille, että kirjanpitäjän työaika maksaa. Tositteet pitää toimittaa oikeassa muodossa, ja jokainen ylimääräinen selvitys voi tarkoittaa lisälaskua.

Agentin kanssa tätä rajoitetta ei ole. Jos ostit jotain vahingossa väärällä kortilla, saat asian selvitettyä saman tien. Jos kuitti puuttuu, agenttisi etsii sitä niin kauan kuin tarvitaan. Jos haluat ymmärtää jonkin kirjauksen perusteet, agentti tutkii tositteet, säädökset ja Verohallinnon ohjeet.

Tämä muuttaa myös sitä, miten kirjanpito kannattaa järjestää. Verkkokaupan tuhansia myyntejä ei tarvitse niputtaa koontitositteiksi työmäärän takia. Kirjanpidon tarkkuus voidaan valita sen mukaan, mitä yrityksen toiminnasta halutaan tietää, eikä sen mukaan, mikä minimoi kirjanpitäjän työn.

Suurin hyöty ei ole säästö vaan se, että koko kirjanpito on jatkuvasti käytettävissä. Voit kysyä yrityksesi taloudesta milloin tahansa, vertailla kustannuksia tai pyytää uuden raportin. Kun agentilla on yhteys pankkiin ja muihin palveluihin, suurin osa kuukauden aineistosta voi olla käsitelty jo ennen kuukauden loppua, ja talouden tilannetta voi seurata koko ajan. Kirjanpidosta tulee jatkuvaa keskustelua yrityksen taloudesta eikä kerran kuussa hoidettava velvollisuus.

## Luotettavuus

Kirjanpidon oikeellisuutta ei jätetä tekoälymallin päättelyn varaan.

- **Laskenta on tavallista ohjelmakoodia.** aggountant ei sisällä tekoälyä. Se numeroi tositteet, tarkistaa debetin ja kreditin tasapainon, laskee ALV:n verokantataulukosta ja kieltäytyy kaikesta, mikä ei täsmää. Agentti tulkitsee ja kerää aineistoa, mutta ei kirjoita kirjanpitoa itse.
- **Laki ratkaisee, ei pyyntö.** Agentti ehdottaa, mitä kulu on, ja aggountant laskee siitä lain mukaisen seurauksen. Jos esimerkiksi pyydät vähentämään 70 prosenttia sellaisen henkilöauton arvonlisäverosta, jota käytetään myös yksityisesti, aggountant vähentää nolla, koska laki ei salli vähennystä. Agentti kertoo syyn ja auttaa löytämään oikean käsittelyn.
- **Ihminen hyväksyy pysyvät teot.** Tositteen sinetöinti, kuukauden sulku ja ilmoituksen lähetys odottavat hyväksyntääsi. Agentti saa ilman lupaa vain lukea ja esikatsella.
- **Kirjanpito on jäljitettävä.** Sinetöityä tositetta ei muokata, vaan virhe korjataan uudella tositteella, kuten kirjanpitolaki edellyttää. Jokainen tosite on ketjutettu edelliseen, joten `aggountant voucher verify` havaitsee jälkikäteiset muutokset. Jos ilmoituksen lähetyksen lopputulos jää epävarmaksi, aggountant selvittää tilanteen ennen kuin mitään lähetetään uudelleen.
- **Aineisto ei riipu tekoälypalvelusta.** Yrityksen kirjanpito on omassa hakemistossaan, osin tavallisina tekstitiedostoina, ja se pysyy luettavana ja tarkastettavana riippumatta siitä, mikä malli on käytössä.

Mikään järjestelmä ei ole erehtymätön. Siksi ohjelmalliset tarkistukset, agentin työnkulut ja tarvittaessa kirjanpitäjän arvio täydentävät toisiaan.

## Tilanne

Versio 0.2, ja jo oikeassa käytössä. Toimintoja lisätään sitä mukaa kuin käyttäjäyritykset kohtaavat uusia tilanteita, joten tavallinen kuukausityö on hyvässä kunnossa ja harvinaisemmat tapaukset täydentyvät. Komennot ja tiedostomuodot voivat vielä muuttua versioiden välillä.

**Valmis oikeaan kirjanpitoon:** tiliotteen tuonti, tavalliset ostotositteet ALV:neen, kuukauden sulku, kuukausiraportit, ALV-ilmoitus (suoraan Verohallinnolle tai käsin) sekä verovapaat matkakulukorvaukset (kilometrikorvaukset ja päivärahat) Tulorekisteriin.

**Ei vielä valmis:** palkanmaksu ja sen ilmoittaminen, aggountantin oma tositteiden haku ja tilinpäätös. Komennot ovat olemassa, mutta niitä ei pidä käyttää oikeaan kirjanpitoon. Käyttöopas kertoo, mitä tehdä sen sijaan.

aggountant on tarkoitettu yhdelle ihmiselle, joka hoitaa yhden yrityksen kirjanpitoa omalla koneellaan. Se ei ole verkkopalvelu eikä monen käyttäjän järjestelmä, eikä se tee ilmoituksia muiden yritysten puolesta.

## Tavoitteet

Ensimmäinen käyttökohde on suomalaisen pienen osakeyhtiön kirjanpito. Pitkän aikavälin tavoite on yleiskäyttöinen, kansainvälinen taloushallinnon työkalukokonaisuus tekoälyagenteille.

- **Tilinpäätös.** Kun kirjanpito on hoidettu huolellisesti pitkin vuotta, tilinpäätöksen pitäisi onnistua pitkälti keskustelemalla agentin kanssa ja tarvittaessa antaa kirjanpitäjän tarkistettavaksi.
- **Palkanlaskenta.** Palkat, palkkalaskelmat, maksujen valmistelu sekä vero- ja Tulorekisteri-ilmoitukset ilman erillistä palkanlaskentapalvelua.
- **Kansainvälinen myynti.** Digitaalisten tuotteiden myynnin verovelvoitteet eri maissa omalla agentilla ilman Merchant of Record -palvelua, kuten Paddle tai Lemon Squeezy.
- **Kansainväliset työsuhteet.** Ulkomailla työskentelevien palkkahallinto ja työnantajavelvoitteet ilman Employer of Record -palvelua, kuten Deel tai Remote, siltä osin kuin hallinnollinen työ voidaan automatisoida.

aggountant on avointa lähdekoodia, koska lakisääteiset velvoitteet koskevat jokaista yritystä, eikä niiden automatisoinnin perustan tarvitse olla yksittäisten palveluntarjoajien hallussa. Avoimuus tarkoittaa myös, että kirjanpidon säännöt, tietomallit ja tarkistukset ovat kenen tahansa tutkittavissa. Tarkoitus ei ole korvata taloushallinnon asiantuntijoita, vaan vapauttaa heidät rutiinityöstä sinne, missä heidän osaamisestaan on eniten hyötyä.

## Asennus
<!-- shipshape-readme:install-start -->
Valmiit binäärit ovat saatavilla macOS arm64:lle ja Linux arm64/x86_64:lle (staattinen musl). Tämä asentaa uusimman [julkaisun](https://github.com/aggountant/aggountant/releases) oikean paketin:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
```

Jokaisen julkaisun paketeilla on SHA-256-tarkistussummat. Voit myös kääntää ohjelman lähdekoodista Rust 1.89:llä tai uudemmalla (<https://rustup.rs>):

```sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
```
<!-- shipshape-readme:install-end -->

## Käytön aloitus

Helpointa on sanoa agentillesi, että haluat aloittaa aggountantin käytön, ja antaa sille linkki tähän repoon: <https://github.com/aggountant/aggountant>. Agentti asentaa ohjelman ja skillit eli ohjetiedostot, jotka opettavat sen toimimaan kirjanpitäjänä, ja perustaa yrityksesi kirjanpidon. Se luo yrityksen hakemiston, tallentaa perustiedot ja avaussaldot ja kirjaa, mistä tiliotteet ja tositteet tulevat. Suora ALV-ilmoittaminen Verohallinnolle edellyttää yrityksen omaa ohjelmistorekisteröintiä ja varmennetta, ja agentti opastaa siinä, kun olet valmis.

Skillit asennetaan agentin skill-hakemistoon (oletuksena `~/.claude/skills`, muualle `--target`-valitsimella):

```sh
aggountant skill list
aggountant skill install
```

Käyttöopas kertoo kuukausikierroksesta, ALV:stä, matkakulukorvauksista, tilikauden tilasta ja virheistä toipumisesta: <https://aggountant.dev>. Jokainen komento myös dokumentoi itsensä: `aggountant --help`.

## Osallistu

Etsimme varhaisia käyttäjiä ja kehittäjiä. Jos hoidat pientä suomalaista yritystä ja haluat pitää kirjanpitosi näin, voimme käydä tilanteesi läpi yhdessä: [Tule varhaiseksi käyttäjäksi](docs/user/fi/start/early-users.md).

Virheraportit, kysymykset ja pull requestit ovat tervetulleita [GitHub Issuesiin](https://github.com/aggountant/aggountant/issues) ja [Discussionsiin](https://github.com/aggountant/aggountant/discussions). Katso [CONTRIBUTING.md](CONTRIBUTING.md); commitit allekirjoitetaan Developer Certificate of Originin mukaisesti. Ilmoita tietoturvahaavoittuvuuksista yksityisesti [SECURITY.md](SECURITY.md):n ohjeen mukaan.

## Lisenssi
<!-- shipshape-readme:license-start -->
Licensed under the [MIT License](LICENSE). Copyright (c) 2026 Jari Mustonen.
<!-- shipshape-readme:license-end -->
