# 6. Palkat ja Tulorekisteri

## Mitä tällä hetkellä voi tehdä

aggountantilla voi ilmoittaa Tulorekisteriin **verovapaat matkakorvaukset**
(kilometrikorvaukset ja päivärahat) yhdelle tulonsaajalle kerrallaan
(`file-wage`). Tämä toimii oikealla Tulorekisterin tuotantoyhteydellä.

**Rahapalkkoja ei vielä voi maksaa eikä ilmoittaa aggountantilla.**
Palkka-ajo (`run-payroll`) on ohjelmassa, mutta sen maksutaulukot
(työeläke-, sairausvakuutus- ja työttömyysvakuutusmaksut) ovat alustavia arvoja,
joita ei ole tarkistettu virallisista lähteistä. Verokortin käsittely on
yksinkertaistettu, eikä ajon takana ole oikeaa maksu- tai
Tulorekisteri-yhteyttä. Älä käytä sitä oikeisiin palkkoihin. Laske, maksa ja
ilmoita palkat toistaiseksi palkanlaskentapalvelulla tai Tulorekisterin
asiointipalvelussa, ja kirjaa palkkatosite aggountantiin erikoiskirjauksena
(luku 4).

## Työntekijän tiedot

Tulonsaajan henkilötiedot tallennetaan kodin henkilötietovarastoon
(`masterdata/pii.sqlite`), joka on rajattu gitin ulkopuolelle:

```sh
aggountant employee add < tyontekija.json
aggountant employee list
aggountant employee show --employee tyontekija-1
```

Syötteessä ovat tunniste (`employee_id`), nimi, henkilötunnus, tilinumero ja
verokortin pidätysprosentti (`tax_card.withholding_pct`). Pidätysprosentti on
tällä hetkellä pakollinen myös henkilölle, jolle ilmoitetaan vain
matkakorvauksia. Se ei vaikuta matkakorvausilmoitukseen.

Ota huomioon nämä rajoitukset:

- **Varasto ei vielä ole salattu.** Komennon ohje puhuu salatusta varastosta,
  mutta tässä versiossa tiedot tallentuvat selkokielisinä. Suojana on vain se,
  että tiedosto pidetään erillään gitistä. Pidä kotihakemisto ja sen
  varmuuskopiot vain omassa käytössäsi.
- **`employee show` näyttää henkilötunnuksen ja tilinumeron selväkielisinä.**
- **Henkilötunnusta ei kannata kirjoittaa keskusteluun agentin kanssa.** Laadi
  syöte itse tiedostoon tai komentoriville ja poista tiedosto sen jälkeen.
  Turvallista syöttötapaa, joka ei näyttäisi tietoja ruudulla, ei vielä ole.

## Verovapaat matkakorvaukset Tulorekisteriin

Ilmoitus koskee yhtä tulonsaajaa ja yhtä maksutapahtumaa, ja siinä voi olla
kahdenlaisia rivejä:

- **311 kilometrikorvaus**: summa ja kokonaiskilometrit
- **331 päiväraha**: summa ja päivärahan laji (1 ateriakorvaus, 2 osapäiväraha,
  3 kokopäiväraha, 4 ulkomaan päiväraha).

Kuittiperusteiset kulut, kuten hotelli tai pysäköinti, eivät kuulu näille
riveille. Työnantajan erillisilmoitusta, korvaavaa ilmoitusta ja mitätöintiä ei
tehdä tällä polulla. Ne tehdään Tulorekisterin asiointipalvelussa.

### Käyttöönotto

1. **Rajapintaoikeus.** Hae Verohallinnon varmennepalvelussa tuotantovarmenteelle
   korvaavalla hakemuksella myös Tulorekisterin palkkatietojen ilmoittamisen
   rajapinta (käyttötarkoitus: ilmoitan itse maksamiani tuloja). Pidä Vero API
   valittuna, jos käytät sitä. Sama varmenne kelpaa molempiin, kun oikeus on
   myönnetty. Myös erillinen varmenne on mahdollinen.
2. **Profiili.** Kodin tiedostoon `.aggountant/tulorekisteri.json` kirjoitetaan
   tuotantotila, Y-tunnus ja vahvistuksesi siitä, että ilmoitusoikeus on
   myönnetty. Tiedostolle ei ole omaa komentoa, joten agentti kirjoittaa sen
   `vero-fi`-skillin ohjeen mukaan. Vahvista oikeus vasta, kun se on myönnetty,
   ei silloin kun hakemus on lähetetty.
3. **Tunnukset.** Oletuksena käytetään samaa tunnustiedostoa kuin
   ALV-ilmoituksessa (luku 5). Erillinen tiedosto annetaan profiilissa.
   Tunnustiedoston pitää olla kodin ulkopuolella.
4. **Tulonsaaja** lisätään työntekijärekisteriin (yllä).

Kodin `.aggountant`-hakemisto toimii myös kodin tunnisteena: kun työskentelet
kodin hakemistossa tai sen alihakemistossa, ohjelma löytää kodin ilman
`--home`-valitsinta (luku 2).

### Ilmoituksen teko

Agentti laatii ilmoituksen kululaskun tai matkalaskun perusteella. Syötteessä
ovat Y-tunnus, maksutapahtuman pysyvä tunniste, tulonsaajan tunniste,
maksupäivä, ansaintajakson alku ja loppu, tieto pääomaosakkuudesta,
yhteyshenkilö ja rivit. Henkilötunnusta syötteessä ei ole, vaan ohjelma hakee sen
työntekijärekisteristä.

```sh
aggountant file-wage preview --timestamp 2026-10-25T12:00:00Z < ilmoitus.json
aggountant file-wage start   --timestamp 2026-10-25T12:00:00Z < ilmoitus.json
```

Esikatselu ja käynnistys eivät ota verkkoyhteyttä. Molemmille annetaan sama
aikaleima. Esikatselu näyttää rivit, jakson, maksupäivän, osakkuustiedon,
yhteyshenkilön ja määräpäivän. aggountant käyttää varovaista määräpäivää:
maksupäivä + 5 kalenteripäivää. Lisäksi se antaa ilmoituksen tarkisteen
(`record_hash`).

**Sinä hyväksyt** rivit, jakson, osakkuustiedon ja tarkisteen. Vasta sitten
agentti lähettää:

```sh
aggountant file-wage approve wr-… --record-hash <tarkiste>
```

Tarkiste on johdettu myös henkilötunnuksesta, joten sitä käsitellään
arkaluonteisena: sitä ei tallenneta gitiin, muistiinpanoihin eikä
keskustelulokiin.

Maksutapahtuman tunnistetta tai maksupäivää ei vaihdeta hylkäyksen tai
epävarmuuden kiertämiseksi. Silloin syntyisi huomaamatta uusi ilmoitus.

### Epävarma lopputulos ja varapolku

Jos lähetyksen tulos jää epäselväksi (aikakatkaisu, palvelun virhe tai
ristiriitainen vastaus), hyväksyntäkomennon toisto **ei lähetä uudelleen**.
Agentti kysyy tilan Tulorekisteristä:

```sh
aggountant file-wage reconcile wr-…
```

Jos tila on vielä käsittelyssä, kysytään myöhemmin uudelleen. Jos Tulorekisteri
on hylännyt ilmoituksen, ajo jää huomiota vaativaan tilaan. Tarkat virheet
näet Tulorekisterin asiointipalvelusta, koska ohjelma ei näytä viranomaisen
virhetekstiä, jossa voi olla henkilötunnus.

Kun rajapintapolku ei onnistu tai ilmoitus pitää korjata, tee ilmoitus
Tulorekisterin asiointipalvelussa. **Tarkista ensin, onko epävarma
rajapintalähetys jo mennyt perille**, ettet tee toista alkuperäistä ilmoitusta.
Kirjaa sen jälkeen palvelun antamat tunnisteet aggountantiin:

```sh
aggountant file-wage complete-manual wr-… \
  --ir-delivery-id <aineiston tunniste> --ir-report-id <ilmoituksen tunniste>
```
