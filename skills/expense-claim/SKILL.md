---
name: expense-claim
description: >-
  Kokoa kululasku: henkilön työmatkat, ajot ja kuitit kululaskun luonnokseksi,
  avoimet kysymykset käyttäjälle ja luonnoksen näyttäminen. Myös yksittäinen
  matka tai kuitti, jonka yhtiö korvaa henkilölle.
cli_version: "0.3.0"
schema_version: 1
---

# Kululasku

Lue ensin [SOUL.md](../aggountant-init/SOUL.md), joka kuvaa roolisi kirjanpitoavustajana
ja `aggountant`-työvälineesi (`aggountant skill print aggountant-init --file SOUL.md`).

Kululasku kootaan ohjelman omistamaksi luonnokseksi: `claim trip add` ja `claim line
add` aloittavat sen (`--new-for <henkilö>`) tai jatkavat sitä (`--claim <tunnus>`), ja
jokainen kirjoitus vastaa sillä, mitä luonnoksesta vielä puuttuu. Luonnos voi olla yksi
matka tai koko kuukausi; tee sellainen kuin käyttäjä haluaa. Syötteet näkyvät
komentojen `--help`-tulosteessa.

## Ohjelma laskee, sinä kysyt

Älä laske päivärahoja, kilometrikorvauksia tai puolituksia itse äläkä kirjoita summia
luonnokseen. Ohjelma laskee ne matkasta, ja yrityksen maksama pienempi summa on ainoa
summa, jonka luonnos ottaa. Sinun työsi on saada matkasta oikeat tiedot: väärä vastaus
tekee korvauksesta väärän, ja osakkaalle maksettu perusteeton kilometrikorvaus on
peiteltyä osinkoa.

Avoimet kysymykset (`open_questions`) ovat vastauksia, joita käyttäjä ei ole antanut.
Kysy ne käyttäjältä, älä arvaa. Tyhjä ei tarkoita "ei mitään": ilmaiset ateriat ja
yrityksen maksamat ateriat kirjataan jokaiselle matkavuorokaudelle, myös silloin kun
niitä oli nolla, koska puuttuva ateria tekisi päivärahasta liian suuren. Käyttäjä
vakuuttaa ateriat oikeiksi hyväksyessään kululaskun, joten kysy ne häneltä äläkä
päättele niitä esimerkiksi hotellivarauksesta.

Kieltäytymiset (`refusals`) ovat asioita, joita ohjelma ei hyväksy sellaisenaan. Kerro
käyttäjälle syy omin sanoin; viesti kertoo, mikä asian ratkaisee. Tarkistukset
(`checks`) eivät estä mitään, mutta käy ne käyttäjän kanssa läpi: ne näkyvät
kululaskussa, ja juuri niistä tarkastaja kysyy.

## Kun tietoa puuttuu

- **Varsinainen työpaikka.** Jos matka saa kysymyksen `REGULAR_WORKPLACE_UNKNOWN` tai
  `CLAIMANT_RELATIONSHIP_UNKNOWN`, kirjaa tieto ensin `claimant`-komennoilla käyttäjän
  vahvistamana, kuten oppaan sivu "Kirjaa varsinainen työpaikka" kertoo. Kysy sitä vain
  kululaskun tekijältä, ei rutiinina muilta.
- **Ajot kalenterista.** Voit ehdottaa ajoja kalenterista tai sähköpostista, mutta ajo
  kirjataan vasta, kun käyttäjä vahvistaa sen. Silloin lähteeksi tulee
  `confirmed_reconstruction`. Vahvistamaton ehdotus ei ole fakta, eikä sitä tallenneta.
- **Kuitit.** Tallenna kuitti ensin `artifact attach --file` -komennolla ja anna
  luonnokseen sen tunniste. Jos kuittia ei ole, kirjaa syy; älä keksi kuittia.
- **Myöhään kirjattu matka.** Matka, joka kirjataan vasta seuraavan kuukauden jälkeen,
  merkitään jälkikäteen kirjatuksi. Se sallitaan, mutta kerro käyttäjälle, että merkintä
  näkyy, ja pyydä mahdollinen todiste (lippu, kokouskutsu) liitteeksi.

Yhtiön omat ostot eivät kuulu kululaskuun. Kululaskun kuittirivi on kulu, jonka henkilö
maksoi itse ja yhtiö korvaa hänelle.

## Luonnoksen näyttäminen ja hylkääminen

`claim show --claim <tunnus> --format markdown` tulostaa kululaskun luettavana. Näytä
se käyttäjälle, kun hän pyytää tai kun luonnos on valmis (`ready_for_approval`).
Hylkää luonnos `claim discard` -komennolla vain käyttäjän pyynnöstä ja hänen
kertomallaan syyllä.

Kululaskua ei vielä voi hyväksyä eikä kirjata aggountantissa. Valmiin luonnoksen
korvaukset kirjataan käsin erikoiskirjauksena ja ilmoitetaan Tulorekisteriin, kuten
oppaan sivu "Laske matkan verovapaat korvaukset" kertoo; luonnoksen laskelma on sama
kuin `claim compute` -komennon. Matkaennakot ovat `capture-receipt`-skillin
[matkaennakko-ohjeessa](../capture-receipt/matkaennakot.md).
