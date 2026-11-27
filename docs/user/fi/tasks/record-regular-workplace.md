# Kirjaa varsinainen työpaikka

Verovapaita matkakorvauksia voi maksaa vain matkasta, joka ei ole matka kodin
ja varsinaisen työpaikan (paikka, jossa teet työtä vakituisesti, esimerkiksi
oma toimisto tai kotitoimisto) välillä. Siksi aggountant tarvitsee jokaiselta
kululaskun tekijältä päivätyn tiedon siitä, mikä hänen varsinainen
työpaikkansa on, ja hänen suhteestaan yhtiöön. Kirjaat sen kerran, ja agentti
kysyy uudelleen vain, kun on syytä epäillä muutosta.

Jos yrityksesi ei tee kululaskuja, tätä ei tarvita, eikä aggountant kysy
mitään.

## Ennen kuin aloitat

- Henkilö on työntekijärekisterissä (`employee add`). Rekisteri on sama myös
  hallituksen jäsenelle ja palkatta työskentelevälle osakkaalle.
- Tiedät, mistä päivästä alkaen tieto pitää paikkansa, esimerkiksi päivän,
  jona yhtiön toimitila siirtyi kotiisi.

## Näin teet sen

1. **Kerro agentille tilanteesi omin sanoin.**

   > Yhtiön toimitila on ollut kotonani Esimerkkikatu 1:ssä Tampereella
   > 1.1.2024 alkaen. Olen toimitusjohtaja ja omistan osan yhtiöstä.

2. **Agentti ehdottaa kirjaukset.** Se tallentaa paikan (osoite ja kunta) ja
   ehdottaa kolme päivättyä tietoa: suhteen yhtiöön (esimerkiksi
   toimitusjohtaja), pääomaosakkuuden ja varsinaisen työpaikan perusteineen.
   Peruste on suljetusta luettelosta, ja jokainen peruste viittaa
   Verohallinnon ohjeen kohtaan, esimerkiksi *yhtiön toimitila on
   kotona* (ohje 3.4) tai *käyt työnantajan toimitilassa viikoittain*
   (ohje 5.2.2, KHO 2017:178). Jos mikään peruste ei sovi, agentti kirjoittaa
   perustelun omin sanoin.

3. **Tarkista ja vahvista.** Katso, että päivämäärä, paikka ja peruste ovat
   oikein. Vain vahvistamasi tieto tallentuu, ja tallennukseen merkitään, kuka
   sen vahvisti ja milloin. Ohjelma ei koskaan päättele näitä tietoja itse.

4. **Kerro muut vakituiset paikat**, jos niitä on:
   - toinen vakituinen paikka samassa kunnassa on toinen varsinainen
     työpaikka;
   - oman yhtiön tai sen konserniyhtiön toimipaikka toisessa kunnassa, jossa
     käyt säännöllisesti, on toissijainen työpaikka (oman yhtiön toinen
     vakituinen toimipaikka toisessa kunnassa). Koti tai asiakkaan tila ei voi
     olla toissijainen työpaikka;
   - asiakkaan tila, jossa käyt pitkään, voidaan kirjata seurattavaksi
     erityiseksi työntekemispaikaksi (tilapäinen paikka, kuten asiakkaan
     luona).

   Hallituksen jäsen, joka ei muuten työskentele yhtiössä, kirjataan niin,
   ettei hänellä ole varsinaista työpaikkaa.

Kun tiedot ovat rekisterissä, agentti näkee jokaisen matkan kohteesta, onko se
varsinainen työpaikka (ei verovapaita korvauksia lainkaan), toissijainen
työpaikka (vain matka- ja majoituskulut, ei päivärahaa eikä matkaa
majoituksesta työpaikalle) vai erityinen työntekemispaikka (korvaukset
tavalliseen tapaan). Paikka, jota rekisterissä ei ole, on erityinen
työntekemispaikka.

### Kun tieto muuttuu tai oli väärin

Kirjattua tietoa ei muokata. Kun jokin muuttuu, esimerkiksi toimisto lopetetaan,
agentti kirjaa korjauksen, joka korvaa vanhan tiedon, ja syyn sille. Korjaus
voi koskea myös mennyttä aikaa. Vanha tieto jää historiaan, joten myöhemmin
näkee, mitä rekisterissä luki silloin, kun aiempi kululasku hyväksyttiin.

Kahta ristiriitaista tietoa samoille päiville ei voi kirjata, esimerkiksi kahta
ensisijaista varsinaista työpaikkaa. Silloin agentti korjaa vanhan tiedon.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant claimant place add < paikka.json
aggountant claimant add < tieto.json
aggountant claimant correct < korjaus.json
aggountant claimant show --employee maija --date 2026-03-04
aggountant claimant show --employee maija --date 2026-03-04 --as-known-at 2026-01-15T00:00:00Z
aggountant claimant history --employee maija
aggountant claimant snapshot < matka.json
```

`claimant add` tallentaa yhden päivätyn tiedon: työpaikan roolin
perusteineen, suhteen yhtiöön tai pääomaosakkuuden. `claimant show` näyttää,
mitä päivänä oli voimassa, ja `--as-known-at` sen, mitä siitä tiedettiin
annettuna hetkenä. `claimant snapshot` luokittelee matkan paikat ja kertoo,
mitä matkasta saa korvata. `aggountant claimant add --help` luettelee
perusteet ja ohjeen kohdat.
</details>

## Valmis, kun

- `claimant show` näyttää henkilölle päivänä voimassa olevan suhteen ja
  kertoo, että varsinainen työpaikka on kirjattu (`regular_workplace_known`
  on `true`).

## Rajoitukset

- Kululaskua ei vielä tehdä ohjelmalla, joten rekisteriä käyttää toistaiseksi
  vain matkan luokittelu ([Mitä on valmista ja mitä ei](../reference/status.md#kuitit-ja-kirjaukset)).
- Rekisteri tallentuu työntekijärekisterin tavoin salaamattomana
  ([Mitä on valmista ja mitä ei](../reference/status.md#tulorekisteri-ja-työntekijät)).

## Jos jokin menee pieleen

- `REGULAR_WORKPLACE_UNKNOWN`, `CLAIMANT_FACT_CONFLICT` tai muu koodi →
  [Virhekoodit](../reference/error-codes.md)

## Katso myös

- [Ilmoita matkakorvaukset Tulorekisteriin](report-travel-allowances.md)
- [Kirjaa matkaennakko ja sen tilitys](record-travel-advance.md)
- [Sanasto](../reference/glossary.md)
