# Erillisilmoitusten kiinnikurominen

Kun yritys on ollut työnantajarekisterissä jo ennen aggountantia, ohjelma ei tiedä,
mistä kuukaudesta velvollisuus alkoi eikä mitkä kuukaudet aiempi palkanlaskija on jo
ilmoittanut. Ilman näitä `file-wage employer-report missing` näyttää väärän listan:
liian myöhäinen alkukuukausi piilottaa puuttuvia kuukausia, ja jo ilmoitetun kuukauden
lähettäminen uudelleen tekisi siitä kaksi ilmoitusta. Selvitä siis historia ennen kuin
lähetät yhtään vanhaa kuukautta.

## Rekisteröinnin alku

`aggountant file-wage employer-report registration` hakee työnantajarekisterin
merkinnät PRH:n avoimesta datasta ja vertaa niitä profiilin alkukuukauteen
kirjoittamatta mitään. Näytä tulos käyttäjälle ja muuta alkukuukausi vasta hänen
luvallaan ([kayttoonotto.md](kayttoonotto.md)). Jos YTJ ei vastaa, rekisteröinti on
"ei tiedossa", ei "ei rekisterissä".

## Mitä on jo ilmoitettu: raportti 630

Aiemmin annetut ilmoitukset saa vain Tulorekisterin asiointipalvelusta. Tavallisesti
tämä on ainoa vaihe, jossa käyttäjän pitää kirjautua sinne; sano se hänelle suoraan. Raportti
on *Maksajan palkkatietojen yhteenveto (csv)* (630). Tilauksen rajaukset ratkaisevat,
onko tuonti oikein:

- kohdekausi rekisteröinnin koko ajalta, enintään 24 kuukautta tilausta kohden
- kieli suomi, koska tuonti tunnistaa vain suomenkieliset sarakkeet
- vastaanottohetken aikaväli tyhjäksi. Rajattu raportti jättää pois muulloin
  vastaanotetut ilmoitukset, eikä puutetta näe tiedostosta: kuukausi näyttäisi
  ilmoittamattomalta, vaikka se on ilmoitettu.

Miten raportti tilataan ja ladataan, on
[e-services-skillin Tulorekisteri-ohjeessa](../e-services/tulorekisteri.md).

Tuo tiedosto komennolla `aggountant file-wage employer-report import-transcript`,
ensin `--dry-run`. Raportissa ei ole maksajan Y-tunnusta, joten varmista käyttäjältä,
että tiedosto on tämän yrityksen. Ristiriidat näytetään käyttäjälle; niitä ei ratkaista
itse. Jos ohjelma ei tunnista tiedoston sarakkeita, tiedostoa ei muokata sopivaksi:
se tilataan uudelleen suomeksi, ja jos se on oikea raportti, asiointipalvelun muoto on
muuttunut ja käyttäjän pitää tietää se.

## Puuttuvat kuukaudet

Tuonnin jälkeen `missing` näyttää kuukaudet, joilta ilmoitusta ei raportin tilaushetkellä
ollut; vanha raportti ei tunne sen jälkeen annettuja. Käy ne läpi vanhimmasta alkaen,
kukin omalla esikatselullaan ja hyväksynnällään ([SKILL.md](SKILL.md)).

Vanhankin kuukauden ilmoitukseen tulevat profiilin nykyiset eläkevakuutustiedot. Kysy
käyttäjältä, pitivätkö ne paikkansa myös niinä kuukausina; jos eivät, kuukausi annetaan
asiointipalvelussa.

Raportin perusteella annetuksi merkityllä kuukaudella ei ole ilmoitusviitettä, joten
aggountant ei voi lähettää sille korvaavaa ilmoitusta. Kun käyttäjä pyytää korjaamaan
sellaista kuukautta, kerro vaihtoehdot: korjaus asiointipalvelussa, tai hän hakee
sieltä ilmoituksen viitteen, se kirjataan komennolla
`aggountant file-wage employer-report complete-manual`, ja sen jälkeen korvaavan voi
lähettää. Anna myös ilmoituksen versio, jos se on tiedossa: korvaava kohdistuu silloin
oikeaan versioon. Ohjelma ei tunne alkuperäisen ilmoituksen sisältöä eikä siksi voi
tarkistaa, että korvaavassa on samat eläkevakuutustiedot, kuten Tulorekisteri vaatii.
Pyydä käyttäjää katsomaan ne alkuperäisestä ilmoituksesta; jos ne eroavat profiilista,
korjaus tehdään asiointipalvelussa.

Kuukausi, josta kaksi raporttia on eri mieltä, pysyy lukittuna, kunnes käyttäjä on
katsonut sen asiointipalvelusta ja olemassa oleva ilmoitus on kirjattu viitteineen.
