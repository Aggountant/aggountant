<!--
Näytesivu suunnitelmasta design.md: sivutyyppi Tehtävä (how-to).
Tuleva paikka: docs/user/fi/tasks/close-month.md.
Korvaa nykyisen luvun 4 kohdan "Kauden sulku". Komennot ja tulosteet on
tarkistettu aggountant 0.1.0:lla.
-->

# Sulje kuukausi

Kun kuukauden tapahtumat on kirjattu, kuukausi suljetaan. Sulun jälkeen
kuukaudelle ei voi enää kirjata, ja ALV-ilmoitus tehdään sen luvuista. Sulkua
ei voi perua, joten tee se vasta, kun kuukausi on valmis.

## Ennen kuin aloitat

- Tiliotteen jokainen tapahtuma on kirjattu tai nimetty avoimeksi
  ([Tuo pankin tiliote](import-bank-statement.md)).
- Kuitit ja laskut on kirjattu ja liitetty tositteisiin
  ([Kirjaa kuitit ja ostolaskut](record-receipts.md)).
- Kuukausiraportin pankkitilin saldo on sama kuin tiliotteen loppusaldo
  ([Katso, miten kuukausi meni](monthly-report.md)).
- Avoimia velvoitteita ei ole. Avoin velvoite estää sulun.

## Näin teet sen

1. **Pyydä agenttia.**

   > Sulje maaliskuu.

2. **Agentti tarkistaa kuukauden.** Se käy läpi yllä olevat ehdot ja kertoo,
   jos jokin puuttuu. Jos puutteita on, korjaa ne ensin. Agentti ei aloita
   sulkua, ennen kuin kuukausi on valmis tai olet päättänyt, että avoimeksi
   nimetty asia kirjataan myöhemmin.

3. **Agentti aloittaa sulun.** Kuukausi siirtyy tilaan `closing`. Siihen ei voi
   enää kirjata, ja sulku jää odottamaan hyväksyntääsi.

4. **Tarkista ja hyväksy.** Agentin pitää kertoa, mikä kuukausi suljetaan,
   montako tositetta siinä on ja täsmääkö pankkitilin saldo. Kysy, jos se ei
   kerro. Hyväksy, kun tiedät, ettei
   kuukauteen tarvitse enää kirjata mitään. Hyväksynnän jälkeen kuukausi
   lukitaan pysyvästi.

   > Hyväksyn sulun.

   **Älä hylkää sulkua.** Jos huomaat puutteen tässä vaiheessa, hyväksy sulku
   silti ja kirjaa puuttuva asia seuraavalle kuukaudelle. Hylätty sulku jättää
   tässä versiossa kuukauden pysyvästi tilaan `closing`, jolloin siihen ei voi
   kirjata eikä sitä voi sulkea.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant obligation list
aggountant run start close-period --period 2026-03
aggountant run answer --run-id cp-… --gate ApproveClose --answer approve
aggountant period show --period 2026-03
```

Neljännesvuosikaudella kausi annetaan muodossa `2026-Q1`.
</details>

## Valmis, kun

Agentti näyttää kuukauden tilan, ja tila on `sealed`:

```
period "2026-03" — "sealed" (…, vat_filed false)
```

`vat_filed false` tarkoittaa, että kuukauden ALV-ilmoitus on vielä tekemättä.
Seuraava vaihe on [ALV-ilmoitus OmaVerossa](file-vat-omavero.md) tai
[suoraan Verohallinnolle](file-vat-direct.md).

## Rajoitukset

- Suljettua tai suljettavana olevaa kuukautta ei voi avata uudelleen.
- Samana vuonna ei voi sulkea sekä kuukausia että neljännesvuosia.
  Kuukausi on suositeltu ja koeteltu vaihtoehto.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md).

## Jos jokin menee pieleen

- **Huomasit virheen sulun jälkeen.** Kirjaa korjaus seuraavalle avoimelle
  kuukaudelle: [Korjaa virheellinen tosite](correct-voucher.md).
- **Kirjaus kieltäytyy koodilla `PERIOD_CLOSING` tai `PERIOD_SEALED`.**
  Kuukauden sulku on kesken tai tehty. Kirjaa seuraavalle kuukaudelle.
- **Kone kaatui tai yhteys katkesi sulun aikana.** Mitään ei menetetty, ja
  agentti jatkaa ajoa: [Kun jokin menee vikaan](../troubleshooting.md#ajo-keskeytyi).
- **Sulku on hylätty.** [Kun jokin menee vikaan](../troubleshooting.md#kuukausi-jai-suljettavaksi).

## Katso myös

- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Kuukauden kirjanpito alusta loppuun](monthly-routine.md)
