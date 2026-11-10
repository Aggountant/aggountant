# ADR-4 arkkitehdille — kohdennus-ulottuvuus tositteen postingilla

Tämä on **yhteenveto** ADR-4:n päätöksestä ohjelmistoarkkitehdille: mitä
päätettiin ja miksi, arkkitehtuurin termein. Varsinainen päätös perusteluineen on
`adr/ADR-4-cost-centre.md`; tämä sivu ei päätä mitään uutta.

## Ongelma

S1 on maalissa: `Posting` ja `VoucherBody` (`crates/model/src/voucher.rs`) ovat
oikeita sinetöityjä, hash-ketjutettuja tyyppejä (`wire_version = 1`,
`schema_version = 1`, kanoniset tavut ADR-2:n JCS-profiililla, `deny_unknown_fields`).
Benchmark nosti **kohdennuksen / kustannuspaikan** ainoaksi aidosti puuttuvaksi
peruspalikaksi (Kitsasin `Kohdennus` + `Vienti.kohdennus`), ja varoitti että se on
**kallis jälkiasentaa `Posting`iin sen jälkeen kun tyyppi jähmettyy**. Ketju on
muuttumaton: kenttää ei voi lisätä jo sinetöityihin tositteisiin. Päätös oli siis
tehtävä nyt, ja se on sidottu kolmelta suunnalta: tagi on **sinetöityä sisältöä**
(ADR-2, deterministinen kanoninen serialisointi, ei rikottavia goldeneita),
sinetöity skeema on muuttumaton kun taas masterdata ei ole (ADR-1), ja PII feilaa
kiinni (ADR-1 — kustannuspaikan *nimi* voi olla henkilön nimi).

## Päätös

**`Posting` saa alusta asti valinnaisen ulottuvuustagin: pieni, rajattu avain–arvo-
kartta validoituja läpinäkymättömiä koodeja (`dimensions`).** Tagi on postingilla,
ei voucherilla; ADR varaa muodon ja pakottaa kentän, mutta lykkää rekisterin,
validoinnin ja raportoinnin F7:ään. Ratkaisun ydin on **sinetöidyn kentän ja
ulottuvuus-masterdatan erottaminen**: posting kantaa vakaan läpinäkymättömän
viittauksen, git-natiivi rekisteri antaa sille merkityksen.

### Muoto

```rust
#[serde(default, skip_serializing_if = "Dimensions::is_empty")]
dimensions: Dimensions,   // newtype over BTreeMap<DimensionKey, DimensionValue>
```

- **`DimensionKey` / `DimensionValue`** ovat validoivia newtypejä `AccountId`/
  `VatCode`-perheessä: tiukka koodikielioppi (rajattu merkistö, pituuskatto),
  **koodeja ei vapaata tekstiä**; serde-deserialisointi ajaa konstruktorivalidoinnin,
  joten kieliopin­vastainen tai PII-muotoinen arvo on esittämätön lukupolulla.
- **`BTreeMap<DimensionKey, DimensionValue>`** on täsmälleen se lankamuoto jonka
  ADR-2 §Decision.2 sallii (string-avaimet, enkooderi lajittelee avaimet UTF-16-
  koodiyksiköittäin, tiukka dekooderi hylkää duplikaattiavaimet, Model rajaa
  merkintämäärän). Yksi kanoninen serialisointi, ei järjestys- tai
  duplikaattiavainmonitulkintaisuutta.
- Avain–arvo-muoto **kattaa suppeammat muodot ilman lisäkustannusta kanoniseen
  muotoon**: uusi ulottuvuustyyppi (osasto, kohde) on **masterdata-lisäys, ei
  skeemamuutos** — sallittujen avainten joukko ei ole sinetöidyssä skeemassa.

### Kriittinen kohta — taaksepäinyhteensopivuus

Kenttä on **valinnainen ja oletuksena poissa**, ja tyhjä kartta serialisoituu
**ei-miksikään** (`skip_serializing_if`):

- **Ei `wire_version`-bumppia, ei golden-rikkoutumista.** Kohdentamaton posting
  tuottaa tavu-identtisen kanonisen langan kuin nyt (avain jätetään pois). Kaikki
  wire-v1-fixturet re-enkoodautuvat identtisesti, ja ADR-2:n
  `stored_bytes == reencoded_bytes` pitää jokaiselle historialliselle tositteelle
  (poissa → `default`-tyhjä → skip → identtinen). Valinnaisen kentän lisäys on
  `schema_version = 1` -lisäys, ei kanonisointimuutos, joten se ei laukaise ADR-2
  §Decision.5:tä.
- **Täsmälleen yksi kanoninen muoto "kohdentamattomalle".** Eksplisiittinen tyhjä
  `"dimensions": {}` on **kielletty** tiukalla lukupolulla — kohdentamattomuus on
  vain avaimen poissaolo, ei läsnä-tyhjä kartta. Ei `{}`-vs-poissa-
  monitulkintaisuutta sinetöidyissä tavuissa.
- **Varaa muoto nyt, lykkää mekaniikka.** Kentän varaaminen on halpaa (yksi
  newtype + yksi valinnainen kenttä); rekisteri, record-time-validointi ja
  rollup-raportointi ovat F7-työtä. Siihen asti CLI ei täytä kenttää → genesis-
  tositteet pysyvät tavu-identtisinä, skeema on jo ulottuvuusvalmis.

### Rekisteri ja sijainti

Ulottuvuuden **merkitys** (sallitut avaimet/arvot, hierarkia, selväkieliset
nimet, aktiivinen/arkistoitu, "tämä tili vaatii kohdennuksen") elää **git-natiivissa
referenssimasterdatassa** (ADR-1), sisältö-hashattuna kuten tilikartta. Sinetti =
fakta (tämä posting on tagattu koodilla X); masterdata = mitä X tarkoittaa ja miten
X summautuu. Kitsasin itseensä viittaava `Kohdennus`-**puu** mallinnetaan
rekisterissä, ei sinetöidyssä postingissa — hierarkia/rollup on masterdata/raportointi-
asia, eikä muuttumattomia fiskaalitavuja pidä kytkeä muuttuvaan organisaatiorakenteeseen.
Ledger validoi record-timella postingin ulottuvuusviittaukset **kiinnitettyä**
rekisteriversiota vasten (sama resolve-then-pin -kuri kuin `chart_version`illa; ei
atomista ristikauppaa), ja resolvoitu rekisteriversio kiinnitetään provenienssiin
(O-31).

### Analytiikka (ADR-3)

ADR-3:n analytiikkareplikan tähtiskeema (yksi faktarivi per posting) altistaa
postingin ulottuvuudet → per-kohdennus P&L on litteä `GROUP BY` (sarakkeina tai
posting→dimension-siltana, resolvoitu ja nimetty build-timella kiinnitetystä
rekisteristä). Tarkka altistus on ADR-3:n toteutusdetalji (O-32).

## Hylätyt vaihtoehdot (tiivis)

- **Lykätä kokonaan / ei varata mitään** — hylätty: pysyvä skeemakatkos +
  jälkiasennuskustannus (benchmarkin varoitus). Varaaminen nyt dominoi.
- **Yksi vapaa ulottuvuus (yksi slotti)** — hylätty: ei kanna ortogonaalisia
  ulottuvuuksia (kustannuspaikka **ja** projekti) ilman myöhempää skeemabumppia;
  kartta kattaa tämän ilmaiseksi.
- **Kiinteät tyypitetyt slotit** — hylätty: kovakoodaa taksonomian sinetöityyn
  skeemaan; kolmas ulottuvuus = bumppi.
- **`Kohdennus`-puu sinetöidyssä postingissa** — hylätty: hierarkia kuuluu
  rekisteriin, ei muuttumattomiin tavuihin.
- **Vapaateksti-label postingilla** — hylätty: PII (ADR-1 fail-closed); posting
  kantaa koodin, label on masterdatassa.
- **Voucher-tason tagi** — hylätty: liian karkea per-projekti-P&L:lle; posting-taso
  (kuten `Vienti.kohdennus`). Voucher-default vain edge-oikotienä.
- **Sinetöimätön sivustore** — hylätty: tagi on osa tamper-evident-faktaa; sinetöinti
  on tarkoitus, ja optional-empty-skip tekee siitä ilmaista kohdentamattomille.

## Seuraukset arkkitehdille

- **`crates/model`-muutos (spinoff, ei tässä commitissa):** `dimensions` `Posting`iin
  + `DimensionKey`/`DimensionValue` newtypet; käsinkirjoitettu `Deserialize` reitittää
  uuden kentän validoivan konstruktorin kautta; lukupolku hylkää läsnä-tyhjän `{}`.
- **F7:** rekisterimasterdata + validointi + provenienssikiinnitys (O-30, O-31);
  ADR-3-analytiikan denormalisointi (O-32).
- **Guardrailit:** merkintämäärän ja avain/arvo-pituuden katot + koodikielioppi +
  canary-testi ettei PII-muotoinen arvo päädy lankaan.
- Ei uutta arkkitehtuuriprimitiiviä — tagi ratsastaa ADR-1/2/3:n substraatilla.
