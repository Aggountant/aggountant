# Kirjanpitäjälle

Kun hoidat kirjanpitoa omistajan puolesta aggountantilla tai saat siitä
aineiston tilinpäätöstä varten, sinun on tiedettävä, miten ohjelma vastaa
kirjanpitolain vaatimuksiin, mitä aineistoa siitä saa ja mitä sillä ei tehdä.

## Tositteet ja kirjausketju

- Tositteet numeroidaan aukottomasti yhteen juoksevaan sarjaan
  sinetöintijärjestyksessä.
- Sinetöidyn tositteen sisällöstä lasketaan tunniste, joka ketjutetaan
  edelliseen.
  [Ketjun tarkistus](sealing-and-approval.md#mitä-sinetöinti-on) paljastaa,
  jos tositetta on muutettu tai se on poistettu jälkikäteen.
- Sinetöityä tositetta ei muokata. Virhe korjataan korjaustositteella, joka
  viittaa alkuperäiseen, ja molemmat jäävät kirjanpitoon.
- Tositteella on erikseen tositepäivä (kuitin tai laskun päivä) ja
  kirjauspäivä. Tiliotteelta kirjatun tapahtuman kirjauspäivä on pankin
  kirjauspäivä.
- [Kuukausi suljetaan](periods.md) hyväksynnällä, eikä suljettuun kuukauteen
  voi kirjata. Korjaukset menevät seuraavalle avoimelle kaudelle.

## Liitteet ja säilytys

- Jokainen lähdetiedosto (kuitti, lasku, tiliote) arkistoidaan yrityskodin
  liitevarastoon sisällöstä lasketulla tunnisteella, ja tosite viittaa siihen.
  Sama tiedosto tunnistetaan, jos se yritetään kirjata uudelleen.
- Alkuperäinen tiliotetiedosto arkistoidaan kerran ja liitetään jokaiseen sen
  tapahtumista syntyvään tositteeseen.
- Yrityksen tililtä maksettu kuitti tai ostolasku kirjataan omana
  tositteenaan ostovelkaa 2700 vastaan liitteineen, ja tiliotteen maksurivi
  kirjataan samaa ostovelkaa vastaan.
- Liitevarasto ja tositetietokanta eivät ole gitissä, vaan omistaja
  [varmuuskopioi ne erikseen](company-home.md#git-ja-varmuuskopio).

## ALV

Tositteeseen tallentuu sinetöinnin hetkellä nimetty
[ALV-käsittely](vat-treatment.md), ja ALV-ilmoituksen luvut lasketaan näistä
merkinnöistä, ei tilinumeroista. Verokanta tulee päivätystä
verokantataulukosta laskun päivän mukaan. ALV-ilmoituksen voi antaa vain
suljetusta kaudesta.

## Mitä aineistoa saat

aggountantista saa:

- [kuukausiraportin](../tasks/monthly-report.md) HTML- tai JSON-muodossa.
  Siinä on kuukauden tuloslaskelma, tilikauden alusta kertynyt tulos ja
  kuukauden lopun tase, ja jokainen tili viittaa tositteisiin, joista sen
  summa koostuu.
- tositteet viennin tarkkuudella koneluettavina ja niiden liitteet
- kausien tilan ja tiedon siitä, onko kauden ALV ilmoitettu.

Pyydä omistajaa tai agenttia
[kokoamaan aineisto](../tasks/year-end.md).

## Rajat

- Virallinen tilinpäätösasiakirja ja liitetietojen sanallinen osa tehdään
  muualla. Tilinpäätösajo kirjaa poistot, jaksotukset ja oikaisut
  päättyneelle vuodelle, laskee tuloslaskelman ja taseen ja sinetöi
  tilikauden.
- Palkat lasketaan, maksetaan ja ilmoitetaan muualla, ja aggountantiin
  kirjataan palkkatosite. Tulorekisteriin aggountant ilmoittaa vain
  verovapaat matkakorvaukset ja työnantajan erillisilmoituksen kuukausilta,
  joilta ei maksettu palkkaa.
- Toisen yrityksen puolesta ei voi ilmoittaa. Yksi koti on yhden yrityksen
  kirjanpito, ja ilmoitukset lähtevät yrityksen omilla tunnuksilla.

Kaikki rajoitukset ovat sivulla
[Mitä on valmista ja mitä ei](../reference/status.md).

## Katso myös

- [Sinä, agentti ja aggountant](division-of-work.md)
- [Korjaa virheellinen tosite](../tasks/correct-voucher.md)
- [Sanasto](../reference/glossary.md)
