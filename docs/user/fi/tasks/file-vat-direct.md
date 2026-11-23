# Lähetä ALV-ilmoitus suoraan Verohallinnolle

Agentti lähettää suljetun kuukauden ALV-ilmoituksen Verohallinnolle Vero
API -rajapinnan kautta yrityksesi omilla tunnuksilla, kun olet hyväksynyt
ilmoituksen. Lähetettyä ilmoitusta ei voi perua, mutta sen voi
[korvata](#korjaa-jo-annettu-ilmoitus).

## Ennen kuin aloitat

- Kuukausi on [suljettu](close-month.md).
- [Suora ilmoittaminen on otettu käyttöön](set-up-vero-api.md), eli
  valmiuden tarkistus näyttää `ready`.
- Kuukautta ei ole vielä ilmoitettu Verohallinnolle millään tavalla.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tee maaliskuun ALV-ilmoitus suoraan Verohallinnolle.

2. **Agentti esikatselee luvut** suljetun kuukauden kirjanpidosta samalla
   tavalla kuin [OmaVerossa tehtävää ilmoitusta](file-vat-omavero.md) varten.

3. **Agentti valmistelee lähetyksen.** Se antaa ohjelmalle kuukauden, myynnin
   ja ostojen veron sekä Verohallinnon tilinumeron, joka näkyy OmaVerossa.
   Ohjelma laskee luvut vielä kerran kirjanpidosta ja kieltäytyy, jos ne
   eroavat tai jos ALV-tilit eivät täsmää ilmoitukseen
   (`VAT_ACCOUNTS_UNRECONCILED`). Sen jälkeen ohjelma tarkistaa valmiuden,
   kysyy kuukauden tilan Verohallinnolta ja jää odottamaan hyväksyntääsi.
   Mitään ei ole vielä lähetetty.

4. **Tarkista ja hyväksy.** Agentti näyttää yrityksen, kuukauden, eräpäivän,
   numeroidut kentät, nettosumman ja maksettavan määrän. Tarkista, että
   kentät ovat samat kuin esikatselussa ja että kuukausi on oikea.
   Hyväksyntä koskee täsmälleen sitä sisältöä, jonka agentti näytti.

   > Hyväksyn ilmoituksen.

   Jos hylkäät, ajo päättyy, eikä mitään lähetetä. Uusi kierros alkaa
   esikatselusta.

5. **Ohjelma lähettää ilmoituksen.** Kun Verohallinto on vastaanottanut sen,
   ohjelma tallentaa kuittauksen (tunniste ja aikaleima) ja merkitsee
   kuukauden ilmoitetuksi.

   Ilmoitus näkyy OmaVerossa tavallisesti vasta noin puolen vuorokauden
   kuluttua. Agentti voi kysyä sen tilan Verohallinnolta lähettämättä
   mitään. **Älä lähetä ilmoitusta uudelleen sen takia, että se ei vielä
   näy.** Mitä silloin tehdään, on kohdassa
   [Ilmoitus ei näy OmaVerossa](../troubleshooting.md#ilmoitus-ei-näy-omaverossa).

6. **Maksa vero itse**, jos maksettavaa on, OmaVeron viitteellä.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant vero-fi status
aggountant file-vat preview --period 2026-03
aggountant file-vat start < ilmoitus.json
aggountant file-vat approve-filing --run-id fv-… --answer approve
aggountant run show --run-id fv-…
aggountant period show --period 2026-03
aggountant file-vat status --period 2026-03
```

`file-vat start` saa Verohallinnon tilinumeron syötteen kentässä
`payee_iban`. Jos agentti kuittaa ALV-tilien merkitsemättömät viennit, syötteessä
on lisäksi kenttä `vat_reconciliation_acknowledgement`, jossa ovat perustelu
(`reason`) ja esikatselun luettelemat tositenumerot (`vouchers`).

Ennen hyväksyntää `file-vat start` lukee kauden Verohallinnolta. Jos luku
epäonnistuu tai kausi ei ole ilmoitettavissa, komento päättyy virheeseen,
joka kertoo syyn ja nimeää ajon. Mitään ei lähetetä, ja `run show` näyttää
ajon tilassa `needs_attention` ja `reconcile`-efektin hylättynä. Ohjelma ei
yritä uudelleen. Kun syy on korjattu, agentti aloittaa uuden ajon.

Ajo odottaa ensin hyväksyntää `ApproveFiling` ja maksettavassa ilmoituksessa
lopuksi hyväksyntää `ApprovePayment`. Hylkäys on `--answer reject`.

Kuittaus näkyy `approve-filing`- ja `run show` -vastauksissa
`submit-filing`-efektin kentissä `outcome.receipt.unique_identifier` ja
`outcome.receipt.accepted_timestamp`. Jos `outcome.state` on
`outcome_unknown`, `outcome.next_command` kertoo komennon, jolla tilanne
selvitetään.
</details>

## Valmis, kun

- Agentti kertoo Verohallinnon kuittauksen tunnisteen ja aikaleiman.
- Kuukauden tila näyttää `vat_filed true`.

## Korjaa jo annettu ilmoitus

Korvaava ilmoitus antaa kauden kaikki luvut uudelleen korjattuina, ja
Verohallinto käyttää sen jälkeen sitä.

1. **Korjaa ensin kirjanpito.** Ilmoitettu kuukausi on lukittu, joten agentti
   kirjaa [korjaustositteen](correct-voucher.md) seuraavalle avoimelle
   kuukaudelle. Kerro agentille, että tosite korjaa ilmoitetun kuukauden
   ALV-ilmoitusta. Agentti merkitsee sen tositteeseen sinetöitäessä
   (`corrects_vat_period`), ja ohjelma laskee tositteen veron silloin
   korjattavan kuukauden lukuihin eikä sen kuukauden ilmoitukseen, jolle
   tosite on kirjattu. Merkintää ei voi lisätä sinetöinnin jälkeen.

   Verohallinnon ohjeen mukaan taloudellisesti vähäisen virheen saa korjata
   myös kuluvan kauden ilmoituksella. Silloin agentti ei tee merkintää eikä
   korvaavaa ilmoitusta anneta. Valinta on sinun.

2. **Pyydä korvaava ilmoitus.**

   > Tee lokakuun ALV-ilmoituksesta korvaava ilmoitus.

   Agentti esikatselee kuukauden uudet luvut ja valmistelee korvaavan
   ilmoituksen. Kerro korjauksen syy: laskuvirhe tai ilmoituksen
   täyttövirhe, oikeuskäytännön muutos, verotarkastuksessa saatu ohjaus tai
   lain tulkintavirhe.

3. **Tarkista ja hyväksy.** Agentti näyttää koko ilmoituksen ja sen, mitkä
   luvut muuttuvat Verohallinnolla nyt olevaan ilmoitukseen nähden.
   Hyväksyntä koskee tätä sisältöä.

4. **Ohjelma lähettää korvaavan ilmoituksen** ja tallentaa sen kuittauksen
   alkuperäisen viereen. Molemmat säilyvät, ja uusin on voimassa.

5. **Tarkista verotili OmaVerosta.** Korvaava ilmoitus muuttaa kauden veroa
   erotuksen verran. Ohjelma ei laske eikä maksa erotusta, joten maksa lisää
   maksettava itse tai odota palautusta.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant voucher seal < korjaus-sinetointi.json
aggountant file-vat preview --period 2026-10
aggountant file-vat replace < korvaava.json
aggountant file-vat approve-filing --run-id fv-… --answer approve
aggountant file-vat status --period 2026-10
```

`file-vat replace` ottaa saman syötteen kuin `file-vat start` ja lisäksi
kentän `replacement_reasons`. Vastauksen `replacement.changes` luettelee
muuttuvat kentät.

`file-vat status` ja `file-vat preview` näyttävät kentässä `filings` kauden
kaikki ilmoitukset kuittauksineen.
</details>

Jos olet jo tehnyt korvaavan ilmoituksen OmaVerossa, pyydä agenttia kirjaamaan
se kuitteineen (`file-vat complete-manual --replacement`).

## Rajoitukset

- Kun veroa on maksettavaa, ajo avaa lopuksi maksun hyväksynnän. aggountantilla
  ei ole maksuyhteyttä, joten vaihe ei valmistu ja ajo jää näyttämään
  odottavaa tilaa. Ilmoitus on silti tehty. Palautukseen oikeuttava ilmoitus
  päättyy ilman maksuvaihetta.
- Suora ilmoittaminen toimii myös neljännesvuosikausille, mutta oikeassa
  käytössä sitä on koeteltu kuukausikausilla.
- Korvaavaa ilmoitusta ei ole vielä lähetetty Verohallinnon tuotantoon.

Rajoitukset ovat kokonaisuudessaan sivulla
[Mitä on valmista ja mitä ei](../reference/status.md#alv-ilmoitus).

## Jos jokin menee pieleen

- `VERO_NOT_READY`: valmius ei ole kunnossa. Esteet ja niiden korjaukset
  ovat sivulla [Vero.fi-valmiuden esteet](../reference/vero-fi-blockers.md).
- `VAT_FIGURES_MISMATCH`: annetut luvut eivät vastaa kirjanpitoa. Agentti
  käyttää esikatselun lukuja.
- `VAT_ACCOUNTS_UNRECONCILED`: ALV-tilillä on vientejä ilman ALV-merkintää,
  ja ilmoituksesta puuttuisi veroa. Mitään ei lähetetty. Jos vero kuuluu
  ilmoitukseen, tee
  [ilmoitus OmaVerossa korjatuin luvuin](file-vat-omavero.md#jos-jokin-menee-pieleen).
  Jos viennit kuuluvat oikeasti ilmoituksen ulkopuolelle, kerro syy
  agentille. Näin on esimerkiksi, kun Verohallinnon palautus on kirjattu
  tilille 2939 tai vienti on avaavan tositteen ALV-saldo. Agentti liittää
  ilmoitusajoon perustelun ja esikatselun luettelemat tositenumerot, ja
  perustelu tallentuu ajoon.
- `VAT_RECONCILIATION_ACK_MISMATCH`: perustelun tositteet eivät ole
  täsmälleen samat kuin esikatselussa, joten ohjelma kieltäytyy.
- `PERIOD_TRANSITION_CONFLICT`: kuukausi on jo ilmoitettu, eikä ohjelma
  ilmoita sitä uudelleen.
- `VERO_VAT_RETURN_NOT_PROCESSED`: Verohallinto ei ole vielä käsitellyt
  voimassa olevaa ilmoitusta, joten korvaavaa ilmoitusta ei voi lähettää.
  Yritä myöhemmin uudelleen.
- Jos yhteys katkesi tai palvelu vastasi virheellä kesken lähetyksen,
  [ilmoituksen lopputulos on epävarma](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).
  Korvaavan ilmoituksen epävarma lopputulos selvitetään samalla tavalla,
  eikä uutta korvaavaa lähetetä arvaamalla.
- `VAT_FILING_UNRESOLVED`: saman kuukauden aiempi ilmoitus on voinut mennä
  perille, joten mitään ei lähetetä. Agentti kysyy Verohallinnolta
  (`file-vat reconcile`). Jos ilmoitus löytyy, aiempi ajo päättyy
  ilmoitettuna. Jos vastaus on "ei löydy", agentti kysyy uudelleen
  aikaisintaan samaan kellonaikaan seuraavana pankkipäivänä. Toinen "ei
  löydy" sulkee aiemman ajon, ja kuukauden voi ilmoittaa uudella ajolla.
  Tähän voi mennä muutama päivä, ja syy on kohdassa
  [Epävarma lopputulos](../concepts/filings.md#epävarma-lopputulos). Jos
  aiempi ajo keskeytyi kesken lähetyksen, agentti kysyy ensin komennolla
  `file-vat status`, kuten kohta
  [ALV-ilmoitus keskeytyi lähetettäessä](../troubleshooting.md#alv-ilmoitus-keskeytyi-lähetettäessä)
  neuvoo.

## Katso myös

- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Tee ALV-ilmoitus OmaVerossa](file-vat-omavero.md)
- [Ota suora ALV-ilmoittaminen käyttöön](set-up-vero-api.md)
