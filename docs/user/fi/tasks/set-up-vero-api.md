# Ota suora ALV-ilmoittaminen käyttöön

Suorassa ilmoittamisessa agentti lähettää ALV-ilmoituksen Verohallinnolle
Vero API -rajapinnan kautta yrityksesi omilla tunnuksilla. Käyttöönotossa on
useita vaiheita ja odotusaikoja, joten aloita se hyvissä ajoin. Siihen asti
teet ilmoituksen [OmaVerossa](file-vat-omavero.md).

## Ennen kuin aloitat

- aggountantilla ei ole yhteistä ohjelmistoavainta. Jokainen käyttäjä
  rekisteröi oman ohjelmistonsa Verohallinnon API-portaalissa ja saa oman
  avaimen, ja yritys hankkii oman varmenteensa.
- Portaali-, sopimus- ja varmennetoimet teet itse, koska ne ovat yrityksen
  oikeustoimia. Agentti kokoaa tiedot, tarkistaa valmiuden, ajaa tarkistukset
  luvallasi ja kertoo, kenen vuoro kunkin esteen poistamisessa on.
- Tämä sivu kuvaa toimet sillä tarkkuudella kuin aggountant ne tarkistaa.
  Niiden täsmälliset nimet ja järjestys ovat Verohallinnon ohjeissa.

## Mitä tarvitset Verohallinnolta

1. **Rekisteröi ohjelmistosi** Vero API -portaalissa ja ota talteen sen
   antama ohjelmistoavain.
2. **Tee yrityksellesi rajapintasopimus**, eli Vero API -sopimus. Lähetetty
   hakemus ei riitä, vaan sopimuksen on oltava voimassa, ennen kuin
   vahvistat sen agentille.
3. **Hae yrityksellesi tuotantovarmenne** Verohallinnon varmennepalvelusta.
   Saat varmenteen, sen yksityisen avaimen ja varmentajan (CA) varmenteen.
4. **Valitse portaalissa ALV-rajapinnan hyväksyntätestit.** Agentti ajaa ne
   alempana vaiheessa 3.
5. **Hanki tuotantolupa.** Kun testit on hyväksytty, aktivoit tuotantokäytön
   portaalissa.

## Näin teet sen

1. **Tallenna tunnukset itse tiedostoon.** Tee JSON-tiedosto kirjanpitokodin
   ulkopuolelle, esimerkiksi
   `~/.config/aggountant/credentials/vero-production.json`. Siinä ovat:
   - `target`: `"production"`
   - `business_id`: yrityksesi Y-tunnus
   - `software_key`: ohjelmistoavaimesi
   - `client_certificate_pem`, `client_private_key_pem` ja
     `ca_certificate_pem`: varmenne, sen yksityinen avain ja varmentajan
     varmenne PEM-muodossa.

   Aseta oikeudet niin, että vain sinä voit lukea tiedoston, äläkä lisää sitä
   gittiin. **Älä liitä tunnuksia keskusteluun agentin kanssa**, koska
   keskustelut tallentuvat usein lokeihin. Kerro agentille vain tiedoston
   polku.

2. **Pyydä agenttia tekemään perusasetukset.**

   > Otetaan suora ALV-ilmoittaminen käyttöön. Tunnukset ovat tiedostossa
   > ~/.config/aggountant/credentials/vero-production.json.

   Agentti kirjoittaa kodin `vero-filing.json`-tiedostoon Y-tunnuksen sekä
   yhteyshenkilön nimen ja puhelinnumeron. Tila on aluksi `disabled`.
   Tunnustiedoston polku tallentuu koneen omaan asetustiedostoon eikä kotiin,
   joten jokaisella koneella voi olla omat tunnuksensa. Sitten agentti
   tarkistaa valmiuden ja kertoo, mitä esteitä on jäljellä ja kenen ne ovat.

3. **Anna lupa hyväksyntätesteihin.** Kun olet valinnut testit portaalissa,
   agentti ajaa Verohallinnon kolme kiinteää testiskenaariota
   tuotantovarmenteellasi. Se pyytää luvan, koska ajo on ensimmäinen yhteys
   Verohallinnon tuotantoon. Ajo ei tee ilmoitusta eikä muuta tilaa. Kun se
   onnistuu, ohjelma tallentaa siitä näytön asetuksiin ja kirjaa testit
   hyväksytyiksi.

4. **Vahvista portaalitoimet agentille** yksi kerrallaan, kun olet tehnyt
   portaalissa rekisteröinnin, sopimuksen ja tuotantoluvan. Ohjelma ei voi
   tarkistaa niitä vaan luottaa sanaasi, joten vahvista vain se, minkä olet
   jo tehnyt.

5. **Hyväksy siirtyminen tuotantoon.** Agentti kirjoittaa asetukset uudelleen
   tilassa `production` ja sitoo tuotantoaktivoinnin nykyisen varmenteen
   sormenjälkeen. Jos valmiuden tarkistus ei mene läpi, ohjelma kieltäytyy
   eikä mikään muutu.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant vero-fi status
aggountant vero-fi configure --dry-run < asetukset.json
aggountant vero-fi configure < asetukset.json
aggountant vero-fi verify-portal
```

`status` tarkistaa valmiuden paikallisesti ilman verkkoyhteyttä, joten sen
voi ajaa milloin tahansa. Ohjelma tekee saman tarkistuksen juuri ennen
jokaista lähetystä.

`configure` yhdistää annetut kentät tallennettuihin asetuksiin. Kenttä, jota
syöte ei mainitse, pysyy ennallaan, ja `--dry-run` luettelee jokaisen kentän,
joka muuttuisi. Ohjelma hylkää syötteen, joka poistaisi tai muuttaisi
`verify-portal`-ajon tallentaman näytön. `--replace` kirjoittaa syötteen koko
asetustiedostoksi, ja sitä tarvitaan vain, kun näyttö poistetaan
tarkoituksella tai rikkinäinen asetustiedosto korvataan.

`verify-portal` ajaa hyväksyntätestit.

`aggountant vero-fi verify --target test` on vapaaehtoinen. Sillä voi etsiä
vikaa testiympäristössä erillisillä testitunnuksilla, eikä sitä tarvita
tuotantoon.
</details>

## Valmis, kun

`aggountant vero-fi status` kertoo, että valmius on kunnossa (`ready`), eikä
esteitä ole.

## Kun varmenne vaihtuu

Varmenne on voimassa kaksi vuotta, ja ohjelma varoittaa 90, 30 ja 7 päivää
ennen sen vanhenemista. Uudella varmenteella on uusi sormenjälki, joten
vanhaan varmenteeseen sidottu näyttö ei kelpaa:

1. Päivitä uusi varmenne ja avain tunnustiedostoon.
2. Pyydä agenttia ajamaan hyväksyntätestit uudelleen ja siirtymään tuotantoon
   uudella sormenjäljellä. Ohjelma sitoo testien näytön ja
   tuotantoaktivoinnin uuteen varmenteeseen.

Rekisteröinnin, sopimuksen ja tuotantoluvan vahvistukset pysyvät voimassa,
eikä niitä tarvitse antaa uudelleen.

Kun tarvitset varmenteen tunnistetta esimerkiksi Verohallinnon tuelle, käytä
`vero-fi status` -komennon näyttämää SHA-256-sormenjälkeä. Se ei ole salainen.

## Jos jokin menee pieleen

- Jos esteitä on jäljellä, jokaisella niistä on koodi ja omistaja. Ne on
  lueteltu sivulla
  [Vero.fi-valmiuden esteet](../reference/vero-fi-blockers.md).
- Jos ohjelmistoavain tai varmenteen yksityinen avain on vuotanut, ota
  yhteyttä Verohallintoon samalla tavalla kuin minkä tahansa muun tunnuksen
  vuotaessa ja hanki uudet.

## Katso myös

- [Lähetä ALV-ilmoitus suoraan Verohallinnolle](file-vat-direct.md)
- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
- [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
