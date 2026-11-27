# Virhekoodit

Kun komento epäonnistuu, aggountant antaa virhekoodin, joka näkyy
sellaisenaan agentin tulosteessa. Useimmat virheet ovat tarkoituksellisia
pysäytyksiä: ohjelma kieltäytyy tekemästä jotain, mikä tekisi kirjanpidosta
väärän, ja kirjanpito jää ennalleen. Taulukossa ovat tavallisimmat koodit ja
se, mitä kullekin tehdään.

## Miten virhe näkyy

Epäonnistunut komento tulostaa virheen näin:

```json
{"error_code":"PERIOD_SEALED","error_kind":"user","message":"PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period","hint":"kirjaa seuraavalle avoimelle kaudelle"}
```

- `error_code` on pysyvä koodi, jonka perusteella agentti tietää, mitä tehdä.
- `error_kind` on `user`, kun syy on syötteessä tai kirjanpidon tilassa ja sen
  voi korjata (paluuarvo 1), tai `system`, kun syy on ympäristössä, kuten
  tiedostossa tai yhteydessä (paluuarvo 2).
- `message` kertoo syyn sanallisesti.
- `hint` on alla olevan taulukon "Mitä tehdään" -sarake suomeksi. Se on mukana
  vain koodeilla, jotka taulukossa on.
- `test_seam` on mukana vain, kun
  [testimuuttuja `AGGOUNTANT_BROKER_SCRIPT` vastasi viranomaisen puolesta](../troubleshooting.md#ilmoitus-merkittiin-tehdyksi-vaikka-mitään-ei-lähetetty).

Agentti saa koodin koko selityksen komennolla `aggountant explain KOODI`
(englanniksi `--lang en`). Ohjelma lukee selitykset tältä sivulta, joten ne
ovat samat kuin alla.

Vero.fi-valmiuden esteiden koodit (`VERO_…`) ovat omalla sivullaan:
[Vero.fi-valmiuden esteet](vero-fi-blockers.md).

## Koodit

| Koodi | Mitä tapahtui | Mitä tehdään |
|---|---|---|
| `ACCOUNT_NOT_FOUND` | kirjaus viittaa tiliin, jota tilikartassa ei ole | agentti valitsee tilin tilikartasta |
| `ALREADY_INITIALIZED` | kodin perustaminen ajettiin olemassa olevaan kotiin | ei mitään, koska koti on jo olemassa eikä siihen koskettu |
| `APPROVAL_REQUIRED` | kirjausta yritettiin ilman hyväksyttyä esikatselua | agentti näyttää ensin esikatselun, ja sinä hyväksyt sen |
| `BANK_ENTRY_POSSIBLY_BOOKED` | tiliotteiden tuonnin ulkopuolella kirjattu tosite (esimerkiksi käsin tehty kirjaus) saattaa jo kirjata tiliotteen uuden tapahtuman: sen tunnisteessa tai selitteessä on pankin tapahtumatunniste, tai se kirjaa saman summan samalle puolelle samaa pankkitiliä enintään 7 päivän päässä; mitään ei sinetöity | jos tosite kirjaa tapahtuman, liitä tapahtuma siihen uuden tositteen sijaan; jos ei, merkitse, ettei se kirjaa tapahtumaa; virheen `details` nimeää tapahtuman ja tositteen ([Tuo pankin tiliote](../tasks/import-bank-statement.md#kun-pankkitapahtumia-on-kirjattu-aiemmin-käsin)) |
| `BANK_VOUCHER_MISMATCH` | tiliotteen tapahtuma liitettiin tositteeseen, joka ei kirjaa sitä (eri pankkitili, summa, puoli tai päivä), tosite on jo liitetty toiseen tapahtumaan tai tapahtuma on jo liitetty toiseen tositteeseen; mitään ei sinetöity | tarkista tositteen numero (`voucher show`) ([Tuo pankin tiliote](../tasks/import-bank-statement.md#jos-jokin-menee-pieleen)) |
| `CAPTURE_VAT_CLASS_UNKNOWN` | kuittiehdotus nimeää ALV-verokantaluokan, jota aggountant ei tunne; mitään ei kirjattu | agentti valitsee kuitin ALV-käsittelyä vastaavan luokan; viesti luettelee luokat, joita osto voi käyttää |
| `CAPTURE_PREPARATION_CHANGED` | kuitin tiedot, ehdotus tai tilikartta muuttuivat hyväksynnän jälkeen | agentti tekee uuden esikatselun, ja sinä hyväksyt sen ([Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)) |
| `CLAIMANT_FACT_CONFLICT` | kululaskun tekijästä kirjattava tieto on ristiriidassa samoille päiville jo kirjatun tiedon kanssa, esimerkiksi kaksi ensisijaista varsinaista työpaikkaa; mitään ei tallennettu | korjaa vanha tieto (`claimant correct`) uuden lisäämisen sijaan ([Kirjaa varsinainen työpaikka](../tasks/record-regular-workplace.md#kun-tieto-muuttuu-tai-oli-väärin)) |
| `CLAIMANT_RELATIONSHIP_UNKNOWN` | henkilön suhdetta yhtiöön ei ole kirjattu matkan päivälle, joten verovapaita matkakorvauksia ei lasketa | kerro agentille, onko henkilö esimerkiksi työntekijä, toimitusjohtaja tai hallituksen jäsen ja mistä päivästä ([Kirjaa varsinainen työpaikka](../tasks/record-regular-workplace.md)) |
| `CLOSE_CONTENT_CHANGED` | sulun hyväksyttäväksi näytetty sisältö on muuttunut sen jälkeen, kun hyväksyntä avautui | hylkää sulku ja aloita se uudelleen ([Sulje kuukausi](../tasks/close-month.md)) |
| `CONTRACTOR_NOT_CLAIMANT` | laskuttavaa toimeksisaajaa tai konsulttia yritettiin kirjata kululaskun tekijäksi | toimeksisaajan matkakulut kuuluvat hänen omaan laskuunsa, eivät kululaskulle |
| `EMPLOYER_REGISTER_MISMATCH` | erillisilmoituksen alkukuukausi on eri kuin YTJ:n työnantajarekisterissä; mitään ei tallennettu | tarkista vertailu; käytä rekisterin kuukautta tai vahvista oma kuukautesi agentille ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#ota-erillisilmoitus-käyttöön-kerran)) |
| `EMPLOYER_REPORT_ALREADY_FILED` | kuukaudella on jo Tulorekisterin vastaanottama työnantajan erillisilmoitus | älä anna uutta ilmoitusta; korjaa korvaavalla ilmoituksella ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#jos-jokin-menee-pieleen)) |
| `EMPLOYER_REPORT_FILED_OUTSIDE` | tuodun raportin 630 mukaan kuukaudelle on jo annettu erillisilmoitus aggountantin ulkopuolella | älä anna uutta ilmoitusta; korjaa ilmoitus asiointipalvelussa tai kirjaa ensin sen viite ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#kun-aiempi-palkanlaskija-on-antanut-ilmoituksia)) |
| `EMPLOYER_REPORT_INVALID_EVIDENCE` | `complete-manual`-komennolla annettu asiointipalvelun näyttö ei ratkaise kuukautta: ilmoitusviite ei ole 32-merkkinen, tai epävarman korvaavan ilmoituksen versio puuttuu tai ei ole korvattua uudempi | anna asiointipalvelun näyttämät viitteet ja versio. Jos palvelussa näkyy vanha versio, korvaava ei ole perillä ja kuukausi jää odottamaan. Jos korvattua versiota ei kirjattu, kysy tila komennolla `file-wage employer-report status` |
| `EMPLOYER_REPORT_INVALID_SETTINGS` | erillisilmoituksen asetuksissa on työeläkevakuutus, jonka eläkelaitostunnus tai vakuutusnumero ei kelpaa ilmoitukseen | tarkista eläkelaitoksen tunnus ja vakuutusnumero |
| `EMPLOYER_REPORT_NOT_CONFIGURED` | Tulorekisteri-profiilista puuttuu erillisilmoituksen asetus | ota erillisilmoitus käyttöön ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#ota-erillisilmoitus-käyttöön-kerran)) |
| `EMPLOYER_REPORT_NO_REFERENCE` | korvattava kuukausi tunnetaan vain raportista 630, jossa ei ole ilmoitusviitettä | korvaa ilmoitus asiointipalvelussa tai kirjaa ensin sen viite (`complete-manual`) |
| `EMPLOYER_REPORT_PREVIOUS_STATED_NEEDLESSLY` | korvattavan ilmoituksen eläkevakuutustiedot annettiin, vaikka aggountant tuntee ne jo (se lähetti ilmoituksen) tai mitään ei korvata | jätä pois `--previous-pension-insurance` ja `--previous-no-pension-insurance` |
| `EMPLOYER_REPORT_PREVIOUS_UNKNOWN` | korvattava ilmoitus annettiin aggountantin ulkopuolella ja siitä kirjattiin vain viite, joten sen eläkevakuutustietoja, jotka korvaavan on toistettava, ei tunneta | katso ne ilmoituksesta asiointipalvelussa ja anna ne (`--previous-pension-insurance` tai `--previous-no-pension-insurance`), tai korjaa ilmoitus asiointipalvelussa ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#kun-aiempi-palkanlaskija-on-antanut-ilmoituksia)) |
| `EMPLOYER_REPORT_OUTCOME_UNKNOWN` | kuukauden aiemman lähetyksen tulos on vielä epäselvä | kysy tila (`file-wage employer-report status`), älä lähetä uudelleen ([Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma)) |
| `EMPLOYER_REPORT_PAYMENTS_CONFLICT` | kuukaudelle ilmoitettiin "Ei palkanmaksua", vaikka sille on matkakorvausilmoitus | ilmoita kuukausi vain matkakorvauksia sisältäväksi |
| `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED` | korvaava ilmoitus muuttaisi eläkevakuutustietoja | mitätöi ilmoitus asiointipalvelussa ja anna sinne uusi |
| `EMPLOYER_REPORT_STATUS_UNKNOWN` | Tulorekisteri ei antanut lopullista tilaa | kysy myöhemmin uudelleen tai tarkista asiointipalvelusta; älä lähetä uudelleen |
| `EMPLOYER_REPORT_TRANSCRIPT_CONFLICT` | tuodut raportit 630 ovat eri mieltä kuukaudesta | tarkista kuukausi asiointipalvelusta ja kirjaa ilmoituksen viite (`complete-manual`) |
| `EMPLOYER_REPORT_WAGES_UNSUPPORTED` | kuukaudella maksettiin palkkaa, eikä aggountant laske sairausvakuutusmaksua | anna ilmoitus asiointipalvelussa ([Mitä on valmista ja mitä ei](status.md#tulorekisteri-ja-työntekijät)) |
| `FISCAL_YEAR_CONFLICT` | tilikauden määrittely siirtäisi kirjattuja tositteita tai suljetun tilikauden toiselle tilikaudelle | lue viesti; määrittely ei voi muuttaa jo kirjattua |
| `FISCAL_YEAR_UNDEFINED` | päivä on ennen yrityksen ensimmäistä tilikautta, tai annettuna vuonna ei pääty tilikautta | tarkista päivä tai vuosi; `fiscal-year list` näyttää määrittelyt |
| `GATE_NOT_OPEN` | hyväksyntään on jo vastattu tai se ei ole auki | pyydä agenttia näyttämään ajon tila |
| `HOME_NOT_INITIALIZED` | komennon kotipolussa ei ole `init`-komennolla perustettua kotia; mitään ei kirjoitettu | viesti kertoo polun ja miten se valittiin. Jos koti tai hakemisto on väärä, korjaa polku ([Kun jokin menee vikaan](../troubleshooting.md#tositteet-näyttävät-kadonneen)). Aja `init` vain uudelle yritykselle |
| `IMPORT_PREPARATION_CHANGED` | tiliote, ehdotetut kirjaukset, tekijä tai tilikartta muuttui tuonnin esikatselun jälkeen; mitään ei sinetöity | agentti tekee uuden esikatselun, ja sinä hyväksyt sen ([Tuo pankin tiliote](../tasks/import-bank-statement.md)) |
| `INVALID_CAMT` | tiliotetiedosto ei ole luettava camt.053- tai camt.052-tiliote: XML on rikki, siitä puuttuu tilin IBAN tai tapahtuman tieto, tai siinä on tapahtuma, jota ohjelma ei osaa tuoda (esimerkiksi peruutus tai koottu tapahtuma); mitään ei kirjoitettu | lue viesti ja hae pankista oikea tiedosto ([Tuo pankin tiliote](../tasks/import-bank-statement.md)) |
| `INVALID_FISCAL_YEAR` | tilikausi on yli 18 kuukautta, ei ala kuukauden ensimmäisenä tai pääty viimeisenä päivänä, tai ei jatka aiempia tilikausia | korjaa päivät (kirjanpitolaki 1:4) |
| `INVALID_INPUT` | syöte on väärän muotoinen tai siinä on tuntematon kenttä | agentti korjaa syötteen |
| `MASTERDATA_CORRUPT` | kodin tilikartta, sen versio tai ALV-roolit puuttuvat tai ovat ristiriidassa keskenään | jos ohjelma on juuri päivitetty, aja `home upgrade --dry-run` ja sitten `--apply` ([Päivitä aggountant](../tasks/upgrade.md)); muuten tiedostoa on muutettu käsin, joten palauta se versionhallinnasta |
| `MASTERDATA_NOT_INITIALIZED` | kodista puuttuu tilikartta | tarkista koti: se on väärä tai sitä ei ole perustettu ([Kun jokin menee vikaan](../troubleshooting.md#tositteet-näyttävät-kadonneen)) |
| `MISSING_ASSIGNMENT` | tiliotteen jollekin tapahtumalle ei ole kirjausta, joten koko tiliote hylättiin | agentti täydentää kirjaukset ([Tuo pankin tiliote](../tasks/import-bank-statement.md)) |
| `NATURAL_KEY_ALREADY_SEALED` | `voucher validate` -komennon varoitus: sama sisältö on jo sinetöity tällä `natural_key`-avaimella | ei mitään; sinetöinti palauttaa olemassa olevan tositteen eikä kirjoita uutta. Älä vaihda avainta varoituksen ohittamiseksi |
| `OPEN_OBLIGATION` | avoin velvoite estää kauden sulun | hoida asia ja merkitse velvoite ratkaistuksi ([Kaudet](../concepts/periods.md#velvoitteet)) |
| `OUTPUT_EXISTS` | raporttitiedosto on jo olemassa | anna uusi tiedostonimi |
| `PERIOD_CLOSING` | kauden sulku on kesken | jos tosite kuuluu tälle kaudelle, hylkää sulku ja kirjaa se; muuten kirjaa seuraavalle kaudelle ([Sulje kuukausi](../tasks/close-month.md)) |
| `PERIOD_NOT_SEALED` | ALV-ilmoitusta yritettiin kaudelle, jota ei ole suljettu, tai raportti tarvitsee edellisen tilikauden loppusaldot, eikä sitä tilikautta ole suljettu | sulje ensin kausi tai edellinen tilikausi. Uuden tilikauden kuukausiraportti odottaa edellisen tilikauden tilinpäätösajoa, mutta kirjaaminen, kuukausien sulku ja ALV-ilmoitukset eivät odota ([Valmistele tilinpäätös](../tasks/year-end.md)). Jos edellisellä tilikaudella on vain käyttöönoton avaavat saldot, katso niiden päiväys kohdasta [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) |
| `PERIOD_SEALED` | kausi on suljettu, tai päivä osuu suljetulle tai sitä edeltävälle tilikaudelle | kirjaa seuraavalle avoimelle kaudelle |
| `PERIOD_TRANSITION_CONFLICT` | kauden tila ei salli pyydettyä muutosta: esimerkiksi kausi on jo ilmoitettu, toinen sulku on kesken, samana vuonna yritetään sekoittaa kuukausia ja neljänneksiä, tai neljännestä yritetään sulkea tilikaudella, joka ei ole kalenterivuosi | lue viesti. Ilmoitettua kautta ei ilmoiteta uudelleen, vaan se korjataan korvaavalla ilmoituksella ([Korjaa jo annettu ilmoitus](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus)) |
| `POSTING_PROJECTION_CHANGED` | tiliotteen tapahtuma on jo kirjattu samoilla pankin tiedoilla, mutta kohdistus on nyt toinen (tili, summa, ALV, päivä tai selite); mitään ei sinetöity | anna tapahtumalle sama kohdistus, jolla sen tosite sinetöitiin (`voucher show`); jos aiempi kirjaus oli väärä, se korjataan uudella tositteella ([Tuo pankin tiliote](../tasks/import-bank-statement.md#jos-jokin-menee-pieleen)) |
| `REGULAR_WORKPLACE_UNKNOWN` | henkilön varsinaista työpaikkaa ei ole kirjattu matkan päivälle, joten verovapaita matkakorvauksia ja matkan kuluja ei lasketa | kerro agentille, missä teet työtä vakituisesti ja mistä päivästä ([Kirjaa varsinainen työpaikka](../tasks/record-regular-workplace.md)) |
| `RUN_NOT_FOUND` | ajoa ei löydy | tarkista, että komento osui oikeaan kotiin |
| `SECONDARY_WORKPLACE_INVALID` | paikka ei voi olla toissijainen työpaikka: se ei ole oman yhtiön tai sen konserniyhtiön toimipaikka, se on samassa kunnassa kuin ensisijainen varsinainen työpaikka, tai ensisijaista varsinaista työpaikkaa ei ole kirjattu | kirjaa samassa kunnassa oleva paikka toiseksi varsinaiseksi työpaikaksi, tai kirjaa ensin ensisijainen varsinainen työpaikka |
| `SOURCE_FACTS_CHANGED` | tiliotteen tapahtuma on jo kirjattu, mutta pankin tiedot siitä (summa, päivä, suunta, osapuoli, viesti tai tila) ovat eri kuin aiemmin; mitään ei sinetöity | selvitä pankista tai tiliotteista, mikä muuttui, ennen kuin tapahtumalle kirjataan mitään; virheen `details` nimeää tapahtuman ja sen tositteen ([Tuo pankin tiliote](../tasks/import-bank-statement.md#jos-jokin-menee-pieleen)) |
| `SOURCE_IDENTITY_UNKNOWN` | tiliotteen tapahtuman tunnisteella on jo tosite, mutta sitä ei voi verrata, koska tapahtumaa ei löydy tositteen liitteenä olevasta tiliotteesta; mitään ei sinetöity | tapahtumaa ei kirjata uudelleen, eikä tiliotetta voi tuoda ennen kuin asia on selvitetty; tarkista tosite (`voucher show`) ja sen liite ([Tuo pankin tiliote](../tasks/import-bank-statement.md#jos-jokin-menee-pieleen)) |
| `TRANSCRIPT_UNKNOWN_HEADER` | tuotava tiedosto ei ole suomenkielinen Tulorekisterin asiointipalvelun raportti 630 tunnetuin sarakkein | tilaa raportti 630 uudelleen suomeksi; muuta ei tallennettu ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md#kun-aiempi-palkanlaskija-on-antanut-ilmoituksia)) |
| `TRAVEL_INPUT_INVALID` | matkan tiedot ovat ristiriitaiset tai virheelliset (esimerkiksi paluu ennen lähtöä tai ajo matkan ulkopuolella); mitään ei laskettu | agentti korjaa matkan tiedot; viesti nimeää matkan ja kentän ([Laske matkan verovapaat korvaukset](../tasks/compute-travel-allowances.md)) |
| `TRIP_TO_REGULAR_WORKPLACE` | matkan kohde on henkilön varsinainen työpaikka, joten matkasta ei makseta verovapaita matkakorvauksia | jos paikka ei enää ole varsinainen työpaikka, korjaa rekisterin tieto; muuten matkaa ei korvata verovapaasti |
| `TULOREKISTERI_NOT_READY` | Tulorekisteri-yhteys ei ole valmis lähettämään: profiili puuttuu, se ei ole tuotantotilassa tai siitä puuttuu vahvistuksesi ilmoitusoikeudesta, tai tunnustiedosto tai varmenne ei ole kunnossa; viesti nimeää jokaisen puuttuvan vaatimuksen | `tulorekisteri status` listaa ne verkotta ja nimeää kullekin omistajan (`human`: vahvistuksesi, varmenne tai asiointipalvelun vaihe; `agent`: asetus). Agentti korjaa omansa komennolla `tulorekisteri configure` ([Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md)) |
| `TULOREKISTERI_REJECTED` | tilakysely vahvisti, että Tulorekisteri hylkäsi ilmoituksen; ajo jää odottamaan huomiota ja `run show` näyttää hylkäyksen | selvitä virhe asiointipalvelusta; hylätty matkakorvausilmoitus annetaan siellä ja kirjataan (`complete-manual`), hylätty korjaus esikatsellaan uudelleen ([Kun jokin menee vikaan](../troubleshooting.md#tulorekisteri-hylkäsi-ilmoituksen)) |
| `VAT_ACCOUNTS_UNRECONCILED` | ALV-tilillä on vientejä ilman ALV-merkintää, ja ilmoituksesta puuttuisi veroa | ennen sulkua kirjaa korjaava tosite samalle kuukaudelle ([Sulje kuukausi](../tasks/close-month.md#ennen-kuin-aloitat)); sulun jälkeen anna ilmoitus OmaVerossa korjatuin luvuin; jos viennit eivät kuulu ilmoitukseen, anna perustelu ([Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen)) |
| `VAT_CURRENCY` | ALV-ilmoituksen lukuja ei voi muodostaa; myös silloin, kun kauden ALV on korjausten vuoksi negatiivinen | katso [Mitä on valmista ja mitä ei](status.md#alv-ilmoitus) |
| `VAT_FIGURES_MISMATCH` | ALV-ilmoituksen luvut eivät vastaa kirjanpitoa | agentti käyttää esikatselun lukuja |
| `VAT_FILING_UNRESOLVED` | saman kauden aiempi ilmoitus on voinut mennä perille, eikä Verohallinto ole vielä vastannut "ei löydy" kahdesti vähintään yhden pankkipäivän välein, tai `run resume` kieltäytyi ALV-ilmoitusajosta, jonka lähetys on voinut alkaa; mitään ei lähetetty | agentti kysyy komennolla `file-vat reconcile`, tai ensin komennolla `file-vat status`, jos aiempi ajo keskeytyi kesken lähetyksen ([Epävarma lopputulos](../concepts/filings.md#epävarma-lopputulos), [keskeytynyt ilmoitus](../troubleshooting.md#alv-ilmoitus-keskeytyi-lähetettäessä)) |
| `VAT_INTERRUPTED_BEFORE_FILING` | näkyy `run show` -näkymässä `reconcile`-vaiheessa ALV-ilmoitusajossa, joka pysähtyi ennen kuin mitään lähetettiin ja jota sitten jatkettiin; ajo on tilassa `not_filed` | ajolle ei tarvitse tehdä mitään; jos kautta ei ole vielä ilmoitettu, ilmoita se uudella ajolla ([Ajo keskeytyi](../troubleshooting.md#ajo-keskeytyi)) |
| `VAT_PAYMENT_ROUTE_UNAVAILABLE` | ALV-maksu hyväksyttiin tai yritettiin uudelleen, mutta tällä reitillä aggountantilla ei ole maksuyhteyttä; mitään ei kirjattu | maksa vero itse OmaVeron viitteellä; agentti päättää ajon komennolla `file-vat user-pays` ([ALV-ilmoitusajo jää odottamaan maksua](../troubleshooting.md#alv-ilmoitusajo-jää-odottamaan-maksua)) |
| `VAT_PERIOD_NOT_FILED` | korvaavaa ilmoitusta tai ilmoitusta korjaavaa tositetta (`corrects_vat_period`) yritettiin kaudelle, jonka ALV-ilmoitusta ei ole annettu | kirjaa korjaus kaudelle itselleen tai avaa kausi uudelleen, ja ilmoita kausi ensimmäisellä ilmoituksella. Korvaava ilmoitus koskee vain annettua ilmoitusta ([Korjaa jo annettu ilmoitus](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus)) |
| `VAT_PERIOD_NOT_SEALED` | näkyy vielä sulkemattoman kuukauden ALV-ilmoituksen esikatselussa: luvut ovat tarkistusta varten tehty luonnos, jota ei voi ilmoittaa (`fileable: false`) | tarkista ALV-täsmäytys, korjaa sen löydökset ja sulje kuukausi ([Sulje kuukausi](../tasks/close-month.md)) |
| `VAT_POSTING_UNTAGGED` | `voucher validate` -komennon varoitus: ALV-tilin viennillä ei ole ALV-koodia, joten ALV-ilmoitus jättäisi sen veron pois | merkitse ALV-koodi veron perusteen riville ennen hyväksyntää, ellei vienti ole ALV:n maksu tai muu ilmoitukseen kuulumaton siirto |
| `VAT_RECONCILIATION_ACK_MISMATCH` | kuittauksesta puuttuu perustelu, sen tositenumerot eivät ole samat kuin esikatselussa tai kuitattavaa ei ole | agentti käyttää esikatselun tositenumeroita |
| `VAT_REPLACEMENT_CHANGES_NOTHING` | näkyy epäonnistuneen `file-vat replace` -komennon viestissä (sen `error_code` on `PROCESS_STORE`): Verohallinnolla on jo samat luvut, joten korvattavaa ei ole; mitään ei lähetetty | jos korvaava ilmoitus tehtiin OmaVerossa, kirjaa se (`file-vat complete-manual --replacement`); muuten tarkista, mitä piti korjata |
| `VAT_REPLACEMENT_DUE` | näkyy jo ilmoitetun kuukauden ALV-ilmoituksen esikatselussa: kuukaudelle on sinetöity ilmoitusta korjaava tosite viimeisimmän ilmoituksen jälkeen, joten voimassa oleva ilmoitus ei vastaa kirjanpitoa | anna korvaava ilmoitus ([Korjaa jo annettu ilmoitus](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus)) |
| `VAT_SETTLEMENT_ROLES_MISSING` | näkyy ALV-ilmoituksen esikatselussa koodin `VAT_ACCOUNTS_UNRECONCILED` rinnalla: tositteiden tilikartassa ei ole merkitty maksutilejä, joten ALV:n maksuja ja palautuksia ei tunnisteta | päivitä kodin perustiedot ja esikatsele uudelleen, tai kuittaa maksut ([Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen)) |
| `VAT_STATUS_UNKNOWN` | ALV-ilmoituksen tilaa ei saatu kysyttyä Verohallinnolta (yhteys, tunnukset tai valmius) | kuitattu ilmoitus on silti tehty; keskeytyneen ilmoituksen kohdalla mitään ei muutettu. Kysy myöhemmin uudelleen, älä lähetä uudelleen ([Ilmoitus ei näy OmaVerossa](../troubleshooting.md#ilmoitus-ei-näy-omaverossa)) |
| `VAT_TREATMENT_CLASS_MISMATCH` | ALV-käsittely ja verokantaluokka eivät kuulu yhteen, esimerkiksi `domestic-input` luokalla `eu-goods`, `eu-services` tai `zero-rated`. Ne ovat nollaverokannan myynnin luokkia, ja osto kirjautuisi ilman ALV:tä; mitään ei kirjattu | viesti kertoo sopivan käsittelyn. EU-osto on `reverse-charge`, jonka `scope` on `eu-goods` tai `eu-services` ja luokka se kotimainen luokka, jonka kantaa ostoon Suomessa sovelletaan (esim. `domestic-standard`). Osto, jossa ei ole Suomen ALV:tä, on `no-vat`; niin kirjataan myös toisesta EU-maasta ostettu palvelu, jonka myynti on arvonlisäverolain mukaan veroton (esim. vakuutus- tai rahoituspalvelu). Veroton tavaroiden yhteisöhankinta on `{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}` ilman `deductibility`-jäsentä ([Miten ALV-käsittely päätetään](../concepts/vat-treatment.md#ostot-toisesta-eu-maasta)) |
| `VERO_NOT_READY` | suoran ilmoittamisen valmius ei ole kunnossa | katso [Vero.fi-valmiuden esteet](vero-fi-blockers.md) |
| `VERO_TEST_NOT_READY` | `vero-fi verify --target test` kieltäytyi, koska testiympäristön valmius ei ole kunnossa, esimerkiksi varmenne on tuotannon | viesti nimeää esteet ([Vero.fi-valmiuden esteet](vero-fi-blockers.md)); testivarmistus on vapaaehtoinen eikä tuotanto tarvitse sitä |
| `VERO_VAT_PERIOD_NOT_FILEABLE` | näkyy epäonnistuneen `file-vat start` -komennon viestissä (sen `error_code` on `PROCESS_STORE`) ja `run show` -näkymässä ajon `reconcile`-vaiheessa: Verohallinto näyttää ALV-kauden muussa tilassa kuin ilmoittamattomana tai puuttuvana, esimerkiksi jo ilmoitettuna; mitään ei lähetetty | tarkista kausi OmaVerosta; kun syy on korjattu, aloita uusi ajo, älä yritä samaa uudelleen |
| `VERO_VAT_RETURN_NOT_PROCESSED` | näkyy epäonnistuneen `file-vat replace` -komennon viestissä (sen `error_code` on `PROCESS_STORE`): Verohallinto ei ole vielä käsitellyt kauden voimassa olevaa ilmoitusta; mitään ei lähetetty | odota käsittelyä ja aloita korvaava ilmoitus myöhemmin uudelleen |
| `VOUCHER_NOT_FOUND` | tositetta ei ole annetulla numerolla | tarkista numero tai koti |
| `WAGE_CORRECTION_ALREADY_SENT` | sama korjaus on jo lähetetty ja Tulorekisteri hylkäsi sen | korjaa hylkäyksen syy ja esikatsele korjaus uudella aikaleimalla |
| `WAGE_CORRECTION_HASH_MISMATCH` | hyväksytty tarkiste ei vastaa korjausta: ilmoitus, aikaleima, henkilötunnus tai rekisterin versio on muuttunut esikatselun jälkeen | agentti tekee uuden esikatselun, ja sinä hyväksyt sen |
| `WAGE_CORRECTION_INVALID` | korjausta ei voi muodostaa: korvaavan ilmoituksen sisältö ei kelpaa, se puuttuu, tai mitätöinnille annettiin ilmoitus | agentti korjaa syötteen |
| `WAGE_CORRECTION_INVALID_EVIDENCE` | asiointipalvelun näyttö ei kelpaa korjauksen kirjaamiseen: viite ei ole 32-merkkinen tai on eri ilmoituksen, tai versio ei ole kirjattua uudempi | tarkista ilmoitusviite ja versio asiointipalvelusta |
| `WAGE_CORRECTION_NOT_ACCEPTED` | korjattavaa ilmoitusta ei ole aggountantin tietojen mukaan hyväksytty | selvitä ensin alkuperäinen ilmoitus (`run show`, `file-wage reconcile`) |
| `WAGE_CORRECTION_OUTCOME_UNKNOWN` | aiemman korjauksen lähetyksen tulos on vielä epäselvä | kysy tila (`file-wage correction status`), älä lähetä uudelleen |
| `WAGE_CORRECTION_REPLACEMENT_NOT_ALLOWED` | korvaava ilmoitus muuttaisi maksajaa, tulonsaajaa, maksupäivää tai ansaintajaksoa | mitätöi ilmoitus (`--action cancel`) ja anna uusi ([Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md#korjaa-hyväksytty-ilmoitus)) |
| `WAGE_CORRECTION_REPORT_CANCELLED` | ilmoitus on mitätöity, eikä siinä ole korjattavaa | anna tarvittaessa uusi ilmoitus omalla tunnisteellaan |
| `WAGE_CORRECTION_STALE_TIMESTAMP` | korjauksen aikaleima on yli vuorokauden päässä lähetyshetkestä | agentti tekee uuden esikatselun nykyisellä ajalla, ja sinä hyväksyt sen |
| `WAGE_CORRECTION_STATUS_UNKNOWN` | Tulorekisteri ei antanut korjaukselle lopullista tilaa | kysy myöhemmin uudelleen tai tarkista asiointipalvelusta; älä lähetä uudelleen |
| `WAGE_CORRECTION_UNCHANGED` | korvaava ilmoitus olisi sama kuin rekisterissä oleva | ei lähetettävää; tarkista, mitä piti korjata |
| `WAGE_REPORT_ALREADY_ACCEPTED` | `file-wage complete-manual` annettiin ilmoitukselle, jonka Tulorekisteri on jo hyväksynyt; mitään ei kirjattu | korjaus tehdään komennolla `file-wage correction` ([Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md#korjaa-hyväksytty-ilmoitus)) |
| `WORKFLOW_VERSION_MISMATCH` | kesken oleva ajo on aloitettu vanhemmalla ohjelmaversiolla | katso [Päivitä aggountant](../tasks/upgrade.md) |
| `WORKPLACE_BASIS_MISMATCH` | työpaikan peruste ei sovi rooliin tai paikkaan, tai perusteelta `other` puuttuu perustelu | agentti valitsee perusteen, joka sopii (`aggountant claimant add --help`), tai kirjoittaa perustelun |
| `YTJ_UNAVAILABLE` | YTJ:n avoin data ei vastannut rekisteröintikyselyyn | rekisteröinti on tuntematon, ei puuttuva; kokeile myöhemmin tai katso ytj.fi |

Jos koodia ei ole taulukossa, `message` kertoo syyn. Pyydä agenttia selittämään se.

## Katso myös

- [Kun jokin menee vikaan](../troubleshooting.md)
- [Vero.fi-valmiuden esteet](vero-fi-blockers.md)
