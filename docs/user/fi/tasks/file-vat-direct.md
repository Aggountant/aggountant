# Lähetä ALV-ilmoitus suoraan Verohallinnolle

Kun suora ilmoittaminen on otettu käyttöön, agentti lähettää suljetun
kuukauden ALV-ilmoituksen Verohallinnolle Vero API -rajapinnan kautta
yrityksesi omilla tunnuksilla, kun olet hyväksynyt sen. Lähetettyä ilmoitusta
ei voi perua.

## Ennen kuin aloitat

- Kuukausi on suljettu ([Sulje kuukausi](close-month.md)).
- Suora ilmoittaminen on käytössä, eli valmiuden tarkistus näyttää `ready`
  ([Ota suora ALV-ilmoittaminen käyttöön](set-up-vero-api.md)).
- Kuukautta ei ole vielä ilmoitettu Verohallinnolle millään tavalla.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tee maaliskuun ALV-ilmoitus suoraan Verohallinnolle.

2. **Agentti esikatselee luvut** suljetun kuukauden kirjanpidosta, kuten
   sivulla [Tee ALV-ilmoitus OmaVerossa](file-vat-omavero.md).

3. **Agentti valmistelee lähetyksen.** Se antaa ohjelmalle kuukauden, myynnin
   ja ostojen veron sekä Verohallinnon tilinumeron, joka näkyy OmaVerossa.
   Ohjelma laskee luvut vielä kerran kirjanpidosta ja kieltäytyy, jos ne
   eroavat. Se kieltäytyy myös, jos ALV-tilit eivät täsmää ilmoitukseen
   (`VAT_ACCOUNTS_UNRECONCILED`). Se tarkistaa valmiuden, kysyy kuukauden tilan Verohallinnolta ja
   jää odottamaan hyväksyntääsi. Mitään ei ole vielä lähetetty.

4. **Tarkista ja hyväksy.** Agentti näyttää yrityksen, kuukauden, eräpäivän,
   numeroidut kentät, nettosumman ja maksettavan määrän. Tarkista, että
   kentät ovat samat kuin esikatselussa ja että kuukausi on oikea.
   Hyväksyntä koskee juuri tätä sisältöä.

   > Hyväksyn ilmoituksen.

   Jos hylkäät, ajo päättyy, eikä mitään lähetetä. Uusi kierros alkaa
   esikatselusta.

5. **Ohjelma lähettää ilmoituksen.** Kun Verohallinto on vastaanottanut sen,
   ohjelma tallentaa kuittauksen (tunniste ja aikaleima) ja merkitsee
   kuukauden ilmoitetuksi.

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

Ilmoitusajon syötteessä Verohallinnon tilinumero on kentässä `payee_iban`.
Ajo odottaa ensin hyväksyntää `ApproveFiling` ja maksettavassa ilmoituksessa
lopuksi `ApprovePayment`. Hylkäys on `--answer reject`.
</details>

## Valmis, kun

- Agentti kertoo Verohallinnon kuittauksen tunnisteen ja aikaleiman.
- Kuukauden tila näyttää `vat_filed true`.

Ilmoitus näkyy OmaVerossa tavallisesti vasta noin puolen vuorokauden
kuluttua. Agentti voi kysyä sen tilan Verohallinnolta lähettämättä mitään.
**Älä lähetä sitä uudelleen sen takia, että se ei vielä näy:**
[Kun jokin menee vikaan](../troubleshooting.md#ilmoitus-ei-näy-omaverossa).

## Rajoitukset

- Kun veroa on maksettavaa, ajo avaa lopuksi maksun hyväksynnän. aggountantilla
  ei ole maksuyhteyttä, joten vaihe ei valmistu ja ajo jää näyttämään
  odottavaa tilaa. Ilmoitus on silti tehty. Palautukseen oikeuttava ilmoitus
  päättyy ilman maksuvaihetta.
- Suoraa ilmoittamista on koeteltu kuukausikausilla.
- Korvaavaa ilmoitusta aggountant ei tee.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#alv-ilmoitus).

## Jos jokin menee pieleen

- **`VERO_NOT_READY`.** Valmius ei ole kunnossa:
  [Vero.fi-valmiuden esteet](../reference/vero-fi-blockers.md).
- **`VAT_FIGURES_MISMATCH`.** Annetut luvut eivät vastaa kirjanpitoa. Agentti
  käyttää esikatselun lukuja.
- **`VAT_ACCOUNTS_UNRECONCILED`.** ALV-tilillä on vientejä ilman ALV-merkintää,
  ja ilmoituksesta puuttuisi veroa. Mitään ei lähetetty. Jos vero kuuluu
  ilmoitukseen, tee ilmoitus OmaVerossa korjatuin luvuin
  ([Tee ALV-ilmoitus OmaVerossa](file-vat-omavero.md#jos-jokin-menee-pieleen)).
  Jos viennit kuuluvat oikeasti ilmoituksen ulkopuolelle, esimerkiksi
  Verohallinnon palautus on kirjattu tilille 2939 tai vienti on avaavan
  tositteen ALV-saldo, kerro syy: agentti lisää
  ilmoitusajon syötteeseen kentän `vat_reconciliation_acknowledgement`, jossa
  ovat perustelu (`reason`) ja esikatselun luettelemat tositenumerot
  (`vouchers`). Perustelu tallentuu ajoon. Jos tositteet eivät ole täsmälleen
  samat, ohjelma kieltäytyy koodilla `VAT_RECONCILIATION_ACK_MISMATCH`.
- **`PERIOD_TRANSITION_CONFLICT`.** Kuukausi on jo ilmoitettu. Sitä ei
  ilmoiteta uudelleen.
- **Yhteys katkesi tai palvelu vastasi virheellä kesken lähetyksen.**
  [Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).

## Katso myös

- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Tee ALV-ilmoitus OmaVerossa](file-vat-omavero.md)
- [Ota suora ALV-ilmoittaminen käyttöön](set-up-vero-api.md)
