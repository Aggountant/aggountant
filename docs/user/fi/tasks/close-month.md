# Sulje kuukausi

Kun kuukauden tapahtumat on kirjattu, kuukausi suljetaan. Sen jälkeen
kuukaudelle ei voi enää kirjata, ja ALV-ilmoitus tehdään sen luvuista. Sulkua
ei voi perua, joten tee se vasta, kun kuukausi on valmis.

## Ennen kuin aloitat

- Tiliotteen jokainen tapahtuma on kirjattu tai nimetty avoimeksi
  ([Tuo pankin tiliote](import-bank-statement.md)).
- Kuitit ja laskut on kirjattu ja liitetty tositteisiin
  ([Kirjaa kuitit ja ostolaskut](record-receipts.md)).
- Kuukausiraportin pankkitilin saldo on sama kuin tiliotteen loppusaldo
  ([Katso, miten kuukausi meni](monthly-report.md)).
- Avoimia velvoitteita ei ole, sillä avoin velvoite estää sulun
  ([Kaudet, sulku ja tilikausi](../concepts/periods.md#velvoitteet)).
- ALV-tilit täsmäävät ALV-ilmoitukseen. ALV-ilmoituksen esikatselun saa myös
  avoimesta kuukaudesta luonnoksena, jota ei voi ilmoittaa, ja se näyttää,
  puuttuuko ALV-tilin vienniltä ALV-merkintä. Ennen sulkua sellainen tosite
  korjataan vielä samalle kuukaudelle. Sulun jälkeen ilmoitus on tehtävä
  OmaVerossa korjatuin luvuin.

## Näin teet sen

1. **Pyydä agenttia.**

   > Sulje maaliskuu.

2. **Agentti tarkistaa kuukauden.** Se käy läpi yllä olevat ehdot ja kertoo,
   mitä puuttuu. Sulun esikatselu näyttää, mitä sulku lukitsisi, eikä muuta
   vielä mitään. Korjaa puutteet ensin: agentti aloittaa sulun vasta, kun
   kuukausi on valmis tai olet päättänyt, että avoimeksi nimetty asia
   kirjataan myöhemmin.

3. **Agentti aloittaa sulun.** Kuukausi siirtyy tilaan `closing`. Siihen ei voi
   enää kirjata, ja sulku jää odottamaan hyväksyntääsi.

4. **Tarkista ja hyväksy.** Ohjelma kokoaa sisällön, jonka hyväksyt: kuukauden
   tositteiden määrän ja numerot, kuukauden tuloksen ja tuloksen tilikauden
   alusta sekä tilien saldot kuukauden lopussa (esimerkiksi pankkitili ja
   ostovelat). Agentti näyttää sen ja kertoo, täsmääkö pankkitili
   tiliotteeseen, täsmäävätkö ALV-tilit ilmoitukseen ja mitä jää avoimeksi. Kysy, jos se ei kerro. Hyväksy, kun tiedät, ettei
   kuukauteen tarvitse enää kirjata mitään, sillä hyväksyntä lukitsee
   kuukauden pysyvästi.

   > Hyväksyn sulun.

   Jos huomaat tässä vaiheessa puutteen, hylkää sulku. Kuukausi palaa tilaan
   `open`, mitään ei poisteta, ja voit kirjata puuttuvan asian ja aloittaa
   sulun alusta.

   > Hylkään sulun, kuukaudesta puuttuu vielä kuitti.

   Jos edellistä tilikautta ei ole vielä suljettu, saldoissa ovat vain tämän
   tilikauden tapahtumat ilman avaavia saldoja, ja sisältö sanoo sen. Sulku ei
   odota tilinpäätöstä.

   Hyväksyt juuri sen sisällön, jonka näit. Jos kirjanpito muuttuu sen jälkeen,
   esimerkiksi aiemmalle avoimelle kuukaudelle kirjataan tosite, joka muuttaa
   loppusaldoja, hyväksyntä hylätään koodilla `CLOSE_CONTENT_CHANGED`. Silloin
   sulku hylätään ja aloitetaan uudelleen, ja hyväksyt sen, mitä kirjanpito
   nyt sanoo.

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
oikeasti hoidettu.

Ennen sulkua `file-vat preview` vastaa `fileable: false` ja huomiokohdalla
`VAT_PERIOD_NOT_SEALED`. Sen `vat_reconciliation.status` on oltava
`reconciled`, tai kohdan `VAT_ACCOUNTS_UNRECONCILED` luettelemien vientien on
oltava sellaisia, jotka eivät kuulu ilmoitukseen.

`run preview` ei muuta mitään. Sen `ready` ja `blockers` kertovat, voiko sulun
aloittaa.

Sulku odottaa hyväksyntää `ApproveClose`. `run start` ja `run show` palauttavat
hyväksyttävän sisällön kentässä `open_gate.content` ja vastauskomennon kentässä
`next_command`. Komennossa vastauksen paikalla on `'<approve|reject>'`, jonka
tilalle agentti vaihtaa päätöksesi mukaan `approve` tai `reject`. Paikkamerkki
sellaisenaan hylätään virheenä. `--answer reject` palauttaa kuukauden tilaan
`open`.

Neljännesvuosikaudella kausi annetaan muodossa `2026-Q1`.
</details>

## Valmis, kun

Agentti näyttää kuukauden tilan, ja se on `sealed`:

```
period "2026-03" — "sealed" (…, vat_filed false)
```

`vat_filed false` tarkoittaa, että kuukauden ALV-ilmoitus on vielä tekemättä.
Tee se seuraavaksi [OmaVerossa](file-vat-omavero.md) tai
[suoraan Verohallinnolle](file-vat-direct.md).

## Rajoitukset

- Suljettua kuukautta ei voi avata uudelleen.
- Samana vuonna ei voi sulkea sekä kuukausia että neljännesvuosia.
  Kuukausi on suositeltu ja koeteltu vaihtoehto.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#tilikausi-ja-kaudet).

## Jos jokin menee pieleen

- Jos huomaat virheen sulun jälkeen, kirjaa korjaus seuraavalle avoimelle
  kuukaudelle: [Korjaa virheellinen tosite](correct-voucher.md).
- `PERIOD_CLOSING`: kuukauden sulku odottaa hyväksyntääsi. Jos kirjaus
  kuuluu tälle kuukaudelle, hylkää sulku, kirjaa ja aloita sulku alusta.
  Muuten kirjaa seuraavalle kuukaudelle.
- `PERIOD_SEALED`: kuukausi on suljettu. Kirjaa seuraavalle avoimelle
  kuukaudelle.
- Jos kone kaatui tai yhteys katkesi sulun aikana, mitään ei menetetty, ja
  agentti jatkaa ajoa: [Ajo keskeytyi](../troubleshooting.md#ajo-keskeytyi).
- Jos kuukausi jäi tilaan `closing`, katso
  [Kuukausi jäi suljettavaksi](../troubleshooting.md#kuukausi-jäi-suljettavaksi).

## Katso myös

- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Kuukauden kirjanpito alusta loppuun](monthly-routine.md)
