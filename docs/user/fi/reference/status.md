# Mitä on valmista ja mitä ei

Tämä sivu kertoo jokaisesta rajoituksesta, mikä ei toimi, miksi ja mitä teet
sen sijaan. Muut sivut viittaavat tänne yhdellä virkkeellä. Rajoitusta
seuraavan issuen tunniste on suluissa; issuet ovat projektin
[GitHub-repositoriossa](https://github.com/aggountant/aggountant) hakemistossa
`issues/`. Sivu koskee versiota 0.1.

## Yleiskuva

aggountant on versiossa 0.1, ja sillä pidetään jo muutaman sellaisen
suomalaisen yrityksen kirjanpitoa, jolla ei ole tilitoimistoa. Ominaisuuksia
lisätään sitä mukaa kuin nämä yritykset kohtaavat uusia tilanteita. Siksi
tavallinen kuukausityö on hyvässä kunnossa, ja harvinaisemmat tapaukset
täydentyvät vähitellen. Komennot ja tiedostomuodot voivat vielä muuttua
versioiden välillä.

**Valmis oikeaan kirjanpitoon:**

- tositteiden tallentaminen ja liitteiden arkistointi
- pankkitiliotteen tuonti (camt.053 ja camt.052)
- tavallisten ostokuittien kirjaaminen ALV-käsittelyineen
- kuukauden sulku
- kuukausiraportit
- ALV-ilmoitus suoraan Verohallinnolle tai käsin OmaVerossa
- verovapaiden matkakorvausten ilmoitus Tulorekisteriin
- tilinpäätöksen kirjaukset ja tilikauden sinetöinti tilinpäätösajolla.

**Ohjelmassa, mutta ei vielä käytettävissä oikeaan kirjanpitoon:**
palkka-ajo, aggountantin oma automaattinen kuittihaku ja poikkeuksellisten
kuittien konsultointi. Niistä kerrotaan alla.

## Asennus

**Valmiita asennuspaketteja ei vielä ole.** aggountant käännetään
lähdekoodista Rust-työkaluilla. Tee näin:
[Asenna aggountant ja skillit](../start/install.md).

## Pankin tiliote

- **Jokaiselle vahvistetulle tapahtumalle tarvitaan kirjaus.** Jos yksikin
  puuttuu (`MISSING_ASSIGNMENT`) tai viittaa tiliin, jota tilikartassa ei ole
  (`ACCOUNT_NOT_FOUND`), koko tiliote hylätään eikä mitään sinetöidä. Tämä
  koskee myös riviä, joka maksaa jo kirjatun kuitin; katso alta.
- **Vain Holvin tiliote on koeteltu.** Muiden pankkien camt-tiedostot voivat
  toimia, mutta niitä ei ole koeteltu. Tarkista ensimmäisen tuonnin jälkeen,
  että pankkitilin saldo vastaa tiliotetta.
- **Myynnin ALV:tä ei pureta tiliotteelta.** Myynti, johon sisältyy ALV,
  kirjataan erikoiskirjauksena ja asiakkaan maksu tiliotteelta
  myyntisaamisia vastaan (`vat-sales-expansion`).
- **Tiliotteen tuonnissa ei ole esikatselun tunnistetta** kuten kuittien
  kirjauksessa. Hyväksyntäsi on se, että katsot agentin kirjauslistan rivi
  riviltä ennen sinetöintiä.
- **Tyhjä tiliote tuodaan onnistuneena ilman tapahtumia** eikä ohjelma kerro
  siitä erikseen. Jos tuonti antaa nolla tapahtumaa, tarkista, että tiedosto
  on oikean kuukauden ja oikean tilin tiliote (`camt-empty-report-and-rename`).

### Pankkitililtä maksettu ostolasku

Kun ostolasku tai kuitti on maksettu yrityksen tililtä, sama osto näkyy sekä
kuitissa että tiliotteella. Tässä versiossa kumpikaan reitti ei ole täydellinen:

- **Kirjattuna tiliotteelta** tositteen liitteenä on tiliote, ei laskua.
  Laskua ei voi liittää tiliotteelta kirjattuun tositteeseen.
- **Kirjattuna kuittina** lasku on liitteenä, mutta tiliotteen tuonti vaatii
  silti kirjauksen samalle maksuriville. Jos kuitti on kirjattu pankkitiliä
  vastaan ja maksurivi kirjataan vielä kuluksi, **sama osto on kirjanpidossa
  kahdesti**.

Ohjelma ei vielä tarjoa tapaa merkitä tiliotteen riviä jo kirjatun kuitin
maksuksi, eikä ohje vielä kerro yhtä vakiintunutta tapaa kirjata sitä.
Tee näin: kun hyväksyt tiliotteen kirjaukset, kysy jokaisesta rivistä, jonka
kuitti on jo kirjattu, mille tilille agentti sen kirjaa, ja varmista, ettei
kulu tule kahdesti. Kuukausiraportin pankkitilin saldon vertaaminen
tiliotteeseen paljastaa kahdenkertaisen kirjauksen ennen sulkua
(`bank-row-for-recorded-receipt`).

## Kuitit ja kirjaukset

- **Poikkeuksellisten kuittien konsultointi ei ole käytössä.** Ohjelmassa on
  reitti (`capture-receipt start`), jossa ulos lähtisivät vain hyväksymäsi
  tiedot, mutta sen takana ei ole yhteyttä konsultointipalveluun. Agentti
  kysyy sinulta tai kirjanpitäjältäsi.
- **aggountantin oma kuittihaku ei ole käytössä.** `hunt-receipts`-komentoon ei
  ole kytketty oikeaa sähköposti- tai verkkokauppatiliyhteyttä. Agentti voi
  etsiä kuitteja omilla työkaluillaan sieltä, minne olet antanut sille pääsyn,
  ja muuten haku on käsityötä.
- **Kuittireitti ei käsittele osittaista vähennysoikeutta eikä
  maahantuontia.** Agentti kertoo, jos osto on sellainen. Kysy
  kirjanpitäjältäsi.
- **Omaa korjauskomentoa ei ole.** Korjaus kirjataan tavallisena
  erikoiskirjauksena: [Korjaa virheellinen tosite](../tasks/correct-voucher.md).
- **Yksittäisen tositteen viennit näkyvät vain agentille.** `voucher show`
  näyttää ilman `--json`-valitsinta vain numeron ja tunnisteen. Pyydä
  agenttia näyttämään tositteen viennit.

## Kaudet

### Hylätty sulku

**Hylätty sulku jättää kauden pysyvästi tilaan `closing`.** Kaudelle ei voi
silloin kirjata eikä sitä voi sulkea uudelleen, eikä tässä versiossa ole
komentoa, jolla kauden saisi takaisin avoimeksi. Tee näin: aloita sulku vasta,
kun kausi on valmis, ja hyväksy aloitettu sulku. Jälkikäteen huomatut asiat
kirjataan seuraavalle kaudelle. Jos sulku on jo hylätty, katso
[Kun jokin menee vikaan](../troubleshooting.md#kuukausi-jäi-suljettavaksi)
(`rejected-close-reopens-period`).

### Tilikausi ja kaudet

- **Kalenterivuodesta poikkeava tilikausi suljetaan kuukausittain.**
  Neljännesvuosisulku on käytettävissä vain kalenterivuoden tilikaudelle.
- **Kaksi tilikautta ei voi päättyä samana kalenterivuonna.** Tilikauden
  muutos tehdään yhdellä enintään 18 kuukauden siirtymätilikaudella.
- **Poistoja ei lasketa tilikaudelle, joka ei ole 12 kuukautta.**
  Tilinpäätösajo kieltäytyy laskemasta niitä; laske sen tilikauden poistot
  itse ja anna ne ajolle oikaisuna, jolloin ne kohdistuvat oikealle vuodelle.
- **Tilikauden määrittelyä ei voi muuttaa.** Sen perään voi määritellä uuden,
  mutta ei niin, että jo kirjattuja tositteita siirtyisi toiselle
  tilikaudelle.
- **Samana vuonna ei voi sulkea sekä kuukausia että neljännesvuosia.**
  Kuukausi on suositeltu ja koeteltu vaihtoehto.

## ALV-ilmoitus

- **Esikatselun ohjeteksti on vanhentunut.** Se kehottaa aina syöttämään luvut
  käsin OmaVeroon, vaikka suora ilmoitus olisi käytössä. Suoran reitin käyttö
  riippuu vain siitä, onko valmius kunnossa (`vat-preview-filing-instruction`).
- **Suoran ilmoituksen maksuvaihe ei valmistu.** Kun veroa on maksettavaa, ajo
  avaa maksun hyväksynnän, mutta aggountantilla ei ole maksuyhteyttä. Ajo jää
  odottamaan, vaikka ilmoitus on tehty. Maksa vero OmaVeron viitteellä itse
  (`vat-run-payment-terminal`).
- **Verohallinnon tilinumero annetaan joka kerta.** Agentti antaa
  ilmoitusajolle OmaVerossa näkyvän tilinumeron. Maksun viitettä ohjelma ei
  tiedä (`tax-payment-references`).
- **Korvaavia ilmoituksia ei tehdä.** Kun ilmoitettuihin lukuihin tulee
  korjaus, tee korvaava ilmoitus OmaVerossa.
- **EU-myynnin ja verottoman myynnin kentät 309–311 eivät tule
  kirjanpidosta.** Jos yritykselläsi on EU-myyntiä tai vientiä, tee ilmoitus
  OmaVerossa ja syötä nämä kentät itse. Yhteenvetoilmoitusta aggountant ei
  tee (`vat-zero-rated-sales-fields`).
- **Kautta, jonka ALV on korjausten vuoksi negatiivinen, ei voi ilmoittaa
  aggountantilla.** Esimerkiksi kuukausi, jonka ainoa myynti on hyvityslasku.
  Virhe nimeää harhaanjohtavasti valuutan (`VAT_CURRENCY`). Tee ilmoitus
  OmaVerossa (`vat-negative-period-filing`).
- **ALV-vientiä ilman ALV-merkintää ei voi korjata ilmoitukseen
  aggountantissa.** Ilmoitus lasketaan ALV-merkityistä vienneistä. Tositteen
  tarkistus varoittaa merkitsemättömästä ALV-viennistä, ja ALV-ilmoituksen
  esikatselu näyttää sen (`VAT_ACCOUNTS_UNRECONCILED`), mutta suljettua
  kuukautta ei voi avata korjattavaksi. Tee silloin ilmoitus OmaVerossa
  korjatuin luvuin ([Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen)).
- **Neljännesvuosikausia on koeteltu vähemmän.** Suoraa ilmoittamista on
  koeteltu kuukausikausilla.
- **Ilmoitin.fi-tiedostoa aggountant ei laadi.**

## Palkat

**Rahapalkkoja ei vielä voi laskea, maksaa eikä ilmoittaa aggountantilla.**
Palkka-ajo (`run-payroll`) on ohjelmassa, mutta sen maksutaulukot
(työeläke-, sairausvakuutus- ja työttömyysvakuutusmaksut) ovat alustavia
arvoja, joita ei ole tarkistettu virallisista lähteistä. Verokortin käsittely
on yksinkertaistettu, eikä ajon takana ole oikeaa maksu- tai
Tulorekisteri-yhteyttä. Tee näin: laske, maksa ja ilmoita palkat
palkanlaskentapalvelulla tai Tulorekisterin asiointipalvelussa ja kirjaa
palkkatosite aggountantiin:
[Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](../tasks/record-other-entries.md).

## Tulorekisteri ja työntekijät

- **Vain verovapaat matkakorvaukset.** aggountant ilmoittaa
  kilometrikorvaukset (311) ja päivärahat (331) yhdelle tulonsaajalle ja
  yhdestä maksusta kerrallaan. Työnantajan erillisilmoitus, korvaava ilmoitus
  ja mitätöinti tehdään Tulorekisterin asiointipalvelussa.
- **Tulorekisteri-profiilille ei ole omaa komentoa.** Agentti kirjoittaa sen
  `vero-fi`-skillin ohjeen mukaan.
- **Työntekijärekisteriä ei ole salattu.** Komennon ohje puhuu salatusta
  varastosta, mutta tiedot tallentuvat selkokielisinä. Suojana on vain se, että
  tiedosto pidetään erillään gitistä. Pidä kotihakemisto ja sen varmuuskopiot
  vain omassa käytössäsi (`employee-store-plaintext-help`).
- **Henkilötiedoille ei ole turvallista syöttötapaa.** `employee show` näyttää
  henkilötunnuksen ja tilinumeron selväkielisinä, eikä tietoja voi syöttää
  näyttämättä niitä. Laadi syöte itse tiedostoon, älä kirjoita
  henkilötunnusta keskusteluun, ja poista tiedosto lisäyksen jälkeen
  (`employee-pii-entry`).
- **Verokortin pidätysprosentti on pakollinen** myös henkilölle, jolle
  ilmoitetaan vain matkakorvauksia. Se ei vaikuta matkakorvausilmoitukseen.

## Tilinpäätös

Kun vuoden kaikki kuukaudet on suljettu, tilinpäätösajo (`close-year`)
laskee poistot ja jaksotukset. Ajo kirjaa ne ja sille annetut oikaisut
seuraavan tilikauden ensimmäiselle avoimelle kaudelle, koska vuoden kuukaudet
on jo suljettu, ja kohdistaa ne suljettavalle tilikaudelle. Siksi ne näkyvät
suljettavan vuoden tuloslaskelmassa ja taseessa, eivät seuraavan vuoden
tuloksessa, ja seuraava vuosi alkaa saldoista, joissa ne ovat mukana. Sen
jälkeen ajo käy läpi neljä hyväksyntää (oikaisukirjaukset, tase-erittelyt,
liitetiedot ja laskelmat) ja sinetöi tilikauden.

- **Hyväksyntävaiheet eivät näytä laskelmia.** Komennon tuloste kertoo vain
  vaiheiden tilan. Kun oikaisukirjaukset on hyväksytty, tilikauden viimeisen
  kuukauden kuukausiraportti näyttää tuloslaskelman ja taseen oikaisuineen.
  Katso se ennen kuin hyväksyt laskelmat (`close-year-gates-show-content`).
- **Oikaisut annetaan ajon alussa.** Ajo kirjaa ne ennen kuin oikaisukirjaukset
  hyväksytään, ja vain ajon omat kirjaukset kohdistetaan suljettavalle
  vuodelle. Tosite, jonka kirjaat itse seuraavan vuoden kaudelle, näkyy
  seuraavan vuoden tuloksessa.
- **Virallinen tilinpäätösasiakirja** liitetietojen sanallisine osineen,
  allekirjoitukset, ilmoitus kaupparekisteriin ja veroilmoitus tehdään
  aggountantin ulkopuolella.

Tilikauden sinetöinti on peruuttamaton. Tee näin:
[Valmistele tilinpäätös](../tasks/year-end.md).

## Yrityskoti ja käyttöönotto

- **Väärä kodin polku voi luoda tyhjän kodin.** Jos komento osuu
  hakemistoon, jossa ei ole kirjanpitoa, ohjelma voi luoda sinne tyhjän
  tositevaraston. Oikea kirjanpito ei vahingoitu, mutta tositteet näyttävät
  kadonneen, kunnes huomaat virheen
  ([Kun jokin menee vikaan](../troubleshooting.md#tositteet-näyttävät-kadonneen),
  `read-commands-create-home`).
- **Varmuuskopio- tai vientikomentoa ei ole.** Varmuuskopio on tavallinen
  hakemiston kopio:
  [Varmuuskopioi ja palauta kirjanpito](../tasks/back-up-and-restore.md)
  (`home-backup-coverage`).
- **Monivuotisen historian tuonti ei ole valmis.** Käyttöönotto alkaa
  avaavista saldoista (`historical-import`).
- **Testiympäristön muuttuja voi merkitä ilmoituksen tehdyksi.**
  `AGGOUNTANT_BROKER_SCRIPT` on tarkoitettu vain ohjelman testaukseen. Jos se
  on asetettuna, `file-vat` voi merkitä kauden ilmoitetuksi, vaikka mitään ei
  lähetetty. Varmista, ettei muuttuja ole voimassa, kun teet oikeaa
  ilmoitusta.

## Katso myös

- [Onko aggountant sinulle?](../start/is-it-for-you.md)
- [Kun jokin menee vikaan](../troubleshooting.md)
