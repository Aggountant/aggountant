---
name: close-year
description: >-
  Päätä tilikausi tilinpäätösajolla: kirjaa päättyneen vuoden poistot, jaksotukset
  ja oikaisut, käy käyttäjän kanssa läpi tase-erittelyt, liitetiedot ja laskelmat
  ja sinetöi tilikausi. Käytä, kun vuoden kuukaudet on suljettu ja tilinpäätöstä
  tehdään. Kuukauden sulku ja ALV-ilmoitus eivät kuulu tähän.
cli_version: "0.1.0"
schema_version: 1
---

# Tilikauden päättäminen

Lue ensin [SOUL.md](../aggountant-init/SOUL.md): rooli, kirjanpitokodin pelisäännöt,
mikä tässä työssä on pysyvää ja miten työkalua luetaan.

Tilinpäätösajo (`aggountant close-year`) tekee kaksi asiaa: se vie tilinpäätöksen
kirjaukset päättyneelle vuodelle, jonka kuukaudet on jo suljettu, ja sinetöi
tilikauden. Virallinen tilinpäätösasiakirja sanallisine liitetietoineen,
allekirjoitukset, kaupparekisteri-ilmoitus ja veroilmoitus tehdään ohjelman
ulkopuolella. Hyvä lopputulos on sinetöity vuosi, jonka tuloslaskelma ja tase ovat
samat kuin siinä tilinpäätöksessä, jonka käyttäjä allekirjoittaa.

## Milloin vuosi sinetöidään

Sinetöityyn vuoteen ei pääse enää mitään: myöhemmin löytyvä kirjaus jää seuraavan
vuoden kirjanpitoon, ja kirjanpito ja allekirjoitettu tilinpäätös eroavat pysyvästi.
Luvut tulevat tilinpäätöksen tekijältä, joka on tilitoimisto, kirjanpitäjä tai
käyttäjä itse. Sovi hänen kanssaan hyödykkeet, jaksotukset ja oikaisut ennen ajon
käynnistämistä, ja vie ajo loppuun vasta, kun tekijän luvut ovat lopulliset. Kun
käyttäjä laatii tilinpäätöksen itse, ajon laskelmat ovat sen luvut, ja asiakirja
viimeistellään niistä ennen viimeistä hyväksyntää.

Odottamisella on hinta, joka kannattaa kertoa käyttäjälle: uuden vuoden
kuukausiraportit kieltäytyvät (`PERIOD_NOT_SEALED`), kunnes edellinen vuosi on
sinetöity, koska avoimen vuoden loppusaldot voivat vielä muuttua. Uuden vuoden
kirjaaminen, kuukausien sulku ja ALV-ilmoitukset jatkuvat sillä välin tavallisesti.
Raporttien takia vuotta ei kannata sinetöidä keskeneräisenä.

## Mitä ajolle annetaan

Vain ajon omat kirjaukset kohdistuvat päättyneelle vuodelle. Ajo kirjaa ne
seuraavan vuoden ensimmäiselle avoimelle kaudelle ja merkitsee ne päättyneen vuoden
kirjauksiksi, joten ne näkyvät oikean vuoden tuloksessa ja seuraava vuosi alkaa
saldoista, joissa ne ovat mukana. Tavallisena tositteena uudelle vuodelle kirjattu
poisto tai jaksotus jäisi uuden vuoden tulokseen. Siksi kaikki, mikä kuuluu
päättyneelle vuodelle, annetaan käynnistyksen syötteessä; kesken ajon sitä ei voi
täydentää.

Käynnistys (`close-year start`) ei vielä kirjaa mitään. Se laskee poistot ja näyttää
ne, joten voit tarkistaa laskelman tekijän lukuja vasten. Ajon, joka ei ole
sinetöinyt mitään, voi hylätä jälkiä jättämättä.

**Poistot.** Ohjelma laskee poiston itse hyödykeluokan enimmäisprosentilla, eikä
määrää voi antaa. Jos yritys poistaa vähemmän tai muulla suunnitelmalla, jätä hyödyke
pois hyödykkeistä ja anna sen poisto oikaisuna. Ohjelmassa ei ole käyttöomaisuuden
rekisteriä: hyödykkeet ja niiden menojäännös vuoden alussa annetaan joka vuosi
uudelleen, eikä ohjelma voi huomata väärää menojäännöstä. Ota ne edellisestä
tilinpäätöksestä tai `knowledge company` -tietueesta, ja tallenna sinetöinnin
jälkeen hyödykkeet ja niiden menojäännös vuoden lopussa sinne seuraavaa vuotta varten.

**Jaksotukset.** Ajo ei pura jaksotusta seuraavana vuonna. Jaksotettu lasku tai
maksu kirjataan uudella vuodella jaksotustiliä vasten, ei uudelleen kuluksi.
Tilinpäätös tehdään kuukausia vuodenvaihteen jälkeen, joten lasku on usein jo
ehditty kirjata uuden vuoden kuluksi: tarkista se ennen käynnistystä ja korjaa
uuden vuoden kirjaus, muuten sama meno on kuluna kahtena vuonna. Kirjaa vielä
purkua odottavat jaksotukset kodin `TODO.md`-tiedostoon, kun ne on sinetöity.

**Oikaisut.** Oikaisu on tekijän antama vienti, jota ohjelma ei laske: uudelleenluokittelu,
kuukausilta löytynyt virhe tai itse laskettu poisto. Jokainen oikaisu sinetöidään
erikseen komennolla `close-year seal-correction` ennen ensimmäistä hyväksyntää. Se on
sinetöinti, joten näytä oikaisu käyttäjälle ja pyydä hyväksyntä ennen komentoa.

## Mitä käyttäjä päättää missäkin vaiheessa

Ajo pysähtyy neljästi. `close-year show` näyttää avoimen vaiheen sisällön juuri
sellaisena kuin hyväksyntä sen sitoo; näytä se käyttäjälle, älä omaa yhteenvetoasi
muistista. Kaksi vaiheista on pysyviä tekoja.

1. **Oikaisukirjaukset** (`ApproveAdjustments`). Ovatko nämä vuoden poistot ja
   jaksotukset, oikeilla tileillä ja summilla, ja onko jokainen oikaisu mukana.
   Hyväksyntä sinetöi poisto- ja jaksotustositteet.
2. **Tase-erittelyt** (`ApproveTaseErittelyt`). Vastaako jokaisen tasetilin
   loppusaldo todellisuutta: pankkitili tiliotetta tilikauden viimeisenä päivänä,
   saamiset ja velat oikeita avoimia eriä, ALV-velka ilmoitettua. Ajo ei tarkista,
   että vuoden ALV-ilmoitukset on annettu: katso se itse (`file-vat status`), ja
   puuttuva ilmoitus annetaan ennen kuin vuosi sinetöidään. Tässä kuukausien aikana huomaamatta
   jääneet virheet tulevat näkyviin, ja käyttäjä tietää saldoista sen, mitä
   aineistosta ei näe.
3. **Liitetiedot** (`ApproveLiitetiedot`). Sisältönä ovat vain ohjelman laskemien
   poistojen menetelmät ja prosentit; oikaisuna annettu poisto ei näy niissä, ja
   sanalliset liitetiedot tehdään muualla. Käyttäjä vahvistaa, että näytetty ei ole
   ristiriidassa tilinpäätösasiakirjan kanssa.
4. **Laskelmat** (`ApproveStatements`). Vertaa tuloslaskelmaa ja tasetta tekijän
   tilinpäätökseen ennen kuin pyydät hyväksyntää. Erilainen rivijako tai pyöristys
   ei haittaa, kunhan ero on selitetty; selittämätön ero selvitetään ensin.
   Hyväksyntä sinetöi tilikauden, eikä sitä voi perua.

## Hylkäys ei peru sinetöityä

Vaiheen hylkääminen päättää ajon, mutta sen jo sinetöimät tositteet jäävät
kirjanpitoon. Kun vaiheessa löytyy virhe, hylkää vaihe ja käynnistä uusi ajo.
Sinetöimättömän kohdan korjaat suoraan syötteeseen. Jo sinetöity kohta annetaan
uudelle ajolle samalla tunnisteella, samoilla tileillä ja samalla summalla, koska
ohjelma tunnistaa tositteen tunnisteesta; jos se oli väärin, se perutaan uudella
vastaoikaisulla eikä jätetä pois. Säilytä siksi käynnistyksen syöte. Ohjelma nimeää
kirjaukset, joita uusi ajo ei kata, ja kieltäytyy näyttämästä ja hyväksymästä
vaihetta, jos kirjanpito on muuttunut sen avautumisen jälkeen.

Keskeytynyt ajo on eri asia: sitä ei hylätä eikä aloiteta alusta, vaan jatketaan
komennolla `close-year resume`. Jatkaminen ei ole lukemista, sillä se vie loppuun
sen, minkä käyttäjä ehti hyväksyä, myös tilikauden sinetöinnin.

Tästä seuraa työjärjestys: käy tase-erittelyjen kysymykset käyttäjän ja tekijän
kanssa läpi jo ennen käynnistystä viimeisen kuukauden raportista, jotta virheet
ehtivät ensimmäisen ajon oikaisuiksi.

## Sinetöinnin jälkeen

Ajo kirjoittaa hyväksytyt laskelmat, tase-erittelyt ja liitetiedot kotiin
hakemistoon `archive/<vuosi>/`. Anna ne tekijälle, jos hän tarvitsee ne, ja
commitoi koti. Jos tekijä sinetöinnin jälkeen vielä muuttaa lukujaan, kerro
käyttäjälle, että muutos voidaan kirjata vain seuraavalle vuodelle.
