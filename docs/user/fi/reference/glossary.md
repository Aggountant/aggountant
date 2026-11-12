# Sanasto

Tämä sanasto selittää kirjanpidon ja aggountantin sanat, joihin törmäät
käytössä. Termit ovat aakkosjärjestyksessä. Suluissa on englanninkielinen nimi
silloin, kun se näkyy ohjelman tulosteessa.

**Agentti.** Tekoälyavustaja (esimerkiksi Claude Code), joka lukee aineiston,
ehdottaa kirjaukset ja ajaa aggountantin komentoja puolestasi. Agentti ei sinetöi
eikä lähetä mitään ilman hyväksyntääsi.

**Ajo (run).** Monivaiheinen tehtävä, joka tallentaa etenemisensä ja jota voi
jatkaa keskeytyksen jälkeen, esimerkiksi kauden sulku tai ALV-ilmoitus. Ajolla
on tunniste, kuten `cp-…` (sulku), `fv-…` (ALV-ilmoitus), `wr-…`
(Tulorekisteri-ilmoitus) tai `cy-…` (tilinpäätös).

**ALV-ilmoitus.** Kauden arvonlisäverotiedot Verohallinnolle numeroiduilla
kentillä. aggountantissa ilmoitus tehdään suljetun kauden luvuista joko suoraan
rajapinnan kautta tai käsin OmaVerossa.

**ALV-kanta.** Arvonlisäveron prosentti. Kanta riippuu tuotteesta ja tositteen
päivästä: yleinen kanta on 25,5 % 1.9.2024 alkaen. aggountant valitsee kannan
kuitin tai laskun päivän perusteella, ei maksupäivän.

**ALV-kausi.** Ajanjakso, jolta ALV ilmoitetaan. aggountantissa se on sama kuin
kirjanpidon sulkukausi: kuukausi tai neljännesvuosi.

**Arvonlisävero (ALV).** Vero, jonka yritys perii myynnistään ja jonka se saa
vähentää ostoistaan. Erotus maksetaan Verohallinnolle tai saadaan palautuksena.

**Avaavat saldot.** Tilien saldot sinä päivänä, jona kirjanpito siirtyy
aggountantiin toisesta järjestelmästä. Ne kirjataan yhdeksi tositteeksi.

**Avoin (open).** Kauden tila, jossa kaudelle voi vielä kirjata.

**camt.053 ja camt.052.** Pankkien sähköisen tiliotteen tiedostomuodot.
aggountant tuo tapahtumat niistä tositteiksi.

**Debet ja kredit.** Kahdenkertaisen kirjanpidon kaksi puolta. Jokaisessa
tositteessa debet- ja kredit-vientien summat ovat yhtä suuret.

**Epävarma lopputulos (outcome unknown).** Tilanne, jossa ilmoituksen
lähetyksen aikana yhteys katkesi tai palvelu vastasi virheellä, eikä tiedetä,
menikö ilmoitus perille. Ilmoitusta ei lähetetä uudelleen, vaan tila kysytään
viranomaiselta.

**Erikoiskirjaus.** Tosite, jonka agentti kokoaa vienti kerrallaan, koska tiliotteen tuonti tai kuittireitti ei sovi siihen, esimerkiksi myyntilasku, palkkatosite, avaavat saldot tai korjaus.

**Esikatselu.** Komennon ajo niin, että se näyttää lopputuloksen mutta ei
tallenna mitään. Hyväksyt esikatselun ennen varsinaista tallennusta.

**Harjoituskoti.** Erillinen yrityskoti, jossa harjoitellaan oppaan harjoitusaineistolla. Se ei liity oikeaan kirjanpitoon, ja sen voi poistaa.

**Hyväksyntä.** Nimenomainen päätöksesi ennen pysyvää tai oikeusvaikutteista
tekoa. Hyväksyntä on sidottu siihen, mitä sinulle näytettiin. Jos sisältö
muuttuu, hyväksyntä ei enää kelpaa.

**Hyväksyntätestit.** Verohallinnon API-portaalin testit, jotka ohjelmiston
pitää läpäistä ennen tuotantokäyttöä. aggountant ajaa ne komennolla
`vero-fi verify-portal`.

**Ilmoitin.fi.** Verohallinnon palvelu ilmoitustiedostojen lähettämiseen.
aggountant ei laadi Ilmoitin.fi-tiedostoja.

**Jaksotus.** Tulon tai menon kirjaaminen sille kaudelle, jolle se kuuluu,
vaikka raha liikkuisi toisella kaudella.

**Kattavuus (completeness).** Kuukausiraportin merkintä siitä, onko kaikki aineisto kirjattu. Se on aina "varmistamaton", koska ohjelma ei voi tietää sitä. Sen arvioit sinä ja agentti lähteitä vasten.

**Kauden sulku.** Kauden kirjanpidon lukitseminen. Sulun jälkeen kaudelle ei voi
kirjata, ja ALV-ilmoitus tehdään sen luvuista.

**Kausi.** Kirjanpidon sulkujakso: kuukausi (`2026-03`) tai neljännesvuosi
(`2026-Q1`).

**Kilometrikorvaus.** Verovapaa korvaus oman auton käytöstä työmatkoilla.
Tulorekisterin tulolaji 311.

**Korjaustosite.** Uusi tosite, jolla aiemman tositteen virhe korjataan.
Alkuperäistä ei muuteta eikä poisteta.

**Korvaava ilmoitus.** Uusi ilmoitus, joka korvaa jo annetun ilmoituksen.
aggountant ei tee korvaavia ilmoituksia. Ne tehdään OmaVerossa tai
Tulorekisterin asiointipalvelussa.

**Kuittaus.** Verohallinnon vastaus siitä, että se on vastaanottanut
ilmoituksen. Kuittaus ei vielä tarkoita, että ilmoitus näkyisi OmaVerossa.

**Käännetty verovelvollisuus.** ALV-menettely, jossa ostaja eikä myyjä tilittää
veron, esimerkiksi ostettaessa palveluja toisesta EU-maasta.

**Liite.** Tositteeseen arkistoitu alkuperäinen aineisto, kuten kuitti, lasku tai
tiliote. aggountant tunnistaa saman tiedoston uudelleen, jos se yritetään
kirjata toiseen kertaan.

**Liitetiedot.** Tilinpäätökseen kuuluvat lisätiedot laskelmien rinnalla.

**Ohjelmistoavain.** Verohallinnon API-portaalin antama avain, joka yksilöi
rekisteröidyn ohjelmiston. Jokainen aggountantin käyttäjä rekisteröi oman
ohjelmistonsa ja saa oman avaimensa. Avain on salainen.

**OmaVero.** Verohallinnon verkkoasiointipalvelu. Siellä näet ilmoitukset ja
maksat veron.

**Poisto.** Pitkävaikutteisen hankinnan, kuten koneen tai laitteen, hankintamenon
jakaminen kuluksi usealle vuodelle.

**Päiväraha.** Verovapaa korvaus työmatkan aiheuttamista elinkustannuksista.
Tulorekisterin tulolaji 331.

**Pääomaosakkuus.** Tieto siitä, että tulonsaaja omistaa osan yrityksestä.
Tulorekisteri-ilmoituksessa se kerrotaan aina erikseen, eikä aggountant päättele
sitä.

**Sinetöinti.** Tositteen pysyvä tallennus. Sinetöity tosite saa numeron ja
liitetään tositeketjuun, eikä sitä voi enää muuttaa.

**Skilli.** aggountantin mukana tuleva toimintaohje agentille, esimerkiksi
`capture-receipt` kuittien käsittelyyn.

**Sormenjälki.** Varmenteen yksilöivä tunniste (SHA-256). Sormenjälki ei ole
salainen, ja sillä voit kertoa esimerkiksi Verohallinnon tuelle, mistä
varmenteesta on kyse.

**Suljettavana (closing).** Kauden tila sulun aloittamisen ja hyväksynnän
välillä. Tässä tilassa kaudelle ei voi kirjata.

**Suljettu (sealed).** Kauden tai tilikauden lopullinen tila. Suljetulle kaudelle
ei voi kirjata, ja korjaukset tehdään seuraavalle avoimelle kaudelle.

**Tase.** Laskelma yrityksen varoista, velasta ja omasta pääomasta tiettynä
päivänä.

**Tase-erittely.** Erittely taseen tilien sisällöstä tilinpäätöshetkellä.

**Tili.** Tilikartan kohta, jolle vienti kirjataan, esimerkiksi 1910 Pankkitili
tai 7690 Atk-laite- ja ohjelmistokulut.

**Tilikartta.** Yrityksen kirjanpidon tililuettelo. `init` luo oletustilikartan,
jonka näet komennolla `aggountant chart list`.

**Tilikausi.** Vuosi, jolta tilinpäätös tehdään. aggountantissa tilikausi on
kalenterivuosi.

**Tilinpäätös.** Tilikauden päätteeksi laadittava tuloslaskelma, tase ja
liitetiedot.

**Tosite.** Kirjanpidon perusyksikkö: yksi tapahtuma vienteineen, päivineen ja
kuvauksineen sekä tarvittaessa liite. aggountantissa tositteet numeroidaan
aukottomasti.

**Tositeketju.** Sinetöidyt tositteet ketjutettuina niin, että jokainen sisältää
tunnisteen edellisestä. Jos jotain tositetta muutetaan, ketjun tarkistus
(`voucher verify`) huomaa sen.

**Tulolaji.** Tulorekisterin koodi, joka kertoo maksun lajin, esimerkiksi 311
kilometrikorvaus tai 331 päiväraha.

**Tulorekisteri.** Kansallinen rekisteri, johon ilmoitetaan palkat, palkkiot ja
korvaukset.

**Tuloslaskelma.** Laskelma kauden tuotoista, kuluista ja tuloksesta.

**Tuotantolupa.** Verohallinnon lupa käyttää rajapintaa oikeisiin
ilmoituksiin hyväksyntätestien jälkeen.

**Vahvistamaton tapahtuma.** Tiliotteen rivi, jota pankki ei ole vielä kirjannut, esimerkiksi korttivaraus. aggountant ei tuo sitä, vaan se kirjataan siltä tiliotteelta, jolla se on toteutunut.

**Vakuutus.** Vahvistuksesi asiasta, jota ohjelma ei voi itse tarkistaa,
esimerkiksi että ohjelmisto on rekisteröity tai tuotantolupa myönnetty. Ohjelma
luottaa vakuutukseesi.

**Valmius.** Vero.fi-yhteyden tila, jonka `aggountant vero-fi status` laskee.
Suora ilmoitus on mahdollinen vain, kun valmius on kunnossa (`ready`).

**Varmenne.** Verohallinnon myöntämä sähköinen todistus, jolla yrityksesi
tunnistautuu rajapintaan. Varmenne on voimassa kaksi vuotta. Sen yksityinen
avain on salainen.

**Velvoite.** Avoin asia määräpäivineen, esimerkiksi puuttuva ajopäiväkirja.
Avoin velvoite estää kautensa sulun.

**Vero API.** Verohallinnon rajapinta, jonka kautta aggountant lähettää
ALV-ilmoituksen suoraan.

**Vienti.** Tositteen yksi rivi: tili, summa ja puoli (debet tai kredit).

**Y-tunnus.** Yrityksen tunnus kaupparekisterissä ja verotuksessa.

**Yrityksen tietovarasto.** Kodin `knowledge/company/`-hakemisto, johon agentti
tallentaa yrityksen faktat ja käytännöt lähteineen ja historioineen.

**Yrityskoti.** Hakemisto, jossa on yhden yrityksen koko kirjanpito: tilikartta,
tositteet, liitteet, kausien tila ja asetukset.

## Katso myös

- [Komennot lyhyesti](commands.md)
- [Virhekoodit](error-codes.md)
