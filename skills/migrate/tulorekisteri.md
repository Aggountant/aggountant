# Tulorekisteri-ilmoitusten siirto

Koskee vain yritystä, joka ilmoittaa Tulorekisteriin: maksaa verovapaita
matkakorvauksia tai on työnantajarekisterissä. Mitä vanhassa ohjelmassa saa tehdä ja
miten aineisto arkistoidaan, on [SKILL.md](SKILL.md)-tiedostossa; ilmoittaminen itse
on skillissä [tulorekisteri](../tulorekisteri/SKILL.md).

aggountant ilmoittaa Tulorekisteriin verovapaat matkakorvaukset ja työnantajan
erillisilmoituksen, ei rahapalkkoja. Jos yritys maksaa palkkaa, palkanlaskenta ja sen
ilmoitukset jäävät palkanlaskijalle tai asiointipalveluun, eikä niitä siirretä. Sovi
siksi ensin, kumpi ilmoitus siirtyy ja mistä kuukaudesta. Vaihtopäivä on kuukauden
loppu: ilmoitukset annetaan kuukausittain, ja kesken kuukauden saman kuukauden
maksuilla olisi kaksi ilmoittajaa.

## Mitä kerätään

- Tulonsaajat. Ne lisätään aggountantin työntekijärekisteriin ennen ensimmäistä
  ilmoitusta ([tulorekisteri: tulonsaaja](../tulorekisteri/kayttoonotto.md#tulonsaaja)),
  ja matka- ja kululaskut käyttävät samaa rekisteriä.
- Annetut ilmoitukset tunnisteineen: ilmoitusviite, maksajan oma viite ja versio.
  Myöhempi korjaus kohdistetaan niihin, eikä niitä saa muualta kuin alkuperäiseltä
  ilmoittajalta tai asiointipalvelusta. aggountant ei tunne toisen lähettämää
  matkakorvausilmoitusta, joten sellainen korjataan asiointipalvelussa näillä
  tiedoilla.
- Maksut, jotka on maksettu mutta ei vielä ilmoitettu vaihtopäivänä (alla).

## Muualla ilmoitetut kuukaudet

Erillisilmoituksen siirrossa ohjelman pitää tietää, mistä kuukaudesta velvollisuus
alkoi ja mitkä kuukaudet on jo ilmoitettu, muuten se ehdottaa jo ilmoitettua kuukautta
uudelleen. Ne saadaan työnantajarekisteristä ja Tulorekisterin raportista 630, ja
menettely on tiedostossa
[kiinnikurominen.md](../tulorekisteri/kiinnikurominen.md). Tee se ennen ensimmäistä
aggountantin erillisilmoitusta, myös silloin, kun vanhan ilmoittajan kuukausissa ei
pitäisi olla aukkoja: raportti osoittaa sen. Arkistoi raportti alueen aineiston
joukkoon.

## Maksettu mutta ilmoittamatta

Ennen vaihtopäivää maksetun korvauksen ilmoitus erääntyy maksukuukautta seuraavan
kuukauden 5. päivänä, usein vaihtopäivän jälkeen. Käy vaihtopäivän kuukauden maksut
läpi käyttäjän ja vanhan ilmoittajan kanssa ja sovi jokaisesta, kuka sen ilmoittaa.
Jos maksu jää kummaltakin, ilmoitus puuttuu; jos molemmat ilmoittavat, Tulorekisterissä
on kaksi ilmoitusta samasta maksusta, ja toinen on mitätöitävä. aggountantin
ilmoittamaan maksuun tulee sen alkuperäinen maksupäivä. Kirjaa sovitut maksut
`TODO.md`-tiedostoon, kunnes kuittaus on saatu.

Kirjaa alueen tietueeseen ([SKILL.md](SKILL.md)) ensimmäinen kuukausi, jonka aggountant
ilmoittaa kummastakin ilmoituksesta, ja kuka ilmoitti sitä edeltävät.
