---
name: tulorekisteri
description: >-
  Ilmoita Tulorekisteriin: verovapaat matkakorvaukset (kilometrikorvaus 311,
  päiväraha 331) ja työnantajan erillisilmoitus joka kuukaudelta. Selvitä
  epävarma lähetys, kirjaa asiointipalvelussa annettu ilmoitus, ota
  Tulorekisteri-yhteys käyttöön ja kuro kiinni puuttuvat kuukaudet.
cli_version: "0.3.0"
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

Kanava on käytössä tuotannossa. Marraskuuhun 2026 mennessä matkakorvausilmoitus
(tulolajit 311 ja 331) on hyväksytty Tulorekisterissä synkronisesti: lähetyksen tila
3, ilmoitus voimassa olevien joukossa versiona 1, ja vastaus toi lähetyksen ja
ilmoituksen tunnisteet (`ir_delivery_id`, `ir_report_id`). Kun `file-wage approve`
palauttaa kuittauksen, ilmoitus on rekisterissä.

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
- Jos kululaskusta on kuitattu matkaennakkoa, ilmoita laskun kilometrikorvaukset ja
  päivärahat kokonaan, ei ennakkoa eikä nettomaksua. Kuittauksella maksetun osan
  maksupäivä on kuittauksen päivä
  ([Verohallinnon ohje, luku 2.1](https://vero.fi/tulorekisteri/yritykset-ja-organisaatiot/yksityiskohtaiset-tulorekisterin-ohjeet/64028/tietojen-ilmoittaminen-tulorekisteriin-luontoisedut-ja-kustannusten-korvaukset12/)).
- `source_reference` yksilöi maksun pysyvästi. Uusi tunniste tai muutettu maksupäivä
  olisi Tulorekisterille uusi ilmoitus, joten niitä ei vaihdeta hylkäyksen tai
  epävarmuuden kiertämiseksi. Samasta syystä kysy ennen aggountantia maksetusta
  korvauksesta, onko aiempi palkanlaskija jo ilmoittanut sen.

Henkilötunnus ei kuulu syötteeseen eikä keskusteluun: ohjelma hakee sen
työntekijärekisteristä. Esikatselun tarkiste `record_hash` on johdettu
henkilötunnuksesta, ja koska tunnusten avaruus on pieni, se on itsekin arkaluonteinen.
Käytä sitä vain hyväksyntäkomennossa; älä kirjoita sitä `TODO.md`:hen, gitiin tai
viesteihin. Vastauksen `sensitive`-kenttä sanoo saman.

Älä myöskään lue tarkistetta esikatselusta itse ja kirjoita sitä komentoriville:
`--record-hash HASH` jää komentohistoriaan ja keskusteluun. Ota tarkiste talteen siitä
samasta esikatselusta, jonka käyttäjä hyväksyy, `mktemp`-komennolla tehtyyn
tiedostoon kodin ulkopuolelle, ja näytä esikatselu ilman sitä:

```sh
aggountant --json file-wage preview --timestamp T < ilmoitus.json \
  | tee >(jq -r .record_hash > TIEDOSTO) | jq 'del(.record_hash)'
```

Hyväksyntä lukee sen tiedostosta (`file-wage approve wr-… --record-hash-file
TIEDOSTO`), ja tiedosto poistetaan heti sen jälkeen. Älä laske tarkistetta
hyväksynnän hetkellä uudesta esikatselusta: se hyväksyisi sen, mitä syötteessä silloin
on, eikä sitä, minkä käyttäjä näki. Korjauksen `submit` ottaa tarkisteen samalla
tavalla.

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

Hylätty matkakorvausilmoitus hoidetaan asiointipalvelussa. Selvitä ensin, onko aiempi
lähetys jo rekisterissä, ettei samasta maksusta synny kahta ilmoitusta.
Asiointipalvelussa annettu ilmoitus, jota aggountant ei saanut perille, kirjataan
viitteineen `complete-manual`-komennolla. Jo hyväksytylle ilmoitukselle komento
kieltäytyy (`WAGE_REPORT_ALREADY_ACCEPTED`) eikä kirjaa mitään: hyväksytty ilmoitus
korjataan alla kuvatulla tavalla.
[e-services](../e-services/SKILL.md) kertoo, miten palvelun sivut toimivat ja missä
selaimen raja kulkee.

## Hyväksytyn matkakorvausilmoituksen korjaus

Rekisterin hyväksymä ilmoitus korjataan aggountantilla, ei asiointipalvelussa. Korjaus
on oma ilmoituksensa: se esikatsellaan, käyttäjä hyväksyy sen tarkisteen ja se
lähetetään kerran (`aggountant file-wage correction preview` ja `submit`).

- **Korvaava ilmoitus** (`--action replace`) antaa koko ilmoituksen uudelleen
  korjattuna: rivit, yhteyshenkilön tai pääomaosakkuuden. Maksajaa, tulonsaajaa,
  maksupäivää ja ansaintajaksoa sillä ei voi muuttaa, koska rekisteri vaatii niiden
  pysyvän samoina. Tarkiste on johdettu henkilötunnuksesta kuten alkuperäisessä.
- **Mitätöinti** (`--action cancel`) poistaa ilmoituksen. Sitä käytetään, kun maksua
  ei ollutkaan tai kun pitää muuttaa jotain, mitä korvaava ei saa muuttaa. Uusi
  ilmoitus annetaan sen jälkeen tavalliseen tapaan omalla `source_reference`-arvollaan,
  koska mitätöidyn ilmoituksen tunniste on käytetty.

Esikatselu näyttää rinnakkain rekisterissä olevan ja korjatun ilmoituksen sekä version,
jota korjataan; näytä käyttäjälle, mikä muuttuu. `aggountant file-wage correction
status` kertoo, mitä rekisterissä nyt on (hyväksytty, korvattu, mitätöity) ja
ilmoituksen koko historian. Alkuperäisen ajon `run show` sanoo `superseded`, kun sen
kuittaus ei enää ole voimassa oleva. Jos korjauksen lähetyksestä ei ole varmuutta, sama
`status` kysyy tilan; mitään ei lähetetä uudelleen ennen kuin se on selvinnyt.

Jos käyttäjä on jo tehnyt korvaavan ilmoituksen tai mitätöinnin asiointipalvelussa,
kirjaa se komennolla `aggountant file-wage correction complete-manual`: ilmoitusviite
pysyy samana, ja palvelun näyttämä uusi versio annetaan mukaan. Sen jälkeen ohjelma ei
tunne ilmoituksen sisältöä, ja se sanoo niin seuraavassa esikatselussa.

Kodin `TODO.md`:ssä määräaika poistetaan, kun ilmoitus on hyväksytty tai kirjattu
käsin annettuna; selvittämättä jäänyt lähetys jää kohdaksi tunnisteineen.
