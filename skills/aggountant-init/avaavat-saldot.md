# Avaava tosite toisesta järjestelmästä

Koskee vain yritystä, joka siirtyy aggountantiin toisesta kirjanpitojärjestelmästä.
Uudella yrityksellä ei ole avaavia saldoja. Lue ensin [SKILL.md](SKILL.md):
saldot kirjataan tavallisena sinetöitynä tositteena, ja ennen sitä kysytään, mitä
saldoluettelo ei kerro.

## Päivä ja sisältö

Päivä on päättyvän järjestelmän viimeinen päivä, ei uuden kirjanpidon ensimmäinen.
Ensimmäiselle uudelle päivälle päivätty avaus toisi mukanaan tuodut tulostilit
ensimmäisen kuukauden tulokseen. Kesken kuukauden siirryttäessä niin käy joka
tapauksessa, joten ehdota siirtymää kuun vaihteeseen. Se, mitä tositteeseen kuuluu,
riippuu siirtymän hetkestä:

- **Kesken tilikauden.** Tosite osuu samalle tilikaudelle kuin uudet tapahtumat, ja
  siinä on tasetilien lisäksi tilikauden alusta kertyneet tulostilit.
- **Vuodenvaihteessa**, kun tilikausi on kalenterivuosi ja aiempi järjestelmä kattaa
  edellisen vuoden loppuun. Tosite päivätään edellisen tilikauden viimeiselle päivälle,
  ja siinä on vain tasetilejä: edellisen tilikauden tulos on omassa pääomassa. Ohjelma avaa uuden
  tilikauden näistä saldoista vaatimatta vanhaa tilikautta suljetuksi, mutta vain jos
  edellisellä tilikaudella ei ole mitään muuta: ei tulostilejä, ei muita päiviä, ei
  aggountantissa suljettua kautta eikä vanhempia tilikausia. Muuten uuden tilikauden
  raportit kieltäytyvät koodilla `PERIOD_NOT_SEALED`.
- **Tilikauden vaihteessa, kun tilikausi ei ala 1.1.** Määrittele ensimmäiseksi uusi
  tilikausi. Edellistä päivää ei silloin ole ohjelmalle olemassa, ja tosite päivätään
  uuden tilikauden ensimmäiselle päivälle. Siinä on vain tasetilejä, joten kuukauden
  tulos ei vääristy, eikä edellisen kohdan ehtoa tarvita.

## Tilimuunnos

Kirjanpito käyttää aina oletustilikarttaa, ja aiemman järjestelmän tilit muunnetaan sille
avaavassa tositteessa. Kirjaa muunnos omaksi tiedostokseen kotiin niin, että jokainen
lähdetili on siinä kerran summineen ja summat täsmäävät sekä lähderaporttiin että
avaavaan tositteeseen; muuten erittely katoaa. Kerro käyttäjälle, mitkä erottelut
yhdistyvät. Jos verotili ja osakasvelat osuisivat samalle tilille, sovi niiden erottelu:
yhdistettyinä niitä ei saa myöhemmin täsmäytettyä.
ALV-tilien avaava saldo on aiempien kausien veroa ja näkyy ensimmäisen ALV-ilmoituksen täsmäytyksessä
merkitsemättömänä vientinä; `vero-fi`-skilli kertoo, miten se kuitataan.

## Onnistuiko avaus

Avauksen jälkeen `aggountant report statements` ensimmäiselle uudelle kuukaudelle
näyttää, onnistuiko se: kuukauden tulos on nolla ennen uusia tapahtumia, tilikauden
alusta kertynyt tulos on lähderaportin mukainen (tilikauden vaihteessa nolla), tase
vastaa lähderaporttia ja pankkitilin saldo tiliotetta.
