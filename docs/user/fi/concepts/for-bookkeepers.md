# Kirjanpitäjälle

Tämä sivu on kirjanpitäjälle tai tilitoimistolle, joka hoitaa kirjanpitoa
omistajan puolesta aggountantilla tai saa siitä aineiston tilinpäätöstä
varten. Se kertoo, miten aggountant vastaa kirjanpitolain vaatimuksiin, mitä
aineistoa siitä saa ja missä sen rajat ovat.

## Tositteet ja kirjausketju

- **Numerointi.** Tositteet numeroidaan aukottomasti yhteen juoksevaan
  sarjaan sinetöintijärjestyksessä.
- **Pysyvyys.** Sinetöidyn tositteen sisällöstä lasketaan tunniste, joka
  ketjutetaan edelliseen. Ketjun tarkistus paljastaa, jos tositetta on
  muutettu tai se on poistettu jälkikäteen
  ([Sinetöinti ja hyväksyntä](sealing-and-approval.md)).
- **Korjaukset.** Sinetöityä tositetta ei muokata. Virhe korjataan
  korjaustositteella, joka viittaa alkuperäiseen, ja molemmat jäävät
  kirjanpitoon.
- **Päivät.** Tositteella on erikseen tositepäivä (kuitin tai laskun päivä) ja
  kirjauspäivä. Tiliotteelta kirjatun tapahtuman kirjauspäivä on pankin
  kirjauspäivä, ja ALV-kanta valitaan laskun päivästä.
- **Kaudet.** Kuukausi suljetaan hyväksynnällä, eikä suljettuun kuukauteen
  voi kirjata. Korjaukset menevät seuraavalle avoimelle kaudelle
  ([Kaudet, sulku ja tilikausi](periods.md)).

## Liitteet ja säilytys

- Jokainen lähdetiedosto (kuitti, lasku, tiliote) arkistoidaan kodin
  liitevarastoon sisällöstä lasketulla tunnisteella, ja tosite viittaa siihen.
  Sama tiedosto tunnistetaan, jos se yritetään kirjata uudelleen.
- Alkuperäinen tiliotetiedosto arkistoidaan kerran ja liitetään jokaiseen sen
  tapahtumista syntyvään tositteeseen.
- Liitevarasto ja tositetietokanta eivät ole gitissä, vaan omistaja
  varmuuskopioi ne erikseen
  ([Yrityskoti](company-home.md#git-ja-varmuuskopio)).

## ALV

ALV-käsittely tallentuu tositteeseen sinetöinnin hetkellä nimettynä
käsittelynä, ja ALV-ilmoituksen luvut lasketaan näistä merkinnöistä, ei
tilinumeroista. Verokanta tulee päivätystä verokantataulukosta. ALV-ilmoitus
lasketaan vain suljetusta kaudesta
([Miten ALV-käsittely päätetään](vat-treatment.md)).

## Mitä aineistoa saat

- **Kuukausiraportti** HTML- tai JSON-muodossa: kuukauden tuloslaskelma,
  tilikauden alusta kertynyt tulos ja kuukauden lopun tase. Jokainen tili
  viittaa tositteisiin, joista sen summa koostuu
  ([Katso, miten kuukausi meni](../tasks/monthly-report.md)).
- **Tositteet** viennin tarkkuudella koneluettavana sekä niiden liitteet.
- **Kausien tila** ja tieto siitä, onko kauden ALV ilmoitettu.

Pyydä omistajaa tai agenttia kokoamaan aineisto:
[Valmistele tilinpäätös](../tasks/year-end.md).

## Rajat

- **Virallinen tilinpäätös laaditaan muualla.** Tilinpäätösajo kirjaa
  poistot, jaksotukset ja oikaisut päättyneelle vuodelle, laskee
  tuloslaskelman ja taseen ja sinetöi tilikauden. Tilinpäätösasiakirja ja
  liitetietojen sanallinen osa tehdään muualla.
- **Palkat** lasketaan, maksetaan ja ilmoitetaan muualla. aggountantiin
  kirjataan palkkatosite. Tulorekisteriin aggountant ilmoittaa vain
  verovapaat matkakorvaukset.
- **Tiliotteelta kirjatun oston liitteenä on tiliote**, ei ostolaskua.
- **Toisen yrityksen puolesta ilmoittamista ei tueta.** Yksi koti on yhden
  yrityksen kirjanpito, ja ilmoitukset lähtevät yrityksen omilla tunnuksilla.

Kaikki rajoitukset: [Mitä on valmista ja mitä ei](../reference/status.md).

## Katso myös

- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
- [Valmistele tilinpäätös](../tasks/year-end.md)
- [Sanasto](../reference/glossary.md)
