# Miten ALV-käsittely päätetään

Agentti ehdottaa jokaiselle ostolle ALV-käsittelyn, ja joskus se kysyy sinulta
jotain, mikä tuntuu ilmeiseltä. Tämä sivu kertoo, miten ALV päätetään, miksi
agentti ei arvaa ja mistä ALV-ilmoituksen luvut tulevat.

## ALV-käsittely on aina nimetty valinta

Jokaiselle ostolle valitaan nimetty käsittely:

- **kotimainen osto**: kuitin summa sisältää ALV:n, ja vähennettävä vero
  lasketaan siitä
- **käännetty verovelvollisuus**: ostaja tilittää veron, esimerkiksi ostettaessa
  palveluja toisesta EU-maasta. Lajeja on kolme: EU-palvelut, EU-tavarat ja
  EU:n ulkopuoliset palvelut.
- **veroton osto**, kun verottomuus on varmistettu kuitista tai yrityksen
  käytännöistä.

Jos ratkaiseva tieto puuttuu, agentti merkitsee kuitin selvitettäväksi eikä
arvaa. Silloin saat kysymyksen, joka kertoo, mitä pitää selvittää.
Tuntemattoman ALV:n kirjaaminen verottomaksi olisi hiljaa väärää kirjanpitoa,
jota kukaan ei huomaisi ennen ilmoitusta.

Osittaista vähennysoikeutta ja maahantuontia kuittireitti ei käsittele. Agentti
kertoo, jos osto on sellainen.

## Kanta valitaan kuitin päivästä

ALV-kanta riippuu tuotteesta ja siitä, milloin kauppa tehtiin. Siksi kanta
valitaan kuitin tai laskun päivästä, ei maksupäivästä eikä pankin
kirjauspäivästä. Kantoja on muutettu viime vuosina:

- yleinen kanta on 25,5 % 1.9.2024 alkaen
- alennettu 14 %:n kanta laski 13,5 %:iin 1.1.2026.

Ohjelma valitsee prosentin itse verokantataulukostaan, kun agentti on kertonut
käsittelyn, kantaluokan ja päivän. Agentti ei kirjoita ALV-rivejä käsin
kuittireitillä eikä tiliotteen ostoissa.

## Käsittely päätetään sinetöidessä

ALV-käsittely tallentuu tositteeseen sinetöinnin hetkellä, ja ALV-ilmoitus
lasketaan näistä tallennetuista merkinnöistä. Ohjelma ei päättele
ALV-käsittelyä jälkikäteen tilinumeron perusteella. Siksi:

- ALV-vienti ilman ALV-merkintää ei näy ilmoituksessa, vaikka se olisi
  ALV-tilillä. Tämä koskee erityisesti erikoiskirjauksia, kuten myyntilaskuja,
  joissa agentti kokoaa ALV-viennit itse
  ([Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](../tasks/record-other-entries.md)).
  Ohjelma ei silloinkaan päättele käsittelyä tilistä, mutta se huomaa
  ristiriidan: ALV-ilmoituksen esikatselu vertaa ALV-tilien liikettä
  ilmoituksen lukuihin, ja suora ilmoitus kieltäytyy, jos ne eivät täsmää.
- Väärä käsittely korjataan korjaustositteella, ja korjaus vaikuttaa sen kauden
  ilmoitukseen, jolle se kirjataan.

## Myynnin ALV

Tiliotteen tuonti ja kuittireitti purkavat ALV:n vain ostoista. Myynti, johon
sisältyy ALV, kirjataan erikoiskirjauksena, esimerkiksi myyntilaskun
perusteella, ja asiakkaan maksu kirjataan tiliotteelta myyntisaamisia vastaan.

## Mitä tämä tarkoittaa sinulle

- Kun hyväksyt oston, katso käsittely ja laskun päivä, älä vain summaa.
- Kun agentti kysyy ALV:stä, se ei tiedä vastausta. Selvitä se kuitista tai
  myyjältä.
- Jos osto on epätavallinen (ulkomainen, osittain yksityinen, maahantuonti),
  kysy kirjanpitäjältäsi.

## Katso myös

- [Kirjaa kuitit ja ostolaskut](../tasks/record-receipts.md)
- [Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md)
- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
