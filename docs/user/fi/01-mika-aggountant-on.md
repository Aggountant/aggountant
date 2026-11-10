# 1. Mikä aggountant on ja kenelle

aggountant on suomalaisen kirjanpidon työkalu, jota käytetään yhdessä
tekoälyagentin kanssa. Agentti (esimerkiksi Claude Code) lukee kuitit,
tiliotteet ja muun aineiston, ehdottaa kirjaukset ja ajaa aggountantin komentoja.
aggountant laskee, tarkistaa ja tallentaa: se huolehtii siitä, että kirjaukset
täsmäävät, ALV lasketaan säännöistä eikä arvaamalla, ja tallennettua tositetta ei
voi jälkikäteen huomaamatta muuttaa.

Työnjako on tämä:

- **Sinä** tuot aineiston tai kerrot, mistä se löytyy, vastaat kysymyksiin, joihin
  vain yrityksen omistaja tietää vastauksen, ja hyväksyt jokaisen pysyvän
  muutoksen ennen kuin se tehdään.
- **Agentti** tekee rutiinityön: lukee lähteet, ehdottaa tilit ja
  ALV-käsittelyn, kokoaa kysymykset yhteen ja näyttää sinulle hyväksyttävän
  kokonaisuuden.
- **aggountant** tekee sen, mikä pitää tehdä täsmälleen oikein: numeroi tositteet
  aukottomasti, tarkistaa debetin ja kreditin tasapainon, laskee ALV:n
  verokantataulukosta, sulkee kaudet ja lähettää ilmoitukset vasta hyväksynnän
  jälkeen.

## Kenelle

aggountant on tehty pienelle yritykselle, jonka kirjanpitoa hoitaa yksi
luotettu ihminen, tavallisesti omistaja itse, agentin avustamana. Ohjelma olettaa,
että:

- **käyttäjä on yksi.** Samaa kirjanpitoa ei ole tarkoitettu käytettäväksi
  usealta ihmiseltä tai koneelta yhtä aikaa.
- **jokaisella yrityksellä on oma yrityskotinsa.** Yrityskoti on hakemisto, jossa
  ovat yrityksen tilikartta, tositteet, liitteet ja kausien tila. Jos hoidat
  kahden yrityksen kirjanpitoa, niillä on kaksi erillistä kotia.
- **yritys ilmoittaa omat asiansa.** Vero.fi- ja Tulorekisteri-yhteydet ovat
  yrityksen omia. Tilitoimistokäyttöä, jossa ilmoitetaan toisen yrityksen
  puolesta, ei tueta.

## Mitä aggountant ei tee

- **Se ei maksa mitään.** ALV:n, palkkojen ja laskujen maksut teet itse pankissa
  tai OmaVerossa. aggountant kertoo maksettavan summan.
- **Se ei päätä puolestasi.** Ohjelma ei sinetöi tositetta eikä lähetä ilmoitusta
  ilman nimenomaista hyväksyntääsi.
- **Se ei ole selainsovellus.** Kaikki tapahtuu komentorivillä agentin kautta.
  Raportit saat HTML-tiedostoina, jotka avaat selaimessa.

## Miksi tositetta ei voi muokata

Kun tosite tallennetaan, se sinetöidään: se saa pysyvän numeron, ja siitä
lasketaan tunniste, joka ketjutetaan edellisiin tositteisiin. Jos joku muuttaa
vanhaa tositetta jälkikäteen, ketjun tarkistus (`aggountant voucher verify`)
paljastaa sen. Virhe korjataan uudella korjaustositteella, aivan kuten
kirjanpitolaki edellyttää, eikä alkuperäistä koskaan poisteta.

Tästä syystä agentti pyytää hyväksyntäsi aina ennen sinetöintiä. Hyväksyntä on
se hetki, jolloin päätät, mitä kirjanpitoon jää pysyvästi.
