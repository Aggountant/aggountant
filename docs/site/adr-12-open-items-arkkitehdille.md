# ADR-12 arkkitehdille: lajista riippumaton erä ja selvityslinkki kirjanpitokannassa tositeketjun ulkopuolella

Tämä on yhteenveto ADR-12:n ehdotuksesta ohjelmistoarkkitehdille. Varsinainen päätös on [ADR-12](adr-12-open-items); tämä sivu ei päätä mitään uutta. ADR on ehdotus, kunnes tuotteen omistaja hyväksyy sen.

## Päätös

Avoin erä on sinetöity velka- tai saamisvienti, jonka avoin määrä johdetaan joka kerta erän synnyttäneistä vienneistä ja selvityslinkeistä. Saldoa, maksettu-lippua tai tilaa ei tallenneta. Erän identiteetti ja linkit ovat kirjanpitokannassa tositteiden rinnalla mutta niiden tiivisteen ulkopuolella, kuten pankkitapahtuman identiteetti (`bank_event`).

## Neljä taulua, joihin vain lisätään

| Taulu | Rivi kertoo |
|---|---|
| Erä | ohjelman antama läpinäkymätön tunniste, laji (`expense_claim`; varattuina `sales_invoice`, `purchase_invoice`, `tax`, `claimant_receivable`), lajin oma avain lajin sisällä uniikkina ja tunnisteet (viite omassa indeksoidussa sarakkeessaan) |
| Erän synty | sinetöity vienti (tositenumero ja vientiindeksi), joka synnyttää tai muuttaa erää |
| Selvitys | sinetöity vienti selvittää erästä summan, lajina `bank`, `offset`, `other_account`, `credit_note` tai `credit_loss`, ja näyttö |
| Erän fakta | päivätty ulkoinen tieto; ensimmäinen laji `payment_scheduled` |

Sarakkeiden nimet omistaa toteuttava viipale. Jokaisella rivillä on kirjaushetki ja kirjaaja. Korjaus on uusi rivi, joka nimeää korvaamansa rivin (uniikki) ja syyn, tai peruutusrivi, kuten työpaikkarekisterissä. Liipaisimet estävät muutokset. Lajit ja koodit tarkistaa ohjelma, ei `CHECK`-ehto, jotta uusi laji ei vaadi taulun uudelleenrakentamista.

## Invariantit

- Mikään koodi ei lue erää `natural_key`:n tekstistä. Avain pysyy idempotenssiavaimena.
- Selvitetty summa tallennetaan, koska yksi siirto voi maksaa kaksi erää. Se on kohdistuspäätös, ei saldo. Saman viennin kohdistukset eivät saa ylittää vientiä.
- Kaksoismaksun tarkistus tehdään tuonnin `BEGIN IMMEDIATE` -transaktiossa ennen linkin kirjoitusta. Hylkäys koskee koko erää, kirjataan hylkäyslokiin ja nimeää erän voimassa olevat selvitykset.
- Uudelleentuonti samalla selvityksellä on havainto. Eri selvitys pysäyttää tuonnin; linkin vaihto on korjausrivi.
- Ajastus on fakta kahdella ajalla (maksupäivä ja kirjaushetki). Se ei sulje erää eikä ole oma tilansa, vaan tarkentaa tilaa "hyväksytty, maksamaton". Vanheneminen mitataan tuotujen tiliotteiden kattavuudesta ja tuonnin päivätoleranssista (`DATE_TOLERANCE_DAYS`), ei kalenterista. Vanhentunut ajastus ei palauta laskua maksettavaksi.
- Taulut eivät ole tiiviste-ketjussa, koska ne eivät muuta raportoitua lukua. `voucher verify` ei lue niitä; `claim verify` tarkistaa, että viennit ovat olemassa, ovat lajin roolitilillä eivätkä ole ylikohdistettuja.
- Toteutus on yksi, `ledger`-kerroksessa. Kululaskun komennot ovat sen nimiä kululaskulle. Yleinen `open-item`-komentoryhmä tulee vasta ensimmäisen muun lajin mukana.

## Migraatio ja ajoitus

`expense-claim-approve-seal` tekee kululaskun ainoan sallitun kirjanpitokannan migraation. Se luo neljä taulua tyhjinä kululaskun versiotaulun rinnalle, ja hyväksyntä kirjoittaa erän ja sen synnyn rivit samassa transaktiossa kuin tositteet. Myöhemmät lajit eivät tarvitse migraatiota. Kululaskukohtaiset taulut olisivat vaatineet myöhemmin toisen migraation, joka kopioi tuotannon kululaskut uusiin tauluihin.

## Yritykset ilman eriä

Migraatio luo tyhjiä tauluja. Tuonti ja sulut tekevät yhden indeksoidun olemassaolokyselyn ja ohittavat muun, eikä tuonti lisää toista läpikäyntiä. Jokainen viipale todistaa regressiotestillä, että synteettisen kodin tuonnin, sulun ja tilinpäätöksen esikatselu on tavulleen sama kuin ennen.

## Rajaukset

Laskutus, muistutukset, perintä, maksujen käynnistäminen, tallennettu saldo tai tila, erän sulkeminen ajastuksen tai ulkoisen tilan perusteella, epäselvän kohdistuksen hyväksyminen ilman ihmistä, ulkoisen laskulistan peilaaminen ja sulun estävä avoin erä.

**Lopputulos:** uusia rakenteita on yksi tauluperhe ja yksi tarkistus tuonnin transaktiossa. Muu käyttää olemassa olevia polkuja: sinettiä, tiliotteen tuontia, hylkäyslokia ja työpaikkarekisterin korjausmallia.
