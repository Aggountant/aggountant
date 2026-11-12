<!--
Näytesivu suunnitelmasta design.md: sivutyyppi Selitys (concept).
Tuleva paikka: docs/user/fi/concepts/sealing-and-approval.md.
Kokoaa nykyisen luvun 1 kohdan "Miksi tositetta ei voi muokata" ja luvun 3
kohdat "Mitä sinä hyväksyt" ja hyväksynnän sitominen näytettyyn.
-->

# Sinetöinti ja hyväksyntä

aggountantia käyttäessäsi huomaat kaksi asiaa heti. Agentti pyytää
hyväksyntääsi ennen jokaista tallennusta, ja kerran tallennettua tositetta ei
voi muuttaa. Tämä sivu kertoo, miksi näin on ja mitä se tarkoittaa
päivittäisessä työssä.

## Mitä sinetöinti on

Kun tosite tallennetaan, se **sinetöidään**. Se saa pysyvän juoksevan numeron,
ja sen sisällöstä lasketaan tunniste, joka ketjutetaan edellisen tositteen
tunnisteeseen. Näin syntyy **tositeketju**, jossa jokainen tosite riippuu
kaikista aiemmista.

Jos joku muuttaisi tai poistaisi vanhan tositteen jälkikäteen, ketju ei enää
täsmäisi. Agentti voi tarkistaa ketjun milloin tahansa, ja tarkistus kertoo,
onko kaikki ennallaan:

```
chain ok — 3 vouchers, head #3
```

Sinetöinti ei ole aggountantin oma keksintö. Kirjanpitolaki edellyttää, että
kirjaukset perustuvat päivättyihin ja numeroituihin tositteisiin, ne tehdään
pysyvällä tavalla ja virheen korjaus jättää alkuperäisen kirjauksen näkyviin. Paperikirjanpidossa sama tehtiin
kirjoittamalla musteella. aggountant tekee siitä teknisesti varman.

## Virhe korjataan uudella tositteella

Koska sinetöityä tositetta ei muokata, virhe korjataan **korjaustositteella**.
Jos Esimerkki Oy:n palvelinvuokra olisi kirjattu toimistokuluihin, uusi tosite
siirtää summan oikealle tilille ja viittaa alkuperäiseen. Kirjanpitoon jää
kumpikin, ja kuka tahansa näkee myöhemmin, mitä tapahtui ja milloin.

Sama koskee kokonaisia kuukausia. Suljetulle kuukaudelle ei kirjata, joten
korjaus menee seuraavalle avoimelle kuukaudelle.

## Miksi hyväksyntä tulee ennen sinetöintiä

Koska sinetöinti on pysyvä, ainoa hetki vaikuttaa siihen, mitä kirjanpitoon
jää, on ennen sinetöintiä. Siksi agentti ei sinetöi mitään ilman sinun
nimenomaista hyväksyntääsi. Hyväksyntä on se hetki, jolloin päätät, mitä
kirjanpitoon jää pysyvästi.

Sama periaate koskee muitakin tekoja, joita ei voi perua:

| Teko | Miksi hyväksyntä vaaditaan |
|---|---|
| tositteen sinetöinti | sinetöity tosite on pysyvä ja korjataan vain korjaustositteella |
| kuukauden sulku | suljetulle kuukaudelle ei voi enää kirjata |
| ALV-ilmoituksen lähetys | ilmoitus on oikeusvaikutteinen eikä sitä voi perua |
| Tulorekisteri-ilmoituksen lähetys | sama kuin ALV-ilmoituksessa |
| Vero.fi-tuotantoon siirtyminen ja sitä koskevat vakuutukset | ohjelma luottaa vakuutuksiisi eikä voi tarkistaa niitä |

Kaikki muu on agentin omaa työtä, jota se tekee kysymättä: tilikartan ja aiempien
tositteiden lukeminen, kirjausten esikatselu, raportit ja tarkistukset. Ne eivät
jätä pysyviä jälkiä.

## Hyväksyt sen, mitä sinulle näytettiin

Hyväksyntä ei ole yleinen lupa. Se on sidottu täsmälleen siihen sisältöön, jonka
agentti näytti. Kun agentti esikatselee kuitin kirjauksen, ohjelma laskee
esikatselusta tunnisteen. Sinetöidessään agentti antaa saman tunnisteen takaisin.
Jos kuitin tiedot, agentin ehdotus tai tilikartta ovat muuttuneet välissä, ohjelma
kieltäytyy:

```
CAPTURE_PREPARATION_CHANGED
```

Agentin on silloin näytettävä uusi esikatselu ja pyydettävä hyväksyntä
uudelleen. Et siis voi hyväksyä yhtä ja saada toista, vaikka agentti erehtyisi.

Viranomaisilmoituksissa sidonta on sama: ALV-ilmoituksen hyväksyntä koskee
juuri sitä sisältöä, joka lähetetään, ja ohjelma laskee luvut vielä kerran
kirjanpidosta ennen lähetystä.

Pankin tiliotteen tuonnissa ei tässä versiossa ole erillistä esikatselun
tunnistetta. Hyväksyntäsi on se, että katsot agentin kirjauslistan rivi riviltä
ennen kuin se sinetöi tiliotteen.

## Mitä tämä tarkoittaa sinulle

- **Katso, mitä hyväksyt.** Hyvä hyväksyntäpyyntö näyttää jokaisesta
  tositteesta päivän, summan, vastapuolen, tilin ja ALV-käsittelyn. Pelkkä
  "kirjaanko 14 tositetta?" ei riitä. Pyydä rivit nähtäväksi, jos agentti ei
  näytä niitä.
- **Kun kieltäydyt tai korjaat, mitään ei ole vielä tallentunut.** Ennen
  hyväksyntää voit muuttaa ehdotusta vapaasti.
- **Virhe sinetöidyssä tositteessa ei ole katastrofi.** Se korjataan uudella
  tositteella, kuten kirjanpidossa aina.
- **Älä yritä korjata tositetietokantaa käsin.** Ketjun tarkistus paljastaa
  muutoksen, ja kirjanpito on silloin palautettava varmuuskopiosta.

## Katso myös

- [Korjaa virheellinen tosite](../tasks/correct-voucher.md)
- [Kaudet, sulku ja tilikausi](periods.md)
- [Sinä, agentti ja aggountant](division-of-work.md)
