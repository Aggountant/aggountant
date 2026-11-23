# Harjoittele ensimmäinen kuukausi

Kirjaat kuvitteellisen Esimerkki Oy:n maaliskuun 2026 alusta loppuun: kirjaat
ostolaskun, tuot pankin tiliotteen, kirjaat kuitin, katsot kuukauden raportin,
suljet kuukauden ja katsot ALV-ilmoituksen luvut. Samalla näet, mitä agentti
tekee itse, mitä se kysyy sinulta ja mitä sinä hyväksyt. Harjoitus kestää noin
puoli tuntia.

Harjoittelet erillisessä harjoituskodissa, joka ei liity yrityksesi oikeaan
kirjanpitoon. Sinetöityä tositetta ei voi muuttaa eikä suljettua kuukautta
avata, joten niitä ei kannata kokeilla ensimmäistä kertaa oikeassa
kirjanpidossa. Harjoituskodin voit lopuksi poistaa kokonaan.

## Ennen kuin aloitat

- [Asenna aggountant ja skillit](install.md).
- Käynnistä agentti, esimerkiksi Claude Code, päätteessä.
- Kopioi harjoitusaineisto hakemistosta [`docs/user/practice/`](../../practice/README.md)
  johonkin kansioon, esimerkiksi `~/aggountant-harjoitus-aineisto/`. Voit myös
  pyytää agenttia hakemaan sen GitHubista. Aineistossa on kolme tiedostoa:
  - `tiliote-2026-03.xml`: Esimerkki Oy:n pankkitiliote maaliskuulta
  - `lasku-pilvipalvelu-2026-03.txt`: Pilvipalvelu Oy:n lasku palvelinvuokrasta
  - `kuitti-toimistotarvike-2026-03.txt`: kuitti toimistotarvikkeista.

  Lasku ja kuitti ovat tekstitiedostoja, jotta voit lukea ne suoraan. Oikeat
  kuitit ovat yleensä PDF-tiedostoja tai kuvia, ja agentti käsittelee ne samalla
  tavalla.

Alla olevat pyynnöt agentille ovat esimerkkejä, ja voit sanoa saman omin
sanoin. Agentin vastaukset vaihtelevat kerrasta toiseen, mutta aggountantin omat
tulosteet ovat samat kuin tällä sivulla.

## 1. Perusta harjoituskoti

Sano agentille:

> Perusta harjoituskoti hakemistoon ~/aggountant-harjoitus. Tämä on vain
> harjoittelua varten: käytä jokaisessa komennossa valitsinta
> --home ~/aggountant-harjoitus äläkä tee yrityksen käyttöönottoa.

Agentti ajaa `aggountant --home ~/aggountant-harjoitus init`, ja aggountant
vastaa:

```
initialized /…/aggountant-harjoitus — chart_version fi-general-2026@…, 98 accounts
```

Harjoituskodissa on nyt oletustilikartta, jossa on 98 tiliä, ja tyhjä
tositevarasto.

Valitsin `--home` on mukana jokaisessa komennossa, koska se menee kaiken muun
edelle. Vaikka olisit jo määrittänyt oikean kirjanpitosi kodin asetuksiin,
harjoitus ei osu siihen vahingossa. Jos epäilet, pyydä agenttia ajamaan
`aggountant config show`. Se kertoo, mitä kotia ohjelma käyttää ja mistä tieto
tuli.

## 2. Kirjaa ostolasku

Pilvipalvelu Oy laskutti 11.3. maaliskuun palvelinvuokran, 124,00 €, ja
yritys maksoi laskun tililtään 14.3. Sama osto näkyy siis sekä laskussa että
tiliotteella, ja kumpikin kirjataan kerran: lasku kirjaa kulun ja ALV:n,
tiliotteen rivi maksun. Sano agentille:

> Kirjaa harjoitusaineiston lasku lasku-pilvipalvelu-2026-03.txt. Se on
> maksettu yrityksen tililtä.

Agentti arkistoi laskun ja näyttää aggountantin esikatselun:

| Tili | Debet | Kredit |
|---|---|---|
| 7690 Atk-laite- ja ohjelmistokulut | 98,80 € | |
| 1763 Arvonlisäverosaaminen | 25,20 € | |
| 2700 Ostovelat | | 124,00 € |

Esikatselu ei tallenna mitään. Vastatili on ostovelka eikä pankkitili, koska
maksu kirjataan seuraavassa vaiheessa tiliotteelta. ALV-rivin ohjelma muodosti
itse verokantataulukosta laskun päivän mukaan. Tarkista, että summa ja päivä
vastaavat laskua ja että tili ja ALV-käsittely ovat oikeat. Kun hyväksyt,
agentti sinetöi tositteen juuri tällaisena:

```
sealed receipt voucher #1 (replayed=false)
```

Hyväksyntä on sidottu siihen, mitä sinulle näytettiin. Jos laskun tiedot,
ehdotus tai tilikartta muuttuvat esikatselun ja sinetöinnin välillä, ohjelma
kieltäytyy ja agentti näyttää uuden esikatselun. Syyn kertoo sivu
[Sinetöinti ja hyväksyntä](../concepts/sealing-and-approval.md).

Pyydä agenttia kirjaamaan sama lasku uudelleen. Ohjelma palauttaa saman
tositteen numero 1 (`replayed=true`) eikä tee uutta, joten sama lasku ei päädy
kirjanpitoon kahdesti vahingossa.

## 3. Tuo pankin tiliote

Sano agentille:

> Tuo harjoitusaineiston tiliote tiliote-2026-03.xml ja ehdota kirjaukset.

Agentti lukee tiliotteen. Lukeminen ei vielä kirjaa mitään:

```
parsed "FI2112345600000785" — 2 transactions
```

Tiliotteella on kolme riviä, mutta aggountant ottaa niistä vain kaksi. Kolmas,
31.3. tehty 50 euron korttivaraus, on pankissa vielä vahvistamatta, ja ohjelma
ottaa sen mukaan vasta, kun se on toteutunut. Ohjelma näyttää agentille myös
tiliotteen jakson (1.–31.3.), alkusaldon 0,00 € ja loppusaldon 2 376,00 € sekä
sen, että kaksi luettua tapahtumaa vievät alkusaldon loppusaldoon.

Agentti tunnistaa 14.3. rivin juuri kirjaamasi laskun maksuksi. Toisesta
rivistä se ei voi tietää kaikkea, joten se kysyy, esimerkiksi:

> 20.3. tilille on tullut 2 500,00 € Maija Meikäläiseltä viestillä
> "Osakepääoma". Onko tämä osakepääoman maksu?

Vastaa:

> Kyllä, maksoin osakepääoman yhtiötä perustettaessa.

Sitten agentti näyttää ehdotuksensa koko tiliotteesta, jokainen rivi
erikseen:

| Päivä | Vastapuoli | Summa | Kirjaus | ALV |
|---|---|---|---|---|
| 14.3. | Pilvipalvelu Oy | −124,00 € | 2700 Ostovelat (tositteen 1 maksu) | ei ALV:tä |
| 20.3. | Maija Meikäläinen | +2 500,00 € | 2000 Osake- tai osuuspääoma | ei ALV:tä |

Tarkista ennen kuin hyväksyt, että tilit ovat oikeat ja ettei mikään osto tule
kirjatuksi kahdesti. Agentti ehdottaa laskun maksurivin ostovelkaa vastaan
ilman ALV:tä, koska kulu ja ALV ovat jo tositteella 1. Jos rivi kirjattaisiin
kuluksi, palvelinvuokra olisi kirjanpidossa kahdesti. Kun kaikki on kunnossa,
sano:

> Hyväksyn, kirjaa.

Lista, jonka hyväksyit, on ohjelman esikatselu. Agentti ajoi tuonnin ensin
kirjaamatta mitään, ja ohjelma tarkisti tilit ja summat ja antoi esikatselulle
tunnisteen. Sillä tunnisteella agentti sinetöi koko tiliotteen kerralla, ja
ohjelma vastaa:

```
sealed 2 new vouchers from "FI2112345600000785", 0 transactions already in the books (attachment "…")
```

Kirjanpidossa on nyt kolme tositetta. Ohjelma arkistoi alkuperäisen
tiliotetiedoston ja liitti sen kumpaankin tiliotteen tositteeseen, ja lasku on
tositteen 1 liitteenä. Maksu kuittasi laskun ostovelan, joten ostovelkojen
saldo on taas nolla.

Kokeile vielä tuoda sama tiliote uudelleen:

> Tuo sama tiliote vielä kerran samoilla kirjauksilla.

Ohjelma tunnistaa jo sinetöidyt tapahtumat eikä kirjaa niitä toista kertaa. Se
vastaa `sealed 0 new vouchers` ja kertoo, että kaksi tapahtumaa on jo
kirjanpidossa. Tositteita on edelleen kolme, minkä näet myös ketjun
tarkistuksesta (`aggountant voucher verify`).

## 4. Kirjaa kuitti

Maija osti 22.3. tulostinpaperia ja kirjekuoria 31,00 eurolla ja maksoi omalla
kortillaan, joten ostoa ei ole yrityksen tiliotteella. Sano agentille:

> Kirjaa kuitti kuitti-toimistotarvike-2026-03.txt.

Agentti arkistoi kuitin, huomaa, että ostoa ei löydy tiliotteelta, ja kysyy,
miten osto maksettiin. Kerro:

> Maksoin sen omalla kortillani.

Agentti ehdottaa, että osto kirjataan yrityksen velaksi sinulle, ja näyttää
aggountantin esikatselun:

| Tili | Debet | Kredit |
|---|---|---|
| 7700 Toimistokulut | 24,70 € | |
| 1763 Arvonlisäverosaaminen | 6,30 € | |
| 2740 Muut lyhytaikaiset velat | | 31,00 € |

Kun hyväksyt, agentti sinetöi tositteen:

```
sealed receipt voucher #4 (replayed=false)
```

Velka sinulle jää kirjanpitoon, kunnes yritys maksaa sen takaisin. Takaisinmaksu
näkyy silloin tiliotteella, ja agentti kirjaa sen tiliä 2740 vastaan samalla
tavalla kuin laskun maksun ostovelkaa vastaan.

## 5. Katso, miten kuukausi meni

Sano agentille:

> Tee maaliskuusta kuukausiraportti HTML-muodossa.

Agentti ajaa `report statements`, ja saat tiedoston, jonka avaat selaimessa.
Raportissa näkyy:

- kuukauden tulos −123,50 € (palvelinvuokra 98,80 € ja toimistokulut 24,70 €)
- Pankkitili 2 376,00 €
- Ostovelat nolla
- taseen loppusummat 2 407,50 € kummallakin puolella.

Vertaa pankkitilin saldoa tiliotteen loppusaldoon. Kumpikin on 2 376,00 €, joten
jokainen tiliotteen tapahtuma on kirjattu. Tämä on tärkein tarkistus ennen
kuukauden sulkua. Ostovelkojen nolla kertoo, että tiliotteen maksu kuittasi
laskun eikä palvelinvuokra ole kirjanpidossa kahdesti. Jos maksurivi olisi
kirjattu kuluksi, pankkitilin saldo täsmäisi silti, mutta ostovelkoihin jäisi
124,00 €.

Raportin alussa lukee "Kattavuus: varmistamaton", koska aggountant ei voi
tietää, onko kaikki aineisto kirjattu. Sen arvioit sinä agentin kanssa lähteitä
vasten, kuten juuri teit.

## 6. Sulje kuukausi

Sano agentille:

> Sulje maaliskuu.

Agentti tarkistaa ensin, että jokainen tiliotteen tapahtuma on kirjattu, lasku
ja kuitti on liitetty, pankkitilin saldo täsmää ja avoimia velvoitteita ei
ole. Sitten se aloittaa sulun:

```
run "cp-…" — "awaiting_gate" ["2026-03"]
gate ApproveClose (bound true)
period 2026-03 (2026-03-01..2026-03-31): … voucher(s) #1..#…
```

Kuukausi on nyt tilassa `closing`. Siihen ei voi enää kirjata, ja sulku odottaa
hyväksyntääsi. Agentti näyttää, mitä hyväksyntä lukitsee: tositteet, kuukauden
tuloksen ja tilien saldot maaliskuun lopussa. Se kertoo myös, että kuukausi
lukitaan pysyvästi ja että myöhemmin huomatut korjaukset kirjataan seuraavalle
kuukaudelle. Sano:

> Hyväksyn sulun.

```
run "cp-…" — "completed" ["2026-03"]
```

Maaliskuu on suljettu (`sealed`). Kokeile vielä kirjata siihen jotain:

> Kirjaa sama toimistotarvikekuitti uudelleen uudella tunnisteella, niin
> näen mitä tapahtuu.

Esikatselu onnistuu, mutta sinetöinti ei:

```
PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period
```

Kirjanpitolaki edellyttää, että suljettua kautta ei muuteta ja korjaus
kirjataan avoimelle kaudelle. Jos huomaat puutteen jo ennen kuin hyväksyt
sulun, hylkää sulku, niin
[kuukausi palaa avoimeksi](../concepts/periods.md#kauden-tilat).

## 7. Katso ALV-ilmoituksen luvut

Sano agentille:

> Näytä maaliskuun ALV-ilmoituksen luvut.

Agentti ajaa `file-vat preview`. Esikatselu lukee vain suljetun kuukauden
kirjanpitoa eikä ota yhteyttä minnekään:

| Kenttä | Summa |
|---|---|
| 307 Kohdekauden vähennettävä vero | 31,50 € |
| 308 Maksettava tai palautukseen oikeuttava vero | −31,50 € |
| Maksettavaa | 0,00 € |

Vähennettävä vero 31,50 € on palvelinvuokran 25,20 € ja kuitin 6,30 € summa.
Myyntiä ei ollut, joten maksettavaa ei ole.

Harjoituksessa et lähetä mitään. Oikeassa kirjanpidossa syötät nämä kentät
OmaVeroon tai annat agentin lähettää ilmoituksen suoraan, jos
[suora ilmoittaminen](../tasks/set-up-vero-api.md) on käytössä.

## Mitä opit

- Agentti tekee itse sen, mihin aineisto riittää, ja kysyy sinulta vain sen,
  mitä se ei voi tietää, kuten mistä 2 500 euroa tuli tai millä kuitti maksettiin.
- Mikään ei tallennu ennen kuin olet hyväksynyt sen, ja hyväksyt aina sen, mitä
  sinulle näytettiin.
- Sinetöity tosite ja suljettu kuukausi ovat pysyviä. Virhe korjataan uudella
  tositteella seuraavalla kaudella.
- Kun osto näkyy sekä laskussa että tiliotteella, lasku kirjaa kulun
  ostovelkaa vastaan ja tiliotteen rivi kirjaa maksun samaa velkaa vastaan.
- Pankkitilin saldon vertaaminen tiliotteeseen kertoo, onko kuukausi kirjattu
  kokonaan, ja ostovelkojen saldo, onko jokin osto kirjattu kahdesti.
- ALV lasketaan suljetun kuukauden kirjauksista, ja kanta valitaan laskun tai
  kuitin päivästä.

## Seuraavaksi

Harjoituskoti on pelkkä hakemisto `~/aggountant-harjoitus`, ja voit poistaa
sen, kun et enää tarvitse sitä. Oikean kirjanpidon aloitat sivulta
[Ota yrityksesi kirjanpito käyttöön](set-up-company.md).
