# aggountant — käyttäjän opas

Pidät yrityksesi kirjanpitoa tekoälyagentin kanssa. Agentti valmistelee
tositteet tiliotteista ja kuiteista, sinä hyväksyt, ja aggountant tallentaa
kirjanpidon pysyvästi ja tekee ilmoitukset. Tämä opas kertoo, mitä sinä teet,
mitä agentti tekee ja mitä hyväksyt ennen kuin mikään tallentuu tai lähtee
viranomaiselle.

aggountant on versiossa 0.1, ja sillä pidetään jo muutaman yrityksen oikeaa
kirjanpitoa. Kaikki ei ole vielä valmista:
[Mitä on valmista ja mitä ei](reference/status.md).

Oppaan pyynnöt agentille ovat esimerkkejä, eivät taikasanoja. Voit sanoa saman
omin sanoin. Esimerkit koskevat kuvitteellista Esimerkki Oy:tä, ja luvut on
keksitty. Englanninkielinen opas on hakemistossa [`../en/`](../en/README.md).

## Aloita tästä

1. [Onko aggountant sinulle?](start/is-it-for-you.md)
2. [Asenna aggountant ja skillit](start/install.md)
3. [Harjoittele ensimmäinen kuukausi](start/practice-month.md)
4. [Ota yrityksesi kirjanpito käyttöön](start/set-up-company.md)

Haluatko mukaan ensimmäisten joukossa tai kehittämään? [Lähde mukaan](start/early-users.md).

## Joka kuukausi

- [Kuukauden kirjanpito alusta loppuun](tasks/monthly-routine.md): tästä
  linkit kaikkiin vaiheisiin
- [Tuo pankin tiliote](tasks/import-bank-statement.md)
- [Kirjaa kuitit ja ostolaskut](tasks/record-receipts.md)
- [Kirjaa myyntilasku, palkka tai muu erikoiskirjaus](tasks/record-other-entries.md)
- [Katso, miten kuukausi meni](tasks/monthly-report.md)
- [Sulje kuukausi](tasks/close-month.md)
- [Tee ALV-ilmoitus OmaVerossa](tasks/file-vat-omavero.md) tai
  [lähetä se suoraan Verohallinnolle](tasks/file-vat-direct.md)

## Kerran tai harvoin

- [Ota suora ALV-ilmoittaminen käyttöön](tasks/set-up-vero-api.md), myös
  varmenteen vaihto
- [Ilmoita matkakorvaukset Tulorekisteriin](tasks/report-travel-allowances.md)
- [Korjaa virheellinen tosite](tasks/correct-voucher.md)
- [Varmuuskopioi ja palauta kirjanpito](tasks/back-up-and-restore.md)
- [Päivitä aggountant](tasks/upgrade.md)

## Vuoden lopussa

- [Valmistele tilinpäätös](tasks/year-end.md)

## Kun jokin menee vikaan

- [Kun jokin menee vikaan](troubleshooting.md): oireen mukaan
- [Virhekoodit](reference/error-codes.md): ruudulla näkyvän koodin mukaan

## Näin aggountant toimii

- [Sinä, agentti ja aggountant](concepts/division-of-work.md)
- [Sinetöinti ja hyväksyntä](concepts/sealing-and-approval.md)
- [Kaudet, sulku ja tilikausi](concepts/periods.md)
- [Miten ALV-käsittely päätetään](concepts/vat-treatment.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](concepts/filings.md)
- [Yrityskoti: mitä tallentuu minne](concepts/company-home.md)
- [Kirjanpitäjälle](concepts/for-bookkeepers.md)

## Hakuteos

- [Mitä on valmista ja mitä ei](reference/status.md)
- [Virhekoodit](reference/error-codes.md)
- [Vero.fi-valmiuden esteet](reference/vero-fi-blockers.md)
- [Komennot lyhyesti](reference/commands.md)
- [Sanasto](reference/glossary.md)
