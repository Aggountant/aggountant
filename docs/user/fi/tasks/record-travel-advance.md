# Kirjaa matkaennakko ja sen tilitys

Kun yritys maksaa sinulle tai työntekijälle rahaa ennen matkaa, ennakko on
yrityksen saaminen henkilöltä, kunnes matkasta tehty kululasku tilittää sen.
Agentti kirjaa ennakon, kululaskun ja niiden kuittauksen tavallisina
tositteina, ja vain erotus maksetaan tai palautetaan.

## Ennen kuin aloitat

- Ennakon maksu näkyy pankin tiliotteella tai tiedät, milloin ja kuinka paljon
  se maksettiin.
- Kun tilität ennakon, sinulla on matkasta tehty matka- tai kululasku.

## Näin teet sen

1. **Kerro agentille ennakosta**, kun tuot tiliotteen, jolla se maksettiin.

   > Tiliotteen 500 euron maksu Maija Meikäläiselle 3.3. on matkaennakko
   > Tampereen matkaa varten.

   Agentti kirjaa rivin tilille 1730 Muut lyhytaikaiset saamiset ilman ALV:tä
   ja kirjoittaa selitteeseen henkilön ja matkan. Tarkista tiliotteen
   esikatselusta, että rivi on saaminen eikä kulu.

2. **Pyydä agenttia kirjaamaan kululasku**, kun matka on tehty.

   > Kirjaa Maijan Tampereen matkan kululasku ja kuittaa ennakko.

   Agentti kirjaa kululaskun kokonaan, kuin ennakkoa ei olisi: kuitit
   kuittireitillä ja kilometrikorvaukset ja päivärahat erikoiskirjauksena, kaikki
   tiliä 2740 Muut lyhytaikaiset velat vastaan. Sen jälkeen se valmistelee
   kuittauksen omaksi tositteekseen: 2740 debet ja 1730 kredit, enintään ennakon
   avoimen osan ja enintään laskun summan verran. Tarkista kuittauksen summa ja
   päivä ennen hyväksyntää. Päivän pitää olla sama kuin erotuksen maksun tai
   palautuksen, koska kuittauksen päivä on Tulorekisterin maksupäivä ja samalla
   päivällä kuittaus ja pankkimaksu osuvat samaan kuukauteen. Jos erotusta ei
   ole, päivä on kululaskun hyväksymispäivä.

3. **Maksa tai palauta erotus**, ja agentti kirjaa sen seuraavalta
   tiliotteelta. Kun yritys maksaa erotuksen, vastatili on 2740. Kun henkilö
   palauttaa ylimääräisen ennakon tai matka peruuntui, vastatili on 1730.

4. **Ilmoita korvaukset Tulorekisteriin** tavalliseen tapaan. Ennakkoa ei
   ilmoiteta. Kululaskun kilometrikorvaukset ja päivärahat ilmoitetaan
   kokonaan, vaikka osa niistä maksettiin kuittaamalla, ja kuittauksen päivä on
   maksupäivä. [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
   kertoo, miten ilmoitus tehdään.

Esimerkki: ennakko on 500,00 € ja kululasku 620,00 €. Kuittaus siirtää
500,00 € tililtä 2740 tilille 1730, ja yritys maksaa Maijalle erotuksen
120,00 € samana päivänä. Lopuksi 1730 on nolla, eikä 2740:ssä ole tästä
laskusta mitään.

Tilitä ennakko pian matkan jälkeen. Agentti muistuttaa ennakosta, joka on
avoinna vielä kuukauden kuluttua matkasta. Osakkaana tilitä tai palauta ennakko
aina ennen vuoden loppua: käyttämätöntä ennakkoa ei voi jälkikäteen muuttaa
osakaslainaksi, ja palauttamaton ennakko voi olla peiteltyä osinkoa.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant import camt seal --file tiliote-2026-03.xml --dry-run < kohdistukset.json
aggountant capture-receipt record --dry-run < kuitti.json
aggountant voucher validate < kululasku.json
aggountant voucher seal < kululasku.json
aggountant voucher validate < ennakon-kuittaus.json
aggountant voucher seal < ennakon-kuittaus.json
```

`import camt seal` kirjaa ennakon maksun ja myöhemmin erotuksen, kumpikin
tiliotteen rivinä omalla vastatilillään. Esikatselun jälkeen agentti sinetöi
hyväksymäsi tiliotteen, kuten [tiliotteen tuonnissa](import-bank-statement.md).

`capture-receipt record` kirjaa matkan kuitit velaksi tilille 2740, kuten
[kuittien kirjauksessa](record-receipts.md).

Ensimmäinen `voucher seal` kirjaa kilometrikorvaukset ja päivärahat tileille
7300 Matkakulut ja 7350 Päivärahat matkalasku liitteenä. Toinen kirjaa ennakon
kuittauksen, myös matkalasku liitteenä.
</details>

## Valmis, kun

- Tilin 1730 saldossa ei ole tätä ennakkoa, ja tilillä 2740 ei ole tästä
  kululaskusta maksamatonta osaa.
- Agentti on kertonut kuittauksen tositenumeron.

## Rajoitukset

- aggountantissa ei ole vielä kululaskua eikä matkaennakkoa omana
  toimintonaan, joten agentti kokoaa tositteet käsin ja sinä tarkistat summat.
  [Mitä on valmista ja mitä ei](../reference/status.md#kuitit-ja-kirjaukset)
  kertoo tilanteen.

## Katso myös

- [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
- [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](record-other-entries.md)
- [Tuo pankin tiliote](import-bank-statement.md)
