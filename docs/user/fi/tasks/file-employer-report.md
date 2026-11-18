# Anna työnantajan erillisilmoitus

Kun yrityksesi on Verohallinnon työnantajarekisterissä, Tulorekisteriin
annetaan joka kuukausi työnantajan erillisilmoitus, myös kuukausilta, joilta et
maksanut mitään. Määräpäivä on seuraavan kuukauden 5. päivä. aggountant lähettää
ilmoituksen samalla Tulorekisterin tuotantoyhteydellä kuin matkakorvaukset ja
kertoo, mitkä kuukaudet puuttuvat.

Erillisilmoitus on eri ilmoitus kuin matkakorvausten palkkatietoilmoitus.
Matkakorvausilmoitus ei korvaa sitä.

## Ennen kuin aloitat

- **Tulorekisterin rajapintaoikeus ja profiili** ovat kunnossa
  ([Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md#ota-ilmoittaminen-käyttöön-kerran)).
  Erillisilmoitus käyttää samaa oikeutta ja tunnustiedostoa.
- **Työnantajarekisteröinnin alkukuukausi.** Agentti voi tarkistaa sen
  YTJ:n avoimesta datasta (katso alla), tai näet sen OmaVerosta tai
  Verohallinnon päätöksestä. Satunnaisesti palkkaa maksava yritys, joka ei ole
  rekisterissä, ei anna erillisilmoitusta kuukausilta, joilta se ei maksanut
  mitään.
- **Työeläkevakuutus**, jos yrityksellä on sellainen: eläkeyhtiö ja
  vakuutusnumero. Jos yrityksessä on vain YEL-vakuutettu omistaja, tietoja ei
  tarvita.

## Ota erillisilmoitus käyttöön (kerran)

1. **Pyydä agenttia lisäämään erillisilmoituksen asetukset.**

   > Yritys on ollut työnantajarekisterissä elokuusta 2026. Lisätään
   > työnantajan erillisilmoitus. Yhteyshenkilö on Maija Meikäläinen,
   > puhelin 040 123 4567. Työeläkevakuutusta ei ole, omistajalla on YEL.

2. **Agentti lisää** Tulorekisteri-profiiliin rekisteröinnin alkukuukauden,
   yhteyshenkilön ja työeläkevakuutukset. Tarkista ne: ne lähtevät
   ilmoituksen mukana, ja alkukuukaudesta lähtien jokaiselta kuukaudelta
   odotetaan ilmoitusta.

### Tarkista rekisteröinnin alku YTJ:stä

Jos et tiedä tai muista, milloin yritys tuli työnantajarekisteriin, pyydä
agenttia tarkistamaan:

> Milloin yritys tuli työnantajarekisteriin?

aggountant hakee Y-tunnuksella yrityksen työnantajarekisterimerkinnät PRH:n
YTJ-avoimesta datasta (ei tunnuksia, vain Y-tunnus lähtee kyselyssä) ja
vertaa niitä profiilin alkukuukauteen. Tulos kertoo, täsmääkö alkukuukausi,
alkaako profiili liian myöhään (jolloin vanhoja kuukausia jää puuttuvien
luettelosta pois) tai liian aikaisin, onko merkintä päättynyt vai eikö yritystä
ole rekisterissä lainkaan. Myös aiemmat rekisterijaksot näkyvät. Komento ei
muuta profiilia: agentti muuttaa alkukuukauden vain, kun sinä hyväksyt sen.
Erillisilmoituksia on annettu tammikuusta 2019 alkaen, joten sitä vanhempi
rekisteröinti tarkoittaa alkukuukautta 2019-01.

Jos YTJ ei vastaa, tulos on "ei tiedossa" eikä "ei rekisterissä"; kokeile
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
   kirjanpidosta ei näe. Vaihtoehtoja on kolme:
   - **Ei mitään maksuja**: ilmoitukseen tulee "Ei palkanmaksua".
   - **Vain verovapaita matkakorvauksia** (kilometrikorvaukset, päivärahat):
     ilmoitukseen tulee työnantajan sairausvakuutusmaksu 0,00 €. "Ei
     palkanmaksua" ei Tulorekisterin ohjeen mukaan ole silloin oikein, koska
     maksuja on tehty.
   - **Palkkaa**: aggountant ei vielä laske sairausvakuutusmaksua, joten se ei
     tee tällaisen kuukauden ilmoitusta. Tee se Tulorekisterin
     asiointipalvelussa ([Mitä on valmista ja mitä ei](../reference/status.md#tulorekisteri-ja-työntekijät)).

   aggountant ei hyväksy "Ei palkanmaksua" -valintaa kuukaudelle, jolle on
   tehty matkakorvausilmoitus.

3. **Agentti esikatselee ilmoituksen.** Esikatselu ei ota verkkoyhteyttä eikä
   tallenna mitään. Se näyttää kuukauden, sisällön, yhteyshenkilön, mahdolliset
   eläkevakuutukset, määräpäivän ja ilmoituksen tarkisteen.

4. **Tarkista ja hyväksy** kuukausi ja sisältö. Ilmoitus on lopullinen, kun
   Tulorekisteri on ottanut sen vastaan; korjaus on korvaava ilmoitus.

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
Aikaleimasta tulee myös ilmoituksen ilmoituspäivä. Profiilin kohta on
`employer_report` (`registered_from`, `contact_name`, `contact_phone`,
`pension_insurances`).
</details>

## Puuttuvat kuukaudet

Pyydä agenttia tarkistamaan, puuttuuko ilmoituksia:

> Puuttuuko työnantajan erillisilmoituksia?

aggountant luettelee rekisteröinnin alusta edelliseen kuukauteen asti kuukaudet,
joilta Tulorekisterillä ei ole aggountantin tiedossa olevaa ilmoitusta.
Jokaisesta näkyvät määräpäivä ja se, onko määräpäivä jo mennyt. Agentti käy
puuttuvat kuukaudet läpi vanhimmasta alkaen, ja sinä hyväksyt jokaisen
kuukauden erikseen. Myöhästyneestä ilmoituksesta voi tulla myöhästymismaksu,
joten myöhästyneet kannattaa antaa heti.

Jos olet jo antanut jonkin kuukauden ilmoituksen asiointipalvelussa, älä
lähetä sitä uudelleen. Kerro agentille ilmoituksen viite asiointipalvelusta,
niin kuukausi merkitään annetuksi.

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
aggountantia, kaikkia viitteitä ei kannata kaivaa yksitellen. Tulorekisterin
raportti 630 kertoo, monellako erillisilmoituksella kukin kuukausi on
ilmoitettu. Tämä on ainoa vaihe, joka vaatii Tulorekisterin
asiointipalvelua; aggountant ei voi tilata raporttia puolestasi.

1. **Tilaa raportti asiointipalvelussa**: Uusi tilaus → *Maksajan
   palkkatietojen yhteenveto (csv)* (630). Valitse kohdekauden aikaväliksi
   rekisteröinnin alusta alkaen, enintään 24 kuukautta kerrallaan, ja kieleksi
   **suomi**. Jätä vastaanottohetken aikaväli tyhjäksi: rajattu raportti jättää
   pois muulloin vastaanotetut ilmoitukset, eikä rajausta näe tiedostosta. Lataa valmis raportti *Tehdyt tilaukset* -sivulta. Jos lataus on
   zip-tiedosto, pura se.
2. **Anna tiedosto agentille.**

   > Tässä Tulorekisterin raportti 630 vuodelta 2025.

3. **Agentti tuo raportin.** aggountant tallentaa tiedoston ja merkitsee
   jokaisen kuukauden, jolla on vähintään yksi erillisilmoitus, annetuksi
   aggountantin ulkopuolella. Kuukausi, jolla ei ole ilmoitusta, jää
   puuttuvaksi. Kuukautta, jonka aggountant jo tuntee omasta lähetyksestään
   tai kirjatusta viitteestä, ei muuteta. Ristiriita näytetään, esimerkiksi
   kun aggountantin mukaan kuukausi on annettu mutta raportissa ei ole
   ilmoitusta. Saman tiedoston tuominen uudelleen ei muuta mitään. Raportissa
   ei ole maksajan Y-tunnusta, joten varmista, että se on oman yrityksesi.

Puuttuvien kuukausien luettelossa raportin kuukaudet näkyvät erikseen,
lähteenä raportti. Tällaiselle kuukaudelle aggountant ei lähetä uutta
ilmoitusta eikä korvaavaa, koska raportissa ei ole ilmoitusviitettä: korjaa
ilmoitus asiointipalvelussa tai kirjaa ensin sen viite.

Jos kaksi tuotua raporttia on eri mieltä kuukaudesta (toisessa on ilmoitus,
toisessa ei), aggountant ei arvaa, kumpi on uudempi. Kuukausi näkyy
puuttuvien luettelossa ristiriitana, eikä sille lähetetä mitään ennen kuin
tarkistat sen asiointipalvelusta ja agentti kirjaa ilmoituksen viitteen.

aggountant lukee vain suomenkielisen raportin, jonka sarakkeet ovat
Tulorekisterin dokumentaation mukaiset. Muunlainen tiedosto hylätään
(`TRANSCRIPT_UNKNOWN_HEADER`), eikä mitään tallenneta.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-wage employer-report import-transcript raportti-630.csv --dry-run
aggountant file-wage employer-report import-transcript raportti-630.csv
aggountant file-wage employer-report missing
```
</details>

## Valmis, kun

Kuukauden tila on `accepted`, ja agentti näyttää Tulorekisterin
ilmoitusviitteen. Puuttuvien kuukausien luettelossa kuukautta ei enää ole. Voit
tarkistaa ilmoituksen myös Tulorekisterin asiointipalvelusta.

## Jos jokin menee pieleen

- **Lähetyksen tulos jäi epäselväksi**: kuukaudelle ei lähetetä mitään
  uudelleen ennen kuin tila on selvitetty. Agentti kysyy tilan Tulorekisteristä
  ([Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma)).
- **Ilmoitus pitää korjata**: pyydä korvaava ilmoitus, esimerkiksi kun
  "Ei palkanmaksua" -kuukaudelle selviää maksetun matkakorvauksia. Korvaava
  ilmoitus toistaa korvattavan ilmoituksen eläkevakuutustiedot.
  Eläkevakuutustietojen korjaus ja mitätöinti tehdään asiointipalvelussa.
- **Tulorekisteri hylkäsi ilmoituksen**:
  [Kun jokin menee vikaan](../troubleshooting.md#tulorekisteri-hylkäsi-ilmoituksen).

## Katso myös

- [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Virhekoodit](../reference/error-codes.md)
