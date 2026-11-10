# 8. Virhetilanteet ja palautuminen

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

Useimmat virheet ovat tarkoituksellisia pysäytyksiä: ohjelma kieltäytyy tekemästä
jotain, mikä tekisi kirjanpidosta väärän. Kieltäytyminen jättää kirjanpidon
ennalleen.

## Tavallisimmat virheet

| Koodi | Mitä tapahtui | Mitä tehdään |
|---|---|---|
| `MASTERDATA_NOT_INITIALIZED` | kodista puuttuu tilikartta | väärä koti tai `init` tekemättä; tarkista `aggountant config show` |
| `ALREADY_INITIALIZED` | `init` ajettiin olemassa olevaan kotiin | ei mitään; koti on jo olemassa |
| `INVALID_INPUT` | syöte on väärän muotoinen tai siinä on tuntematon kenttä | agentti korjaa syötteen |
| `PERIOD_SEALED` | kausi on suljettu | kirjaa seuraavalle avoimelle kaudelle |
| `PERIOD_CLOSING` | kauden sulku on kesken | hyväksy sulku tai kirjaa seuraavalle kaudelle (luku 4) |
| `PERIOD_NOT_SEALED` | ALV-esikatselu avoimesta kaudesta | sulje kausi ensin |
| `PERIOD_TRANSITION_CONFLICT` | kauden tila ei salli pyydettyä muutosta, esimerkiksi kausi on jo ilmoitettu tai samana vuonna yritetään sekoittaa kuukausia ja neljänneksiä | lue viesti; ilmoitettua kautta ei ilmoiteta uudelleen |
| `OPEN_OBLIGATION` | avoin velvoite estää kauden sulun | hoida asia ja merkitse velvoite ratkaistuksi |
| `CAPTURE_PREPARATION_CHANGED` | kuitin tiedot muuttuivat hyväksynnän jälkeen | uusi esikatselu ja uusi hyväksyntä |
| `APPROVAL_REQUIRED` | kirjausta yritettiin ilman hyväksyttyä esikatselua | esikatselu ensin |
| `OUTPUT_EXISTS` | raporttitiedosto on jo olemassa | anna uusi tiedostonimi |
| `VAT_FIGURES_MISMATCH` | ALV-ilmoituksen luvut eivät vastaa kirjanpitoa | käytä esikatselun lukuja |
| `VERO_NOT_READY` | Vero.fi-valmius ei ole kunnossa | `aggountant vero-fi status` (luku 5) |
| `GATE_NOT_OPEN` | hyväksyntään on jo vastattu | katso ajon tila `run show` -komennolla |
| `WORKFLOW_VERSION_MISMATCH` | kesken oleva ajo on aloitettu vanhemmalla ohjelmaversiolla | katso alta kohta "Ohjelman päivitys" |

## Keskeytynyt ajo

Sulku-, ilmoitus- ja tilinpäätösajot tallentavat jokaisen vaiheensa. Jos kone
kaatuu tai yhteys katkeaa kesken, mitään ei menetetä. Ajoa jatketaan:

| Ajo | Jatkaminen |
|---|---|
| kauden sulku | `aggountant run resume --run-id cp-…` |
| ALV-ilmoitus, epävarma lähetys | `aggountant file-vat reconcile --run-id fv-…` (luku 5) |
| Tulorekisteri-ilmoitus, epävarma lähetys | `aggountant file-wage reconcile wr-…` (luku 6) |
| tilinpäätös | `aggountant close-year resume --run-id cy-…` |

Jatkaminen tarkistaa ensin kirjanpidon todellisen tilan. Jos esimerkiksi kauden
sinetöinti ehti tapahtua ennen kaatumista, ajo huomaa sen eikä tee mitään
toiseen kertaan. Viranomaisilmoitusta ei koskaan lähetetä uudelleen
automaattisesti.

Kaikki ajot näet komennolla `aggountant run list`, ja yksittäisen ajon vaiheet ja
odottavat hyväksynnät komennolla `aggountant run show --run-id …`.

## Väärin sinetöity tosite

Sinetöityä tositetta ei muokata eikä poisteta. Virhe korjataan uudella
korjaustositteella avoimelle kaudelle:

- **väärä tili tai ALV-käsittely**: tosite, joka siirtää summan oikealle tilille
- **kahdesti kirjattu tai yritykselle kuulumaton tapahtuma**: vastakirjaus, joka
  kumoaa alkuperäisen
- **väärä summa**: erotuksen kirjaus.

Agentti valmistelee korjaustositteen erikoiskirjauksena, viittaa kuvauksessa
korjattavaan tositteeseen ja näyttää sen sinulle hyväksyttäväksi. Omaa
korjauskomentoa ohjelmassa ei vielä ole. Alkuperäinen ja korjaus jäävät
molemmat kirjanpitoon. Kirjanpitolaki edellyttää juuri tätä.

## Ketjun tarkistus epäonnistuu

`aggountant voucher verify` tarkistaa, ettei yhtään sinetöityä tositetta ole
muutettu tai poistettu. Jos tarkistus epäonnistuu, tositetietokantaa on
käsitelty muuten kuin aggountantilla tai tiedosto on vaurioitunut. Älä yritä
korjata tietokantaa käsin. Palauta koti varmuuskopiosta (luku 2) ja aja
tarkistus uudelleen.

## Tositteet näyttävät kadonneen

Tarkista ensin, minkä kodin ohjelma löysi: `aggountant config show`. Väärin
kirjoitettu `--home`-polku tai väärästä hakemistosta ajettu komento voi osoittaa
tyhjään tai juuri syntyneeseen kotiin. Oikea kirjanpito on silloin tallessa
toisessa paikassa.

## Kausi jäi suljettavaksi

Jos kauden sulku on hylätty (`--answer reject`), kausi jää tilaan `closing`, eikä
sitä saa tässä versiossa takaisin avoimeksi eikä suljettua uudelleen.
Kaudelle ei myöskään voi kirjata. Tämä on tunnettu puute. Siksi luvun 4 ohje on:
aloita sulku vasta, kun kausi on valmis, ja korjaa jälkikäteen huomatut asiat
seuraavalla kaudella.

## Ohjelman päivitys

Kun päivität aggountantin:

1. Asenna skillit uudelleen (`aggountant skill install`), koska ohjeet on
   kirjoitettu saman version komentoja varten.
2. Jos jokin ajo oli kesken ja jatkaminen antaa `WORKFLOW_VERSION_MISMATCH`, ajon
   määritys on muuttunut versioiden välillä. Agentti voi siirtää ajon uuteen
   versioon komennolla `aggountant run migrate --run-id … --confirm`. Valitsin
   `--confirm` vaaditaan, koska siirto muuttaa ajon versiota pysyvästi.
3. Jos uusi versio odottaa kodista tietoa, jota vanha koti ei sisällä, erillistä
   päivityskomentoa ei vielä ole. Älä kopioi tiedostoja käsin, vaan pyydä apua
   projektin issue-seurannasta.

## Testiympäristön muuttuja

Ympäristömuuttuja `AGGOUNTANT_BROKER_SCRIPT` on tarkoitettu vain ohjelman
testaukseen. Se korvaa Verohallinnon vastaukset etukäteen kirjoitetuilla
vastauksilla. **Jos se on asetettuna, `file-vat` voi merkitä kauden
ilmoitetuksi, vaikka mitään ei lähetetty.** Varmista, ettei muuttuja ole voimassa,
kun teet oikeaa ilmoitusta.
