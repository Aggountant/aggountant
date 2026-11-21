---
name: tulorekisteri
description: >-
  Ilmoita Tulorekisteriin: verovapaat matkakorvaukset (kilometrikorvaus 311,
  päiväraha 331) ja työnantajan erillisilmoitus joka kuukaudelta. Selvitä
  epävarma lähetys, kirjaa asiointipalvelussa annettu ilmoitus, ota
  Tulorekisteri-yhteys käyttöön ja kuro kiinni puuttuvat kuukaudet.
cli_version: "0.1.0"
schema_version: 1
---

# Ilmoitukset Tulorekisteriin

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi (`aggountant skill print aggountant-init --file SOUL.md`).

Aggountant antaa Tulorekisteriin kaksi ilmoitusta yrityksen omalla varmenteella:
verovapaiden matkakorvausten palkkatietoilmoituksen ja työnantajan erillisilmoituksen.
Rahapalkkoja se ei ilmoita. Tulorekisteri on eri kanava kuin ALV-ilmoituksen Vero API:
sillä on oma rajapintaoikeutensa, ja sen virheet ja korjaukset hoidetaan Tulorekisterin
asiointipalvelussa, ei OmaVerossa. Kumpikin ilmoitus on annettu, kun se on lähetetty,
joten käyttäjä hyväksyy jokaisen esikatselusta. Käyttöönotto on tiedostossa
[kayttoonotto.md](kayttoonotto.md).

## Matkakorvausilmoitus

Yksi ilmoitus on yksi maksu yhdelle tulonsaajalle (`aggountant file-wage preview`,
jonka ohje kertoo syötteen). Ohjelma näyttää varovaisen määräpäivän, viisi päivää
maksupäivästä. Laadi ilmoitus
matka- tai kululaskusta ja kysy käyttäjältä se, mitä laskusta ei näe:

- Ansaintajakso annetaan itse; ohjelma ei päättele sitä kululaskusta.
- `joint_owner` on tieto pääomaosakkuudesta. Se ei seuraa toimitusjohtajan asemasta
  eikä ole sama asia kuin työttömyysturvan osaomistajuus.
- Riveille kuuluvat vain kilometrikorvaukset ja päivärahat. Kuittia vastaan korvatut
  kulut, kuten hotelli tai pysäköinti, eivät kuulu niille.
- `source_reference` yksilöi maksun pysyvästi. Uusi tunniste tai muutettu maksupäivä
  olisi Tulorekisterille uusi ilmoitus, joten niitä ei vaihdeta hylkäyksen tai
  epävarmuuden kiertämiseksi. Samasta syystä kysy ennen aggountantia maksetusta
  korvauksesta, onko aiempi palkanlaskija jo ilmoittanut sen.

Henkilötunnus ei kuulu syötteeseen eikä keskusteluun: ohjelma hakee sen
työntekijärekisteristä. Esikatselun tarkiste `record_hash` on johdettu
henkilötunnuksesta, ja koska tunnusten avaruus on pieni, se on itsekin arkaluonteinen.
Käytä sitä vain hyväksyntäkomennossa; älä kirjoita sitä `TODO.md`:hen, gitiin tai
viesteihin. Vastauksen `sensitive`-kenttä sanoo saman.

Käyttäjä hyväksyy rivit, jakson, maksupäivän ja osakkuustiedon. Esikatselulle ja
käynnistykselle annetaan sama aikaleima, koska tarkiste sitoo sen.

## Työnantajan erillisilmoitus

Verohallinnon työnantajarekisterissä oleva yritys antaa erillisilmoituksen jokaiselta
kuukaudelta, myös silloin kun mitään ei maksettu. Määräpäivä on seuraavan kuukauden
5. päivä, lauantaina tai pyhänä seuraava pankkipäivä (ohjelma näyttää aina 5. päivän).
Matkakorvausilmoitus ei korvaa sitä. Satunnainen työnantaja, joka ei ole rekisterissä,
ei anna ilmoitusta kuukausilta, joilta se ei maksanut mitään. Kuukaudelta, jolta se
maksoi jotain, aggountant ei anna erillisilmoitusta: selvitä käyttäjän kanssa
Tulorekisterin ohjeesta, tarvitaanko se, ja jos tarvitaan, hän antaa sen
asiointipalvelussa.

`aggountant file-wage employer-report missing` luettelee kuukaudet, joilta ilmoitus
puuttuu. Sisällön ratkaisee se, mitä kuukaudella maksettiin (`--payments`). Selvitä se
kirjanpidosta ja tiliotteelta tai kysy; ohjelma ei päättele sitä tileistä. Kuukausi,
jolta maksettiin vain verovapaita matkakorvauksia, ei ole "Ei palkanmaksua" vaan
sairausvakuutusmaksu 0 € (Tulorekisterin ohje, kohta 4.3). Rahapalkkakuukautta
aggountant ei osaa: käyttäjä antaa sen asiointipalvelussa.

Jokainen kuukausi esikatsellaan, hyväksytetään ja lähetetään erikseen, myös silloin
kun kuukausia on monta. Aikaleima on samalla ilmoituspäivä, joten se saa lähetettäessä
olla enintään vuorokauden vanha. Tämän ilmoituksen tarkisteessa ei ole henkilötietoja,
joten sen saa näyttää.

Hyväksytyn kuukauden korjaus on korvaava ilmoitus (`--replace`). Eläkevakuutustietoja
sillä ei voi muuttaa, vaan se tehdään asiointipalvelussa. Jos yrityksen aiempia
ilmoituksia ei tunneta tai rekisteröinnin alku on epäselvä, lue
[kiinnikurominen.md](kiinnikurominen.md) ennen kuin lähetät vanhoja kuukausia.

## Kun lähetyksestä ei ole varmuutta

Aikakatkaisu tai ristiriitainen palaute ei kerro, onko ilmoitus rekisterissä. Ohjelma
ei lähetä uudelleen eikä anna lähettää: tila kysytään (`aggountant file-wage reconcile`
tai erillisilmoitukselle `aggountant file-wage employer-report status`), ja vastaus
kertoo, mitä seuraavaksi. Jos tila ei selviä, kysy myöhemmin uudelleen tai pyydä
käyttäjää katsomaan asiointipalvelusta. Ohjelma ei näytä Tulorekisterin virhetekstejä,
koska niissä voi olla henkilötunnus; tarkan virheen näkee asiointipalvelusta.

Hylätty tai korjattava matkakorvausilmoitus hoidetaan asiointipalvelussa, koska
aggountant ei tee sille korvaavaa ilmoitusta eikä mitätöintiä. Selvitä ensin, onko
aiempi lähetys jo rekisterissä, ettei samasta maksusta synny kahta ilmoitusta.
Asiointipalvelussa annettu ilmoitus, jota aggountant ei saanut perille, kirjataan
viitteineen `complete-manual`-komennolla. Jo hyväksytyn ilmoituksen korjausta ohjelma
ei kirjaa, vaikka komento onnistuu: arkistoi korjauksen kuitti ja merkitse asia kodin
muistiin.
[e-services](../e-services/SKILL.md) kertoo, miten palvelun sivut toimivat ja missä
selaimen raja kulkee.

Kodin `TODO.md`:ssä määräaika poistetaan, kun ilmoitus on hyväksytty tai kirjattu
käsin annettuna; selvittämättä jäänyt lähetys jää kohdaksi tunnisteineen.
