# Ilmoita matkakorvaukset Tulorekisteriin

Kun yritys maksaa työntekijälle tai sinulle itsellesi verovapaita
matkakorvauksia, ne ilmoitetaan Tulorekisteriin. aggountant ilmoittaa
kilometrikorvaukset ja päivärahat yhdelle tulonsaajalle ja yhdestä maksusta
kerrallaan Tulorekisterin tuotantoyhteydellä. Rahapalkkoja sillä
[ei vielä voi maksaa eikä ilmoittaa](../reference/status.md#palkat).

Jos yritys on työnantajarekisterissä, se antaa Tulorekisteriin lisäksi joka
kuukausi [työnantajan erillisilmoituksen](file-employer-report.md).
Kuukaudelta, jolta maksoit vain matkakorvauksia, siihen tulee
sairausvakuutusmaksu 0 €.

## Ennen kuin aloitat

- Yritykselle on myönnetty Tulorekisterin rajapintaoikeus. Hae sitä
  Verohallinnon varmennepalvelussa: Tuotantovarmenteet → organisaatiosi →
  Näytä tiedot → Tee korvaava hakemus. Tulorekisterin rajapintoja ovat
  Palkkatiedot ja Etuustiedot, kumpikin SFTP- tai Web service -muodossa.
  Valitse **Palkkatiedot (Web service)** ja käyttötarkoitukseksi "ilmoitan
  itse maksamiani tuloja". Pidä Vero API valittuna, jos käytät sitä.
  Hakemuksen voi hyväksyä heti. Uutta varmennetta et tarvitse, koska sama
  varmenne kelpaa molempiin, mutta voit käyttää myös erillistä. Kun oikeus on
  myönnetty, vahvistuksessa lukee "Palkkatiedot tuottaja (Web service)" ja,
  jos pidit Vero API:n, "Vero API (Web service)".
- Tunnustiedosto on kodin ulkopuolella. Oletuksena aggountant käyttää samaa
  tiedostoa kuin [suorassa ALV-ilmoittamisessa](set-up-vero-api.md). Jos
  haluat Tulorekisterille oman tiedoston, agentti kirjoittaa sen polun
  profiiliin.
- Sinulla on matka- tai kululasku, josta korvaus maksettiin, ja tiedät
  maksupäivän.

## Ota ilmoittaminen käyttöön (kerran)

1. **Pyydä agenttia kirjoittamaan Tulorekisteri-profiili.**

   > Otetaan Tulorekisterin matkakorvausilmoitukset käyttöön. Rajapintaoikeus
   > on myönnetty.

   Agentti kirjoittaa profiiliin tuotantotilan, yrityksen Y-tunnuksen ja
   vahvistuksesi siitä, että ilmoitusoikeus on myönnetty. Vahvista oikeus
   vasta, kun se on myönnetty. Hakemuksen lähettäminen ei vielä riitä.

   Ohjelma kirjoittaa tuotantotilan vasta, kun yhteys on valmis. Muuten se
   luettelee, mikä puuttuu: jokaisen puutteen koodin, sen, kuuluuko puute
   sinulle (`human`) vai agentille (`agent`), ja mitä tehdä.

2. **Kirjoita tulonsaajan tiedot tiedostoon**, ja agentti lisää hänet
   työntekijärekisteriin. Tiedot ovat tunniste, nimi, henkilötunnus,
   tilinumero ja verokortin pidätysprosentti. Pidätysprosentti on tällä
   hetkellä pakollinen myös henkilölle, jolle ilmoitetaan vain
   matkakorvauksia, vaikka se ei vaikuta matkakorvausilmoitukseen.

   **Älä kirjoita henkilötunnusta keskusteluun agentin kanssa.** Anna
   agentille vain tiedoston nimi ja poista tiedosto, kun tulonsaaja on
   lisätty. Syöttötapaa, joka pitäisi tiedot poissa ruudulta, ei vielä ole.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant tulorekisteri configure < profiili.json
aggountant tulorekisteri status
aggountant employee add < tyontekija.json
aggountant employee list
```

`tulorekisteri configure` kirjoittaa profiilin kodin tiedostoon
`.aggountant/tulorekisteri.json`.

`tulorekisteri status` näyttää myöhemminkin ilman verkkoyhteyttä, mitä
yhteydestä vielä puuttuu.

Työntekijän syötteessä tunniste on `employee_id` ja pidätysprosentti
`tax_card.withholding_pct`.

`aggountant employee show --employee <tunniste>` näyttää henkilötunnuksen ja
tilinumeron selväkielisinä, joten agentti ei aja sitä tarpeettomasti.
</details>

## Näin teet ilmoituksen

1. **Pyydä agenttia.**

   > Ilmoita Maija Meikäläisen lokakuun matkalaskun korvaukset
   > Tulorekisteriin. Maksupäivä oli 24.10.2026.

2. **Agentti laatii ilmoituksen** matka- tai kululaskusta.
   Kilometrikorvauksen rivillä (311) ovat summa ja kokonaiskilometrit.
   Päivärahan rivillä (331) ovat summa ja päivärahan laji: 1 ateriakorvaus,
   2 osapäiväraha, 3 kokopäiväraha tai 4 ulkomaan päiväraha.
   Kuittiperusteiset kulut, kuten hotelli tai pysäköinti, eivät kuulu näille
   riveille.

   Ilmoituksessa ovat myös maksupäivä, ansaintajakson alku ja loppu, tieto
   pääomaosakkuudesta ja yhteyshenkilö. Tulorekisterin asiointipalvelussa
   pääomaosakkuus on nimellä "Osakasasemassa maksajaan". Se ei ole
   työttömyysturvalain osaomistajuus. Ansaintajaksoa ja pääomaosakkuutta
   aggountant ei päättele, joten agentti kysyy pääomaosakkuuden sinulta ja
   ansaintajakson, jos se ei käy ilmi matkalaskusta. Henkilötunnusta agentti ei
   kirjoita ilmoitukseen, vaan ohjelma hakee sen työntekijärekisteristä.

3. **Agentti esikatselee ilmoituksen.** Esikatselu ei ota verkkoyhteyttä. Se
   näyttää rivit, jakson, maksupäivän, osakkuustiedon, yhteyshenkilön ja
   määräpäivän. Määräpäivänä aggountant näyttää varovaisesti viidennen
   kalenteripäivän maksupäivästä.

4. **Tarkista ja hyväksy** rivit, jakso, maksupäivä ja osakkuustieto.

   > Hyväksyn ilmoituksen.

5. **Agentti lähettää ilmoituksen.** Esikatselu antaa tarkisteen, ja lähetys
   onnistuu vain sillä, joten Tulorekisteriin lähtee juuri se ilmoitus, jonka
   näit.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant --json file-wage preview --timestamp 2026-10-25T12:00:00Z < ilmoitus.json \
  | tee >(jq -r .record_hash > TIEDOSTO) | jq 'del(.record_hash)'
aggountant file-wage start   --timestamp 2026-10-25T12:00:00Z < ilmoitus.json
aggountant file-wage approve wr-… --record-hash-file TIEDOSTO
aggountant run show --run-id wr-…
```

Esikatselu antaa tarkisteen kentässä `record_hash`. Tarkiste on johdettu
henkilötunnuksesta, joten se on arkaluonteinen: sitä käytetään vain
hyväksyntään, eikä sitä tallenneta pysyvästi, kirjata lokiin tai jaeta.
Esikatselu sanoo saman kentässä `sensitive.record_hash`. Komentorivillä
tarkiste jäisi komentohistoriaan ja keskusteluun. Siksi agentti ottaa
näkemäsi esikatselun tarkisteen talteen yksityiseen väliaikaistiedostoon
(`mktemp`, kodin ulkopuolella) ja näyttää esikatselun ilman sitä.

`file-wage start` saa saman aikaleiman kuin esikatselu.

`file-wage approve` lukee tarkisteen tiedostosta (`--record-hash-file`), ja
agentti poistaa tiedoston sen jälkeen. Kun Tulorekisteri on hyväksynyt
ilmoituksen, vastauksessa on sen kuittaus `submit-wage`-efektin kentissä
`outcome.receipt.ir_delivery_id` ja `outcome.receipt.ir_report_id`. Jos
`outcome.state` on `outcome_unknown`, `outcome.next_command` nimeää komennon
`file-wage reconcile`, joka kysyy tilan Tulorekisteriltä.

`run show` näyttää saman kuittauksen ilman verkkoyhteyttä.
</details>

## Valmis, kun

Agentti kertoo, että Tulorekisteri on vastaanottanut ilmoituksen. Voit
tarkistaa sen myös Tulorekisterin asiointipalvelusta.

## Korjaa hyväksytty ilmoitus

Kun Tulorekisteri on jo hyväksynyt ilmoituksen ja siinä on virhe, korjaa se
aggountantilla samaan tapaan kuin teit ilmoituksen: agentti esikatselee
korjauksen, sinä hyväksyt sen ja agentti lähettää sen kerran.

> Lokakuun matkalaskun kilometrit olivat väärin, oikea määrä on 160 km ja
> korvaus 96,00 €. Korjaa Tulorekisteri-ilmoitus.

Korvaava ilmoitus antaa koko ilmoituksen uudelleen korjattuna. Sillä
korjataan rivit, yhteyshenkilö tai tieto pääomaosakkuudesta. Maksajaa,
tulonsaajaa, maksupäivää ja ansaintajaksoa se ei voi muuttaa, koska
Tulorekisteri vaatii niiden pysyvän samoina.

Mitätöinti poistaa ilmoituksen. Sitä käytetään, kun maksua ei ollutkaan tai
kun pitää muuttaa jotain, mitä korvaava ilmoitus ei voi muuttaa. Sen jälkeen
annetaan tarvittaessa uusi ilmoitus tavalliseen tapaan.

Esikatselu näyttää Tulorekisterissä nyt olevan ilmoituksen ja korjatun
ilmoituksen rinnakkain. Tarkista ennen hyväksyntää, että vain se muuttuu,
minkä pitikin. Korjaus on yhtä lopullinen kuin alkuperäinen ilmoitus.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-wage correction preview wr-… --action replace --timestamp 2026-11-02T08:00:00Z < korjattu.json
aggountant file-wage correction submit  wr-… --action replace --timestamp 2026-11-02T08:00:00Z --record-hash-file TIEDOSTO < korjattu.json
aggountant file-wage correction preview wr-… --action cancel --timestamp 2026-11-02T08:00:00Z
aggountant file-wage correction status  wr-…
```

`wr-…` on alkuperäisen ilmoituksen ajo.

Korvaavan ilmoituksen syöte on koko ilmoitus korjattuna. Sen tarkiste on
arkaluonteinen kuten alkuperäisen, joten agentti ottaa sen talteen tiedostoon
samalla tavalla.

Mitätöinti ei lue syötettä. Mitätöinnin jälkeen uusi ilmoitus tarvitsee oman
`source_reference`-arvonsa.

`correction status` näyttää, mitä Tulorekisterissä nyt on (`accepted`,
`replaced`, `cancelled`), voimassa olevan kuittauksen ja ilmoituksen koko
historian. Alkuperäisen ajon `run show` näyttää tilan `superseded`, kun sen
kuittaus ei enää ole voimassa oleva.

Jos olet jo tehnyt korvaavan ilmoituksen tai mitätöinnin Tulorekisterin
asiointipalvelussa, agentti kirjaa sen näytöstä:

```sh
aggountant file-wage correction complete-manual wr-… --action replace --ir-report-id <ilmoitusviite> --report-version 2
```
</details>

## Rajoitukset

Työntekijärekisteriä ei vielä ole salattu. Pidä kotihakemisto ja sen
varmuuskopiot vain omassa käytössäsi. Muut rajoitukset ovat kohdassa
[Tulorekisteri ja työntekijät](../reference/status.md#tulorekisteri-ja-työntekijät).

## Jos jokin menee pieleen

- Jos lähetyksen tulos jäi epäselväksi (aikakatkaisu, palvelun virhe),
  hyväksynnän toisto ei lähetä ilmoitusta uudelleen. Katso
  [Ilmoituksen lopputulos on epävarma](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).
- Jos Tulorekisteri hylkäsi ilmoituksen, katso syy asiointipalvelusta ja lue
  [Tulorekisteri hylkäsi ilmoituksen](../troubleshooting.md#tulorekisteri-hylkäsi-ilmoituksen).
- Jos korjauksen tulos jäi epäselväksi, `file-wage correction status` kysyy
  tilan Tulorekisteriltä. Ohjelma ei lähetä korjausta uudelleen, eikä uutta
  korjausta voi tehdä, ennen kuin tila on selvinnyt.

Älä vaihda maksun tunnistetta tai maksupäivää kiertääksesi hylkäyksen tai
epävarmuuden, koska silloin syntyisi huomaamatta uusi ilmoitus.

## Katso myös

- [Anna työnantajan erillisilmoitus](file-employer-report.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Yrityskoti: mitä tallentuu minne](../concepts/company-home.md)
