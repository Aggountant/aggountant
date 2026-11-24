# ADR-11 kirjanpitäjälle: kululasku on matkoja ja rivejä, joista hyväksyntä sinetöi tositteet

Tämä on yhteenveto ADR-11:n päätöksestä kirjanpidon ammattilaiselle. Varsinainen päätös on [ADR-11](adr-11-expense-claim); tämä sivu ei päätä mitään uutta.

## Mitä päätettiin

Kululasku on yhden hakijan korvaushakemus, jossa on matkoja ja rivejä. Sitä ei ole sidottu kuukauteen: lasku voi koota kuukauden tai yhden matkan. Hakija voi olla työntekijä, toimitusjohtaja, hallituksen jäsen tai palkaton osakas. Konsultin kulut kuuluvat hänen omaan laskuunsa, joten konsultille ei tehdä kululaskua. Toiminimi on rajattu pois.

- **Matka** sisältää matkalaskun lakisääteiset tiedot (ennakkoperintäasetus 17 §): tarkoitus, kohde, tarvittaessa reitti, kulkutapa, lähtö- ja paluuaika, ulkomailla maa jokaisen matkavuorokauden päättyessä sekä ajot kilometreineen. Matkalle kirjataan myös ilmaiset ateriat ja yrityksen maksamat ateriat matkavuorokausittain, koska ne pienentävät päivärahaa.
- **Kaavamaiset korvaukset** (kilometrikorvaus, päiväraha, ateriakorvaus, yömatkaraha) ohjelma laskee matkasta Verohallinnon vuosittaisen päätöksen taulukosta, myös ulkomaan päivärahat maittain. Niitä ei kirjoiteta käsin. Yritys voi maksaa vähemmän, mutta ei enempää: enimmäismäärän ylitys olisi palkkaa, ja ohjelma estää sen, kunnes tuotteessa on palkanlaskenta.
- **Kuittirivit** ovat hakijan maksamia yrityksen kuluja kuitteineen. Korttimaksukuitti ilman ALV-erittelyä, kuititon kulu (selvitys ja hakijan vakuutus), ulkomainen kuitti ja edustus eivät tuota ALV-vähennystä.

## Varsinainen työpaikka

Ohjelma ei laske matkakorvauksia verovapaiksi, ennen kuin hakijalle on kirjattu varsinainen työpaikka (paikka, jossa hän tekee työtä vakituisesti) matkan päivälle. Tieto on päivätty merkintä henkilötietokannassa, ja jokainen matka ottaa siitä tilannekuvan, joka sinetöidään matkan mukana. Jos merkintää myöhemmin korjataan taaksepäin, ohjelma luettelee hyväksytyt laskut, joihin korjaus vaikuttaa, mutta ei muuta niitä itse.

Ohjelma seuraa myös kolmen vuoden sääntöä: kun asiakkaan tilat ovat pääasiallinen työskentelypaikka (yli puolet työpäivistä tai työajasta), se laskee viimeisen verovapaan päivän ja kieltäytyy sen jälkeen. Pääasiallisuuden arvioi ihminen.

## Hyväksyntä ja kirjaukset

Hyväksyntä sidotaan täsmälleen esikatseltuun sisältöön. Hyväksyjän nimi ja rooli tallennetaan. Oman laskun hyväksyntä on sallittu, ja se näkyy merkintänä "hyväksyjä on hakija". Yhden hengen yhtiössä erillistä varmentajaa ei voi olla, ja siitä voi kirjata maininnan. Toinen hyväksyjä, esimerkiksi tilitoimisto sähköpostilla, liitetään samaan hyväksyntään näyttöineen.

Hyväksyntä sinetöi samalla kertaa:

- jokaisesta kuitista oman tositteen kuitin päivälle, ALV-purkuineen kuten kuitin kirjauksessa, vastatilinä velka hakijalle
- jokaiselta kuukaudelta, jona laskun matka päättyy, yhden tositteen kaavamaisista korvauksista kuukauden viimeiselle päivälle (suoriteperuste).

Jokaisen tositteen liitteenä on hyväksytty kululaskuasiakirja, joka sisältää matkat, rivit, laskelman, sovelletun taulukon ja kuittien tiivisteet. Hyväksytyn laskun sisältöä ei muuteta. Virhe korjataan uudella versiolla, korjaustositteella ja tarvittaessa Tulorekisterin korvaavalla ilmoituksella. Jos korvausta on jo maksettu liikaa, ylitys on Tulorekisterissä veronalaista palkkaa ja perusteetonta etua, jota ohjelma ei vielä osaa ilmoittaa. Ohjelma kirjaa korjauksen ja saatavan hakijalta ja näyttää Tulorekisterin korjauksen tehtäväksi asiointipalvelussa, kunnes sen näyttö on kirjattu.

## Maksu ja Tulorekisteri

Maksu täsmäytetään tiliotteen tuonnissa kululaskuun. Muuten maksettu lasku (esimerkiksi kuittaus saatavaa vastaan) kirjataan omalla komennollaan näyttöineen. Tulorekisteri-ilmoitus tehdään nykyisellä `file-wage`-ilmoituksella hyväksytystä asiakirjasta: kilometrikorvaus 311 ja päiväraha 331. Yksi 331-määrä voi kantaa useamman päivärahatyypin. Kuukauden maksut voi koota yhteen ilmoitukseen, jonka määräpäivä on maksukuukautta seuraavan kuun 5. päivä.

Ohjelma näyttää jokaisesta keskeneräisestä laskusta tilan, määräpäivän ja seuraavan komennon: odottaa hyväksyntää, hyväksytty mutta maksamatta, maksettu mutta ilmoittamatta. Jokaisesta tällaisesta tilasta pääsee pois komennolla, joten mikään ei jää roikkumaan. Hyväksymätön luonnos, jonka matkat osuvat kauteen, estää kauden sulun, ellei sitä käsitellä. Maksamaton tai ilmoittamaton hyväksytty lasku näkyy sulussa varoituksena.

## Edelleenlaskutus

Matkalle ja kuittiriville voi merkitä asiakas- tai projektiviitteen. Ohjelma laskee laskutettavan kustannuspohjan (kuitista ALV:tön osuus, jos ALV vähennettiin, ja kaavamaisesta korvauksesta maksettu määrä). Varsinainen laskutus ja sen hinta kuuluvat laskutukseen, eivät kululaskuun.

## Vanhat tiedostopohjaiset kululaskut

Jos yritys on pitänyt kululaskuja tiedostoina, jo hyväksyttyjä kuukausia ei sinetöidä uudelleen. Ne rekisteröidään sellaisinaan, ja avoin kuukausi tuodaan luonnokseksi, joka kertoo, mitä tietoja puuttuu.

**Lopputulos:** kululasku on matkalasku liitteineen, jonka ohjelma laskee ja joka lukitaan hyväksynnässä. Maksu ja ilmoitus ovat omia tapahtumiaan, ja ohjelma kertoo, mitä niistä puuttuu.
