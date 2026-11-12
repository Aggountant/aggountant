# Päivitä aggountant

Uusi aggountant-versio asennetaan samalla tavalla kuin ensimmäinen, ja sen
jälkeen skillit asennetaan uudelleen. Komennot ja tiedostomuodot voivat vielä
muuttua versioiden välillä, joten tee päivitys silloin, kun mikään ajo ei ole
kesken.

## Ennen kuin aloitat

- Kuukauden sulku tai ilmoitus ei ole kesken. Pyydä agenttia tarkistamaan
  ajojen tila.
- Sinulla on tuore varmuuskopio kodista
  ([Varmuuskopioi ja palauta kirjanpito](back-up-and-restore.md)).

## Näin teet sen

1. **Pyydä agenttia päivittämään.**

   > Päivitä aggountant uusimpaan versioon ja asenna skillit uudelleen.

2. **Agentti asentaa uuden version** samalla komennolla kuin ensimmäisellä
   kerralla ([Asenna aggountant ja skillit](../start/install.md)).

3. **Agentti asentaa skillit uudelleen.** Tämä on tarpeen joka kerta, koska
   skillit on kirjoitettu saman version komentoja varten. Vanhat skillit
   ohjaisivat agenttia käyttämään komentoja, joita ei ehkä enää ole.

4. **Käynnistä agentti uudelleen**, jotta se lukee uudet skillit.

5. **Jos jokin ajo oli kuitenkin kesken** ja sen jatkaminen antaa koodin
   `WORKFLOW_VERSION_MISMATCH`, ajon määritys on muuttunut versioiden välillä.
   Agentti voi siirtää ajon uuteen versioon. Siirto muuttaa ajon versiota
   pysyvästi, joten agentti pyytää siihen luvan.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant run list
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill install
aggountant run migrate --run-id cp-… --confirm
```
</details>

## Valmis, kun

- `aggountant --version` näyttää uuden version.
- `aggountant voucher verify` vastaa `chain ok`.

## Rajoitukset

- Jos uusi versio odottaa kodista tietoa, jota vanha koti ei sisällä,
  erillistä päivityskomentoa ei vielä ole. Älä kopioi tiedostoja käsin, vaan
  pyydä apua projektin [issue-seurannasta](https://github.com/aggountant/aggountant/issues).

## Katso myös

- [Asenna aggountant ja skillit](../start/install.md)
- [Kun jokin menee vikaan](../troubleshooting.md)
