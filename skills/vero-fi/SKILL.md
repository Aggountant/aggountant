---
name: vero-fi
description: >-
  Ota aggountantin suora Vero.fi-ALV-ilmoittaminen käyttöön, tarkista valmius,
  uusi varmenne, lähetä ALV-ilmoitus hyväksynnän kautta, selvitä epävarma tai
  näkymätön lähetys tai kirjaa manuaalinen ilmoitus (OmaVero, Ilmoitin.fi).
  Ilmoita Tulorekisteriin verovapaat matkakorvaukset ja kuukausittainen
  työnantajan erillisilmoitus, ja kuro kiinni puuttuvat kuukaudet.
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
puuttuvan veron ohittamiseen. ALV:n maksu tai palautus jää vertailun ulkopuolelle vain,
kun sen muut viennit ovat ALV-tilejä, tilikartan maksutilejä (pankki- ja verotilit) tai
pieniä korko- ja viivästysmaksueriä. `VAT_SETTLEMENT_ROLES_MISSING` tarkoittaa, ettei
tositteiden tilikartalla ole maksutilien rooleja: ehdota ensin `home upgrade`
-ajoa (vakiotilikartta) ja uutta esikatselua; vasta sitten maksujen kuittausta. Esikatselu toimii myös jo ilmoitetulle kaudelle, joten
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
`PERIOD_TRANSITION_CONFLICT`. Jos kausilistan luku epäonnistuu tai kausi ei kelpaa,
ajo jää tilaan `needs_attention` (`reconcile`-efekti hylätty), virhe kertoo syykoodin
ja nimeää ajon, eikä mitään lähetetty: kerro syy käyttäjälle ja aloita korjauksen
jälkeen uusi ajo, älä yritä samaa ajoa uudelleen. Vastaus sisältää
`run_id`-tunnisteen (`fv-…`), ja `aggountant run show --run-id` näyttää ajon portit
ja efektit milloin tahansa.

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
`AcceptedTimestamp`) audit trailiin ja merkitsee kauden `vat_filed`. Kuitti on
`approve-filing`-vastauksessa `submit-filing`-efektin kentässä `outcome`
(`state: accepted`, `receipt.unique_identifier`, `receipt.accepted_timestamp`), ja
`aggountant --json run show --run-id fv-…` näyttää saman myöhemmin verkkoa käyttämättä.
Lue kuitti näistä, älä ajon journal-tiedostoista. `state: outcome_unknown` kertoo
`next_command`-kentässä reconcile-komennon. Palautettava
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
`processed_values_differ` (käsitelty kuittauksen jälkeen eri luvuin: `differences`
luettelee kentät `{field, return_field, approved, authority}`; pyydä käyttäjää vertaamaan
juuri niitä OmaVerossa). `file-vat reconcile` palauttaa samat `differences`, kun
Verohallinnon käsittelemä ilmoitus ei vastaa hyväksyttyä. Ilman kuittausta se vastaa paikallisista tiedoista
(`not_filed`, `filed_without_receipt`, `outcome_unknown` ja `next_command`
reconcileen). Keskeytyneestä lähetyksestä se kysyy Verohallinnolta (ks. alla).
`VAT_STATUS_UNKNOWN` tarkoittaa vain, ettei tilaa saatu luettua; kuittaus todistaa
ilmoituksen silti. Kerro käyttäjälle sopiva odotusaika huomioiden viikonloput, ja jos ilmoitus ei näy
vuorokauden jälkeen tai eräpäivä painaa, kokoa hänelle Verohallinnon ALV-palvelulle
soittoa varten kauden tiedot, `UniqueIdentifier`, `AcceptedTimestamp`, Y-tunnus ja
lähetetyt kentät. Uusi lähetys vain Veron nimenomaisella ohjeella.

## Epävarma lopputulos

Aikakatkaisu, 5xx tai 2xx-vastaus ilman luettavaa kuittia (puuttuva tai tyhjä
`UniqueIdentifier`/`AcceptedTimestamp`, lukukelvoton runko tai muu kuin 200) jättää
efektin `OUTCOME_UNKNOWN`-tilaan ja ajon `NeedsAttention`-tilaan; se ei ole hylkäys.
Ennen tätä tallennetuissa ajoissa sama tilanne näkyy hylkäyskoodina `MALFORMED: …` tai
`HTTP_2xx: …`, ja sitä käsitellään samoin. Ilmoitus voi olla Verossa tai olla olematta, joten sitä ei
päätellä kumpaankaan suuntaan. `aggountant file-vat reconcile --run-id fv-…` lukee
`GetFiledVATReturn/v2`-vastauksen ja kirjaa sen ajoon (`authority_checks`):
täsmäävät arvot päättävät ajon paikallisesti ilman uutta lähetystä (`outcome: adopted`);
vanhempi tai poikkeava ilmoitus jättää ajon odottamaan (`unresolved`). `Missing` on yksi
"ei löydy" (`not_found`); toinen `Missing` aikaisintaan vastauksen `next_check_from`-hetkellä
(sama kellonaika seuraavana suomalaisena pankkipäivänä) sulkee ajon ilmoittamattomana (`closed_not_filed`), ja
vasta sitten uusi `file-vat start` voi lähettää kauden. Muu vastaus välissä nollaa
laskun; epäonnistunut tai lukukelvoton kysely ei kirjaa mitään. Siihen asti
`file-vat start` ja `approve-filing` kieltäytyvät koodilla `VAT_FILING_UNRESOLVED`
(myös kun toisen ajon lähetys on keskeytynyt). Älä kierrä tätä: kerro käyttäjälle, milloin
kysytään uudelleen, tai pyydä häntä katsomaan OmaVero. Jos uusi ajo muodostaisi eri
pyynnön kuin aiempi yritys, se kieltäytyy; silloin ilmoitus tehdään käsin
(`complete-manual` ja "ei löydy" -vastaukset näyttönä). `file-vat retry` koskee vain
maksuefektiä; ilmoitusta ei koskaan lähetetä uudelleen sokkona.

## Keskeytynyt lähetys

Jos ajo keskeytyi lähetyksen ja kirjauksen välissä, ilmoitusefekti jää
`executing`-tilaan, ajon `outcome.next_command` on `file-vat status`, ja uusi ajo
kieltäytyy (`VAT_FILING_UNRESOLVED`). Jatkamiskomentoa ei ole. `aggountant
file-vat status --period … --json` katsoo outboxin ja kysyy `GetFiledVATReturn/v2`:n
kirjaamatta mitään:

- `interrupted_filed`: kuittaus on outboxissa tai Vero näyttää hyväksytyn
  ilmoituksen käsiteltynä. `next_command` (`file-vat reconcile --run-id …`) kirjaa
  ilmoituksen (ja kuittauksen, jos sellainen on) lähettämättä mitään.
- `interrupted_not_filed`: pyyntö ei lähtenyt tai Vero hylkäsi sen suoraan, eikä
  kaudelta ole ilmoitusta. `reconcile` kirjaa vastauksen ja sulkee ajon
  (`closed_not_filed`); sen jälkeen uusi `file-vat start` lähettää kauden kerran.
- `filing_interrupted`: pyyntö on voinut päätyä Veroon (`Missing` ei todista
  katoamista), tai Vero näyttää muuta. `Missing`-vastauksella `reconcile` toimii
  kuten epävarmassa lopputuloksessa: kaksi "ei löydy" -vastausta pankkipäivän
  välein sulkee ajon. Muulla vastauksella `next_command` puuttuu: vertaa OmaVerossa
  ja kysy myöhemmin. Jos ajo ei lähettänyt mitään mutta Vero näyttää hyväksytyt
  luvut käsiteltyinä, ilmoitus on tehty muuta kautta: sitä ei oteta ajon omaksi,
  vaan käyttäjä vahvistaa sen ja se kirjataan `complete-manual`illa (Veron vastaus
  `--remote-status-evidence-ref`-näyttönä). Sen jälkeen `reconcile` sulkee ajon
  (`superseded`).

Kuitatun ilmoituksen tila voi myös kertoa `next_command`-kentässä `reconcile`n, jos
ajo keskeytyi kirjauksen jälkeen; se vain päivittää ajon. `VAT_STATUS_UNKNOWN` ei
muuta mitään eikä koskaan oikeuta uuteen lähetykseen.

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

## Määräajat kodin `TODO.md`-tiedostossa

Kodin `TODO.md` on yrityksen muistilista odottavasta työstä ja määräajoista
(`aggountant-init` kuvaa sen). Lue sen määräajat ennen kuin kerrot käyttäjälle, mitä
ilmoituksia on edessä, ja päivitä se heti ilmoituksen tilan muututtua (jos koti on
git-repositorio, samalla kertaa ajon tilan kanssa). Määräaika poistetaan, kun
ilmoitus on käsitelty (ALV:ssa `file-vat status` kertoo `processed`) tai kirjattu
`complete-manual`-komennolla; lisää silloin seuraavan kauden määräaika, jos se on
tiedossa. Kuitattu mutta vielä
käsittelemätön ilmoitus tai `outcome_unknown`-tilaan jäänyt ajo kirjataan kohdaksi,
jossa on ajon tunniste, eräpäivä ja milloin tila tarkistetaan uudelleen. Tiedosto on
muistilista, ei tilan lähde: ilmoituksen tila luetaan aina ohjelmalta (`file-vat
status`, `run show`), ja ristiriidassa ohjelma on oikeassa ja kohta korjataan.

Odottava ilmoitus tai määräpäivä ei kuulu `knowledge company`-tietueeksi. Tietoon
kuuluu pysyvä fakta lähteineen, esimerkiksi yrityksen ALV-kauden pituus.

## Tulorekisterin verovapaat matkakorvaukset (311 ja 331)

Tämä on eri kanava kuin Vero API:n ALV-ilmoitus. `run-payroll` on edelleen
synteettinen koe eikä lähetä oikeaa palkkailmoitusta. Oikea, rajattu polku on
`aggountant file-wage`: yksi tulonsaaja, yksi maksutapahtuma, vain verovapaat
matkakorvaukset. Palkkatietoilmoituksen korvaava ilmoitus ja mitätöinti API:n
kautta eivät kuulu tähän polkuun. Kuukausittainen työnantajan erillisilmoitus on
oma ilmoituksensa samassa kanavassa (alla). Ohje ei ole lupa lähettää mitään:
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
  "credentials_file": null,
  "employer_report": {
    "registered_from": "2026-08",
    "contact_name": "Maija Meikäläinen",
    "contact_phone": "+358 40 1234567",
    "pension_insurances": []
  }
}
```

`employer_report` puuttuu, kunnes yritys on työnantajarekisterissä. Sen
kirjoittaa `file-wage employer-report configure` (erillisilmoituksen osio
alla); älä muokkaa lohkoa käsin.

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
aggountant --json run show --run-id wr-FULL_IDENTITY_DIGEST
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

Hyväksytyn ilmoituksen kuitti on `approve`-vastauksen `submit-wage`-efektin
kentässä `outcome` (`state: accepted`; `receipt.ir_delivery_id` = IRDeliveryId,
`receipt.ir_report_id` = IRItemId, `receipt.version`, `receipt.source` `api` tai
`e_service`). `run show --run-id wr-…` lukee saman myöhemmin ilman verkkoa ja
credentialia; reconcile on vain epävarman tuloksen selvittämiseen, ja
`state: outcome_unknown` nimeää sen `next_command`-kentässä. Älä lue ajon
journal-tiedostoja.

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

## Työnantajan erillisilmoitus (joka kuukausi)

Verohallinnon työnantajarekisterissä oleva yritys antaa Tulorekisteriin
työnantajan erillisilmoituksen **jokaiselta kuukaudelta**, myös silloin kun mitään
ei maksettu. Määräpäivä on seuraavan kuukauden 5. päivä (lauantaina tai
pyhäpäivänä seuraava pankkipäivä; ohjelma näyttää aina 5. päivän). Ilmoitus on
eri asia kuin palkkatietoilmoitus: matkakorvausilmoitus ei korvaa sitä.
Satunnainen työnantaja, joka ei ole rekisterissä, ei anna sitä kuukausilta, joilta
se ei maksanut mitään; silloin `employer_report` jätetään profiilista pois.

Profiilin `employer_report` on yrityksen asetus: `registered_from` on
työnantajarekisteröinnin ensimmäinen kuukausi (tarkista se `registration`-komennolla,
alla), yhteyshenkilö tulee ilmoitukseen, ja
`pension_insurances` (`provider_code` = Tulorekisterin eläkelaitostunnus,
`policy_number`) lähetetään "Ei palkanmaksua" -ilmoituksen mukana. Tyhjä lista
on oikea vain, jos yrityksellä ei ole vakuuttamisvelvollisuutta (esimerkiksi vain
YEL-vakuutettu omistaja); kysy käyttäjältä, älä arvaa. Yhteyshenkilön ja
eläkevakuutuksen tiedot ovat käyttäjän antamia, eivät keksittyjä.

Lohkon kirjoittaa `configure`, ei käsin tehty JSON-muokkaus. Se tarkistaa
puhelinnumeron, eläkelaitostunnuksen ja vakuutusnumeron muodon, vertaa
`registered_from`-arvoa YTJ:n työnantajarekisteriin (sama vertailu kuin
`registration`), kirjoittaa vain `employer_report`-lohkon (profiilin muut tavut
pysyvät) ja näyttää `before`/`after`. Aja ensin `--dry-run`, näytä tulos
käyttäjälle ja aja sitten ilman sitä. Eläkevakuutukset annetaan
`--pension-insurance KOODI:NUMERO` (toistettava) tai `--no-pension-insurance`;
kumpaakaan ei oleteta. Samat kentät voi antaa myös JSONina vakiosyötteessä
ilman kenttävalitsimia.

- `EMPLOYER_REGISTER_MISMATCH`: YTJ on eri mieltä alkukuukaudesta (viesti
  kertoo vertailun ja rekisterin mukaisen kuukauden). Ehdota rekisterin
  kuukautta; `--accept-register-mismatch` vain, kun käyttäjä vahvistaa oman
  kuukautensa. Tulos näyttää silloin `registration_verdict:
  mismatch_accepted` ja vertailun.
- `YTJ_UNAVAILABLE`: mitään ei kirjoitettu. Kokeile myöhemmin; `--offline`
  (kirjoitus ilman tarkistusta, `registration_verdict: not_checked`) vain
  käyttäjän luvalla, ja aja `registration` myöhemmin.
- `EMPLOYER_REPORT_INVALID_SETTINGS` tai `INVALID_INPUT`: korjaa arvo
  käyttäjän kanssa, älä muotoile sitä itse sopivaksi.

```sh
aggountant --json file-wage employer-report configure --registered-from 2020-02 \
  --contact-name "Maija Meikäläinen" --contact-phone "+358 40 1234567" \
  --pension-insurance 46:12-3456789 --dry-run
aggountant --json file-wage employer-report configure --registered-from 2020-02 \
  --contact-name "Maija Meikäläinen" --contact-phone "+358 40 1234567" \
  --no-pension-insurance
```

Sisällön ratkaisee se, mitä kuukaudella maksettiin. Kysy tai selvitä se
kirjanpidosta ja tiliotteelta; ohjelma ei päättele sitä tileistä:

- `none`: ei mitään maksuja → "Ei palkanmaksua" (tulolaji 101).
- `tax-free-reimbursements-only`: vain verovapaita matkakorvauksia →
  työnantajan sairausvakuutusmaksu 0,00 € (102), ei "Ei palkanmaksua"
  (Tulorekisterin ohje, kohta 4.3). Ohjelma kieltää `none`-valinnan, jos
  kuukaudelle on `file-wage`-ilmoitus, ja varoittaa, jos matkakorvauksille ei
  löydy ilmoitusta.
- `wages`: rahapalkkaa → `EMPLOYER_REPORT_WAGES_UNSUPPORTED`. Sairausvakuutusmaksua
  ei vielä lasketa; käyttäjä antaa kuukauden ilmoituksen asiointipalvelussa ja
  kirjaa sen `complete-manual`-komennolla.

```sh
aggountant --json file-wage employer-report missing
aggountant --json file-wage employer-report preview --month 2026-10 \
  --payments none --timestamp 2026-11-02T08:00:00Z
# Vasta kun käyttäjä on hyväksynyt kuukauden, sisällön ja record_hashin:
aggountant --json file-wage employer-report submit --month 2026-10 \
  --payments none --timestamp 2026-11-02T08:00:00Z --record-hash PREVIEW_RECORD_HASH
aggountant --json file-wage employer-report status --month 2026-10
```

Esikatselu ei kirjoita mitään eikä käytä verkkoa. Sama `--month`, `--payments`,
`--timestamp` (ja `--replace`) annetaan esikatselulle ja lähetykselle; aikaleima on
myös ilmoituksen ilmoituspäivä, joten se saa olla enintään vuorokauden vanha
lähetettäessä. `record_hash` on allekirjoittamattoman XML-tietueen SHA-256. Siinä
ei ole henkilötietoja, joten sen saa näyttää käyttäjälle. Lähetys sitoo juuri
nämä tavut. Ohjelma ei koskaan päätä itse lähettää kuukautta: jokainen kuukausi
esikatsellaan ja hyväksytään erikseen, myös kiinnikurottaessa.

**Kiinnikurominen, kun historia on epäselvä.** Kun käyttäjä ei tiedä
rekisteröinnin alkua tai sitä, mitkä kuukaudet aiempi palkanlaskija on jo
ilmoittanut, etene tässä järjestyksessä:

1. **Rekisteröinti YTJ:stä.** `registration` hakee Y-tunnuksella
   työnantajarekisterin merkinnät PRH:n avoimesta datasta ja vertaa ne
   `registered_from`-arvoon (`comparison`: `match`,
   `profile_later_than_register` = kuukausia piilossa `missing`-listalta,
   `profile_earlier_than_register`, `register_ended`, `not_in_register`,
   `profile_not_set`). Komento ei kirjoita mitään. Näytä tulos ja
   `implied_registered_from`; muuta profiilia vasta käyttäjän luvalla,
   `configure --registered-from <implied_registered_from>` -komennolla.
   `YTJ_UNAVAILABLE` tarkoittaa "ei tiedossa", ei koskaan "ei rekisterissä".
2. **Raportti 630 asiointipalvelusta.** Kerro käyttäjälle suoraan, että tämä
   on ainoa vaihe, joka vaatii Tulorekisterin asiointipalvelua: Uusi tilaus →
   *Maksajan palkkatietojen yhteenveto (csv)* (630), kohdekauden aikaväli
   rekisteröinnin kattamalta ajalta enintään 24 kuukautta kerrallaan, kieleksi
   **suomi**, **vastaanottohetken aikaväli tyhjäksi** (rajattu raportti jättää
   pois muulloin vastaanotetut ilmoitukset, eikä sitä näe tiedostosta), lataus
   *Tehdyt tilaukset* -sivulta (zip puretaan).
3. **Tuonti.** `import-transcript` ensin `--dry-run`-valinnalla, sitten ilman.
   Raportissa ei ole maksajan Y-tunnusta: varmista käyttäjältä, että tiedosto
   on tämän yrityksen. Tulos kertoo `recorded` (merkitty annetuksi
   aggountantin ulkopuolella), `already_held` (aggountant tunsi jo; ei
   muutettu), `not_reported` (jää puuttuvaksi) ja `conflicts`. Näytä
   ristiriidat käyttäjälle äläkä ratkaise niitä itse. Jos kaksi raporttia
   on eri mieltä kuukaudesta (`transcripts_disagree`), kuukausi on
   `missing`-listalla tilassa `transcript_conflict`, eikä sille lähetetä
   mitään (`EMPLOYER_REPORT_TRANSCRIPT_CONFLICT`): käyttäjä tarkistaa
   kuukauden asiointipalvelusta, ja olemassa olevan ilmoituksen viite
   kirjataan `complete-manual`-komennolla.
   `TRANSCRIPT_UNKNOWN_HEADER`: tiedosto ei ole suomenkielinen raportti 630
   asiointipalvelun tunnetuin sarakkein; älä muokkaa tiedostoa sopivaksi vaan
   pyydä tilaamaan se uudelleen suomeksi, tai jos se on oikea raportti, kerro
   käyttäjälle, että asiointipalvelun muoto on muuttunut.
4. **`missing`**, ja sen jälkeen jokainen puuttuva kuukausi vanhimmasta
   alkaen: esikatselu, käyttäjän hyväksyntä, lähetys.

```sh
aggountant --json file-wage employer-report registration
aggountant --json file-wage employer-report import-transcript raportti-630.csv --dry-run
aggountant --json file-wage employer-report import-transcript raportti-630.csv
aggountant --json file-wage employer-report missing
```

Raportin perusteella annetuksi merkityllä kuukaudella (`filed_outside`) ei ole
ilmoitusviitettä. Uusi lähetys palauttaa `EMPLOYER_REPORT_FILED_OUTSIDE` ja
korvaava (`--replace`) `EMPLOYER_REPORT_NO_REFERENCE`: korjaus tehdään
asiointipalvelussa, tai käyttäjä hakee viitteen ja se kirjataan
`complete-manual`-komennolla, minkä jälkeen korvaava onnistuu. Sano tämä
käyttäjälle, kun hän pyytää korjaamaan tällaista kuukautta.

**Puuttuvat kuukaudet.** `missing` luettelee kuukaudet rekisteröinnistä edelliseen
kuukauteen asti, joilta ei ole hyväksyttyä ilmoitusta eikä tuodun raportin 630
näyttöä, määräpäivineen ja tiloineen (`not_filed`, `pending`, `rejected`,
`outcome_unknown`, `transcript_conflict`); raportin kuukaudet ovat erikseen kohdassa `filed_outside`
lähteineen. Käy puuttuvat läpi vanhimmasta alkaen: selvitä kunkin kuukauden
maksut, esikatsele, hyväksytä ja lähetä. Jos kuukausi on jo annettu
asiointipalvelussa, älä lähetä sitä uudelleen vaan kirjaa viite:

```sh
aggountant --json file-wage employer-report complete-manual --month 2026-08 \
  --ir-report-id 22222222222233334444555555555555 --report-version 1
```

Viite on kuvitteellinen; käytä asiointipalvelun ilmoitusviitettä. Versio
(`--report-version`) ja tietueviite (`--ir-delivery-id`) annetaan, jos ne ovat
tiedossa; tunnettu versio tekee myöhemmästä korvaavasta ilmoituksesta tarkemman.
Jos epävarma lähetys oli korvaava ilmoitus, versio on pakollinen ja sen pitää
olla korvattua versiota uudempi; asiointipalvelun vanha versio tarkoittaa, ettei
korvaava ole perillä, ja kuukausi jää odottamaan (`EMPLOYER_REPORT_INVALID_EVIDENCE`).
Jos korvatun version numeroa ei kirjattu, korvaava ratkaistaan vain
`status`-tilakyselyllä.

**Kuittaus estää uuden lähetyksen.** Kun kuukaudella on hyväksytty ilmoitus,
uusi lähetys palauttaa `EMPLOYER_REPORT_ALREADY_FILED`. Korjaus on korvaava
ilmoitus `--replace`-valinnalla: ohjelma kohdistaa sen Tulorekisterin
ilmoitusviitteeseen ja tunnettuun versioon. Korvaavassa on samat
eläkevakuutustiedot kuin edellisessä versiossa (Tulorekisterin käsittelysääntö):
"Ei palkanmaksua" → 0 € -korvaava toistaa edellisen version vakuutukset.
Eläkevakuutustietojen korjausta ei tehdä korvaavalla
(`EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED`); mitätöinti ja uusi ilmoitus tehdään
asiointipalvelussa.

**Epävarma lopputulos.** Aikakatkaisu, 5xx tai ristiriitainen palaute jättää
kuukauden tilaan `outcome_unknown`. Silloin kaikki uudet lähetykset kuukaudelle
estetään (`EMPLOYER_REPORT_OUTCOME_UNKNOWN`). `status --month` tekee allekirjoitetun
tilakyselyn eikä lähetä uudelleen. `EMPLOYER_REPORT_STATUS_UNKNOWN` tarkoittaa,
että kysy myöhemmin uudelleen tai tarkista asiointipalvelusta ja kirjaa viite
`complete-manual`-komennolla. Hylätty kuukausi (`rejected`) on lopullinen: selvitä
syy asiointipalvelusta ja esikatsele uudelleen uudella aikaleimalla.
