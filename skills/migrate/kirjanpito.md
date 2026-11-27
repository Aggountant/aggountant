# Kirjanpidon siirto

Mitä vanhassa ohjelmassa saa tehdä ja miten aineisto viedään, tarkistetaan ja
arkistoidaan, on [SKILL.md](SKILL.md)-tiedostossa. Avaavat saldot ja muu käyttöönotto
ovat skillissä [aggountant-init](../aggountant-init/SKILL.md); tämä tiedosto kertoo,
mitä vanhasta kirjanpidosta kerätään ja mitä siitä luovutetaan käyttöönottoon.

Uusi kirjanpito alkaa vaihtopäivän avaavista saldoista. Kirjanpidon vaihtopäivä on
muiden alueiden pohja: ALV-ilmoitus lasketaan aggountantin tositteista, joten
aggountant voi ilmoittaa vain kausia, jotka ovat kokonaan sen kirjanpidossa.

## Mitä kerätään

Tilinpäätös, toimintakertomus, kirjanpidot (päiväkirja ja pääkirja), tililuettelo ja
luettelo kirjanpidoista ja aineistoista säilytetään vähintään 10 vuotta tilikauden
päättymisestä. Tositteet, liiketapahtumia koskeva kirjeenvaihto ja muu
kirjanpitoaineisto säilytetään vähintään kuusi vuotta sen vuoden lopusta, jonka aikana
tilikausi päättyi (kirjanpitolaki 2 luku 10 §). Kerää siis jokainen tilikausi, jonka
säilytysaika on kesken, ei vain viimeisintä. Luetteloon tulevat tilikaudet, raportit
kultakin, tositesarjat ja niiden numerovälit, laskujen määrät tiloittain, tiliotteiden
jaksot ja liitteet.

- Tililuettelo tilinumeroineen, nimineen ja ALV-koodeineen. Se on avaavien saldojen
  tilimuunnoksen pohja.
- Saldoluettelo tai tase vaihtopäivältä, ja kesken tilikauden siirryttäessä myös
  tuloslaskelma. Avaava tosite tehdään näistä, ja saldoluettelo on sen liite.
- Päiväkirja ja pääkirja jokaiselta tilikaudelta.
- Tilinpäätökset, toimintakertomukset ja tase-erittelyt. Viimeisin on seuraavan
  tilinpäätöksen vertailutieto.
- Tositteet ja niiden kaikki liitteet. Vanhat kululaskut ovat jo tositteita
  liitteineen ja tulevat arkistoon niiden mukana.
- Osto- ja myyntilaskut kaikissa tiloissa. Vaihtopäivänä avoimet tarvitaan
  käyttöönotossa, koska saldoluettelo ei erittele ostovelkoja eikä myyntisaamisia.
- Tositteiden ja laskujen kommentit ja muistiinpanot. Niissä lukee usein, miksi jokin
  kirjattiin niin kuin kirjattiin, eivätkä ne tule raporttien mukana.
- Pankin tiliotteet sähköisenä (camt.053 tai camt.052) ja PDF-muodossa. Vaihtopäivän
  jälkeiset tuodaan aggountantiin, aiemmat menevät arkistoon.

Kirjanpidoista tarkista, että jokaisen tositteen debet ja kredit täsmäävät ja että
tilikauden summat vastaavat tilinpäätöstä. Laske tositteet ja laskut ja vertaa
luetteloon.

## Luovutus käyttöönottoon

`aggountant-init` tekee saldoluettelosta ja tililuettelosta avaavan tositteen ja kertoo,
mille päivälle se tulee. Arkistosta siihen siirtyy kolme asiaa:

- Vaihtopäivän saldoluettelo ja tililuettelo sekä vaihtopäivänä avoimet laskut.
- Ensimmäinen tuotava tiliote alkaa vaihtopäivää seuraavasta päivästä. Aiemmat
  tapahtumat ovat jo avaavissa saldoissa, ja `aggountant import camt` vaatii
  kirjauksen tiliotteen jokaiselle tapahtumalle, joten vaihtopäivän ylittävä tiliote
  kirjaisi samat tapahtumat toiseen kertaan. Pyydä pankista uusi rajaus.
- Ennen vaihtopäivää päivätty lasku, joka maksetaan vasta sen jälkeen, voi olla jo
  avaavissa saldoissa ostovelkana. Tarkista se saldoluettelosta ja avoimien laskujen
  luettelosta ennen kuin kirjaat laskun kuluksi; muuten sama kulu on kirjattu kahdesti,
  kerran kummassakin järjestelmässä. Sama koskee hyväksyttyä mutta maksamatonta
  kululaskua.

Kun avaava tosite on sinetöity, kirjaa alueen tietue ([SKILL.md](SKILL.md)).
