# Miten ALV-käsittely päätetään

Agentti ehdottaa jokaiselle ostolle ALV-käsittelyn. Kun kuitista ei selviä,
miten osto verotetaan, se kysyy sinulta, vaikka vastaus tuntuisi ilmeiseltä.
ALV-ilmoituksen luvut lasketaan tositteisiin tallennetuista käsittelyistä.

## ALV-käsittely on aina nimetty valinta

Osto kirjataan jollakin näistä käsittelyistä:

- Kotimaisessa ostossa kuitin summa sisältää ALV:n, ja vähennettävä vero
  lasketaan siitä.
- Käännetyssä verovelvollisuudessa ostaja tilittää veron, esimerkiksi kun
  palvelu ostetaan toisesta EU-maasta. Lajeja on kolme: EU-palvelut,
  EU-tavarat ja EU:n ulkopuoliset palvelut.
- Veroton osto kirjataan, kun verottomuus on varmistettu kuitista tai
  yrityksen käytännöistä.

Jos ratkaiseva tieto puuttuu, agentti merkitsee kuitin selvitettäväksi ja
kysyy sinulta, mitä pitää selvittää. Se ei arvaa, koska verottomaksi arvattu
osto on virhe, jota kukaan ei huomaa ennen ALV-ilmoitusta.

Osittaista vähennysoikeutta ja maahantuontia
[kuittireitti ei käsittele](../reference/status.md#kuitit-ja-kirjaukset).
Agentti kertoo, jos osto on sellainen.

## Kanta valitaan kuitin päivästä

ALV-kanta riippuu tuotteesta ja siitä, milloin kauppa tehtiin. Siksi kanta
valitaan kuitin tai laskun päivästä, ei maksupäivästä eikä pankin
kirjauspäivästä. Päivällä on väliä, koska kantoja on muutettu viime vuosina:
yleinen kanta on ollut 25,5 % 1.9.2024 alkaen, ja alennettu 14 %:n kanta laski
13,5 %:iin 1.1.2026.

Agentti kertoo ohjelmalle käsittelyn, kantaluokan ja päivän, ja ohjelma
valitsee prosentin verokantataulukostaan. Kuittireitillä ja tiliotteen
ostoissa agentti ei kirjoita ALV-rivejä käsin.

Myynnin nollakantaluokat eivät sovi ostolle. Ohjelma torjuu ne ja kertoo
sopivan käsittelyn (`VAT_TREATMENT_CLASS_MISMATCH`). Ainoa poikkeus on veroton
tavaroiden yhteisöhankinta, josta kerrotaan seuraavassa osiossa.

## Ostot toisesta EU-maasta

Käännetyssä verovelvollisuudessa kantaluokka on se Suomen kanta, jota tavaraan
tai palveluun sovelletaan. Se, että osto tuli toisesta EU-maasta, kerrotaan
erikseen. Verollinen tavaroiden yhteisöhankinta kirjataan siis Suomen
kantaluokalla (esim. `domestic-standard`) silloinkin, kun myyjän laskulla ei
ole ALV:tä. Laskulta ALV puuttuu, koska ostaja tilittää sen, joten puuttuminen
ei tee ostosta verotonta.

Toisesta EU-maasta ostettu palvelu, jonka myynnin arvonlisäverolaki säätää
verottomaksi, esimerkiksi vakuutus- tai rahoituspalvelu, kirjataan verottomana
ostona (`{"type":"no-vat"}`) eikä käännettynä verovelvollisuutena. Siitä ei
tule Suomen ALV:tä eikä vähennettävää veroa, eikä se näy ALV-ilmoituksessa.
Verohallinnon täyttöohjeen
[Näin ilmoitat arvonlisäveron](https://www.vero.fi/yritykset-ja-yhteisot/verot-ja-maksut/arvonlisaverotus/ilmoitus-ja-maksuohjeet/)
mukaan kohdassa "Palveluostot muista EU-maista" ei ilmoiteta yleissäännön
mukaisia palveluja, joiden myynti on veroton. Syventävän ohjeen
[Palvelujen ulkomaankaupan arvonlisäverotus](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48679/)
luvun 9 mukaan kohtaan kuuluvat palvelut, joista ostaja on verovelvollinen
Suomessa.

Tavaroissa sääntö on toinen. Veroton tavaroiden yhteisöhankinta ilmoitetaan
ALV-ilmoituksella, mutta siitä ei suoriteta eikä vähennetä veroa. Hankinta on
veroton esimerkiksi silloin, kun tavaran maahantuonti olisi Suomessa veroton
tai tavara viedään arvonlisäverovarastoon. Se kirjataan käännettynä
verovelvollisuutena nollakantaluokalla:

```json
{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}
```

Summa on veroton hankintahinta, eikä ALV-tilejä kirjata. Ilmoituksessa arvo
näkyy ainoastaan kohdassa "Tavaraostot muista EU-maista", vero tavaraostoista
on nolla eikä vähennettävä vero muutu (syventävän ohjeen
[Arvonlisäverotus EU-tavarakaupassa](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48691/arvonlisaverotus-eu-tavarakaupassa4/)
luvut 6.6 ja 12). Tämä on ainoa ostokäsittely, joka hyväksyy
nollakantaluokan. Osittaista vähennystä siihen ei voi liittää, koska
vähennettävää veroa ei ole.

Verotonta käsittelyä käytetään vain, kun verottomuus on varmistettu, koska
ohjelma ei erota sitä verollisesta hankinnasta, jonka kantaluokaksi on
vahingossa annettu `zero-rated`.

## Käsittely päätetään sinetöidessä

ALV-käsittely tallentuu tositteeseen, kun tosite sinetöidään, ja ALV-ilmoitus
lasketaan näistä merkinnöistä. Ohjelma ei päättele käsittelyä jälkikäteen
tilinumerosta.

ALV-vienti, jolla ei ole ALV-merkintää, ei siksi näy ilmoituksessa, vaikka se
olisi ALV-tilillä. Näin voi käydä varsinkin
[erikoiskirjauksissa](../tasks/record-other-entries.md), kuten myyntilaskuissa,
joissa agentti kokoaa ALV-viennit itse. Ohjelma huomaa silloin ristiriidan:
ALV-ilmoituksen esikatselu vertaa ALV-tilien liikettä ilmoituksen lukuihin, ja
suora ilmoitus kieltäytyy, jos ne eivät täsmää.

Väärä käsittely korjataan korjaustositteella, ja korjaus vaikuttaa sen kauden
ilmoitukseen, jolle se kirjataan.

## Myynnin ALV

Tiliotteen tuonti ja kuittireitti purkavat ALV:n vain ostoista. Myynti, johon
sisältyy ALV, kirjataan erikoiskirjauksena, esimerkiksi myyntilaskun
perusteella, ja asiakkaan maksu kirjataan tiliotteelta myyntisaamisia vastaan.

## Mitä tämä tarkoittaa sinulle

- Kun hyväksyt oston, tarkista summan lisäksi käsittely ja laskun päivä.
- Kun agentti kysyy ALV:stä, selvitä vastaus kuitista tai myyjältä.
- Jos osto on epätavallinen (ulkomainen, osittain yksityinen, maahantuonti),
  kysy kirjanpitäjältäsi.

## Katso myös

- [Kirjaa kuitit ja ostolaskut](../tasks/record-receipts.md)
- [Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md)
- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
