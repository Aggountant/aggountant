# Matka- ja kululaskujen siirto

Koskee vain yritystä, joka maksaa henkilöille kululaskuja tai matkakorvauksia. Mitä
vanhassa ohjelmassa saa tehdä ja miten aineisto arkistoidaan, on
[SKILL.md](SKILL.md)-tiedostossa.

Alue edellyttää, että kirjanpito on siirretty, koska laskun maksu kirjataan
aggountantiin; että henkilöt ovat työntekijärekisterissä, koska laskut ja
työpaikkatiedot viittaavat sinne; ja että Tulorekisterin alue on sovittu, koska
ilmoittamaton korvaus ilmoitetaan sen kautta ([tulorekisteri.md](tulorekisteri.md)).

## Vanhat laskut jäävät arkistoon

Vanhaa kululaskua ei syötetä eikä sinetöidä uudelleen. Se on jo tosite liitteineen,
ja se arkistoidaan niiden mukana kirjanpidon alueessa ([kirjanpito.md](kirjanpito.md)).
Uudelleen syötetty lasku olisi sama kulu toiseen kertaan, ja uudelleen laskettu
korvaus voisi erota siitä, mitä henkilölle maksettiin ja Tulorekisteriin ilmoitettiin.
Jos yrityksellä on laskuja omassa tiedostomuodossaan, niiden muunnos on sen oma työ;
aggountantissa ei ole tuontia toisen ohjelman laskuille.

## Mitä siirtyy vaihtopäivänä

- **Hyväksytyt mutta maksamattomat laskut.** Ne ovat avaavissa saldoissa velkana
  henkilölle. Maksu kirjataan sitä velkaa vastaan, ei kuluksi, muuten sama kulu on
  kirjattu kahdesti.
- **Maksetut mutta ilmoittamattomat korvaukset.** Ne kuuluvat Tulorekisterin alueen
  sopimukseen siitä, kuka ilmoittaa minkäkin maksun.
- **Kunkin henkilön varsinainen työpaikka.** Matkan verovapaus riippuu siitä, ja se
  kirjataan työpaikkarekisteriin käyttäjän vahvistamana, päivättynä ja perusteineen
  (`aggountant claimant add`; paikat `aggountant claimant place`). Älä päättele sitä
  vanhoista laskuista: ne kertovat, minne matkustettiin, eivät sitä, mikä työpaikka
  oli. Jos vanhassa ohjelmassa on työpaikka-asetus, se on lähde, jonka käyttäjä
  vahvistaa tai korjaa.
- **Aiempi työskentely asiakkaan tiloissa.** Kolmen vuoden sääntö laskee työskentelyn
  samassa erityisessä työntekemispaikassa myös ajalta ennen aggountantia, joten
  luetteloi henkilöittäin paikat ja jaksot vanhoista laskuista ja vahvistuta ne
  käyttäjällä.

## Odottaa ohjelmaa

Vaihtopäivänä keskeneräinen lasku syötetään kululaskun luonnokseksi
(`expense-claim`-skilli), jolloin ohjelma laskee korvaukset ja kysyy puuttuvan, eikä
omaa laskelmaa tehdä. Tätä kirjoitettaessa ohjelmassa ei ole kululaskun hyväksyntää,
vanhan aggountant-tositteen liittämistä laskuun eikä kolmen vuoden säännön seurantaa.
Jos `aggountant --help` jo näyttää ne, käytä niitä tämän osion sijaan.

Siihen asti:

- Valmiin luonnoksen korvaukset kirjataan kuten uusi kululasku nykyisin
  (`capture-receipt`-skilli).
- Aiemman työskentelyn luettelo säilytetään arkiston johdettujen tiedostojen joukossa,
  koska siinä on henkilötietoja, ja `TODO.md`-tiedostoon kirjataan, että se syötetään
  ohjelmaan, kun seuranta on käytössä.

Kirjaa alueen tietueeseen ([SKILL.md](SKILL.md)) ensimmäinen päivä, jonka laskut
käsitellään aggountantissa, ja mitkä henkilöt ja työpaikat kirjattiin.
