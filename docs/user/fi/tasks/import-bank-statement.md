# Tuo pankin tiliote

Kuukauden kirjanpito alkaa tavallisesti pankin tiliotteesta. Agentti lukee
tiliotteen, ehdottaa jokaiselle tapahtumalle kirjauksen, ja hyväksynnälläsi
koko tiliote sinetöidään kerralla tositteiksi.

## Ennen kuin aloitat

- Sinulla on kuukauden tiliote sähköisenä ISO 20022 -tiedostona (camt.053 tai
  camt.052). Holvin tiliote on koeteltu. Holvin itsepalvelusta saatava
  "Electronic account statement" on camt.052-muotoinen ja käy sellaisenaan.
  Muiden pankkien camt-tiedostot voivat toimia, mutta niitä ei ole koeteltu.
- Kuukauden kuitit ja laskut ovat paikassa, jonka kodin `AGENTS.md` kertoo.

## Näin teet sen

1. **Pyydä agenttia.**

   > Tuo maaliskuun tiliote tiliote-2026-03.xml ja ehdota kirjaukset.

2. **Agentti lukee tiliotteen.** Lukeminen ei kirjaa mitään. Ohjelma listaa
   pankissa vahvistetut tapahtumat ja antaa jokaiselle pysyvän tunnisteen.
   Pankissa vielä vahvistamattomat tapahtumat (varaukset) jätetään pois, ja ne
   kirjataan siltä tiliotteelta, jolla ne ovat toteutuneet. Ohjelma näyttää
   myös tiliotteen jakson, alku- ja loppusaldon sekä sen, vievätkö luetut
   tapahtumat alkusaldon loppusaldoon. Jos eivät vie, tiedostoa ei ole luettu
   oikein, eikä sitä tuoda.

3. **Agentti valmistelee ehdotuksen.** Ohjelma ei päätä tilejä eikä
   ALV-käsittelyä. Agentti etsii jokaiselle tapahtumalle kuitin tai laskun ja
   katsoo, miten vastaavat tapahtumat on kirjattu aiemmin. Kun rivi maksaa
   ostoa, jolla on kuitti tai lasku, kuitti kirjataan omana tositteenaan
   ostovelkaa vastaan, ja rivi kirjataan ostovelkaa 2700 vastaan ilman ALV:tä
   ([Kun osto näkyy myös tiliotteella](record-receipts.md#kun-osto-näkyy-myös-tiliotteella)).
   Muille riveille, kuten pankin palvelumaksuille, agentti ehdottaa vastatilin
   ja ostoissa ALV-käsittelyn. Sen, mitä se ei voi päätellä, se kysyy
   koottuna.

4. **Tarkista ja hyväksy.** Agentti esikatselee tuonnin: ohjelma tarkistaa
   ehdotuksen kuten sinetöinnissä (tilit, summat, ALV-purku, kauden tila) ja
   näyttää jokaisen tositteen, joka sinetöitäisiin, kirjaamatta mitään.
   Pyydä lista nähtäväksi riveittäin, jos agentti ei näytä sitä. Katso jokaiselta riviltä päivä, vastapuoli, summa, tili ja
   ALV-käsittely. Tarkista erityisesti:
   - ALV-kanta on valittu **laskun päivästä**, ei maksupäivästä.
   - **Mikään osto ei tule kirjatuksi kahdesti.** Rivi, joka maksaa kuitin tai
     laskun, kirjataan tilille 2700 Ostovelat (tai 2740, kun yritys maksaa
     sinulle takaisin omalla rahallasi maksetun oston), ei kuluksi eikä
     ALV:lle. Kulu ja ALV ovat jo kuitin tositteella.

   Esikatselu antaa tunnisteen, kuten kuittien kirjauksessa. Hyväksyntäsi
   sidotaan siihen: sinetöinti onnistuu vain, jos se tuottaa täsmälleen
   esikatsellut tositteet.

   > Hyväksyn, kirjaa.

5. **Agentti sinetöi koko tiliotteen kerralla.** Alkuperäinen tiliotetiedosto
   arkistoidaan ja liitetään jokaiseen sen tapahtumista syntyvään uuteen
   tositteeseen. Joko koko tiliote sinetöidään tai siitä ei kirjata mitään.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant import camt053 parse --file tiliote-2026-03.xml
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant import camt053 seal --file tiliote-2026-03.xml --dry-run < kohdistukset.json
aggountant import camt053 seal --file tiliote-2026-03.xml \
  --approved-preparation-hash sha256:v1:… < kohdistukset.json
aggountant import camt053 observations
```

Sama komento lukee myös camt.052-tiedostot (myös nimellä `import camt052`).
</details>

## Valmis, kun

- Ohjelma vastaa `sealed N new vouchers from "FI…", M transactions already in
  the books`, ja tiliotteen jokainen vahvistettu tapahtuma on joko kirjattu tai
  agentti on nimennyt sen avoimeksi.
- Kuukausiraportin pankkitilin saldo vastaa tiliotteen loppusaldoa
  ([Katso, miten kuukausi meni](monthly-report.md)).

Saman tiliotteen voi tuoda uudelleen, ja päällekkäiset tiliotteet (esimerkiksi
kaksi vuositiliotetta, joissa vuodenvaihteen tapahtumat ovat molemmissa) voi
tuoda peräkkäin. Ohjelma tunnistaa jo kirjatun tapahtuman pankin
tapahtumatunnisteesta eikä kirjaa sitä toista kertaa. Se merkitsee vain, että
tapahtuma näkyi myös tässä tiedostossa; `import camt053 observations` näyttää,
mitkä tapahtumat kussakin tiliotetiedostossa oli. Jo kirjatulle tapahtumalle
annetaan sama kohdistus kuin ensimmäisellä kerralla. Esikatselu kertoo
jokaisesta tapahtumasta, onko se uusi vai jo kirjattu.

## Rajoitukset

- **Jokaiselle vahvistetulle tapahtumalle tarvitaan kirjaus.** Jos yksikin
  puuttuu tai viittaa tiliin, jota tilikartassa ei ole, koko tiliote hylätään
  eikä mitään sinetöidä. Myös jo kirjatun kuitin maksurivi kirjataan: se
  kuittaa kuitin ostovelan.
- Ostolaskua ei voi liittää tiliotteelta kirjattuun tositteeseen. Liitteenä on
  tiliote. Siksi lasku kirjataan omana tositteenaan, ja tiliotteen rivi
  kirjaa vain maksun.
- ALV puretaan tiliotteelta vain ostoista. Myynti, johon sisältyy ALV,
  kirjataan erikseen: [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](record-other-entries.md).

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#pankin-tiliote).

## Jos jokin menee pieleen

- **Koko tiliote hylättiin.** Jokin tapahtuma jäi ilman kirjausta tai tili
  puuttuu tilikartasta. Mitään ei sinetöity, ja agentti korjaa ehdotuksen:
  [Virhekoodit](../reference/error-codes.md).
- **Tuonti hylättiin koodilla `SOURCE_FACTS_CHANGED` tai
  `POSTING_PROJECTION_CHANGED`.** Tiliotteella on tapahtuma, joka on jo
  kirjattu, mutta pankin tiedot siitä ovat muuttuneet (`SOURCE_FACTS_CHANGED`)
  tai sille ehdotettiin eri kohdistusta kuin aiemmin
  (`POSTING_PROJECTION_CHANGED`). Mitään ei sinetöity. Virhe nimeää jokaisen
  tällaisen tapahtuman ja sen tositteen. Muuttuneet pankin tiedot selvitetään
  ennen kuin tapahtumalle kirjataan mitään; eri kohdistus korjataan antamalla
  sama kohdistus kuin tositteella, ja jos aiempi kirjaus oli väärä, se
  korjataan uudella tositteella ([Virhekoodit](../reference/error-codes.md)).
- **Sinetöinti hylättiin koodilla `IMPORT_PREPARATION_CHANGED`.** Tiliote,
  ehdotus tai tilikartta muuttui esikatselun jälkeen. Mitään ei sinetöity;
  agentti esikatselee uudelleen ja pyytää hyväksyntäsi uudelle listalle.
- **Pankkitilin saldo ei täsmää tiliotteeseen.** Jokin tapahtuma puuttuu tai on
  kirjattu kahdesti: [Kun jokin menee vikaan](../troubleshooting.md#pankkitilin-saldo-ei-täsmää-tiliotteeseen).

## Katso myös

- [Kirjaa kuitit ja ostolaskut](record-receipts.md)
- [Miten ALV-käsittely päätetään](../concepts/vat-treatment.md)
- [Kuukauden kirjanpito alusta loppuun](monthly-routine.md)
