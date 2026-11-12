# Onko aggountant sinulle?

Tämä sivu auttaa päättämään, kannattaako aggountantia kokeilla: mitä se tekee,
mitä se vaatii sinulta, mitä se ei tee ja kuka vastaa kirjanpidosta. Lukeminen
vie viisi minuuttia.

## Mitä aggountant tekee

aggountant on suomalaisen osakeyhtiön kirjanpidon työkalu, jota käytetään
tekoälyagentin kanssa. Kuukausi kulkee näin:

1. Agentti lukee kuukauden tiliotteen ja kuitit ja ehdottaa jokaiselle
   tapahtumalle tilin ja ALV-käsittelyn.
2. Se kysyy sinulta vain sen, mitä se ei voi päätellä, esimerkiksi oliko
   ravintolalasku asiakastapaaminen.
3. Sinä katsot ehdotukset ja hyväksyt. aggountant tallentaa tositteet
   pysyvästi, sulkee kuukauden hyväksynnälläsi ja laskee ALV-ilmoituksen.
4. ALV-ilmoitus lähtee suoraan Verohallinnolle tai syötät luvut itse
   OmaVeroon.

Työnjako on kerrottu tarkemmin sivulla
[Sinä, agentti ja aggountant](../concepts/division-of-work.md).

aggountant on versiossa 0.1, ja sillä pidetään jo muutaman sellaisen
yrityksen kirjanpitoa, jolla ei ole tilitoimistoa. Ominaisuuksia lisätään
sitä mukaa kuin nämä yritykset kohtaavat uusia tilanteita. Siksi tavallinen
kuukausityö on hyvässä kunnossa, ja harvinaisemmat tapaukset täydentyvät
vähitellen. Komennot ja tiedostomuodot voivat vielä muuttua versioiden
välillä. Jos haluat mukaan ensimmäisten joukossa, voimme myös tavata:
[Lähde mukaan ensimmäisten joukossa](early-users.md).

## Kenelle se sopii

aggountant on tehty pienelle yritykselle, jonka kirjanpitoa hoitaa yksi
luotettu ihminen, tavallisesti omistaja itse, agentin avustamana. Se sopii
sinulle, jos:

- yrityksesi on pieni suomalainen osakeyhtiö, jonka tilikausi on kalenterivuosi
- pankkisi antaa tiliotteen sähköisenä camt.053- tai camt.052-tiedostona.
  Holvin tiliote on koeteltu, muiden pankkien tiedostoja ei vielä ole
- osaat käynnistää päätteessä toimivan agentin, kuten Claude Coden, mutta et
  halua opetella kirjanpito-ohjelman komentoja
- tiedät suunnilleen, mitä tosite, ALV ja tase ovat.

Ohjelma olettaa kolme asiaa:

- **Käyttäjä on yksi.** Samaa kirjanpitoa ei käytetä usealta ihmiseltä tai
  koneelta yhtä aikaa.
- **Jokaisella yrityksellä on oma yrityskotinsa**, hakemisto, jossa on sen
  koko kirjanpito. Kahdella yrityksellä on kaksi kotia.
- **Yritys ilmoittaa omat asiansa.** Vero.fi- ja Tulorekisteri-yhteydet ovat
  yrityksen omia. Tilitoimistokäyttöä, jossa ilmoitetaan toisen yrityksen
  puolesta, ei tueta.

## Mitä se vaatii sinulta

- **Agentin ja tietokoneen.** aggountant on komentoriviohjelma ilman omaa
  tekoälyä. Tekoäly on agentissa, jonka sinä hankit ja käynnistät.
- **Asennuksen lähdekoodista.** Valmiita asennuspaketteja ei vielä ole:
  [Asenna aggountant ja skillit](install.md).
- **Hyväksynnän jokaiselle pysyvälle teolle.** Agentti tekee rutiinityön,
  mutta sinä katsot ja hyväksyt tositteet, kuukauden sulun ja ilmoitukset.
- **Oman Vero API -rekisteröinnin**, jos haluat lähettää ALV-ilmoituksen
  suoraan. Rekisteröinti, rajapintasopimus ja varmenne ovat yrityksesi omia,
  ja käyttöönotossa on useita vaiheita:
  [Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md). Ilman
  sitä syötät ALV-luvut itse OmaVeroon.

## Mitä se ei tee

- **Se ei maksa mitään.** ALV:n, palkkojen ja laskujen maksut teet itse
  pankissa tai OmaVerossa. aggountant kertoo maksettavan summan.
- **Se ei päätä puolestasi.** Mitään ei tallenneta eikä lähetetä ilman
  nimenomaista hyväksyntääsi.
- **Se ei ole selainsovellus.** Kaikki tapahtuu komentorivillä agentin kautta.
  Raportit saat HTML-tiedostoina, jotka avaat selaimessa.
- **Se ei vielä hoida kaikkea.** Rahapalkkoja ei voi vielä maksaa eikä
  ilmoittaa, kuittien automaattinen haku ei ole käytössä, ja tilinpäätös
  tehdään toistaiseksi aggountantin ulkopuolella. Tarkemmin:
  [Mitä on valmista ja mitä ei](../reference/status.md).

## Kuka vastaa kirjanpidosta

Sinä tai yrityksesi hallitus, kuten aina. aggountant on työkalu: se laskee,
tarkistaa ja kieltäytyy tallentamasta sellaista, mikä ei täsmää, mutta se ei
tiedä, onko kulu yrityksen vai onko kaikki aineisto kirjattu. Agentti voi
erehtyä, ja siksi jokainen pysyvä teko odottaa hyväksyntääsi. Opas ei ole
oikeudellinen tai verotuksellinen neuvo.

## Kokeile ilman riskiä

[Harjoittele ensimmäinen kuukausi](practice-month.md) kirjaa kuvitteellisen
yrityksen maaliskuun erillisessä harjoituskodissa noin puolessa tunnissa.
Näet siinä, miltä työ agentin kanssa tuntuu, ennen kuin otat aggountantin
käyttöön omassa yrityksessäsi.

## Katso myös

- [Asenna aggountant ja skillit](install.md)
- [Lähde mukaan ensimmäisten joukossa](early-users.md)
- [Kirjanpitäjälle](../concepts/for-bookkeepers.md)
- [Mitä on valmista ja mitä ei](../reference/status.md)
