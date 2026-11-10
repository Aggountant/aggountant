# ADR-5 arkkitehdille — sinettiä edeltävä draft-ikkuna ja korjauspinta

Tämä on **yhteenveto** ADR-5:n päätöksestä ohjelmistoarkkitehdille: mitä päätettiin
ja miksi, arkkitehtuurin termein. Varsinainen päätös perusteluineen on
`adr/ADR-5-seal-timing.md`; tämä sivu ei päätä mitään uutta.

## Ongelma

Seal-on-record on lukittu (D-05/D-06): tosite hash-ketjutetaan ja sinetöidään heti
kirjattaessa, sinetöity on muuttumaton, korjaus on uusi sinetöity tosite. Laillisuus
on selvä (survey); avoin kysymys on **sinettiä edeltävä muokkausvaihe** (O-29):
onko draft **ohut** (sinetöi ~heti, kaikki jälkikäteinen korjaus on oikaisutosite,
A) vai **ensiluokkainen kestävä ikkuna** (tosite voi olla "vastaanotettu / kesken"
päiviä ennen sinetöintiä + O-29-muutosloki, B).

Kolme maastonpiirrettä: (1) draft-olio on jo olemassa arvona (`VoucherDraft`
`crates/model`issa) mutta **koditon** — se elää stdoutissa, tempfilessä tai agentin
kontekstissa; (2) sinetin invariantit ovat **rakenteellisesti eristetyt** drafista —
gaplessness ja ketju syntyvät `BEGIN IMMEDIATE`-sinetöintitransaktiossa, jossa
numero varataan vasta insertin yhteydessä; drafilla ei ole numeroa, hashia eikä
ketjupaikkaa; (3) peukaloinnin paljastava append-only-loki on jo olemassa (O-25
hylkäysloki, oma hash-ketju, `verify` kävelee sen).

## Päätös

**Option B rajattuna hybridinä.** Ensiluokkainen sinettiä edeltävä ikkuna, mutta
toteutettuna **intake / work-item -vaiheena** (ei muuttuvana voucherina), **ohuella
commit-rajalla** (hyväksyntä sinetöi heti — ei "hyväksytty mutta sinetöimätön"
-jonoa) ja **skill-kohtaisella sinetöintitahdilla** (selvä kuitti sinetöi yhdellä
intentiolla; epäselvä ja `hunt-receipts` jäävät intakeen). Sinetöintitransaktio on
muuttumaton. Ei `crates/`/`skills/`-koodia tässä.

### Miksi intake eikä "durable voucher draft"

Kestävä olio nimeltä "draft voucher" ajautuu kohti "melkein kirjattua": raportit
kasvattavat *include-drafts*-boolean, close alkaa päätellä sen yli, analytiikka
haluaa altistaa sen → **toinen muuttuva ledger** (juuri Kitsasin muuttuva
`KIRJANPIDOSSA`-ikkuna). Intake = *saapunut/käsittelyssä oleva aineisto*, ei
`kirjaus`: liite + ehdotetut viennit + avoimet kysymykset + kohdekausi + provenienssi.
**Ei numeroa, hashia, ketjupaikkaa; ei vaikutusta mihinkään saldoon/raporttiin/
ilmoitukseen.** Hyväksynnässä agentti kääntää nykyisen intake-kandidaatin
transientiksi `VoucherDraft`iksi → `validate` → `seal` (sama S1-polku).

- **Blast radius sinetin invarianteille = nolla.** Kestävä intake muuttaa vain
  `seal()`:n *syötettä*, ei `seal()`:ia. Ohut draft ei ole turvallisempi — sama
  sinetöintitransaktio molemmissa.
- **Yksi kirjoitusportti vahvistuu.** Pre-seal-tila, joka muuten kertyisi skillin
  muistiin, tempfileen tai `[T→integration]`-nimiavaruuteen, saa nimetyn, tyypitetyn,
  hash-ketju-verifioitavan kodin Ledger-portin taakse. **Poistaa** A5:n
  alimäärittelemättömän namespacen sen sijaan että lisäisi.

### Korjauspinta ja läpinäkyvyysraja

Pre-seal: **draft-revise-silmukka** intake-verbeillä (`intake list|show|revise|
discard|prepare-voucher`); `prepare-voucher` emittoi `VoucherDraft`in. Post-seal: **ei
`voucher edit`ia**; ensiluokkainen `voucher correct <numero> --mode
reclass|reverse|adjust --reason …`, joka rakentaa transientin oikaisu-`VoucherDraft`in
linkitettynä alkuperäiseen (`corrects: <numero>` provenienssiin) ja sinetöi saman
polun kautta. Läpinäkyvyysraja = **"absorb the mechanics, surface the fact"**: agentti
saa piilottaa debit/credit-rakennelman, ei koskaan oikaisutositteen *olemassaoloa*;
esikatselu ("ei muuta #123:a; luo oikaisu #171 linkitettynä #123:een; netto …") ja
tulos nimetään. Sana "edit" kielletty sinetöidylle. **Hyväksyntä sitoo sisältöön**
Preview-gaten `bind-payload-hash`illa (O-23) — muutos hyväksynnän jälkeen vaatii uuden
hyväksynnän.

### Must-seal Ledger-pakotettuna

Ratkaisematon intake mallinnetaan **obligation/blocker**-tilana (kuten D1:n "avoimet
obligationit blokkaavat sulun"). `close-period` / `close-year` / `file-vat` kysyvät
Ledgeriltä kauden ratkaisemattomat intaket ja **estävät** etenemisen (refuse
journaloidaan O-25-mekanismilla); skill ei voi ohittaa "sinetöimällä nopeasti ensin".
Tunnettu tapahtuma sinetöidään väliaikaiskäsittelyllä cutoffissa (vaatii
suspense-/clearing-tilit joita S1:n 5-tilinen kartta ei tarjoa → O-34).

### O-29-loki

**Oma append-only hash-ketju** (`SHA256(prev ‖ canonical(event))`), yhden
kirjoitusportin sisällä, `verify`:n kävelemä — täsmälleen O-25-mekanismi, mutta
**erillinen ketju voucher-ketjusta** (draft on pre-booking, ei osa kirjausketjua
ennen sinetöintiä), joten draft-kirjoitukset eivät serialisoidu sinetöintiketjun
headille. O-25 refaktoroidaan **geneeriseksi namespacetuksi** audit-lokiksi
(`namespace: rejection | intake_change | …`). Kirjaa **hyväksytyt versiot** (ei
jokaista LLM-ehdotusta): actor, aika, `run_id`, syy, before/after **hash tai kompakti
diff**, liite-/pankkitapahtumaviitteet, hyväksyntäsidonta, lopullinen voucher-numero/
hash. Raaka PII (OCR/vapaateksti) pysyy salatussa storessa (ADR-1 `Cipher`), lokiin
vain hash/diff (O-35). Lopullinen revisio-hash + hyväksyntä-payload-hash kiinnitetään
sinetöidyn tositteen provenienssiin (`draft_id` + `draft_change_log_head`).

### Raportointi (ADR-3)

Oikaisu renderöityy oletuksena "corrected to X" -netotuksena (porautuen molempiin),
mutta lakisääteinen päiväkirja/pääkirja/arkisto säilyttää jokaisen tositteen. Linkitys
on **rakenteista dataa sinetöintihetkellä** (`corrects`), ei raporttiaikainen
heuristiikka kuvausteksteistä.

## Hylätyt vaihtoehdot (tiivis)

- **Option A (ohut vain)** — hylätty: capture-before-clarity jää koditta; ei vähennä
  yleisintä korjaustapahtumaa (post-seal). *Ohut commit-raja säilytetään* (determinäätti
  hyväksytty kirjaus sinetöidään heti).
- **Option C (edit-until-lock)** — scope out: peruu D-05/D-06.
- **Rajaton kestävä "voucher draft"** — hylätty: toinen muuttuva ledger takaoven kautta.
- **Hiljainen "edit" joka piilottaa oikaisutositteen** — hylätty (liian näkymätön).
- **Pakollinen manuaalinen korjausseremonia** — hylätty (liian friktio).
- **Ei O-29-lokia** — hylätty (agentti voisi kirjoittaa pre-seal-historian uusiksi).
- **Full-chain-weight draft-loki / draft-events voucher-ketjuun** — hylätty
  ylisuunnitteluna (serialisoisi draft-kirjoitukset sinetöinnin kanssa; draft on
  pre-booking).
- **Vain-muistissa istuntokohtainen draft (security-fallback)** — hylätty: haihtuu
  istuntojen yli, kaataa monipäiväisen capturen ja `hunt-receipts`-kursorit.

## Seuraukset arkkitehdille

- **Spinoff (ei tässä):** intake-vaihe + `intake`-verbit; `voucher correct` +
  `corrects`-provenienssi; geneerinen namespacetettu O-25/O-29-loki + `verify`-laajennus;
  must-seal-preconditionit close/file-runeihin; ADR-3-korjausnetotus;
  contract-testipatteri (intake ei numeroitu / ei ketjutettu / ei arkistoitu kirjauksena
  / ei laskettu / ei voi valtuuttaa ilmoitusta; sinetöinti intakesta varaa numeron vain
  atomisessa sinetöintitransaktiossa; hylätty intake ei jätä osittaista tositetta).
- **Guardrailit:** intake näkymätön kaikille johdetuille lukupinnoille paitsi
  "inbox"-näkymälle (analytiikan totuuslähde on sinetöity store); ei "fast seal from
  intake" -ohituspolkua; O-29 kirjaa diffit/hashit, ei vapaatekstiä.
- Ei uutta arkkitehtuuriprimitiiviä — ikkuna ratsastaa ADR-1/2/3:n substraatilla ja
  O-25:n lokimekanismilla.
