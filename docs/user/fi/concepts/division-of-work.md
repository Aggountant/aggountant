# Sinä, agentti ja aggountant

aggountantia käyttäessäsi kirjanpitoa tekee kolme osapuolta: sinä, agentti ja
aggountant-ohjelma. Tämä sivu kertoo, mikä kuuluu kenellekin ja miksi
työnjako on tällainen.

## Kolme osapuolta

- **Sinä** tuot aineiston tai kerrot, mistä se löytyy, vastaat kysymyksiin,
  joihin vain yrityksen omistaja tietää vastauksen, ja hyväksyt jokaisen
  pysyvän muutoksen ennen kuin se tehdään.
- **Agentti** (esimerkiksi Claude Code) tekee rutiinityön: lukee kuitit,
  tiliotteet ja muun aineiston, ehdottaa tilit ja ALV-käsittelyn, kokoaa
  kysymykset yhteen ja näyttää sinulle hyväksyttävän kokonaisuuden. Se ajaa
  aggountantin komentoja puolestasi.
- **aggountant** tekee sen, minkä pitää mennä täsmälleen oikein: numeroi
  tositteet aukottomasti, tarkistaa debetin ja kreditin tasapainon, laskee
  ALV:n verokantataulukosta, sulkee kaudet ja lähettää ilmoitukset vasta
  hyväksynnän jälkeen.

aggountant ei sisällä tekoälyä. Tekoäly on agentissa, jota käytät. Agentti
puolestaan ei kirjoita kirjanpitoa itse vaan pyytää aggountantia
tallentamaan, ja aggountant kieltäytyy kaikesta, mikä ei täsmää.

## Miksi työ on jaettu näin

Agentti on hyvä lukemaan sekalaista aineistoa ja päättelemään, mitä tapahtuma
on, mutta se voi erehtyä. aggountant ei päättele mitään, mutta se laskee ja
tarkistaa aina samalla tavalla. Kun agentti ehdottaa ja aggountant tarkistaa,
kumpikin tekee sitä, missä se on luotettava. Sinä teet sen, mitä kumpikaan ei
voi: tiedät, kuuluuko kulu yritykselle, ja päätät, mitä kirjanpitoon jää.

## Agentti toimii kirjanpitoa tuntevan työtoverin tavoin

aggountantin mukana tulevat skillit kertovat agentille, miten se toimii.
Kaikilla skilleillä on yhteinen roolikuvaus (`SOUL.md`): agentti toimii kuin
kirjanpitoa tunteva työtoveri. Se ratkaisee itse sen, mihin lähteet ja
yrityksen aiempi kirjanpito riittävät, ja kysyy sinulta vain sen, mitä se ei
voi päätellä.

Agentti ajaa ilman lupaa kaiken, mikä vain lukee tai esikatselee eikä jätä
pysyviä jälkiä, esimerkiksi:

- tilikartan, aiempien tositteiden ja yrityksen tallennettujen tietojen
  lukeminen
- tositteen esikatselu ja tarkistus
- ALV-ilmoituksen lukujen esikatselu suljetusta kuukaudesta
- suoran ilmoittamisen valmiuden tarkistus ilman verkkoyhteyttä
- raporttien tekeminen.

Pysyvät teot, kuten tositteen sinetöinti, kuukauden sulku ja ilmoituksen
lähetys, odottavat hyväksyntääsi:
[Sinetöinti ja hyväksyntä](sealing-and-approval.md).

## Agentti kokoaa kysymykset

Kuukauden aineistossa voi olla kymmeniä tapahtumia, eikä agentin kuulu
keskeyttää sinua jokaisen kuitin kohdalla. Se kysyy, kun vaihtoehdot eroavat
tavalla, jolla on sinulle merkitystä (tili, ALV-käsittely, kuuluuko kulu
yritykselle), eikä se voi päätellä valintaasi. Kysymykset tulevat koottuina,
ja vastauksesi agentti voi tallentaa yrityksen tietovarastoon, jotta samaa ei
kysytä uudelleen ([Yrityskoti](company-home.md#yrityksen-tietovarasto)).

## Mitä tämä tarkoittaa sinulle

- Kerro agentille tavoite, älä komentoja: "kirjataan maaliskuu" riittää.
- Vastaa kysymyksiin tarkasti. Vastauksesi ohjaa myös tulevia kirjauksia.
- Katso hyväksyntäpyynnöt aina. Agentti voi erehtyä, ja hyväksyntä on se
  kohta, jossa virhe pysähtyy.

## Katso myös

- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
- [Asenna aggountant ja skillit](../start/install.md)
- [Yrityskoti: mitä tallentuu minne](company-home.md)
