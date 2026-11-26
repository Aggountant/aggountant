# ADR-11 kirjanpitäjälle: ohjelma laskee kululaskun, hyväksyntä sinetöi sen

Tämä on yhteenveto ADR-11:n päätöksestä kirjanpidon ammattilaiselle. Varsinainen päätös on [ADR-11](adr-11-expense-claim); tämä sivu ei päätä mitään uutta.

## Mitä päätettiin

Kululasku on yhden hakijan korvaushakemus, jossa on matkoja ja rivejä. Sitä ei ole sidottu kuukauteen: lasku voi koota kuukauden tai yhden matkan. Hakija voi olla työntekijä tai esimerkiksi hallituksen jäsen tai palkaton osakas. Konsultin kulut kuuluvat hänen omaan laskuunsa, joten konsultille ei tehdä kululaskua. Toiminimi on rajattu pois.

Agentti kirjaa matkat ja kuitit ohjelman komennoilla. Ohjelma omistaa luonnoksen, ja luettava kululasku on näkymä, jota ei tallenneta eikä muokata.

- **Kaavamaiset korvaukset** (kilometrikorvaus, päiväraha, ateriakorvaus, yömatkaraha) ohjelma laskee matkasta Verohallinnon vuosittaisen päätöksen taulukosta, myös ulkomaan päivärahat maittain. Niitä ei kirjoiteta käsin. Yritys voi maksaa vähemmän, mutta ei enempää: enimmäismäärän ylitys olisi palkkaa, ja ohjelma estää sen, kunnes tuotteessa on palkanlaskenta.
- **Tulkinta jää ihmiselle.** Varsinainen työpaikka, ilmainen ateria, työajan osuus, kuitin laji, edustus, edelleenlaskutus ja se, tapahtuiko ajo, kysytään ja kirjataan lähteineen. Agentti täyttää ilmaiset ja yrityksen maksamat ateriat, mutta tyhjä vastaus ei tarkoita "ei aterioita". Hakija vakuuttaa tiedot oikeiksi hyväksyessään.
- **Ohjelma kieltäytyy**, kun lähteet eivät anna laskea: enimmäismäärän ylitys, puuttuva lakisääteinen tieto, päällekkäiset matkat, jo kirjattu kuitti tai tapaus, jota Verohallinnon päätös ja ohje eivät ratkaise. Kieltäytyminen estää hyväksynnän, ei luonnoksen jatkamista.
- **Kuittirivit** saavat saman ALV-käsittelyn kuin sama kuitti saisi kuitin kirjauksessa.

Kun lähteet jättävät laskennalle valinnan, se on ratkaistu näin:

- Vuodenvaihteen yli menevä matkavuorokausi saa kahden vuoden määristä pienemmän.
- Ilmainen ateria on yksi ateria hinnasta riippumatta, ja päiväraha puolitetaan aterioiden lukumäärän mukaan. Matkustajan itse ostama ja yrityksen maksama ateria (esimerkiksi yrityksen kortilla) ei ole ilmainen ateria: verovapaa on päivärahan ja aterian hinnan erotus, mutta vähintään puolet päivärahasta (Verohallinnon ohje VH/630/00.01.00/2026, 8.3.3 ja esimerkki 60). Kallista ateriaa ei siis estetä.
- Pyöristys tehdään kerran kululaskulle, puolikkaat nollasta poispäin.
- Matkaan yhdistetty vapaa-aika lasketaan käyttäjän ilmoittamasta työajasta ja matka-ajoista (sama ohje, 8.3.2): päivärahojen määrä tulee tästä kestosta, ja ilmaiset ateriat ja ulkomailla maa kysytään jokaiselta lasketulta päivärahavuorokaudelta, ei kellon mukaan. Ensimmäisessä versiossa tätä ei estetä.

## Varsinainen työpaikka ja kolmen vuoden sääntö

Ohjelma ei laske matkakorvauksia verovapaiksi, ennen kuin hakijalle on kirjattu varsinainen työpaikka matkan päivälle. Tieto on päivätty merkintä henkilötietokannassa, ja jokainen matka ottaa siitä tilannekuvan, joka sinetöidään matkan mukana. Jos merkintää myöhemmin korjataan taaksepäin, ohjelma luettelee hyväksytyt laskut, joihin korjaus vaikuttaa, mutta ei muuta niitä itse.

Kolmen vuoden sääntöä seurataan tiukasti: viimeisen verovapaan päivän jälkeen ohjelma kieltäytyy, ja jos asiakkaan kohteen pääasiallisuus on arvioitava eikä arviota ole kirjattu, hyväksyntä estyy. Pääasiallisuuden arvioi ihminen.

## Hyväksyntä ja kirjaukset

Hyväksyntä sidotaan täsmälleen esikatseltuun sisältöön. Hyväksyntämerkintä kertoo, onko se oma hyväksyntä (hyväksyjän nimi, rooli ja aika) vai kirjattu ulkoisen matkalaskuohjelman hyväksyntä. Oman laskun hyväksyntä on sallittu ja näkyy sellaisena. Toinen hyväksyjä on toinen merkintä samassa hyväksynnässä.

Jokainen yhtiö kirjaa, kuka sen kululaskut hyväksyy, ja vaihe "odottaa hyväksyntää (hyväksyjä X)" on ensimmäisessä versiossa kaikissa yhtiöissä. Useamman hengen yhtiössä lasku odottaa hyväksyjää, kunnes tämä hyväksyy tai hylkää sen tai hakija vetää sen pois. Yhden hengen yhtiössä vaihe kuitataan heti, ja hakijan hyväksyntä kirjataan näkyvästi omaksi hyväksynnäksi.

Hyväksyntä sinetöi samalla kertaa jokaisesta kuitista oman tositteen kuitin päivälle ja jokaiselta kuukaudelta, jona laskun matka päättyy, yhden tositteen kaavamaisista korvauksista (suoriteperuste). Jokaisen tositteen liitteenä on hyväksytty kululaskuasiakirja: matkat, työpaikan tilannekuvat, laskelma sovellettuine taulukoineen ja hyväksyntä. Asiakirjan säilytysaika päätetään, kun poisto rakennetaan, todennäköisesti noin kymmenen vuotta tiedoille, joihin Tulorekisterin korjaus voi nojata; siihen asti mitään ei poisteta.

Kululaskuveloille tulee tilikartan seuraavassa versiossa oma tili (ehdotus `2741` Kululaskuvelat). Siihen asti velka kirjataan tilille `2740` Muut lyhytaikaiset velat, ja kunkin laskun avoin määrä johdetaan tositteiden linkeistä.

Jos matkan kuukausi on jo suljettu, korvaukset kirjataan saman tilikauden ensimmäiselle avoimelle kaudelle, ja asiakirjassa näkyvät matkan omat päivät. Tilikauden yli mennään tilinpäätöksen siirtovelalla.

Hyväksytyn laskun sisältöä ei muuteta. Virhe korjataan uudella versiolla ja korjaustositteella, ja jo ilmoitettu Tulorekisteri-tieto korjataan Verohallinnon ohjeen mukaan. Jos korvausta on maksettu liikaa, ylitys on veronalaista palkkaa, jota ohjelma ei vielä osaa ilmoittaa: ohjelma kirjaa korjauksen ja saatavan hakijalta ja näyttää Tulorekisterin korjauksen tehtäväksi muualla, kunnes sen näyttö on kirjattu.

## Maksu ja Tulorekisteri

Maksu tunnistetaan tiliotteen tuonnissa ja liitetään laskuihin, jotka se maksaa. Muuten maksettu lasku kirjataan näyttöineen. Kun lasku maksetaan kuittaamalla hakijalta olevaa saatavaa vastaan, kuittauksen päivä on Tulorekisterin maksupäivä (Verohallinnon ohje VH/5655/05.00.00/2026, luku 2.1). Jos kuittaus ja erotuksen pankkimaksu osuisivat eri kuukausille, lähde ei ratkaise asiaa, joten kuittaus päivätään samalle päivälle kuin erotuksen maksu. aggountant ei maksa mitään eikä pidä ostoreskontraa: laskun avoin määrä johdetaan tositteiden linkeistä. Tulorekisteri-ilmoitus tehdään hyväksytyistä asiakirjoista ja maksupäivästä, ei kirjoiteta uudelleen. Yksi päivärahamäärä voi kantaa useamman päivärahatyypin, ja kuukauden maksut saman hakijan osalta voi koota yhteen ilmoitukseen. Määräpäivä on lakisääteinen.

Ohjelma näyttää jokaisesta keskeneräisestä laskusta tilan, määräpäivän ja seuraavan komennon, ja jokaisesta tilasta pääsee pois komennolla. Hyväksymätön luonnos, jonka sisältö osuu kauteen, estää kauden sulun. Maksamaton tai ilmoittamaton hyväksytty lasku näkyy sulussa varoituksena.

## Ulkoinen matkalaskuohjelma ja edelleenlaskutus

Ulkoisen ohjelman vaiheet (lähetetty, hyväksytty, kuka ja milloin) tuodaan faktoina. Siellä tehty hyväksyntä voidaan kirjata aggountantin hyväksynnäksi vain, jos ohjelman oma laskelma täsmää ulkoisesti hyväksyttyyn sisältöön; muuten ero korjataan lähteessä tai lasku hyväksytään aggountantissa itse.

Matkalle ja kuittiriville voi merkitä edelleenlaskutusviitteen. Ohjelma laskee laskutettavan kustannuspohjan; laskutus ja sen hinta kuuluvat laskutukseen, eivät kululaskuun.

## Matkaennakot

Ensimmäisessä versiossa ohjelma ei käsittele matkaennakoita, vaan ne kirjataan käsin nykyisillä komennoilla. Maksettu ennakko on saaminen henkilöltä tilillä `1730` Muut lyhytaikaiset saamiset (seuraavassa tilikarttaversiossa oma tili, ehdotus `1740` Matkaennakot). Kululasku kirjataan aina bruttona velaksi, ja ennakon kuittaus on oma tositteensa (debet velka, kredit saaminen). Erotus kirjataan tiliotteelta kumpaan suuntaan tahansa. Ennakkoa ei ilmoiteta Tulorekisteriin, mutta kululaskun kilometrikorvaukset (311) ja päivärahat (331) ilmoitetaan kokonaan. Agentti varoittaa ennakosta, joka on avoinna kuukauden kuluttua matkan päättymisestä, ja osakkaan ennakosta aina ennen 31.12. Ohjelmallinen tuki tulee ensimmäisen version jälkeen.

## Vanhat kululaskut

Jo sinetöityjä tositteita ei sinetöidä uudelleen; ne voi liittää kululaskutunnisteeseen sellaisinaan. Siirrossa toisesta ohjelmasta vanhat kululaskut menevät arkistoon tositteiden mukana, ja eteenpäin siirtyvät avoimet laskut, ilmoittamattomat korvaukset, varsinainen työpaikka ja aiempi työskentely asiakkaan kohteissa.

**Lopputulos:** kululasku on matkalasku liitteineen, jonka ohjelma laskee ja joka lukitaan hyväksynnässä. Maksu ja ilmoitus ovat omia tapahtumiaan, ja ohjelma kertoo, mitä niistä puuttuu.
