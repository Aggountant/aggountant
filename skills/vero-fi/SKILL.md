---
name: vero-fi
description: >-
  Ota aggountantin suora Vero.fi-ALV-ilmoittaminen käyttöön, tarkista valmius,
  uusi varmenne, lähetä ALV-ilmoitus hyväksynnän kautta, selvitä epävarma tai
  näkymätön lähetys tai kirjaa manuaalinen ilmoitus (OmaVero, Ilmoitin.fi).
cli_version: "0.1.0"
schema_version: 1
---

# Vero.fi-asiointi

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi.

Aggountant lähettää yrityksen oman ALV-ilmoituksen Vero API:n kautta. Se ei asioi toisen
organisaation puolesta (valtuutustoken credential-nipussa on valmiusesteenä) eikä maksa
veroa. Hyvä lopputulos on, että ilmoitus lähtee vain käyttäjän hyväksymänä, täsmälleen
ohjelman näyttämillä luvuilla, ja että sen tila on jälkikäteen jäljitettävä: mitä
lähetettiin, mitä Vero kuittasi ja mikä on vielä vahvistamatta. Ilmoitus on
oikeusvaikutteinen toimi, jota ei voi peruuttaa. Korvaava ilmoitus vaatii kaikki arvot,
perustelun ja uuden hyväksynnän, eikä aggountant tuota sitä.

Komentojen liput näet `aggountant vero-fi --help`- ja `aggountant file-vat --help`
-ohjeista sekä alikomentojen omista ohjeista. Stdin-JSONia ohjeet eivät kuvaa, joten
molemmat muodot ovat tässä.

## Mikä ratkaisee valmiuden

`aggountant vero-fi status` on ainoa valmiusarvio, ja sama arvio ajetaan uudestaan
juuri ennen jokaista tuotantokutsua. Se on paikallinen eikä ota verkkoyhteyttä, joten
sitä voi ajaa vapaasti. Tulos kertoo `ready`, `mode`, esteet koodilla ja omistajalla
(`human` = käyttäjän antama tieto tai hänen portaali- tai varmennetoimensa, `agent` =
konfiguraatio tai näyttö, jonka voit itse korjata), varoitukset, nykyisen varmenteen
sormenjäljen ja voimassaolon sekä profiilin ja credential-nipun polut. Kerro esteet
käyttäjälle sellaisenaan omistajan mukaan; oma rinnakkainen arvio olisi vain vähemmän
luotettava kopio.

Tuotantovalmius (`ready: true`) tarkoittaa, että profiilissa on Y-tunnus ja
yhteyshenkilö, kaikki neljä ihmisen attestointia ovat tosia, credential-nippu on
luettavissa kirjanpitokodin ulkopuolelta ja sen Y-tunnus ja target täsmäävät,
tuotantovarmenne on voimassa, ja sekä portaaliverifioinnin näyttö että aktivointi on
sidottu juuri tämän varmenteen sormenjälkeen. Testiympäristön tarkistus ei ole ehto.

## Salaisuudet ja niiden sijainti

Credential-nippu on JSON-tiedosto, jossa on ei-salaiset `target` (`test` tai
`production`) ja `business_id` sekä salaiset `software_key`, `client_certificate_pem`,
`client_private_key_pem` ja `ca_certificate_pem`. Käyttäjä laatii ja tallentaa sen itse,
esimerkiksi `~/.config/aggountant/credentials/`-hakemistoon. Aggountant saa vain polun,
joka tallennetaan alustakohtaiseen `config.json`-tiedostoon (ympäristömuuttuja
`AGGOUNTANT_VERO_CREDENTIALS_FILE` ohittaa sen yhdellä koneella).

Salaisuuksia ei pyydetä keskusteluun, eikä nippu saa olla kirjanpitokodin alla (status
estää sen). Kirjanpitokoti on git-versioitu selkotekstiä, ja keskusteluloki päätyy
usein talteen. Ohjelmistoavain on käyttäjän oma: hän on rekisteröinyt ohjelmistonsa
Verohallinnolle ja saanut avaimen itse, eikä aggountantilla ole yhteistä avainta. Kun
tunnistat varmenteen jollekulle, esimerkiksi Verohallinnon tuelle, statusin näyttämä
SHA-256-sormenjälki riittää ja on turvallinen.

## Profiili ja `configure`

Ei-salaiset tiedot ovat kirjanpitokodin `vero-filing.json`-profiilissa, jonka
`aggountant vero-fi configure` kirjoittaa stdin-JSONista:

```json
{
  "schema_version": 1,
  "mode": "disabled",
  "business_id": "1234567-8",
  "contact_name": "Etunimi Sukunimi",
  "contact_phone": "+358401234567",
  "attestations": {
    "software_registered": false,
    "api_agreement_active": false,
    "production_verification_completed": false,
    "production_permission_active": false,
    "activation_certificate_fingerprint_sha256": null,
    "portal_verification": null
  },
  "credential_path": "/home/user/.config/aggountant/credentials/vero-production.json"
}
```

`mode` on `disabled` tai `production`; tuntemattomat kentät hylätään. `credential_path`
on vapaaehtoinen ja päivittää vain alustakonfiguraation. Komento korvaa profiilin
kokonaan, ei yhdistä: jos jätät `attestations.portal_verification`-lohkon pois, se
`verify-portal`-ajon tallentama näyttö katoaa ja tuotanto estyy
(`VERO_PORTAL_VERIFICATION_MISSING`). Lue siis nykyinen profiili (polku on
statusin `profile_path`) ja kanna sen näyttölohkot mukana muuttumattomina. Status ei
tulosta attestointeja, joten profiilitiedosto on niiden ainoa lähde.

`--dry-run` arvioi valmiuden ja kertoo mitä kirjoitettaisiin kirjoittamatta mitään.
Tuotantomääritys kirjoittuu vain, jos arvio on `ready`; muuten komento kieltäytyy
(`VERO_NOT_READY`) ja tila jää entiselleen. Sitä ei tarvitse itse varmistella.

Neljä attestointia ovat käyttäjän lausumia asioista, joita ohjelma ei voi tarkistaa:
ohjelmisto on rekisteröity Vero API -portaaliin, yrityksen rajapintasopimus on
voimassa, tuotantovarmenteen verifiointi on hyväksytty ja tuotantolupa on aktiivinen.
Ohjelma luottaa niihin, joten `true` kirjataan vasta käyttäjän vahvistettua kyseisen
toimen tehdyksi, ei odotuksena. Poikkeus on `production_verification_completed`: onnistunut
`verify-portal` kirjaa sen profiiliin itse. Kaikki neljä ovat käyttäjän omia toimia:
hän rekisteröi ohjelmistonsa Vero API -portaalissa ja saa oman ohjelmistoavaimensa,
ja yritys tekee sopimuksen, hankkii varmenteen ja aktivoi luvan.

## Käyttöönotto

Portaalitoimet ovat käyttäjän: sopimus ja käyttöehdot ovat yrityksen oikeustoimia, ja
selainautomaatio on ADR-9:ssä hylätty kanavana. Sinä kokoat ei-salaiset tiedot,
tarkistat valmiuden, ajat verifioinnit ja kerrot mikä este on kenenkin vuoro.

Järjestyksellä on kaksi syytä. Ensin tarvitaan tuotantonippu paikalleen ja
`disabled`-profiili, jossa on Y-tunnus ja yhteyshenkilö, koska `verify-portal` lukee
molemmat. `aggountant vero-fi verify-portal` ajaa adapterilla Veron kolme kiinteää
Verification-skenaariota (`GetVATPeriods/v1`, `FileVATReturn/v2`,
`GetFiledVATReturn/v2`) staattisella datalla tuotantovarmenteella. Se ei tee
ilmoitusta eikä muuta tilaa; se tallentaa profiiliin aikaleiman, operaatiot,
sormenjäljen ja verifiointikuitin. Portaalissa käyttäjän on ensin valittava ALV:n
hyväksyntätestit. Vasta sitten tuotantomääritys voi onnistua, koska aktivointi
(`activation_certificate_fingerprint_sha256`) ja portaalinäyttö sidotaan samaan
sormenjälkeen, jonka status näyttää. Pyydä käyttäjältä nimenomainen päätös ennen
`verify-portal`-ajoa (se on ensimmäinen kutsu Veron tuotantoon) ja ennen tuotantoon
siirtymistä.

`aggountant vero-fi verify --target test` on vapaaehtoinen kehitys- tai
vikadiagnostiikkaluku testiympäristöstä. Se edellyttää, että credential-polussa on
juuri silloin `test`-targetin nippu; muuten komento kieltäytyy (`VERO_TEST_NOT_READY`,
esteenä `VERO_CREDENTIAL_TARGET_MISMATCH`). Sen näyttö ei ole tuotannon ehto eikä
korvaa mitään.

Varmenne on voimassa kaksi vuotta, ja status varoittaa 90, 30 ja 7 päivää ennen
umpeutumista. Uusi varmenne on uusi sormenjälki, joten siihen sidottu näyttö ei enää
kelpaa: `verify-portal` ja tuotantomääritys uudella sormenjäljellä ajetaan uudestaan,
ja attestoinneista vahvistetaan käyttäjältä ne, jotka koskevat varmennetta. Vanhan
varmenteen näytön kopiointi uudelle tekisi statusista valheellisen.

Testisauma `AGGOUNTANT_BROKER_SCRIPT` korvaa Veron käsikirjoitetuilla vasteilla
valmiudesta riippumatta. Jos se on ympäristössä asetettu, `file-vat` merkitsee kauden
ilmoitetuksi ilman että mitään on lähetetty. Tarkista, ettei se ole voimassa, kun
tarkoitus on oikea lähetys.

## ALV-ilmoitus

Luvut syntyvät sinetöidystä kaudesta. `aggountant file-vat preview --period 2026-08`
on lukeva komento ilman ajoa, tunnuksia tai verkkoa: se palauttaa numeroidut
lomakekentät, `sales_vat`, `purchase_vat`, `net_vat` ja `payment_amount`. Selvitä
avoimet aineistokysymykset tässä vaiheessa; kun kausi on sinetöity ja lähetetty,
korjaus tarkoittaa korvaavaa ilmoitusta.

Esikatselu täsmäyttää myös ALV-tilit ilmoitukseen (`vat_reconciliation`, `attention`).
`unreconciled` tarkoittaa, että ALV-tileillä on vientejä ilman `vat_code`-merkintää ja
ilmoituksen luvuista puuttuu veroa; `untagged_postings` nimeää tositteet ja tilit.
Kerro tämä käyttäjälle ennen kuin ehdotat lähetystä, ja käy tositteet läpi lähdettä
vasten. Jos vero kuuluu kauden ilmoitukseen, suljettua kautta ei voi korjata
aggountantissa: käyttäjä ilmoittaa kauden OmaVerossa korjatuin luvuin, eikä samaa veroa
kirjata enää merkinnällä myöhemmälle kaudelle, koska se ilmoitettaisiin kahdesti.
`file-vat start` kieltäytyy tällaisesta kaudesta koodilla `VAT_ACCOUNTS_UNRECONCILED`
ennen kuin mitään lähtee. Kuittaus `vat_reconciliation_acknowledgement`
(`{"reason": …, "vouchers": [...]}`, tositteet täsmälleen esikatselun mukaan) on vain
vienneille, jotka eivät kuulu ilmoitukseen, kuten tilille 2939 kirjattu Verohallinnon
palautus tai avaavan tositteen ALV-saldo, ja perusteen antaa käyttäjä. Älä käytä sitä
puuttuvan veron ohittamiseen. Esikatselu toimii myös jo ilmoitetulle kaudelle, joten
sillä voi tarkistaa aiemmat ilmoitukset muuttamatta mitään.

Ilmoitusajo käynnistyy stdin-JSONista:

```json
{
  "period": "2026-08",
  "sales_vat": {"amount": "33.59", "currency": "EUR"},
  "purchase_vat": {"amount": "45.68", "currency": "EUR"},
  "payee_iban": "FI0000000000000000"
}
```

```sh
aggountant --json file-vat start < start.json
```

Luvut otetaan esikatselusta. Ohjelma johtaa ne itse uudelleen sinetöidystä
kirjanpidosta ja kieltäytyy, jos syöte poikkeaa (`VAT_FIGURES_MISMATCH`): näin
hyväksyntä sitoutuu lukuihin, jotka kirjanpito pystyy toistamaan. Y-tunnus ja
yhteyshenkilö tulevat profiilista; jos annat ne, niiden pitää täsmätä. `payee_iban`
on pakollinen ja päätyy vain paikalliseen maksuehdotukseen (käytä OmaVeron näyttämää
Verohallinnon tiliä); suora reitti ei maksa mitään.

Ajo tarkistaa valmiuden, lukee Veron kausilistan (`GetVATPeriods`) ja pysähtyy
`ApproveFiling`-portille. Kauden pitää olla Verossa `Unfiled` tai `Missing`; muu tila
antaa `VERO_VAT_PERIOD_NOT_FILEABLE`, ja paikallisesti jo ilmoitettu kausi
`PERIOD_TRANSITION_CONFLICT`. Vastaus sisältää `run_id`-tunnisteen (`fv-…`), ja
`aggountant run show --run-id` näyttää ajon portit ja efektit milloin tahansa.

Portti on sidottu lähetettävän pyynnön täsmällisiin tavuihin ja Veron kausitietoon.
Esitä käyttäjälle se, minkä hän hyväksyy: yritys, kausi ja eräpäivä, numeroidut kentät
euroina, nettosumma ja maksettava. Lähetä vain tähän ehdotukseen annetulla
hyväksynnällä:

```sh
aggountant --json file-vat approve-filing --run-id fv-… --answer approve
```

Jos kausi on välissä avattu ja sinetöity uudelleen, ohjelma hylkää vanhentuneen
ehdotuksen ennen lähetystä. Hylkäys (`--answer reject`) päättää ajon hylättynä; se ei
ole lupa korjata lukuja ja lähettää omin päin, vaan uusi kierros alkaa esikatselusta.

Onnistunut lähetys tallentaa Veron HTTP 200 -kuitin (`UniqueIdentifier`,
`AcceptedTimestamp`) audit trailiin ja merkitsee kauden `vat_filed`. Palautettava
ilmoitus (negatiivinen netto) päättää ajon ilman maksuvaihetta. Maksettava ilmoitus
avaa `ApprovePayment`-portin, mutta suoralla Vero-reitillä ei ole maksuadapteria, joten
sen hyväksyminen ei voi onnistua: käyttäjä maksaa OmaVeron viitteellä erikseen. Kerro
käyttäjälle kuitti, kauden tila ja maksettava summa. Palautuksen kohdistusta (pankkiin
vai vanhoihin veroihin) ei päätellä; se kirjataan vasta Verohallinnon myöhemmästä
tapahtumasta.

## Vastaanotettu, käsittely vahvistamatta

HTTP 200 todistaa, että Vero vastaanotti ilmoituksen, ei että se on käsitelty.
Ensimmäisissä tuotantoilmoituksissa (13.9.2026) OmaVero näytti kaudet edelleen
ilmoittamattomina seuraavana aamuna, ja `GetVATPeriods` piti niitä edelleen
ilmoittamattomina (`Missing` tai `Unfiled`). Verohallinnon puhelintuki selitti
14.9.2026, että API:n vastaanottamat aineistot noudetaan jatkokäsittelyyn väliajoin ja
noin 12 tunnin viive on normaali; ohje oli katsoa seuraavana päivänä. Se on käytännön
havainto, ei luvattu palvelutaso.

Näkymätön ilmoitus ei siis ole epäonnistunut, eikä se koskaan oikeuta uuteen
lähetykseen: uusi ajo pysähtyisi joka tapauksessa `PERIOD_TRANSITION_CONFLICT`-esteeseen,
ja Veron puolelle päätynyt kahdennus olisi korvaava ilmoitus, jota kukaan ei tarkoittanut.
Tilan voi kysyä lähettämättä mitään: `aggountant file-vat status --period 2026-03
--json` lukee kuitatun ilmoituksen `GetFiledVATReturn/v2`-tilan ja kertoo `received`
(kuitattu, ei vielä näkyvissä), `pending_authority_processing`, `processed` tai
`processed_values_differ` (käsitelty kuittauksen jälkeen eri luvuin: pyydä käyttäjää
vertaamaan OmaVerossa). Ilman kuittausta se vastaa paikallisista tiedoista
(`not_filed`, `filed_without_receipt`, `outcome_unknown` ja `next_command`
reconcileen, `filing_interrupted`). `VAT_STATUS_UNKNOWN` tarkoittaa vain, ettei tilaa
saatu luettua; kuittaus todistaa ilmoituksen silti. Kerro käyttäjälle sopiva odotusaika huomioiden viikonloput, ja jos ilmoitus ei näy
vuorokauden jälkeen tai eräpäivä painaa, kokoa hänelle Verohallinnon ALV-palvelulle
soittoa varten kauden tiedot, `UniqueIdentifier`, `AcceptedTimestamp`, Y-tunnus ja
lähetetyt kentät. Uusi lähetys vain Veron nimenomaisella ohjeella.

## Epävarma lopputulos

Aikakatkaisu tai 5xx lähetyksessä jättää efektin `OUTCOME_UNKNOWN`-tilaan ja ajon
`NeedsAttention`-tilaan. Ilmoitus voi olla Verossa tai olla olematta, joten sitä ei
päätellä kumpaankaan suuntaan. `aggountant file-vat reconcile --run-id fv-…` lukee
`GetFiledVATReturn/v2`-vastauksen: täsmäävät arvot päättävät ajon paikallisesti ilman
uutta lähetystä; puuttuva, vanhempi tai poikkeava ilmoitus jättää ajon odottamaan,
koska Vero ei välttämättä vielä näytä uusinta. Odota ja aja reconcile uudelleen, tai
pyydä käyttäjää katsomaan OmaVero. `file-vat retry` koskee vain maksuefektiä; ilmoitusta
ei koskaan lähetetä uudelleen sokkona.

## Manuaalinen reitti

Kun suora reitti ei ole käytettävissä tai käyttäjä haluaa asioida itse, esikatselun
numeroidut kentät ovat ilmoitettavat luvut. Käyttäjä syöttää ne OmaVeroon tai laatii
Ilmoitin.fi-tiedoston ja maksaa erikseen. Arkistoi hänen toimittamansa kuitti tai muu
yksilöity näyttö ja kirjaa ilmoitus tehdyksi:

```sh
aggountant file-vat complete-manual --period 2026-08 --channel omavero \
  --evidence-ref <näytön tunniste>
```

Komento ei lähetä eikä maksa. Jos samalla kaudella on `OUTCOME_UNKNOWN`-tilaan jäänyt
API-yritys, komento vaatii lisäksi `--remote-status-evidence-ref`-viitteen näyttöön
siitä, että Veron tila on selvitetty: muuten manuaalinen ilmoitus voisi huomaamatta
olla kaksoisilmoitus tai korvaava ilmoitus.

## Käyttäjän aika ja päätökset

Status, esikatselu ja `--dry-run` ovat ilmaisia ja jälkiä jättämättömiä; aja niitä
kysymättä ja kokoa käyttäjälle yksi kokonaiskuva sen sijaan että kysyt jokaista estettä
erikseen. Käyttäjän nimenomainen päätös tarvitaan asioihin, joita ei saa takaisin tai
jotka ovat hänen lausumiaan: ensimmäinen kutsu Veron tuotantoon (`verify-portal`),
tuotantotilaan siirtyminen, jokainen attestointi, ja jokainen ilmoituksen lähetys
täsmälleen näytettyyn ehdotukseen. Kun kysyt, näytä mitä päätös koskee ja mihin
vastaus vaikuttaa, ja vie muu työ eteenpäin vastausta odottaessa.

## Tulorekisterin verovapaat matkakorvaukset (311 ja 331)

Tämä on eri kanava kuin Vero API:n ALV-ilmoitus. `run-payroll` on edelleen
synteettinen koe eikä lähetä oikeaa palkkailmoitusta. Oikea, rajattu polku on
`aggountant file-wage`: yksi tulonsaaja, yksi maksutapahtuma, vain verovapaat
matkakorvaukset. Työnantajan erillisilmoitus, korvaava ilmoitus ja mitätöinti
API:n kautta eivät kuulu tähän polkuun. Ohje ei ole lupa lähettää mitään:
käyttäjän pitää hyväksyä juuri tämän ilmoituksen esikatselu.

### Rajapintaoikeus ja konfigurointi

Käyttäjä avaa Verohallinnon varmennepalvelun → Tuotantovarmenteet → **Tee korvaava
hakemus**. Vero API pidetään valittuna; lisäksi valitaan Tulorekisterin
palkkatietojen ilmoittamisen rajapinta, käyttötarkoitus **ilmoitan itse maksamiani
tuloja**. Aiempi WS-varmenne kelpaa virallisen ohjeen mukaan myös tähän kanavaan,
kun oikeus on myönnetty. Erillinen varmenne on sallittu. Älä merkitse oikeutta
myönnetyksi hakemuksen lähettämisen perusteella.

Kirjanpitokodin `.aggountant/tulorekisteri.json` sisältää ei-salaisen profiilin:

```json
{
  "schema_version": 1,
  "mode": "production",
  "business_id": "1234567-1",
  "reporting_permission_active": true,
  "credentials_file": null
}
```

`credentials_file: null` käyttää nykyistä Vero-credentialin polkua
(`AGGOUNTANT_VERO_CREDENTIALS_FILE` > alustakonfiguraation
`vero_credentials_file`). Erilliselle varmenteelle annetaan absoluuttinen polku.
Nippu on samaa muotoa kuin yllä, myös `software_key` mukana vaikka sitä ei
lähetetä Tulorekisteriin. Nipun on oltava kirjanpitokodin ulkopuolella.
Tuotantotila, ihmisen oikeusattestaatio, oma Y-tunnus, credentialin
`target: production`, voimassa oleva varmenne ja yhteensopiva RSA-avain vaaditaan.
Valtuutustokenia ei tueta. Salaisuuksia ei laiteta keskusteluun tai gitiin.
TLS tarkistaa palvelimen nimen ja luottamusketjun; HTTP-uudelleenohjauksia ei seurata.

### Esikatselu ja hyväksyntä

Tulonsaaja on ensin tallennettava `employee`-komennolla PII-storeen.
Ilmoituksen stdin-JSON **ei sisällä HETUa**. Esimerkki käyttää kuvitteellisia tietoja:

```json
{
  "business_id": "1234567-1",
  "source_reference": "expense-payment-2026-09-001",
  "employee_id": "employee-1",
  "payment_date": "2026-10-25",
  "period_start": "2026-09-01",
  "period_end": "2026-09-30",
  "joint_owner": true,
  "contact_name": "Fictional Contact",
  "contact_phone": "+358 40 1234567",
  "lines": [
    {"income_type": "311", "amount": "120.00", "kilometers": 218},
    {"income_type": "331", "amount": "54.00", "allowance_code": 3}
  ]
}
```

`joint_owner` on eksplisiittinen pääomaosakkuustieto (IncomeEarnerType 2), ei
päätelmä toimitusjohtajan asemasta eikä työttömyysturvan osaomistajaluokitus.
Kauden alku ja loppu annetaan itse: ohjelma ei arvaa kululaskun kautta.
331:n `allowance_code`: 1 ateriakorvaus, 2 osapäiväraha, 3 kokopäiväraha,
4 ulkomaan päiväraha. Summat ovat positiivisia EUR-senttejä desimaalimerkkijonoina.
311 tarvitsee positiiviset kokonaiskilometrit. Kuittiperusteisia hotelli- tai
pysäköintikuluja ei lisätä näihin riveihin.

```sh
aggountant --json file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json
aggountant --json file-wage start --timestamp 2026-10-25T12:00:00Z < report.json
aggountant --json run show wr-FULL_IDENTITY_DIGEST
# Vasta käyttäjän hyväksyttyä rivit, kauden, osakkuuden ja hashin:
aggountant --json file-wage approve wr-FULL_IDENTITY_DIGEST --record-hash PREVIEW_RECORD_HASH
```

Esikatselu ja start eivät käytä verkkoa tai credentialia. Sama timestamp sidotaan
molempiin komentoihin. `start --dry-run` näyttää suunnitelman kirjoittamatta.
Esikatselussa näkyvät rivit, kausi, maksupäivä, osakkuustieto, yhteyshenkilö,
DeliveryId/ReportId sekä konservatiivinen työnkulun määräpäivä maksupäivä + 5
kalenteripäivää (esimerkissä 30.10.). `record_hash` on **allekirjoittamattoman,
kanonisen XML-tietueen** SHA-256; se sitoo myös PII-storesta ratkaistun HETUn ja
timestampin. HETU ei näy selväkielisenä, mutta pienestä tunnisteavaruudesta johtuen
**record_hash on henkilötiedoista johdettu arkaluonteinen hyväksyntätunniste**:
älä tallenna sitä gitiin, ohjeisiin tai keskustelulokiin. Myös `start --dry-run`
-tulosta käsitellään näin. Prosessin git-versioitu tila ja tavallinen `run show`
sisältävät vain ei-henkilötietoperusteisen ehdotustarkisteen; täysi tarkiste
annetaan hyväksyntäkomennolle tilapäisesti ja tarkistetaan PII-storea vasten.
Allekirjoitus ja varmenne lisätään vasta
hyväksynnän jälkeen. Tämä hash ei ole allekirjoitetun SOAP-kehyksen hash.

`source_reference` yksilöi maksutapahtuman pysyvästi. Älä vaihda sitä tai
maksupäivää kiertääksesi hylkäyksen/epävarmuuden: sisältömuutos ei saa synnyttää
huomaamatta uutta ilmoitusta. Sama luonnollinen avain omistaa yhden paikallisen
Runin; muuttunut sisältö tai HETU estää vanhan hyväksynnän käytön.

### Epävarma tulos ja asiointipalvelun varapolku

```sh
aggountant --json file-wage reconcile wr-FULL_IDENTITY_DIGEST
```

Timeout, 5xx, SOAP Fault, `DDVS0280` yksinään sekä puuttuva/ristiriitainen palaute
jäävät epävarmoiksi. Hyväksyntäkomennon toisto **ei lähetä uudelleen**.
Reconcile tekee allekirjoitetun `GetDeliveryDataStatus`-kyselyn ja hyväksyy vain
oman tuotantoilmoituksen täsmäävät DeliveryId/ReportId-tunnisteet ja version 1.
Jos status on vielä käsittelyssä, selvitä myöhemmin uudelleen. Ohjelma ei vaihda
tunnisteita eikä lähetä automaattista korjausta. Raakaa viranomaisvirhetekstiä ei
näytetä, koska se voi sisältää HETUn; tarkat virheet voi katsoa asiointipalvelusta.

Jos API-polku ei ehdi hyväksyttäväksi tai ilmoitus on korjattava, käyttäjä tekee
sen Tulorekisterin asiointipalvelussa (Suomi.fi), ei OmaVerossa. **Tarkista ensin,
onko epävarma API-lähetys jo hyväksytty.** Älä tee toista alkuperäistä ilmoitusta,
jos vanha on jo rekisterissä. Kirjaa palvelun hyväksytyn ilmoituksen viitteet:

```sh
aggountant --json file-wage complete-manual wr-FULL_IDENTITY_DIGEST \
  --ir-delivery-id 11111111222233334444555555555555 \
  --ir-report-id 22222222222233334444555555555555
```

Viitteet yllä ovat kuvitteellisia 32-merkkisiä Tulorekisterin viitteitä; käytä oikeita asiointipalvelun viitteitä.
Manuaalinen valmistuminen on erillinen, nimenomainen teko eikä ohjelman hiljainen
varapolku. Korvaavat ilmoitukset ja mitätöinnit tehdään toistaiseksi palvelussa.

Tilahaun vahvistama hylkäys kirjataan `TULOREKISTERI_REJECTED`-koodilla
(`journal show`), ja Run jää `needs_attention`-tilaan. Se ei ole enää ohje
odottaa käsittelyn valmistumista: selvitä virhe asiointipalvelusta ja käytä
manuaalista varapolkua. Myös ennen yhteyden muodostumista tapahtunut yhteysvirhe
käsitellään tässä rajatussa polussa konservatiivisesti ilman uutta API-lähetystä;
tarkista ensin palvelusta, ettei alkuperäinen ole rekisterissä.
Tilahaku ja manuaalinen valmistuminen eivät edellytä tulonsaajan henkilötietorivin
säilymistä. Integraation outbox pitää omassa `.gitignore`-tiedostossaan tietokannan
ja WAL-tiedostot poissa gitistä; tarkista ennen ensimmäistä oikeaa käyttöä, ettei
vanhaa outbox-tietokantaa ole jo lisätty seurantaan (ignore ei poista seurattua
historiaa).
