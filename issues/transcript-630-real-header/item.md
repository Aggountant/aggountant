---
created: 2026-11-19
updated: 2026-11-19
type: bug
status: fixed
priority: high
lane: wage-reporting
lane_seq: 0
collision: [crates/process/src/transcript_630.rs]
preparation: ready
closed: 2026-11-19
closed_by: claude-opus-5-5
---

# Raportti 630: oikea asiointipalvelun CSV hylätään (TRANSCRIPT_UNKNOWN_HEADER)

## Description

`file-wage employer-report import-transcript` hylkää jokaisen oikean raportin 630
virheellä `TRANSCRIPT_UNKNOWN_HEADER`. Raportit tilattiin Tulorekisterin
asiointipalvelusta 19.11.2026 (oikea yritys, suomeksi, ilmoittajan tyyppi Maksaja,
vastaanottohetken aikaväli tyhjä, neljä 24 kuukauden jaksoa 2/2020–10/2026, bundle
0.1.0-dev.g7d922dab70da). `crates/process/src/transcript_630.rs` muodostaa otsikon
sanatarkasti dokumentista *Raportit ja aineistot tulorekisteristä 2026* §4.6.2.
Oikea tiedosto poikkeaa siitä kahdella tavalla.

1. Sarakkeet 26–29 (1-pohjaisesti) ovat oikeassa tiedostossa muotoa
   `… perusteena olevat perusteettomaksi eduksi ilmoitetut tulot`, esimerkiksi
   `Työeläkevakuutusmaksun perusteena olevat perusteettomaksi eduksi ilmoitetut tulot`.
   Koodi odottaa muotoa `… perusteena olevat perusteettomat edut`. Muut 29 saraketta ja
   niiden järjestys täsmäävät (33 saraketta).
2. Palkkatietorivien tyyppi on monikossa: `Palkkatietoilmoitukset: Maksajan omat suoritukset`.
   Koodi (`TYPES_EARNINGS`) odottaa muotoa `Palkkatietoilmoitus: …`. Kun otsikko korjataan,
   tämä kaataisi jäsennyksen seuraavaksi, koska jokaisella rivillä pitää olla tunnettu tyyppi.
   `Työnantajan erillisilmoitus` ja ajanjaksot `Päivä yhteensä`, `Kuukausi yhteensä` ja
   `Aikaväli yhteensä` täsmäävät.

Muita havaintoja oikeasta tiedostosta:

- Rajapinta (`/api/queryResultFile`) palauttaa ISO-8859-1-tekstiä. Selaimen *Lataa
  raportti (csv)* tallentaa saman sisällön UTF-8-muodossa BOMin kanssa, CRLF-rivinvaihdoin.
  Jäsennin hyväksyy kummankin.
- Erillisilmoitusrivillä tyhjä sairausvakuutusmaksu (sarake 32) on tyhjä merkkijono, ei
  `0,00`. Tämä toistuu kuukausilla, joilla on maksettu vain verovapaita korvauksia tai ei
  mitään. Lukumäärä (sarake 31) on aina täytetty.
- Kohdekauden alkupäivä on muodossa `D.M.YYYY`, esimerkiksi `1.10.2026`.

Oikeat tiedostot ovat yrityksen yksityisessä aggountant-kodissa. Niissä ei ole
henkilötunnuksia, mutta ne ovat yrityksen palkkasummia. Tee testifixture keksityillä
summilla ja käytä oikeaa otsikkoa ja rivityyppejä.

## Acceptance Criteria

- [x] Asiointipalvelun oikealla otsikolla tehty raportti 630 jäsentyy. Testifixture käyttää
      oikeaa 33 sarakkeen otsikkoa ja monikkomuotoista palkkatietotyyppiä.
- [x] Ratkaise, hyväksytäänkö myös dokumentin mukainen otsikkomuoto. Kirjaa perustelu.
- [x] Tyhjä sairausvakuutusmaksu- tai vähennyssolu tulkitaan nollaksi, kun lukumäärä on täytetty.
- [ ] `import-transcript --dry-run` toimii yrityksen neljälle oikealle tiedostolle.
      Tulos: jokainen kuukausi 2/2020–10/2026 on `recorded`.

## Resolution

### 2026-11-19T10:01:55Z · @claude-opus-5-5

Parser accepts the real e-service header and plural earnings type; documented wording refused (see result.md). The dry run on the real files is left to the conductor.
