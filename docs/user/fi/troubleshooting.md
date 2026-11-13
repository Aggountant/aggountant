# Kun jokin menee vikaan

Tämä sivu on järjestetty sen mukaan, mitä huomaat. Jokaisen oireen kohdalla
kerrotaan, mitä todennäköisesti tapahtui, mitä tarkistat ja mitä teet, ja
missä väärä reaktio olisi peruuttamaton, myös mitä et tee. Jos ruudulla on
virhekoodi, katso myös [Virhekoodit](reference/error-codes.md).

Useimmat virheet ovat tarkoituksellisia pysäytyksiä: ohjelma kieltäytyy
tekemästä jotain, mikä tekisi kirjanpidosta väärän. Kieltäytyminen jättää
kirjanpidon ennalleen.

## Ajo keskeytyi

**Mitä todennäköisesti tapahtui.** Kone kaatui, yhteys katkesi tai agentin
istunto päättyi kesken kuukauden sulun, ilmoituksen tai tilinpäätösajon.
Ajot tallentavat jokaisen vaiheensa, joten mitään ei menetetty.

**Tarkista.** Pyydä agenttia näyttämään ajot ja niiden tila: "näytä kesken
olevat ajot".

**Tee näin.** Pyydä agenttia jatkamaan ajoa. Jatkaminen tarkistaa ensin
kirjanpidon todellisen tilan. Jos esimerkiksi kuukauden sulku ehti tapahtua
ennen kaatumista, ajo huomaa sen eikä tee mitään toiseen kertaan.

| Ajo | Jatkaminen |
|---|---|
| kuukauden sulku (`cp-…`) | `aggountant run resume --run-id cp-…` |
| ALV-ilmoitus (`fv-…`), epävarma lähetys | `aggountant file-vat reconcile --run-id fv-…` |
| Tulorekisteri-ilmoitus (`wr-…`), epävarma lähetys | `aggountant file-wage reconcile wr-…` |
| tilinpäätös (`cy-…`) | `aggountant close-year resume --run-id cy-…` |

**Älä** käynnistä ilmoitusta uudelleen alusta. Viranomaisilmoitusta ei koskaan
lähetetä uudelleen automaattisesti, eikä sitä pidä lähettää käsinkään ennen
kuin tiedetään, menikö ensimmäinen perille.

## Kuukausi jäi suljettavaksi

**Mitä todennäköisesti tapahtui.** Kuukauden sulku hylättiin. Hylätty sulku
jättää kuukauden pysyvästi tilaan `closing`: siihen ei voi kirjata, eikä sitä
voi sulkea uudelleen (`PERIOD_TRANSITION_CONFLICT`). Tämä on tunnettu puute.

**Tarkista.** Pyydä agenttia näyttämään kuukauden tila. Jos se on `closing` ja
sulkuajo on `rejected`, kyse on tästä.

**Tee näin.** Tässä versiossa kuukautta ei saa takaisin avoimeksi. Kirjaa
puuttuvat ja korjattavat asiat seuraavalle kuukaudelle ja kerro tilanteesta
projektin [issue-seurannassa](https://github.com/aggountant/aggountant/issues).
Kuukauden ALV-ilmoitus on silti tehtävä OmaVerossa. aggountant ei laske sen
lukuja eikä kirjaa sitä tehdyksi, koska kuukautta ei ole suljettu
(`PERIOD_NOT_SEALED`), joten pyydä agenttia laskemaan luvut kuukauden
tositteista ja tarkista ne itse.

**Älä** yritä muuttaa tilaa käsin tiedostoista. Aloita jatkossa sulku vasta,
kun kuukausi on valmis, ja hyväksy aloitettu sulku
([Sulje kuukausi](tasks/close-month.md)).

## Tositteet näyttävät kadonneen

**Mitä todennäköisesti tapahtui.** Komento osui väärään kotiin. Väärin
kirjoitettu `--home`-polku tai väärästä hakemistosta ajettu komento voi
osoittaa tyhjään tai juuri syntyneeseen kotiin. Väärä polku voi jopa luoda
tyhjän hakemiston (tunnettu puute). Oikea kirjanpito on silloin tallessa
toisessa paikassa. Sama syy on usein koodin `MASTERDATA_NOT_INITIALIZED`
takana.

**Tarkista.** Pyydä agenttia ajamaan `aggountant config show`. Se kertoo,
mikä koti on käytössä ja mistä tieto tuli (valitsin, ympäristömuuttuja,
asetustiedosto tai hakemiston tunniste).

**Tee näin.** Korjaa polku tai aseta oikea koti oletukseksi
([Yrityskoti](concepts/company-home.md#miten-ohjelma-löytää-kodin)). Väärään
paikkaan syntyneen tyhjän hakemiston voi poistaa, kun olet varmistanut, että
siinä ei ole tositteita.

**Älä** aja `init`-komentoa uudelleen "korjataksesi" kotia. Se ei koske
olemassa olevaan kotiin, mutta väärään paikkaan se perustaa uuden, tyhjän
kirjanpidon.

## Pankkitilin saldo ei täsmää tiliotteeseen

**Mitä todennäköisesti tapahtui.** Jokin tiliotteen tapahtuma puuttuu
kirjanpidosta, tai sama tapahtuma on kirjattu kahdesti. Tavallisin syy
kahdenkertaiseen kirjaukseen on osto, joka on kirjattu ensin kuitista
pankkitiliä vastaan ja sitten uudelleen tiliotteen rivistä kuluksi. Tiliotteen
tuonti vaatii kirjauksen jokaiselle riville, myös jo kirjatun kuitin
maksulle ([Mitä on valmista ja mitä ei](reference/status.md#pankin-tiliote)).

**Tarkista.** Pyydä agenttia vertaamaan kuukauden pankkitilin vientejä
tiliotteen riveihin ja etsimään tapahtumat, jotka puuttuvat tai esiintyvät
kahdesti. Erotus on usein täsmälleen jonkin kuitin summa.

**Tee näin.** Puuttuva tapahtuma kirjataan. Kahdesti kirjattu kumotaan
vastakirjauksella ([Korjaa virheellinen tosite](tasks/correct-voucher.md)).
Tee tämä ennen kuukauden sulkua, jolloin korjaus osuu samalle kuukaudelle.

## Ilmoitus ei näy OmaVerossa

**Mitä todennäköisesti tapahtui.** Verohallinto on vastaanottanut
ilmoituksen, mutta ei ole vielä käsitellyt sitä. Rajapinnan kautta tullut
ilmoitus on näkynyt OmaVerossa tavallisesti vasta noin puolen vuorokauden
kuluttua ([Viranomaisilmoitukset](concepts/filings.md#kuittaus-ei-ole-käsittely)).

**Tarkista.** Pyydä agenttia kysymään ilmoituksen tila
(`aggountant file-vat status --period 2026-03`). Komento vain lukee: se ei
lähetä mitään eikä muuta kirjanpitoa tai ajoa. Kun kaudella on kuittaus,
komento kysyy Verohallinnolta, miten ilmoitus näkyy siellä, ja kertoo yhden
tilan:

- `received`: kuittaus on, mutta Verohallinto ei vielä näytä ilmoitusta.
- `pending_authority_processing`: Verohallinto käsittelee ilmoitusta tai
  näyttää vielä aiemman ilmoituksen.
- `processed`: Verohallinto on käsitellyt ilmoituksen hyväksytyin luvuin.
- `processed_values_differ`: Verohallinnossa on kuittauksen jälkeen käsitelty
  ilmoitus, jonka luvut poikkeavat hyväksytyistä. Vertaa sitä OmaVerossa.

Ilmoituksen todistaa kuittaus, ei se, näkyykö ilmoitus OmaVerossa. Jos tilan
kysely epäonnistuu (`VAT_STATUS_UNKNOWN`), tila on vain tuntematon:
ilmoitus on silti tehty. Jos lähetyksen lopputulos on epävarma, komento ohjaa
`file-vat reconcile`-komentoon
([alla](#ilmoituksen-lopputulos-on-epävarma)).

**Tee näin.** Odota seuraavaan arkipäivään. Jos ilmoitus ei sittenkään näy tai
eräpäivä on lähellä, soita Verohallinnon ALV-palveluun. Agentti kokoaa sinulle
puhelua varten kauden, kuittauksen tunnisteen ja aikaleiman, Y-tunnuksen ja
lähetetyt kentät.

**Älä** lähetä ilmoitusta uudelleen sen takia, että se ei vielä näy. Toinen
lähetys olisi Verohallinnossa korvaava ilmoitus, eikä aggountant edes salli
toista lähetystä samalle kaudelle (`PERIOD_TRANSITION_CONFLICT`).

## Ilmoituksen lopputulos on epävarma

**Mitä todennäköisesti tapahtui.** Yhteys katkesi tai viranomaisen palvelu
vastasi virheellä kesken lähetyksen, eikä ohjelma tiedä, menikö ilmoitus
perille. Ajo jää huomiota vaativaan tilaan
([Viranomaisilmoitukset](concepts/filings.md#epävarma-lopputulos)).

**Tarkista.** Pyydä agenttia kysymään tila viranomaiselta. Kysely ei lähetä
ilmoitusta.

**Tee näin.**

- Jos viranomaisella on täsmälleen sama ilmoitus, ajo päättyy ilman uutta
  lähetystä.
- Jos ilmoitusta ei näy tai se poikkeaa, odota ja pyydä agenttia kysymään
  uudelleen myöhemmin, tai tarkista OmaVerosta tai Tulorekisterin
  asiointipalvelusta itse.
- Jos päätät tehdä ilmoituksen käsin, selvitä ensin, ettei rajapintalähetys
  mennyt perille. ALV-ilmoituksen käsin kirjaaminen vaatii silloin näytön
  Verohallinnon tilasta.

**Älä** toista hyväksyntää tai käynnistä ilmoitusta uudelleen. Hyväksynnän
toisto ei lähetä uudelleen, ja uusi ajo voisi tuottaa kaksoisilmoituksen.
`file-vat retry` koskee vain maksuvaihetta, ei ilmoitusta.

## Tulorekisteri hylkäsi ilmoituksen

**Mitä todennäköisesti tapahtui.** Tulorekisteri on käsitellyt ilmoituksen ja
hylännyt sen. Ajo jää huomiota vaativaan tilaan. Ohjelma ei näytä
Tulorekisterin virhetekstiä, koska siinä voi olla henkilötunnus.

**Tarkista.** Katso tarkka virhe Tulorekisterin asiointipalvelusta.

**Tee näin.** Tee ilmoitus tai korjaus Tulorekisterin asiointipalvelussa. Anna
sen jälkeen agentille palvelun antamat aineiston ja ilmoituksen tunnisteet,
niin agentti kirjaa ne aggountantiin
(`aggountant file-wage complete-manual wr-… --ir-delivery-id … --ir-report-id …`).
Sama varapolku käy, kun rajapintapolku ei muuten onnistu.

**Älä** tee asiointipalvelussa uutta alkuperäistä ilmoitusta ennen kuin olet
tarkistanut, onko epävarma rajapintalähetys jo mennyt perille. Älä myöskään
vaihda maksun tunnistetta tai maksupäivää.

## ALV-ilmoitusajo jää odottamaan maksua

**Mitä todennäköisesti tapahtui.** Ilmoitus on lähetetty, ja ajo avasi maksun
hyväksynnän, koska veroa on maksettavaa. aggountantilla ei ole maksuyhteyttä,
joten tämä vaihe ei valmistu.

**Tee näin.** Maksa vero OmaVeron viitteellä itse. Kuukausi on ilmoitettu
(`vat_filed true`), ja odottava maksuvaihe voi jäädä sellaiseksi.

## Ketjun tarkistus epäonnistuu

**Mitä todennäköisesti tapahtui.** Tositetietokantaa on käsitelty muuten kuin
aggountantilla, tai tiedosto on vaurioitunut. `aggountant voucher verify`
tarkistaa, ettei yhtään sinetöityä tositetta ole muutettu tai poistettu.

**Tee näin.** Palauta koti varmuuskopiosta ja aja tarkistus uudelleen
([Varmuuskopioi ja palauta kirjanpito](tasks/back-up-and-restore.md)).

**Älä** yritä korjata tietokantaa käsin. Se ei palauta ketjua ehjäksi, ja
voit menettää tiedon siitä, mitä tapahtui.

## Ajo ei jatku päivityksen jälkeen

**Mitä todennäköisesti tapahtui.** Ajo aloitettiin vanhemmalla
ohjelmaversiolla, ja sen määritys on muuttunut (`WORKFLOW_VERSION_MISMATCH`).

**Tee näin.** Anna agentille lupa siirtää ajo uuteen versioon
([Päivitä aggountant](tasks/upgrade.md)).

## Ilmoitus merkittiin tehdyksi, vaikka mitään ei lähetetty

**Mitä todennäköisesti tapahtui.** Ympäristömuuttuja `AGGOUNTANT_BROKER_SCRIPT`
on asetettuna. Se on tarkoitettu vain ohjelman testaukseen ja korvaa
Verohallinnon vastaukset etukäteen kirjoitetuilla vastauksilla. Kun se on
asetettuna, `file-vat` voi merkitä kauden ilmoitetuksi, vaikka mitään ei
lähetetty.

**Tarkista.** Pyydä agenttia tarkistamaan, onko muuttuja asetettuna.

**Tee näin.** Poista muuttuja ja tee ilmoitus OmaVerossa
([Tee ALV-ilmoitus OmaVerossa](tasks/file-vat-omavero.md)). Kauden merkintä
ilmoitetuksi on pysyvä, joten kerro tilanteesta projektin
[issue-seurannassa](https://github.com/aggountant/aggountant/issues).

**Älä** tee oikeita ilmoituksia, kun muuttuja on asetettuna.

## Katso myös

- [Virhekoodit](reference/error-codes.md)
- [Mitä on valmista ja mitä ei](reference/status.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](concepts/filings.md)
