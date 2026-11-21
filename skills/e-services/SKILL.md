---
name: e-services
description: >-
  Käytä viranomaisten verkkopalveluja käyttäjän selaimessa: OmaVero (kirjeiden
  lataus, lomakkeet) ja Tulorekisterin asiointipalvelu (raportin 630 tilaus ja
  lataus). Kertoo, miten sivut käyttäytyvät ja mitä selaimen on sallittava,
  päivättyinä havaintoina. Avaa ennen kuin teet mitään näillä sivuilla.
cli_version: "0.1.0"
schema_version: 1
---

# Verkkopalvelujen käyttö selaimessa

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi (`aggountant skill print aggountant-init --file SOUL.md`).

## Tieto ei ole lupa

Tämän skillin merkinnät kertovat, miten palvelun sivu käyttäytyy. Mikään merkintä ei
anna lupaa tehdä mitään. Kun aggountantilla on komento tehtävään, käytä sitä:
ALV-ilmoitus kulkee `file-vat`-komennolla ja työnantajan ilmoitus `file-wage`-komennolla,
kumpikin omalla hyväksynnällään. Selain ei ole aggountantin varakanava: kun ohjelma
estää lähetyksen, aiempi lähetys on selvittämättä tai hyväksyntä odottaa, selaimella
ei tehdä samaa ilmoitusta sen ohi.

Ilmoituksen, jonka käyttäjä antaa itse (käsin annettu ilmoitus tai korjaus, jota
aggountant ei tuota), lomakkeen voit täyttää hänen nimenomaisesta pyynnöstään.
Lähetys ja sen vahvistus ovat hänen tekojaan: pysähdy ennen lähetyspainiketta. Tulos
kirjataan aggountantin manuaalisen ilmoituksen polulla (`complete-manual`), jotta
kirjanpito tietää, mitä viranomaiselle on annettu.

Älä poimi evästeitä, tunnisteita tai istuntotunnuksia sivulta, äläkä tallenna
tunnisteen sisältäviä osoitteita. Merkinnöissä polut ovat malleja (`./Retrieve/…`).

## Mistä tieto löytyy

Avaa vain sen palvelun tiedosto, jonka sivulla olet:

- [omavero.md](omavero.md): OmaVero.
- [tulorekisteri.md](tulorekisteri.md): Tulorekisterin asiointipalvelu.
- [selain.md](selain.md): mitä selaimen on sallittava, jotta näitä palveluja voi
  käyttää. Se kertoo vaatimuksen, ei sitä, miten jokin kone on asetettu; koneen
  asetukset ovat käyttäjän.

Tämä skilli kertoo vain, miten sivuja käytetään. Mitä palvelusta haetaan ja miksi on
tehtäväskillissä (esimerkiksi `vero-fi` kertoo raportin 630 rajaukset, koska ne
ratkaisevat, onko tuonti oikein). Yrityksen oma käytäntö, kuten kuka kirjautuu tai
mikä pankkitili on käytössä, on `knowledge company`-tiedoissa.

## Merkinnän lukeminen

Jokainen merkintä on yksi asia, joka sivusta on tiedettävä: tason kaksi otsikko,
jossa on pysyvä tunniste ja nimi, neljä otsakeriviä ja selitys.

- **Palvelu ja sivu** kertoo, missä merkintä pätee.
- **Lähde** on `havaittu` (toimivaksi nähty), `dokumentoitu` (palvelun julkisessa
  ohjeessa, linkin kanssa) tai molemmat.
- **Havaittu** on päivä, jolloin käyttäytyminen nähtiin ensimmäisen kerran, ja
  **Vahvistettu** viimeisin päivä, jolloin joku näki sen yhä toimivan.
- **Rajapinta** nimeää aggountantin komennon tai viranomaisen rajapinnan, joka tekee
  saman tehtävän, tai sanoo, ettei sellaista ole. Jos komento on, käytä sitä, ellei
  käyttäjä nimenomaan halua antaa ilmoitusta itse.

Tunniste (esimerkiksi `omavero-kirje-lataus`) pysyy, vaikka teksti muuttuu. Viittaa
siihen, kun kerrot käyttäjälle, minkä ohjeen mukaan toimit tai mikä ohje ei enää
pitänyt paikkaansa.

## Tuoreus

Merkintä, joka on vahvistettu viimeisen 180 päivän aikana, kuvaa odotettavan
käyttäytymisen. Vanhempi merkintä on vihje: kokeile sitä, varaudu siihen, että sivu on
muuttunut, ja kerro käyttäjälle, että toimit vanhan ohjeen varassa.

Kun sivu käyttäytyy toisin kuin merkintä kertoo tai merkintää ei ole, kerro
käyttäjälle, mitä havaitsit ja minkä merkinnän tunnisteeseen se liittyy. Älä
paikkaa eroa arvaamalla toimintoa, jolla on oikeusvaikutuksia.
