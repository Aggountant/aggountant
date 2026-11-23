# Valmistele tilinpäätös

Kun tilikauden kaikki kuukaudet on suljettu, kokoat aineiston tilinpäätöksen
tekijälle ja kirjaat poistot, jaksotukset ja oikaisut tilinpäätösajolla.
Aineisto on kuukausiraportit ja sinetöidyt tositteet liitteineen. Virallinen
tilinpäätösasiakirja tehdään toistaiseksi aggountantin ulkopuolella,
tilitoimiston tai kirjanpitäjän kanssa tai itse.

## Ennen kuin aloitat

- Vuoden kaikki kuukaudet on kirjattu, suljettu ja ALV ilmoitettu.
- Tiedät, kuka tekee tilinpäätöksen ja missä muodossa hän haluaa aineiston.

## Näin teet sen

1. **Tarkista vuoden tila.**

   > Näytä vuoden 2026 kuukausien tila ja tarkista tositeketju.

   Agentti näyttää jokaisen kuukauden tilan ja tarkistaa ketjun. Jokaisen
   kuukauden pitää olla `sealed` ja `vat_filed true`. Tilinpäätösajo vaatii
   vain, että kuukaudet on suljettu, joten ALV-ilmoitukset tarkistat itse.

2. **Pyydä tilikauden viimeisen kuukauden raportti.**

   > Tee joulukuusta kuukausiraportti HTML-muodossa.

   Raportissa on tase tilikauden viimeisen kuukauden lopussa ja tuloslaskelma
   tilikauden alusta.

3. **Kokoa luovutettava aineisto.** Pyydä agenttia kokoamaan kuukausiraportit,
   tositeluettelo ja arkistoidut liitteet kansioon, jonka voit antaa
   tilinpäätöksen tekijälle. Kerro tekijälle samalla avaavat saldot ja muu,
   mitä hänen pitää tietää, esimerkiksi hankinnat, joista tehdään poistoja.

4. **Sovi tekijän kanssa, mitä tilinpäätösajoon annetaan.** Poistot,
   jaksotukset ja oikaisut kuuluvat päättyneelle vuodelle, jonka kuukaudet on
   jo suljettu. Tilinpäätösajo (`close-year`) kirjaa ne seuraavan vuoden
   ensimmäiselle avoimelle kaudelle ja kohdistaa ne päättyneelle vuodelle,
   joten ne näkyvät oikean vuoden tuloksessa. Sovi, antaako tekijä
   poistettavat hyödykkeet, jaksotukset ja oikaisut ajoa varten.

   Ensimmäisessä aggountantilla tehtävässä tilinpäätöksessä annat jokaisen
   hyödykkeen ja sen menojäännöksen vuoden alussa edellisestä
   tilinpäätöksestä. Kun vuosi sinetöidään, aggountant tallentaa hyödykkeet
   ja niiden menojäännöksen vuoden lopussa, ja seuraavan vuoden ajo jatkaa
   niistä. Silloin annat vain uudet hankinnat ja kerrot, mitkä hyödykkeet on
   myyty tai poistettu käytöstä.

5. **Tarkista poistolaskelma ennen ajoa.**

   > Näytä vuoden 2026 poistolaskelma: kone 2 000,00 euroa koneisiin ja
   > kalustoon.

   Agentti näyttää hyödykkeet, menetelmän, prosentin ja poiston. Mitään ei
   vielä kirjata.

6. **Käynnistä ajo ja hyväksy sen vaiheet.**

   > Tee vuoden 2026 tilinpäätösajo: kone 2 000,00 euroa koneisiin ja
   > kalustoon, ei jaksotuksia eikä oikaisuja.

   Ajo pyytää hyväksynnät vuorollaan, ja ennen jokaista agentti näyttää,
   mitä olet hyväksymässä:

   1. oikaisukirjaukset: lasketut poistot, jaksotukset ja oikaisut
   2. tase-erittelyt
   3. liitetiedot: poistomenetelmät ja -prosentit
   4. tilinpäätöslaskelmat: tuloslaskelma ja tase

   Hyväksyntä koskee täsmälleen sitä sisältöä, jonka agentti näytti.
   Ensimmäinen hyväksyntä sinetöi poisto- ja jaksotustositteet pysyvästi
   kirjanpitoon. Jokainen oikaisu sinetöidään omana tositteenaan ennen sitä,
   ja agentti pyytää hyväksyntäsi kullekin erikseen. Jos hylkäät jonkin
   vaiheen, ajo päättyy, mutta jo sinetöidyt tositteet jäävät kirjanpitoon.
   Viimeinen hyväksyntä sinetöi tilikauden, eikä sitä voi perua. Sen jälkeen
   ajo kirjoittaa hyväksytyt laskelmat, tase-erittelyt ja liitetiedot
   kirjanpitokodin tiedostoon `archive/<vuosi>/tilinpaatos.json`.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant period show --period 2026-12
aggountant fiscal-year show --year 2026
aggountant voucher verify
aggountant voucher list --posting-from 2026-01-01 --posting-to 2026-12-31
aggountant report statements --period 2026-12 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-12.html
aggountant close-year start --dry-run < tilinpaatos-2026.json
aggountant close-year start < tilinpaatos-2026.json
aggountant close-year seal-correction --run-id cy-… --correction <id>
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveAdjustments --answer approve
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveTaseErittelyt --answer approve
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveLiitetiedot --answer approve
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveStatements --answer approve
```
</details>

## Valmis, kun

- Tilinpäätöksen tekijällä on vuoden raportit, tositteet ja liitteet.
- Vuoden poistot, jaksotukset ja oikaisut on kirjattu tilinpäätösajolla ja
  tilikausi on `sealed`, tai olet sopinut tekijän kanssa toisin.

## Rajoitukset

- Ajo kohdistaa päättyneelle vuodelle vain omat kirjauksensa. Tosite, jonka
  kirjaat itse seuraavan vuoden kaudelle, näkyy seuraavan vuoden tuloksessa.
- Virallisen tilinpäätösasiakirjan laatiminen ja allekirjoitus, ilmoitus
  kaupparekisteriin ja yhtiön veroilmoitus tehdään aggountantin ulkopuolella.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#tilinpäätös).

## Jos jokin menee pieleen

- Jos ajo keskeytyy, agentti voi näyttää avoimen vaiheen myöhemmin
  uudelleen.
- Jos sisältö on muuttunut vaiheen avautumisen jälkeen, agentti ei voi
  näyttää sitä etkä voi hyväksyä sitä. Hylkää vaihe ja aloita ajo alusta.
- Jos annat edelliseltä vuodelta siirtyneelle hyödykkeelle eri luvun kuin
  sinetöity vuosi, ajo kieltäytyy.
- Jos tiedoston `tilinpaatos.json` kirjoitus epäonnistuu, agentti kertoo sen
  ja kirjoittaa tiedoston uudelleen jatkamalla ajoa. Kirjanpito on joka
  tapauksessa oikein.
- `PERIOD_NOT_SEALED`: seuraavan tilikauden kuukausien kuukausiraportti
  torjutaan, kunnes tämä vuosi on sinetöity, koska sen tase alkaa tämän
  vuoden loppusaldoista. Kirjaaminen, kuukausien sulku ja ALV-ilmoitukset
  uudella vuodella eivät odota. Tarkemmin sivulla
  [Katso, miten kuukausi meni](monthly-report.md#jos-jokin-menee-pieleen).
- Jos aiempi, kesken jäänyt tilinpäätösajo on jo kirjannut osan
  kirjauksista, ota uuteen ajoon samat hyödykkeet, jaksotukset ja oikaisut
  samoin summin. Muuten ajo ei käynnisty tai sinetöi vuotta ja nimeää
  puuttuvat kirjaukset. Hyväksymäsi sisällön pitää kattaa jokainen
  kirjanpidossa oleva tilinpäätöskirjaus, myös sellainen, jonka aiempi ajo
  kirjasi uudelle vuodelle mutta ei vielä kohdistanut päätettävään vuoteen
  ja joka muuten jäisi uuden vuoden tulokseen. Jos tällainen kirjaus oli
  virheellinen, ota se silti mukaan ja peru se oikaisulla uudessa ajossa.
  Kun yksi ajo on sinetöinyt vuoden, muut saman vuoden avoimet ajot voi vain
  hylätä.

## Katso myös

- [Kirjanpitäjälle](../concepts/for-bookkeepers.md)
- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
- [Katso, miten kuukausi meni](monthly-report.md)
