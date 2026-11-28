# Tuo pankin tiliote

Kuukauden kirjanpito alkaa tavallisesti pankin tiliotteesta. Agentti lukee
tiliotteen ja ehdottaa jokaiselle tapahtumalle kirjauksen, ja kun hyväksyt
ehdotuksen, koko tiliote sinetöidään kerralla tositteiksi.

## Ennen kuin aloitat

- Sinulla on kuukauden tiliote sähköisenä ISO 20022 -tiedostona (camt.053 tai
  camt.052). Holvin tiliote on koeteltu, ja Holvin itsepalvelusta saatava
  "Electronic account statement" käy sellaisenaan (se on camt.052-muotoinen).
  Muiden pankkien camt-tiedostot voivat toimia, mutta niitä ei ole koeteltu.
- Kuukauden kuitit ja laskut ovat paikassa, jonka kodin `AGENTS.md` kertoo.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tuo maaliskuun tiliote tiliote-2026-03.xml ja ehdota kirjaukset.

2. **Agentti lukee tiliotteen.** Lukeminen ei kirjaa vielä mitään. Ohjelma
   listaa pankin vahvistamat tapahtumat ja antaa jokaiselle pysyvän
   tunnisteen. Vahvistamattomat tapahtumat (varaukset) se jättää pois, ja ne
   kirjataan siltä tiliotteelta, jolla ne ovat toteutuneet. Ohjelma näyttää
   myös tiliotteen jakson, alku- ja loppusaldon sekä sen, vievätkö luetut
   tapahtumat alkusaldon loppusaldoon. Jos eivät vie, tiedostoa ei ole luettu
   oikein, eikä agentti tuo sitä. Kun pankin tiedosto kertoo ne, tapahtumassa
   näkyy myös pankin tapahtumatyyppi (esimerkiksi korttiosto), siihen
   sisältyvät pankin kulut, ulkomaisen maksun summa alkuperäisessä
   valuutassa sekä se, palauttaako tai peruuttaako tapahtuma aiemman maksun,
   ja millä syyllä. Maksun viitenumero (kotimainen tai RF-viite) näkyy
   sellaisenaan, ja ohjelma kertoo, täsmääkö sen tarkiste. Virheellistäkään
   viitettä ei hylätä eikä jätetä pois.

3. **Agentti valmistelee ehdotuksen.** Tilit ja ALV-käsittelyn ehdottaa
   agentti, sillä ohjelma ei päätä niitä. Agentti etsii jokaiselle
   tapahtumalle kuitin tai laskun ja katsoo, miten vastaavat tapahtumat on
   kirjattu aiemmin. Kun rivi maksaa oston, jolla on kuitti tai lasku, kuitti
   kirjataan omana tositteenaan ostovelkaa vastaan ja tiliotteen rivi
   ostovelkaa 2700 vastaan ilman ALV:tä, kuten kohta
   [Kun osto näkyy myös tiliotteella](record-receipts.md#kun-osto-näkyy-myös-tiliotteella)
   kuvaa. Muille riveille, kuten pankin palvelumaksuille, agentti ehdottaa
   vastatilin ja ostoissa ALV-käsittelyn. Palautunut tai pankin peruuttama
   maksu kirjataan samoille tileille kuin alkuperäinen maksu, jolloin ne
   kumoavat toisensa. Jos lähtevän maksun viite on jokin yrityksen
   [tallennetuista Verohallinnon viitteistä](file-vat-direct.md#tallenna-verohallinnon-maksutiedot-kerran),
   ohjelma merkitsee sen veronmaksuksi, ja agentti ehdottaa kirjausta
   verotilille kulun sijaan. Viite ei kerro, minkä kauden veroa maksu
   koskee; sen agentti päättelee summasta, eräpäivästä ja annetuista
   ilmoituksista. Sen, mitä se ei voi päätellä, se kysyy sinulta yhdellä
   kertaa.

4. **Tarkista ja hyväksy.** Agentti esikatselee tuonnin. Ohjelma tarkistaa
   ehdotuksen samoin kuin sinetöinnissä (tilit, summat, ALV-purku, kauden
   tila) ja näyttää jokaisen tositteen, jonka se sinetöisi, kirjaamatta
   mitään. Esikatselu kertoo myös, mikä tapahtuma on uusi ja mikä jo
   kirjattu. Pyydä lista nähtäväksi riveittäin, jos agentti ei näytä sitä, ja
   katso jokaiselta riviltä päivä, vastapuoli, summa, tili ja ALV-käsittely.
   Tarkista erityisesti:
   - ALV-kanta on valittu laskun päivän mukaan eikä maksupäivän.
   - Mikään osto ei tule kirjatuksi kahdesti. Kuitin tai laskun maksava rivi
     kuuluu tilille 2700 Ostovelat (tai tilille 2740, kun yritys maksaa
     sinulle takaisin omalla rahallasi maksamasi oston). Kulutilille ja
     ALV:lle se ei kuulu, koska kulu ja ALV ovat jo kuitin tositteella.

   > Hyväksyn, kirjaa.

   Hyväksyntäsi koskee juuri näkemiäsi tositteita. Esikatselu antaa
   tunnisteen, johon hyväksyntä sidotaan, ja sinetöinti onnistuu vain, jos se
   tuottaa täsmälleen esikatsellut tositteet.

5. **Agentti sinetöi koko tiliotteen kerralla.** Joko koko tiliote sinetöidään
   tai siitä ei kirjata mitään. Alkuperäinen tiliotetiedosto arkistoidaan ja
   liitetään jokaiseen uuteen tositteeseen, joka sen tapahtumista syntyy.

Saman tiliotteen voi tuoda uudelleen, ja päällekkäiset tiliotteet voi tuoda
peräkkäin, esimerkiksi kaksi vuositiliotetta, joissa molemmissa ovat
vuodenvaihteen tapahtumat. Ohjelma tunnistaa jo kirjatun tapahtuman pankin
tapahtumatunnisteesta eikä kirjaa sitä toista kertaa. Se merkitsee vain, että
tapahtuma oli myös tässä tiedostossa. Agentti antaa jo kirjatulle
tapahtumalle saman kohdistuksen kuin ensimmäisellä kerralla.

### Kun pankkitapahtumia on kirjattu aiemmin käsin

Jos yritys on kirjannut pankkitapahtumia tavallisina kirjauksina (esimerkiksi
ennen kuin se alkoi tuoda tiliotteita), ohjelma ei tunnista pankin
tapahtumatunnisteesta, että kirjaus jo kattaa tapahtuman. Ennen kuin se
sinetöi tapahtumalle uuden tositteen, se siksi etsii tositetta, joka saattaa jo
kirjata sen: tositetta, jonka tunnisteessa tai selitteessä on pankin
tapahtumatunniste, tai tositetta, joka kirjaa saman summan samalle puolelle
samaa pankkitiliä enintään 7 päivän päässä pankin päivästä. Jos sellainen
löytyy, ohjelma pysäyttää koko tiliotteen ja nimeää tapahtuman ja tositteen
(`BANK_ENTRY_POSSIBLY_BOOKED`). Mitään ei kirjata kahteen kertaan.

Agentti näyttää tositteen ja kysyy, kumpi tapaus on kyseessä:

- **Tosite kirjaa tämän tapahtuman.** Agentti liittää tapahtuman tositteeseen
  uuden tositteen sijaan. Ohjelma tarkistaa, että tositteen pankkirivillä on
  sama tili, summa, puoli ja päivä (7 päivän sisällä); itse tositetta ei
  muuteta. Esikatselu näyttää tapahtuman liitettynä siihen tositteeseen, ja
  myöhemmillä tiliotteilla sama tapahtuma liitetään siihen samalla tavalla.
  Yhteen tositteeseen voi liittää vain yhden pankkitapahtuman, ja ohjelma
  kysyy siitä edelleen muiden samansuuruisten tapahtumien kohdalla. Jos yksi
  tosite kirjaa useita pankkitapahtumia, esimerkiksi kuukauden korttimaksut,
  vain yhden niistä voi liittää siihen. Muista tositteen kyseinen osa
  peruutetaan korjaustositteella, ja tiliote kirjaa tapahtuman sitten
  uudelleen.
- **Tosite on jotain muuta**, esimerkiksi toinen samansuuruinen maksu. Agentti
  merkitsee, ettei tämä tosite kirjaa tapahtumaa, ja tapahtuma saa uuden
  tositteen.

Tiliotteiden tuontiin siirtyvä yritys liittää aiemmin käsin kirjatut
pankkitapahtumansa näin ensimmäisellä kerralla, kun se tuo niitä sisältävän
tiliotteen.

### Kun tiliotteella ei ole tapahtumia

Jos tiliotteella on saldot mutta ei yhtään pankin vahvistamaa tapahtumaa,
ohjelma sanoo sen erikseen ja kertoo, minkä tilin ja minkä jakson tiliote on.
Tyhjä tiliote voi olla hiljainen kuukausi, mutta samalta näyttää myös väärän
tilin tai väärän kuukauden tiliote tai kesken katkennut vienti. Agentti
näyttää sinulle tilin ja jakson ja kysyy, onko pankissa todella ollut
tapahtumia sillä jaksolla. Jos alku- ja loppusaldo eroavat toisistaan,
tiedostosta puuttuu tapahtumia, eikä agentti tuo sitä.

Kun vahvistat, että kuukausi oli hiljainen, tiliote tuodaan tavalliseen
tapaan esikatselun ja hyväksynnän kautta. Tositteita ei synny, mutta
tiliotetiedosto arkistoidaan.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant import camt parse --file tiliote-2026-03.xml
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant import camt seal --file tiliote-2026-03.xml --dry-run < kohdistukset.json
aggountant import camt seal --file tiliote-2026-03.xml \
  --approved-preparation-hash sha256:v1:… < kohdistukset.json
aggountant import camt observations
```

Komento lukee sekä camt.053- että camt.052-tiedostot. Aiemmat nimet
`import camt053` ja `import camt052` toimivat edelleen samana komentona.
Tyhjän tiliotteen tulosteessa on kenttä `empty_report`.

`import camt observations` näyttää, mitkä tapahtumat kussakin
tiliotetiedostossa oli.
</details>

## Valmis, kun

- Ohjelma vastaa `sealed N new vouchers from "FI…", M transactions already in
  the books`, ja tiliotteen jokainen vahvistettu tapahtuma on kirjattu. Jos
  tapahtuman kuitti puuttuu vielä, agentti on nimennyt sen avoimeksi kodin
  `TODO.md`-tiedostoon.
- [Kuukausiraportin](monthly-report.md) pankkitilin saldo on sama kuin
  tiliotteen loppusaldo.

## Rajoitukset

- Jokainen vahvistettu tapahtuma tarvitsee kirjauksen, myös jo kirjatun
  kuitin maksurivi, joka kuittaa kuitin ostovelan. Jos yksikin kirjaus
  puuttuu, koko tiliote hylätään.
- Ostolaskua ei voi liittää tiliotteelta kirjattuun tositteeseen, koska sen
  liitteenä on tiliote. Siksi lasku kirjataan omana tositteenaan, ja
  tiliotteen rivi kirjaa vain maksun.
- ALV puretaan tiliotteelta vain ostoista. Myynti, johon sisältyy ALV,
  [kirjataan erikoiskirjauksena](record-other-entries.md).

Tiliotteen tuonnin kaikki rajoitukset ovat sivulla
[Mitä on valmista ja mitä ei](../reference/status.md#pankin-tiliote).

## Jos jokin menee pieleen

- `MISSING_ASSIGNMENT`: jokin tapahtuma jäi ilman kirjausta, joten koko
  tiliote hylättiin. Mitään ei sinetöity, ja agentti täydentää ehdotuksen.
- `ACCOUNT_NOT_FOUND`: kirjaus viittaa tiliin, jota tilikartassa ei ole.
  Mitään ei sinetöity, ja agentti valitsee tilin tilikartasta.
- `SOURCE_FACTS_CHANGED` tarkoittaa, että tiliotteella on jo kirjattu
  tapahtuma, josta pankin tiedot ovat muuttuneet. Mitään ei sinetöity, ja
  virhe nimeää jokaisen tällaisen tapahtuman ja sen tositteen. Selvitä, mikä
  muuttui, ennen kuin tapahtumalle kirjataan mitään.
- `POSTING_PROJECTION_CHANGED` tarkoittaa, että jo kirjatulle tapahtumalle
  ehdotettiin eri kohdistusta kuin aiemmin. Mitään ei sinetöity, ja virhe
  nimeää tapahtumat ja niiden tositteet. Agentti antaa saman kohdistuksen
  kuin tositteella. Jos aiempi kirjaus oli väärä, se korjataan uudella
  tositteella.
- `BANK_ENTRY_POSSIBLY_BOOKED` tarkoittaa, että tiliotteiden tuonnin
  ulkopuolella kirjattu tosite saattaa jo kirjata tiliotteen uuden
  tapahtuman. Mitään ei sinetöity. Agentti kysyy sinulta, kirjaako tosite
  tapahtuman, kuten
  [Kun pankkitapahtumia on kirjattu aiemmin käsin](#kun-pankkitapahtumia-on-kirjattu-aiemmin-käsin)
  kertoo.
- `BANK_VOUCHER_MISMATCH` tarkoittaa, että tapahtuma liitettiin tositteeseen,
  joka ei kirjaa sitä: sen pankkirivin tili, summa, puoli tai päivä on eri,
  tai tosite on jo liitetty toiseen tapahtumaan. Mitään ei sinetöity.
  Tarkista tositteen numero.
- `IMPORT_PREPARATION_CHANGED` tarkoittaa, että tiliote, ehdotus tai
  tilikartta muuttui esikatselun jälkeen. Mitään ei sinetöity. Agentti
  esikatselee uudelleen ja pyytää hyväksyntäsi uudelle listalle.
- Jos pankkitilin saldo ei täsmää tiliotteeseen, jokin tapahtuma puuttuu tai
  on kirjattu kahdesti. Vianetsinnän kohta
  [Pankkitilin saldo ei täsmää tiliotteeseen](../troubleshooting.md#pankkitilin-saldo-ei-täsmää-tiliotteeseen)
  neuvoo, miten syy löytyy.
- Muut koodit selitetään sivulla [Virhekoodit](../reference/error-codes.md).

## Katso myös

- [Kirjaa kuitit ja ostolaskut](record-receipts.md)
- [Miten ALV-käsittely päätetään](../concepts/vat-treatment.md)
- [Kuukauden kirjanpito alusta loppuun](monthly-routine.md)
