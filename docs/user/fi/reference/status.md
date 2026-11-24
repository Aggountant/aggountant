# Mitä on valmista ja mitä ei

aggountant on versiossa 0.2, ja sillä pidetään jo muutaman sellaisen
suomalaisen yrityksen kirjanpitoa, jolla ei ole tilitoimistoa. Ominaisuuksia
lisätään sitä mukaa kuin nämä yritykset kohtaavat uusia tilanteita, joten
tavallinen kuukausityö on hyvässä kunnossa ja harvinaisemmat tapaukset
täydentyvät vähitellen. Komennot ja tiedostomuodot voivat vielä muuttua
versioiden välillä.

Alla on aiheittain jokainen rajoitus: mikä ei toimi, miksi ja mitä teet sen
sijaan. Suluissa on rajoitusta seuraavan issuen tunniste. Issuet ovat projektin
[GitHub-repositoriossa](https://github.com/aggountant/aggountant) hakemistossa
`issues/`.

## Yleiskuva

Oikeaan kirjanpitoon ovat valmiita:

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

Ohjelmassa ovat myös palkka-ajo, aggountantin oma automaattinen kuittihaku ja
poikkeuksellisten kuittien konsultointi, mutta niitä ei vielä voi käyttää
oikeaan kirjanpitoon. Niistä kerrotaan alla.

## Asennus

aggountant toimii macOS:llä ja Linuxilla. Valmis asennuspaketti on Apple
Silicon -Macille ja Linuxille (arm64 ja x86_64); Intel-Macilla aggountant
käännetään lähdekoodista Rust-työkaluilla. Windowsia ei ole testattu. Ohjeet ovat sivulla
[Asenna aggountant ja skillit](../start/install.md).

## Pankin tiliote

- Jokaiselle vahvistetulle tapahtumalle tarvitaan kirjaus. Jos yksikin
  puuttuu (`MISSING_ASSIGNMENT`) tai viittaa tiliin, jota tilikartassa ei ole
  (`ACCOUNT_NOT_FOUND`), koko tiliote hylätään eikä mitään sinetöidä. Myös jo
  kirjatun kuitin maksurivi tarvitsee kirjauksen: se kirjataan
  [kuitin ostovelkaa vastaan](../tasks/record-receipts.md#kun-osto-näkyy-myös-tiliotteella).
- Vain Holvin tiliote on koeteltu. Muiden pankkien camt-tiedostot voivat
  toimia. Tiliotteen lukeminen näyttää, vievätkö sen tapahtumat alkusaldon
  loppusaldoon; tarkista lisäksi ensimmäisen tuonnin jälkeen, että pankkitilin
  saldo vastaa tiliotetta.
- Myynnin ALV:tä ei pureta tiliotteelta. Myynti, johon sisältyy ALV, kirjataan
  erikoiskirjauksena ja asiakkaan maksu tiliotteelta myyntisaamisia vastaan
  (`vat-sales-expansion`).
- Esikatselu sitoo hyväksyntäsi vain, kun agentti antaa esikatselun
  tunnisteen sinetöinnille. Silloin sinetöinti hylkää kaiken, mikä muuttui
  esikatselun jälkeen (`IMPORT_PREPARATION_CHANGED`). Ilman tunnistetta
  sinetöintiä ei verrata mihinkään esikatseluun.
- Tyhjä tiliote tuodaan onnistuneena ilman tapahtumia, eikä ohjelma kerro
  siitä erikseen. Jos tuonti antaa nolla tapahtumaa, tarkista, että tiedosto
  on oikean kuukauden ja oikean tilin tiliote (`camt-empty-report-and-rename`).
- Ohjelma ei yhdistä kuittia ja sen maksuriviä. Ne kohtaavat ostovelkatilillä
  2700 (tai 2740), mutta ohjelma ei tarkista, että jokainen kuitti tulee
  maksetuksi tai ettei samaa ostoa ole kirjattu myös tiliotteen riviltä
  kuluksi. Tarkista kuun lopussa, että ostovelkojen saldo on vain
  maksamattomien laskujen summa.

## Kuitit ja kirjaukset

- Poikkeuksellisten kuittien konsultointi ei ole käytössä. Ohjelmassa on
  reitti (`capture-receipt start`), jossa ulos lähtisivät vain hyväksymäsi
  tiedot, mutta sen takana ei ole yhteyttä konsultointipalveluun. Agentti
  kysyy sinulta tai kirjanpitäjältäsi.
- aggountantin oma kuittihaku ei ole käytössä, koska `hunt-receipts`-komentoon
  ei ole kytketty oikeaa sähköposti- tai verkkokauppatiliyhteyttä. Agentti voi
  etsiä kuitteja omilla työkaluillaan sieltä, minne olet antanut sille pääsyn.
  Muuten haku on käsityötä.
- Kuittireitti ei käsittele osittaista vähennysoikeutta eikä maahantuontia.
  Agentti kertoo, jos osto on sellainen, ja silloin kysyt kirjanpitäjältäsi.
- Omaa korjauskomentoa ei ole. Korjaus kirjataan tavallisena
  erikoiskirjauksena: [Korjaa virheellinen tosite](../tasks/correct-voucher.md).
- Yksittäisen tositteen viennit näkyvät vain agentille, koska `voucher show`
  näyttää ilman `--json`-valitsinta vain numeron ja tunnisteen. Pyydä
  agenttia näyttämään tositteen viennit.

## Tilikausi ja kaudet

- Kalenterivuodesta poikkeava tilikausi suljetaan kuukausittain.
  Neljännesvuosisulku on käytettävissä vain kalenterivuoden tilikaudelle.
- Samana vuonna ei voi sulkea sekä kuukausia että neljännesvuosia. Kuukausi
  on suositeltu ja koeteltu vaihtoehto.
- Kaksi tilikautta ei voi päättyä samana kalenterivuonna. Tilikauden muutos
  tehdään yhdellä enintään 18 kuukauden siirtymätilikaudella.
- Tilikauden määrittelyä ei voi muuttaa. Sen perään voi määritellä uuden,
  mutta ei niin, että jo kirjattuja tositteita siirtyisi toiselle
  tilikaudelle.
- Poistoja ei lasketa tilikaudelle, joka ei ole 12 kuukautta. Laske sen
  tilikauden poistot itse ja anna ne tilinpäätösajolle oikaisuna, jolloin ne
  kohdistuvat oikealle vuodelle. Edelliseltä vuodelta siirtyneet hyödykkeet
  merkitään sille vuodelle poistuneiksi ja annetaan menojäännöksineen
  uudelleen seuraavassa tilinpäätöksessä.

## ALV-ilmoitus

- aggountant ei maksa veroa. Kun suorassa ilmoituksessa on maksettavaa, ajo
  päättyy, kun ilmoitus on annettu, ja näyttää määrän, tilin ja eräpäivän;
  maksa vero itse OmaVeron viitteellä.
- Verohallinnon tilinumero annetaan joka kerta: agentti antaa ilmoitusajolle
  OmaVerossa näkyvän tilinumeron. Maksun viitettä ohjelma ei tiedä
  (`tax-payment-references`).
- Korvaava ilmoitus ei maksa. Jo annettu ilmoitus
  [korjataan aggountantilla](../tasks/file-vat-direct.md#korjaa-jo-annettu-ilmoitus),
  mutta lisää maksettavaa tai palautuvaa erotusta ohjelma ei laske: se näkyy
  OmaVeron verotilillä. Korvaavaa ilmoitusta ei ole vielä lähetetty
  Verohallinnon tuotantoon.
- EU-myynnin ja verottoman myynnin kentät 309–311 eivät tule kirjanpidosta.
  Jos yritykselläsi on EU-myyntiä tai vientiä, tee ilmoitus OmaVerossa ja
  syötä nämä kentät itse. Yhteenvetoilmoitusta aggountant ei tee
  (`vat-zero-rated-sales-fields`).
- Kautta, jonka ALV on korjausten vuoksi negatiivinen, ei voi ilmoittaa
  aggountantilla. Sellainen on esimerkiksi kuukausi, jonka ainoa myynti on
  hyvityslasku. Virhe nimeää harhaanjohtavasti valuutan (`VAT_CURRENCY`). Tee
  ilmoitus OmaVerossa (`vat-negative-period-filing`).
- Sulun jälkeen löytynyttä ALV-vientiä, jolta puuttuu ALV-merkintä, ei voi
  korjata ilmoitukseen aggountantissa, koska ilmoitus lasketaan
  ALV-merkityistä vienneistä. Tositteen tarkistus varoittaa
  merkitsemättömästä ALV-viennistä, ja ALV-ilmoituksen esikatselu näyttää sen
  (`VAT_ACCOUNTS_UNRECONCILED`) jo avoimelle kuukaudelle, jolloin korjaava
  tosite vielä korjaa sen. Suljettua kuukautta ei voi avata korjattavaksi,
  joten tee silloin
  [ilmoitus OmaVerossa korjatuin luvuin](../tasks/file-vat-omavero.md#jos-jokin-menee-pieleen).
- Suora ilmoittaminen toimii myös neljännesvuosikausille, mutta oikeassa
  käytössä sitä on koeteltu kuukausikausilla.
- Ilmoitin.fi-tiedostoa aggountant ei laadi.

## Palkat

Rahapalkkoja ei vielä voi laskea, maksaa eikä ilmoittaa aggountantilla.
Palkka-ajo (`run-payroll`) on ohjelmassa, mutta sen maksutaulukot
(työeläke-, sairausvakuutus- ja työttömyysvakuutusmaksut) ovat alustavia
arvoja, joita ei ole tarkistettu virallisista lähteistä. Verokortin käsittely
on yksinkertaistettu, eikä ajon takana ole oikeaa maksu- tai
Tulorekisteri-yhteyttä.

Laske, maksa ja ilmoita palkat palkanlaskentapalvelulla tai Tulorekisterin
asiointipalvelussa ja kirjaa palkkatosite aggountantiin:
[Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](../tasks/record-other-entries.md).

## Tulorekisteri ja työntekijät

- Tulorekisteriin aggountant ilmoittaa vain verovapaat matkakorvaukset:
  kilometrikorvaukset (311) ja päivärahat (331) yhdelle tulonsaajalle ja
  yhdestä maksusta kerrallaan. Myös hyväksytyn ilmoituksen
  [korvaava ilmoitus ja mitätöinti](../tasks/report-travel-allowances.md#korjaa-hyväksytty-ilmoitus)
  tehdään aggountantilla.
- [Työnantajan erillisilmoituksen](../tasks/file-employer-report.md)
  aggountant antaa vain kuukausilta, joilta ei maksettu palkkaa: "Ei
  palkanmaksua" -ilmoituksen ja kuukausille, joilta maksettiin vain
  verovapaita matkakorvauksia, sairausvakuutusmaksun 0 €. Palkkakuukauden
  sairausvakuutusmaksua se ei laske, joten sellainen kuukausi hylätään
  (`EMPLOYER_REPORT_WAGES_UNSUPPORTED`). Anna sen kuukauden ilmoitus
  asiointipalvelussa ja anna viite agentille. Myös eläkevakuutustietojen
  korjaus ja ilmoituksen mitätöinti tehdään asiointipalvelussa.
- Erillisilmoituksen määräpäivänä näytetään aina kuukauden 5. päivä, vaikka
  lauantaina tai pyhänä ilmoituksen saa antaa seuraavana pankkipäivänä.
- Työntekijärekisteriä ei ole salattu, vaan tiedot tallentuvat
  selväkielisinä. Suojana on vain se, että tiedosto pidetään erillään
  gitistä. Pidä kotihakemisto ja sen varmuuskopiot vain omassa käytössäsi.
- Henkilötiedoille ei ole turvallista syöttötapaa. `employee show` näyttää
  henkilötunnuksen ja tilinumeron selväkielisinä, eikä tietoja voi syöttää
  näyttämättä niitä. Laadi syöte itse tiedostoon, älä kirjoita
  henkilötunnusta keskusteluun, ja poista tiedosto lisäyksen jälkeen
  (`employee-pii-entry`).
- Verokortin pidätysprosentti on pakollinen myös henkilölle, jolle
  ilmoitetaan vain matkakorvauksia. Se ei vaikuta matkakorvausilmoitukseen.

## Tilinpäätös

Kun vuoden kaikki kuukaudet on suljettu, tilinpäätösajo (`close-year`)
laskee poistot ja jaksotukset. Koska vuoden kuukaudet on jo suljettu, ajo
kirjaa ne ja sille annetut oikaisut seuraavan tilikauden ensimmäiselle
avoimelle kaudelle ja kohdistaa ne suljettavalle tilikaudelle. Ne näkyvät
siis suljettavan vuoden tuloslaskelmassa ja taseessa, ja seuraava vuosi alkaa
saldoista, joissa ne ovat mukana. Sen jälkeen ajo käy läpi neljä hyväksyntää
(oikaisukirjaukset, tase-erittelyt, liitetiedot ja laskelmat) ja sinetöi
tilikauden. Sinetöintiä ei voi perua. Vaiheet ovat sivulla
[Valmistele tilinpäätös](../tasks/year-end.md).

- Poisto lasketaan aina hyödykeluokan enimmäisprosentilla. Sinetöity vuosi
  tallentaa hyödykkeet ja niiden menojäännöksen, ja seuraava tilinpäätösajo
  poistaa ne siitä eteenpäin. Jos yritys poistaa jonakin vuonna vähemmän tai
  muulla suunnitelmalla, merkitse hyödyke sille vuodelle poistuneeksi, anna
  sen poisto oikaisuna ja anna hyödyke menojäännöksineen uudelleen seuraavana
  vuonna.
- Hyödykkeen myyntiä tai romutusta ajo ei kirjaa. Kirjaa se itse ja merkitse
  hyödyke poistuneeksi.
- Oikaisut annetaan ajon alussa. Ajo kirjaa ne ennen kuin oikaisukirjaukset
  hyväksytään, ja vain ajon omat kirjaukset kohdistetaan suljettavalle
  vuodelle. Tosite, jonka kirjaat itse seuraavan vuoden kaudelle, näkyy
  seuraavan vuoden tuloksessa.
- Virallinen tilinpäätösasiakirja liitetietojen sanallisine osineen,
  allekirjoitukset, ilmoitus kaupparekisteriin ja veroilmoitus tehdään
  aggountantin ulkopuolella.

## Yrityskoti ja käyttöönotto

- Varmuuskopio- tai vientikomentoa ei ole. Varmuuskopio on tavallinen
  hakemiston kopio:
  [Varmuuskopioi ja palauta kirjanpito](../tasks/back-up-and-restore.md)
  (`home-backup-coverage`).
- Monivuotisen historian tuonti ei ole valmis. Käyttöönotto alkaa avaavista
  saldoista, ja vanhan järjestelmän historia säilytetään arkistona:
  [Siirrä kirjanpito toisesta ohjelmasta](../start/move-from-another-program.md)
  (`historical-import`).
- [Kodin päivitys](../tasks/upgrade.md) ei korjaa eikä korvaa tiedostoa,
  jonka se jätti ennalleen. Jos ohjelman mukana ei tule ALV-rooleja tai
  maksutilien rooleja kodin tilikartalle, päivitys ei myöskään voi lisätä
  niitä (`unavailable`). Älä kopioi tiedostoja käsin, vaan pyydä apua
  projektin [issue-seurannasta](https://github.com/aggountant/aggountant/issues).
- Yrityksen omaa tilikarttaa ei voi tuoda. Kirjanpito käyttää
  oletustilikarttaa, ja vanhan järjestelmän tilit muunnetaan sille avaavassa
  tositteessa (`sie4-chart-import`).
- Testiympäristön muuttuja voi merkitä ilmoituksen tehdyksi.
  `AGGOUNTANT_BROKER_SCRIPT` on tarkoitettu vain ohjelman testaukseen. Jos se
  on asetettuna, `file-vat` voi merkitä kauden ilmoitetuksi, vaikka mitään ei
  lähetetty. Tällaisessa tulosteessa on kenttä `test_seam`. Varmista, ettei
  muuttuja ole asetettuna, kun teet oikeaa ilmoitusta.

## Katso myös

- [Onko aggountant sinulle?](../start/is-it-for-you.md)
- [Kun jokin menee vikaan](../troubleshooting.md)
