# Selaimen vaatimukset

Mitä selaimen on sallittava, jotta verkkopalveluja voi käyttää. Nämä ovat
vaatimuksia, eivät koneen asetusohjeita: miten selain on asetettu sallimaan ne, on
käyttäjän asia. Jos jokin näistä estyy, kerro käyttäjälle, mikä palvelu tarvitsee
mitä, äläkä muuta selaimen asetuksia itse.

Merkintöjen muoto, tuoreus ja raja tiedon ja luvan välillä ovat
[SKILL.md](SKILL.md)-tiedostossa.

## selain-ponnahdusikkunat — Ponnahdusikkunat sallittava

- **Palvelu ja sivu:** OmaVero ja Tulorekisterin asiointipalvelu › koko palvelu
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-11-19 · **Vahvistettu:** 2026-11-19
- **Rajapinta:** ei

Palvelut avaavat osan sivuistaan ja tiedostoistaan ponnahdusikkunaan. Selaimen on
sallittava ponnahdusikkunat näiden palvelujen verkkotunnuksille; estetty ikkuna
näyttää siltä, ettei painike tehnyt mitään.

## selain-useat-lataukset — Useat lataukset kerralla sallittava

- **Palvelu ja sivu:** OmaVero ja Tulorekisterin asiointipalvelu › tiedostojen lataus
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-11-19 · **Vahvistettu:** 2026-11-19
- **Rajapinta:** ei

Palvelut voivat antaa useita tiedostoja samalla toiminnolla. Selaimen on sallittava
useat automaattiset lataukset näille verkkotunnuksille, muuten vain ensimmäinen
tiedosto tallentuu eikä estosta välttämättä näy ilmoitusta.
