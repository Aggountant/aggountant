# Kun jokin menee vikaan

Etsi alta se, mitä huomasit. Jos ruudulla on virhekoodi, katso myös
[Virhekoodit](reference/error-codes.md).

Useimmat virheet ovat tarkoituksellisia pysäytyksiä: aggountant kieltäytyy
tekemästä jotain, mikä tekisi kirjanpidosta väärän. Kieltäytyminen jättää
kirjanpidon ennalleen.

## Ajo keskeytyi

**Mitä todennäköisesti tapahtui.** Kone kaatui, yhteys katkesi tai agentin
istunto päättyi kesken kuukauden sulun, ilmoituksen tai tilinpäätösajon.
Ajo tallentaa jokaisen vaiheensa, joten mitään ei menetetty.

**Tarkista.** Pyydä agenttia näyttämään ajot ja niiden tila: "näytä kesken
olevat ajot".

**Tee näin.** Pyydä agenttia jatkamaan ajoa. Ajo tarkistaa ensin kirjanpidon
todellisen tilan. Jos esimerkiksi kuukausi ehti sulkeutua ennen kaatumista,
ajo huomaa sen eikä tee mitään toiseen kertaan.

| Ajo | Jatkaminen |
|---|---|
| kuukauden sulku (`cp-…`) | `aggountant run resume --run-id cp-…` |
| ALV-ilmoitus (`fv-…`), pysähtyi ennen lähetystä (`running`, `reconcile` `executing`) | `aggountant run resume --run-id fv-…` |
| ALV-ilmoitus (`fv-…`), epävarma lähetys | `aggountant file-vat reconcile --run-id fv-…` |
| ALV-ilmoitus (`fv-…`), keskeytyi lähetettäessä | `aggountant file-vat status --period VVVV-KK` ja sen nimeämä komento ([alla](#alv-ilmoitus-keskeytyi-lähetettäessä)) |
| Tulorekisteri-ilmoitus (`wr-…`), epävarma lähetys | `aggountant file-wage reconcile wr-…` |
| Tulorekisteri-ilmoituksen korjaus (`wc-…`), epävarma lähetys | `aggountant file-wage correction status wr-…` |
| työnantajan erillisilmoitus (`er-…`), epävarma lähetys | `aggountant file-wage employer-report status --month VVVV-KK` |
| tilinpäätös (`cy-…`) | `aggountant close-year resume --run-id cy-…` |

ALV-ilmoitusajo, joka pysähtyi ennen kuin mitään lähetettiin, päättyy
jatkettaessa tilaan `not_filed`, ja sen `reconcile`-vaiheessa on koodi
`VAT_INTERRUPTED_BEFORE_FILING`. Tilassa `not_filed` oleva ajo on päättynyt:
se ei antanut ilmoitusta eikä lähettänyt mitään, joten se ei vaadi toimia
eikä estä uutta ajoa samalle kaudelle. Jos ajo on voinut jo aloittaa
lähetyksen, jatkaminen kieltäytyy (`VAT_FILING_UNRESOLVED`) ja ohjaa
komentoon `file-vat status`.

**Älä** käynnistä ilmoitusta uudelleen alusta. Ohjelma ei lähetä
viranomaisilmoitusta uudelleen itsestään, eikä sitä pidä lähettää käsinkään,
ennen kuin tiedät, menikö ensimmäinen perille.

## Kuukausi jäi suljettavaksi

**Mitä todennäköisesti tapahtui.** Kuukauden sulku odottaa yhä vastaustasi.
Toinen mahdollisuus on, että sulku hylättiin aiemmalla versiolla tai kone
kaatui kesken hylkäyksen. Ohjelma kieltäytyy kirjaamasta kuukauteen koodilla
`PERIOD_CLOSING` ja aloittamasta uutta sulkua koodilla
`PERIOD_TRANSITION_CONFLICT`.

**Tarkista.** Pyydä agenttia näyttämään kuukauden tila ja sulkuajon tila.

**Tee näin.** Jos sulkuajon tila on `awaiting_gate`,
[hyväksy tai hylkää sulku](tasks/close-month.md). Jos tila on `rejected`,
pyydä agenttia jatkamaan ajoa (`aggountant run resume --run-id cp-…`). Ajo vie
silloin hylkäyksen loppuun ja palauttaa kuukauden tilaan `open`, ja voit taas
kirjata kuukauteen ja aloittaa uuden sulun.

**Älä** yritä muuttaa tilaa käsin tiedostoista.

## Tositteet näyttävät kadonneen

**Mitä todennäköisesti tapahtui.** Komento osui väärään kotiin. Väärin
kirjoitettu `--home`-polku tai väärästä hakemistosta ajettu komento voi
osoittaa toiseen kotiin tai hakemistoon, jossa ei ole kirjanpitoa. Oikea
kirjanpito on tallessa omassa paikassaan. Jos polussa ei ole perustettua
kotia, komento pysähtyy koodiin `HOME_NOT_INITIALIZED` eikä kirjoita mitään.
Virheilmoitus kertoo, mitä polkua komento katsoi ja miten polku valittiin.

**Tarkista.** Pyydä agenttia ajamaan `aggountant config show`. Se kertoo,
mikä koti on käytössä ja mistä tieto tuli: valitsimesta,
ympäristömuuttujasta, asetustiedostosta, hakemiston tunnisteesta vai
oletuksesta eli alihakemistosta `accounting`.

**Tee näin.** Korjaa polku tai
[aseta oikea koti oletukseksi](concepts/company-home.md#miten-ohjelma-löytää-kodin).
Aiemmat versiot saattoivat jättää väärään paikkaan tyhjän kodin
(`accounting`-hakemiston, jossa on `fiscal.sqlite` ja `artifacts/`). Voit
poistaa sen, kun olet tarkistanut, ettei siinä ole tositteita.

**Älä** aja `init`-komentoa uudelleen "korjataksesi" kotia. Olemassa olevaan
kotiin se ei koske, mutta väärään paikkaan se perustaa uuden, tyhjän
kirjanpidon.

## Pankkitilin saldo ei täsmää tiliotteeseen

**Mitä todennäköisesti tapahtui.** Jokin tiliotteen tapahtuma puuttuu
kirjanpidosta, tai sama tapahtuma on kirjattu kahdesti. Tavallisimmin
kahdesti on kirjattu osto, jonka kuitti on kirjattu pankkitiliä vastaan,
vaikka maksu kirjataan myös tiliotteelta. Kuitin vastatili on ostovelka 2700
(tai 2740), ja tiliotteen rivi kuittaa velan
([Kun osto näkyy myös tiliotteella](tasks/record-receipts.md#kun-osto-näkyy-myös-tiliotteella)).

Jos pankkitilin saldo täsmää mutta ostovelkoihin jää summa, jota yritys ei
ole kenellekään velkaa, sama osto on todennäköisesti kirjattu kuitista
ostovelaksi ja tiliotteen riviltä vielä kuluksi.

**Tarkista.** Pyydä agenttia vertaamaan kuukauden pankkitilin vientejä
tiliotteen riveihin ja etsimään tapahtumat, jotka puuttuvat tai esiintyvät
kahdesti. Erotus on usein täsmälleen jonkin kuitin summa.

**Tee näin.** Pyydä agenttia kirjaamaan puuttuva tapahtuma ja kumoamaan
kahdesti kirjattu [vastakirjauksella](tasks/correct-voucher.md). Tee tämä
ennen kuukauden sulkua, niin korjaus osuu samalle kuukaudelle.

## Ilmoitus ei näy OmaVerossa

**Mitä todennäköisesti tapahtui.** Verohallinto on vastaanottanut
ilmoituksen mutta ei ole vielä käsitellyt sitä. Rajapinnan kautta lähetetty
ilmoitus on näkynyt OmaVerossa tavallisesti vasta noin puolen vuorokauden
kuluttua ([Kuittaus ei ole käsittely](concepts/filings.md#kuittaus-ei-ole-käsittely)).

**Tarkista.** Pyydä agenttia kysymään ilmoituksen tila
(`aggountant file-vat status --period 2026-03`). Komento ei lähetä mitään
eikä muuta kirjanpitoa tai ajoa. Kun kaudella on kuittaus, komento kysyy
Verohallinnolta, miten ilmoitus näkyy siellä, ja kertoo yhden tilan:

- `received`: kuittaus on, mutta Verohallinto ei vielä näytä ilmoitusta.
- `pending_authority_processing`: Verohallinto käsittelee ilmoitusta tai
  näyttää vielä aiemman ilmoituksen.
- `processed`: Verohallinto on käsitellyt ilmoituksen hyväksymilläsi luvuilla.
- `processed_values_differ`: Verohallinto on kuittauksen jälkeen käsitellyt
  ilmoituksen, jonka luvut poikkeavat hyväksymistäsi. Vastaus luettelee
  jokaisen poikkeavan kentän ja sen molemmat luvut (`differences`, myös
  viestissä). Vertaa näitä kenttiä OmaVerossa. Kenttä, jonka Verohallinto
  jättää pois, ei ole poikkeama, kun hyväksymäsi luku on nolla.

Kuittaus todistaa, että ilmoitus on tehty, vaikka ilmoitus ei vielä näkyisi
OmaVerossa. Jos tilan kysely epäonnistuu (`VAT_STATUS_UNKNOWN`), tilaa ei
saatu selville, mutta ilmoitus on silti tehty. Jos lähetyksen lopputulos on
epävarma, komento ohjaa `file-vat reconcile` -komentoon
([Ilmoituksen lopputulos on epävarma](#ilmoituksen-lopputulos-on-epävarma)).
Jos ajo keskeytyi kesken lähetyksen, komento kysyy Verohallinnolta tuon ajon
ilmoituksesta
([ALV-ilmoitus keskeytyi lähetettäessä](#alv-ilmoitus-keskeytyi-lähetettäessä)).

**Tee näin.** Odota seuraavaan arkipäivään. Jos ilmoitus ei sittenkään näy tai
eräpäivä on lähellä, soita Verohallinnon ALV-palveluun. Agentti kokoaa sinulle
puhelua varten kauden, kuittauksen tunnisteen ja aikaleiman, Y-tunnuksen ja
lähetetyt kentät.

**Älä** lähetä ilmoitusta uudelleen sen takia, että se ei vielä näy. Toinen
lähetys olisi Verohallinnossa korvaava ilmoitus, ja aggountant kieltäytyy
uudesta ilmoitusajosta jo ilmoitetulle kaudelle
(`PERIOD_TRANSITION_CONFLICT`).

## Ilmoituksen lopputulos on epävarma

**Mitä todennäköisesti tapahtui.** Yhteys katkesi tai viranomaisen palvelu
vastasi virheellä kesken lähetyksen, tai palvelu ilmoitti onnistumisesta
ilman luettavaa kuittia. Ohjelma ei tiedä, menikö ilmoitus perille, ja ajo jää
[huomiota vaativaan tilaan](concepts/filings.md#epävarma-lopputulos).

**Tarkista.** Pyydä agenttia kysymään tila viranomaiselta. Kysely ei lähetä
ilmoitusta.

**Tee näin.**

- Jos viranomaisella on täsmälleen sama ilmoitus, ajo päättyy ilman uutta
  lähetystä.
- Jos ilmoitusta ei näy tai se poikkeaa, odota ja pyydä agenttia kysymään
  myöhemmin uudelleen, tai tarkista itse OmaVerosta tai Tulorekisterin
  asiointipalvelusta.
- ALV-ilmoituksessa ajo sulkeutuu ilmoittamattomana, kun Verohallinto on
  vastannut "ei löydy" kahdesti vähintään yhden pankkipäivän välein. Komennon
  vastaus kertoo, mistä hetkestä alkaen toinen kysely lasketaan
  (`next_check_from`): samasta kellonajasta seuraavana pankkipäivänä. Aloita
  sulkeutumisen jälkeen kaudelle uusi ilmoitusajo.
- Jos päätät tehdä ilmoituksen käsin, selvitä ensin, ettei rajapintalähetys
  mennyt perille. Käsin tehdyn ALV-ilmoituksen kirjaaminen vaatii silloin
  näytön Verohallinnon tilasta.

**Älä** toista hyväksyntää tai käynnistä ilmoitusta uudelleen ennen sitä.
Hyväksynnän toisto ei lähetä uudelleen, ja aggountant kieltäytyy uudesta
saman kauden ALV-ilmoitusajosta (`VAT_FILING_UNRESOLVED`), koska se voisi
tuottaa kaksoisilmoituksen.

## ALV-ilmoitus keskeytyi lähetettäessä

**Mitä todennäköisesti tapahtui.** Kone kaatui tai istunto päättyi, kun
ALV-ilmoitusta lähetettiin, eikä lopputulos ehtinyt tallentua. Ilmoitus on
voinut mennä perille tai jäädä menemättä. Ajo näyttää ilmoituksen yhä kesken
olevana, ja aggountant kieltäytyy uudesta saman kauden ilmoitusajosta
(`VAT_FILING_UNRESOLVED`).

**Tarkista.** Pyydä agenttia kysymään tila
(`aggountant file-vat status --period 2026-03`). Komento katsoo, mitä
keskeytynyt ajo jätti jälkeensä, ja kysyy Verohallinnolta. Se ei lähetä eikä
kirjaa mitään. Se kertoo yhden tilan:

- `interrupted_filed`: Verohallinto otti ilmoituksen vastaan. Kuittaus ehti
  tallentua ennen keskeytystä, tai Verohallinto näyttää hyväksymäsi
  ilmoituksen käsiteltynä.
- `interrupted_not_filed`: ilmoitus ei päässyt Verohallintoon tai
  Verohallinto hylkäsi sen suoraan, eikä Verohallinnolla ole kaudelta
  ilmoitusta.
- `filing_interrupted`: asia ei ole vielä selvä. Ilmoitus on voinut päätyä
  Verohallintoon, joka ei vielä näytä sitä, tai Verohallinto näyttää kaudelta
  jotain muuta.

**Tee näin.** Pyydä agenttia ajamaan tilan nimeämä komento
(`aggountant file-vat reconcile --run-id fv-…`). Se kirjaa vastauksen:

- Jos Verohallinto otti ilmoituksen vastaan, komento kirjaa ilmoituksen ja
  sen kuittauksen, jos sellainen on, ja ajo jatkuu maksuvaiheeseen. Mitään ei
  lähetetä.
- Jos ilmoitus ei päässyt Verohallintoon, komento sulkee ajon
  ilmoittamattomana ja tallentaa Verohallinnon vastauksen ajoon. Aloita
  kaudelle uusi ilmoitusajo. Se lähettää ilmoituksen kerran.
- Jos ilmoitus on voinut päätyä Verohallintoon mutta ei näy, toimi kuten
  [epävarman lopputuloksen](#ilmoituksen-lopputulos-on-epävarma) kanssa.
  Ilmoituksen näkyminen voi kestää päiviä, joten ajo sulkeutuu vasta toisen
  "ei löydy" -vastauksen jälkeen.

Jos ajo ei lähettänyt mitään mutta Verohallinto näyttää hyväksymäsi
ilmoituksen käsiteltynä, ilmoitus on tehty muuta kautta. Varmista se
OmaVerosta ja pyydä agenttia kirjaamaan se käsin tehdyksi
(`file-vat complete-manual`, näyttönä Verohallinnon vastaus). Sen jälkeen
`file-vat reconcile` sulkee keskeytyneen ajon ottamatta ilmoitusta omakseen.

Jos kysely epäonnistuu (`VAT_STATUS_UNKNOWN`), mikään ei muutu. Kysy
myöhemmin uudelleen.

**Älä** aloita uutta ilmoitusajoa tai ilmoita käsin, ennen kuin tila on
kertonut, mitä tapahtui. Tuntematon vastaus ei ole syy ilmoittaa uudelleen.
`file-vat retry` koskee vain maksuvaihetta, ei ilmoitusta.

## Tulorekisteri hylkäsi ilmoituksen

**Mitä todennäköisesti tapahtui.** Tulorekisteri on käsitellyt ilmoituksen ja
hylännyt sen. Ajo jää huomiota vaativaan tilaan. Ohjelma ei näytä
Tulorekisterin virhetekstiä, koska siinä voi olla henkilötunnus.

**Tarkista.** Katso tarkka virhe Tulorekisterin asiointipalvelusta.

**Tee näin.** Toimi sen mukaan, mikä ilmoitus hylättiin:

- Matkakorvausilmoitus: anna ilmoitus Tulorekisterin asiointipalvelussa ja
  kerro agentille palvelun antamat aineiston ja ilmoituksen tunnisteet.
  Agentti kirjaa ne aggountantiin
  (`aggountant file-wage complete-manual wr-… --ir-delivery-id … --ir-report-id …`).
  Sama varapolku käy, kun rajapintalähetys ei muusta syystä onnistu. Komento
  kirjaa vain ilmoituksen, jota Tulorekisteri ei aggountantin tietojen mukaan
  ole vielä hyväksynyt. Hyväksytylle ilmoitukselle se kieltäytyy
  (`WAGE_REPORT_ALREADY_ACCEPTED`), ja
  [korjaus tehdään ohjelmalla](tasks/report-travel-allowances.md#korjaa-hyväksytty-ilmoitus).
- Korjaus: alkuperäinen ilmoitus on rekisterissä ennallaan. Korjaa
  hylkäyksen syy ja pyydä agenttia esikatselemaan korjaus uudelleen.
- Työnantajan erillisilmoitus: korjaa hylkäyksen syy ja pyydä agenttia
  esikatselemaan ja lähettämään ilmoitus uudelleen. Voit myös antaa
  ilmoituksen asiointipalvelussa, jolloin agentti kirjaa sen viitteen
  (`aggountant file-wage employer-report complete-manual --month … --ir-report-id …`).

**Älä** tee asiointipalvelussa uutta alkuperäistä ilmoitusta, ennen kuin olet
tarkistanut, onko epävarma rajapintalähetys jo mennyt perille. Älä myöskään
vaihda maksun tunnistetta tai maksupäivää kiertääksesi hylkäyksen, koska
silloin syntyisi huomaamatta uusi ilmoitus.

## ALV-ilmoitusajo jää odottamaan maksua

**Mitä todennäköisesti tapahtui.** Ilmoitus lähetettiin aggountantin
aiemmalla versiolla, joka jätti maksettavan ilmoituksen ajon odottamaan maksun
hyväksyntää `ApprovePayment`. aggountantilla ei ole maksuyhteyttä, joten
hyväksyntä ei mene läpi. Kuukausi on ilmoitettu (`vat_filed true`).

**Tee näin.** Maksa vero itse OmaVeron viitteellä. Agentti päättää ajon
komennolla `file-vat user-pays --run-id fv-…`; se ei lähetä mitään ja kirjaa,
että maksat veron, ei sitä, että se on maksettu.

## Ketjun tarkistus epäonnistuu

**Mitä todennäköisesti tapahtui.** Tositetietokantaa on käsitelty muulla kuin
aggountantilla, tai tiedosto on vaurioitunut. `aggountant voucher verify`
tarkistaa, ettei yhtään sinetöityä tositetta ole muutettu tai poistettu.

**Tee näin.**
[Palauta koti varmuuskopiosta](tasks/back-up-and-restore.md) ja aja tarkistus
uudelleen.

**Älä** yritä korjata tietokantaa käsin. Ketju ei palaudu siitä ehjäksi, ja
voit menettää tiedon siitä, mitä tapahtui.

## Ajo ei jatku päivityksen jälkeen

**Mitä todennäköisesti tapahtui.** Ajo aloitettiin vanhemmalla
ohjelmaversiolla, ja ajon määritys on sen jälkeen muuttunut
(`WORKFLOW_VERSION_MISMATCH`).

**Tee näin.** Anna agentille lupa
[siirtää ajo uuteen versioon](tasks/upgrade.md).

## Ilmoitus merkittiin tehdyksi, vaikka mitään ei lähetetty

**Mitä todennäköisesti tapahtui.** Ympäristömuuttuja `AGGOUNTANT_BROKER_SCRIPT`
on asetettuna. Se on tarkoitettu vain ohjelman testaukseen ja korvaa
Verohallinnon vastaukset etukäteen kirjoitetuilla. Silloin `file-vat` voi
merkitä kauden ilmoitetuksi, vaikka mitään ei lähetetty.

**Tarkista.** Pyydä agenttia tarkistamaan, onko muuttuja asetettuna. Komento
kertoo siitä myös itse: tulosteessa on kenttä `test_seam` (virheessäkin), ja
ilman `--json`-valitsinta virhevirtaan tulee varoitus, joka alkaa
`warning: AGGOUNTANT_BROKER_SCRIPT is set`.

**Tee näin.** Poista muuttuja ja
[tee ilmoitus OmaVerossa](tasks/file-vat-omavero.md). Kauden merkintä
ilmoitetuksi on pysyvä, joten kerro tilanteesta projektin
[issue-seurannassa](https://github.com/aggountant/aggountant/issues).

**Älä** tee oikeita ilmoituksia, kun muuttuja on asetettuna.

## Katso myös

- [Virhekoodit](reference/error-codes.md)
- [Mitä on valmista ja mitä ei](reference/status.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](concepts/filings.md)
