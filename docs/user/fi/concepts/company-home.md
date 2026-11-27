# Yrityskoti: mitä tallentuu minne

Yrityksen koko kirjanpito on yhdessä hakemistossa, yrityskodissa: tilikartta,
tositteet, liitteet, kausien tila, yrityksen tallennetut tiedot ja asetukset.

## Yksi koti, yksi yritys

Jos hoidat kahden yrityksen kirjanpitoa, niillä on kaksi erillistä kotia.
Kotia ei ole tehty usean ihmisen tai koneen yhtäaikaiseen käyttöön.

## Miten ohjelma löytää kodin

aggountant etsii kodin tässä järjestyksessä ja käyttää ensimmäistä, jonka se
löytää:

1. komennon `--home`-valitsin
2. ympäristömuuttuja `AGGOUNTANT_HOME`
3. asetustiedoston `home`-arvo
4. lähin `.aggountant`-hakemisto nykyisestä hakemistosta ylöspäin
5. nykyisen hakemiston alihakemisto `accounting`.

Asetustiedosto on JSON-muotoinen, ja sen paikan kertoo
`aggountant config path`. Kotia varten siihen kirjoitetaan
`{"home": "/polku/kotiin"}`. Samaan tiedostoon tallentuu suoran
ALV-ilmoittamisen tunnustiedoston polku, jonka ohjelma kirjoittaa itse.

`.aggountant`-hakemiston etsintä pysähtyy git-repositorion juureen, ja koti on
se hakemisto, jossa `.aggountant` on. Kodin perustaminen ei luo tätä
hakemistoa. Agentti luo sen kirjoittaessaan Tulorekisteri-profiilin, ja voit
luoda sen itse kodin juureen (`mkdir .aggountant`), jos haluat kodin sisällä
ajettujen komentojen löytävän kodin.

`aggountant config show` kertoo, mikä koti on käytössä ja mistä tieto tuli,
esimerkiksi `home /…/esimerkki-oy (flag)`. Vain `init` luo kodin. Jos polku
on kirjoitettu väärin, muut komennot pysähtyvät koodiin `HOME_NOT_INITIALIZED`
ja kertovat, mitä polkua ne katsoivat ja miten polku valittiin. Vianetsintä on
kohdassa
[Tositteet näyttävät kadonneen](../troubleshooting.md#tositteet-näyttävät-kadonneen).

## Git ja varmuuskopio

Kodin kannattaa olla git-repositorio. Gitiin tallentuvat tilikartta, yrityksen
tiedot, ajojen tila ja asetukset, joten muutoksista jää historia, jonka voit
tarkastaa.

Git ei silti ole kirjanpitosi varmuuskopio, koska kodin `.gitignore` rajaa
siitä pois kirjanpidon tärkeimmän osan: tositetietokannan ja arkistoidut
liitteet. Pois jäävät myös henkilötiedot, koska niitä ei saa tallentaa
historiaan, josta niitä ei voi poistaa. Siksi
[koko koti varmuuskopioidaan erikseen](../tasks/back-up-and-restore.md).

## Yrityksen tietovarasto

Agentti tallentaa työssä opitut yrityksen faktat ja käytännöt kodin
`knowledge/company/`-hakemistoon, esimerkiksi tilikauden, tavan kirjata tietyn
toimittajan laskut tai sen, että puhelinliittymä on kokonaan yrityksen
käytössä. Jokaisella tiedolla on lähde ja historia, joten näet myöhemmin, mistä
tieto tuli ja milloin se muuttui. Vanhentunutta tietoa ei poisteta. Se
merkitään pois käytöstä (`knowledge company retire`), ja aiempi tieto ja
muutoksen syy säilyvät.

Voit pyytää agenttia näyttämään tiedot, esimerkiksi "mitä tiedät
puhelinliittymästä?" Agentti käyttää silloin komentoja
`aggountant knowledge company list`, `aggountant knowledge company search` ja
`aggountant knowledge company history`.

## Kodin AGENTS.md

Kodin juuren `AGENTS.md`-tiedostossa kerrot omin sanoin, mistä kuitit tulevat
ja minne käsitelty aineisto viedään, koska aggountant ei määrää kumpaakaan.
Agentti lukee tiedoston ennen kuin se etsii, kopioi tai siirtää mitään.
Tiedosto kirjoitetaan, kun
[otat yrityksesi kirjanpidon käyttöön](../start/set-up-company.md).

## Kodin TODO.md

Kodin juuren `TODO.md` on muistilista siitä, mikä odottaa: muiden lupaamat
laskut ja kuitit, avoimet kuukaudet ja se, mitä niiden sulku vielä vaatii,
sekä määräajat, kuten ALV-ilmoitus ja tilinpäätös. Agentti lukee sen istunnon
alussa ja päivittää sen samalla, kun kirjaa asian, joka ratkaisee kohdan,
joten seuraava istunto jatkaa siitä, mihin edellinen jäi. Tiedosto on
tavallista tekstiä, jota voit lukea ja muokata itsekin. Ohjelma ei lue sitä.

Odottava asia ei kuulu yrityksen tietovarastoon, koska siellä on vain pysyviä
faktoja lähteineen. Myös [velvoitteet](periods.md#velvoitteet)
(`aggountant obligation`) ovat eri asia: ne syntyvät ohjelmassa ja estävät
kauden sulun.

## Salaisuudet eivät kuulu kotiin

Salasanoja, tunnuksia ja avaimia ei kirjoiteta `AGENTS.md`- tai
`TODO.md`-tiedostoon, yrityksen tietovarastoon eikä keskusteluun agentin
kanssa, koska koti on git-versioitua tekstiä ja keskustelut tallentuvat usein
lokeihin. Vero.fi- ja Tulorekisteri-tunnukset ovat omassa tiedostossaan kodin
ulkopuolella, ja ohjelma kieltäytyy käyttämästä tunnustiedostoa, joka on kodin
sisällä.

## Mitä kodissa on

| Kodissa | Mitä se on | Kuuluuko gitiin |
|---|---|---|
| `masterdata/reference/` | tilikartta ja ALV-roolit | kyllä |
| `knowledge/company/` | yrityksen tallennetut faktat ja käytännöt | kyllä |
| `process/` | sulku- ja ilmoitusajojen tila, avoimet velvoitteet | kyllä |
| `vero-filing.json` | suoran ALV-ilmoittamisen asetukset ilman salaisuuksia | kyllä |
| `.aggountant/tulorekisteri.json` | Tulorekisteri-profiili ilman salaisuuksia | kyllä |
| `AGENTS.md` | yrityksen oma kuvaus aineistolähteistä | kyllä |
| `TODO.md` | odottavat asiat, avoimet kuukaudet ja määräajat | kyllä |
| `fiscal.sqlite` | sinetöidyt tositteet, kausien tila ja tositeketju | **ei** |
| `artifacts/` | arkistoidut kuitit, laskut ja tiliotteet | **ei** |
| `masterdata/pii.sqlite` | työntekijöiden henkilötiedot ja kululaskun tekijöiden varsinaiset työpaikat | **ei** |

Osa hakemistoista syntyy vasta, kun niitä tarvitaan.

## Katso myös

- [Varmuuskopioi ja palauta kirjanpito](../tasks/back-up-and-restore.md)
- [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md)
- [Sinä, agentti ja aggountant](division-of-work.md)
