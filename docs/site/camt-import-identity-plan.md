# CAMT-tuonnin tapahtumaidentiteetti ja päällekkäiset viennit

> **Suunnitelmaluonnos — ei toteutuspäätös.** Tämä sivu on Jarin arvioitavaksi ennen kuin CAMT-tuontiin tehdään koodimuutoksia. Se yhdistää kaksi havaintoa: päällekkäinen vuosivienti ei saa estää seuraavaa erää, ja korjattu pankkitapahtuma ei saa kadota hiljaa.

## Tavoite

Pankkitapahtuma voi esiintyä useassa pankin CAMT-viennissä, esimerkiksi kahden vuoden rajalla. Järjestelmän pitää tällöin tunnistaa se samaksi tapahtumaksi, säilyttää kaikki alkuperäiset XML-viennit todisteena ja kirjata tapahtuma vain kerran. Jos pankki on muuttanut tapahtuman faktoja tai käyttäjä on muuttanut sen kirjanpidollista kohdistusta, tuonnin pitää pysähtyä näkyvään ristiriitaan.

Kun vuosiviennit menevät päällekkäin, osa tapahtumista esiintyy kahdesti. Nykyinen koko-XML-liite muuttaa muuten samasta tapahtumasta muodostuvan tositteen request-hashia. Siksi seuraava vuosierä voi hylkääntyä, vaikka tapahtuma olisi muuttumaton.

## Ehdotettu malli

Erotetaan neljä asiaa, joita ei saa käyttää toistensa korvikkeina:

| Asia | Merkitys | Käyttö |
|---|---|---|
| Tapahtuma-avain | Pankin yksilöllinen viite + tili | Löytää sama pankkitapahtuma eri viennistä. |
| Lähdesormenjälki | Versionoitu tiiviste pankin tapahtumafaktoista ennen tiliöintiä | Havaitsee pankin korjaaman tapahtuman. |
| Projektiotiiviste | Tiiviste tiliöinnistä, summista, ALV-käsittelystä ja päivistä ilman XML-konttia | Havaitsee muuttuneen kirjanpidollisen ehdotuksen. |
| Lähdehavainto | Tapahtuma-avain + sen XML-artefaktin tiiviste | Säilyttää tiedon siitä, missä kaikissa alkuperäisissä vienneissä tapahtuma nähtiin. |

Ledgeriin lisätään yksi atominen pankkitapahtuman sinetöintioperaatio. Se käsittelee sekä tositteen että lähdehavainnon samassa transaktiossa:

1. Parseri tuottaa tapahtuma-avaimen ja lähdesormenjäljen.
2. Tuonti säilyttää alkuperäisen XML:n artefaktina, mutta **ei liitä koko XML:ää jokaisen tositteen kanonisiin tavuihin**.
3. Tiliöintiehdotuksesta lasketaan projektiotiiviste.
4. Ledger vertaa tapahtuma-avainta aiempaan tapahtumaan:
   - avainta ei ole → sinetöi uuden tositteen ja ensimmäisen lähdehavainnon;
   - sama lähdesormenjälki ja sama projektiotiiviste → kirjaa vain uuden lähdehavainnon ja palauttaa `duplicate_source_observation`;
   - eri lähdesormenjälki → pysähtyy `SOURCE_FACTS_CHANGED`-ristiriitaan;
   - sama lähdesormenjälki mutta eri projektiotiiviste → pysähtyy `POSTING_PROJECTION_CHANGED`-ristiriitaan.

Nykyinen `request_hash` säilyy sinetöintipyynnön eheyssuojana. Sitä ei löysennetä eikä sen perusteella hyväksytä erilaista tositetta.

## Erän atomisuus

Ehdotus on, että `import camt seal` tekee ensin koko erän parse-, kohdistus- ja ristiriitapreflightin. Jos yksikin tapahtuma on muuttunut tai kohdistus on ristiriidassa, erä ei kirjoita yhtään uutta tositetta tai lähdehavaintoa. Hyväksyttävä erä kirjoitetaan yhtenä Ledger-transaktiona.

Tämä poistaa nykyisen epäselvän tilanteen, jossa erä voi ehtiä sinetöidä alkupään tapahtumia ennen kuin myöhempi päällekkäisyys kaataa komennon.

## Pre-MVP-rajaukset

- Ei tueta vanhoja binäärejä, vanhoja storeskeemoja tai vanhojen tositteiden automaattista migraatiota. Testi- ja pilot-kodit voidaan nollata ennen tätä muutosta.
- Ei yritetä tehdä yleistä CAMT-korjaus- tai reversal-käsittelyä tässä työssä. Muuttunut pankkifakta pysähtyy näkyvään ristiriitaan, josta syntyy oma korjauspolku myöhemmin.
- Ei automaattista tili- tai ALV-kohdistusta. Agentti toimittaa kohdistuksen kuten nykyäänkin.
- Kokonaiset XML-artefaktit säilyvät, mutta niiden esiintyminen useassa viennissä ei muuta tapahtuman kirjanpidollista identiteettiä.

## Hyväksymiskriteerit

Ennen toteutuspäätöstä suunnitelmasta tarkistetaan erityisesti:

1. Päällekkäiset vuosiviennit tuottavat laskettavasti yhtä monta yksilöllistä tapahtumaa kuin pankissa on, ja jokaisesta toistosta yhden `duplicate_source_observation`-havainnon.
2. Sama tapahtuma kahdessa XML:ssä ei luo toista tositetta eikä kaada erää.
3. Muutettu määrä, päivämäärä tai pankin status pysäyttää erän ennen kirjoituksia.
4. Sama pankkitapahtuma eri kohdistuksella pysäyttää erän ennen kirjoituksia.
5. Alkuperäisten XML-artefaktien ja tapahtumahavaintojen yhteys on tarkastettavissa.
6. Onnistuneen erän laskurit erottelevat uudet tositteet, duplikaattihavainnot ja ristiriidat.

## Toteutusjärjestys hyväksynnän jälkeen

1. Lukitaan tämä rajaus ja julkiset tulostyypit.
2. Tehdään Ledgerin atominen tapahtuma- ja havaintorekisteri testeineen.
3. Kytketään CAMT-CLI tähän rajapintaan ja poistetaan koko-XML-liitteen vaikutus tositteen identiteettiin.
4. Ajetaan synteettiset rajatestit ja hyväksyntäkoe usean vuoden päällekkäisillä vienneillä.
5. Päivitetään toteutus-DAG vasta kun toteutuksen rajaus on hyväksytty.

## Lähteet

- Issue: `camt-event-identity` — atominen CAMT-tuonti ja tapahtumaidentiteetti päällekkäisten vientien yli.
- Toteutettu mutta vielä kytkemätön perusta: lähdesormenjälki (`import-seal-source-fingerprint`).
