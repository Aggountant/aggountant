# ALV-ilmoitusten siirto

Mitä vanhassa ohjelmassa saa tehdä ja miten aineisto arkistoidaan, on
[SKILL.md](SKILL.md)-tiedostossa. Ilmoittaminen itse on skillissä
[vero-fi](../vero-fi/SKILL.md).

aggountant laskee ALV-ilmoituksen omista sinetöidyistä tositteistaan. Se voi siis
ilmoittaa vain kauden, joka on kokonaan sen kirjanpidossa, ja ALV:n vaihtopäivä on
aikaisintaan kirjanpidon vaihtopäivä. Jos kirjanpito vaihtuu kesken ALV-kauden,
esimerkiksi neljännesvuosikauden toisen kuukauden alussa, kauden ilmoitukseen tarvitaan
molempien järjestelmien luvut, eikä aggountantin esikatselu näe vanhan osuutta. Silloin
kauden ilmoittaa käyttäjä OmaVerossa yhdistetyin luvuin, ja laskelma ja kuittaus
menevät arkistoon. Ehdota siksi kirjanpidon vaihtoa ALV-kauden vaihteeseen ja kysy
kauden pituus käyttäjältä tai OmaVerosta, ei vanhan ohjelman asetuksista.

## Mitä kerätään

- Annetut ALV-ilmoitukset ja niiden vastaanottokuittaukset jokaiselta säilytettävältä
  kaudelta. OmaVero näyttää ne myöhemminkin, mutta vanhan ohjelman kuittaus kertoo,
  mitkä luvut juuri se lähetti.
- Korvaavat ilmoitukset ja selvitykset, joita kausista on annettu, ja Verohallinnon
  päätökset. Myöhempi korjaus tehdään alkuperäisen ilmoituksen lukuja vasten.
- Kuka ilmoitti: oma ohjelma, tilitoimisto vai käyttäjä OmaVerossa. Tieto kertoo,
  mistä puuttuva kuittaus kysytään.

## Viimeinen ja ensimmäinen kausi

Vanhan järjestelmän viimeisen kauden määräpäivä osuu usein vaihtopäivän jälkeen. Sovi
nimenomaisesti, kuka sen ilmoittaa, ja kirjaa se `TODO.md`-tiedostoon, kunnes kuittaus
on arkistossa: oletus, että toinen hoitaa, jättää kauden kokonaan ilmoittamatta.

Ensimmäinen aggountantin kausi on ensimmäinen ALV-kausi, joka alkaa ALV:n vaihtopäivää
seuraavana päivänä. Sen esikatselu näyttää avaavan tositteen ALV-tilien saldot
merkitsemättöminä vienteinä, koska ne ovat vanhojen kausien veroa; vero-fi kertoo,
miten ne kuitataan. ALV:n voi siirtää myöhemmin kuin kirjanpidon, kun esimerkiksi
tilitoimisto ilmoittaa vielä joitakin kausia aggountantin luvuista OmaVerossa. Ne
kaudet kirjataan käsin annettuina, ja alueen vaihtopäivä on viimeisen sellaisen kauden
loppu.

Kirjaa alueen tietueeseen ([SKILL.md](SKILL.md)) ALV-kauden pituus, viimeinen vanhalla
tavalla ilmoitettu kausi ja kuka sen ilmoitti.
