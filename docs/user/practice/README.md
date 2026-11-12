# Harjoitusaineisto

Tämä hakemisto on oppaan harjoituskuukauden aineisto:
[Harjoittele ensimmäinen kuukausi](../fi/start/practice-month.md), englanniksi
[Practise your first month](../en/start/practice-month.md). Yritys,
tilinumero, vastapuolet ja summat on keksitty. Esimerkki Oy, Pilvipalvelu Oy ja
Toimistotarvike Oy eivät ole oikeita yrityksiä.

| Tiedosto | Mitä se on |
|---|---|
| `tiliote-2026-03.xml` | Esimerkki Oy:n maaliskuun 2026 tiliote camt.053-muodossa: alkusaldo 0,00 €, Pilvipalvelu Oy:n laskun maksu 124,00 €, osakepääoman maksu 2 500,00 € ja pankissa vielä vahvistamaton 50,00 euron varaus. Loppusaldo 2 376,00 €. |
| `lasku-pilvipalvelu-2026-03.txt` | Pilvipalvelu Oy:n lasku palvelinvuokrasta, laskun päivä 11.3.2026, 124,00 € sis. ALV 25,5 % |
| `kuitti-toimistotarvike-2026-03.txt` | Toimistotarvike Oy:n kuitti 22.3.2026, 31,00 € sis. ALV 25,5 %, maksettu omalla kortilla |
| `run-practice-month.sh` | ajaa harjoituskuukauden komennot kertakäyttöisessä kodissa ja tarkistaa oppaan luvut |

Lasku ja kuitti ovat tekstitiedostoja, jotta ne voi lukea suoraan ja
tarkistaa, ettei niissä ole oikeita tietoja.

## Aineiston tarkistus

`run-practice-month.sh` tekee harjoituksen niin kuin agentti sen tekisi:
perustaa kertakäyttöisen kodin, tuo tiliotteen, kirjaa kuitin, tekee raportin,
sulkee kuukauden ja esikatselee ALV-ilmoituksen. Se tarkistaa samalla luvut,
jotka opas mainitsee (kuukauden tulos −123,50 €, pankkitili 2 376,00 €, taseen
loppusummat 2 407,50 €, kentät 307 ja 308). Skripti tarvitsee `jq`:n.

```sh
docs/user/practice/run-practice-month.sh path/to/aggountant
```

Koti luodaan `$TMPDIR`-hakemistoon ja poistetaan lopuksi. Kun ympäristömuuttuja
`KEEP_PRACTICE_HOME` on asetettu, koti jätetään tarkasteltavaksi. Kun komento,
syötteen muoto tai tuloste muuttuu, päivitä skripti ja oppaan sivu samassa
muutoksessa.
