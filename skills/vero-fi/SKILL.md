---
name: vero-fi
description: >-
  Anna yrityksen ALV-ilmoitus Verohallinnolle: esikatsele suljetun kauden luvut,
  lähetä ilmoitus Vero API:n kautta käyttäjän hyväksymänä tai kirjaa käsin
  annettu ilmoitus (OmaVero, Ilmoitin.fi), selvitä epävarma tai OmaVerossa
  näkymätön lähetys, ota suora ilmoittaminen käyttöön ja uusi varmenne.
  Tulorekisterin ilmoitukset ovat skillissä tulorekisteri.
cli_version: "0.3.0"
schema_version: 1
---

# ALV-ilmoitus Verohallinnolle

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi (`aggountant skill print aggountant-init --file SOUL.md`).

Aggountant lähettää yrityksen oman ALV-ilmoituksen Vero API:n kautta. Se ei asioi
toisen organisaation puolesta eikä maksa veroa. Annettua ilmoitusta ei voi perua:
lähetyksen jälkeen löytynyt virhe korjataan korvaavalla ilmoituksella (alla), joka on
oma hyväksyttävä lähetyksensä. Siksi avoimet kysymykset selvitetään ennen lähetystä ja
käyttäjä hyväksyy juuri ne luvut, jotka lähtevät.

## Tavallinen kuukausi

Ilmoitus tehdään suljetusta kaudesta. `aggountant file-vat preview --period 2026-08`
laskee ilmoituksen kentät kirjanpidosta käyttämättä verkkoa, myös jo ilmoitetulle
kaudelle. Avoimelle tai suljettavalle kaudelle se antaa luonnoksen
(`fileable: false`), jolla täsmäytys tehdään ennen sulkua (`capture-receipt`);
sen lukuja ei lähetetä.

Esikatselu myös täsmäyttää ALV-tilit ilmoituksen lukuihin. Jos se nimeää ALV-tilien
vientejä, joilla ei ole ALV-merkintää (`untagged_postings`), käy ne tositteet läpi
lähdettä vasten ja kerro tilanne käyttäjälle ennen kuin ehdotat lähetystä, koska
tapauksia on kaksi ja ne ratkeavat eri tavoin:

- Vero kuuluu kauden ilmoitukseen, ja ilmoituksen luvuista puuttuu veroa. Suljettua
  kautta ei voi korjata aggountantissa, joten käyttäjä antaa kauden ilmoituksen
  OmaVerossa korjatuin luvuin (alla). Laske korjatut kentät tositteista ja näytä
  käyttäjälle, miten ne eroavat esikatselusta. Samaa veroa ei kirjata ALV-merkinnällä
  myöhemmälle kaudelle, koska se ilmoitettaisiin silloin kahdesti.
- Viennit eivät kuulu ilmoitukseen, esimerkiksi Verohallinnon palautus tilillä 2939
  tai avaavan tositteen ALV-saldo. Silloin lähetyksen syötteeseen tulee kuittaus
  `vat_reconciliation_acknowledgement`, jonka perusteen antaa käyttäjä. Kuittaus
  toteaa, ettei veroa puutu; puuttuvan veron ohittamiseen se ei ole.

`aggountant file-vat start` ottaa esikatselun luvut, tarkistaa valmiuden, kysyy kauden
tilan Verohallinnolta ja jää odottamaan hyväksyntää. Mitään ei ole vielä lähetetty.
Syötteen `payee_iban` on OmaVeron näyttämä Verohallinnon tili. Jos käynnistys
epäonnistuu, mitään ei lähtenyt, mutta ajo on käytetty: kun syy on korjattu, aloitetaan
uusi ajo.

Näytä käyttäjälle se, minkä hän hyväksyy: yritys, kausi ja eräpäivä, numeroidut kentät
euroina, nettosumma ja maksettava määrä. Hyväksyntä koskee juuri tätä ehdotusta, ja
`aggountant file-vat approve-filing` lähettää sen. Hylkäys päättää ajon; jos luvut
muuttuvat, uusi kierros alkaa esikatselusta.

Onnistuneen lähetyksen vastauksessa on Verohallinnon kuittaus, ja `run show` näyttää
sen myöhemmin ilman verkkoa. Kerro se ja maksettava määrä käyttäjälle. Kuittaus todistaa, että Verohallinto vastaanotti
ilmoituksen, ei että se on käsitelty: OmaVerossa ilmoitus on näkynyt vasta noin puolen
vuorokauden kuluttua. `aggountant file-vat status --period 2026-08` kertoo käsittelyn
tilan lähettämättä mitään.

Veron maksaa käyttäjä itse OmaVeron viitteellä. Maksettavassa ilmoituksessa ajo avaa
lopuksi maksun hyväksynnän, jota suoralla reitillä ei voi viedä läpi, joten ajo jää
näyttämään odottavaa tilaa. Ilmoitus on silti annettu. Palautuksen kohdistusta ei
päätellä, vaan se kirjataan Verohallinnon myöhemmästä tapahtumasta.

Kodin `TODO.md`:ssä kauden määräaika poistetaan vasta, kun `file-vat status` kertoo
ilmoituksen käsitellyksi tai käsin annettu ilmoitus on kirjattu. Kuitattu mutta
käsittelemätön tai selvittämättä jäänyt ilmoitus jää kohdaksi, jossa on ajon tunniste,
eräpäivä ja se, milloin tila kysytään uudelleen.

## Kun lähetyksestä ei ole varmuutta

Aikakatkaisu, keskeytynyt ajo tai ilmoitus, joka ei näy OmaVerossa, ei kerro, onko
ilmoitus perillä. Verohallinto noutaa rajapinnan vastaanottamat ilmoitukset käsittelyyn
viiveellä, joten sen "ei löydy" -vastaus (`Missing`) tai tyhjä OmaVero ei todista,
ettei ilmoitusta ole. Toinen lähetys olisi kaksoisilmoitus tai korvaava ilmoitus, jota
kukaan ei tarkoittanut. Korvaavan ilmoituksen kohdalla Verohallinto näyttää aiempaa
ilmoitusta, kunnes korvaava on käsitelty, joten sekään ei kerro, että korvaava puuttuu.

Kysy tila komennolla `file-vat status` ja seuraa vastauksen `message`- ja
`next_command`-kenttiä: ohjelma tietää, mitä ajolle tapahtui, kysyy Verohallinnolta
lähettämättä mitään ja sallii uuden lähetyksen vasta, kun se on itse todennut kauden
ilmoittamattomaksi. Sen estettä ei kierretä muutetulla syötteellä eikä OmaVeron
lomakkeella. Kerro käyttäjälle, milloin tila kysytään uudelleen. Havaittu
käsittelyviive, eri tilanteet ja se, milloin Verohallinnolle kannattaa soittaa, ovat
tiedostossa [hairiot.md](hairiot.md).

Jos vastauksessa on kenttä `test_seam`, Verohallinnon vastaukset tulivat testiskriptistä
(`AGGOUNTANT_BROKER_SCRIPT`): ohjelma on voinut merkitä kauden ilmoitetuksi, vaikka
mitään ei lähetetty. Kerro se käyttäjälle heti.

## Jo annetun ilmoituksen korjaus

Ilmoitettu kausi on kirjanpidossa lukittu, joten korjaustosite kirjataan seuraavalle
avoimelle kaudelle. Se ei vielä korjaa ilmoitusta, ja ilman merkintää sen vero menisi
kirjauskauden ilmoitukseen. Kun virhe kuuluu korjata sille kaudelle, jolla se tapahtui,
tosite sinetöidään kentällä `corrects_vat_period` (ks. `aggountant voucher seal --help`):
sen vero lasketaan silloin korjattavan kauden lukuihin eikä kirjauskauden. Kentän voi
antaa vain sinetöitäessä, joten ratkaise kohdistus käyttäjän kanssa ennen sinetöintiä.
Verohallinnon ohjeen mukaan taloudellisesti vähäisen virheen saa korjata myös kuluvan
kauden ilmoituksella; silloin kenttä jätetään pois. Kumpi tapa on oikea, on käyttäjän
ratkaisu, ei summasta pääteltävä.

Korjatun kauden `aggountant file-vat preview` näyttää uudet luvut ja sanoo, että
korvaava ilmoitus on antamatta. Se annetaan komennolla `aggountant file-vat replace`:
koko ilmoitus uudelleen ja korjauksen syy, jonka käyttäjä valitsee. Esikatselu näyttää,
mitkä luvut muuttuvat Verohallinnolla nyt olevaan ilmoitukseen nähden; käyttäjä hyväksyy
sen kuten ensimmäisen ilmoituksen, ja epävarma lopputulos selvitetään samoin (alla).
Ajo ei maksa mitään: lisää maksettava tai palautuva erotus näkyy OmaVeron verotilillä,
ja käyttäjä maksaa sen itse. Korvaavaa ei voi antaa ennen kuin Verohallinto on
käsitellyt voimassa olevan ilmoituksen; odota ja yritä uudelleen.

Jos käyttäjä on jo tehnyt korvaavan ilmoituksen OmaVerossa, se kirjataan näyttöineen
komennolla `aggountant file-vat complete-manual` valitsimella `--replacement`.
Aiemmat kuittaukset säilyvät; `aggountant file-vat status` näyttää kauden kaikki
ilmoitukset ja sen, mikä on voimassa.

## Käsin annettu ilmoitus

Kun suora reitti ei ole käytössä tai käyttäjä haluaa asioida itse, esikatselun
numeroidut kentät ovat ilmoitettavat luvut, ellei niistä puutu veroa (yllä). Käyttäjä antaa ne OmaVerossa tai
Ilmoitin.fi-tiedostona ja maksaa erikseen. Jos hän pyytää sinua täyttämään OmaVeron
lomakkeen, [e-services](../e-services/SKILL.md) kertoo rajan ja
[omavero.md](../e-services/omavero.md) sen, miten sivu toimii; lähetys ja sen vahvistus
ovat hänen. Arkistoi hänen toimittamansa kuitti tai muu yksilöity näyttö ja kirjaa
ilmoitus komennolla `aggountant file-vat complete-manual`. Kaudelle, jolla on
selvittämätön suora lähetys, ei anneta ilmoitusta käsin ennen kuin Verohallinnon tila
on selvitetty, koska siitä voisi tulla kaksoisilmoitus.

## Käyttöönotto ja varmenne

`aggountant vero-fi status` on ainoa valmiusarvio, ja ohjelma tekee saman arvion
uudelleen juuri ennen jokaista lähetystä. Kerro sen esteet käyttäjälle sellaisinaan:
kullakin on omistaja, joka kertoo, onko vuoro sinun vai hänen. Suoran ilmoittamisen
käyttöönotto, tunnusten sijainti ja
varmenteen uusinta ovat tiedostossa [kayttoonotto.md](kayttoonotto.md).

## Tulorekisteri

Matkakorvausten ilmoitus ja työnantajan erillisilmoitus kulkevat eri kanavaa ja ovat
omassa skillissään: [tulorekisteri](../tulorekisteri/SKILL.md).
