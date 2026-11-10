# 4. Kuukausikierto: pankki, kuitit ja kauden sulku

Kuukauden kirjanpito etenee samassa järjestyksessä joka kuukausi:

1. tuo pankin tiliote
2. kirjaa kuitit ja laskut ja liitä ne tositteisiin
3. selvitä avoimet asiat ja velvoitteet
4. sulje kausi
5. tee ALV-ilmoitus (luku 5).

Voit pyytää agenttia hoitamaan koko kuukauden kerralla, esimerkiksi "kirjataan
maaliskuu". Agentti käyttää `capture-receipt`-skilliä, tekee itse sen, mihin
lähteet riittävät, ja tuo sinulle kysymykset ja hyväksyttävät tositteet
koottuina.

## Pankin tiliote

aggountant lukee pankin sähköisen tiliotteen ISO 20022 -muodossa (camt.053 tai
camt.052). Tuki on tehty Holvin tiliotteelle. Muiden pankkien camt-tiedostot
voivat toimia, mutta niitä ei ole koeteltu.

Tuonti tehdään kahdessa vaiheessa. Ensin agentti lukee tiedoston:

```sh
aggountant import camt053 parse --file tiliote-2026-03.xml
```

Lukeminen ei kirjaa mitään. Se listaa tapahtumat ja antaa jokaiselle pysyvän
tunnisteen. Ohjelma ei päätä tilejä eikä ALV-käsittelyä. Ne ovat agentin
ehdotus, jonka sinä hyväksyt. Agentti kokoaa jokaiselle tapahtumalle ehdotuksen:
pankkitili, vastatili tai -tilit ja ostoissa tarvittaessa ALV-käsittely ja
laskun päivä. Hyväksynnän jälkeen koko tiliote sinetöidään kerralla:

```sh
aggountant import camt053 seal --file tiliote-2026-03.xml < kohdistukset.json
```

Hyvä tietää:

- **Jokaiselle tiliotteen tapahtumalle tarvitaan kohdistus.** Jos yksikin puuttuu
  tai viittaa tiliin, jota tilikartassa ei ole, koko erä hylätään eikä mitään
  sinetöidä.
- **Alkuperäinen tiliotetiedosto arkistoidaan** ja liitetään jokaiseen sen
  tapahtumista syntyvään tositteeseen.
- **Saman tiliotteen voi tuoda uudelleen.** Jo sinetöidyt tapahtumat tunnistetaan
  eikä niitä kirjata toista kertaa.
- **Tällä komennolla ei ole erillistä esikatselutunnistetta**, toisin kuin kuittien
  kirjauksella. Hyväksyntäsi on se, että katsot agentin kohdistuslistan ennen kuin
  se ajaa `seal`-komennon. Pyydä lista nähtäväksi riveittäin.
- **ALV puretaan tuonnissa vain ostoista.** Myyntiä, johon sisältyy ALV, ei
  kirjata tiliotteelta. Agentti valmistelee sen erillisenä tositteena.
- **Jokainen tapahtuma kirjataan kerran.** Jos tiliotteen rivi on maksu kuitista,
  joka on jo kirjattu, sitä ei kirjata uudelleen. Agentti tarkistaa aiemmat
  tositteet ennen ehdotusta.

## Kuitit ja laskut

Tavallinen ostokuitti kirjataan näin:

1. **Lähdetiedosto arkistoidaan.** `aggountant artifact attach --file kuitti.pdf`
   tallentaa tiedoston ja palauttaa sen tunnisteen. Sama tiedosto tunnistetaan
   myöhemmin, joten saman kuitin kahdentuminen huomataan
   (`aggountant voucher list --attachment-ref <tunniste>`).
2. **Agentti esikatselee kirjauksen.** Se kirjaa kuitista luetut tiedot (summa,
   kuitin päivä, kuvaus) ja oman ehdotuksensa (kulutili, maksutili,
   ALV-käsittely) ja ajaa `aggountant capture-receipt record --dry-run`.
   Ohjelma muodostaa kirjauksen ALV-rivit verokantataulukosta kuitin päivän
   mukaan. Esikatselu ei tallenna mitään.
3. **Sinä hyväksyt.** Agentti näyttää lähdetiedot, ehdotuksen ja syntyvät
   kirjausrivit. Esimerkki: ohjelmistolisenssi 124,00 € (sis. ALV 25,5 %)
   kirjautuu 98,80 € tilille 7690 Atk-laite- ja ohjelmistokulut, 25,20 € tilille
   1763 Arvonlisäverosaaminen ja 124,00 € tilin 1910 Pankkitili kreditiin.
4. **Tosite sinetöidään.** Agentti ajaa saman komennon uudelleen ja antaa
   esikatselun tunnisteen (`--approved-preparation-hash`). Jos jokin on muuttunut
   välissä, ohjelma kieltäytyy (`CAPTURE_PREPARATION_CHANGED`) ja esikatselu
   tehdään uudelleen.

Saman kuitin uusi kirjausyritys ei synnytä toista tositetta. Ohjelma palauttaa jo
sinetöidyn tositteen. Kuitin päivä on tärkeä, koska ALV-kanta valitaan sen
perusteella. Yleinen kanta on 25,5 % 1.9.2024 alkaen, ja alennettu 14 %:n kanta
laski 13,5 %:iin 1.1.2026.

ALV-käsittely on aina nimetty valinta: kotimainen osto, käännetty verovelvollisuus
(EU-palvelut, EU-tavarat, EU:n ulkopuoliset palvelut) tai veroton osto, kun
verottomuus on varmistettu. Jos ratkaiseva tieto puuttuu, agentti merkitsee kuitin
selvitettäväksi eikä arvaa. Silloin saat kysymyksen, joka kertoo, mitä pitää
selvittää. Osittaista vähennysoikeutta tai maahantuontia tämä reitti ei käsittele.

**Erikoiskirjaukset**, joita kuittireitti ei tue (esimerkiksi myyntilaskut,
avaavat saldot ja korjaukset), agentti valmistelee tositeluonnoksena ja
tarkistaa ennen sinetöintiä (`voucher draft`, `voucher validate`,
`voucher seal`). Sama sääntö pätee: agentti näyttää rivit ja sinä hyväksyt ennen
sinetöintiä.

### Poikkeukselliset kuitit ja konsultointi

`capture-receipt start` on reitti kuiteille, joiden käsittelyyn tarvittaisiin
ulkopuolista konsultointia. Siinä ulos lähtisivät vain hyväksymäsi tiedot, eivät
alkuperäiset tiedostot. Tämän version ohjelmassa ei kuitenkaan ole yhteyttä
ulkopuoliseen konsultointipalveluun, joten reittiä ei voi vielä käyttää.
Poikkeuksellisen kuitin kohdalla agentti kysyy sinulta tai kirjanpitäjältäsi.

## Puuttuvat kuitit

Kun tiliotteella on maksu, jolle ei löydy kuittia, agentti etsii kuittia kodin
`AGENTS.md`-tiedostossa kuvatuista paikoista. Jos kuitti ei löydy, agentti
nimeää tapahtuman avoimeksi ja kertoo, mitä puuttuu.

Ohjelmassa on `hunt-receipts`-komento kuittien automaattiseen hakuun
sähköpostista ja verkkokauppojen tileiltä. Tässä versiossa siihen ei ole
kytketty oikeaa sähköposti- tai kauppatiliyhteyttä, joten haku on toistaiseksi
agentin ja sinun käsityötä.

## Velvoitteet

Velvoite on avoin asia määräpäivineen, esimerkiksi puuttuva ajopäiväkirja.
Avoin velvoite estää kautensa sulun, jotta asia ei unohdu.

```sh
aggountant obligation list
aggountant obligation resolve --id <tunniste>
```

Tässä versiossa velvoitteita syntyy vain poikkeuksellisten kuittien
konsultointireitin kautta, joten tavallisessa käytössä lista on yleensä tyhjä.
Merkitse velvoite ratkaistuksi vasta, kun asia on oikeasti hoidettu.

## Kauden sulku

Kun kuukauden kaikki tapahtumat on kirjattu, kausi suljetaan. Sulun jälkeen
kaudelle ei voi enää kirjata, ja ALV-ilmoitus tehdään suljetun kauden luvuista.

**Ennen kuin aloitat sulun**, varmista agentin kanssa:

- tiliotteen jokainen tapahtuma on kirjattu tai nimetty avoimeksi
- kuitit on liitetty
- pankkitilin saldo vastaa tiliotteen loppusaldoa (kuukausiraportista, luku 7)
- avoimia velvoitteita ei ole.

Sulku aloitetaan näin:

```sh
aggountant run start close-period --period 2026-03
```

Kausi siirtyy tilaan `closing`, ja sulkuajo jää odottamaan hyväksyntääsi
(`ApproveClose`). Tässä tilassa kaudelle ei voi enää sinetöidä uusia tositteita.
Kun hyväksyt, kausi sinetöidään:

```sh
aggountant run answer --run-id cp-… --gate ApproveClose --answer approve
```

Kauden tilan näet milloin tahansa:

```sh
aggountant period show --period 2026-03
```

Tila on `open` (avoin), `closing` (sulku käynnissä) tai `sealed` (suljettu).
Tulosteessa näkyy myös, onko kauden ALV-ilmoitus tehty (`vat_filed`).

**Tärkeää: älä aloita sulkua liian aikaisin, äläkä hylkää sitä.** Tässä versiossa
ei ole komentoa, jolla kauden saisi takaisin avoimeksi. Jos aloitat sulun ja
huomaat sitten puutteen:

- **Jos hyväksyt sulun**, kausi suljetaan, ja puuttuva tai korjattava tapahtuma
  kirjataan seuraavalle avoimelle kaudelle. Tämä on kirjanpidossa tavallinen
  tapa korjata suljettua kautta.
- **Jos hylkäät sulun** (`--answer reject`), kausi jää pysyvästi tilaan `closing`.
  Silloin sille ei voi kirjata eikä sitä voi sulkea uudelleen. Tämä on tunnettu
  puute.

Suljetulle kaudelle ei voi kirjata (`PERIOD_SEALED`). Korjaukset kirjataan
seuraavalle avoimelle kaudelle.
