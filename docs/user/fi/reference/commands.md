# Komennot lyhyesti

Taulukosta näet, mihin tehtävään kukin aggountantin komentoryhmä liittyy, ja
tunnistat sen avulla, mitä agentti ajaa. Valitsimet ja syötteen muodon kertoo
ohjelma itse: `aggountant --help` ja esimerkiksi
`aggountant voucher seal --help`. JSON-syötettä lukeva komento näyttää syötteen
kentät ja esimerkin ohjeensa INPUT-osiossa. Jokainen komento hyväksyy
valitsimet `--home` (mikä koti) ja `--json` (koneluettava tuloste agentille).

| Komento | Mitä se tekee | Missä oppaassa |
|---|---|---|
| `init` | perustaa yrityskodin ja oletustilikartan | [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) |
| `home upgrade` | lisää olemassa olevaan kotiin perustiedot, joita uusi versio tarvitsee | [Päivitä aggountant](../tasks/upgrade.md) |
| `config` | näyttää, mikä koti on käytössä ja mistä tieto tuli, sekä asetustiedoston paikan | [Miten ohjelma löytää kodin](../concepts/company-home.md#miten-ohjelma-löytää-kodin) |
| `skill` | listaa, asentaa ja näyttää agentin skillit | [Asenna aggountant ja skillit](../start/install.md) |
| `explain` | selittää virhekoodin: mitä tapahtui ja mitä tehdään | [Virhekoodit](error-codes.md) |
| `chart` | listaa tilikartan tilit | [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) |
| `knowledge` | lukee ja ylläpitää yrityksen tallennettuja faktoja ja käytäntöjä (`knowledge company`) | [Yrityksen tietovarasto](../concepts/company-home.md#yrityksen-tietovarasto) |
| `import camt` | lukee pankin tiliotteen (camt.053 tai camt.052) ja sinetöi sen kirjaukset; vanha nimi `import camt053` toimii yhä | [Tuo pankin tiliote](../tasks/import-bank-statement.md) |
| `artifact` | arkistoi lähdetiedoston, kuten kuitin tai vahvistuksen | [Kirjaa kuitit ja ostolaskut](../tasks/record-receipts.md) |
| `capture-receipt record` | kirjaa tavallisen ostokuitin esikatselun ja hyväksynnän kautta | [Kirjaa kuitit ja ostolaskut](../tasks/record-receipts.md) |
| `voucher` | laatii tositeluonnoksen, tarkistaa ja sinetöi sen, hakee sinetöityjä tositteita, tarkistaa ketjun ja listaa päivänä voimassa olevat ALV-koodit | [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](../tasks/record-other-entries.md), [Korjaa virheellinen tosite](../tasks/correct-voucher.md) |
| `report statements` | tekee kuukausiraportin HTML- tai JSON-muodossa | [Katso, miten kuukausi meni](../tasks/monthly-report.md) |
| `obligation` | listaa velvoitteet ja merkitsee avoimen velvoitteen ratkaistuksi | [Velvoitteet](../concepts/periods.md#velvoitteet) |
| `run` | esikatselee kuukauden sulun, aloittaa sulkuajon, vie siihen hyväksyntäsi, jatkaa keskeytynyttä ajoa, näyttää ajon tilan ja siirtää ajon uuteen versioon | [Sulje kuukausi](../tasks/close-month.md) |
| `period` | näyttää kauden tilan (`open`, `closing`, `sealed`) ja sen, onko kauden ALV-ilmoitus tehty | [Kaudet, sulku ja tilikausi](../concepts/periods.md) |
| `fiscal-year` | näyttää tilikauden päivät ja tilan (`open`, `sealed`) ja määrittelee kalenterivuodesta poikkeavan tilikauden | [Tilikauden tila](../concepts/periods.md#tilikauden-tila) |
| `file-vat` | esikatselee ALV-ilmoituksen, lähettää sen suoraan, lähettää korvaavan ilmoituksen, näyttää lähetetyn ilmoituksen tilan, selvittää lähetyksen, jonka lopputulos jäi tuntemattomaksi, ja kirjaa käsin tehdyn ilmoituksen | [Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md), [Lähetä ALV-ilmoitus suoraan Verohallinnolle](../tasks/file-vat-direct.md) |
| `tax-payments` | tallentaa ja näyttää yrityksen Verohallinnon maksutiedot: tilinumeron ja viitenumerot, joista ALV-ilmoituksen maksuehdotus ottaa omansa | [Lähetä ALV-ilmoitus suoraan Verohallinnolle](../tasks/file-vat-direct.md#tallenna-verohallinnon-maksutiedot-kerran) |
| `tulorekisteri` | tallentaa Tulorekisteri-yhteyden profiilin ja tarkistaa valmiuden | [Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md) |
| `vero-fi` | tallentaa suoran ilmoittamisen asetukset, tarkistaa valmiuden ja ajaa hyväksyntätestit | [Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md) |
| `employee` | lisää työntekijän työntekijärekisteriin ja näyttää rekisterin tiedot | [Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md) |
| `claim compute` | laskee matkan verovapaat korvaukset esikatseluna kirjaamatta mitään | [Laske matkan verovapaat korvaukset](../tasks/compute-travel-allowances.md) |
| `claimant` | kirjaa kululaskun tekijän paikat, varsinaisen työpaikan, suhteen yhtiöön ja pääomaosakkuuden päivättyinä tietoina, korjaa niitä ja luokittelee matkan paikat | [Kirjaa varsinainen työpaikka](../tasks/record-regular-workplace.md) |
| `file-wage` | lähettää matkakorvausilmoituksen Tulorekisteriin, ja `file-wage correction` korvaa tai mitätöi hyväksytyn ilmoituksen | [Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md) |
| `file-wage employer-report` | antaa työnantajan erillisilmoituksen, listaa puuttuvat kuukaudet, tarkistaa rekisteröinnin YTJ:stä ja tuo raportin 630 | [Anna työnantajan erillisilmoitus](../tasks/file-employer-report.md) |
| `close-year` | tilinpäätösajo: kirjaa poistot, jaksotukset ja oikaisut ja sinetöi tilikauden | [Valmistele tilinpäätös](../tasks/year-end.md) |

## Ohjelmassa, mutta ei käytössä

Näitä komentoja ei voi vielä käyttää oikeaan kirjanpitoon. Syyn ja korvaavan
tavan kertoo [Mitä on valmista ja mitä ei](status.md).

| Komento | Mitä se tekisi |
|---|---|
| `run-payroll` | palkka-ajo |
| `hunt-receipts`, `intake` | kuittien automaattinen haku ja sen löydökset |
| `capture-receipt start`, `answer`, `resume` | poikkeuksellisen kuitin konsultointi |

## Katso myös

- [Sanasto](glossary.md)
- [Virhekoodit](error-codes.md)
- [Mitä on valmista ja mitä ei](status.md)
