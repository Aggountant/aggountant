# ADR-12 kirjanpitäjälle: avoimet erät johdetaan tositteista, reskontraa ei pidetä

Tämä on yhteenveto ADR-12:n ehdotuksesta kirjanpidon ammattilaiselle. Varsinainen päätös on [ADR-12](adr-12-open-items); tämä sivu ei päätä mitään uutta. ADR on ehdotus, kunnes tuotteen omistaja hyväksyy sen.

## Mistä on kyse

Kuukausisulkujen välillä aggountant ei tiedä, mikä on vielä auki. Tuotantokäytössä tuli vastaan tapaus, jossa myyntilaskut ja kululaskujen maksut tehdään pankin omilla työkaluilla, joten kirjanpito saa niistä tiedon vasta sulussa tiliotteelta. Kesken kuun hyväksytty ja pankissa ajastettu kululasku näytti sulussa maksamattomalta ja oli vähällä tulla maksetuksi toiseen kertaan.

Tiliote ei auta, koska camt.053- ja camt.054-sanomissa on vain kirjattuja tapahtumia (Finanssialan tiliote-ohje). KILA:n yleisohjeen mukaan maksuliikenneohjelman maksuehdotukset ja maksuluettelot eivät ole tositteita, ellei niissä ole lakisääteisiä tietoja, jotka eivät ilmene muualta (5.7), joten ajastettua maksua ei voi kirjata, mutta sen voi tallentaa päivättynä tietona.

## Mitä ehdotetaan

- **Avoin erä johdetaan.** Erä on sinetöity velka- tai saamisvienti, jota myöhempi vienti ei ole vielä kokonaan selvittänyt: hyväksytty kululasku nyt, myöhemmin myyntilasku, ostolasku tai ilmoitettu vero. Avoin määrä lasketaan joka kerta erän synnyttäneistä vienneistä ja selvityslinkeistä. Saldoa, maksettu-merkintää tai tilaa ei tallenneta. Jokainen lasku on edelleen oma tositteensa pääkirjassa, joten KILA:n määritelmän mukaan kyse on pääkirjanpidon osa-alueesta eikä osakirjanpidosta (1.4.1).
- **Periaate "Vouchers, not ledgers of receivables" säilyy,** mutta sen sanamuotoa tarkennetaan: oma reskontra, laskutus, muistutukset, perintä ja maksujen käynnistäminen ovat edelleen tuotteen ulkopuolella, johdettu avoimien erien näkymä ei.
- **"Maksu ajastettu" on päivätty tieto, ei tila eikä tosite.** Siinä on päivä, jolle maksu on ajastettu, lähde (käyttäjä tai nimetty järjestelmä) ja kirjaushetki. Se ei koskaan sulje erää; vain sinetöity maksu sulkee. Ajastettu kululasku ei ole maksettavien listalla, ja sulku näyttää varoituksen "maksu ajastettu päivälle X" eikä "maksamaton". Ajastuksen voi kirjata, siirtää tai perua myös suljetun kuukauden laskulle, koska se ei ole kirjaus.
- **Vanhentunut ajastus mitataan tiliotteesta.** Kun tuotu tiliote kattaa ajastetun päivän ja seitsemän päivää sen jälkeen eikä maksua näy, tila on "ajastettu maksu ei näy pankissa: tarkista". Lasku ei palaa itsestään maksettavaksi, koska maksu on voinut lähteä muualta. Käyttäjä ratkaisee: puuttuva tiliote tuodaan, ajastus perutaan, maksu ajastetaan uudelleen tai muulla tavalla tehty maksu kirjataan näyttöineen.
- **Toinen maksu ei selvitä jo maksettua erää.** Tiliotteen tuonti tarkistaa avoimen määrän samassa kirjoitustapahtumassa kuin linkin ja kieltäytyy, jos erä on jo maksettu. Virhe kertoo aiemman maksun tositteen, päivän, summan ja pankkiviitteen. Jos raha todella lähti kahdesti, tapahtuma kirjataan saamiseksi maksun saajalta.
- **Väärä linkki korjataan uudella merkinnällä,** joka nimeää korvaamansa merkinnän ja syyn. Kirjanpidon luvut eivät muutu. Väärälle tilille kirjattu maksu korjataan edelleen korjaustositteella.
- **Yritys, jolla ei ole avoimia eriä, ei huomaa mitään.** Tuonti ja sulut toimivat ja tulostavat kuten ennenkin.

## Mitä tuotteen omistajalta kysytään

ADR:n avoimissa kysymyksissä on kahdeksan valintaa suosituksineen. Kirjanpitäjää koskevat erityisesti nämä:

- Tehdäänkö ensin vain kululaskut ja myynti- ja ostolaskut myöhemmin (suositus), ja onko pankin laskutustyökalu avoimien myyntilaskujen ensisijainen näkymä niin, että aggountant täsmäyttää sulussa (suositus)?
- Riittääkö ajastukselle lähde ilman pakollista näyttöä (suositus)?
- Hylätäänkö kululaskun osamaksu ensimmäisessä versiossa kirjausohjeen kanssa: maksettu osa saamiseksi hakijalta ja kuittaus, kun loput maksetaan, jolloin Tulorekisteriin tulee yksi maksupäivä (suositus)? Saaminen nimeää laskun, ja niin kauan kuin se on auki, laskun tila ja maksettavien lista näyttävät avoimen määrän saamisella vähennettynä, jotta jo maksettua osaa ei tarjota uudelleen.
- Näytetäänkö kahdesti maksettu kululasku tilana "liikaa maksettu", kunnes saaminen on palautettu tai kuitattu (suositus)?
- Pysähtyykö tuonti, kun tavallinen kirjaus vähentää kululaskuvelkaa ja vastapuolella on avoin kululasku samalla summalla (suositus: kyllä, vain yrityksissä, joilla on avoimia kululaskuja)?

**Lopputulos:** aggountant kertoo, mitä kirjanpito tietää olevan auki, ja estää maksamasta samaa kululaskua kahdesti. Laskutus, perintä ja maksaminen jäävät pankille ja laskutusjärjestelmälle.
