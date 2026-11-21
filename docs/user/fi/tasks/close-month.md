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
- Avoimia velvoitteita ei ole. Avoin velvoite estää sulun
  ([Kaudet, sulku ja tilikausi](../concepts/periods.md#velvoitteet)).
- ALV-tilit täsmäävät ALV-ilmoitukseen. ALV-ilmoituksen esikatselu toimii myös
  avoimelle kuukaudelle luonnoksena, jota ei voi ilmoittaa, ja näyttää, puuttuuko
  ALV-tilin vienniltä ALV-merkintä. Ennen sulkua tällainen tosite korjataan
  vielä samalle kuukaudelle; sulun jälkeen ilmoitus on tehtävä OmaVerossa
  korjatuin luvuin.

## Näin teet sen

1. **Pyydä agenttia.**

   > Sulje maaliskuu.

2. **Agentti tarkistaa kuukauden.** Se käy läpi yllä olevat ehdot ja kertoo,
   jos jokin puuttuu. Sulun esikatselu näyttää jo tässä vaiheessa, mitä sulku
   lukitsisi, muuttamatta mitään. Jos puutteita on, korjaa ne ensin. Agentti
   ei aloita sulkua, ennen kuin kuukausi on valmis tai olet päättänyt, että avoimeksi
   nimetty asia kirjataan myöhemmin.

3. **Agentti aloittaa sulun.** Kuukausi siirtyy tilaan `closing`. Siihen ei voi
   enää kirjata, ja sulku jää odottamaan hyväksyntääsi.

4. **Tarkista ja hyväksy.** Ohjelma kokoaa hyväksyttävän sisällön: kuukauden
   tositteiden määrän ja numerot, kuukauden tuloksen ja tuloksen tilikauden
   alusta, tilien saldot kuukauden lopussa (esimerkiksi pankkitili ja
   ostovelat) ja kuukauden avoimet velvoitteet. Agentti näyttää sen ja kertoo,
   täsmääkö pankkitili tiliotteeseen, täsmäävätkö ALV-tilit ilmoitukseen ja
   mitä jää avoimeksi. Kysy, jos se ei
   kerro. Hyväksy, kun tiedät, ettei kuukauteen tarvitse enää kirjata mitään.
   Hyväksynnän jälkeen kuukausi lukitaan pysyvästi.

   Jos edellistä tilikautta ei ole vielä suljettu, saldoissa ovat vain tämän
   tilikauden tapahtumat ilman avaavia saldoja, ja sisältö sanoo sen. Sulku ei
   odota tilinpäätöstä.

   Hyväksyntä koskee juuri näytettyä sisältöä. Jos kirjanpito muuttuu sen
   jälkeen (esimerkiksi edelliselle avoimelle kuukaudelle kirjataan tosite,
   joka muuttaa loppusaldoja), hyväksyntä hylätään koodilla
   `CLOSE_CONTENT_CHANGED`. Silloin sulku hylätään ja aloitetaan uudelleen,
   jotta hyväksyt sen, mitä kirjanpito nyt sanoo.

   > Hyväksyn sulun.

   Jos huomaat puutteen tässä vaiheessa, hylkää sulku. Kuukausi palaa tilaan
   `open`, ja voit kirjata puuttuvan asian ja aloittaa sulun alusta. Mitään ei
   poisteta.

   > Hylkään sulun, kuukaudesta puuttuu vielä kuitti.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant obligation list
aggountant obligation resolve --id <tunniste>
aggountant file-vat preview --period 2026-03
aggountant run preview close-period --period 2026-03
aggountant run start close-period --period 2026-03
aggountant run show --run-id cp-…
aggountant run answer --run-id cp-… --gate ApproveClose --answer approve
aggountant period show --period 2026-03
```

Velvoite merkitään ratkaistuksi (`obligation resolve`) vasta, kun asia on
oikeasti hoidettu. `run preview` ei muuta mitään; sen `ready` ja `blockers`
kertovat, voiko sulun aloittaa. Sulku odottaa hyväksyntää `ApproveClose`, ja
`run start` ja `run show` palauttavat sen sisällön kentässä `open_gate.content`
sekä vastauskomennon kentässä `next_command`. Vastauksen paikalla on
`'<approve|reject>'`: agentti vaihtaa sen tilalle päätöksesi mukaan `approve` tai
`reject`, ja paikkamerkki itsessään hylätään virheenä. Hylkäys on `--answer reject`, ja se
palauttaa kuukauden tilaan `open`. Neljännesvuosikaudella kausi annetaan muodossa `2026-Q1`.
Ennen sulkua `file-vat preview` vastaa `fileable: false` ja huomiokohdalla
`VAT_PERIOD_NOT_SEALED`; `vat_reconciliation.status` on oltava `reconciled`,
tai kohdan `VAT_ACCOUNTS_UNRECONCILED` luettelemien vientien on oltava sellaisia,
jotka eivät kuulu ilmoitukseen.
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

- Suljettua kuukautta ei voi avata uudelleen.
- Samana vuonna ei voi sulkea sekä kuukausia että neljännesvuosia.
  Kuukausi on suositeltu ja koeteltu vaihtoehto.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#tilikausi-ja-kaudet).

## Jos jokin menee pieleen

- **Huomasit virheen sulun jälkeen.** Kirjaa korjaus seuraavalle avoimelle
  kuukaudelle: [Korjaa virheellinen tosite](correct-voucher.md).
- **Kirjaus kieltäytyy koodilla `PERIOD_CLOSING` tai `PERIOD_SEALED`.**
  Kuukauden sulku on kesken tai tehty. Kirjaa seuraavalle kuukaudelle.
- **Kone kaatui tai yhteys katkesi sulun aikana.** Mitään ei menetetty, ja
  agentti jatkaa ajoa: [Kun jokin menee vikaan](../troubleshooting.md#ajo-keskeytyi).
- **Kuukausi jäi tilaan `closing`.** [Kun jokin menee vikaan](../troubleshooting.md#kuukausi-jäi-suljettavaksi).

## Katso myös

- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Kuukauden kirjanpito alusta loppuun](monthly-routine.md)
