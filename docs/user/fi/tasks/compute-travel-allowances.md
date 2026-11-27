# Laske matkan verovapaat korvaukset

Kun olet tehnyt työmatkan, aggountant laskee siitä verovapaat
enimmäiskorvaukset: kilometrikorvauksen, päivärahat, ateriakorvauksen ja
yömatkarahan. Laskenta on esikatselu. Se ei kirjaa mitään, ei tallenna matkaa
eikä ilmoita mitään Tulorekisteriin.

## Ennen kuin aloitat

- Tiedät matkan lähtö- ja paluuajan, minne matka suuntautui ja kuinka kaukana
  työntekemispaikka oli kodista ja varsinaisesta työpaikasta.
- Tiedät jokaiselta matkavuorokaudelta, montako ilmaista ateriaa sait ja
  maksoiko yritys jonkin ateriasi.
- Jos ajoit omalla autolla, tiedät ajojen päivät ja kilometrit.
- Yrityskotia ei tarvita. Laskennan voi kokeilla myös keksityllä matkalla.

## Näin teet sen

1. **Kerro agentille matkasta.**

   > Laske Maija Meikäläisen Tampereen matkan korvaukset: lähtö kotoa 10.3.
   > klo 7.30, paluu klo 19.15, ajoi omalla autolla 344,6 km. Tampere on
   > Maijalle erityinen työntekemispaikka, 172 km kotoa. Asiakas tarjosi
   > lounaan.

2. **Agentti** kokoaa matkan tiedot ja laskee ne. Jos laskennalta puuttuu
   vastaus, agentti kysyy sen: ensimmäinen laskenta näyttää matkavuorokaudet,
   ja jokaiselta päivärahapäivältä tarvitaan ilmaisten aterioiden määrä.
   Tyhjä vastaus ei tarkoita, ettei aterioita ollut, joten agentti kysyy,
   vaikka vastaus olisi nolla. Ulkomaanmatkalla agentti kysyy myös, missä
   maassa kukin matkavuorokausi päättyi.

3. **Tarkista tulos.** Katso matkavuorokaudet ja niiden kellonajat,
   päivärahan peruste (koko- tai osapäiväraha, ulkomaanpäiväraha) ja
   puolitukset ilmaisten aterioiden takia, kilometrit ja summat. Esimerkin
   matkasta tulee yksi kokopäiväraha 54,00 € (yksi ilmainen ateria ei puolita
   kokopäivärahaa) ja kilometrikorvaus 189,53 €, Tulorekisteriin 345 km.
   Tulos kertoo myös, minkä vuoden Verohallinnon päätöksen mukaan summat on
   laskettu.

Tulos näyttää lisäksi Tulorekisterin rivit (kilometrikorvaus 311 ja päiväraha
331) ja ehdotetut viennit (7300 Matkakulut, 7350 Päivärahat ja vastatili 2740
Muut lyhytaikaiset velat) kuukaudelta, jona matka päättyi. Ne ovat ehdotuksia.
Kirjaa korvaukset toistaiseksi erikoiskirjauksena, kuten
[matkaennakon tilityksessä](record-travel-advance.md), ja ilmoita ne
[Tulorekisteriin](report-travel-allowances.md).

### Kun laskenta kieltäytyy

aggountant ei arvaa. Jos matkasta puuttuu tieto tai tilanne on sellainen, jota
Verohallinnon päätös ja ohje eivät ratkaise, tuloksessa on kieltäytyminen ja
sen syy. Kieltäytynyttä laskelmaa ei voi hyväksyä, mutta muut summat näkyvät
silti. Tavallisimmat:

| Koodi | Mitä se tarkoittaa | Mitä teet |
|---|---|---|
| `TRAVEL_FREE_MEALS_UNANSWERED` | päivältä puuttuu ilmaisten aterioiden määrä | kerro määrä, myös nolla |
| `ALLOWANCE_ABOVE_MAXIMUM` | maksettava korvaus on suurempi kuin verovapaa enimmäismäärä | maksa enintään enimmäismäärä; ylimenevä osa olisi palkkaa, jota aggountant ei vielä käsittele |
| `TRAVEL_RATES_MISSING` | matkan vuodelle ei ole Verohallinnon päätöstä tässä versiossa | päivitä aggountant |
| `TRAVEL_COUNTRY_INVALID` | maakoodi ei ole ISO 3166 -koodi tai kaupunkia ei ole päätöksessä | korjaa koodi; tuntematon koodi ei koskaan saa "muun maan" päivärahaa |
| `TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM` | ateriakorvausta pyydettiin matkalta, josta maksetaan päiväraha | valitse jompikumpi; ateriakorvaus kuuluu vain matkoille ilman päivärahaa |
| `TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING` | yömatkarahaa pyydettiin yöltä, jonka majoituksen yritys maksoi | jätä yömatkaraha pois siltä yöltä |

Muut kieltäytymiset koskevat tilanteita, joita lähteet eivät ratkaise: sama
päivä, jolla on sekä ilmainen että yrityksen maksama ateria, ulkomaanmatka,
joka kulkee Suomen kautta, yömatkaraha ulkomaanmatkalta, matka vapaa-ajan asunnolta, työpaikan muuttuminen
kesken matkan, rajanylitysraha, komennus, ulkomailla työskentely ja erityisalan päivittäiset
matkat vaihtuvalle työmaalle. Kysy
niistä kirjanpitäjältäsi.

### Säännöt, joita lähteet eivät kerro

Näissä tilanteissa aggountant noudattaa tuoteomistajan päätöstä, ja tulos
mainitsee säännön, kun summa riippuu siitä:

- Vuodenvaihteen yli menevä matkavuorokausi saa kahden vuoden summista
  pienemmän.
- Matkavuorokausi on 24 tuntia kulunutta aikaa, joten kesäajan alku tai loppu
  lasketaan niin kuin se tapahtui.
- Summat pyöristetään sentteihin kerran koko laskelmasta, puolikkaat poispäin
  nollasta.
- Ilmainen ateria on yksi ateria hinnasta riippumatta. Yrityksen maksama
  oma ateria vähennetään päivärahasta, mutta päiväraha ei koskaan jää alle
  puolen.
- Kun matkaan yhdistetään vapaa-aikaa, päivärahat lasketaan kertomastasi
  työajasta ja matka-ajasta.

<details>
<summary>Komento, jonka agentti ajaa</summary>

```sh
aggountant --json claim compute < matka.json
```

`claim compute` lukee matkat JSON-muodossa ja tulostaa laskelman. Se ei
tarvitse yrityskotia eikä kirjoita mitään. Syötteen kentät näkyvät komennolla
`aggountant claim compute --help`.
</details>

## Valmis, kun

- Laskelmassa ei ole kieltäytymisiä (`approvable` on `true`).
- Tiedät matkan päivärahat, kilometrikorvauksen ja Tulorekisteriin
  ilmoitettavat kilometrit.

## Rajoitukset

- Matkaa ei vielä tallenneta, eikä kululaskua voi hyväksyä aggountantissa:
  korvaukset kirjataan ja ilmoitetaan käsin.
- Varsinaisen työpaikan rekisteriä ei vielä ole. Agentti kertoo laskennalle,
  onko matkan kohde erityinen työntekemispaikka, toissijainen työpaikka vai
  varsinainen työpaikka, ja tulos mainitsee sen. Varmista luokitus itse.
- [Mitä on valmista ja mitä ei](../reference/status.md#kuitit-ja-kirjaukset)
  kertoo tilanteen.

## Katso myös

- [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
- [Kirjaa matkaennakko ja sen tilitys](record-travel-advance.md)
- [Sanasto](../reference/glossary.md)
