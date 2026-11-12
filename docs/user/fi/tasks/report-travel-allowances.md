# Ilmoita matkakorvaukset Tulorekisteriin

Kun yritys maksaa työntekijälle tai itsellesi verovapaita matkakorvauksia,
ne ilmoitetaan Tulorekisteriin. aggountant ilmoittaa kilometrikorvaukset ja
päivärahat yhdelle tulonsaajalle ja yhdestä maksusta kerrallaan oikealla
Tulorekisterin tuotantoyhteydellä. Sivun alkuosa on kertaluonteinen
käyttöönotto.

Rahapalkkoja aggountantilla ei vielä voi maksaa eikä ilmoittaa
([Mitä on valmista ja mitä ei](../reference/status.md#palkat)).

## Ennen kuin aloitat

- **Rajapintaoikeus.** Hae Verohallinnon varmennepalvelussa
  tuotantovarmenteelle korvaavalla hakemuksella myös Tulorekisterin
  palkkatietojen ilmoittamisen rajapinta (käyttötarkoitus: ilmoitan itse
  maksamiani tuloja). Pidä Vero API valittuna, jos käytät sitä. Sama varmenne
  kelpaa molempiin, kun oikeus on myönnetty. Myös erillinen varmenne on
  mahdollinen.
- **Tunnukset.** Oletuksena käytetään samaa tunnustiedostoa kuin
  ALV-ilmoituksessa ([Ota suora ALV-ilmoittaminen käyttöön](set-up-vero-api.md)).
  Erillisen tiedoston polku annetaan profiilissa. Tunnustiedoston pitää olla
  kodin ulkopuolella.
- **Matka- tai kululasku**, josta korvaus maksettiin, ja tieto maksupäivästä.

## Ota ilmoittaminen käyttöön (kerran)

1. **Pyydä agenttia kirjoittamaan Tulorekisteri-profiili.**

   > Otetaan Tulorekisterin matkakorvausilmoitukset käyttöön. Rajapintaoikeus
   > on myönnetty.

   Agentti kirjoittaa kodin tiedostoon `.aggountant/tulorekisteri.json`
   tuotantotilan, Y-tunnuksen ja vahvistuksesi siitä, että ilmoitusoikeus on
   myönnetty. Tiedostolle ei ole omaa komentoa, joten agentti kirjoittaa sen
   `vero-fi`-skillin ohjeen mukaan. **Vahvista oikeus vasta, kun se on
   myönnetty**, ei silloin kun hakemus on lähetetty.

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
aggountant file-wage preview --timestamp 2026-10-25T12:00:00Z < ilmoitus.json
aggountant file-wage start   --timestamp 2026-10-25T12:00:00Z < ilmoitus.json
aggountant file-wage approve wr-… --record-hash <tarkiste>
```

Työntekijän syötteessä tunniste on `employee_id` ja pidätysprosentti
`tax_card.withholding_pct`. Esikatselu antaa tarkisteen kentässä `record_hash`.
Esikatselulle ja käynnistykselle annetaan sama aikaleima.
`aggountant employee show --employee <tunniste>` näyttää henkilötunnuksen ja
tilinumeron selväkielisinä, joten agentti ei aja sitä tarpeettomasti.
</details>

## Valmis, kun

Agentti kertoo, että Tulorekisteri on vastaanottanut ilmoituksen. Voit
tarkistaa sen myös Tulorekisterin asiointipalvelusta.

## Rajoitukset

- Työnantajan erillisilmoitusta, korvaavaa ilmoitusta ja mitätöintiä ei tehdä
  aggountantilla. Ne tehdään Tulorekisterin asiointipalvelussa.
- Työntekijärekisteriä ei vielä ole salattu, vaikka komennon ohje niin sanoo.
  Pidä kotihakemisto ja sen varmuuskopiot vain omassa käytössäsi.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#tulorekisteri-ja-työntekijät).

## Jos jokin menee pieleen

- **Lähetyksen tulos jäi epäselväksi** (aikakatkaisu, palvelun virhe):
  hyväksynnän toisto ei lähetä uudelleen.
  [Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).
- **Tulorekisteri hylkäsi ilmoituksen tai se pitää korjata.** Tee se
  asiointipalvelussa: [Kun jokin menee vikaan](../troubleshooting.md#tulorekisteri-hylkäsi-ilmoituksen).
- **Älä vaihda maksun tunnistetta tai maksupäivää** kiertääksesi hylkäyksen tai
  epävarmuuden. Silloin syntyisi huomaamatta uusi ilmoitus.

## Katso myös

- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
