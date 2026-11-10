# ADR-5 kirjanpitäjälle — milloin tosite sinetöidään ja miten korjataan

Tämä on **yhteenveto** ADR-5:n päätöksestä kirjanpidon ammattilaiselle: mitä
päätettiin ja miksi, kirjanpitäjän termein. Varsinainen päätös perusteluineen on
`adr/ADR-5-seal-timing.md`; tämä sivu ei päätä mitään uutta.

## Mistä tässä on kyse

aggountant **sinetöi jokaisen tositteen heti kirjattaessa** muuttumattomaan
hash-ketjuun (seal-on-record, päätetty D-05/D-06). Sinetöityä tositetta ei voi enää
muokata — virhe korjataan **oikaisutositteella**, ei vanhaa muuttamalla. Tämä on
lain mukaista: kirjausketju (KPL 2:6) ja korjauksen jäljitettävyys (KPL 2:5) pitävät
koko ajan, ja muuttamattomuus (KPL 2:7) sitoo viimeistään tilinpäätöstä laadittaessa.
Markkinan malli (Kitsas) muokkaa vapaasti kauden ollessa auki ja kirjaa muutokset
`Tositeloki`-lokiin; aggountant sen sijaan **ei muuta koskaan** vaan tekee uuden
tositteen. Molemmat ovat laillisia.

Sinetöinnin ainoa aito käytettävyyskustannus on **sinettiä edeltävä
muokkausvaihe**: mihin jää kuitti, jonka tiliöinti tai ALV-käsittely ei ole vielä
selvillä? Tämä ADR ratkaisee sen (avoin kohta O-29).

## Mitä päätettiin

**1. Tositteelle tulee ensiluokkainen sinettiä edeltävä käsittelyvaihe — mutta se on
*saapunut/käsittelyssä oleva aineisto*, ei kirjaus.** Kuitti voidaan ottaa vastaan ja
pitää **käsittelyssä tunteja tai päiviä**: agentti tallentaa liitteen, ehdottaa
tiliöinnin ja jättää avoimet kysymykset auki, kunnes tiedot selviävät. Tämä
**käsittelyssä oleva aineisto ei ole kirjanpidossa**: sillä ei ole tositenumeroa
eikä sinettiä, eikä se näy missään saldossa, ALV-laskelmassa, raportissa tai kauden
täsmäytyksessä. Kun aineisto hyväksytään, se **sinetöidään heti** — välitilaa
"hyväksytty mutta sinetöimätön" ei ole. Selkeät tilat: *saapunut*, *täsmennystä
vailla*, *valmis hyväksyttäväksi*. Kiellettyjä: "kirjattu luonnos" tai mikä tahansa
joka antaa ymmärtää, että summa on jo mukana kirjanpidossa.

Näin **kirjaa selvä kuitti heti yhdellä vahvistuksella**, mutta epäselvä jää
näkyvään "työjonoon" (esim. kuittien metsästys, A5) sen sijaan että se pakotettaisiin
arvaamalla kirjanpitoon tai jäisi kokonaan järjestelmän ulkopuolelle.

**2. Korjaaminen: sinetöityä ei "muokata", vaan tehdään oikaisutosite — agentti
hoitaa mekaniikan, mutta kertoo aina mitä syntyi.** Käyttäjä sanoo luonnollisella
kielellä ("tuo viime viikon kuitti oli henkilökohtainen, korjaa se"), ja agentti
rakentaa oikaisutositteen (`voucher correct`, tavat: **oikaisu / peruutus /
tarkennus**), joka linkittyy alkuperäiseen. **Ennen sinetöintiä** agentti näyttää:
"En muuta tositetta #123. Teen oikaisutositteen #171, joka linkittyy #123:een.
Nettovaikutus: 7690 −124,00 €, 7790 +124,00 €. Sinetöidäänkö?" ja **jälkeenpäin**
nimeää tuloksen. Sana **"muokkaa" on kielletty** sinetöidyn tositteen kohdalla.

Tämä on tietoinen keskitie: liian näkymätön korjaus ("korjasin sen") rikkoo
käyttäjän mielikuvan heti kun päiväkirjassa näkyy kaksi tositetta ja heikentää
audit-luottamusta; liian seremoniallinen (käyttäjä joutuu itse rakentamaan
vastakirjaukset) hävää Kitsaalle arkifriktiossa. **Sujuva teko, läpinäkyvä jälki.**

**3. Milloin luonnos on pakko sinetöidä.** Käsittelyssä oleva aineisto on
**sinetöitävä tai nimenomaisesti ratkaistava** (`discard --reason …`, siirto
seuraavalle kaudelle perusteella) ennen kuin se voi vaikuttaa mihinkään ulos
lähtevään lukuun — tai jäädä siitä pois. Ledger **estää**:
- **kausisulun** (`close-period`), jos kaudelle kuuluvaa aineistoa on ratkaisematta;
- **ALV-ilmoituksen** — kausi on oltava `Sealed`, ALV lasketaan vain sinetöidyistä;
- **tilikauden sulun** — oikaisut sinetöidään ennen tilinpäätöksen hyväksyntää.

Erityisesti: **tunnettua tapahtumaa (esim. pankista lähtenyt maksu) ei saa jättää
pois** vain siksi, että tiliöinti on epäselvä — se sinetöidään perustellulla
väliaikaiskäsittelyllä (selvittely-/suspense-tili) ja korjataan myöhemmin.
("Epäselvä ALV" ei ole peruste jättää tunnettua pankkitapahtumaa kirjaamatta.) S1:n
viiden tilin kartta ei vielä osaa väliaikaistiliä — tämä on kirjattu jatkokohdaksi
(O-34).

**4. O-29-muutosloki otetaan käyttöön.** Koska aineisto elää sinettiä edeltävässä
vaiheessa, sen **hyväksytyt muutokset** kirjataan omaan, peukaloinnin paljastavaan
lokiinsa (audit-pariteetti `Tositeloki`n kanssa, ja jotta agenttikaan ei voi hiljaa
kirjoittaa historiaa uusiksi). Loki on kevyt mutta hash-ketjutettu — samaa
mekaniikkaa kuin nykyinen hylkäysloki — ja siihen kirjataan hyväksytyt versiot, ei
jokaista agentin väliehdotusta. Raaka teksti (esim. kuitin OCR) pysyy salatussa
aineistossa, lokiin vain rakenteinen muutos.

## Mitä tämä tarkoittaa käytännössä

- **Raportit netottavat korjaukset siististi.** Käyttäjän näkymässä oikaisu näkyy
  muodossa "korjattu arvoon X" (porautuen molempiin tositteisiin), mutta
  **päiväkirja, pääkirja ja arkisto säilyttävät kaikki tositteet** — mitään ei
  poisteta lakisääteisistä kirjoista.
- **ALV-korjaus ilmoituksen jälkeen** tuottaa kaksi seurausta: kirjanpidon
  oikaisutositteen **ja** mahdollisen korjausilmoituksen/seuraavan kauden oikaisun
  OmaVeroon. Agentti näyttää molemmat eikä anna ymmärtää, että kirjanpidon
  korjaaminen olisi korjannut ilmoituksen.
- **Vanhat ja kohdentamattomat viennit** eivät muutu miksikään — käsittelyvaihe on
  vain uusi, näkyvä paikka keskeneräiselle aineistolle, ei osa kirjanpitoa.

## Mitä tässä ei vielä ratkaistu

- Väliaikais-/selvittelytilit provisionaalista kirjausta varten (O-34).
- Hylätyn/keskeneräisen aineiston säilytys- ja poistokäytäntö, kun siinä on
  henkilötietoa (O-35).
- Ehdotetut skenaariolisäykset (kuittien käsittelyjono, sinetöidyn tositteen oikaisu,
  ilmoitetun ALV-kauden korjaus, tilinpäätöstä edeltävät uudelleenluokittelut) on
  kirjattu suosituksiksi, ei viety `scenarios.md`:hen tässä.

**Lopputulos:** epäselvä kuitti saa näkyvän kodin ilman että kirjanpitoon syntyy
arvausta, korjaus on käyttäjälle yhden lauseen työ mutta jättää rehellisen
kaksois­tosite-jäljen, eikä sinettiä edeltävä vaihe voi muuttua paikaksi, jonne
todellisia velvoitteita piilotetaan ketjun ulkopuolelle.
