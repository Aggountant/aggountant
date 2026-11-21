# Tulorekisterin asiointipalvelu

Merkintöjen muoto, tuoreus ja raja tiedon ja luvan välillä ovat
[SKILL.md](SKILL.md)-tiedostossa.

## tulorekisteri-raportti-tilaus — Raportin 630 tilaaminen

- **Palvelu ja sivu:** Tulorekisterin asiointipalvelu › Uusi tilaus
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-11-18 · **Vahvistettu:** 2026-11-18
- **Rajapinta:** ei; raporttia ei saa rajapinnasta. Tuonti: `aggountant file-wage employer-report import-transcript`

Raportti tilataan, sitä ei ladata suoraan: valitse Uusi tilaus ja raportiksi
*Maksajan palkkatietojen yhteenveto (csv)* (630). Rajaukset (kohdekausi, kieli,
vastaanottohetki) tulevat tehtäväskillistä; `tulorekisteri` kertoo ne ja syyn niille.
Tilaus ei anna tiedostoa heti; valmis raportti haetaan Tehdyt tilaukset -sivulta
(`tulorekisteri-raportti-lataus`).

## tulorekisteri-raportti-lataus — Valmiin raportin lataaminen

- **Palvelu ja sivu:** Tulorekisterin asiointipalvelu › Tehdyt tilaukset
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-11-18 · **Vahvistettu:** 2026-11-18
- **Rajapinta:** ei; raporttia ei saa rajapinnasta. Tuonti: `aggountant file-wage employer-report import-transcript`

Valmis raportti ladataan Tehdyt tilaukset -sivulta zip-pakettina. Pura paketti ja
anna tuonnille sen sisältämä csv-tiedosto, älä pakettia.
