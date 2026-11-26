# ADR-11 arkkitehdille: ohjelman omistama luonnos, sinetöity asiakirja ja faktoista johdettu tila

Tämä on yhteenveto ADR-11:n päätöksestä ohjelmistoarkkitehdille. Varsinainen päätös on [ADR-11](adr-11-expense-claim); tämä sivu ei päätä mitään uutta.

## Päätös

Kululasku on ohjelman omistama luonnos, ADR-5:n intake-laji git-hallinnan ulkopuolella, johon kirjoitetaan vain CLI:n kautta. Jokainen kirjoitus validoi tyypitetyt kentät ja palauttaa avoimet kysymykset. Markdown on renderöity näkymä, jota ei tallenneta. Domainin Suomi-moduuli laskee kaavamaiset korvaukset ja kieltäytyy koneluettavalla koodilla, kun syöte vaatii tulkintaa. Yksi esikatselun sisältöön sidottu hyväksyntä kirjoittaa samassa transaktiossa jäädytetyn asiakirjan liitevarastoon ja sinetöi tositteet, joiden liitteenä on asiakirjan tiiviste. Sen jälkeen kululaskuun ei kirjoiteta. Maksu, Tulorekisteri-ilmoitus ja korjaukset ovat omia faktojaan, ja tila johdetaan niistä joka kerta.

Kenttäluettelot, komentojen nimet, polut, taulut, virhekoodit ja asiakirjan sisältö ovat toteutusissueiden omistamia, eivät ADR:n.

## Missä mikin asuu

| Tieto | Paikka |
|---|---|
| Luonnos | Datakodin intake, git-hallinnan ulkopuolella, vain ohjelman kirjoittama |
| Kuitit ja todisteet | Liitevarasto keruuhetkellä, luonnoksessa tiiviste |
| Hyväksytty asiakirja | Liitevarasto, hash-ketjun ulkopuolella, jotta henkilötiedot voi poistaa säilytysajan jälkeen ketjua rikkomatta; säilytysaika päätetään, kun poisto rakennetaan |
| Tositteet | Kirjanpitokanta: yksi per kuittirivi kuitin päivälle, yksi per kuukausi, jona matka päättyy, kaavamaisille korvauksille |
| Linkit (versio, tositteet, maksu) | Kirjanpitokanta, uudet taulut yksisuuntaisella migraatiolla |
| Työpaikkarekisteri, suhde yhtiöön, osakkuus | Henkilötietokanta, vain lisättävä, kaksiaikainen (milloin tieto pätee ja milloin ohjelma sai tietää), uudet taulut yksisuuntaisella migraatiolla |
| Korvausmäärät | Ohjelman mukana tuleva taulukko, vuosipainos päätöksineen, valitaan matkan päivästä |

Molemmat migraatiot tehdään vasta niitä tarvitsevassa viipaleessa, varmuuskopio ensin ja migroitua kopiota alkuperäiseen verraten.

## Invariantit

- Laskennan syötekentät ovat tyypitettyjä ja ohjelman validoimia. Vapaa teksti on vain selitystä.
- Kaavamaisia rivejä ei syötetä: ne johdetaan matkasta. Pienempi maksettava määrä sallitaan, suurempi kieltäytyy.
- Ilman voimassa olevaa työpaikkamerkintää matkan päivälle ei synny verovapaita matkakorvauksia. Kolmen vuoden raja ja kirjaamaton pääasiallisuusarvio estävät hyväksynnän, eivät luonnosta.
- Puuttuva taulukkopainos kieltäytyy, eikä mitään ekstrapoloida.
- Hyväksyntämerkintä kertoo lajinsa: oma hyväksyntä tai kirjattu ulkoinen hyväksyntä (hyväksyjä, aika, järjestelmä, viite; kirjaaja erikseen). Ulkoinen kirjataan vain, kun ohjelman laskelma täsmää ulkoisesti hyväksyttyyn sisältöön. Odottavaa "ensimmäinen hyväksyntä tehty" -tilaa ei tallenneta.
- Yhtiö kirjaa kululaskujen hyväksyjän. "Odottaa hyväksyntää (hyväksyjä X)" on johdettu tila kaikissa yhtiöissä, ja sen päättävät hyväksyntä, hylkäys ja hakijan peruutus. Yhden hengen yhtiössä vaihe kuitataan heti näkyvänä omana hyväksyntänä. Hyväksyjä kysytään yhtiön ensimmäisen laskun lähetyksessä, ei `init`issä; yhtiö ilman kululaskuja ei kirjaa mitään, ja sen sulut, migraatioiden jälkeinen data ja komentojen tuloste pysyvät ennallaan.
- Laskennan valinnat: vuodenvaihteen matkavuorokausi pienemmällä määrällä, ilmainen ateria yhtenä ateriana hinnasta riippumatta, yrityksen maksama oma ateria vähennyksenä (vähintään puolet päivärahasta), pyöristys kerran kululaskulle puolikkaat nollasta poispäin, ja vapaa-ajan yhdistäminen ilmoitetuista ajoista laskettuna eikä kieltäytyen.
- Kuittauksen päivä on Tulorekisterin maksupäivä.
- Jokaisesta keskeneräisestä johdetusta tilasta on päättävä komento; tilat ja komennot omistaa `expense-claim-status`.
- Suljettuun kuukauteen osuvan matkan korvaukset kirjataan saman tilikauden ensimmäiselle avoimelle kaudelle; tilikauden raja ylitetään tilinpäätöksen siirtovelalla.
- Suomi on Domainin moduuli, ei parametri. Kululasku, kuittirivit, hyväksyntä ja tila eivät sisällä mitään Suomi-spesifistä, eikä plugin-kerrosta rakenneta nyt.

## Miksi ei ajoa

ADR ottaa tuotteen säännöksi, että ajo (Run) on vain ohjelman omalle peruuttamattomalle ulkoiselle teolle, kuten lähetykselle viranomaiselle tai maksulle. Laskun myöhemmät vaiheet päättää joku muu: käyttäjä maksaa, Tulorekisteri vastaa ja asiakas maksaa. Niitä odottava ajo jäisi auki viikoiksi ja näyttäisi keskeneräistä työtä asian jo ratkettua muualla, kuten ALV-ajoille kävi. `file-wage` pysyy ajona, koska se on ohjelman oma peruuttamaton lähetys. Ensimmäinen kululaskuviipale kirjaa säännön `crates/AGENTS.md`:hen.

## Rajaukset

- Matkaennakot kirjataan ensimmäisessä versiossa käsin (ADR:n kohta 11). Ohjelmallinen tuki vaatii tilikarttaversion ja osasuoritukset maksujen kohdistukseen, ja se tulee ensimmäisen version jälkeen. Sama tilikarttaversio antaa kululaskuveloille oman tilin; siihen asti velka on tilillä `2740`.

- aggountant ei maksa mitään eikä pidä reskontraa (`crates/AGENTS.md`: "Vouchers, not ledgers of receivables"). Laskun avoin määrä johdetaan tositteiden linkeistä.
- Vanhojen sinetöityjen tositteiden liittäminen kululaskutunnisteeseen kuuluu tuotteeseen. Yhden kodin oman tiedostomuodon siirto ei ole tuotekomento, ja muu historia siirretään vaiheittaisella `migrate`-skillillä.

**Lopputulos:** uusia rakenteita on neljä: luonnos, asiakirja, linkkitaulut ja rekisteri. Kaikki muu käyttää olemassa olevia polkuja: kuitin ALV-käsittelyä, tiliotteen tuontia, `file-wage`-ajoa ja liitevarastoa.
