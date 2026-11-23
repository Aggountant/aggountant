# Sinetöinti ja hyväksyntä

Agentti pyytää hyväksyntääsi ennen jokaista tallennusta, eikä kerran
tallennettua tositetta voi muuttaa. Molemmat johtuvat siitä, että tallennus on
pysyvä.

## Mitä sinetöinti on

Kun tosite tallennetaan, se sinetöidään. Se saa pysyvän juoksevan numeron, ja
sen sisällöstä lasketaan tunniste, joka ketjutetaan edellisen tositteen
tunnisteeseen. Näin syntyy tositeketju, jossa jokainen tosite riippuu kaikista
aiemmista.

Jos vanhaa tositetta muutetaan tai se poistetaan jälkikäteen, ketju ei enää
täsmää. Agentti voi tarkistaa ketjun milloin tahansa. Kun kaikki on ennallaan,
tarkistus tulostaa:

```
chain ok — 3 vouchers, head #3
```

Vaatimus tulee kirjanpitolaista: kirjausten on perustuttava päivättyihin ja
numeroituihin tositteisiin, ne on tehtävä pysyvällä tavalla, ja virheen
korjauksen on jätettävä alkuperäinen kirjaus näkyviin. Paperikirjanpidossa
tämä hoidettiin kirjoittamalla musteella, aggountantissa sen tekee
tositeketju.

## Virhe korjataan uudella tositteella

Sinetöidyn tositteen virhe korjataan korjaustositteella. Jos Esimerkki Oy:n
palvelinvuokra on kirjattu toimistokuluihin, uusi tosite siirtää summan
oikealle tilille ja viittaa alkuperäiseen. Kirjanpitoon jäävät molemmat, ja
kuka tahansa näkee myöhemmin, mitä tapahtui ja milloin.

Suljetulle kuukaudelle ei kirjata, joten sen virheen korjaus menee seuraavalle
avoimelle kuukaudelle.

## Miksi hyväksyntä tulee ennen sinetöintiä

Sinetöinnin jälkeen tositteeseen ei voi enää vaikuttaa, joten päätät ennen
sitä, mitä kirjanpitoon jää. Siksi agentti sinetöi vasta, kun olet
nimenomaisesti hyväksynyt tositteen.

Hyväksyntääsi odottavat muutkin teot, joita ei voi perua:

| Teko | Miksi hyväksyntä vaaditaan |
|---|---|
| tositteen sinetöinti | sinetöity tosite on pysyvä ja korjataan vain korjaustositteella |
| kuukauden sulku | suljetulle kuukaudelle ei voi enää kirjata |
| ALV-ilmoituksen lähetys | ilmoitus on oikeusvaikutteinen eikä sitä voi perua |
| Tulorekisteri-ilmoituksen lähetys | sama kuin ALV-ilmoituksessa |
| Vero.fi-tuotantoon siirtyminen ja sitä koskevat vakuutukset | ohjelma luottaa vakuutuksiisi eikä voi tarkistaa niitä |

Kaiken muun agentti tekee kysymättä: se lukee tilikarttaa ja aiempia
tositteita, esikatselee kirjauksia ja ajaa raportteja ja tarkistuksia. Niistä
ei jää pysyvää jälkeä.

## Hyväksyt sen, mitä sinulle näytettiin

Hyväksyntä koskee täsmälleen sitä sisältöä, jonka agentti näytti. Kun agentti
esikatselee kuitin kirjauksen, ohjelma laskee esikatselusta tunnisteen, ja
sinetöidessään agentti antaa saman tunnisteen takaisin. Jos kuitin tiedot,
agentin ehdotus tai tilikartta ovat muuttuneet välissä, ohjelma kieltäytyy:

```
CAPTURE_PREPARATION_CHANGED
```

Agentti näyttää silloin uuden esikatselun ja pyytää hyväksyntää uudelleen. Et
siis voi hyväksyä yhtä ja saada toista, vaikka agentti erehtyisi.

Pankin tiliotteen tuonti toimii samoin. Sen esikatselu näyttää jokaisen
tositteen, jonka tiliote tuottaisi, ja jos jokin muuttuu esikatselun jälkeen,
ohjelma ei sinetöi mitään (`IMPORT_PREPARATION_CHANGED`).

Viranomaisilmoituksissa hyväksyntä sidotaan samalla tavalla: ALV-ilmoituksen
hyväksyntä koskee juuri sitä sisältöä, joka lähetetään, ja ohjelma laskee
luvut vielä kerran kirjanpidosta ennen lähetystä.

## Mitä tämä tarkoittaa sinulle

- Katso, mitä hyväksyt. Hyvä hyväksyntäpyyntö näyttää jokaisesta tositteesta
  päivän, summan, vastapuolen, tilin ja ALV-käsittelyn. Pelkkä "kirjaanko 14
  tositetta?" ei riitä, joten pyydä rivit nähtäväksi, jos agentti ei näytä
  niitä.
- Ennen hyväksyntää voit muuttaa ehdotusta vapaasti tai hylätä sen, sillä
  mitään ei ole vielä tallentunut.
- Älä korjaa tositetietokantaa käsin. Ketjun tarkistus paljastaa muutoksen,
  ja kirjanpito on silloin
  [palautettava varmuuskopiosta](../tasks/back-up-and-restore.md).

## Katso myös

- [Korjaa virheellinen tosite](../tasks/correct-voucher.md)
- [Kaudet, sulku ja tilikausi](periods.md)
- [Sinä, agentti ja aggountant](division-of-work.md)
