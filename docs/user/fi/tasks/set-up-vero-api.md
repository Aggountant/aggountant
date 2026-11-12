# Ota suora ALV-ilmoittaminen käyttöön

Suora ilmoittaminen on yrityksesi oma yhteys Verohallintoon: agentti lähettää
ALV-ilmoituksen Vero API -rajapinnan kautta yrityksesi omilla tunnuksilla.
Käyttöönotossa on useita vaiheita ja odotusaikoja, joten aloita se hyvissä
ajoin. Siihen asti teet ilmoituksen
[OmaVerossa](file-vat-omavero.md). Sivun lopussa kerrotaan, mitä teet, kun
varmenne vaihtuu.

## Ennen kuin aloitat

- **Jokainen käyttäjä rekisteröi oman ohjelmistonsa ja saa oman
  ohjelmistoavaimensa** Verohallinnon API-portaalista, ja yritys hankkii oman
  varmenteensa. aggountantilla ei ole yhteistä avainta, jonka voisi lainata.
- Portaali-, sopimus- ja varmennetoimet teet itse, koska ne ovat yrityksen
  oikeustoimia. Agentti ei asioi portaaleissa puolestasi. Se kokoaa tiedot,
  tarkistaa valmiuden, ajaa tarkistukset luvallasi ja kertoo, kenen vuoro
  kunkin esteen poistamisessa on.
- Toimien täsmälliset nimet ja järjestys ovat Verohallinnon ohjeissa. Tällä
  sivulla ne on kuvattu sillä tarkkuudella kuin aggountant ne tarkistaa.

## Mitä tarvitset Verohallinnolta

1. **Ohjelmiston rekisteröinti ja ohjelmistoavain.** Rekisteröi ohjelmistosi
   Vero API -portaalissa ja ota talteen sen antama ohjelmistoavain.
2. **Rajapintasopimus.** Yrityksen Vero API -sopimus tai -hakemus.
3. **Tuotantovarmenne.** Hae yrityksellesi varmenne Verohallinnon
   varmennepalvelusta. Saat varmenteen, sen yksityisen avaimen ja varmentajan
   (CA) varmenteen. Varmenne on voimassa kaksi vuotta.
4. **ALV:n hyväksyntätestit.** Valitse portaalissa ALV-rajapinnan
   hyväksyntätestit. aggountant ajaa ne puolestasi (vaihe 3 alla).
5. **Tuotantolupa.** Kun testit on hyväksytty, portaalissa aktivoidaan
   tuotantokäyttö.

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
   keskustelut tallentuvat usein lokeihin. Kerro agentille vain tiedoston polku.

2. **Pyydä agenttia tekemään perusasetukset.**

   > Otetaan suora ALV-ilmoittaminen käyttöön. Tunnukset ovat tiedostossa
   > ~/.config/aggountant/credentials/vero-production.json.

   Agentti kirjoittaa kodin `vero-filing.json`-tiedoston: Y-tunnus,
   yhteyshenkilön nimi ja puhelinnumero. Tila on aluksi `disabled`.
   Tunnustiedoston polku tallentuu koneen omaan asetustiedostoon, ei kotiin,
   joten jokaisella koneella voi olla omat tunnuksensa. Sitten agentti
   tarkistaa valmiuden ja kertoo jäljellä olevat esteet ja niiden omistajat.

3. **Anna lupa hyväksyntätesteihin.** Kun olet valinnut testit portaalissa,
   agentti ajaa Verohallinnon kolme kiinteää testiskenaariota
   tuotantovarmenteellasi. Ajo ei tee ilmoitusta eikä muuta tilaa, mutta se on
   ensimmäinen yhteys Verohallinnon tuotantoon, ja siksi agentti pyytää luvan.
   Onnistunut ajo tallentaa näytön asetuksiin ja kirjaa testit hyväksytyiksi.

4. **Vahvista portaalitoimet.** Kun olet tehnyt portaalissa rekisteröinnin,
   sopimuksen ja tuotantoluvan, vahvistat ne agentille yksi kerrallaan.
   Ohjelma ei voi tarkistaa niitä vaan luottaa sanaasi, joten **vahvista vain
   tehdyt asiat, älä odotettuja**.

5. **Hyväksy siirtyminen tuotantoon.** Agentti kirjoittaa asetukset uudelleen
   tilassa `production` ja sitoo tuotantoaktivoinnin nykyisen varmenteen
   sormenjälkeen. Ohjelma hyväksyy tuotantotilan vain, jos valmiuden tarkistus
   menee läpi. Muuten se kieltäytyy, eikä mikään muutu.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant vero-fi status
aggountant vero-fi configure --dry-run < asetukset.json
aggountant vero-fi configure < asetukset.json
aggountant vero-fi verify-portal
```

`configure` korvaa koko asetustiedoston. Agentin pitää kuljettaa aiemmat
näyttötiedot mukana, tai muuten testinäyttö katoaa ja tuotanto estyy.
`vero-fi`-skilli ohjaa agenttia tässä.

`aggountant vero-fi verify --target test` on vapaaehtoinen vianetsintä
testiympäristöstä erillisillä testitunnuksilla. Sitä ei tarvita tuotantoon.
</details>

## Valmis, kun

`aggountant vero-fi status` kertoo, että valmius on kunnossa (`ready`), eikä
esteitä ole. Tarkistus on paikallinen eikä ota verkkoyhteyttä, joten sen voi
ajaa milloin tahansa. Sama tarkistus tehdään automaattisesti juuri ennen
jokaista lähetystä.

## Kun varmenne vaihtuu

Varmenne on voimassa kaksi vuotta, ja ohjelma varoittaa 90, 30 ja 7 päivää
ennen sen vanhenemista. Uudella varmenteella on uusi sormenjälki, joten
vanhaan varmenteeseen sidottu näyttö ei kelpaa:

1. Päivitä uusi varmenne ja avain tunnustiedostoon.
2. Pyydä agenttia ajamaan hyväksyntätestit uudelleen ja siirtymään tuotantoon
   uudella sormenjäljellä.
3. Vahvista agentille ne portaalitoimet, jotka koskevat uutta varmennetta.

Kun tarvitset varmenteen tunnistetta esimerkiksi Verohallinnon tuelle, käytä
`vero-fi status` -komennon näyttämää SHA-256-sormenjälkeä. Se ei ole salainen.

## Jos jokin menee pieleen

- **Esteitä on jäljellä.** Jokaisella esteellä on koodi ja omistaja:
  [Vero.fi-valmiuden esteet](../reference/vero-fi-blockers.md).
- **Ohjelmistoavain tai varmenteen yksityinen avain on vuotanut.** Ota yhteyttä
  Verohallintoon samalla tavalla kuin minkä tahansa muun tunnuksen
  vuotaessa, ja hanki uudet.

## Katso myös

- [Lähetä ALV-ilmoitus suoraan Verohallinnolle](file-vat-direct.md)
- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
- [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
