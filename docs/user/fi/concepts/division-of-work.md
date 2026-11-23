# Sinä, agentti ja aggountant

Kirjanpitoa tekee kolme osapuolta: sinä, agentti ja aggountant-ohjelma.

## Kolme osapuolta

Sinä tuot aineiston tai kerrot, mistä se löytyy. Vastaat kysymyksiin, joihin
vain yrityksen omistaja tietää vastauksen, ja hyväksyt jokaisen pysyvän
muutoksen ennen kuin se tehdään.

Agentti (esimerkiksi Claude Code) tekee rutiinityön. Se lukee kuitit,
tiliotteet ja muun aineiston, ehdottaa tilit ja ALV-käsittelyn ja näyttää
sinulle hyväksyttävän kokonaisuuden. aggountantin komennot se ajaa puolestasi.

aggountant tekee sen, minkä pitää mennä täsmälleen oikein. Se numeroi
tositteet aukottomasti, tarkistaa, että debet ja kredit ovat tasan, laskee
ALV:n verokantataulukosta, sulkee kaudet ja lähettää ilmoitukset vasta
hyväksynnän jälkeen.

aggountantissa ei ole tekoälyä, vaan se on agentissa, jota käytät. Agentti
taas ei kirjoita kirjanpitoa itse. Se pyytää aggountantia tallentamaan, ja
aggountant kieltäytyy kaikesta, mikä ei täsmää.

## Miksi työ on jaettu näin

Agentti osaa lukea sekalaista aineistoa ja päätellä, mikä tapahtuma on
kyseessä, mutta se voi erehtyä. aggountant ei päättele mitään, mutta laskee ja
tarkistaa joka kerta samalla tavalla. Siksi agentti ehdottaa ja aggountant
tarkistaa. Sinulle jää se, mitä kumpikaan ei voi tehdä: tiedät, kuuluuko kulu
yritykselle, ja päätät, mitä kirjanpitoon jää.

## Agentti toimii kirjanpitoa tuntevan työtoverin tavoin

Agentti saa ohjeensa aggountantin mukana tulevista skilleistä, joilla on
yhteinen roolikuvaus. Sen mukaan agentti ratkaisee itse sen, mihin
lähteet ja yrityksen aiempi kirjanpito riittävät, ja kysyy sinulta vain sen,
mitä se ei voi päätellä.

Lupaa kysymättä agentti ajaa kaiken, mikä vain lukee tai esikatselee eikä jätä
pysyviä jälkiä. Se esimerkiksi

- lukee tilikartan, aiemmat tositteet ja yrityksen tallennetut tiedot
- esikatselee ja tarkistaa tositteen
- esikatselee suljetun kuukauden ALV-ilmoituksen luvut
- tarkistaa suoran ilmoittamisen valmiuden ilman verkkoyhteyttä
- tekee raportteja.

Pysyvät teot, kuten tositteen sinetöinti, kuukauden sulku ja ilmoituksen
lähetys, odottavat hyväksyntääsi:
[Sinetöinti ja hyväksyntä](sealing-and-approval.md).

## Agentti kokoaa kysymykset

Kuukauden aineistossa voi olla kymmeniä tapahtumia, joten agentti ei keskeytä
sinua jokaisen kuitin kohdalla. Se kysyy silloin, kun vaihtoehtojen erolla on
sinulle merkitystä (tili, ALV-käsittely, kuuluuko kulu yritykselle) eikä se
voi päätellä, minkä valitsisit. Kysymykset se esittää kerralla. Vastauksesi se
voi tallentaa [yrityksen tietovarastoon](company-home.md#yrityksen-tietovarasto),
jotta samaa ei kysytä uudelleen.

## Mitä tämä tarkoittaa sinulle

- Kerro agentille tavoite, älä komentoja. "Kirjataan maaliskuu" riittää.
- Vastaa kysymyksiin tarkasti, koska vastauksesi ohjaa myös tulevia
  kirjauksia.
- Katso jokainen hyväksyntäpyyntö. Hyväksyntä on se kohta, jossa agentin virhe
  pysähtyy.

## Katso myös

- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
- [Asenna aggountant ja skillit](../start/install.md)
- [Yrityskoti: mitä tallentuu minne](company-home.md)
