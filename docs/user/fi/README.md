# aggountant — käyttäjän opas

Tämä opas kertoo, miten pidät yrityksesi kirjanpitoa aggountantilla ja
tekoälyagentilla: mitä teet itse, mitä agentti tekee puolestasi ja mitä sinä
hyväksyt ennen kuin mikään tallentuu pysyvästi tai lähtee viranomaiselle.

Opas on kirjoitettu yrityksen omistajalle tai kirjanpitäjälle, ei ohjelmoijalle.
Komentoja on mukana, koska agentti ajaa niitä ja sinun on hyvä tunnistaa, mitä
se tekee. Kaikki esimerkit koskevat kuvitteellista Esimerkki Oy:tä, ja luvut on
keksitty. Oppaan englanninkielinen versio on hakemistossa
[`../en/`](../en/README.md).

## Sisällys

1. [Mikä aggountant on ja kenelle](01-mika-aggountant-on.md)
2. [Asennus ja yrityskodin perustaminen](02-asennus-ja-yrityskoti.md)
3. [Agentti ja skillit](03-agentti-ja-skillit.md)
4. [Kuukausikierto: pankki, kuitit ja kauden sulku](04-kuukausikierto.md)
5. [ALV ja Vero.fi](05-alv-ja-vero-fi.md)
6. [Palkat ja Tulorekisteri](06-palkat-ja-tulorekisteri.md)
7. [Tilinpäätös, raportit ja tilikauden tila](07-tilinpaatos-ja-raportit.md)
8. [Virhetilanteet ja palautuminen](08-virhetilanteet.md)
9. [Sanasto](09-sanasto.md)

## Mitä on valmista ja mitä ei

aggountant on varhaisessa vaiheessa (versio 0.1). Näillä osilla pidetään jo
oikeaa kirjanpitoa:

- tositteiden tallentaminen ja liitteiden arkistointi
- pankkitiliotteen tuonti (camt.053 ja camt.052)
- tavallisten ostokuittien kirjaaminen ALV-käsittelyineen
- kuukauden sulku
- kuukausiraportit
- ALV-ilmoitus suoraan Verohallinnolle tai käsin OmaVerossa
- verovapaiden matkakorvausten ilmoitus Tulorekisteriin.

Palkka-ajo, kuittien automaattinen haku ja tilinpäätös ovat ohjelmassa, mutta
niitä ei vielä voi käyttää oikeaan kirjanpitoon. Kyseiset luvut kertovat syyn ja
sen, mitä teet niiden sijaan.
