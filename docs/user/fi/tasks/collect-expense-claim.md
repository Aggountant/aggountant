# Kokoa kululasku

Kululasku kootaan aggountantissa luonnokseksi matka ja kuitti kerrallaan,
päivien tai viikkojen mittaan. Luonnoksessa voi olla yksi matka tai koko
kuukauden matkat ja kuitit. Jokaisen lisäyksen jälkeen aggountant kertoo,
mitä luonnoksesta vielä puuttuu, ja laskee matkojen verovapaat korvaukset.
Luonnos ei kirjaa mitään kirjanpitoon.

## Ennen kuin aloitat

- Kululaskun tekijän suhde yhtiöön ja varsinainen työpaikka on kirjattu:
  [Kirjaa varsinainen työpaikka](record-regular-workplace.md). Ilman niitä
  matkasta ei lasketa verovapaita korvauksia eikä matkan kuluja.
- Tiedät matkoista sen, mitä matkalaskussa pitää lain mukaan näkyä: matkan
  tarkoituksen, kohteen, lähtö- ja paluuajan, matkustustavan ja omalla autolla
  ajetuista matkoista reitin ja kilometrit.
- Kuitit ovat tallessa tiedostoina tai kuvina.

## Näin teet sen

1. **Kerro agentille matkasta tai kuitista.**

   > Aloita Maija Meikäläisen maaliskuun kululasku. Hän kävi 10.3. asiakkaan
   > luona Tampereella omalla autollaan, lähti kotoa klo 7.30 ja palasi
   > klo 19.15. Asiakas tarjosi lounaan. Pysäköinti maksoi 12,40 €, kuitti
   > on liitteenä.

2. **Agentti** lisää matkan ja kuitin luonnokseen. Kuitti tallennetaan
   yrityskodin liitevarastoon heti, ja luonnokseen jää sen tunniste. Jos agentti
   ehdottaa ajoja kalenterista tai sähköpostista, ajo tulee luonnokseen vasta,
   kun vahvistat sen.

3. **Vastaa avoimiin kysymyksiin.** aggountant listaa jokaisen lisäyksen
   jälkeen, mitä puuttuu, ja agentti kysyy ne sinulta. Tyhjä vastaus ei
   tarkoita "ei mitään": jos et kerro ilmaisista aterioista, päiväraha olisi
   liian suuri. Siksi agentti kysyy, vaikka vastaus olisi nolla. Kysymyksiä
   ovat esimerkiksi matkan tarkoitus, mistä matka alkoi, montako ilmaista
   ateriaa sait ja maksoiko yritys jonkin ateriasi kunakin
   matkavuorokautena, oliko auto omasi tai hallinnassasi, ja ulkomailla, missä
   maassa kukin matkavuorokausi päättyi.

4. **Katso luonnos** milloin tahansa: pyydä agenttia näyttämään kululasku.
   Näet matkat, matkavuorokaudet ja päivärahat, ajot ja kilometrikorvaukset,
   kuitit ja loppusumman. Tarkista erityisesti ateriat ja kilometrit, koska
   aggountant laskee summat niistä.

### Kysymykset, kieltäytymiset ja tarkistukset

Luonnos kertoo kolmenlaista asiaa. **Avoin kysymys** on vastaus, jota et ole
vielä antanut. **Kieltäytyminen** on asia, jota aggountant ei hyväksy
sellaisenaan. Kumpikin estää kululaskun hyväksymisen, mutta ei luonnoksen
tallentamista. **Tarkistus** ei estä mitään, mutta se näkyy kululaskussa,
ja agentti kysyy siitä.

| Koodi | Mitä se tarkoittaa | Mitä teet |
|---|---|---|
| `TRIP_STATUTORY_FIELD_MISSING` | matkalta puuttuu tarkoitus, kohde, matkustustapa tai (ajoista) reitti | kerro puuttuva tieto |
| `TRIP_START_PLACE_UNANSWERED` | ei tiedetä, alkoiko matka kotoa, työpaikalta vai muualta | kerro, mistä lähdit |
| `TRIP_DESTINATION_PLACES_UNANSWERED` | ei tiedetä, menikö matka rekisteriin kirjattuun paikkaan, kuten toissijaiselle työpaikalle | kerro, minne matka meni; rekisteri luokittelee matkan siitä |
| `TRAVEL_FREE_MEALS_UNANSWERED`, `TRAVEL_COMPANY_PAID_MEALS_UNANSWERED` | matkavuorokaudelta puuttuu ilmaisten tai yrityksen maksamien aterioiden tieto | kerro määrä tai summat, myös nolla |
| `DRIVE_VEHICLE_OWNERSHIP_UNANSWERED` | ei tiedetä, oliko ajoneuvo omasi tai hallinnassasi | kerro; kilometrikorvaus on verovapaa vain silloin |
| `LINE_RECEIPT_MISSING` | kuittiriviltä puuttuu kuitti | anna kuitti tai kerro, miksi sitä ei ole |
| `TRIP_OVERLAP` | matka on päällekkäin toisen saman henkilön matkan kanssa | korjaa ajat tai poista toinen matka |
| `RECEIPT_ALREADY_BOOKED`, `RECEIPT_DUPLICATE` | sama kuitti on jo kirjattu tositteeksi tai toiselle riville | poista rivi; kuitti kirjataan kerran |
| `ARTIFACT_MISSING` | kuitti- tai liitetiedostoa ei löydy yrityskodin liitevarastosta, esimerkiksi palautetusta varmuuskopiosta | tallenna tiedosto uudelleen ja korjaa rivi |
| `LINE_TRIP_COSTS_NOT_TAX_FREE` | kuitti kuuluu matkaan, joka ei rekisterin mukaan ole työmatka | katso matkan oma kieltäytyminen |
| `TRIP_RECORDED_AFTERWARDS` | matka kirjattiin vasta matkaa seuraavan kuukauden jälkeen | ei estä mitään; kululasku näyttää merkinnän |
| `DRIVE_KM_BELOW_DISTANCE`, `DRIVE_KM_ABOVE_ROUTE` | ajetut kilometrit poikkeavat paljon matkan etäisyydestä | tarkista kilometrit tai kerro reitti |
| `RECEIPT_OUTSIDE_TRIPS`, `RECEIPT_POSSIBLE_DUPLICATE` | kuitti on päivätty minkään matkan ulkopuolelle tai näyttää toisen kuitin kaksoiskappaleelta | tarkista, että kulu kuuluu yritykselle ja on mukana kerran |

Matkan korvausten laskennan omat kieltäytymiset, kuten
`ALLOWANCE_ABOVE_MAXIMUM`, on selitetty sivulla
[Laske matkan verovapaat korvaukset](compute-travel-allowances.md#kun-laskenta-kieltäytyy).

### Jälkikäteen kirjattu matka

Matka, joka kirjataan luonnokseen vasta matkan päättymiskuukautta seuraavan
kuukauden jälkeen, merkitään jälkikäteen kirjatuksi. Kululaskut kootaan
yleensä kuukausittain, joten seuraavan kuukauden loppuun mennessä kirjattu
matka on tavallista. Myöhemmin kirjattu matka perustuu muistiin tai
jälkikäteen koottuun selvitykseen. Se sallitaan, mutta merkintä näkyy
kululaskussa. Laki ei anna rajalle lukua; raja seuraa kuukausittaista
kululaskurytmiä.

### Luonnoksen hylkääminen

Jos luonnosta ei tarvita, pyydä agenttia hylkäämään se ja kerro syy. Luonnos
poistetaan; sen tunniste, viimeisen version tiiviste ja syy jäävät
lokiin, sisältö ei.

<details>
<summary>Komennot, jotka agentti ajaa</summary>

```sh
aggountant --json artifact attach --file kuitti.pdf
aggountant --json claim trip add --new-for maija < matka.json
aggountant --json claim line add --claim maija-2026-03-10 < kuitti.json
aggountant --json claim trip revise --claim maija-2026-03-10 --trip t1 < matka.json
aggountant claim show --claim maija-2026-03-10 --format markdown
aggountant --json claim discard --claim maija-2026-03-10 --reason "Tehty uudelleen"
```

`claim trip` ja `claim line` lisäävät, korjaavat ja poistavat matkoja ja
kuittirivejä, ja jokainen kirjoitus vastaa luonnoksen avoimilla kysymyksillä.
`claim show` näyttää luonnoksen JSON- tai Markdown-muodossa, ja `claim list`
listaa avoimet luonnokset. Syötteiden kentät näkyvät komennoilla
`aggountant claim trip add --help` ja `aggountant claim line add --help`.
</details>

## Valmis, kun

- Luonnoksessa ei ole avoimia kysymyksiä eikä kieltäytymisiä
  (`ready_for_approval` on `true`).
- Olet katsonut kululaskun ja sen tarkistukset.

## Rajoitukset

- Kululaskua ei vielä voi hyväksyä eikä kirjata aggountantissa. Valmiin
  luonnoksen korvaukset kirjataan toistaiseksi käsin, kuten sivulla
  [Laske matkan verovapaat korvaukset](compute-travel-allowances.md) kerrotaan.
- Luonnos on yrityskodin `intake/expense-claims/`-kansiossa, jota git ei
  seuraa. Se ei ole salattu, kuten ei muukaan henkilötietovarasto tässä
  versiossa.
- [Mitä on valmista ja mitä ei](../reference/status.md#kuitit-ja-kirjaukset)
  kertoo tilanteen.

## Katso myös

- [Laske matkan verovapaat korvaukset](compute-travel-allowances.md)
- [Kirjaa varsinainen työpaikka](record-regular-workplace.md)
- [Sanasto](../reference/glossary.md)
