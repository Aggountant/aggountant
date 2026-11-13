# Virhekoodit

Tältä sivulta löydät tavallisimmat virhekoodit, joihin törmäät, ja mitä niille
tehdään. Koodit ovat aggountantin omia, ja ne näkyvät sellaisinaan agentin
tulosteessa.

## Miten virhe näkyy

Kun komento epäonnistuu, se tulostaa virheen koodeineen, esimerkiksi:

```json
{"error_code":"PERIOD_SEALED","error_kind":"user","message":"PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period"}
```

- `error_code` on pysyvä koodi, jonka perusteella agentti tietää, mitä tehdä.
- `error_kind` on `user`, kun syy on syötteessä tai kirjanpidon tilassa ja sen
  voi korjata (paluuarvo 1), tai `system`, kun syy on ympäristössä, kuten
  tiedostossa tai yhteydessä (paluuarvo 2).
- `message` kertoo syyn sanallisesti.

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
| `CAPTURE_PREPARATION_CHANGED` | kuitin tiedot, ehdotus tai tilikartta muuttuivat hyväksynnän jälkeen | uusi esikatselu ja uusi hyväksyntä ([Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md)) |
| `FISCAL_YEAR_CONFLICT` | tilikauden määrittely siirtäisi kirjattuja tositteita tai suljetun tilikauden toiselle tilikaudelle | lue viesti; määrittely ei voi muuttaa jo kirjattua |
| `FISCAL_YEAR_UNDEFINED` | päivä on ennen yrityksen ensimmäistä tilikautta, tai annettuna vuonna ei pääty tilikautta | tarkista päivä tai vuosi; `fiscal-year list` näyttää määrittelyt |
| `GATE_NOT_OPEN` | hyväksyntään on jo vastattu tai se ei ole auki | pyydä agenttia näyttämään ajon tila |
| `INVALID_FISCAL_YEAR` | tilikausi on yli 18 kuukautta, ei ala kuukauden ensimmäisenä tai pääty viimeisenä päivänä, tai ei jatka aiempia tilikausia | korjaa päivät (kirjanpitolaki 1:4) |
| `INVALID_INPUT` | syöte on väärän muotoinen tai siinä on tuntematon kenttä | agentti korjaa syötteen |
| `MASTERDATA_CORRUPT` | kodin tilikartta, sen versio tai ALV-roolit puuttuvat tai ovat ristiriidassa keskenään | päivityksen jälkeen: `home upgrade --dry-run` ja `--apply` ([Päivitä aggountant](../tasks/upgrade.md)); muuten tiedostoa on muutettu käsin, palauta se versionhallinnasta |
| `MASTERDATA_NOT_INITIALIZED` | kodista puuttuu tilikartta | väärä koti tai kotia ei ole perustettu ([Kun jokin menee vikaan](../troubleshooting.md#tositteet-näyttävät-kadonneen)) |
| `MISSING_ASSIGNMENT` | tiliotteen jollekin tapahtumalle ei ole kirjausta, joten koko tiliote hylättiin | agentti täydentää kirjaukset ([Tuo pankin tiliote](../tasks/import-bank-statement.md)) |
| `OPEN_OBLIGATION` | avoin velvoite estää kauden sulun | hoida asia ja merkitse velvoite ratkaistuksi ([Kaudet](../concepts/periods.md#velvoitteet)) |
| `OUTPUT_EXISTS` | raporttitiedosto on jo olemassa | anna uusi tiedostonimi |
| `PERIOD_CLOSING` | kauden sulku on kesken | hyväksy sulku tai kirjaa seuraavalle kaudelle ([Sulje kuukausi](../tasks/close-month.md)) |
| `PERIOD_NOT_SEALED` | ALV-ilmoitusta yritettiin kaudelle, jota ei ole suljettu, tai raportti tarvitsee edellisen tilikauden loppusaldot, eikä sitä tilikautta ole suljettu | sulje kausi tai edellinen tilikausi ensin; jos edellisellä tilikaudella on vain käyttöönoton avaavat saldot, katso niiden päiväys kohdasta [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) |
| `PERIOD_SEALED` | kausi on suljettu, tai päivä osuu suljetulle tai sitä edeltävälle tilikaudelle | kirjaa seuraavalle avoimelle kaudelle |
| `PERIOD_TRANSITION_CONFLICT` | kauden tila ei salli pyydettyä muutosta, esimerkiksi kausi on jo ilmoitettu, sulku hylättiin tai samana vuonna yritetään sekoittaa kuukausia ja neljänneksiä tai sulkea neljännestä tilikaudella, joka ei ole kalenterivuosi | lue viesti; ilmoitettua kautta ei ilmoiteta uudelleen ([Kun jokin menee vikaan](../troubleshooting.md)) |
| `RUN_NOT_FOUND` | ajoa ei löydy | tarkista, että komento osui oikeaan kotiin |
| `VAT_ACCOUNTS_UNRECONCILED` | ALV-tilillä on vientejä ilman ALV-merkintää, ja ilmoituksesta puuttuisi veroa | ilmoitus OmaVerossa korjatuin luvuin, tai perustelu, jos viennit eivät kuulu ilmoitukseen ([Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen)) |
| `VAT_CURRENCY` | ALV-ilmoituksen lukuja ei voi muodostaa; myös silloin, kun kauden ALV on korjausten vuoksi negatiivinen | [Mitä on valmista ja mitä ei](status.md#alv-ilmoitus) |
| `VAT_FIGURES_MISMATCH` | ALV-ilmoituksen luvut eivät vastaa kirjanpitoa | agentti käyttää esikatselun lukuja |
| `VAT_RECONCILIATION_ACK_MISMATCH` | kuittauksesta puuttuu perustelu, sen tositenumerot eivät ole samat kuin esikatselussa tai kuitattavaa ei ole | agentti käyttää esikatselun tositenumeroita |
| `VAT_STATUS_UNKNOWN` | ALV-ilmoituksen tilaa ei saatu kysyttyä Verohallinnolta (yhteys, tunnukset tai valmius) | ilmoitus on silti tehty, kuittaus todistaa sen; kysy myöhemmin uudelleen, älä lähetä uudelleen ([Ilmoitus ei näy OmaVerossa](../troubleshooting.md#ilmoitus-ei-näy-omaverossa)) |
| `VERO_NOT_READY` | suoran ilmoittamisen valmius ei ole kunnossa | [Vero.fi-valmiuden esteet](vero-fi-blockers.md) |
| `VOUCHER_NOT_FOUND` | tositetta ei ole annetulla numerolla | tarkista numero tai koti |
| `WORKFLOW_VERSION_MISMATCH` | kesken oleva ajo on aloitettu vanhemmalla ohjelmaversiolla | [Päivitä aggountant](../tasks/upgrade.md) |

Muissa tapauksissa `message` kertoo syyn. Pyydä agenttia selittämään se.

## Katso myös

- [Kun jokin menee vikaan](../troubleshooting.md)
- [Vero.fi-valmiuden esteet](vero-fi-blockers.md)
