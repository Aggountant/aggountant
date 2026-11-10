# ADR-8: Yrityksen oma tilikartta adoptoidaan SIE4 `#KONTO` -tuonnilla

- **Status:** Accepted (2026-08-16)
- **Date:** 2026-08-16
- **Authors:** aggountant design-v2. Päätös soveltaa Jari Mustosen 2026-08-14 lukitsemaa tuotelinjaa: sekä aggountantin standardikartta että yrityksen oma kartta tuetaan, standardi on oletus. Tässä ADR:ssä ratkaistaan vain olemassa olevan kartan käyttöönottotapa. Arkkitehti-, kirjanpito-, dataeheys-, ylläpidettävyys- ja onboarding-linssit arvioivat toteutustavan. Product owner: Jari Mustonen.
- **Refines:** ADR-1 (git-natiivi, review-portattu ei-PII-masterdata), ADR-3 (analytiikkareplika on johdettu eikä auktoriteetti), ADR-7 (ALV-jalat ovat todellisia postingeja ja tarvitsevat karttakohtaiset roolit), O-19/O-26 (`chart_version` masterdatassa ja provenienssissa). Liittyy tulevaan `onboarding-authority-adr`:ään: sen suositus **Onb-A** pitää vanhan historian referenssinä ja aloittaa sinetöidyn kirjan live-kaudesta.

---

## Context

Aggountantin oma `fi-general-2026` on uusiin yrityksiin sopiva oletus. Se ei kuitenkaan ole Suomen lakisääteinen numerokartta: KPL edellyttää liiketoimintaan sopivaa ja johdonmukaisesti käytettyä karttaa, ei tiettyjä tilinumeroita. Olemassa oleva yritys tarvitsee siksi oman karttansa, jotta se voi jatkaa tutuilla tileillä ja jotta sen aiempi kirjanpito voidaan toistaa uskollisesti.

Ensimmäisen olemassa olevan yrityksen vanhan kirjanpito-ohjelman aineisto on jo purettu tilikohtaiseksi rakenteeksi. Se osoittaa käytännön eron: yrityksen todellisissa tileissä on numeroita, joita geneerinen kartta ei aina sisällä tai numeroi samoin. Lähdejärjestelmän SIE4-vienti antaa juuri tarvittavan koneellisen rajapinnan: `#KONTO` täydellisen kartan, myös taseen luokat 1–2, sekä `#VER`/`#TRANS`-tositteet. Nykyinen P&L-näkymä ei yksin anna koko karttaa.

Kysymys ei ole enää siitä, tuetaanko kahta polkua. Se on päätetty. Kysymys on: **miten olemassa olevan yrityksen kartta tuodaan aggountantin masterdataksi siten, että kartan sisältö, vuosittainen muutos ja ALV-roolit ovat tarkistettavia eivätkä standardikartan tai historiallisten tositteiden merkitykset muutu?**

Vaihtoehdot:

- **Chart-A:** deterministinen SIE4 `#KONTO` -tuonti yrityskohtaiseen `masterdata/reference`-karttaan.
- **Chart-B:** `init`-komennon uudelleengenerointi yrityksen kartan näköiseksi.
- **Chart-C:** käsin muokattava/agentin rakentama kartta ilman kanonista lähdetuontia.

## Decision

**Valitaan Chart-A. Olemassa olevan yrityksen oma tilikartta adoptoidaan deterministisellä SIE4 `#KONTO` -tuonnilla yrityksen git-natiiviin `masterdata/reference`-masterdataan. `aggountant init` säilyy uuden yrityksen geneerisen `fi-general-2026`-standardikartan bootstrapina eikä sitä uudelleengeneroida yhden yrityksen kartaksi.**

Tuonti on erillinen, eksplisiittinen onboarding-toiminto, ei `init`in sivuvaikutus. Se lukee SIE4:n kartan, validoi ja normalisoi sen fail-closed, kirjoittaa reviewattavan karttasnapshotin sekä johtaa kanonisten JCS-tavujen content-hashin `chart_version`iksi. Jokainen aktiivinen tilikausi sidotaan siihen karttaversioon, jota sille käytetään; kartan muuttuminen tuottaa uuden snapshotin, ei vanhan ylikirjoitusta. `chart_version` leimataan edelleen jokaiseen sinetöityyn tositteeseen O-19/O-26:n mukaisesti.

### 1. SIE4 on kartan lähde, ei välttämättä historian auktoriteetti

SIE4 `#KONTO` on kartan adoptiolähde, koska se kuljettaa kokonaisen, koneellisesti luettavan yrityskartan. Samassa tiedostossa olevat `#VER`/`#TRANS`-rivit ovat arvokasta vertailu- ja analyysiaineistoa, mutta tämä ADR ei sinetöi niitä uudelleen. Se rajaus kuuluu `onboarding-authority-adr`:ään. Kun Onb-A hyväksytään, vanhan järjestelmän historia jää referenssi-/analyysidataksi ja aggountantin auktoritatiivinen sinetöity fiscal alkaa ensimmäisestä live-kaudesta alkusaldojen kautta.

SIE4:n puuttuminen ei oikeuta hiljaiseen arvaamiseen. Tuonnin myöhempi yleinen adapteriraja voi hyväksyä muun yhtä täydellisen, todistettavan karttalähteen, mutta ensimmäisen toteutuksen ensisijainen ja testattava muoto on SIE4. Vajaasta lähteestä, kuten pelkästä P&L-tililuettelosta, voidaan valmistella ehdotus, mutta sitä ei saa aktivoida kartaksi ilman puuttuvien tilien ja pakollisten metatietojen eksplisiittistä tarkistusta.

### 2. Kartta on snapshotteja, ei yksi muokattava nykytila

Tuonti tuottaa yhden sisältöosoitteisen karttasnapshotin per käyttöönotettava karttaversio. Snapshot sisältää vähintään tilin tunnuksen, nimen, tyypin ja lähdeprovenienssin sekä jatkossa F25:n tarvitsemat luokitus- ja roolimetatiedot. Ihmisen luettava tiedosto on git-diffattava; `chart_version` johdetaan kanonisesta sisällöstä eikä tiedostomuotoilusta. Aktivoitu snapshot on sidottu tilikauteen tai muuhun eksplisiittiseen voimassaolorajaan. Vanhaa snapshotia ei muokata, sillä se on jo sinetöityjen tositteiden provenance-osoite.

Tämä ei tee SIE4:n numeroinnista aggountantin universaalia standardia. Se tekee yrityksen omasta kartasta yrityksen omaa masterdataa. Standardikartta ja tuotu kartta ovat rinnakkaisia, nimettyjä karttalähteitä samalla masterdata- ja `chart_version`-sopimuksella.

### 3. ALV-roolit ovat pakollinen karttakohtainen rikastus ennen aktivointia

Pelkkä `#KONTO` kertoo tilin numeron ja nimen, ei kaikissa tapauksissa luotettavasti sitä, mikä ALV-rooli tilillä on. Siksi SIE4-tuonti ei saa aktivoida yrityskarttaa ennen kuin sen käyttämät ALV-tilit ja niiden roolit on validoitu ja kirjattu karttasnapshotiin. Roolit ovat metatietoa, eivät globaalisti kovakoodattuja numeroita.

Tämä avaa eksplisiittisesti issueen `vat-account-role-chart-metadata` johtavan työn: tuodulle yrityskartalle kirjataan vähintään käytetyt vähennettävän ALV:n, käännetyn verovelvollisuuden osto- ja myynti-ALV:n sekä myynnin ALV:n roolit. Esimerkiksi käännetyn verovelvollisuuden tilit `1798` ja `2992` ovat siten karttakohtaisia roolikohteita, eivät oletuskartan oletuksia. Tiliin voidaan lisäksi liittää odotettu ALV-käsittely, jotta ADR-7:n record-time guard voi kieltäytyä väärästä verokoodista. Epäselvä tai ristiriitainen rooli estää aktivoinnin tai vaatii tarkistettavan täydentämisen, ei nollailmoituksen riskiä.

### 4. Molemmat käyttäjäpolut ja katkos

- **Uusi yritys:** `aggountant init` asentaa aggountantin standardikartan. Tämä on oletuspolku.
- **Olemassa oleva yritys:** onboarding tuo oman kartan SIE4:stä, rikastaa ja tarkistaa sen roolit sekä aktivoi snapshotin. Yritys jatkaa omilla tilinumeroillaan.
- **Siirtyminen vanhasta kartasta standardikarttaan:** tämä on tietoinen katkos, ei `init`in uudelleenajo eikä karttatiedoston ylikirjoitus. Vanha kartta säilyy historiallisena snapshotina; cut-over tarvitsee eksplisiittisen tilikarttamappauksen, alkusaldojen käsittelyn ja live-kauden aloituspisteen. Tämän mappauksen toteutus- ja auktoriteettisäännöt kuuluvat onboarding/cut-over-työhön, eivät tähän ADR:ään.

### 5. Välittömät seuraukset estetyille issueille

- **`sv-finding-default` avautuu toteutukseen:** lähdejärjestelmästä haetaan SIE4 ja sen `#KONTO` tuodaan yrityksen masterdataksi. Näin koko kartta, tase-tilit sekä myöhemmän vertailun `#VER`/`#TRANS`-aineisto saadaan uskollisesti käyttöön. Issue ei tarkoita standardikartan muuttamista yrityksen kartaksi.
- **`vat-account-role-chart-metadata` avautuu toteutukseen:** F25:n karttaimportin tuottama snapshot on sille roolimetadatan omistava lähde. Työ korvaa globaalit tilinumerot kartta- ja versiokohtaisella resoluutiolla ja kieltäytyy aktivoimattomasta/roolittomasta kartasta ALV-projektiossa.

## Consequences

### Positive

- Yritys voi jatkaa omilla tutuilla tileillään ilman että vanhan järjestelmän kartta naamioidaan geneeriseksi oletukseksi.
- Koko kartta, myös tase-tilit, tulee koneellisesti tarkistettavasta lähteestä; sama aineisto palvelee historian vertailua.
- Vuosittainen kartan evoluutio säilyy snapshotteina ja vanhat tositteet ovat aina tulkittavissa niiden pinnatulla `chart_version`illa.
- ADR-7:n ALV-jalat resolvoituvat kartan rooleista, joten `1798`/`2992`-kaltaiset tilit eivät jää oletuskarttaan sidotuksi hiljaiseksi virheeksi.
- `init` pysyy turvallisena, idempotenttina bootstrapina: se ei ylikirjoita jo käytössä olevaa yrityskarttaa.

### Negative / costs (accepted)

- SIE4-parseri, karttasnapshotin validointi, roolirikastus ja activation review ovat uutta F25-työtä.
- Kartan vuosiversiot ja cut-over tarvitsevat eksplisiittisen voimassaolosidonnan; yhden "nykykartta"-tiedoston helppous hylätään historiallisen toistettavuuden vuoksi.
- SIE4:n `#KONTO` ei yksin ratkaise kaikkia semanttisia rooleja. Puuttuva rooli on käsiteltävä näkyvänä onboarding-tehtävänä, ei pääteltävä tilinumerosta.

### Rejected alternatives

**Chart-B: `init`in uudelleengenerointi, REJECTED.** Se tekisi yhden yrityksen kartasta geneerisen oletuksen, sotkisi uuden yrityksen bootstrapin olemassa olevan yrityksen migraatioon eikä mallintaisi kartan vuosittaista muutosta. Se myös tekisi jo alustetun datakodin uudelleenajosta vaarallisen ylikirjoituspolun.

**Chart-C: käsin rakennettu kartta ilman kanonista tuontia, REJECTED ensisijaisena polkuna.** Se voi olla täydennys tai fallback, mutta ei olemassa olevan yrityksen todellisen kartan adoptiomekanismi: se ei todista, että tase-tilit, nimet ja vuosiversio ovat lähdejärjestelmän mukaiset, ja se kutsuu agentin arvaamaan ALV-rooleja. SIE4:n puuttuessa tarvitaan eksplisiittinen, reviewattu korvaava lähde, ei hiljainen käsinrekonstruktio.

**Yksi globaalisti kovakoodattu ALV-tilitaulu, REJECTED.** Se on jo osoitettu vaaralliseksi `vat-account-role-chart-metadata`-issuessa: yrityskartassa tunnistamattomat tilit voivat johtaa nollaan tai väärään ALV-projektioon. Roolit kuuluvat karttasnapshotiin.

### Biggest risk

**Kartta aktivoidaan näennäisesti täydellisenä, vaikka ALV- tai tase-tilien semantiikka puuttuu.** Tämän seurauksena uusi kirjaus voisi käyttää väärää tiliä tai ALV-projektio olla hiljaisesti vajaa. Torjunta on fail-closed activation: lähdeprovenienssi, pakolliset rakenteelliset tarkistukset, karttakohtaiset ALV-roolit ja selkeä review ennen kuin karttaa käytetään sinetöintiin. SIE4:n hankkiminen pienentää riskiä, koska se täydentää nykyisen P&L-näkymän tase-tilit ja koneluettavat tositerivit.

---

## Review-process lenses

Viisi linssiä konvergoi Chart-A:han. Ne tiivistävät hyväksytyn päätöksen perustelun, eivät avaa jo lukittua kahden käyttäjäpolun tuotelinjaa uudelleen.

- **Arkkitehti.** SIE4 `#KONTO` → git-natiivi masterdata jatkaa ADR-1:n omistajuusrajaa: kartta on yrityskohtaista referenssidataa, ei `Policy` eikä fiscalin johdettu lukupinta. Snapshot + `chart_version` sopii O-19/O-26:een; `init` säilyy erillisenä bootstrapina.
- **Kirjanpito / auditoitavuus.** Yrityksen aiemmin käyttämät tilit, myös tase- ja ALV-tilit, on säilytettävä sellaisina kuin ne ovat, jos kirjaa jatketaan ja historiaa verrataan. SIE4 on tähän tarkoitukseen oikea koneellinen lähde, kun taas geneerisen kartan uudelleennimeäminen hävittää jatkuvuuden.
- **Dataeheys.** Kartan merkitys on ajallinen: yksi mutable tiedosto ei saa muuttaa vanhaan tositteeseen pinnatun tilin tulkintaa. Content-hashatut snapshotit ja activation-gate tekevät vanhan version osoitettavaksi. Rooliton kartta on fail-closed.
- **Ylläpidettävyys.** Deterministinen parseri + normalisointi + golden-fixture on pienempi ja testattavampi pinta kuin yrityskohtainen `init`-haara tai käsin rakennettu erikoiskartta. Roolirikastus on erillinen, näkyvä vaihe eikä tilinumero-heuristiikka joka vuotaa jokaiseen ALV-kuluttajaan.
- **Onboarding / ergonomia.** Käyttäjä valitsee vain ymmärrettävän polun: uusi yritys käyttää standardia, olemassa oleva tuo nykyisen karttansa. Historiallisen datan auktoriteettikysymystä ei piiloteta karttatuontiin: Onb-A:n rajaus ja alkusaldo-silta ratkaistaan siihen tarkoitetussa ADR:ssä.

## Related

- **ADR-1:** kartta on git-natiivia, review-portattua ei-PII-masterdataa.
- **ADR-3:** vanhan järjestelmän historia on Onb-A:ssa johdetun analytiikan/referenssin aineistoa, ei automaattisesti aggountantin fiscal-auktoriteettia.
- **ADR-7:** ALV-jalat ovat sinetöityjä postingeja; niiden tilit ratkaistaan karttakohtaisista rooleista.
- **`sv-finding-default`:** suljettava toteutuksella, jossa lähdejärjestelmän SIE4 `#KONTO` tuo yrityksen todellisen kartan.
- **`vat-account-role-chart-metadata`:** avautuu toteutukseen: roolit ja odotetut ALV-käsittelyt ovat F25:n tuoman karttasnapshotin metadataa.
- **`onboarding-authority-adr`:** ratkaisee erikseen Onb-A/Onb-B:n, historian auktoriteetin ja alkusaldojen sillan.
- `candidate.md §2/§3/§5`, `scenarios.md G1`, `glossary.md` tilikartta, `coverage.md` §7, `benchmark.md §3.1`, `decisions.md` D-12.

*Dated 2026-08-16: initial decision. This ADR decides the adoption mechanism for an existing company chart only: deterministic SIE4 `#KONTO` import into versioned, git-native masterdata, with per-chart VAT-role enrichment and review before activation. It does not add code, change the standard-chart default, retroactively seal history, or decide the onboarding-authority boundary; those are follow-up implementation or ADR work.*
