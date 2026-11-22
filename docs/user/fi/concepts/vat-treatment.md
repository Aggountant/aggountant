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

Myös käännetyssä verovelvollisuudessa kantaluokka on se Suomen kanta, jota
tavaraan tai palveluun sovelletaan; se, että osto tuli toisesta EU-maasta,
kerrotaan erikseen. Myynnin nollakantaluokat eivät sovi ostolle, ja ohjelma
torjuu ne ja kertoo sopivan käsittelyn (`VAT_TREATMENT_CLASS_MISMATCH`).
Ainoa poikkeus on alla kuvattu veroton tavaroiden yhteisöhankinta.

Toisesta EU-maasta ostettu palvelu, jonka myynnin arvonlisäverolaki säätää
verottomaksi, esimerkiksi vakuutus- tai rahoituspalvelu, kirjataan verottomana
ostona (`{"type":"no-vat"}`), ei käännettynä verovelvollisuutena. Siitä ei tule
Suomen ALV:tä eikä vähennettävää veroa, eikä se näy ALV-ilmoituksessa:
Verohallinnon täyttöohjeen mukaan kohdassa "Palveluostot muista EU-maista" ei
ilmoiteta yleissäännön mukaisia palveluja, joiden myynti on veroton
([Näin ilmoitat arvonlisäveron](https://www.vero.fi/yritykset-ja-yhteisot/verot-ja-maksut/arvonlisaverotus/ilmoitus-ja-maksuohjeet/)),
ja syventävän ohjeen
[Palvelujen ulkomaankaupan arvonlisäverotus](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48679/)
luku 9 kuvaa kohdan palveluiksi, joista ostaja on verovelvollinen Suomessa.
Pelkkä se, ettei myyjän laskulla ole ALV:tä, ei tee ostosta verotonta:
käännetyssä verovelvollisuudessa laskulla ei ole ALV:tä, koska ostaja tilittää
sen.

Tavaroissa sääntö on toinen: veroton tavaroiden yhteisöhankinta ilmoitetaan
ALV-ilmoituksella, mutta siitä ei suoriteta eikä vähennetä veroa. Sen arvo
ilmoitetaan ainoastaan kohdassa "Tavaraostot muista EU-maista" (syventävä ohje
[Arvonlisäverotus EU-tavarakaupassa](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48691/arvonlisaverotus-eu-tavarakaupassa4/),
luvut 6.6 ja 12). Hankinta on veroton esimerkiksi silloin, kun tavaran
maahantuonti olisi Suomessa veroton tai tavara viedään
arvonlisäverovarastoon. Se kirjataan käännettynä verovelvollisuutena
nollakantaluokalla:
`{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}`.
Summa on veroton hankintahinta, eikä ALV-tilejä kirjata. Ilmoituksessa arvo
näkyy kohdassa "Tavaraostot muista EU-maista", vero tavaraostoista on nolla
eikä vähennettävä vero muutu. Tämä on ainoa ostokäsittely, joka hyväksyy
nollakantaluokan. Osittaista vähennystä siihen ei voi liittää, koska
vähennettävää veroa ei ole. Verollinen tavaroiden yhteisöhankinta kirjataan
Suomen kantaluokalla (esim. `domestic-standard`) silloinkin, kun myyjän
laskulla ei ole ALV:tä. Siksi verotonta käsittelyä käytetään vain, kun
verottomuus on varmistettu: ohjelma ei erota sitä verollisesta hankinnasta,
jonka kantaluokaksi on vahingossa annettu `zero-rated`.

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
