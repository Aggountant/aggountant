# ADR-4 kirjanpitäjälle — kustannuspaikka ja kohdennus tositteella

Tämä on **yhteenveto** ADR-4:n päätöksestä kirjanpidon ammattilaiselle: mitä
päätettiin ja miksi, kirjanpitäjän termein. Varsinainen päätös perusteluineen on
`adr/ADR-4-cost-centre.md`; tämä sivu ei päätä mitään uutta.

## Mistä tässä on kyse

Moni pieni osakeyhtiö haluaa nähdä **tuloksen projekteittain tai
kustannuspaikoittain** — esimerkiksi paljonko yksittäinen projekti tuotti tai
mille osastolle kulu kuuluu. Kitsaassa tämä hoidetaan **kohdennuksella**: jokainen
vienti voidaan kohdistaa kustannuspaikalle tai projektille, ja raportit saadaan
kohdennuksittain.

aggountant:n tositemallissa tätä ei vielä ollut. Vertailu Kitsaaseen nosti sen
esiin **ainoana aidosti puuttuvana peruspalikkana**. Ongelma on ajoitus: tosite
**sinetöidään muuttumattomaan ketjuun** (ADR-1/ADR-2), eikä sinetöityyn
tositteeseen voi enää jälkikäteen lisätä kenttää. Jos kohdennuskenttä jätettäisiin
pois nyt ja lisättäisiin vasta myöhemmin, kirjanpitoon syntyisi **pysyvä katkos**:
osa vienneistä ei rakenteellisesti voisi koskaan kantaa kohdennusta. Siksi asia oli
ratkaistava **ennen kuin tositteen muoto lukittuu**.

## Mitä päätettiin

**Jokainen vienti (posting) saa alusta asti valinnaisen kohdennuskentän.** Kenttä on
**kevyt avain–arvo-lista**: avain kertoo *minkä ulottuvuuden* kyse on
(`kustannuspaikka`, `projekti`, myöhemmin vaikka `osasto`) ja arvo on kohteen
**koodi**. Esimerkiksi yksi vienti voi kantaa `{kustannuspaikka: MYYNTI}` tai
tarvittaessa useamman ulottuvuuden samaan aikaan, `{kustannuspaikka: MYYNTI,
projekti: P-2026-01}`.

Muutama kirjanpidon kannalta olennainen valinta:

- **Kohdennus on vientikohtainen, ei tositekohtainen.** Sama tosite (esimerkiksi
  yksi ostolasku) voi jakautua kahdelle projektille, joten kohdennus on
  vientirivillä — samoin kuin Kitsaassa. Käyttöliittymä voi tarjota "kohdista koko
  tosite" -oikotien, mutta se puretaan vientikohtaisiksi merkinnöiksi ennen
  sinetöintiä.
- **Sinetöity tosite kantaa vain koodin, ei nimeä.** Kustannuspaikan tai projektin
  **nimi** (joka voi olla vaikka henkilön nimi) ei mene muuttumattomaan ketjuun,
  koska sieltä sitä ei saisi enää poistettua. Ketjuun tallentuu vain **koodi**;
  selväkielinen nimi, hierarkia ja muut tiedot ovat erillisessä, muokattavassa
  **kohdennusrekisterissä** (samaan tapaan kuin tilikartta on omana,
  versioituna aineistonaan).
- **Kohdennusten "puurakenne" (esim. kustannuspaikka alakustannuspaikkoineen) on
  rekisterissä, ei tositteella.** Kitsaan kohdennus on itseensä viittaava puu;
  aggountant pitää saman hierarkian ja raporttien summauksen **rekisterin ja
  raportoinnin** puolella, jotta organisaatiorakenteen muuttaminen ei koskaan
  vaadi kirjanpitohistorian uudelleenkirjoittamista.

## Mitä tämä tarkoittaa käytännössä

- **Vanhat ja kohdentamattomat viennit näyttävät täsmälleen samalta kuin ennen.**
  Kenttä on valinnainen ja tyhjänä täysin näkymätön — se ei muuta jo tehtyjen
  tositteiden muotoa eikä katkaise sinettiketjua. Kohdennus vain *voi* nyt olla
  mukana.
- **Kohdennuksittainen tulos (per-projekti P&L) syntyy analytiikkapuolelta.** Kun
  kenttä on tositteella, ADR-3:n analytiikkalukupinta osaa summata tuloksen
  kohdennuksittain suoraan — projektikohtainen raportti on tavallinen ryhmittely.

## Mitä tässä ei vielä ratkaistu

- Itse **kohdennusrekisterin** rakennetta (mitkä avaimet ja koodit ovat sallittuja,
  hierarkia, "tämä tili vaatii aina kohdennuksen" -säännöt) ei rakenneta tässä —
  se on myöhempää työtä (F7, avoin kohta O-30). Nyt varataan vain tositteen
  **muoto**; kenttä otetaan täysimääräisesti käyttöön rekisterin myötä.
- **Kohdennusrekisterin version kiinnittäminen** provenienssiin (jotta vanhat
  kohdennusraportit toistuvat samanlaisina, kuten tilikartan versio) ratkaistaan
  myöhemmin (O-31).

**Lopputulos:** tositteen muoto on nyt **valmis kantamaan kohdennusta alusta asti
ilman että mitään vanhaa rikkoutuu**, ja projekti-/kustannuspaikkakohtainen
raportointi on mahdollista rakentaa ilman kallista jälkiasennusta.
