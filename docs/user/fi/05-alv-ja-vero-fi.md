# 5. ALV ja Vero.fi

ALV-ilmoitus tehdään suljetun kauden luvuista. Kirjanpidosta johdetut luvut
voi ilmoittaa kahdella tavalla:

- **suoraan Verohallinnolle** Vero API -rajapinnan kautta, kun yrityksesi on
  ottanut sen käyttöön (alla)
- **käsin** OmaVerossa tai Ilmoitin.fi-palvelussa, minkä jälkeen ilmoitus
  kirjataan aggountantiin tehdyksi.

Kummallakin tavalla vero maksetaan erikseen OmaVerossa tai pankissa.
aggountant ei maksa mitään.

## Ilmoitettavat luvut

Kun kausi on suljettu (luku 4), agentti esikatselee ilmoituksen:

```sh
aggountant file-vat preview --period 2026-03
```

Esikatselu lukee vain kirjanpitoa. Se ei käynnistä ajoa, ei lue tunnuksia eikä
ota verkkoyhteyttä. Tuloksessa on ALV-ilmoituksen numeroidut kentät euroina,
myynnin ja ostojen vero, nettosumma ja maksettava määrä. Esimerkiksi kuukausi,
jolla on vain yksi 124,00 €:n ostokuitti, antaa kenttään 307 (kohdekauden
vähennettävä vero) 25,20 € ja kenttään 308 (maksettava tai palautukseen
oikeuttava vero) −25,20 €. Maksettavaa ei ole.

Kaudesta, jota ei ole suljettu, esikatselu kieltäytyy (`PERIOD_NOT_SEALED`). Selvitä
aineistoon liittyvät kysymykset ennen sulkua. Sulun ja ilmoituksen jälkeen
korjaus tarkoittaa korvaavaa ilmoitusta, jota aggountant ei tee.

Esikatselun ohjeteksti kehottaa tällä hetkellä aina syöttämään luvut käsin
OmaVeroon, vaikka suora ilmoitus olisi käytössä. Teksti on vanhentunut. Suoran
reitin käyttö riippuu vain siitä, onko Vero.fi-valmius kunnossa.

## Vero.fi-käyttöönotto

Suora ilmoittaminen on yrityksesi oma yhteys Verohallintoon. **Jokainen käyttäjä
rekisteröi oman ohjelmistonsa ja saa oman ohjelmistoavaimensa** Verohallinnon
API-portaalista, ja yritys hankkii oman varmenteensa. aggountantilla ei ole
yhteistä avainta, jonka voisi lainata. Ilmoitus lähtee aina yrityksesi omilla
tunnuksilla.

Portaali-, sopimus- ja varmennetoimet teet itse, koska ne ovat yrityksen
oikeustoimia. Agentti ei asioi portaaleissa puolestasi. Se kokoaa
tiedot, tarkistaa valmiuden, ajaa tarkistukset luvallasi ja kertoo, kenen vuoro
kunkin esteen poistamisessa on. Toimien täsmälliset nimet ja järjestys ovat
Verohallinnon ohjeissa. Tässä ne on kuvattu sillä tarkkuudella kuin aggountant
ne tarkistaa.

### Mitä tarvitset Verohallinnolta

1. **Ohjelmiston rekisteröinti ja ohjelmistoavain.** Rekisteröi ohjelmistosi
   Vero API -portaalissa ja ota talteen sen antama ohjelmistoavain.
2. **Rajapintasopimus.** Yrityksen Vero API -sopimus tai -hakemus.
3. **Tuotantovarmenne.** Hae yrityksellesi varmenne Verohallinnon
   varmennepalvelusta. Saat varmenteen, sen yksityisen avaimen ja
   varmentajan (CA) varmenteen. Varmenne on voimassa kaksi vuotta.
4. **ALV:n hyväksyntätestit.** Valitse portaalissa ALV-rajapinnan
   hyväksyntätestit. aggountant ajaa ne puolestasi (alla).
5. **Tuotantolupa.** Kun testit on hyväksytty, portaalissa aktivoidaan
   tuotantokäyttö.

### Tunnukset koneellesi

Tallenna tunnukset itse JSON-tiedostoksi, esimerkiksi
`~/.config/aggountant/credentials/vero-production.json`. Tiedostossa on:

- `target`: `"production"`
- `business_id`: yrityksesi Y-tunnus
- `software_key`: ohjelmistoavaimesi
- `client_certificate_pem`, `client_private_key_pem` ja `ca_certificate_pem`:
  varmenne, sen yksityinen avain ja varmentajan varmenne PEM-muodossa.

Tiedoston pitää olla kirjanpitokodin ulkopuolella, eikä sitä saa lisätä gittiin.
Aseta oikeudet niin, että vain sinä voit lukea sen. Älä liitä tunnuksia
keskusteluun agentin kanssa. Kun tarvitset varmenteen tunnistetta esimerkiksi
Verohallinnon tuelle, käytä `vero-fi status` -komennon näyttämää SHA-256-
sormenjälkeä. Se ei ole salainen.

Ohjelmistoavain on sinun omasi. Jos se vuotaa, ota yhteyttä Verohallintoon
samalla tavalla kuin minkä tahansa muun tunnuksen vuotaessa.

### Valmiuden tarkistus

```sh
aggountant vero-fi status
```

Tarkistus on paikallinen eikä ota verkkoyhteyttä, joten sen voi ajaa milloin
tahansa. Sama tarkistus tehdään automaattisesti juuri ennen jokaista lähetystä.
Tuloksessa näkyy, onko valmius kunnossa (`ready`), sekä esteet, varoitukset ja
varmenteen voimassaolo. Jokaisella esteellä on omistaja: `human` tarkoittaa
sinun tietoasi tai portaalitointasi, `agent` asetusta, jonka agentti voi korjata.

| Este | Mitä se tarkoittaa |
|---|---|
| `VERO_BUSINESS_ID_MISSING`, `VERO_CONTACT_NAME_MISSING`, `VERO_CONTACT_PHONE_MISSING` | Y-tunnus tai yhteyshenkilön tiedot puuttuvat asetuksista |
| `VERO_SOFTWARE_REGISTRATION_UNATTESTED` | et ole vahvistanut ohjelmiston rekisteröintiä |
| `VERO_API_AGREEMENT_UNATTESTED` | et ole vahvistanut rajapintasopimusta |
| `VERO_PRODUCTION_VERIFICATION_UNATTESTED` | hyväksyntätestejä ei ole kirjattu hyväksytyiksi |
| `VERO_PRODUCTION_PERMISSION_UNATTESTED` | et ole vahvistanut tuotantolupaa |
| `VERO_PORTAL_VERIFICATION_MISSING` | hyväksyntätestien ajosta ei ole näyttöä |
| `VERO_CREDENTIAL_PATH_MISSING`, `VERO_CREDENTIAL_UNAVAILABLE`, `VERO_CREDENTIAL_INVALID` | tunnustiedostoa ei ole määritetty, sitä ei voi lukea tai se on virheellinen |
| `VERO_CREDENTIAL_IN_ACCOUNTING_HOME` | tunnustiedosto on kirjanpitokodin sisällä |
| `VERO_CREDENTIAL_BUSINESS_ID_MISMATCH` | tunnusten Y-tunnus ei vastaa asetuksia |
| `VERO_CREDENTIAL_TARGET_MISMATCH` | tunnukset ovat testi- eikä tuotantoympäristön |
| `VERO_DELEGATED_USE_UNSUPPORTED` | tunnuksissa on valtuutustunniste. Toisen puolesta ilmoittamista ei tueta. |
| `VERO_CERTIFICATE_EXPIRED`, `VERO_CERTIFICATE_NOT_YET_VALID` | varmenne ei ole voimassa |
| `VERO_ACTIVATION_CERTIFICATE_MISMATCH`, `VERO_PORTAL_VERIFICATION_CERTIFICATE_MISMATCH` | tuotantoaktivointi tai testinäyttö on tehty eri varmenteella kuin nykyinen |
| `VERO_DISABLED` | suora ilmoittaminen ei ole päällä |

Ohjelma varoittaa 90, 30 ja 7 päivää ennen varmenteen vanhenemista.

### Käyttöönoton vaiheet

1. **Perusasetukset.** Agentti kirjoittaa kodin `vero-filing.json`-tiedoston
   komennolla `aggountant vero-fi configure`. Tila on aluksi `disabled`, ja
   tiedostossa ovat Y-tunnus, yhteyshenkilön nimi ja puhelinnumero sekä
   tunnustiedoston polku. Polku tallentuu koneen omaan asetustiedostoon, ei
   kotiin, joten jokaisella koneella voi olla omat tunnuksensa. Valitsin
   `--dry-run` näyttää tuloksen kirjoittamatta mitään.
2. **Hyväksyntätestit.** Kun olet valinnut testit portaalissa ja tunnukset ovat
   paikallaan, agentti ajaa ne:

   ```sh
   aggountant vero-fi verify-portal
   ```

   Komento ajaa Verohallinnon kolme kiinteää testiskenaariota
   tuotantovarmenteellasi. Se ei tee ilmoitusta eikä muuta tilaa, mutta se on
   ensimmäinen yhteys Verohallinnon tuotantoon. Siksi agentti pyytää siihen
   luvan. Onnistunut ajo tallentaa näytön asetuksiin ja kirjaa testit
   hyväksytyiksi.
3. **Vakuutukset.** Kun olet tehnyt portaalissa rekisteröinnin, sopimuksen ja
   tuotantoluvan, vahvistat ne agentille yksi kerrallaan. Ohjelma ei voi
   tarkistaa niitä vaan luottaa sanaasi, joten vahvista vain tehdyt asiat, ei
   odotettuja.
4. **Tuotantoon.** Agentti kirjoittaa asetukset uudelleen tilassa `production`
   ja sitoo tuotantoaktivoinnin nykyisen varmenteen sormenjälkeen. Ohjelma
   hyväksyy tuotantotilan vain, jos valmiustarkistus menee läpi. Muuten se
   kieltäytyy (`VERO_NOT_READY`), eikä mikään muutu.

`configure` korvaa koko asetustiedoston. Agentin pitää kuljettaa aiemmat
näyttötiedot mukana, tai muuten testinäyttö katoaa ja tuotanto estyy.
`vero-fi`-skilli ohjaa agenttia tässä.

`aggountant vero-fi verify --target test` on vapaaehtoinen vianetsintä
testiympäristöstä erillisillä testitunnuksilla. Sitä ei tarvita tuotantoon.

### Varmenteen uusiminen

Uudella varmenteella on uusi sormenjälki, joten vanhaan varmenteeseen sidottu
näyttö ei kelpaa. Kun vaihdat varmenteen, aja `verify-portal` uudelleen ja
siirry tuotantoon uudella sormenjäljellä. Vahvista agentille ne vakuutukset,
jotka koskevat uutta varmennetta.

## Suora ALV-ilmoitus

1. **Esikatselu** (yllä) antaa luvut.
2. **Ilmoitusajo käynnistetään** esikatselun luvuilla:

   ```sh
   aggountant file-vat start < ilmoitus.json
   ```

   Syötteessä ovat kausi, myynnin vero, ostojen vero ja Verohallinnon tilinumero
   (`payee_iban`), joka näkyy OmaVerossa. Ohjelma laskee luvut uudelleen
   kirjanpidosta ja kieltäytyy, jos ne eroavat (`VAT_FIGURES_MISMATCH`).
   Ajo tarkistaa valmiuden, hakee kauden tilan Verohallinnosta ja jää
   odottamaan hyväksyntääsi (`ApproveFiling`). Kauden pitää olla Verohallinnossa
   vielä ilmoittamatta.
3. **Sinä hyväksyt.** Agentti näyttää yrityksen, kauden, eräpäivän, numeroidut
   kentät, nettosumman ja maksettavan määrän. Hyväksyntä on sidottu juuri tähän
   lähetettävään sisältöön:

   ```sh
   aggountant file-vat approve-filing --run-id fv-… --answer approve
   ```

   Hylkäys (`--answer reject`) päättää ajon. Uusi kierros alkaa esikatselusta.
4. **Lähetys.** Kun Verohallinto vastaanottaa ilmoituksen, ohjelma tallentaa sen
   kuittauksen (tunniste ja aikaleima) ja merkitsee kauden ilmoitetuksi.

Ajon vaiheet näet milloin tahansa komennolla
`aggountant run show --run-id fv-…`.

**Maksu.** Jos veroa on maksettavaa, ajo avaa vielä maksun hyväksynnän
(`ApprovePayment`). aggountantilla ei ole maksuyhteyttä, joten tämä vaihe ei voi
valmistua, ja ajo jää näyttämään odottavaa tilaa. Ilmoitus on silti tehty.
Maksa vero OmaVeron viitteellä itse. Palautukseen oikeuttava ilmoitus päättyy
ilman maksuvaihetta.

### Ilmoitus ei näy OmaVerossa

Kuittaus tarkoittaa, että Verohallinto vastaanotti ilmoituksen, ei sitä, että
ilmoitus olisi jo käsitelty. Käytännössä OmaVero on näyttänyt rajapinnan kautta
tulleen ilmoituksen vasta noin puolen vuorokauden kuluttua. Verohallinnon tuen
mukaan noin 12 tunnin viive on normaali. Tämä on käytännön havainto, ei luvattu
palveluaika.

**Älä lähetä ilmoitusta uudelleen sen takia, että se ei vielä näy.** Toinen
lähetys olisi Verohallinnossa korvaava ilmoitus, eikä aggountant edes salli
toista lähetystä samalle kaudelle (`PERIOD_TRANSITION_CONFLICT`). Odota
seuraavaan arkipäivään. Jos ilmoitus ei sittenkään näy tai eräpäivä on lähellä,
soita Verohallinnon ALV-palveluun. Agentti kokoaa sinulle puhelua varten kauden,
kuittauksen tunnisteen ja aikaleiman, Y-tunnuksen ja lähetetyt kentät.

### Epävarma lopputulos

Jos yhteys katkeaa tai Verohallinnon palvelu vastaa virheellä kesken lähetyksen,
ohjelma ei tiedä, menikö ilmoitus perille. Silloin ajo jää huomiota vaativaan
tilaan, eikä ilmoitusta lähetetä uudelleen. Agentti kysyy tilan Verohallinnolta:

```sh
aggountant file-vat reconcile --run-id fv-…
```

Jos Verohallinnolla on täsmälleen sama ilmoitus, ajo päättyy ilman uutta
lähetystä. Jos ilmoitusta ei näy tai se poikkeaa, ajo jää odottamaan, koska
Verohallinto ei välttämättä vielä näytä uusinta tietoa. Odota ja kysy uudelleen
tai tarkista OmaVero. Ilmoitusta ei koskaan lähetetä uudelleen sokkona.
`file-vat retry` koskee vain maksuvaihetta.

## Ilmoitus käsin OmaVerossa

Jos suora ilmoittaminen ei ole käytössä tai haluat asioida itse, syötä
esikatselun numeroidut kentät OmaVeroon (tai laadi Ilmoitin.fi-tiedosto itse) ja
maksa vero. Arkistoi sen jälkeen OmaVeron vahvistus ja kirjaa ilmoitus tehdyksi:

```sh
aggountant artifact attach --file omavero-vahvistus-2026-03.pdf
aggountant file-vat complete-manual --period 2026-03 --channel omavero \
  --evidence-ref <vahvistuksen tunniste>
```

Kanava on `omavero` tai `ilmoitin`. Komento ei lähetä eikä maksa mitään. Se
merkitsee kauden ilmoitetuksi, mikä on pysyvä tila. Jos samalle kaudelle on jäänyt
epävarma rajapintalähetys, komento vaatii lisäksi näytön siitä, että
Verohallinnon tila on selvitetty (`--remote-status-evidence-ref`). Muuten
käsin tehty ilmoitus voisi huomaamatta olla kaksoisilmoitus.

## Kuukausi- ja neljännesvuosikaudet

ALV-ilmoitus tehdään samalle kaudelle, jolla kirjanpito suljetaan: kuukaudelle
(`2026-03`) tai neljännekselle (`2026-Q1`). Suoraa ilmoittamista on koeteltu
kuukausikausilla.
