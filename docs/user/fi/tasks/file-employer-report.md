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
- **Työnantajarekisteröinnin alkukuukausi.** Näet sen OmaVerosta tai
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
