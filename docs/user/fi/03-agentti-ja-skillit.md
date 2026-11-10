# 3. Agentti ja skillit

aggountant ei sisällä tekoälyä. Tekoäly on agentissa, jota käytät, esimerkiksi
Claude Codessa. Jotta agentti osaa toimia kirjanpitäjänä, aggountantin mukana
tulee **skillejä**: suomenkielisiä toimintaohjeita, jotka agentti lukee ennen kuin
se tarttuu tehtävään.

## Skillien asennus

Ohjelmaan pakatut skillit näet komennolla:

```sh
aggountant skill list
```

Skillejä on kolme:

| Skilli | Mihin agentti käyttää sitä |
|---|---|
| `aggountant-init` | yrityskodin käyttöönotto tai keskeneräisen käyttöönoton jatkaminen |
| `capture-receipt` | kuittien, tositteiden ja kuukauden tapahtumien käsittely |
| `vero-fi` | Vero.fi-käyttöönotto, ALV-ilmoitus ja Tulorekisterin matkakorvaukset |

Kaikki asennetaan kerralla:

```sh
aggountant skill install
```

Oletuksena skillit menevät Claude Coden skillihakemistoon `~/.claude/skills/`.
Toista agenttia varten anna sen skillihakemisto `--target`-valitsimella. Asenna
skillit uudelleen aina, kun päivität aggountantin, koska ohjeet on kirjoitettu
saman version komentoja varten. Komento `aggountant skill print <nimi>` näyttää
skillin sisällön mitään asentamatta.

Kaikilla skilleillä on yhteinen roolikuvaus (`SOUL.md`). Sen mukaan agentti toimii
kuin kirjanpitoa tunteva työtoveri: se ratkaisee itse sen, mihin lähteet ja
yrityksen aiempi kirjanpito riittävät, ja kysyy sinulta vain sen, mitä se ei voi
päätellä.

## Mitä agentti tekee itse

Agentti ajaa ilman lupaa kaiken, mikä vain lukee tai esikatselee eikä jätä
pysyviä jälkiä. Tällaisia ovat esimerkiksi:

- tilikartan, aiempien tositteiden ja yrityksen faktojen lukeminen
- tositeluonnoksen esikatselu ja tarkistus
- ALV-ilmoituksen lukujen esikatselu sinetöidystä kaudesta
- Vero.fi-valmiuden tarkistus ilman verkkoyhteyttä
- raporttien tekeminen.

Agentti myös kokoaa kysymyksensä yhteen. Kuukauden aineistossa voi olla
kymmeniä tapahtumia, eikä agentin kuulu keskeyttää sinua jokaisen kuitin kohdalla.
Se kysyy, kun vaihtoehdot eroavat tavalla, jolla on sinulle merkitystä (tili,
ALV-käsittely, kuuluuko kulu yritykselle), eikä se voi päätellä valintaasi.

## Mitä sinä hyväksyt

Pysyvät ja oikeusvaikutteiset teot tehdään vasta, kun olet hyväksynyt ne
nimenomaisesti:

| Teko | Miksi hyväksyntä vaaditaan |
|---|---|
| tositteen sinetöinti | sinetöity tosite on pysyvä ja korjataan vain korjaustositteella |
| kauden sulku | suljetulle kaudelle ei voi enää kirjata |
| ALV-ilmoituksen lähetys | ilmoitus on oikeusvaikutteinen eikä sitä voi perua |
| Tulorekisteri-ilmoituksen lähetys | sama kuin ALV-ilmoituksessa |
| Vero.fi-tuotantoon siirtyminen ja sitä koskevat vakuutukset | ohjelma luottaa vakuutuksiisi eikä voi tarkistaa niitä |

Hyväksyntä on sidottu siihen, mitä sinulle näytettiin. Kuitin kirjauksessa
agentti näyttää esikatselun, ja ohjelma laskee siitä tunnisteen. Jos lähdetieto,
ehdotus tai tilikartta muuttuu ennen sinetöintiä, ohjelma kieltäytyy, ja agentin
on näytettävä uusi esikatselu. Et siis voi hyväksyä yhtä ja saada toista.

Hyvä hyväksyntäpyyntö näyttää jokaisen tositteen olennaiset tiedot: päivän,
summan, vastapuolen, tilin ja ALV-käsittelyn. Pelkkä "kirjaanko 14 tositetta?"
ei riitä. Pyydä nähtäväksi rivit, jos agentti ei näytä niitä.

## Yrityksen tietovarasto

Agentti tallentaa työssä opitut yrityksen faktat ja käytännöt kodin
`knowledge/company/`-hakemistoon, esimerkiksi tilikauden, tavan kirjata tietyn
toimittajan laskut tai sen, että puhelinliittymä on kokonaan yrityksen käytössä.
Jokaisella tiedolla on lähde ja historia, joten näet myöhemmin, mistä tieto tuli
ja milloin se muuttui.

```sh
aggountant knowledge company list
aggountant knowledge company search --query "puhelin"
aggountant knowledge company history --id "tunniste"
```

Vanhentunutta tietoa ei poisteta. Se merkitään pois käytöstä
(`knowledge company retire`), jolloin aiempi tieto ja muutoksen syy säilyvät.

## Kodin AGENTS.md: mistä aineisto tulee

aggountant ei määrää, mistä kuitit tulevat tai minne käsitelty aineisto viedään.
Kirjoita se agentin kanssa kodin juureen tiedostoon `AGENTS.md` omin sanoin.
Tiedostossa kannattaa kertoa:

- minne kuitit ja laskut saapuvat (sähköposti, pilvikansio, paikallinen kansio,
  vastuuhenkilö)
- missä kulu- ja matkalaskujen luonnokset ovat
- missä ajopäiväkirja pidetään
- minne käsitelty alkuperäisaineisto arkistoidaan ja milloin sen saa siirtää
  pois lähteestä
- mitkä kodin kansiot kuuluvat gitiin ja mitkä eivät.

Ilman tätä tiedostoa agentti joutuu kysymään joka kerta tai arvaamaan.
`aggountant-init` auttaa kirjoittamaan tiedoston, ja `capture-receipt` lukee sen
ennen kuin se etsii, kopioi tai siirtää mitään.

**Salasanoja, tunnuksia ja avaimia ei kirjoiteta `AGENTS.md`-tiedostoon, yrityksen
tietovarastoon eikä keskusteluun agentin kanssa.** Koti on git-versioitua
tekstiä, ja keskustelut tallentuvat usein lokeihin.
