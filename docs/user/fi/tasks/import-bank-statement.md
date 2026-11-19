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
   kirjataan siltä tiliotteelta, jolla ne ovat toteutuneet.

3. **Agentti valmistelee ehdotuksen.** Ohjelma ei päätä tilejä eikä
   ALV-käsittelyä. Agentti etsii jokaiselle tapahtumalle kuitin tai laskun ja
   katsoo, miten vastaavat tapahtumat on kirjattu aiemmin. Kun rivi maksaa
   ostoa, jolla on kuitti tai lasku, kuitti kirjataan omana tositteenaan
   ostovelkaa vastaan, ja rivi kirjataan ostovelkaa 2700 vastaan ilman ALV:tä
   ([Kun osto näkyy myös tiliotteella](record-receipts.md#kun-osto-näkyy-myös-tiliotteella)).
   Muille riveille, kuten pankin palvelumaksuille, agentti ehdottaa vastatilin
   ja ostoissa ALV-käsittelyn. Sen, mitä se ei voi päätellä, se kysyy
   koottuna.

4. **Tarkista ja hyväksy.** Pyydä lista nähtäväksi riveittäin, jos agentti ei
   näytä sitä. Katso jokaiselta riviltä päivä, vastapuoli, summa, tili ja
   ALV-käsittely. Tarkista erityisesti:
   - ALV-kanta on valittu **laskun päivästä**, ei maksupäivästä.
   - **Mikään osto ei tule kirjatuksi kahdesti.** Rivi, joka maksaa kuitin tai
     laskun, kirjataan tilille 2700 Ostovelat (tai 2740, kun yritys maksaa
     sinulle takaisin omalla rahallasi maksetun oston), ei kuluksi eikä
     ALV:lle. Kulu ja ALV ovat jo kuitin tositteella.

   Tiliotteen tuonnissa ei ole erillistä esikatselun tunnistetta kuten kuittien
   kirjauksessa. Hyväksyntäsi on se, että katsot listan ennen sinetöintiä.

   > Hyväksyn, kirjaa.

5. **Agentti sinetöi koko tiliotteen kerralla.** Alkuperäinen tiliotetiedosto
   arkistoidaan ja liitetään jokaiseen sen tapahtumista syntyvään tositteeseen.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant import camt053 parse --file tiliote-2026-03.xml
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant import camt053 seal --file tiliote-2026-03.xml < kohdistukset.json
```

Sama komento lukee myös camt.052-tiedostot (myös nimellä `import camt052`).
</details>

## Valmis, kun

- Ohjelma vastaa `sealed N vouchers from "FI…"`, ja tiliotteen jokainen
  vahvistettu tapahtuma on joko kirjattu tai agentti on nimennyt sen avoimeksi.
- Kuukausiraportin pankkitilin saldo vastaa tiliotteen loppusaldoa
  ([Katso, miten kuukausi meni](monthly-report.md)).

Saman tiliotteen voi tuoda uudelleen. Jo sinetöidyt tapahtumat tunnistetaan
eikä niitä kirjata toista kertaa, vaikka ohjelma tulostaa saman
`sealed`-rivin.

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
- **Pankkitilin saldo ei täsmää tiliotteeseen.** Jokin tapahtuma puuttuu tai on
  kirjattu kahdesti: [Kun jokin menee vikaan](../troubleshooting.md#pankkitilin-saldo-ei-täsmää-tiliotteeseen).

## Katso myös

- [Kirjaa kuitit ja ostolaskut](record-receipts.md)
- [Miten ALV-käsittely päätetään](../concepts/vat-treatment.md)
- [Kuukauden kirjanpito alusta loppuun](monthly-routine.md)
