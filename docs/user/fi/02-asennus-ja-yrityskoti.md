# 2. Asennus ja yrityskodin perustaminen

## Ohjelman asennus

Valmiita asennuspaketteja ei vielä ole. Toistaiseksi aggountant käännetään
lähdekoodista, mihin tarvitaan Rust-työkalut (versio 1.89 tai uudempi, asennus
osoitteesta <https://rustup.rs>). Lähdekoodihakemistossa asennus tehdään näin:

```sh
cargo install --locked --path crates/cli
```

Tarkista, että ohjelma löytyy:

```sh
aggountant --version
aggountant --help
```

`aggountant --help` listaa kaikki komennot, ja jokaisella komennolla on oma
ohjeensa, esimerkiksi `aggountant voucher seal --help`. Agentti lukee näitä
ohjeita itse. Sinun ei tarvitse opetella komentoja ulkoa.

## Mikä yrityskoti on

Yrityskoti on yksi hakemisto, jossa on yhden yrityksen koko kirjanpito:

| Kodissa | Mitä se on | Kuuluuko gitiin |
|---|---|---|
| `masterdata/reference/` | tilikartta ja ALV-roolit | kyllä |
| `knowledge/company/` | yrityksen tallennetut faktat ja käytännöt | kyllä |
| `process/` | sulku- ja ilmoitusajojen tila, avoimet velvoitteet | kyllä |
| `vero-filing.json` | Vero.fi-asetukset ilman salaisuuksia | kyllä |
| `AGENTS.md` | yrityksen oma kuvaus aineistolähteistä (luvussa 3) | kyllä |
| `fiscal.sqlite` | sinetöidyt tositteet, kausien tila ja ketju | **ei** |
| `artifacts/` | arkistoidut kuitit, laskut ja tiliotteet | **ei** |
| `masterdata/pii.sqlite` | työntekijöiden henkilötiedot | **ei** |

## Kodin perustaminen

Valitse kodille paikka ja aja `init`:

```sh
aggountant --home ~/kirjanpito/esimerkki-oy init
```

Komento luo hakemiston, tallennusrakenteet, oletustilikartan (`fi-general-2026`,
98 tiliä) ja `.gitignore`-tiedoston. Jos koti on jo olemassa, `init` kieltäytyy
(`ALREADY_INITIALIZED`) eikä koske siihen. Komennolla ei siis voi vahingossa
tyhjentää olemassa olevaa kirjanpitoa, eikä sillä voi myöskään korjata tai
päivittää vanhaa kotia.

`init` ei kysy yrityksen nimeä, Y-tunnusta tai tilikautta. Ne kerätään
käyttöönotossa yrityksen tietovarastoon, ja agentti hoitaa tämän
`aggountant-init`-skillillä (luku 3).

Tilikartan tilit näet komennolla:

```sh
aggountant chart list
```

## Miten ohjelma löytää kodin

Jokainen komento tarvitsee tiedon siitä, minkä yrityksen kirjanpitoa se käsittelee.
Ohjelma etsii kodin tässä järjestyksessä ja käyttää ensimmäisen löytämänsä:

1. `--home`-valitsin komennossa
2. ympäristömuuttuja `AGGOUNTANT_HOME`
3. asetustiedoston `home`-arvo (tiedoston paikan kertoo `aggountant config path`)
4. lähin `.aggountant`-hakemisto nykyisestä hakemistosta ylöspäin. Etsintä
   pysähtyy git-repositorion juureen. Koti on se hakemisto, jossa
   `.aggountant` on. `init` ei luo tätä tunnistetta: luo se itse kodin juureen
   (`mkdir .aggountant`), jos haluat kodin sisällä ajettujen komentojen
   löytävän kodin. Agentti luo sen myös kirjoittaessaan Tulorekisteri-profiilin
   (luku 6).
5. nykyisen hakemiston alihakemisto `accounting`.

Komento `aggountant config show` kertoo, mikä koti on käytössä ja mistä tieto
tuli. Aja se aina, jos et ole varma. Jos kirjoitat kodin polun väärin, ohjelma
voi luoda väärään paikkaan tyhjän hakemiston (tunnettu puute). Se ei vahingoita
oikeaa kirjanpitoa, mutta tositteet näyttävät kadonneen, kunnes huomaat
virheen.

Asetustiedosto on JSON-muotoinen. Kotia varten siihen kirjoitetaan
`{"home": "/polku/kotiin"}`. Samaan tiedostoon tallentuu myös Vero.fi-tunnusten
polku (luku 5), mutta sen kirjoittaa ohjelma itse.

## Git ja varmuuskopio

Kodin kannattaa olla git-repositorio. Gitiin tallentuvat tilikartta, yrityksen
faktat, ajojen tila ja asetukset, joten muutoksista jää historia, jonka voit
tarkastaa.

**Git ei kuitenkaan ole varmuuskopio kirjanpidostasi.** `init` rajaa gitistä pois
tositetietokannan (`fiscal.sqlite`), arkistoidut liitteet (`artifacts/`) ja
henkilötiedot (`masterdata/pii.sqlite`). Tositetietokanta ja liitteet ovat
kirjanpidon tärkein osa, ja henkilötietoja ei saa tallentaa historiaan, josta niitä
ei voi poistaa.

Varmuuskopioi siksi koko kotihakemisto erikseen, myös gitistä rajatut tiedostot:

- Kopioi koti, kun mikään aggountant-komento ei ole käynnissä. Mukaan tulevat
  myös `fiscal.sqlite`-alkuiset tiedostot ja `artifacts/`-hakemisto.
- Säilytä kopio toisessa paikassa kuin alkuperäinen. Liitteiden pitää säilyä
  kirjanpitolain säilytysajan.
- Kun palautat kopion, aja ensin `aggountant voucher verify`. Se tarkistaa, että
  tositeketju on ehjä.

aggountantissa ei ole omaa varmuuskopio- tai vientikomentoa.

## Käyttöönotto aiemmasta järjestelmästä

Jos yritys siirtyy aggountantiin toisesta kirjanpito-ohjelmasta, käyttöönottoon
kuuluu enemmän kuin `init`. Agentti käy sen läpi kanssasi
`aggountant-init`-skillin avulla:

- **Perustiedot**: virallinen nimi, Y-tunnus, tilikausi, mistä päivästä
  aggountant-kirjanpito alkaa ja jatketaanko vanhaa tilikarttaa. Ne tallennetaan
  lähteineen yrityksen tietovarastoon. Paras lähde on kaupparekisteriote tai
  vanhan järjestelmän raportti, ei muistinvarainen vastaus.
- **Avaavat saldot** kirjataan yhtenä tavallisena tositteena, ja vanhan
  järjestelmän saldoluettelo liitetään siihen. Tositteen päivä on vanhan
  järjestelmän viimeinen päivä. Jos se päivätään uuden kirjanpidon ensimmäiselle
  päivälle, koko aiempi tulos näkyy ensimmäisen kuukauden tuloksena.
- **Avoimet asiat**, jotka eivät näy saldoluettelosta: onko yrityksen laskuja
  maksettu omalta tililtäsi, onko verotilillä velkaa tai maksujärjestely ja onko
  avoimia myynti- tai ostolaskuja.

Avauksen jälkeen agentti tekee ensimmäisestä kuukaudesta raportin (luku 7). Siinä
kuukauden tuloksen pitää olla nolla ennen uusia tapahtumia, taseen pitää vastata
vanhan järjestelmän raporttia ja pankkitilin saldon tiliotetta.

Monivuotisen historian tuonti vanhasta järjestelmästä ei ole vielä valmis.
Käyttöönotto alkaa avaavista saldoista.

## Tilikausi ja kaudet

aggountant käsittelee tilikauden kalenterivuotena. Kirjanpito suljetaan
kuukausittain (`2026-03`) tai neljännesvuosittain (`2026-Q1`), mutta samana
vuonna ei voi käyttää molempia. Kuukausi on suositeltu ja koeteltu vaihtoehto.
Kalenterivuodesta poikkeavaa tilikautta ei vielä tueta.
