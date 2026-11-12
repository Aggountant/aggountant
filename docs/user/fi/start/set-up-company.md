# Ota yrityksesi kirjanpito käyttöön

Tällä sivulla perustat yrityksesi oikean kirjanpidon: yrityskodin, yrityksen
perustiedot, avaavat saldot ja kuvauksen siitä, mistä kuitit ja tiliotteet
tulevat. Agentti käy käyttöönoton läpi kanssasi `aggountant-init`-skillin
avulla. Käyttöönotto tehdään kerran, ja keskeneräisen käyttöönoton voi jatkaa
myöhemmin.

## Ennen kuin aloitat

- aggountant ja skillit on asennettu ([Asenna aggountant ja skillit](install.md)).
- Olet tehnyt harjoituskuukauden tai tiedät muuten, miten agentin kanssa
  toimitaan ([Harjoittele ensimmäinen kuukausi](practice-month.md)).
- Sinulla on käsillä yrityksen virallinen nimi, Y-tunnus ja tilikausi,
  mieluiten kaupparekisteriotteesta.
- Jos yritys siirtyy toisesta kirjanpito-ohjelmasta: vanhan järjestelmän
  saldoluettelo (tase) viimeiseltä päivältä, jonka vanha järjestelmä kattaa.

## Näin teet sen

1. **Pyydä agenttia aloittamaan.**

   > Otetaan Esimerkki Oy:n kirjanpito käyttöön aggountantilla. Koti tulee
   > hakemistoon ~/kirjanpito/esimerkki-oy.

2. **Agentti perustaa kodin.** Se luo hakemiston, tositevaraston,
   oletustilikartan (`fi-general-2026`, 98 tiliä) ja `.gitignore`-tiedoston.
   Jos kodissa on jo kirjanpito, ohjelma kieltäytyy (`ALREADY_INITIALIZED`)
   eikä koske siihen. Perustaminen ei kysy yrityksen nimeä, Y-tunnusta eikä
   tilikautta, vaan ne kerätään seuraavassa vaiheessa. Agentti suosittelee tekemään kodista git-repositorion ja
   asettamaan sen oletuskodiksi, jotta komennot löytävät sen ilman polkua.
   Mitä kotiin tallentuu ja miksi git ei ole varmuuskopio, kerrotaan sivulla
   [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md).

3. **Kerro perustiedot.** Agentti kysyy yrityksen virallisen nimen, Y-tunnuksen,
   tilikauden, päivän, josta aggountant-kirjanpito alkaa, ja jatketaanko vanhan
   järjestelmän tilikarttaa. Se tallentaa vastaukset lähteineen yrityksen
   tietovarastoon. Anna lähteeksi kaupparekisteriote tai vanhan järjestelmän
   raportti, ei muistinvaraista vastausta: tieto ohjaa myöhemmin agentin työtä.
   Jos tilikausi ei ole kalenterivuosi, agentti kirjaa sen myös kirjanpitoon
   (`fiscal-year define`) ennen ensimmäistä tositetta. Se määrittelee
   tilikauden, johon ensimmäinen tosite osuu; seuraavat tilikaudet jatkuvat
   itsestään.

4. **Tarkista ja hyväksy avaavat saldot.** Jos yritys tulee toisesta
   järjestelmästä, agentti kirjaa vanhan järjestelmän saldot yhtenä
   tavallisena tositteena ja liittää saldoluettelon siihen. Tarkista ennen
   hyväksyntää:
   - Tositteen päivä on **vanhan järjestelmän viimeinen päivä**. Jos se
     päivätään uuden kirjanpidon ensimmäiselle päivälle, koko aiempi tulos
     näkyy ensimmäisen kuukauden tuloksena.

     Jos yritys siirtyy aggountantiin sellaisen tilikauden alussa, joka ei
     ala 1.1., avaavat saldot päivätään tilikauden ensimmäiselle päivälle,
     koska kirjanpidossa ei ole sitä edeltävää tilikautta. Tilikauden alussa
     saldoissa on vain tasetilejä, joten tämä ei siirrä tulosta.
   - Jokainen saldo vastaa saldoluetteloa, ja tosite täsmää.

   Uudella yrityksellä ei ole avaavia saldoja. Ensimmäinen tapahtuma on
   yleensä osakepääoman maksu, kuten harjoituskuukaudessa.

5. **Kerro avoimet asiat**, jotka eivät näy saldoluettelosta. Agentti kysyy,
   onko yrityksen laskuja maksettu omalta tililtäsi, onko verotilillä velkaa
   tai maksujärjestely ja onko avoimia myynti- tai ostolaskuja.

6. **Kuvaa, mistä aineisto tulee.** Agentti kirjoittaa kanssasi kodin juureen
   tiedoston `AGENTS.md`, jossa kerrot omin sanoin:
   - minne kuitit ja laskut saapuvat (sähköposti, pilvikansio, paikallinen
     kansio, vastuuhenkilö)
   - missä kulu- ja matkalaskujen luonnokset ovat
   - missä ajopäiväkirja pidetään
   - minne käsitelty alkuperäisaineisto arkistoidaan ja milloin sen saa siirtää
     pois lähteestä
   - mitkä kodin kansiot kuuluvat gitiin ja mitkä eivät.

   Ilman tätä tiedostoa agentti joutuu kysymään joka kerta tai arvaamaan.
   **Älä kirjoita tiedostoon salasanoja, tunnuksia tai avaimia.**

7. **Agentti tekee ensimmäisestä kuukaudesta raportin.** Tarkista se ennen kuin
   kirjaat kuukauden tapahtumia (alla, "Valmis, kun").

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant --home ~/kirjanpito/esimerkki-oy init
aggountant config path
aggountant config show
aggountant chart list
aggountant knowledge company add < tieto.json
aggountant fiscal-year define --start 2026-07-01 --end 2027-06-30
aggountant voucher draft < avaavat-saldot.json
aggountant voucher validate < avaavat-saldot.json
aggountant voucher seal < avaavat-saldot-sinetointi.json
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html \
  --history-start 2026-03-01 --limitation "Avaavat saldot vanhasta järjestelmästä"
```

Oletuskoti asetetaan asetustiedostoon, jonka paikan `config path` kertoo.
Tiedosto on JSON-muotoinen: `{"home": "/polku/kotiin"}`.
</details>

## Valmis, kun

- `aggountant config show` näyttää yrityksesi kodin.
- Ensimmäisen kuukauden raportissa, ennen uusia tapahtumia, kuukauden tulos on
  nolla, tase vastaa vanhan järjestelmän raporttia ja pankkitilin saldo
  tiliotetta.
- Kodin juuressa on `AGENTS.md`, joka kertoo aineiston lähteet.

## Rajoitukset

- Monivuotisen historian tuonti vanhasta järjestelmästä ei ole valmis.
  Käyttöönotto alkaa avaavista saldoista.
- `init` ei korjaa eikä päivitä vanhaa kotia, eikä erillistä päivityskomentoa
  ole.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md).

## Jos jokin menee pieleen

- **`ALREADY_INITIALIZED`.** Koti on jo olemassa. Mitään ei muutettu.
- **Tositteet näyttävät kadonneen tai `MASTERDATA_NOT_INITIALIZED`.** Komento
  osui väärään kotiin: [Kun jokin menee vikaan](../troubleshooting.md#tositteet-näyttävät-kadonneen).

## Katso myös

- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
- [Kuukauden kirjanpito alusta loppuun](../tasks/monthly-routine.md)
- [Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md)
