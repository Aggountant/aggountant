# ADR-3 arkkitehdille — audit-arkisto + analytiikan lukupinta

Tämä on **yhteenveto** ADR-3:stä ohjelmistoarkkitehdille: päätös, keskeiset
kompromissit ja niiden perustelu. Täydet perustelut, uhkamalli ja hylätyt
vaihtoehdot ovat `adr/ADR-3-fiscal-store.md`; tämä sivu tiivistää, ei päätä
uudelleen.

## Päätös yhdellä lauseella

**Sinetöityyn per-tosite-ketjuun liitetään kaksi johdettua lukuprojektiota — (1)
vientikelpoinen, tiiviste-varmennettu ja valinnaisesti allekirjoitettu
`fiscal archive` -audit-arkisto ja (2) erillinen, johdettu, vain-luku
`fiscal analytics` -replika vakaalla SQL-näkymäkontraktilla — ja molemmat
rakennetaan suoraan auktoritatiivisesta fiscal-SQLitestä eikä kumpikaan voi
kirjoittaa sinetöityä varastoa.** ADR päättää arkkitehtuurin; **ei toteuta** (ei
`crates/`-muutoksia).

Kantava periaate, ADR-1:n jatkona: **sinetöidyllä fiscal-SQLitellä on täsmälleen
yksi tehtävä — kirjoitusportitettu seal/chain/counter/rejection-transaktio. Jokainen
lukupinta (git-peili selailuun, audit-arkisto luovutukseen/säilytykseen,
analytiikkareplika SQL:ään) on johdettu, kertakäyttöinen projektio siitä yhdestä
auktoriteetista.** Arkisto ja replika ovat git-peilin sisaruksia — ADR-1:n
single-authority-invariantti *vahvistuu*, ei veny.

## Facet 1 — audit-arkisto

- **Kanoninen muoto = ADR-2:n wire.** Arkiston auktoritatiiviset tavut ovat
  **ADR-2:n kanoninen tosite-JSON kopioituna tavu tavulta** fiscal-SQLitestä (jo
  ketjun preimage, ihmisluettava, `git diff`-attava, **ei uutta
  serialisointikoodia**). PDF/CBOR/oma auktoritatiivisena hylätään (drift + luettavuus).
- **Rakennetaan auktoriteetista, tarkistuksen jälkeen.** `fiscal archive` lukee
  konsistentin snapshotin, **verifioi ketjun lennossa**, ja vaatii `fiscal verify`
  (SQLite↔peili lockstep) läpi ensin. Peiliä ei koskaan kohdella auktoritatiivisena.
- **Sisältö = itsenäinen tilikausiarkisto:** tositteiden kanoninen JSON, kauden
  sinetit + **rejection-loki (O-25)**, **resolvoitu masterdata sisällytettynä**
  (tilikartta + policyt, content-hashattuna — koska ADR-1:n git-masterdata voidaan
  rebaseta), ja **tilikauden arkistossa pakolliset renderöidyt kirjanpitokirjat**
  (päiväkirja/pääkirja/tase-erittelyt/tilinpäätös/liitetiedot PDF/A — KPA 2:8: nippu
  JSONia ei ole tilikausiarkisto). Lisäksi `MANIFEST.sha256` (turvallinen
  bundle-profiili: kiinteät ASCII-polut, ei traversalia/symlinkkejä), `ARCHIVE.json`
  ja arkiston tiiviste `SHA256(domain_sep ‖ manifest)` **ei-kehämäisenä**
  (tiiviste/allekirjoitus eivät ole manifestin syötteitä).
- **Kausi-ketju.** Kukin arkisto upottaa edellisen arkiston tiivisteen →
  **karkea, ulkoisesti todennettava meta-ketju tilikausien yli** hienon
  per-tosite-ketjun päällä. Kitsasilla ei ole vastaavaa. Kausiarkistot kevyitä,
  tilikausiarkistot pakollisia ja täysiä.
- **Varmennus porrastettuna, rehellinen uhkamalli:**
  - **Tier 0 (pakollinen, avaimeton):** tiiviste + per-tosite-ketju + kausi-meta-ketju
    + **pakollinen ulkoinen tiivisteen talletus** (tuloste/sähköposti/ulkoinen repo).
    Tarkistettavissa pelkällä `sha256sum -c`. = Kitsas-taso nimenomaisena vaiheena.
  - **Tier 1 (suositeltu):** detached **Ed25519** (minisign/signify) **omalla
    allekirjoitusavaimella** — **ei** ADR-1:n `age`-salausidentiteetti, ei
    artifact/PII/seal-avain. Autentikoi vain jos verifioija on **pinnannut
    julkisen avaimen ulkoisesti** (arkistoon niputettu avain = pelkkä mukavuus). Ei
    kestä koneen+avaimen kompromissia (sanotaan suoraan). Avainelinkaari →
    key-lifecycle-ADR; siihen asti Tier 1 valinnainen.
  - **Tier 2 (valinnainen, suositeltu jo v1:stä):** **RFC 3161** -aikaleima
    tiivisteelle. **Ei pysyvää avainta**, itsenäinen artefakti, suojaa
    "allekirjoitin-oman-kirjanpitoni" -uhkaa vastaan. Aito ero Kitsasiin.
- **"Todennettavissa ohjelman ulkopuolella":** `sha256sum -c` + valinnainen
  `minisign -V` (ulkoisesti pinnattu avain) + dokumentoitu ~1-sivun ketjun
  uudelleenkävely `VERIFY.md`:ssä (suoritettavissa `aggountant verify-archive`:lla mutta
  ei siitä riippuvainen). Testataan **vuosittain vanhan arkiston verifioinnilla
  puhtaalla koneella**.

## Facet 2 — analytiikkareplika

- **Erillinen, johdettu, vain-luku SQLite — ei live-fiscal-tiedosto.** Fyysisesti
  eri tiedosto, sinetöidyn varaston **uudelleenrakennettava projektio** (git-peilin
  sisarus). Kertakäyttöinen, ei koskaan auktoritatiivinen, kantaa
  `built_from_chain_head`-watermarkin. **Sinetöityä varastoa ei koskaan avata
  analytiikkapolulla** → vain-luku **rakenteellisesti**, ei konfiguraatiolla.
- **Full rebuild on demand** (ei inkrementaalista CDC:tä tällä volyymilla) — poistaa
  staleness-drift- ja kursori-toipumisbugit. Osittaista snapshotia **ei koskaan
  julkaista** (vaje = kova virhe, vanha snapshot jää voimaan).
- **Denormalisoitu tähtiskeema agentin SQL:lle:** yksi faktarivi per vienti;
  **rahamäärät täsmällisinä kokonaislukuina** + etumerkkikonventio + `is_debit` (ei
  liukulukua, ei desimaalimerkkijonoja aritmetiikkaan); tilin nimi/laji + resolvoitu
  ALV **upotettuna build-ajassa** pinnatusta `chart_version`ista (agentti ei kurota
  gitiin/fiscaliin); **positiivinen luokittelu-whitelist** (PII ei vuoda);
  provenance `_meta`-näkymä. Vakaa kontrakti = **versioidut SQL-näkymät**;
  fyysiset taulut voivat kehittyä.
- **Enforcement (defense in depth):** agentti avaa **vain** julkaistun immutable
  snapshotin `SQLITE_OPEN_READONLY`:lla eikä **koskaan saa fiscal-polkua**;
  `sqlite3_set_authorizer` kieltää `ATTACH`/kirjoitukset/`load_extension`/vaaralliset
  pragmat/ei-sallitut UDF:t; immutable snapshot-julkaisu privilegioidusta
  builderistä; **suositus:** OS-tason capability-erottelu; kyselybudjetit
  (koko/askel/muisti). Rehellinen raja: yhden käyttäjän asennuksessa ilman
  uid-erottelua takuu nojaa "agentti ei saa polkua + authorizer + O_RDONLY" -kerroksiin.

## Ratkaiseva kompromissi ja ainoa erimielisyys

Facet 2: **replika vs. vakaa-näkymä-samaan-tiedostoon.** Ylläpidettävyyslinssi
(ainoa eriävä) puolsi jälkimmäistä: `SQLITE_OPEN_READONLY` = kernelin O_RDONLY,
ilmainen, ei projektoria. **Hylätty replikan hyväksi:** (1) O_RDONLY ei anna
"provably read-only under arbitrary SQL" samassa uid:ssä — agentti voi avata
**toisen** kirjoitettavan kahvan samaan polkuun; (2) pitkä agenttikysely live-DB:llä
**näännyttää WAL-checkpointin** ja uhkaa kirjoituspolkua; (3) kunnollinen
analytiikkaskeema (denormalisoitu, kokonaisluvut) ei kuulu transaktiovarastoon
(paisuttaa backup/VACUUM, rikkoo ADR-1:n keep-lean). Full-rebuild-on-demand kumoaa
projektorin rappeutumishuolen. **Turvallisuudelle väistetään** turvakonfliktissa.
Ylläpidettävyyden **oikeat huolet otettiin sisään:** ei CDC:tä, pakollinen
varmennustaso **avaimeton**, wire-enkooderin uudelleenkäyttö, `sha256sum -c` +
built-in-verify hauraan skriptin sijaan.

## Hyväksytyt kustannukset

- Kaksi lisäprojektiota (arkisto + replika) git-peilin lisäksi; rakennettava samasta
  snapshotista, arkisto↔peili-täsmäytys.
- Pakollinen tilikausi-PDF-renderöinti (moottori = toteutusdetalji, `typst`/HTML→PDF).
- Tier 1 tuo **oman** Ed25519-avaimen (kolmas kryptoidentiteetti); katettu tekemällä
  Tier 1 suositelluksi ja pakollinen/paras taso (Tier 0, Tier 2) **avaimettomiksi**.
- Analytiikan enforcement-pino (authorizer-allowlist, immutable-julkaisu, budjetit)
  on oikeaa koodia + kontrakti-testit yli SQLite-päivitysten.

## Suurin riski

**Julkaistu-mutta-divergoitunut lukuprojektio luullaan totuudeksi, tai
talletus/avaimen-pinnaus-kuri jää tekemättä.** Protokolla voi olla virheetön ja
audit-tarina silti pettää: arkisto tuotettu peilin divergoituessa (`fiscal verify`
ensin + byte-täsmäytys), replika hiljaa vanhentunut (rebuild per sessio + watermark
tuloksissa), pakollinen talletus ohitettu (fail-closed/vahvistus), julkista avainta
ei pinnattu (`VERIFY.md` merkitsee niputetun avaimen epäluotettavaksi). Molemmat
lukupinnat ovat **vain-luku auktoriteettia vastaan** — ADR-1:n single-write-gate ei
heikkene — mutta niiden *arvo* nojaa operatiiviseen kuriin, jonka työkalujen on
**pakotettava**, ei oletettava.

## Liittyvät

- Varsinainen päätös: `adr/ADR-3-fiscal-store.md`.
- **ADR-1** (single-authority + johdetut projektiot; keskeneräinen git-peili +
  `fiscal verify` ovat tämän toteutuksen ehtoja; key-lifecycle-ADR = Tier 1:n koti),
  **ADR-2** (arkiston tavut = ADR-2 wire; ketjun uudelleenkävely
  `SHA256(prev ‖ canonical_wire)`).
- `benchmark.md §3.2/§3.4`, `benchmark-koonti.md` K-10…K-15;
  `s1-sealed-voucher/spike-notes.md` liput #1/#2; `decisions.md` D-06, O-27…O-29.
