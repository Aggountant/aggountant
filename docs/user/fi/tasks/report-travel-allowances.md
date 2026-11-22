# Ilmoita matkakorvaukset Tulorekisteriin

Kun yritys maksaa työntekijälle tai itsellesi verovapaita matkakorvauksia,
ne ilmoitetaan Tulorekisteriin. aggountant ilmoittaa kilometrikorvaukset ja
päivärahat yhdelle tulonsaajalle ja yhdestä maksusta kerrallaan oikealla
Tulorekisterin tuotantoyhteydellä. Sivun alkuosa on kertaluonteinen
käyttöönotto.

Rahapalkkoja aggountantilla ei vielä voi maksaa eikä ilmoittaa
([Mitä on valmista ja mitä ei](../reference/status.md#palkat)).

Jos yritys on työnantajarekisterissä, Tulorekisteriin annetaan lisäksi joka
kuukausi työnantajan erillisilmoitus. Kuukaudelta, jolta maksoit vain
matkakorvauksia, siihen tulee sairausvakuutusmaksu 0 €:
[Anna työnantajan erillisilmoitus](file-employer-report.md).

## Ennen kuin aloitat

- **Rajapintaoikeus.** Avaa Verohallinnon varmennepalvelussa
  Tuotantovarmenteet → organisaatiosi → Näytä tiedot → Tee korvaava hakemus.
  Tulorekisterin rajapinnoista (Palkkatiedot ja Etuustiedot, kumpikin SFTP tai
  Web service) valitse **Palkkatiedot (Web service)**, käyttötarkoitus
  "ilmoitan itse maksamiani tuloja". Pidä Vero API valittuna, jos käytät sitä.
  Kun oikeus on myönnetty, vahvistuksessa lukee "Palkkatiedot tuottaja (Web
  service)" (ja "Vero API (Web service)", jos pidit sen). Hakemuksen voi
  hyväksyä heti, eikä uutta varmennetta tarvita: sama kelpaa molempiin. Myös
  erillinen varmenne on mahdollinen.
- **Tunnukset.** Oletuksena käytetään samaa tunnustiedostoa kuin
  ALV-ilmoituksessa ([Ota suora ALV-ilmoittaminen käyttöön](set-up-vero-api.md)).
  Erillisen tiedoston polku annetaan profiilissa. Tunnustiedoston pitää olla
  kodin ulkopuolella.
- **Matka- tai kululasku**, josta korvaus maksettiin, ja tieto maksupäivästä.

## Ota ilmoittaminen käyttöön (kerran)

1. **Pyydä agenttia kirjoittamaan Tulorekisteri-profiili.**

   > Otetaan Tulorekisterin matkakorvausilmoitukset käyttöön. Rajapintaoikeus
   > on myönnetty.

   Agentti kirjoittaa komennolla `tulorekisteri configure` kodin tiedostoon
   `.aggountant/tulorekisteri.json` tuotantotilan, Y-tunnuksen ja
   vahvistuksesi siitä, että ilmoitusoikeus on myönnetty. **Vahvista oikeus
   vasta, kun se on myönnetty**, ei silloin kun hakemus on lähetetty.
   Tuotantotila kirjoitetaan vasta, kun yhteys on valmis; muuten komento
   luettelee, mikä puuttuu. `tulorekisteri status` näyttää saman verkotta
   myöhemminkin: jokaisen puuttuvan asian koodin, sen, kuuluuko se sinulle
   (`human`) vai agentille (`agent`), ja mitä tehdä.

2. **Lisää tulonsaaja työntekijärekisteriin.** Tulonsaajan tiedot ovat
   tunniste, nimi, henkilötunnus, tilinumero ja verokortin pidätysprosentti.
   Pidätysprosentti on tällä hetkellä pakollinen myös henkilölle, jolle
   ilmoitetaan vain matkakorvauksia. Se ei vaikuta matkakorvausilmoitukseen.

   **Älä kirjoita henkilötunnusta keskusteluun agentin kanssa.** Laadi syöte
   itse tiedostoon, anna agentille vain tiedoston nimi ja poista tiedosto
   lisäyksen jälkeen. Turvallista syöttötapaa, joka ei näyttäisi tietoja
   ruudulla, ei vielä ole.

## Näin teet ilmoituksen

1. **Pyydä agenttia.**

   > Ilmoita Maija Meikäläisen lokakuun matkalaskun korvaukset
   > Tulorekisteriin. Maksupäivä oli 24.10.2026.

2. **Agentti laatii ilmoituksen** matka- tai kululaskusta. Rivejä on kahta
   lajia:
   - **311 kilometrikorvaus**: summa ja kokonaiskilometrit
   - **331 päiväraha**: summa ja päivärahan laji (1 ateriakorvaus,
     2 osapäiväraha, 3 kokopäiväraha, 4 ulkomaan päiväraha).

   Kuittiperusteiset kulut, kuten hotelli tai pysäköinti, eivät kuulu näille
   riveille. Ilmoituksessa ovat myös maksupäivä, ansaintajakson alku ja loppu,
   tieto pääomaosakkuudesta ja yhteyshenkilö. Henkilötunnusta agentti ei
   kirjoita ilmoitukseen, vaan ohjelma hakee sen työntekijärekisteristä.

3. **Agentti esikatselee ilmoituksen.** Esikatselu ei ota verkkoyhteyttä. Se
   näyttää rivit, jakson, maksupäivän, osakkuustiedon, yhteyshenkilön ja
   määräpäivän. aggountant käyttää varovaista määräpäivää: maksupäivä + 5
   kalenteripäivää. Lisäksi se antaa ilmoituksen tarkisteen.

4. **Tarkista ja hyväksy** rivit, jakson, osakkuustiedon ja tarkisteen.
   Pääomaosakkuus kerrotaan aina erikseen. aggountant ei päättele sitä.

   > Hyväksyn ilmoituksen.

5. **Agentti lähettää ilmoituksen** juuri hyväksymälläsi tarkisteella.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant employee add < tyontekija.json
aggountant employee list
aggountant --json file-wage preview --timestamp 2026-10-25T12:00:00Z < ilmoitus.json \
  | tee >(jq -r .record_hash > TIEDOSTO) | jq 'del(.record_hash)'
aggountant file-wage start   --timestamp 2026-10-25T12:00:00Z < ilmoitus.json
aggountant file-wage approve wr-… --record-hash-file TIEDOSTO
aggountant run show --run-id wr-…
```

Työntekijän syötteessä tunniste on `employee_id` ja pidätysprosentti
`tax_card.withholding_pct`. Esikatselu antaa tarkisteen kentässä `record_hash`.
Tarkiste on johdettu henkilötunnuksesta, joten se on arkaluonteinen: sitä
käytetään vain hyväksyntään, eikä sitä tallenneta pysyvästi, kirjata lokiin tai jaeta.
Esikatselu sanoo saman kentässä `sensitive.record_hash`. Siksi agentti
pitää sen poissa komentoriviltä, jolta se jäisi komentohistoriaan ja
keskusteluun: se ottaa näkemäsi esikatselun tarkisteen talteen yksityiseen
väliaikaistiedostoon (`mktemp`, kodin ulkopuolella), näyttää esikatselun ilman
sitä, antaa tiedoston hyväksynnälle (`--record-hash-file`) ja poistaa sen
lopuksi.
Esikatselulle ja käynnistykselle annetaan sama aikaleima.
`aggountant employee show --employee <tunniste>` näyttää henkilötunnuksen ja
tilinumeron selväkielisinä, joten agentti ei aja sitä tarpeettomasti.

Kun Tulorekisteri on hyväksynyt ilmoituksen, `file-wage approve`- ja
`run show` -vastauksissa on sen kuittaus `submit-wage`-efektin kentissä
`outcome.receipt.ir_delivery_id` ja `outcome.receipt.ir_report_id`. `run show`
lukee sen ilman verkkoa. Jos `outcome.state` on `outcome_unknown`,
`outcome.next_command` nimeää komennon `file-wage reconcile`, joka kysyy tilan
Tulorekisteriltä.
</details>

## Valmis, kun

Agentti kertoo, että Tulorekisteri on vastaanottanut ilmoituksen. Voit
tarkistaa sen myös Tulorekisterin asiointipalvelusta.

## Korjaa hyväksytty ilmoitus

Kun Tulorekisteri on jo hyväksynyt ilmoituksen ja siinä on virhe, korjaus
tehdään aggountantilla samalla tavalla kuin ilmoitus: agentti esikatselee
korjauksen, sinä hyväksyt sen ja agentti lähettää sen kerran.

> Lokakuun matkalaskun kilometrit olivat väärin, oikea määrä on 160 km ja
> korvaus 96,00 €. Korjaa Tulorekisteri-ilmoitus.

Korjauksia on kahta lajia:

- **Korvaava ilmoitus** antaa koko ilmoituksen uudelleen korjattuna. Sillä
  korjataan rivit, yhteyshenkilö tai tieto pääomaosakkuudesta. Maksajaa,
  tulonsaajaa, maksupäivää ja ansaintajaksoa se ei voi muuttaa, koska
  Tulorekisteri vaatii niiden pysyvän samoina.
- **Mitätöinti** poistaa ilmoituksen. Sitä käytetään, kun maksua ei ollutkaan
  tai kun pitää muuttaa jotain, mitä korvaava ilmoitus ei saa muuttaa. Sen
  jälkeen annetaan tarvittaessa uusi ilmoitus tavalliseen tapaan.

Esikatselu näyttää Tulorekisterissä nyt olevan ilmoituksen ja korjatun
ilmoituksen rinnakkain. Tarkista, että juuri se muuttuu, minkä pitikin, ja
hyväksy vasta sitten. Korjaus on yhtä lopullinen kuin alkuperäinen ilmoitus.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-wage correction preview wr-… --action replace --timestamp 2026-11-02T08:00:00Z < korjattu.json
aggountant file-wage correction submit  wr-… --action replace --timestamp 2026-11-02T08:00:00Z --record-hash-file TIEDOSTO < korjattu.json
aggountant file-wage correction preview wr-… --action cancel --timestamp 2026-11-02T08:00:00Z
aggountant file-wage correction status  wr-…
```

`wr-…` on alkuperäisen ilmoituksen ajo. Korvaavan ilmoituksen syöte on koko
ilmoitus korjattuna, ja sen tarkiste on arkaluonteinen kuten alkuperäisen,
joten se otetaan talteen tiedostoon samalla tavalla.
Mitätöinti ei lue syötettä. `correction status` näyttää, mitä Tulorekisterissä
nyt on (`accepted`, `replaced`, `cancelled`), voimassa olevan kuittauksen ja
ilmoituksen koko historian. Alkuperäisen ajon `run show` näyttää tilan
`superseded`, kun sen kuittaus ei enää ole voimassa oleva. Mitätöinnin jälkeen
uusi ilmoitus tarvitsee oman `source_reference`-arvonsa.

Jos olet jo tehnyt korvaavan ilmoituksen tai mitätöinnin Tulorekisterin
asiointipalvelussa, agentti kirjaa sen näytöstä:

```sh
aggountant file-wage correction complete-manual wr-… --action replace --ir-report-id <ilmoitusviite> --report-version 2
```
</details>

## Rajoitukset

- Korvaava ilmoitus ei voi muuttaa maksajaa, tulonsaajaa, maksupäivää eikä
  ansaintajaksoa. Niitä varten ilmoitus mitätöidään ja annetaan uudelleen.
- Työntekijärekisteriä ei vielä ole salattu. Pidä kotihakemisto ja sen
  varmuuskopiot vain omassa käytössäsi.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#tulorekisteri-ja-työntekijät).

## Jos jokin menee pieleen

- **Lähetyksen tulos jäi epäselväksi** (aikakatkaisu, palvelun virhe):
  hyväksynnän toisto ei lähetä uudelleen.
  [Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).
- **Tulorekisteri hylkäsi ilmoituksen.** Katso syy asiointipalvelusta:
  [Kun jokin menee vikaan](../troubleshooting.md#tulorekisteri-hylkäsi-ilmoituksen).
- **Korjauksen tulos jäi epäselväksi.** `file-wage correction status` kysyy
  tilan; korjausta ei lähetetä uudelleen eikä uutta korjausta voi tehdä ennen
  kuin se on selvinnyt.
- **Älä vaihda maksun tunnistetta tai maksupäivää** kiertääksesi hylkäyksen tai
  epävarmuuden. Silloin syntyisi huomaamatta uusi ilmoitus.

## Katso myös

- [Anna työnantajan erillisilmoitus](file-employer-report.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
