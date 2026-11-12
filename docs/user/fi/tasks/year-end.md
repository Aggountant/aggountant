# Valmistele tilinpäätös

Tilinpäätös tehdään toistaiseksi aggountantin ulkopuolella, tilitoimiston tai
kirjanpitäjän kanssa tai itse. aggountantista saat sille aineiston:
kuukausiraportit ja sinetöidyt tositteet liitteineen. Tällä sivulla kokoat
aineiston ja luovutat sen.

## Ennen kuin aloitat

- Vuoden kaikki kuukaudet on kirjattu, suljettu ja ALV ilmoitettu.
- Tiedät, kuka tekee tilinpäätöksen ja missä muodossa hän haluaa aineiston.

## Näin teet sen

1. **Tarkista vuoden tila.**

   > Näytä vuoden 2026 kuukausien tila ja tarkista tositeketju.

   Agentti näyttää jokaisen kuukauden tilan ja tarkistaa ketjun. Jokaisen
   kuukauden pitää olla `sealed` ja `vat_filed true`.

2. **Pyydä joulukuun raportti.**

   > Tee joulukuusta kuukausiraportti HTML-muodossa.

   Raportissa on joulukuun lopun tase ja tuloslaskelma vuoden alusta.

3. **Kokoa luovutettava aineisto.** Pyydä agenttia kokoamaan kuukausiraportit,
   tositeluettelo ja arkistoidut liitteet kansioon, jonka voit antaa
   tilinpäätöksen tekijälle. Kerro samalla avaavat saldot ja muut asiat, jotka
   tekijän pitää tietää, esimerkiksi hankinnat, joista tehdään poistoja.

4. **Tilinpäätöksen tekijä** laatii poistot, jaksotukset, tilinpäätöslaskelmat
   ja liitetiedot. Nämä kirjaukset kuuluvat päättyneelle vuodelle, jonka
   kuukaudet on jo suljettu, eikä aggountantissa ole vielä tapaa kirjata niitä
   sinne. Jos ne kirjattaisiin seuraavan vuoden ensimmäiselle kuukaudelle, ne
   näkyisivät väärän vuoden tuloksessa. Sovi tekijän kanssa, miten ne
   huomioidaan.

Myös virallisen tilinpäätösasiakirjan laatiminen ja allekirjoitus, ilmoitus
kaupparekisteriin ja yhtiön veroilmoitus tehdään aggountantin ulkopuolella.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant period show --period 2026-12
aggountant fiscal-year show --year 2026
aggountant voucher verify
aggountant voucher list --posting-from 2026-01-01 --posting-to 2026-12-31
aggountant report statements --period 2026-12 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-12.html
```
</details>

## Valmis, kun

- Tilinpäätöksen tekijällä on vuoden raportit, tositteet ja liitteet.
- Olet sopinut tekijän kanssa, miten tilinpäätöskirjaukset huomioidaan.

## Rajoitukset

Ohjelmassa on tilinpäätösajo (`close-year`), mutta **älä käytä sitä oikeaan
tilinpäätökseen**. Siinä vuoden lopun poistot ja jaksotukset puuttuvat
suljettavan vuoden laskelmista, ja hyväksyntävaiheet eivät näytä laskelmia,
joita hyväksyt. Tilikauden sinetöinti on peruuttamaton.
Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#tilinpäätös).

## Katso myös

- [Kirjanpitäjälle](../concepts/for-bookkeepers.md)
- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Katso, miten kuukausi meni](monthly-report.md)
