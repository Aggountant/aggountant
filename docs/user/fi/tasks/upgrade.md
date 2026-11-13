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

5. **Agentti päivittää kodin perustiedot.** Uusi versio voi tarvita
   perustietoja, joita vanha versio ei kotia perustaessaan kirjoittanut:
   tilikartan kiinteitä kopioita ja tilien ALV-rooleja. Agentti ajaa ensin
   kuivaharjoituksen, joka vain luettelee puuttuvat tiedostot, ja sen jälkeen
   varsinaisen päivityksen. Päivitys lisää vain puuttuvia tiedostoja
   hakemistoon `masterdata/reference/`: se ei vaihda käytössä olevaa
   tilikarttaa eikä muuta olemassa olevaa tiedostoa, joten aiemmin
   sinetöidyt tositteet tarkistuvat kuten ennenkin. Jos jokin tiedosto on
   kodissa eri sisältöisenä kuin ohjelman mukana tuleva, päivitys jättää sen
   ennalleen ja kertoo siitä (`skipped`). Toinen ajo ei tee mitään.

   Lisätyt tiedostot näkyvät `git status` -tulosteessa. Vie ne
   versionhallintaan; se on kirjanpito siitä, mitä päivitys lisäsi.

6. **Jos jokin ajo oli kuitenkin kesken** ja sen jatkaminen antaa koodin
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
aggountant home upgrade --dry-run
aggountant home upgrade --apply
aggountant run migrate --run-id cp-… --confirm
```
</details>

## Valmis, kun

- `aggountant --version` näyttää uuden version.
- `aggountant home upgrade --dry-run` ei enää lisäisi mitään, ja
  `masterdata_opens` on `true`.
- `aggountant voucher verify` vastaa `chain ok`.

## Rajoitukset

- Päivitys ei korjaa eikä korvaa tiedostoa, joka on kodissa eri sisältöisenä
  kuin ohjelman mukana tuleva. Jos yrityksellä on oma tilikartta, jolle
  ohjelman mukana ei tule ALV-rooleja, päivitys ei voi lisätä niitä
  (`unavailable`). Älä kopioi tiedostoja käsin, vaan pyydä apua projektin
  [issue-seurannasta](https://github.com/aggountant/aggountant/issues).

## Katso myös

- [Asenna aggountant ja skillit](../start/install.md)
- [Kun jokin menee vikaan](../troubleshooting.md)
