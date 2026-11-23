# Päivitä aggountant

Kun päivität, asennat uuden version ohjelmasta ja skillit uudelleen, ja
agentti päivittää yrityskodin perustiedot. Komennot ja tiedostomuodot voivat
vielä muuttua versioiden välillä, joten päivitä silloin, kun mikään ajo ei ole
kesken.

## Ennen kuin aloitat

- Kuukauden sulku tai ilmoitus ei ole kesken. Pyydä agenttia tarkistamaan
  ajojen tila.
- Olet [varmuuskopioinut kodin](back-up-and-restore.md) juuri ennen
  päivitystä. Uusi versio voi muuttaa tositetietokannan (`fiscal.sqlite`)
  muotoa, kun se avaa kodin ensimmäisen kerran, eikä vanhempi versio sen
  jälkeen enää avaa tietokantaa. Varmuuskopio on silloin ainoa tie takaisin
  vanhempaan versioon. Projektin
  [muutosloki](https://github.com/aggountant/aggountant/blob/main/CHANGELOG.md)
  kertoo versio kerrallaan, tapahtuuko näin ja mitkä komennot muuttuivat.

## Näin teet sen

1. **Pyydä agenttia päivittämään.**

   > Päivitä aggountant uusimpaan versioon ja asenna skillit uudelleen.

2. **Agentti asentaa uuden version** samalla tavalla kuin
   [ensimmäisellä kerralla](../start/install.md). Asennusohjelma hakee
   GitHubista uusimman julkaisun. Jos koneellesi ei ole valmista pakettia,
   agentti kääntää uuden version lähdekoodista.

3. **Agentti asentaa skillit uudelleen.** Skillit on kirjoitettu saman
   version komentoja varten, joten ne asennetaan jokaisen päivityksen
   yhteydessä. Vanhat skillit ohjaisivat agenttia käyttämään komentoja, joita
   ei ehkä enää ole.

4. **Käynnistä agentti uudelleen**, jotta se lukee uudet skillit.

5. **Agentti päivittää kodin perustiedot.** Uusi versio voi tarvita
   perustietoja, joita vanha versio ei kotia perustaessaan kirjoittanut:
   tilikartan kiinteitä kopioita, tilien ALV-rooleja ja maksutilien roolit eli
   tiedon siitä, minkä pankki- ja verotilien kautta ALV maksetaan ja
   palautetaan. Ilman maksutilien rooleja koti aukeaa, mutta ALV-ilmoituksen
   esikatselu pyytää tarkistamaan jokaisen ALV:n maksun.

   Agentti ajaa ensin kuivaharjoituksen, joka vain luettelee puuttuvat
   tiedostot, ja sen jälkeen varsinaisen päivityksen. Päivitys lisää
   puuttuvat tiedostot. Jos ALV-roolien tiedostoa ei ole muokattu ja uusi
   versio tuo siihen käännetyn verovelvollisuuden tilipareja, päivitys
   täydentää sen. Vanhat roolit säilyvät, joten aiemmin sinetöidyt tositteet
   tarkistuvat kuten ennenkin. Käytössä olevaa tilikarttaa päivitys ei
   vaihda. Muun tiedoston, jonka sisältö on kodissa eri kuin ohjelman mukana
   tulevassa, se jättää ennalleen ja kertoo siitä. Näin käy esimerkiksi, kun
   yritys on tuonut oman tilikarttansa.

6. **Vie muutokset versionhallintaan.** Lisätyt ja päivitetyt tiedostot
   näkyvät `git status` -tulosteessa, ja versionhallintaan vietyinä ne
   kertovat myöhemmin, mitä päivitys muutti.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant run list
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill install
aggountant home upgrade --dry-run
aggountant home upgrade --apply
aggountant run migrate --run-id cp-… --confirm
```

`curl`-komento asentaa valmiin paketin. `cargo install` kääntää ohjelman
lähdekoodista, ja sitä tarvitaan vain, kun valmista pakettia ei ole.

`home upgrade --dry-run` luettelee, mitä päivitys lisäisi, eikä kirjoita
mitään.

`home upgrade --apply` lisää puuttuvat tiedostot hakemistoon
`masterdata/reference/`. Maksutilien roolit ovat tiedostossa
`settlement-account-roles.json`. Olemassa olevista tiedostoista päivitys
muuttaa vain ALV-roolit (`vat-account-roles.json`), ja vain silloin, kun
tiedosto on tavu tavulta sellainen kuin aiempi aggountantin versio sen
kirjoitti ja uusi versio lisää siihen käännetyn verovelvollisuuden tilipareja,
esimerkiksi EU-tavaraostoille (`eu-goods`). Tulosteessa tällä tiedostolla on `"action": "update"`. Ennalleen jätetyt
tiedostot ovat kohdassa `skipped`. Toinen ajo ei tee mitään.

`run migrate` tarvitaan vain, jos ajo jäi kesken ennen päivitystä.
</details>

## Valmis, kun

- `aggountant --version` näyttää uuden version.
- `aggountant home upgrade --dry-run` ei enää lisäisi mitään, ja
  `masterdata_opens` on `true`.
- `aggountant voucher verify` vastaa `chain ok`.

## Rajoitukset

- Päivitys ei korjaa eikä korvaa tiedostoa, jonka se jätti ennalleen. Jos
  ohjelman mukana ei tule ALV-rooleja tai maksutilien rooleja kodin
  tilikartalle, päivitys ei myöskään voi lisätä niitä (`unavailable`). Älä
  kopioi tiedostoja käsin, vaan pyydä apua projektin
  [issue-seurannasta](https://github.com/aggountant/aggountant/issues).

## Jos jokin menee pieleen

- Jos jokin ajo oli kuitenkin kesken ja sen jatkaminen antaa koodin
  `WORKFLOW_VERSION_MISMATCH`, ajon määritys on muuttunut versioiden välillä.
  Agentti voi siirtää ajon uuteen versioon. Siirto muuttaa ajon versiota
  pysyvästi, joten agentti pyytää siihen luvan.

## Katso myös

- [Asenna aggountant ja skillit](../start/install.md)
- [Kun jokin menee vikaan](../troubleshooting.md)
