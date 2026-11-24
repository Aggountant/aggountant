# ADR-11 arkkitehdille: ohjelman omistama luonnos, sinetöity asiakirja ja faktoista johdettu tila

Tämä on yhteenveto ADR-11:n päätöksestä ohjelmistoarkkitehdille. Varsinainen päätös on [ADR-11](adr-11-expense-claim); tämä sivu ei päätä mitään uutta.

## Päätös

Kululasku on ohjelman omistama luonnos, johon kirjoitetaan vain CLI:n kautta (JSON stdinissä). Puhdas `domain::travel::fi` laskee kaavamaiset korvaukset ja kieltäytyy koneluettavalla koodilla, kun syöte vaatii tulkintaa. Yksi sisältöön sidottu komento hyväksyy laskun ja kirjoittaa samassa transaktiossa jäädytetyn asiakirjan liitevarastoon sekä sinetöi tositteet, joiden `attachment_ref` on asiakirjan tiiviste. Hyväksynnän jälkeen kululaskuun ei kirjoiteta. Maksu, Tulorekisteri-ilmoitus ja korjaukset ovat omia faktojaan, ja tila johdetaan niistä joka kerta. Pitkää kululaskuajoa (Run) ei tehdä.

## Missä mikin asuu

| Tieto | Paikka |
|---|---|
| Luonnos | `intake/expense-claims/<claim_id>/draft.json`, atominen kirjoitus, hakemiston oma `.gitignore`. ADR-5:n intake-laji. |
| Kuitit ja todisteet | `artifacts/` keruuhetkellä, luonnoksessa tiiviste |
| Hyväksytty asiakirja | `artifacts/`, kanoninen JSON wire-kooderilla, oma `record_type` ja `schema_version` |
| Tositteet | `fiscal.sqlite`: yksi per kuittirivi (kuitin päivä, Domainin ALV-purku), yksi per kuukausi kaavamaisille korvauksille |
| Linkit (versio, tositteet, maksu, kuittitiiviste) | `fiscal.sqlite`, ketjun ulkopuoliset taulut samassa transaktiossa kuin sinetöinti (`bank_event`-ennakkotapaus) |
| Työpaikkarekisteri, suhde yhtiöön, osakkuus | `pii.sqlite`, kaksiaikainen ja vain lisättävä (`valid_from`/`valid_to` ja `recorded_at`, korjaus `supersedes`-rivillä) |
| Korvausmäärät | `crates/domain/data/fi-travel-allowances.json`, vuosipainokset, ulkomaan päivärahat ISO 3166 -avaimin, tiiviste asiakirjaan |

Tositteen tiiviste kattaa asiakirjan, ja asiakirja kattaa kuitit ja matkan tilannekuvan työpaikkarekisteristä. Maksu, kuittaukset ja rekisterin nykyarvot jäävät sinetin ulkopuolelle. `claim verify` seuraa linkit ja tarkistaa, että asiakirja on olemassa ja tiivistyy oikein, mitä `voucher verify` ei tee.

## Invariantit

- Laskennan syötekentät ovat tyypitettyjä ja validoidaan jokaisessa kirjoituksessa. Vapaa teksti on vain selitystä.
- Kaavamaisia rivejä ei syötetä: ne johdetaan matkasta. Pienempi maksettava määrä sallitaan, suurempi kieltäytyy (`ALLOWANCE_ABOVE_MAXIMUM`).
- Ilman voimassa olevaa työpaikkamerkintää matkan päivälle ei synny verovapaita matkakorvauksia. Lähteiden ratkaisemattomat tapaukset kieltäytyvät. Vuodenvaihteen matkavuorokausi saa pienemmän määrän, ja kesäaika lasketaan kuluneena aikana.
- Taulukkopainos valitaan matkan osan päivästä. Puuttuva painos kieltäytyy (`TRAVEL_RATES_MISSING`), eikä mitään ekstrapoloida.
- Hyväksyntä sitoo esikatselun tiivisteen ja tallentaa jokaisen hyväksyjän. `self_approved` on näkyvä merkintä. Odottavaa "ensimmäinen hyväksyntä tehty" -tilaa ei tallenneta.
- Jokaisesta keskeneräisestä johdetusta tilasta on päättävä komento (taulukko ADR:n luvussa 7). Hylkäys ennen peruuttamatonta tekoa ei jätä jälkeä.
- Suomi on moduuli, ei parametri: matkalla on `jurisdiction: FI`, ja kaikki Suomi-spesifinen on `domain::travel::fi`:ssä. Muuta arvoa ei hyväksytä, eikä trait-kerrosta rakenneta nyt.

## Muutokset nykyiseen

- `ledger` saa kululaskun batch-sinetöinnin ja linkkitaulut (yksisuuntainen migraatio).
- `pii.sqlite` saa rekisteritaulut (yksisuuntainen migraatio, harmiton kodille ilman kululaskuja).
- `TravelLine::DailyAllowance` kantaa joukon päivärahatyyppejä, ja XML tuottaa yhden 331-tapahtuman (`AllowanceCode` `maxOccurs="unbounded"` vendoroidussa skeemassa). Vanha yksi koodi luetaan edelleen.
- `file-wage from-claim` rakentaa syötteen sinetöidyistä asiakirjoista ja voi koota saman hakijan saman maksukuukauden maksut. Yksi maksu kuuluu täsmälleen yhteen ilmoitukseen: jo ilmoitettua maksua ei oteta uuteen ilmoitukseen, vaan se muuttuu vain ilmoituksensa korjauksella.
- Tiliotetuonti tunnistaa kululaskun maksun ja kirjoittaa maksulinkin samassa batchissa.
- Uusi roolitiedosto `expense-claim-payable` (lisäys, `fi-general-2026` → `2740`).

## Miksi ei ajoa

Laskun myöhemmät vaiheet päättää joku muu: käyttäjä maksaa, Tulorekisteri kuittaa ja asiakas maksaa. Ajo jäisi auki viikoiksi, osuisi jokaisen julkaisun versiolukkoon ja tarvitsisi silti peruutus-, "maksettu muualla"- ja korjauskomennot. `file-wage` pysyy ajona, koska se on ohjelman oma peruuttamaton lähetys.

**Lopputulos:** uusia rakenteita on neljä: luonnos, asiakirja, linkkitaulut ja rekisteri. Kaikki muu käyttää olemassa olevia polkuja: kuitin ALV-purkua, tiliotteen batchia, `file-wage`-ajoa ja liitevarastoa.
