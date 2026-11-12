# Yrityskoti: mitä tallentuu minne

Yrityksen koko kirjanpito on yhdessä hakemistossa, yrityskodissa. Tämä sivu
kertoo, miten ohjelma löytää kodin, mitä kotiin tallentuu, mikä kuuluu gitiin
ja mikä ei, ja mitä kotiin ei koskaan kirjoiteta.

## Yksi koti, yksi yritys

Kodissa ovat yrityksen tilikartta, tositteet, liitteet, kausien tila,
yrityksen tallennetut tiedot ja asetukset. Jos hoidat kahden yrityksen
kirjanpitoa, niillä on kaksi erillistä kotia. Kotia ei ole tarkoitettu
käytettäväksi usealta ihmiseltä tai koneelta yhtä aikaa.

## Miten ohjelma löytää kodin

Jokainen komento tarvitsee tiedon siitä, minkä yrityksen kirjanpitoa se
käsittelee. Ohjelma etsii kodin tässä järjestyksessä ja käyttää ensimmäisen
löytämänsä:

1. `--home`-valitsin komennossa
2. ympäristömuuttuja `AGGOUNTANT_HOME`
3. asetustiedoston `home`-arvo (tiedoston paikan kertoo `aggountant config path`)
4. lähin `.aggountant`-hakemisto nykyisestä hakemistosta ylöspäin. Etsintä
   pysähtyy git-repositorion juureen, ja koti on se hakemisto, jossa
   `.aggountant` on. Kodin perustaminen ei luo tätä hakemistoa. Agentti luo
   sen kirjoittaessaan Tulorekisteri-profiilin, ja voit luoda sen itse kodin
   juureen (`mkdir .aggountant`), jos haluat kodin sisällä ajettujen komentojen löytävän kodin.
5. nykyisen hakemiston alihakemisto `accounting`.

`aggountant config show` kertoo, mikä koti on käytössä ja mistä tieto tuli,
esimerkiksi `home /…/esimerkki-oy (flag)`. Jos polku on kirjoitettu väärin,
komento voi osua tyhjään kotiin, jolloin tositteet näyttävät kadonneen
([Kun jokin menee vikaan](../troubleshooting.md#tositteet-näyttävät-kadonneen)).

Asetustiedosto on JSON-muotoinen. Kotia varten siihen kirjoitetaan
`{"home": "/polku/kotiin"}`. Samaan tiedostoon tallentuu suoran
ALV-ilmoittamisen tunnustiedoston polku, mutta sen kirjoittaa ohjelma itse.

## Git ja varmuuskopio

Kodin kannattaa olla git-repositorio. Gitiin tallentuvat tilikartta, yrityksen
tiedot, ajojen tila ja asetukset, joten muutoksista jää historia, jonka voit
tarkastaa.

**Git ei kuitenkaan ole varmuuskopio kirjanpidostasi.** Kodin `.gitignore`
rajaa gitistä pois tositetietokannan, arkistoidut liitteet ja henkilötiedot.
Tositetietokanta ja liitteet ovat kirjanpidon tärkein osa, ja henkilötietoja
ei saa tallentaa historiaan, josta niitä ei voi poistaa. Siksi koko koti
varmuuskopioidaan erikseen:
[Varmuuskopioi ja palauta kirjanpito](../tasks/back-up-and-restore.md).

## Yrityksen tietovarasto

Agentti tallentaa työssä opitut yrityksen faktat ja käytännöt kodin
`knowledge/company/`-hakemistoon, esimerkiksi tilikauden, tavan kirjata tietyn
toimittajan laskut tai sen, että puhelinliittymä on kokonaan yrityksen
käytössä. Jokaisella tiedolla on lähde ja historia, joten näet myöhemmin, mistä
tieto tuli ja milloin se muuttui. Vanhentunutta tietoa ei poisteta, vaan se
merkitään pois käytöstä (`knowledge company retire`), jolloin aiempi tieto ja muutoksen syy säilyvät.

Voit pyytää agenttia näyttämään tiedot: "mitä tiedät puhelinliittymästä?"
Agentti käyttää silloin komentoja `aggountant knowledge company list`,
`aggountant knowledge company search` ja `aggountant knowledge company history`.

## Kodin AGENTS.md

aggountant ei määrää, mistä kuitit tulevat tai minne käsitelty aineisto
viedään. Se kerrotaan kodin juuren `AGENTS.md`-tiedostossa omin sanoin.
Agentti lukee sen ennen kuin se etsii, kopioi tai siirtää mitään. Tiedosto
kirjoitetaan käyttöönotossa:
[Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md).

## Salaisuudet eivät kuulu kotiin

**Salasanoja, tunnuksia ja avaimia ei kirjoiteta `AGENTS.md`-tiedostoon,
yrityksen tietovarastoon eikä keskusteluun agentin kanssa.** Koti on
git-versioitua tekstiä, ja keskustelut tallentuvat usein lokeihin. Vero.fi- ja
Tulorekisteri-tunnukset ovat omassa tiedostossaan kodin ulkopuolella, ja
ohjelma kieltäytyy käyttämästä tunnustiedostoa, joka on kodin sisällä.

## Mitä kodissa on

| Kodissa | Mitä se on | Kuuluuko gitiin |
|---|---|---|
| `masterdata/reference/` | tilikartta ja ALV-roolit | kyllä |
| `knowledge/company/` | yrityksen tallennetut faktat ja käytännöt | kyllä |
| `process/` | sulku- ja ilmoitusajojen tila, avoimet velvoitteet | kyllä |
| `vero-filing.json` | suoran ALV-ilmoittamisen asetukset ilman salaisuuksia | kyllä |
| `.aggountant/tulorekisteri.json` | Tulorekisteri-profiili ilman salaisuuksia | kyllä |
| `AGENTS.md` | yrityksen oma kuvaus aineistolähteistä | kyllä |
| `fiscal.sqlite` | sinetöidyt tositteet, kausien tila ja tositeketju | **ei** |
| `artifacts/` | arkistoidut kuitit, laskut ja tiliotteet | **ei** |
| `masterdata/pii.sqlite` | työntekijöiden henkilötiedot | **ei** |

Osa hakemistoista syntyy vasta, kun niitä tarvitaan.

## Katso myös

- [Varmuuskopioi ja palauta kirjanpito](../tasks/back-up-and-restore.md)
- [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md)
- [Sinä, agentti ja aggountant](division-of-work.md)
