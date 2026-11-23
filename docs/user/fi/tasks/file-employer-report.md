# Anna työnantajan erillisilmoitus

Kun yrityksesi on Verohallinnon työnantajarekisterissä, annat Tulorekisteriin
joka kuukausi työnantajan erillisilmoituksen, myös kuukausilta, joilta et
maksanut mitään. Määräpäivä on seuraavan kuukauden 5. päivä. Satunnaisesti
palkkaa maksava yritys, joka ei ole rekisterissä, ei anna erillisilmoitusta
kuukausilta, joilta se ei maksanut mitään.

aggountant lähettää ilmoituksen samalla Tulorekisterin tuotantoyhteydellä kuin
matkakorvaukset ja kertoo, mitkä kuukaudet puuttuvat. Matkakorvausten
palkkatietoilmoitus on eri ilmoitus eikä korvaa erillisilmoitusta.

## Ennen kuin aloitat

- [Tulorekisterin rajapintaoikeus ja profiili](report-travel-allowances.md#ota-ilmoittaminen-käyttöön-kerran)
  ovat kunnossa. Erillisilmoitus käyttää samaa oikeutta ja tunnustiedostoa
  kuin matkakorvaukset.
- Tiedät, mistä kuukaudesta alkaen yritys on ollut työnantajarekisterissä.
  Näet sen OmaVerosta tai Verohallinnon päätöksestä, tai agentti
  [tarkistaa sen YTJ:stä](#tarkista-rekisteröinnin-alku-ytjstä).
- Jos yrityksellä on työeläkevakuutus, tiedät eläkeyhtiön ja vakuutusnumeron.
  Jos yrityksessä on vain YEL-vakuutettu omistaja, näitä ei tarvita.

## Ota erillisilmoitus käyttöön (kerran)

1. **Pyydä agenttia lisäämään erillisilmoituksen asetukset.**

   > Yritys on ollut työnantajarekisterissä elokuusta 2026. Lisätään
   > työnantajan erillisilmoitus. Yhteyshenkilö on Maija Meikäläinen,
   > puhelin 040 123 4567. Työeläkevakuutusta ei ole, omistajalla on YEL.

2. **Agentti lisää asetukset Tulorekisteri-profiiliin:** rekisteröinnin
   alkukuukauden, yhteyshenkilön ja työeläkevakuutukset. Ohjelma tarkistaa,
   että puhelinnumero, eläkelaitosten tunnukset ja vakuutusnumerot ovat
   muodossa, jonka ilmoitus hyväksyy, ja vertaa alkukuukautta
   [YTJ:n työnantajarekisteriin](#tarkista-rekisteröinnin-alku-ytjstä).

3. **Tarkista asetukset, jotka agentti näyttää.** Ne lähtevät ilmoituksen
   mukana, ja alkukuukaudesta lähtien jokaiselta kuukaudelta odotetaan
   ilmoitusta.

Jos rekisterissä on eri alkukuukausi kuin annoit, ohjelma ei tallenna
asetuksia. Agentti näyttää vertailun ja tallentaa sinun kuukautesi vain, jos
vahvistat sen (`EMPLOYER_REGISTER_MISMATCH`). Jos YTJ ei vastaa
(`YTJ_UNAVAILABLE`), ohjelma ei tallenna mitään, ellette sovi agentin kanssa
tallentamisesta ilman tarkistusta.

<details>
<summary>Komento, jonka agentti ajaa</summary>

```sh
aggountant file-wage employer-report configure --registered-from 2026-08 \
  --contact-name "Maija Meikäläinen" --contact-phone "040 123 4567" \
  --no-pension-insurance --dry-run
```

Työeläkevakuutus annetaan muodossa
`--pension-insurance ELÄKELAITOSTUNNUS:VAKUUTUSNUMERO`, ja valitsimen voi
toistaa. Jos vakuutuksia ei ole, se sanotaan erikseen valitsimella
`--no-pension-insurance`. Samat asetukset voi antaa JSONina vakiosyötteessä.

`--accept-register-mismatch` tallentaa kuukauden, josta YTJ on eri mieltä, ja
`--offline` tallentaa ilman YTJ-tarkistusta.

Komento muuttaa profiilista vain kohdan `employer_report` (`registered_from`,
`contact_name`, `contact_phone`, `pension_insurances`) ja näyttää sen ennen ja
jälkeen.
</details>

### Tarkista rekisteröinnin alku YTJ:stä

Jos et tiedä, milloin yritys tuli työnantajarekisteriin, pyydä agenttia
tarkistamaan:

> Milloin yritys tuli työnantajarekisteriin?

Ohjelma hakee yrityksen työnantajarekisterimerkinnät PRH:n YTJ-avoimesta
datasta ja vertaa niitä profiilin alkukuukauteen. Kyselyssä lähtee vain
Y-tunnus, eikä siihen tarvita tunnuksia. Tulos kertoo jonkin näistä:

- alkukuukausi täsmää
- profiili alkaa liian myöhään, jolloin vanhoja kuukausia jää pois puuttuvien
  luettelosta
- profiili alkaa liian aikaisin
- rekisterimerkintä on päättynyt
- yritystä ei ole rekisterissä.

Myös aiemmat rekisterijaksot näkyvät. Erillisilmoituksia on annettu
tammikuusta 2019 alkaen, joten sitä vanhempi rekisteröinti tarkoittaa
alkukuukautta 2019-01.

Tarkistus ei muuta profiilia. Agentti muuttaa alkukuukauden
käyttöönottokomennolla vasta, kun hyväksyt sen.

Jos YTJ ei vastaa, tulos on "ei tiedossa" eikä "ei rekisterissä". Kokeile
myöhemmin uudelleen tai katso yritys ytj.fi:stä.

<details>
<summary>Komento, jonka agentti ajaa</summary>

```sh
aggountant file-wage employer-report registration
```
</details>

## Näin teet sen

1. **Pyydä agenttia.**

   > Annetaan lokakuun työnantajan erillisilmoitus.

2. **Agentti selvittää, mitä kuukaudella maksettiin**, ja kysyy sinulta, jos
   kirjanpidosta ei näe.
   - Jos et maksanut mitään, ilmoitukseen tulee "Ei palkanmaksua". Ohjelma ei
     hyväksy tätä valintaa kuukaudelle, jolle on tehty matkakorvausilmoitus.
   - Jos maksoit vain verovapaita matkakorvauksia (kilometrikorvauksia,
     päivärahoja), ilmoitukseen tulee työnantajan sairausvakuutusmaksu
     0,00 €. "Ei palkanmaksua" ei Tulorekisterin ohjeen mukaan ole silloin
     oikein, koska maksuja on tehty.
   - Jos maksoit palkkaa, tee ilmoitus Tulorekisterin asiointipalvelussa.
     Ohjelma
     [ei vielä laske sairausvakuutusmaksua](../reference/status.md#tulorekisteri-ja-työntekijät),
     joten se ei tee tällaisen kuukauden ilmoitusta
     (`EMPLOYER_REPORT_WAGES_UNSUPPORTED`).

3. **Agentti esikatselee ilmoituksen.** Esikatselu ei ota verkkoyhteyttä eikä
   tallenna mitään. Se näyttää kuukauden, sisällön, yhteyshenkilön, mahdolliset
   eläkevakuutukset, määräpäivän ja ilmoituksen tarkisteen.

4. **Tarkista ja hyväksy** kuukausi ja sisältö. Ilmoitus on lopullinen, kun
   Tulorekisteri on ottanut sen vastaan, ja sen jälkeen sitä korjataan
   korvaavalla ilmoituksella.

   > Hyväksyn lokakuun erillisilmoituksen.

5. **Agentti lähettää ilmoituksen** juuri hyväksymälläsi tarkisteella ja
   kertoo Tulorekisterin kuittauksen.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-wage employer-report preview --month 2026-10 \
  --payments none --timestamp 2026-11-02T08:00:00Z
aggountant file-wage employer-report submit --month 2026-10 \
  --payments none --timestamp 2026-11-02T08:00:00Z --record-hash <tarkiste>
aggountant file-wage employer-report status --month 2026-10
```

`--payments` on `none`, `tax-free-reimbursements-only` tai `wages`.
Esikatselulle ja lähetykselle annetaan samat valinnat ja sama aikaleima.
Aikaleimasta tulee myös ilmoituksen ilmoituspäivä.
</details>

## Puuttuvat kuukaudet

Pyydä agenttia tarkistamaan, puuttuuko ilmoituksia:

> Puuttuuko työnantajan erillisilmoituksia?

Ohjelma luettelee rekisteröinnin alusta edelliseen kuukauteen asti kuukaudet,
joilta sen tiedossa ei ole Tulorekisteriin annettua ilmoitusta. Jokaisesta
näkyvät määräpäivä ja se, onko määräpäivä jo mennyt. Agentti käy puuttuvat
kuukaudet läpi vanhimmasta alkaen, ja sinä hyväksyt jokaisen kuukauden
erikseen. Myöhästyneestä ilmoituksesta voi tulla myöhästymismaksu, joten
myöhästyneet kannattaa antaa heti.

Jos olet jo antanut jonkin kuukauden ilmoituksen asiointipalvelussa, älä
lähetä sitä uudelleen. Kerro agentille ilmoituksen viite asiointipalvelusta,
niin agentti merkitsee kuukauden annetuksi.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-wage employer-report missing
aggountant file-wage employer-report complete-manual --month 2026-08 \
  --ir-report-id <ilmoitusviite>
```
</details>

### Kun aiempi palkanlaskija on antanut ilmoituksia

Jos tilitoimisto tai palkkapalvelu on antanut osan kuukausista ennen
aggountantia, viitteitä ei kannata kaivaa yksitellen. Tulorekisterin raportti
630 kertoo, monellako erillisilmoituksella kukin kuukausi on ilmoitettu.
Tilaat raportin itse Tulorekisterin asiointipalvelusta, koska ohjelma ei voi
tilata sitä puolestasi.

1. **Tilaa raportti asiointipalvelussa:** Uusi tilaus → *Maksajan
   palkkatietojen yhteenveto (csv)* (630). Valitse kohdekauden aikaväliksi
   rekisteröinnin alusta alkaen, enintään 24 kuukautta kerrallaan, ja kieleksi
   **suomi**. Jätä vastaanottohetken aikaväli tyhjäksi, koska rajattu raportti
   jättää pois muulloin vastaanotetut ilmoitukset, eikä rajausta näe
   tiedostosta. Lataa valmis raportti *Tehdyt tilaukset* -sivulta. Jos lataus
   on zip-tiedosto, pura se.

2. **Anna tiedosto agentille.** Raportissa ei ole maksajan Y-tunnusta, joten
   varmista, että se on oman yrityksesi.

   > Tässä Tulorekisterin raportti 630 vuodelta 2025.

3. **Agentti tuo raportin.** Ohjelma tallentaa tiedoston ja merkitsee
   jokaisen kuukauden, jolla on vähintään yksi erillisilmoitus, annetuksi
   aggountantin ulkopuolella. Kuukausi, jolla ei ole ilmoitusta, jää
   puuttuvaksi.

Tuonti ei muuta kuukautta, jonka ohjelma jo tuntee omasta lähetyksestään tai
kirjatusta viitteestä. Jos raportti ja ohjelman tiedot ovat ristiriidassa,
esimerkiksi kun kuukausi on ohjelman mukaan annettu mutta raportissa ei ole
ilmoitusta, agentti näyttää ristiriidan. Saman tiedoston tuominen uudelleen ei
muuta mitään.

Puuttuvien kuukausien luettelossa raportin kuukaudet näkyvät erikseen,
lähteenä raportti. Tällaiselle kuukaudelle ohjelma ei lähetä uutta eikä
korvaavaa ilmoitusta, koska raportissa ei ole ilmoitusviitettä. Korjaa
ilmoitus asiointipalvelussa tai kerro agentille ensin sen viite.

Kun viite on kirjattu, ohjelma ei silti tiedä, mitä eläkevakuutustietoja sen
ulkopuolella annetussa ilmoituksessa oli, ja korvaavan ilmoituksen pitää
toistaa ne. Tietoja ei saa mistään raportista tai rajapinnasta. Avaa siksi
ennen korvaavaa ilmoitusta alkuperäinen ilmoitus asiointipalvelun lähetetyistä
ilmoituksista ja kerro agentille sen eläkevakuutukset: eläkeyhtiön tunnus ja
vakuutusnumero, tai ettei niitä ole. Ilman näitä tietoja ohjelma hylkää
korvaavan ilmoituksen (`EMPLOYER_REPORT_PREVIOUS_UNKNOWN`). Jos tiedot eroavat
nykyisistä asetuksista, ohjelma hylkää "Ei palkanmaksua" -korvaavan, ja teet
korjauksen asiointipalvelussa.

Jos kaksi tuotua raporttia on eri mieltä kuukaudesta, eli toisessa on ilmoitus
ja toisessa ei, kuukausi näkyy puuttuvien luettelossa ristiriitana. Ohjelma ei
lähetä sille mitään ennen kuin tarkistat kuukauden asiointipalvelusta ja
agentti kirjaa ilmoituksen viitteen.

Ohjelma lukee vain suomenkielisen raportin, jonka sarakkeet ovat samat kuin
asiointipalvelun tuottamassa tiedostossa. Muunlaisen tiedoston se hylkää
(`TRANSCRIPT_UNKNOWN_HEADER`) eikä tallenna mitään.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-wage employer-report import-transcript raportti-630.csv --dry-run
aggountant file-wage employer-report import-transcript raportti-630.csv
aggountant file-wage employer-report missing
aggountant file-wage employer-report complete-manual --month 2025-03 --ir-report-id VIITE
aggountant file-wage employer-report preview --month 2025-03 \
  --payments tax-free-reimbursements-only --timestamp 2026-11-21T08:00:00Z \
  --replace --previous-pension-insurance 46:12-3456789
```
</details>

## Valmis, kun

Kuukauden tila on `accepted`, ja agentti näyttää Tulorekisterin
ilmoitusviitteen. Puuttuvien kuukausien luettelossa kuukautta ei enää ole. Voit
tarkistaa ilmoituksen myös Tulorekisterin asiointipalvelusta.

## Jos jokin menee pieleen

- Jos lähetyksen tulos jäi epäselväksi, ohjelma ei lähetä kuukaudelle mitään
  uudelleen ennen kuin tila on selvitetty. Agentti kysyy tilan
  Tulorekisteristä, kuten kohdassa
  [Ilmoituksen lopputulos on epävarma](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).
- Jos ilmoitus pitää korjata, pyydä agentilta korvaava ilmoitus, esimerkiksi
  kun "Ei palkanmaksua" -kuukaudelle selviää maksetun matkakorvauksia.
  Korvaava ilmoitus toistaa korvattavan ilmoituksen eläkevakuutustiedot.
  Eläkevakuutustiedot korjaat ja ilmoituksen mitätöit asiointipalvelussa:
  korvaava ilmoitus, joka muuttaisi eläkevakuutustietoja, hylätään
  (`EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED`).
- Jos Tulorekisteri hylkäsi ilmoituksen, katso
  [Tulorekisteri hylkäsi ilmoituksen](../troubleshooting.md#tulorekisteri-hylkäsi-ilmoituksen).

## Katso myös

- [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Virhekoodit](../reference/error-codes.md)
