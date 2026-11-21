# Virhekoodit

Tältä sivulta löydät tavallisimmat virhekoodit, joihin törmäät, ja mitä niille
tehdään. Koodit ovat aggountantin omia, ja ne näkyvät sellaisinaan agentin
tulosteessa.

## Miten virhe näkyy

Kun komento epäonnistuu, se tulostaa virheen koodeineen, esimerkiksi:

```json
{"error_code":"PERIOD_SEALED","error_kind":"user","message":"PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period","hint":"kirjaa seuraavalle avoimelle kaudelle"}
```

- `error_code` on pysyvä koodi, jonka perusteella agentti tietää, mitä tehdä.
- `error_kind` on `user`, kun syy on syötteessä tai kirjanpidon tilassa ja sen
  voi korjata (paluuarvo 1), tai `system`, kun syy on ympäristössä, kuten
  tiedostossa tai yhteydessä (paluuarvo 2).
- `message` kertoo syyn sanallisesti.
- `hint` on tämän sivun taulukon "Mitä tehdään"-sarake suomeksi. Se on
  mukana vain koodeilla, jotka taulukossa on.
- `test_seam` on mukana vain, kun testimuuttuja `AGGOUNTANT_BROKER_SCRIPT`
  vastasi viranomaisen puolesta
  ([Kun jokin menee vikaan](../troubleshooting.md#ilmoitus-merkittiin-tehdyksi-vaikka-mitään-ei-lähetetty)).

Agentti voi pyytää koko selityksen komennolla `aggountant explain KOODI`
(englanniksi `--lang en`). Ohjelma lukee selitykset tältä sivulta, joten
ne ovat samat kuin alla.

Useimmat virheet ovat tarkoituksellisia pysäytyksiä: ohjelma kieltäytyy
tekemästä jotain, mikä tekisi kirjanpidosta väärän. Kieltäytyminen jättää
kirjanpidon ennalleen. Vero.fi-valmiuden esteiden koodit (`VERO_…`) ovat
omalla sivullaan: [Vero.fi-valmiuden esteet](vero-fi-blockers.md).

## Koodit

| Koodi | Mitä tapahtui | Mitä tehdään |
|---|---|---|
| `ACCOUNT_NOT_FOUND` | kirjaus viittaa tiliin, jota tilikartassa ei ole | agentti valitsee tilin tilikartasta |
| `ALREADY_INITIALIZED` | kodin perustaminen ajettiin olemassa olevaan kotiin | ei mitään; koti on jo olemassa eikä siihen koskettu |
| `APPROVAL_REQUIRED` | kirjausta yritettiin ilman hyväksyttyä esikatselua | esikatselu ja hyväksyntä ensin |
| `CAPTURE_VAT_CLASS_UNKNOWN` | kuittiehdotus nimeää ALV-verokantaluokan, jota aggountant ei tunne; mitään ei kirjattu | agentti valitsee kuitin ALV-käsittelyä vastaavan luokan; viesti luettelee tunnetut luokat |
| `CAPTURE_PREPARATION_CHANGED` | kuitin tiedot, ehdotus tai tilikartta muuttuivat hyväksynnän jälkeen | uusi esikatselu ja uusi hyväksyntä ([Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)) |
| `CLOSE_CONTENT_CHANGED` | sulun hyväksyntäportin näyttämä sisältö on muuttunut portin avautumisen jälkeen | hylkää sulku ja aloita se uudelleen ([Sulje kuukausi](../tasks/close-month.md)) |
| `EMPLOYER_REGISTER_MISMATCH` | erillisilmoituksen alkukuukausi on eri kuin YTJ:n työnantajarekisterissä; mitään ei tallennettu | tarkista vertailu; käytä rekisterin kuukautta tai vahvista oma kuukautesi agentille ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#ota-erillisilmoitus-käyttöön-kerran)) |
| `EMPLOYER_REPORT_ALREADY_FILED` | kuukaudella on jo Tulorekisterin vastaanottama työnantajan erillisilmoitus | ei uutta ilmoitusta; korjaus on korvaava ilmoitus ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#jos-jokin-menee-pieleen)) |
| `EMPLOYER_REPORT_FILED_OUTSIDE` | tuodun raportin 630 mukaan kuukaudelle on jo annettu erillisilmoitus aggountantin ulkopuolella | ei uutta ilmoitusta; korjaus asiointipalvelussa tai kirjaa ensin viite ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#kun-aiempi-palkanlaskija-on-antanut-ilmoituksia)) |
| `EMPLOYER_REPORT_INVALID_EVIDENCE` | `complete-manual`-komennolla annettu asiointipalvelun näyttö ei ratkaise kuukautta: ilmoitusviite ei ole 32-merkkinen viite, tai epävarman korvaavan ilmoituksen versio puuttuu tai ei ole korvattua uudempi | anna asiointipalvelun näyttämät viitteet ja versio; jos palvelussa näkyy vanha versio, korvaava ei ole perillä ja kuukausi jää odottamaan; jos korvattua versiota ei kirjattu, kysy tila komennolla `file-wage employer-report status` |
| `EMPLOYER_REPORT_INVALID_SETTINGS` | erillisilmoituksen asetusten työeläkevakuutuksen eläkelaitostunnus tai vakuutusnumero ei kelpaa ilmoitukseen | tarkista eläkelaitoksen tunnus ja vakuutusnumero |
| `EMPLOYER_REPORT_NOT_CONFIGURED` | Tulorekisteri-profiilista puuttuu erillisilmoituksen asetus | [Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#ota-erillisilmoitus-käyttöön-kerran) |
| `EMPLOYER_REPORT_NO_REFERENCE` | korvattava kuukausi tunnetaan vain raportista 630, jossa ei ole ilmoitusviitettä | korvaa asiointipalvelussa tai kirjaa ensin viite (`complete-manual`) |
| `EMPLOYER_REPORT_PREVIOUS_STATED_NEEDLESSLY` | korvattavan ilmoituksen eläkevakuutustiedot annettiin, vaikka aggountant tuntee ne jo (se lähetti ilmoituksen) tai mitään ei korvata | jätä pois `--previous-pension-insurance` ja `--previous-no-pension-insurance` |
| `EMPLOYER_REPORT_PREVIOUS_UNKNOWN` | korvattava ilmoitus annettiin aggountantin ulkopuolella ja siitä kirjattiin vain viite, joten sen eläkevakuutustietoja, jotka korvaavan on toistettava, ei tunneta | katso ne ilmoituksesta asiointipalvelussa ja anna ne (`--previous-pension-insurance` tai `--previous-no-pension-insurance`), tai korjaa ilmoitus asiointipalvelussa |
| `EMPLOYER_REPORT_OUTCOME_UNKNOWN` | kuukauden aiemman lähetyksen tulos on vielä epäselvä | kysy tila (`employer-report status`), älä lähetä uudelleen ([Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma)) |
| `EMPLOYER_REPORT_PAYMENTS_CONFLICT` | "Ei palkanmaksua" kuukaudelle, jolle on matkakorvausilmoitus | ilmoita kuukausi vain matkakorvauksia sisältäväksi |
| `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED` | korvaava ilmoitus muuttaisi eläkevakuutustietoja | mitätöi ilmoitus asiointipalvelussa ja anna sinne uusi |
| `EMPLOYER_REPORT_STATUS_UNKNOWN` | Tulorekisteri ei antanut lopullista tilaa | kysy myöhemmin uudelleen tai tarkista asiointipalvelusta; älä lähetä uudelleen |
| `EMPLOYER_REPORT_TRANSCRIPT_CONFLICT` | tuodut raportit 630 ovat eri mieltä kuukaudesta | tarkista kuukausi asiointipalvelusta ja kirjaa ilmoituksen viite (`complete-manual`) |
| `EMPLOYER_REPORT_WAGES_UNSUPPORTED` | kuukaudella maksettiin palkkaa, eikä aggountant laske sairausvakuutusmaksua | anna ilmoitus asiointipalvelussa ([Mitä on valmista ja mitä ei](status.md#tulorekisteri-ja-työntekijät)) |
| `FISCAL_YEAR_CONFLICT` | tilikauden määrittely siirtäisi kirjattuja tositteita tai suljetun tilikauden toiselle tilikaudelle | lue viesti; määrittely ei voi muuttaa jo kirjattua |
| `FISCAL_YEAR_UNDEFINED` | päivä on ennen yrityksen ensimmäistä tilikautta, tai annettuna vuonna ei pääty tilikautta | tarkista päivä tai vuosi; `fiscal-year list` näyttää määrittelyt |
| `GATE_NOT_OPEN` | hyväksyntään on jo vastattu tai se ei ole auki | pyydä agenttia näyttämään ajon tila |
| `IMPORT_PREPARATION_CHANGED` | tiliote, ehdotetut kirjaukset, tekijä tai tilikartta muuttui tuonnin esikatselun jälkeen; mitään ei sinetöity | uusi esikatselu ja uusi hyväksyntä ([Tuo pankin tiliote](../tasks/import-bank-statement.md)) |
| `INVALID_FISCAL_YEAR` | tilikausi on yli 18 kuukautta, ei ala kuukauden ensimmäisenä tai pääty viimeisenä päivänä, tai ei jatka aiempia tilikausia | korjaa päivät (kirjanpitolaki 1:4) |
| `INVALID_INPUT` | syöte on väärän muotoinen tai siinä on tuntematon kenttä | agentti korjaa syötteen |
| `MASTERDATA_CORRUPT` | kodin tilikartta, sen versio tai ALV-roolit puuttuvat tai ovat ristiriidassa keskenään | päivityksen jälkeen: `home upgrade --dry-run` ja `--apply` ([Päivitä aggountant](../tasks/upgrade.md)); muuten tiedostoa on muutettu käsin, palauta se versionhallinnasta |
| `MASTERDATA_NOT_INITIALIZED` | kodista puuttuu tilikartta | väärä koti tai kotia ei ole perustettu ([Kun jokin menee vikaan](../troubleshooting.md#tositteet-näyttävät-kadonneen)) |
| `MISSING_ASSIGNMENT` | tiliotteen jollekin tapahtumalle ei ole kirjausta, joten koko tiliote hylättiin | agentti täydentää kirjaukset ([Tuo pankin tiliote](../tasks/import-bank-statement.md)) |
| `NATURAL_KEY_ALREADY_SEALED` | `voucher validate` -komennon varoitus: sama sisältö on jo sinetöity tällä `natural_key`-avaimella | ei mitään; sinetöinti palauttaa olemassa olevan tositteen eikä kirjoita uutta. Älä vaihda avainta varoituksen ohittamiseksi |
| `OPEN_OBLIGATION` | avoin velvoite estää kauden sulun | hoida asia ja merkitse velvoite ratkaistuksi ([Kaudet](../concepts/periods.md#velvoitteet)) |
| `OUTPUT_EXISTS` | raporttitiedosto on jo olemassa | anna uusi tiedostonimi |
| `PERIOD_CLOSING` | kauden sulku on kesken | hyväksy sulku tai kirjaa seuraavalle kaudelle ([Sulje kuukausi](../tasks/close-month.md)) |
| `PERIOD_NOT_SEALED` | ALV-ilmoitusta yritettiin kaudelle, jota ei ole suljettu, tai raportti tarvitsee edellisen tilikauden loppusaldot, eikä sitä tilikautta ole suljettu | sulje kausi tai edellinen tilikausi ensin; uuden tilikauden kuukausiraportti odottaa edellisen tilikauden tilinpäätösajoa, mutta kirjaaminen, kuukausien sulku ja ALV-ilmoitukset eivät odota ([Valmistele tilinpäätös](../tasks/year-end.md)); jos edellisellä tilikaudella on vain käyttöönoton avaavat saldot, katso niiden päiväys kohdasta [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) |
| `PERIOD_SEALED` | kausi on suljettu, tai päivä osuu suljetulle tai sitä edeltävälle tilikaudelle | kirjaa seuraavalle avoimelle kaudelle |
| `PERIOD_TRANSITION_CONFLICT` | kauden tila ei salli pyydettyä muutosta, esimerkiksi kausi on jo ilmoitettu, toinen sulku on kesken tai samana vuonna yritetään sekoittaa kuukausia ja neljänneksiä tai sulkea neljännestä tilikaudella, joka ei ole kalenterivuosi | lue viesti; ilmoitettua kautta ei ilmoiteta uudelleen, vaan sen korjaus on korvaava ilmoitus ([Korjaa jo annettu ilmoitus](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus)) |
| `RUN_NOT_FOUND` | ajoa ei löydy | tarkista, että komento osui oikeaan kotiin |
| `TRANSCRIPT_UNKNOWN_HEADER` | tuotava tiedosto ei ole suomenkielinen Tulorekisterin asiointipalvelun raportti 630 tunnetuin sarakkein | tilaa raportti 630 uudelleen suomeksi; muuta ei tallennettu ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#kun-aiempi-palkanlaskija-on-antanut-ilmoituksia)) |
| `TULOREKISTERI_NOT_READY` | Tulorekisteri-yhteys ei ole valmis lähettämään: profiili puuttuu, se ei ole tuotantotilassa tai siitä puuttuu vahvistuksesi ilmoitusoikeudesta, tai tunnustiedosto tai varmenne ei ole kunnossa; viesti nimeää jokaisen puuttuvan vaatimuksen | `tulorekisteri status` listaa ne verkotta, kullekin omistaja (`human`: vahvistuksesi, varmenne tai asiointipalvelun vaihe; `agent`: asetus); agentti korjaa omansa komennolla `tulorekisteri configure` ([Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md)) |
| `TULOREKISTERI_REJECTED` | tilakysely vahvisti, että Tulorekisteri hylkäsi ilmoituksen; ajo jää odottamaan huomiota ja `run show` näyttää hylkäyksen | selvitä virhe asiointipalvelusta; hylätty matkakorvausilmoitus annetaan siellä ja kirjataan (`complete-manual`), hylätty korjaus esikatsellaan uudelleen ([Kun jokin menee vikaan](../troubleshooting.md#tulorekisteri-hylkäsi-ilmoituksen)) |
| `VAT_ACCOUNTS_UNRECONCILED` | ALV-tilillä on vientejä ilman ALV-merkintää, ja ilmoituksesta puuttuisi veroa | ennen sulkua korjaava tosite samalle kuukaudelle ([Sulje kuukausi](../tasks/close-month.md#ennen-kuin-aloitat)); sulun jälkeen ilmoitus OmaVerossa korjatuin luvuin; tai perustelu, jos viennit eivät kuulu ilmoitukseen ([Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen)) |
| `VAT_CURRENCY` | ALV-ilmoituksen lukuja ei voi muodostaa; myös silloin, kun kauden ALV on korjausten vuoksi negatiivinen | [Mitä on valmista ja mitä ei](status.md#alv-ilmoitus) |
| `VAT_FIGURES_MISMATCH` | ALV-ilmoituksen luvut eivät vastaa kirjanpitoa | agentti käyttää esikatselun lukuja |
| `VAT_FILING_UNRESOLVED` | saman kauden aiempi ilmoitus on voinut mennä perille, eikä Verohallinto ole vielä vastannut "ei löydy" kahdesti vähintään yhden pankkipäivän välein | mitään ei lähetetty; agentti kysyy komennolla `file-vat reconcile`, tai ensin komennolla `file-vat status`, jos aiempi ajo keskeytyi kesken lähetyksen ([Epävarma lopputulos](../concepts/filings.md#epävarma-lopputulos), [keskeytynyt ilmoitus](../troubleshooting.md#alv-ilmoitus-keskeytyi-lähetettäessä)) |
| `VAT_PERIOD_NOT_FILED` | korvaavaa ilmoitusta tai ilmoitusta korjaavaa tositetta (`corrects_vat_period`) yritettiin kaudelle, jonka ALV-ilmoitusta ei ole annettu | ilmoittamattoman kauden korjaus kirjataan kaudelle itselleen tai kausi avataan uudelleen, ja kausi ilmoitetaan ensimmäisellä ilmoituksella; korvaava ilmoitus koskee vain annettua ilmoitusta ([Korjaa jo annettu ilmoitus](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus)) |
| `VAT_PERIOD_NOT_SEALED` | näkyy vielä sulkemattoman kuukauden ALV-ilmoituksen esikatselussa: luvut ovat tarkistusta varten tehty luonnos, jota ei voi ilmoittaa (`fileable: false`) | tarkista ALV-täsmäytys, korjaa sen löydökset ja sulje kuukausi ([Sulje kuukausi](../tasks/close-month.md)) |
| `VAT_POSTING_UNTAGGED` | `voucher validate` -komennon varoitus: ALV-tilin viennillä ei ole ALV-koodia, joten ALV-ilmoitus jättäisi sen veron pois | merkitse ALV-koodi veron perusteen riville ennen hyväksyntää, ellei vienti ole ALV:n maksu tai muu ilmoitukseen kuulumaton siirto |
| `VAT_RECONCILIATION_ACK_MISMATCH` | kuittauksesta puuttuu perustelu, sen tositenumerot eivät ole samat kuin esikatselussa tai kuitattavaa ei ole | agentti käyttää esikatselun tositenumeroita |
| `VAT_REPLACEMENT_CHANGES_NOTHING` | näkyy epäonnistuneen `file-vat replace` -komennon viestissä (sen `error_code` on `PROCESS_STORE`): Verohallinnolla on jo samat luvut, joten korvattavaa ei ole; mitään ei lähetetty | jos korvaava ilmoitus tehtiin OmaVerossa, kirjaa se (`file-vat complete-manual --replacement`); muuten tarkista, mitä piti korjata |
| `VAT_REPLACEMENT_DUE` | näkyy jo ilmoitetun kuukauden ALV-ilmoituksen esikatselussa: kuukaudelle on sinetöity ilmoitusta korjaava tosite viimeisimmän ilmoituksen jälkeen, joten voimassa oleva ilmoitus ei vastaa kirjanpitoa | anna korvaava ilmoitus ([Korjaa jo annettu ilmoitus](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus)) |
| `VAT_SETTLEMENT_ROLES_MISSING` | näkyy ALV-ilmoituksen esikatselussa koodin `VAT_ACCOUNTS_UNRECONCILED` rinnalla: tositteiden tilikartassa ei ole merkitty maksutilejä, joten ALV:n maksuja ja palautuksia ei tunnisteta | kodin perustietojen päivitys ja uusi esikatselu, tai maksujen kuittaus ([Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen)) |
| `VAT_STATUS_UNKNOWN` | ALV-ilmoituksen tilaa ei saatu kysyttyä Verohallinnolta (yhteys, tunnukset tai valmius) | kuitattu ilmoitus on silti tehty; keskeytyneen ilmoituksen kohdalla mitään ei muutettu. Kysy myöhemmin uudelleen, älä lähetä uudelleen ([Ilmoitus ei näy OmaVerossa](../troubleshooting.md#ilmoitus-ei-näy-omaverossa)) |
| `VERO_NOT_READY` | suoran ilmoittamisen valmius ei ole kunnossa | [Vero.fi-valmiuden esteet](vero-fi-blockers.md) |
| `VERO_TEST_NOT_READY` | `vero-fi verify --target test` kieltäytyi, koska testiympäristön valmius ei ole kunnossa, esimerkiksi varmenne on tuotannon | viesti nimeää esteet ([Vero.fi-valmiuden esteet](vero-fi-blockers.md)); testivarmistus on vapaaehtoinen eikä tuotanto tarvitse sitä |
| `VERO_VAT_PERIOD_NOT_FILEABLE` | näkyy epäonnistuneen `file-vat start` -komennon viestissä (sen `error_code` on `PROCESS_STORE`) ja `run show` -näkymässä ajon `reconcile`-vaiheessa: Verohallinto näyttää ALV-kauden muussa tilassa kuin ilmoittamattomana tai puuttuvana, esimerkiksi jo ilmoitettuna; mitään ei lähetetty | tarkista kausi OmaVerosta; kun syy on korjattu, aloita uusi ajo, älä yritä samaa uudelleen |
| `VERO_VAT_RETURN_NOT_PROCESSED` | näkyy epäonnistuneen `file-vat replace` -komennon viestissä (sen `error_code` on `PROCESS_STORE`): Verohallinto ei ole vielä käsitellyt kauden voimassa olevaa ilmoitusta; mitään ei lähetetty | odota käsittelyä ja aloita korvaava ilmoitus myöhemmin uudelleen |
| `VOUCHER_NOT_FOUND` | tositetta ei ole annetulla numerolla | tarkista numero tai koti |
| `WAGE_CORRECTION_ALREADY_SENT` | sama korjaus on jo lähetetty ja Tulorekisteri hylkäsi sen | korjaa hylkäyksen syy ja esikatsele korjaus uudella aikaleimalla |
| `WAGE_CORRECTION_HASH_MISMATCH` | hyväksytty tarkiste ei vastaa korjausta: ilmoitus, aikaleima, henkilötunnus tai rekisterin versio on muuttunut esikatselun jälkeen | uusi esikatselu ja uusi hyväksyntä |
| `WAGE_CORRECTION_INVALID` | korjausta ei voi muodostaa: korvaavan ilmoituksen sisältö ei kelpaa, se puuttuu, tai mitätöinnille annettiin ilmoitus | agentti korjaa syötteen |
| `WAGE_CORRECTION_INVALID_EVIDENCE` | asiointipalvelun näyttö ei kelpaa korjauksen kirjaamiseen: viite ei ole 32-merkkinen tai on eri ilmoituksen, tai versio ei ole kirjattua uudempi | tarkista ilmoitusviite ja versio asiointipalvelusta |
| `WAGE_CORRECTION_NOT_ACCEPTED` | korjattavaa ilmoitusta ei ole aggountantin tietojen mukaan hyväksytty | selvitä ensin alkuperäinen ilmoitus (`run show`, `file-wage reconcile`) |
| `WAGE_CORRECTION_OUTCOME_UNKNOWN` | aiemman korjauksen lähetyksen tulos on vielä epäselvä | kysy tila (`file-wage correction status`), älä lähetä uudelleen |
| `WAGE_CORRECTION_REPLACEMENT_NOT_ALLOWED` | korvaava ilmoitus muuttaisi maksajaa, tulonsaajaa, maksupäivää tai ansaintajaksoa | mitätöi ilmoitus (`--action cancel`) ja anna uusi ([Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md#korjaa-hyväksytty-ilmoitus)) |
| `WAGE_CORRECTION_REPORT_CANCELLED` | ilmoitus on mitätöity, eikä siinä ole korjattavaa | anna tarvittaessa uusi ilmoitus omalla tunnisteellaan |
| `WAGE_CORRECTION_STALE_TIMESTAMP` | korjauksen aikaleima on yli vuorokauden päässä lähetyshetkestä | uusi esikatselu nykyisellä ajalla ja uusi hyväksyntä |
| `WAGE_CORRECTION_STATUS_UNKNOWN` | Tulorekisteri ei antanut korjaukselle lopullista tilaa | kysy myöhemmin uudelleen tai tarkista asiointipalvelusta; älä lähetä uudelleen |
| `WAGE_CORRECTION_UNCHANGED` | korvaava ilmoitus olisi sama kuin rekisterissä oleva | ei lähetettävää; tarkista, mitä piti korjata |
| `WAGE_REPORT_ALREADY_ACCEPTED` | `file-wage complete-manual` annettiin ilmoitukselle, jonka Tulorekisteri on jo hyväksynyt; mitään ei kirjattu | korjaus tehdään komennolla `file-wage correction` ([Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md#korjaa-hyväksytty-ilmoitus)) |
| `WORKFLOW_VERSION_MISMATCH` | kesken oleva ajo on aloitettu vanhemmalla ohjelmaversiolla | [Päivitä aggountant](../tasks/upgrade.md) |
| `YTJ_UNAVAILABLE` | YTJ:n avoin data ei vastannut rekisteröintikyselyyn | rekisteröinti on tuntematon, ei puuttuva; kokeile myöhemmin tai katso ytj.fi |

Muissa tapauksissa `message` kertoo syyn. Pyydä agenttia selittämään se.

## Katso myös

- [Kun jokin menee vikaan](../troubleshooting.md)
- [Vero.fi-valmiuden esteet](vero-fi-blockers.md)
