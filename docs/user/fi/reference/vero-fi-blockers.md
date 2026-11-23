# Vero.fi-valmiuden esteet

`aggountant vero-fi status` näyttää esteet ja varoitukset, jotka koskevat
suoraa ALV-ilmoittamista. Tarkistus on paikallinen eikä ota verkkoyhteyttä.
Ohjelma tekee saman tarkistuksen juuri ennen jokaista suoraa ALV-ilmoitusta
ja lähettää ilmoituksen vain, kun valmius on kunnossa (`ready`) eikä esteitä
ole.

Jokaisella esteellä on omistaja. Esteen `human` poistat sinä: annat puuttuvan
tiedon, vahvistat agentille tekemäsi portaalitoimen tai hoidat asian
Verohallinnon portaalissa. Este `agent` on asetus, jonka agentti voi korjata.
Käyttöönoton vaiheet ovat sivulla
[Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md).

## Sinun esteesi (`human`)

| Este | Mitä se tarkoittaa |
|---|---|
| `VERO_DISABLED` | suora ilmoittaminen ei ole päällä (asetusten tila ei ole `production`) |
| `VERO_BUSINESS_ID_MISSING` | Y-tunnus puuttuu asetuksista |
| `VERO_CONTACT_NAME_MISSING` | yhteyshenkilön nimi puuttuu asetuksista |
| `VERO_CONTACT_PHONE_MISSING` | yhteyshenkilön puhelinnumero puuttuu asetuksista |
| `VERO_SOFTWARE_REGISTRATION_UNATTESTED` | et ole vahvistanut ohjelmiston rekisteröintiä |
| `VERO_API_AGREEMENT_UNATTESTED` | et ole vahvistanut rajapintasopimusta |
| `VERO_PRODUCTION_VERIFICATION_UNATTESTED` | hyväksyntätestejä ei ole kirjattu hyväksytyiksi |
| `VERO_PRODUCTION_PERMISSION_UNATTESTED` | et ole vahvistanut tuotantolupaa |
| `VERO_CREDENTIAL_BUSINESS_ID_MISMATCH` | tunnustiedoston Y-tunnus ei vastaa asetuksia |
| `VERO_DELEGATED_USE_UNSUPPORTED` | tunnuksissa on valtuutustunniste, eikä ohjelma tue ilmoittamista toisen puolesta |
| `VERO_CERTIFICATE_NOT_YET_VALID` | varmenne ei ole vielä voimassa |
| `VERO_CERTIFICATE_EXPIRED` | varmenne on vanhentunut |
| `VERO_ACTIVATION_CERTIFICATE_MISMATCH` | tuotantoaktivointi on tehty eri varmenteella kuin nykyinen |

## Agentin esteet (`agent`)

| Este | Mitä se tarkoittaa |
|---|---|
| `VERO_PROFILE_INVALID` | kodin `vero-filing.json` puuttuu tai on virheellinen, esimerkiksi kun asetuksia ei ole vielä tehty |
| `VERO_PORTAL_VERIFICATION_MISSING` | hyväksyntätestien ajosta ei ole näyttöä |
| `VERO_PORTAL_VERIFICATION_CERTIFICATE_MISMATCH` | hyväksyntätestit on ajettu eri varmenteella kuin nykyinen |
| `VERO_CREDENTIAL_PATH_MISSING` | tunnustiedoston polkua ei ole määritetty |
| `VERO_CREDENTIAL_IN_ACCOUNTING_HOME` | tunnustiedosto on kirjanpitokodin sisällä |
| `VERO_CREDENTIAL_UNAVAILABLE` | tunnustiedostoa ei voi lukea |
| `VERO_CREDENTIAL_INVALID` | tunnustiedosto on virheellinen |
| `VERO_CREDENTIAL_TARGET_MISMATCH` | tunnukset ovat testi- eikä tuotantoympäristön |

## Varoitukset

Varoitus ei estä ilmoittamista.

| Varoitus | Mitä se tarkoittaa |
|---|---|
| `VERO_CERTIFICATE_EXPIRES_WITHIN_90_DAYS` | varmenne vanhenee 90 päivän kuluessa |
| `VERO_CERTIFICATE_EXPIRES_WITHIN_30_DAYS` | varmenne vanhenee 30 päivän kuluessa |
| `VERO_CERTIFICATE_EXPIRES_WITHIN_7_DAYS` | varmenne vanhenee 7 päivän kuluessa |

Uuden varmenteen käyttöönotto on kohdassa
[Kun varmenne vaihtuu](../tasks/set-up-vero-api.md#kun-varmenne-vaihtuu).

## Katso myös

- [Ota suora ALV-ilmoittaminen käyttöön](../tasks/set-up-vero-api.md)
- [Virhekoodit](error-codes.md)
