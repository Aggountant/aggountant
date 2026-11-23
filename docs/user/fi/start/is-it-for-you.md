# Onko aggountant sinulle?

aggountant on suomalaisen osakeyhtiön kirjanpidon työkalu, jota käytetään
tekoälyagentin kanssa.

## Mitä aggountant tekee

Kuukausi kulkee näin:

1. Agentti lukee kuukauden tiliotteen ja kuitit ja ehdottaa jokaiselle
   tapahtumalle tilin ja ALV-käsittelyn.
2. Se kysyy sinulta vain sen, mitä se ei voi päätellä, esimerkiksi oliko
   ravintolalasku asiakastapaaminen.
3. Sinä katsot ehdotukset ja hyväksyt. aggountant tallentaa tositteet
   pysyvästi, sulkee kuukauden hyväksynnälläsi ja laskee ALV-ilmoituksen.
4. ALV-ilmoitus lähtee suoraan Verohallinnolle tai syötät luvut itse
   OmaVeroon.

Työnjaosta kerrotaan tarkemmin sivulla
[Sinä, agentti ja aggountant](../concepts/division-of-work.md).

aggountant on varhainen 0.x-versio, ja sillä pidetään jo muutaman sellaisen
yrityksen kirjanpitoa, jolla ei ole tilitoimistoa. Ominaisuuksia lisätään
sitä mukaa kuin nämä yritykset kohtaavat uusia tilanteita, joten tavallinen
kuukausityö on hyvässä kunnossa ja harvinaisemmat tapaukset täydentyvät
vähitellen. Komennot ja tiedostomuodot voivat vielä muuttua versioiden
välillä. Jos haluat mukaan ensimmäisten joukossa, lue
[Lähde mukaan ensimmäisten joukossa](early-users.md).

## Kenelle se sopii

aggountant on tehty pienelle yritykselle, jonka kirjanpitoa hoitaa yksi
ihminen, tavallisesti omistaja itse, agentin avustamana. Se sopii sinulle,
jos:

- yrityksesi on pieni suomalainen osakeyhtiö
- saat pankistasi tiliotteen camt.053- tai camt.052-tiedostona (Holvin
  tiliote on koeteltu, muiden pankkien tiedostoja ei vielä)
- osaat käynnistää päätteessä toimivan agentin, kuten Claude Coden, mutta et
  halua opetella kirjanpito-ohjelman komentoja
- tiedät suunnilleen, mitä tosite, ALV ja tase ovat.

Samaa kirjanpitoa ei käytetä usealta ihmiseltä tai koneelta yhtä aikaa.
Jokaisella yrityksellä on oma yrityskotinsa eli hakemisto, jossa on sen koko
kirjanpito, joten kahdella yrityksellä on kaksi kotia. Vero.fi- ja
Tulorekisteri-yhteydet ovat yrityksen omia. Tilitoimistokäyttöä, jossa
ilmoitetaan toisen yrityksen puolesta, ei tueta.

## Mitä se vaatii sinulta

- Hankit ja käynnistät agentin itse. aggountant on komentoriviohjelma, jossa
  ei ole omaa tekoälyä.
- Asennat ohjelman ja sen skillit koneellesi:
  [Asenna aggountant ja skillit](install.md).
- Katsot ja hyväksyt tositteet, kuukauden sulun ja ilmoitukset. Agentti tekee
  rutiinityön, mutta mitään ei tallenneta pysyvästi eikä lähetetä ilman
  hyväksyntääsi.
- Jos haluat lähettää ALV-ilmoituksen suoraan, tarvitset oman Vero
  API -rekisteröinnin. Rekisteröinti, rajapintasopimus ja varmenne ovat
  yrityksesi omia, ja käyttöönotossa on useita vaiheita:
  [Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md). Ilman
  sitä syötät ALV-luvut itse OmaVeroon.

## Mitä se ei tee

- Se ei maksa mitään. ALV:n, palkat ja laskut maksat itse pankissa tai
  OmaVerossa, ja aggountant kertoo maksettavan summan.
- Se ei ole selainsovellus. Kaikki tapahtuu komentorivillä agentin kautta, ja
  raportit saat HTML-tiedostoina, jotka avaat selaimessa.
- Se ei vielä hoida kaikkea. Rahapalkkoja ei voi vielä laskea, maksaa eikä
  ilmoittaa, kuittien automaattinen haku ei ole käytössä, ja virallinen
  tilinpäätösasiakirja ja veroilmoitus tehdään aggountantin ulkopuolella.
  Kaikki rajoitukset:
  [Mitä on valmista ja mitä ei](../reference/status.md).

## Kuka vastaa kirjanpidosta

Sinä tai yrityksesi hallitus, kuten aina. aggountant laskee, tarkistaa ja
kieltäytyy tallentamasta sellaista, mikä ei täsmää, mutta se ei tiedä, onko
kulu yrityksen vai onko kaikki aineisto kirjattu. Agentti voi erehtyä, ja
siksi jokainen pysyvä teko odottaa hyväksyntääsi. Opas ei ole oikeudellinen
tai verotuksellinen neuvo.

## Kokeile ilman riskiä

[Harjoittele ensimmäinen kuukausi](practice-month.md) kirjaa kuvitteellisen
yrityksen maaliskuun erillisessä harjoituskodissa noin puolessa tunnissa.
Näet, miltä työ agentin kanssa tuntuu, ennen kuin otat aggountantin käyttöön
omassa yrityksessäsi.

## Katso myös

- [Asenna aggountant ja skillit](install.md)
- [Kirjanpitäjälle](../concepts/for-bookkeepers.md)
- [Mitä on valmista ja mitä ei](../reference/status.md)
