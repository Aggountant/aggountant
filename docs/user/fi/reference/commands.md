# Komennot lyhyesti

Tämä sivu näyttää, mihin tehtävään kukin aggountantin komentoryhmä liittyy,
jotta tunnistat, mitä agentti ajaa. Valitsimia ja syötteiden muotoa tässä ei
ole: ne kertoo ohjelma itse, `aggountant --help` ja esimerkiksi
`aggountant voucher seal --help`. Jokainen komento hyväksyy valitsimet
`--home` (mikä koti) ja `--json` (koneluettava tuloste agentille).

| Komento | Mitä se tekee | Missä oppaassa |
|---|---|---|
| `init` | perustaa yrityskodin ja oletustilikartan | [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) |
| `config` | näyttää, mikä koti on käytössä ja mistä tieto tuli; asetustiedoston paikka | [Yrityskoti](../concepts/company-home.md) |
| `skill` | listaa, asentaa ja näyttää agentin skillit | [Asenna aggountant ja skillit](../start/install.md) |
| `chart` | listaa tilikartan tilit | [Ota yrityksesi kirjanpito käyttöön](../start/set-up-company.md) |
| `knowledge` | yrityksen tallennetut faktat ja käytännöt (`knowledge company`) | [Yrityskoti](../concepts/company-home.md#yrityksen-tietovarasto) |
| `import camt053` | lukee pankin tiliotteen ja sinetöi sen kirjaukset (myös camt.052) | [Tuo pankin tiliote](../tasks/import-bank-statement.md) |
| `artifact` | arkistoi lähdetiedoston, kuten kuitin tai vahvistuksen | [Kirjaa kuitit ja ostolaskut](../tasks/record-receipts.md) |
| `capture-receipt` | kirjaa tavallisen ostokuitin esikatselun ja hyväksynnän kautta | [Kirjaa kuitit ja ostolaskut](../tasks/record-receipts.md) |
| `voucher` | tositeluonnos, tarkistus, sinetöinti, haku ja ketjun tarkistus | [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](../tasks/record-other-entries.md), [Korjaa virheellinen tosite](../tasks/correct-voucher.md) |
| `report statements` | kuukausiraportti HTML- tai JSON-muodossa | [Katso, miten kuukausi meni](../tasks/monthly-report.md) |
| `obligation` | avoimet velvoitteet ja niiden ratkaiseminen | [Kaudet](../concepts/periods.md#velvoitteet) |
| `run` | kuukauden sulkuajo: aloitus, hyväksyntä, jatkaminen, tila ja siirto uuteen versioon | [Sulje kuukausi](../tasks/close-month.md) |
| `period` | kauden tila (`open`, `closing`, `sealed`) ja ALV-ilmoituksen tila | [Kaudet](../concepts/periods.md) |
| `fiscal-year` | tilikauden tila (`open`, `sealed`) | [Kaudet](../concepts/periods.md#tilikauden-tila) |
| `file-vat` | ALV-ilmoituksen esikatselu, suora lähetys, tilan selvitys ja käsin tehdyn ilmoituksen kirjaus | [Tee ALV-ilmoitus OmaVerossa](../tasks/file-vat-omavero.md), [suoraan](../tasks/file-vat-direct.md) |
| `vero-fi` | suoran ilmoittamisen asetukset, valmiuden tarkistus ja hyväksyntätestit | [Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md) |
| `employee` | työntekijärekisteri | [Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md) |
| `file-wage` | matkakorvausilmoitus Tulorekisteriin | [Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md) |

## Ohjelmassa, mutta ei käytössä

Nämä komennot ovat ohjelmassa, mutta niitä ei voi vielä käyttää oikeaan
kirjanpitoon. Syy ja korvaava tapa ovat sivulla
[Mitä on valmista ja mitä ei](status.md).

| Komento | Mitä se tekisi |
|---|---|
| `run-payroll` | palkka-ajo |
| `hunt-receipts`, `intake` | kuittien automaattinen haku ja sen löydökset |
| `capture-receipt start`, `answer`, `resume` | poikkeuksellisen kuitin konsultointi |
| `close-year` | tilinpäätösajo |

## Katso myös

- [Sanasto](glossary.md)
- [Virhekoodit](error-codes.md)
