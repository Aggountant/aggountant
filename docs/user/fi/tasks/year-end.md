# Valmistele tilinpäätös

Tilinpäätös tehdään toistaiseksi aggountantin ulkopuolella, tilitoimiston tai
kirjanpitäjän kanssa tai itse. aggountantista saat sille aineiston:
kuukausiraportit ja sinetöidyt tositteet liitteineen. Tällä sivulla kokoat
aineiston, luovutat sen ja kirjaat tilinpäätöksen kirjaukset.

## Ennen kuin aloitat

- Vuoden kaikki kuukaudet on kirjattu, suljettu ja ALV ilmoitettu.
- Tiedät, kuka tekee tilinpäätöksen ja missä muodossa hän haluaa aineiston.

## Näin teet sen

1. **Tarkista vuoden tila.**

   > Näytä vuoden 2026 kuukausien tila ja tarkista tositeketju.

   Agentti näyttää jokaisen kuukauden tilan ja tarkistaa ketjun. Jokaisen
   kuukauden pitää olla `sealed` ja `vat_filed true`.

2. **Pyydä tilikauden viimeisen kuukauden raportti.**

   > Tee joulukuusta kuukausiraportti HTML-muodossa.

   Raportissa on tilikauden viimeisen kuukauden lopun tase ja tuloslaskelma
   tilikauden alusta.

3. **Kokoa luovutettava aineisto.** Pyydä agenttia kokoamaan kuukausiraportit,
   tositeluettelo ja arkistoidut liitteet kansioon, jonka voit antaa
   tilinpäätöksen tekijälle. Kerro samalla avaavat saldot ja muut asiat, jotka
   tekijän pitää tietää, esimerkiksi hankinnat, joista tehdään poistoja.

4. **Kirjaa tilinpäätöksen kirjaukset tilinpäätösajolla.** Poistot,
   jaksotukset ja oikaisut kuuluvat päättyneelle vuodelle, jonka kuukaudet on
   jo suljettu. Tilinpäätösajo (`close-year`) kirjaa ne seuraavan vuoden
   ensimmäiselle avoimelle kaudelle ja kohdistaa ne päättyneelle vuodelle, joten
   ne näkyvät oikean vuoden tuloksessa. Tavallisena tositteena seuraavan
   vuoden kaudelle kirjattuina ne näkyisivät seuraavan vuoden tuloksessa.
   Sovi tekijän kanssa, antaako hän poistettavat hyödykkeet, jaksotukset ja
   oikaisut ajoa varten.

   > Tee vuoden 2026 tilinpäätösajo: kone 2 000,00 euroa koneisiin ja
   > kalustoon, ei jaksotuksia eikä oikaisuja.

   Agentti näyttää lasketut poistot. Kun hyväksyt oikaisukirjaukset, pyydä
   joulukuun kuukausiraportti uudelleen: siinä tuloslaskelma ja tase ovat
   oikaisuineen. Hyväksy laskelmat vasta, kun olet katsonut sen. Viimeinen
   hyväksyntä sinetöi tilikauden, eikä sitä voi perua.

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
aggountant close-year start < tilinpaatos-2026.json
aggountant close-year seal-correction --run-id cy-… --correction <id>
aggountant close-year answer --run-id cy-… --gate ApproveAdjustments --answer approve
aggountant close-year answer --run-id cy-… --gate ApproveStatements --answer approve
```
</details>

## Valmis, kun

- Tilinpäätöksen tekijällä on vuoden raportit, tositteet ja liitteet.
- Vuoden poistot, jaksotukset ja oikaisut on kirjattu tilinpäätösajolla ja
  tilikausi on `sealed`, tai olet sopinut tekijän kanssa toisin.

## Rajoitukset

Tilinpäätösajon hyväksyntävaiheet eivät näytä laskelmia, joita hyväksyt, joten
katso tilikauden viimeisen kuukauden raportti ennen laskelmien hyväksyntää.
Ajo kohdistaa päättyneelle vuodelle vain omat kirjauksensa. Tilikauden
sinetöinti on peruuttamaton.
Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#tilinpäätös).

## Katso myös

- [Kirjanpitäjälle](../concepts/for-bookkeepers.md)
- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Katso, miten kuukausi meni](monthly-report.md)
