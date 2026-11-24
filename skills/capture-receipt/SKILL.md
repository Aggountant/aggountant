---
name: capture-receipt
description: >-
  Hoida yrityksen kuukauden kirjanpito: kirjaa kuitit ja ostolaskut, tuo pankin
  tiliote, kirjaa myyntilaskut ja muut erikoiskirjaukset, täsmäytä ja sulje
  kuukausi. Myös yksittäinen kuitti tai tositenippu.
cli_version: "0.3.0"
schema_version: 1
---

# Kuukauden kirjanpito

Lue ensin [SOUL.md](../aggountant-init/SOUL.md): roolisi, kirjanpitokodin
pelisäännöt, se mikä tässä työssä on pysyvää ja miten työkalua luetaan
(`aggountant skill print aggountant-init --file SOUL.md`).

Kuukausi on valmis, kun jokainen tapahtuma on sinetöity tositteeksi lähde
liitteenä tai nimetty avoimeksi kodin `TODO.md`-tiedostoon niin, että käyttäjä
tietää mitä siitä puuttuu, pankkitili täsmää tiliotteeseen, kuukausi on suljettu
ja käyttäjä tietää, mitä ilmoituksia siitä seuraa. Yksittäinen kuitti tai
tositenippu kirjataan samoin kesken kuukauden.

Kuukaudessa on kymmeniä tapahtumia. Tuo kysymykset ja hyväksyttävät kirjaukset
käyttäjälle koottuina, ei kuitti kerrallaan. Erän hyväksyntä tarkoittaa
jokaisen rivin olennaisia tietoja (päivä, vastapuoli, summa, tili, ALV), ei
lukumäärää.

## Aineisto

Mistä kuitit ja tiliote tulevat ja minne käsitelty alkuperäinen menee, on kodin
`AGENTS.md`-tiedostossa. Miten yritys on käsitellyt vastaavat tapahtumat
aiemmin, näkyy komennoilla `aggountant voucher list` ja
`aggountant voucher show`. Arkistoi lähdetiedosto komennolla
`aggountant artifact attach` ennen kirjausta ja anna sen viite tositteelle.
Silloin tosite kantaa todisteensa, ja `voucher list --attachment-ref` paljastaa
myöhemmin, onko sama tiedosto jo kirjattu.

## Kuitit ja ostolaskut

Tavallinen osto kirjataan komennolla `aggountant capture-receipt record`.
Ohjelma johtaa siitä tasapainoisen kirjauksen ALV-vienteineen tilikartan
ALV-rooleista ja päivätystä verokantataulukosta, ja sinetöity käsittely kantaa
ALV-ilmoituksen tarvitsemat tiedot. Käsin koottu tosite ei ole tavallisen oston
reitti eikä kiertotie ostolle, josta tämä komento kieltäytyy.

ALV-käsittely on aina nimenomainen väite, koska se on ilmoituksen rahaa.

- Kanta valitaan kuitin tai laskun päivästä, ei pankin kirjauspäivästä. Kannat
  ovat muuttuneet: yleinen kanta on 25,5 % 1.9.2024 alkaen ja alennettu 14 %:n
  kanta laski 13,5 %:iin 1.1.2026.
- Kun ratkaiseva tieto puuttuu, käsittely on `clarification-required`, ja kun
  osto ei kuulu tälle reitille (osittainen vähennysoikeus, maahantuonti), se on
  `unsupported`. Ohjelma kieltäytyy kummastakin, ja juuri se on tarkoitus:
  kysymys menee käyttäjälle ja osto jää avoimeksi. Tuntemattoman ALV:n
  merkitseminen `no-vat`iksi olisi hiljaa väärää kirjanpitoa, jota kukaan ei
  huomaa ennen ilmoitusta.

`natural_key` johdetaan lähteestä, jotta sama kuitti tunnistetaan myöhemmin.
Avaimen vaihtaminen kieltäytymisen kiertämiseksi tuottaa kahdennetun tositteen.

### Vastatili: kun osto näkyy myös tiliotteella

Yrityksen tililtä tai kortilla maksettu osto näkyy kahdessa lähteessä, kuitissa
ja tiliotteella, ja tiliotteen tuonti vaatii kirjauksen jokaiselle tiliotteen
vahvistetulle riville. Siksi kumpikin kirjataan kerran ja kumpikin kirjaa eri
asian: kuitti kirjaa kulun ja ALV:n, tiliotteen rivi kirjaa maksun. Ne kohtaavat
velkatilillä (numerot ovat oletustilikartan; yrityksen omassa kartassa käytetään
vastaavia tilejä):

| Miten osto maksettiin | Kuitin `contra_account` | Tiliotteen rivi |
|---|---|---|
| yrityksen tililtä tai kortilla | `2700` Ostovelat | `2700`, ei ALV:tä |
| omistajan omalla rahalla | `2740` Muut lyhytaikaiset velat | takaisinmaksu `2740`, ei ALV:tä |
| käteisellä yrityksen kassasta, eikä tiliotteella | `1900` Käteisvarat | ei riviä |

Pankkitili ei ole kuitin vastatili: jos kuitti kirjataan `1910`:tä vastaan ja
rivi tuonnissa kuluksi, sama osto on kirjanpidossa kahdesti.

Järjestys on vapaa: jos tiliote tuodaan ennen kuin kuitti löytyy, rivi
kirjataan silti `2700`:aa vastaan ja myöhempi kuitti kuittaa sen. Ennen kuin
kirjaat kuitin, tarkista maksupäivän ympäriltä, onko sen maksurivi jo sinetöity
suoraan kuluksi. Jos on, kuittia ei kirjata uudelleen: arkistoi se, vertaa
sinetöidyn tositteen tiliä ja ALV:tä siihen ja kerro käyttäjälle, että kulu on
kirjanpidossa ilman kuittia liitteenä. Jos käsittely oli väärä, se korjataan
uudella tositteella. Rivi, jonka ainoa lähde on tiliote itse (esimerkiksi pankin
palvelumaksu), kirjataan tuonnissa suoraan kulutilille.

## Tiliote

`aggountant import camt053 parse` listaa tiliotteen rivit, ja
`aggountant import camt053 seal` sinetöi ne antamillasi kohdistuksilla, joiden
muoto on sen `--help`-ohjeessa. Jokainen rivi tarvitsee kohdistuksen: yksikin
puuttuva hylkää koko tiliotteen. Maksu, jonka kuitti puuttuu, kirjataan siksi
`2700`:lle, jossa se pysyy näkyvissä. Rivi, jonka luonnetta et tiedä (tuntematon
suoritus tilille), kysytään käyttäjältä ennen tuontia, ja tuonti odottaa
vastausta: arvattu tili olisi sinetöity arvaus.

Katso `parse`n tuloksesta ennen sinetöintiä, että tili ja jakso
(`statements[].period`) ovat sen tiliotteen, jota olit tuomassa. Ohjelma tuo
väärän kuukauden tiliotteen ja tyhjän tiedoston yhtä lailla onnistuneesti, joten
nolla riviä kelpaa vain, kun tiliote itse näyttää, ettei tapahtumia ollut.
`balances_reconcile: true` kertoo, että luetut rivit vievät pankin alkusaldon
sen loppusaldoon. `false` tarkoittaa, että ohjelma ei lukenut tiedostoa niin
kuin pankki sen tarkoitti: älä tuo sitä, vaan kerro käyttäjälle. `null` on
tiliote, jossa ei ole alku- tai loppusaldoa.

Aja `seal` ensin `--dry-run`-valitsimella. Se tarkistaa kohdistukset samoin kuin
sinetöinti (tilit, summat, ALV-purku, kauden tila) kirjoittamatta mitään ja
palauttaa sinetöitävät tositteet ja `preparation_hash`-tiivisteen. Näytä
tositteet käyttäjälle rivi riviltä. Hyväksynnän jälkeen sinetöi samalla
tiedostolla ja syötteellä sekä `--approved-preparation-hash`-valitsimella. Jos
jokin muuttui esikatselun jälkeen, sinetöinti hylätään
(`IMPORT_PREPARATION_CHANGED`) ennen kuin mitään kirjoitetaan; tee silloin uusi
esikatselu ja kysy hyväksyntä uudelleen.

Päällekkäisen tiliotteen jo kirjattu tapahtuma tunnistetaan pankin
tapahtumatunnisteesta (`outcome: duplicate_source_observation`, tositteen numero
`voucher_number`), eikä sitä kirjata uudelleen. Sekin tarvitsee kohdistuksen, ja
sen on oltava sama, jolla tosite sinetöitiin: katso se `voucher show`lla.
`POSTING_PROJECTION_CHANGED` tarkoittaa, että kohdistus poikkeaa, ja
`SOURCE_FACTS_CHANGED`, että pankki on muuttanut jo kirjatun tapahtuman tietoja;
molemmat pysäyttävät koko tiliotteen, ja virheen `details.conflicts` luettelee
kaikki. Muuttunutta pankkitapahtumaa ei kohdisteta uudelleen arvaamalla, vaan
kysytään käyttäjältä.

`BANK_ENTRY_POSSIBLY_BOOKED` tarkoittaa, että tuonnin ulkopuolella (esimerkiksi
`voucher seal`illa) kirjattu tosite saattaa jo kirjata uuden rivin; virhe nimeää
tositteen. Näytä se käyttäjälle (`voucher show`) ja kysy. Jos se kirjaa rivin,
siirrä rivi kohdistuksista `existing_vouchers`-listaan tositteen numerolla:
tuonti sitoo rivin tositteeseen sinetöimättä uutta, ja myöhemmät tiliotteet
tarvitsevat saman sidonnan. Jos ei, lisää numero kohdistuksen
`not_booked_by`-listaan. Kumpaakaan ei valita arvaamalla: väärä valinta kirjaa
rahan kahdesti tai jättää sen kirjaamatta.

- Myynnin ALV:tä tuonti ei pura. Myyntilasku kirjataan erikoiskirjauksena ja
  asiakkaan maksu tiliotteelta myyntisaamisia vastaan.
- ALV:n maksu tai palautus kirjataan ALV-tilille niin, että vastatilinä on vain
  pankkitili. Muu tase-erä samalla tositteella estää ALV-täsmäytystä
  tunnistamasta sitä maksuksi.
- Vain Holvin tiliote on koeteltu (11/2026). Muun pankin ensimmäisessä
  tuonnissa `balances_reconcile` ja saldon täsmäytys ovat ainoa näyttö siitä,
  että tuonti luki tiedoston oikein.

## Erikoiskirjaukset ja niiden ALV-merkintä

Myyntilaskuille, palkkatositteille ja korjauksille ei ole omaa reittiä. Ne
valmistellaan itse: `aggountant voucher draft`, `voucher validate` ja
`voucher seal`. Tarkista
`validate`lla sama tiedosto, jonka sinetöit, ja näytä käyttäjälle viennit ennen
sinetöintiä, sillä tässä ohjelma ei laske ALV-rivejä puolestasi.

ALV-ilmoitus näkee tällaisen tositteen veron vain `vat_code`-merkinnästä, ei
tilinumerosta, joten merkitsemätön vero puuttuu ilmoituksesta. Koodin paikka ja
muoto ovat `voucher seal --help`issä, ja `voucher vat-codes --document-date`
listaa tositteen päivänä sallitut koodit.

Jos `voucher validate` varoittaa merkitsemättömästä ALV-viennistä, korjaa luonnos
ennen hyväksyntää, ellei vienti ole ALV:n maksu tai muu ilmoitukseen kuulumaton
siirto. Varoitus ei estä sinetöintiä; sinetöidyn puutteen löytää täsmäytyksen
ALV-tarkistus (alla) ennen sulkua.

## Täsmäytys

`aggountant report statements` näyttää kuukauden tuloksen ja kuun lopun taseen.
Kaksi lukua paljastaa tavallisimmat virheet:

- Pankkitilin saldo on tiliotteen loppusaldo, jonka `parse` ja esikatselu
  näyttävät kohdassa `statements[].closing_balance`.
- `2700`:n saldo on vielä maksamattomien laskujen summa. Ylimääräinen saldo,
  kun pankki täsmää, on merkki ostosta, joka on kirjattu kuitista ja lisäksi
  tiliotteen riviltä kuluksi. Debet-saldo on maksu, jonka kuittia ei ole vielä
  kirjattu, eli avoin kohta.

Ohjelma ei tarkista kumpaakaan eikä yhdistä kuittia sen maksuriviin, ja kaksi
vastakkaista virhettä kumoaa toisensa saldossa. Selitä siksi `2700`:n saldo
laskuina, älä vain summana, ja nosta erot käyttäjälle ennen sulkua.

Uuden tilikauden kuukaudelle `report statements` torjuu koodilla
`PERIOD_NOT_SEALED`, kunnes edellinen tilikausi on sinetöity `close-year`-ajolla,
koska tase alkaa sen loppusaldoista. Se ei ole kirjausvirhe: kirjaa, täsmäytä
pankki tiliotteeseen tositteista ja sulje kuukaudet silti, ja kerro
käyttäjälle, että raportti odottaa tilinpäätöstä.

ALV-tilit täsmäytetään ilmoitukseen ennen sulkua komennolla
`aggountant file-vat preview --period <kausi>`. Avoimelle kaudelle se on luonnos
(`fileable: false`, huomiokohta `VAT_PERIOD_NOT_SEALED`), mutta täsmäytys on sama
kuin suljetulla. Jos `vat_reconciliation.status` on `unreconciled`, käy
`VAT_ACCOUNTS_UNRECONCILED`-kohdan tositteet läpi lähdettä vasten. Ilmoitukseen
kuuluva vero korjataan nyt korjaavalla tositteella samalle kaudelle, ALV-koodi
veron perusteen rivillä; sulun jälkeen sama korjaus onnistuu vain OmaVerossa
käsin. Ilmoitukseen kuulumattomat viennit, kuten ALV:n maksu, jäävät ja ne
kuitataan ilmoitusta lähetettäessä (`vero-fi`).

## Sulku

Sulku lukitsee kauden kirjaukset pysyvästi, ja ALV-ilmoitus tehdään sen
luvuista. Suljettava kausi on yrityksen ALV-kausi: neljännesvuosittain
ilmoittava sulkee neljänneksen, eikä samana vuonna voi sulkea sekä kuukausia
että neljänneksiä. Myöhemmin löytynyt virhe korjataan tositteella seuraavalle
avoimelle kaudelle; jos virhe muuttaa suljetun kauden ALV:tä, `vero-fi` kertoo
ensin, miten ilmoitus korjataan.

`aggountant run preview close-period` näyttää sulun sisällön muuttamatta
mitään. `aggountant run start close-period` sen sijaan siirtää kuukauden heti
suljettavaksi, eikä sille voi sen jälkeen kirjata. Aloita se vasta, kun
täsmäytys on tehty, avoimia velvoitteita ei ole (avoin velvoite estää sulun) ja
käyttäjä on päättänyt, mitä `TODO.md`:n avoimille kohdille tehdään.

Hyväksyntäportti näyttää, mitä hyväksyntä lukitsee, ja sen tiiviste sitoo juuri
sen sisällön. Näytä se käyttäjälle ja lisää siihen, mitä ohjelma ei kokoa
porttiin: täsmääkö pankkitili tiliotteeseen, ALV-täsmäytyksen tila,
selittyvätkö ostovelat ja mitä jää avoimeksi. Vastaa porttiin komennolla
`aggountant run answer` vasta hänen vastauksensa jälkeen.
Avoin kohta ei estä sulkua, jos käyttäjä päättää niin tietäen, että se jää pois
tämän kauden luvuista ja ilmoituksesta. Hylkäys palauttaa kuukauden avoimeksi
eikä poista mitään, joten se on oikea vastaus, kun kuukaudelle halutaan vielä
kirjata. `aggountant period show` kertoo tilan ja suljettavaksi jääneen
kuukauden ajon: siihen vastataan, uutta ei aloiteta.

## Ilmoitukset

Suljetusta kaudesta annetaan ALV-ilmoitus, ja työnantajarekisterissä oleva
yritys antaa Tulorekisteriin erillisilmoituksen joka kuukausi, myös ilman
maksuja. ALV-ilmoitus on `vero-fi`-skillin ja erillisilmoitus
`tulorekisteri`-skillin työtä.
