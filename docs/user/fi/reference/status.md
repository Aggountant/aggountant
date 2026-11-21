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
- työnantajan erillisilmoitus Tulorekisteriin kuukausilta, joilta ei maksettu
  palkkaa
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
  (`ACCOUNT_NOT_FOUND`), koko tiliote hylätään eikä mitään sinetöidä. Myös jo
  kirjatun kuitin maksurivi kirjataan: se kirjataan kuitin ostovelkaa
  vastaan ([Kun osto näkyy myös tiliotteella](../tasks/record-receipts.md#kun-osto-näkyy-myös-tiliotteella)).
- **Vain Holvin tiliote on koeteltu.** Muiden pankkien camt-tiedostot voivat
  toimia, mutta niitä ei ole koeteltu. Tiliotteen lukeminen näyttää, vievätkö
  sen tapahtumat alkusaldon loppusaldoon; tarkista lisäksi ensimmäisen
  tuonnin jälkeen, että pankkitilin saldo vastaa tiliotetta.
- **Myynnin ALV:tä ei pureta tiliotteelta.** Myynti, johon sisältyy ALV,
  kirjataan erikoiskirjauksena ja asiakkaan maksu tiliotteelta
  myyntisaamisia vastaan (`vat-sales-expansion`).
- **Esikatselu sitoo hyväksynnän vain, kun agentti antaa sen tunnisteen
  sinetöinnille.** Tunnisteen kanssa sinetöinti hylkää kaiken, mikä muuttui
  esikatselun jälkeen (`IMPORT_PREPARATION_CHANGED`). Ilman tunnistetta
  sinetöintiä ei verrata mihinkään esikatseluun, jotta aiemmat tavat ajaa
  tuonti toimivat edelleen.
- **Tyhjä tiliote tuodaan onnistuneena ilman tapahtumia** eikä ohjelma kerro
  siitä erikseen. Jos tuonti antaa nolla tapahtumaa, tarkista, että tiedosto
  on oikean kuukauden ja oikean tilin tiliote (`camt-empty-report-and-rename`).
- **Ohjelma ei yhdistä kuittia ja sen maksuriviä.** Kuitti ja tiliotteen rivi
  kohtaavat ostovelkatilillä 2700 (tai 2740), mutta ohjelma ei tarkista, että
  jokainen kuitti tulee maksetuksi tai ettei samaa ostoa ole kirjattu myös
  tiliotteen riviltä kuluksi. Tarkista kuun lopussa, että ostovelkojen saldo
  on vain maksamattomien laskujen summa.

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

### Tilikausi ja kaudet

- **Kalenterivuodesta poikkeava tilikausi suljetaan kuukausittain.**
  Neljännesvuosisulku on käytettävissä vain kalenterivuoden tilikaudelle.
- **Kaksi tilikautta ei voi päättyä samana kalenterivuonna.** Tilikauden
  muutos tehdään yhdellä enintään 18 kuukauden siirtymätilikaudella.
- **Poistoja ei lasketa tilikaudelle, joka ei ole 12 kuukautta.**
  Tilinpäätösajo kieltäytyy laskemasta niitä; laske sen tilikauden poistot
  itse ja anna ne ajolle oikaisuna, jolloin ne kohdistuvat oikealle vuodelle.
  Edelliseltä vuodelta siirtyneet hyödykkeet merkitään sille vuodelle
  poistuneiksi ja annetaan menojäännöksineen uudelleen seuraavassa
  tilinpäätöksessä.
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
- **Korvaava ilmoitus ei maksa.** Jo annetun ilmoituksen korjaus tehdään
  aggountantilla
  ([Korjaa jo annettu ilmoitus](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus)),
  mutta lisää maksettavaa tai palautuvaa erotusta ohjelma ei laske: se näkyy
  OmaVeron verotilillä. Korvaavaa ilmoitusta ei ole vielä lähetetty
  Verohallinnon tuotantoon.
- **EU-myynnin ja verottoman myynnin kentät 309–311 eivät tule
  kirjanpidosta.** Jos yritykselläsi on EU-myyntiä tai vientiä, tee ilmoitus
  OmaVerossa ja syötä nämä kentät itse. Yhteenvetoilmoitusta aggountant ei
  tee (`vat-zero-rated-sales-fields`).
- **Kautta, jonka ALV on korjausten vuoksi negatiivinen, ei voi ilmoittaa
  aggountantilla.** Esimerkiksi kuukausi, jonka ainoa myynti on hyvityslasku.
  Virhe nimeää harhaanjohtavasti valuutan (`VAT_CURRENCY`). Tee ilmoitus
  OmaVerossa (`vat-negative-period-filing`).
- **Sulun jälkeen löytynyttä ALV-vientiä ilman ALV-merkintää ei voi korjata
  ilmoitukseen aggountantissa.** Ilmoitus lasketaan ALV-merkityistä vienneistä.
  Tositteen tarkistus varoittaa merkitsemättömästä ALV-viennistä, ja
  ALV-ilmoituksen esikatselu näyttää sen (`VAT_ACCOUNTS_UNRECONCILED`) myös
  avoimelle kuukaudelle ennen sulkua, jolloin korjaava tosite vielä korjaa sen.
  Suljettua kuukautta ei voi avata korjattavaksi: tee silloin ilmoitus
  OmaVerossa korjatuin luvuin ([Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen)).
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
  yhdestä maksusta kerrallaan. Hyväksytyn ilmoituksen korvaava ilmoitus ja
  mitätöinti tehdään myös aggountantilla
  ([Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md#korjaa-hyväksytty-ilmoitus)).
- **Työnantajan erillisilmoitus vain kuukausilta ilman palkkaa.** aggountant
  antaa "Ei palkanmaksua" -ilmoituksen ja kuukausille, joilta maksettiin vain
  verovapaita matkakorvauksia, sairausvakuutusmaksun 0 €. Palkkakuukauden
  sairausvakuutusmaksua se ei laske, joten sellainen kuukausi hylätään
  (`EMPLOYER_REPORT_WAGES_UNSUPPORTED`). Tee näin: anna kuukauden ilmoitus
  asiointipalvelussa ja anna viite agentille. Myös eläkevakuutustietojen
  korjaus ja ilmoituksen mitätöinti tehdään asiointipalvelussa. Määräpäivänä
  näytetään aina kuukauden 5. päivä, vaikka lauantaina tai pyhänä ilmoituksen
  saa antaa seuraavana pankkipäivänä
  ([Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md)).
- **Työntekijärekisteriä ei ole salattu.** Tiedot tallentuvat selkokielisinä.
  Suojana on vain se, että tiedosto pidetään erillään gitistä. Pidä
  kotihakemisto ja sen varmuuskopiot vain omassa käytössäsi.
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
liitetiedot ja laskelmat) ja sinetöi tilikauden. Jokainen hyväksyntävaihe
näyttää juuri sen, mitä hyväksyt.

- **Poisto lasketaan aina hyödykeluokan enimmäisprosentilla.** Sinetöity
  vuosi tallentaa hyödykkeet ja niiden menojäännöksen, ja seuraava
  tilinpäätösajo poistaa ne siitä eteenpäin. Jos yritys poistaa jonakin
  vuonna vähemmän tai muulla suunnitelmalla, merkitse hyödyke sille vuodelle
  poistuneeksi, anna sen poisto oikaisuna ja anna hyödyke menojäännöksineen
  uudelleen seuraavana vuonna. Myyntiä tai romutusta ajo ei kirjaa: kirjaa
  se itse ja merkitse hyödyke poistuneeksi.
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
  avaavista saldoista, ja vanhan järjestelmän historia säilytetään arkistona:
  [Siirrä kirjanpito toisesta ohjelmasta](../start/move-from-another-program.md)
  (`historical-import`).
- **Yrityksen omaa tilikarttaa ei voi tuoda.** Kirjanpito käyttää
  oletustilikarttaa, ja vanhan järjestelmän tilit muunnetaan sille avaavassa
  tositteessa (`sie4-chart-import`).
- **Testiympäristön muuttuja voi merkitä ilmoituksen tehdyksi.**
  `AGGOUNTANT_BROKER_SCRIPT` on tarkoitettu vain ohjelman testaukseen. Jos se
  on asetettuna, `file-vat` voi merkitä kauden ilmoitetuksi, vaikka mitään ei
  lähetetty. Tällainen tuloste on merkitty kentällä `test_seam`. Varmista,
  ettei muuttuja ole voimassa, kun teet oikeaa ilmoitusta.

## Katso myös

- [Onko aggountant sinulle?](../start/is-it-for-you.md)
- [Kun jokin menee vikaan](../troubleshooting.md)
