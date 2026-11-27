# Ota yrityksesi kirjanpito käyttöön

Kun aloitat yrityksesi oikean kirjanpidon aggountantilla, agentti perustaa
kanssasi yrityskodin, tallentaa yrityksen perustiedot ja kirjaa avaavat
saldot. Lisäksi kirjoitatte muistiin, mistä kuitit ja tiliotteet tulevat ja
mikä jäi odottamaan. Agentti käyttää tähän `aggountant-init`-skilliä.
Käyttöönotto tehdään kerran, ja keskeneräistä käyttöönottoa voi jatkaa
myöhemmin: agentti katsoo kodista, mikä on jo tehty, eikä kysy samaa
uudelleen.

## Ennen kuin aloitat

- Olet [asentanut aggountantin ja skillit](install.md).
- Olet tehnyt [harjoituskuukauden](practice-month.md) tai tiedät muuten,
  miten agentin kanssa toimitaan.
- Sinulla on käsillä yrityksen virallinen nimi, Y-tunnus ja tilikausi,
  mieluiten kaupparekisteriotteesta.
- Jos yritys siirtyy toisesta kirjanpito-ohjelmasta, sinulla on vanhan
  järjestelmän tililuettelo ja saldoluettelo (tase) viimeiseltä päivältä,
  jonka vanha järjestelmä kattaa. Sivu
  [Siirrä kirjanpito toisesta ohjelmasta](move-from-another-program.md)
  kertoo, mitä muuta vanhasta ohjelmasta viedään talteen ja miten. Valitse
  siirtymälle kuun vaihde: kesken kuukauden siirryttäessä vanhan
  järjestelmän osuus kuukaudesta näkyy ensimmäisen kuukauden tuloksessa.

## Näin teet sen

1. **Pyydä agenttia aloittamaan.**

   > Otetaan Esimerkki Oy:n kirjanpito käyttöön aggountantilla. Koti tulee
   > hakemistoon ~/kirjanpito/esimerkki-oy.

2. **Agentti perustaa kodin.** Se luo hakemiston, tositevaraston,
   oletustilikartan (`fi-general-2026`, 98 tiliä) ja `.gitignore`-tiedoston.
   Agentti suosittelee tekemään kodista git-repositorion ja asettamaan sen
   oletuskodiksi, jotta komennot löytävät sen ilman polkua. Sivu
   [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md) kertoo,
   mitä kotiin tallentuu ja miksi git ei ole varmuuskopio.

3. **Agentti näyttää valmiuslistan ja pyytää aineiston kerralla.** Lista
   kertoo seitsemästä kohdasta, mikä on valmis, mikä puuttuu ja mikä ei koske
   yritystäsi: tunnistetiedot, tilikausi, siirtymäpäivä, aineiston
   kattavuus (pankkitilit, kuittien reitti, avoimet erät, omalta tililtä
   maksetut kulut ja yksityisen omaisuuden käyttö yrityksen työhön),
   tilimuunnos ja avaavat saldot (vain toisesta ohjelmasta siirtyvälle),
   täsmäytykset ja ensimmäisen kuukauden sulku. Puuttuvasta agentti kokoaa
   yhden aineistopyynnön, joka on räätälöity yrityksellesi: se nimeää
   pankkitilisi ja kaudet ja pyytää vain sitä, mikä koskee yritystäsi,
   kunkin kohdan perusteluineen. Esimerkiksi verotilin otteita se pyytää
   vain, jos taseessa on verotiliä. Pyyntö kirjataan kodin
   `TODO.md`-tiedostoon, ja kun jatkatte myöhemmin, agentti näyttää listan
   uudelleen ja pyytää vain sen, mikä yhä puuttuu.

4. **Kerro perustiedot.** Agentti kysyy yrityksen virallisen nimen,
   Y-tunnuksen, tilikauden ja päivän, josta aggountant-kirjanpito alkaa. Se
   tallentaa vastaukset lähteineen yrityksen tietovarastoon ja nojaa niihin
   myöhemmässä työssään. Katso vastaukset siksi kaupparekisteriotteesta tai
   vanhan järjestelmän raportista äläkä vastaa muistista. Jos tilikausi ei
   ole kalenterivuosi, agentti kirjaa ennen ensimmäistä tositetta myös
   kirjanpitoon tilikauden, johon ensimmäinen tosite osuu. Seuraavat
   tilikaudet jatkuvat itsestään.

5. **Tarkista ja hyväksy avaavat saldot.** Uudella yrityksellä ei ole avaavia
   saldoja, ja sen ensimmäinen tapahtuma on yleensä osakepääoman maksu, kuten
   harjoituskuukaudessa. Toisesta järjestelmästä tulevan yrityksen saldot
   agentti kirjaa yhtenä tavallisena tositteena ja liittää saldoluettelon
   siihen. Kirjanpito käyttää aggountantin oletustilikarttaa, joten agentti
   muuntaa vanhan järjestelmän tilit sille ja tallentaa muunnoksen kotiin.
   Usea vanha tili voi yhdistyä yhdeksi. Agentti kertoo, mitkä erottelut näin
   katoavat, ja muunnostiedostosta näet, mistä kukin saldo tuli. Tarkista
   ennen hyväksyntää:
   - Jokainen saldo vastaa saldoluetteloa, ja tosite täsmää.
   - Tositteen päivä on vanhan järjestelmän viimeinen päivä. Jos tosite
     päivätään uuden kirjanpidon ensimmäiselle päivälle, koko aiempi tulos
     näkyy ensimmäisen kuukauden tuloksena.

   Tositteen sisältö riippuu siitä, milloin yritys siirtyy:
   - Kesken tilikauden siirtyvän yrityksen tosite osuu samalle tilikaudelle
     kuin uudet tapahtumat. Siinä ovat tasetilien lisäksi tilikauden
     tulostilit, joten tilikauden alusta kertynyt tulos jatkuu oikein.
   - Tilikauden vaihteessa siirtyvän yrityksen tosite päivätään edellisen
     tilikauden viimeiselle päivälle. Jos vanha järjestelmä kattaa vuoden
     2025 ja aggountant alkaa 1.1.2026, päivä on 31.12.2025. Tositteessa on
     vain tasetilejä, koska edellisen tilikauden tulos on siirretty omaan
     pääomaan. Ohjelma käyttää tositetta uuden tilikauden avaavina saldoina
     eikä vaadi vanhan järjestelmän vuotta suljetuksi.
   - Jos uusi tilikausi ei ala 1.1., avaavat saldot päivätään
     poikkeuksellisesti tilikauden ensimmäiselle päivälle, koska
     kirjanpidossa ei ole sitä edeltävää tilikautta. Tilikauden alussa
     saldoissa on vain tasetilejä, joten tämä ei siirrä tulosta.

6. **Kerro avoimet asiat**, jotka eivät näy saldoluettelosta. Agentti kysyy,
   onko yrityksen laskuja maksettu omalta tililtäsi, onko verotilillä velkaa
   tai maksujärjestely ja onko avoimia myynti- tai ostolaskuja. Jos
   verotilillä on saldoa tai maksujärjestely, agentti pyytää siirtovuoden
   kaikki OmaVeron tapahtumaotteet.

7. **Kuvaa, mistä aineisto tulee.** Agentti kirjoittaa kanssasi kodin juureen
   tiedoston `AGENTS.md`, jossa kerrot omin sanoin:
   - minne kuitit ja laskut saapuvat (sähköposti, pilvikansio, paikallinen
     kansio, vastuuhenkilö)
   - missä kulu- ja matkalaskujen luonnokset ovat
   - missä ajopäiväkirja pidetään
   - minne käsitelty alkuperäisaineisto arkistoidaan ja milloin sen saa
     siirtää pois lähteestä
   - mitkä kodin kansiot kuuluvat gitiin ja mitkä eivät.

   Ilman tätä tiedostoa agentti joutuu kysymään joka kerta tai arvaamaan.
   **Älä kirjoita tiedostoon salasanoja, tunnuksia tai avaimia.**

8. **Agentti kirjoittaa kodin juureen `TODO.md`-tiedoston**, johon se kokoaa,
   mikä jäi odottamaan: mitä odotetaan muilta (esimerkiksi perustajan omalta
   tililtään maksaman rekisteröintimaksun kuitti), mitkä kuukaudet ovat auki
   ja mitä niiden sulku vielä vaatii, sekä määräajat (esimerkiksi
   ALV-ilmoitus ja ensimmäisen tilikauden tilinpäätös). Tarkista, että lista
   vastaa omaa käsitystäsi. Agentti lukee tiedoston jokaisen istunnon alussa
   ja päivittää sen, kun kohta ratkeaa, kuten kohta
   [Kodin TODO.md](../concepts/company-home.md#kodin-todomd) kertoo.

9. **Agentti tekee ensimmäisestä kuukaudesta raportin.** Tarkista siitä
   kohdan [Valmis, kun](#valmis-kun) luvut, ennen kuin kirjaat kuukauden
   tapahtumia.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant --home ~/kirjanpito/esimerkki-oy init
aggountant config path
aggountant config show
aggountant knowledge company list
aggountant fiscal-year list
aggountant voucher list --posting-from 2026-02-28 --posting-to 2026-03-31
aggountant period show --period 2026-03
aggountant chart list
aggountant knowledge company add < tieto.json
aggountant fiscal-year define --start 2026-07-01 --end 2027-06-30
aggountant voucher draft < avaavat-saldot.json
aggountant voucher validate < avaavat-saldot-sinetointi.json
aggountant voucher seal < avaavat-saldot-sinetointi.json
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html \
  --history-start 2026-03-01 --limitation "Avaavat saldot vanhasta järjestelmästä"
```

Oletuskoti asetetaan asetustiedostoon, jonka paikan `config path` kertoo.
Tiedosto on JSON-muotoinen: `{"home": "/polku/kotiin"}`.

`fiscal-year define` ajetaan vain, jos tilikausi ei ole kalenterivuosi.

`knowledge company list`, `fiscal-year list`, `voucher list` ja
`period show` vain lukevat. Niistä ja kodin tiedostoista agentti kokoaa
valmiuslistan jokaisen istunnon alussa.
</details>

## Valmis, kun

- `aggountant config show` näyttää yrityksesi kodin.
- Jos yritys siirtyi toisesta ohjelmasta, ensimmäisen kuukauden raportissa,
  ennen uusia tapahtumia, kuukauden tulos on nolla, tase vastaa vanhan
  järjestelmän raporttia ja pankkitilin saldo tiliotetta.
- Kodin juuressa on `AGENTS.md`, joka kertoo aineiston lähteet, ja
  `TODO.md`, joka kertoo, mikä odottaa.
- Agentin valmiuslistan jokainen kohta on valmis tai ei koske yritystäsi.

## Rajoitukset

- Monivuotista historiaa ei voi vielä tuoda vanhasta järjestelmästä, joten
  käyttöönotto alkaa avaavista saldoista.
- Vanhan järjestelmän tilikarttaa ei voi tuoda, joten kirjanpito käyttää
  oletustilikarttaa.
- `init` ei korjaa eikä päivitä vanhaa kotia. Sen tekee
  [aggountantin päivitys](../tasks/upgrade.md).

Käyttöönoton kaikki rajoitukset ovat sivulla
[Mitä on valmista ja mitä ei](../reference/status.md#yrityskoti-ja-käyttöönotto).

## Jos jokin menee pieleen

- `ALREADY_INITIALIZED`: kodissa on jo kirjanpito. Ohjelma kieltäytyi
  perustamasta kotia eikä muuttanut mitään.
- `PERIOD_NOT_SEALED` uuden tilikauden raportissa: edellisen tilikauden
  puolella on muuta kuin tilikauden vaihteen avaavat saldot, eli
  tulostilejä, aiempi päivä tai suljettu kausi.
- Jos tositteet näyttävät kadonneen tai komento pysähtyy koodiin
  `HOME_NOT_INITIALIZED`, komento osui väärään kotiin. Vianetsinnän kohta
  [Tositteet näyttävät kadonneen](../troubleshooting.md#tositteet-näyttävät-kadonneen)
  neuvoo, miten oikea koti löytyy.

## Katso myös

- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
- [Kuukauden kirjanpito alusta loppuun](../tasks/monthly-routine.md)
- [Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md)
