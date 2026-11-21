# OmaVero

Merkintöjen muoto, tuoreus ja raja tiedon ja luvan välillä ovat
[SKILL.md](SKILL.md)-tiedostossa.

## omavero-kirje-lataus — Kirjeen lataaminen tiedostoksi

- **Palvelu ja sivu:** OmaVero › Kirjeet ja päätökset › kirjeen avaus
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-10-02 · **Vahvistettu:** 2026-11-12
- **Rajapinta:** ei; kirjeitä ei saa Vero API:sta

Kirjeen avaaminen lisää sivulle `./Retrieve/…`-linkin. Hae tiedosto samassa
istunnossa binäärinä: XHR, jossa
`overrideMimeType('text/plain; charset=x-user-defined')`, ja muunna vastaus
base64:ksi ennen tallennusta. Tavallinen tekstinä luettu vastaus rikkoo PDF:n.

## omavero-kirjelista-jarjestys — Kirjelista järjestyy uudelleen

- **Palvelu ja sivu:** OmaVero › Kirjeet ja päätökset › kirjelista
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-10-02 · **Vahvistettu:** 2026-11-12
- **Rajapinta:** ei; kirjeitä ei saa Vero API:sta

Lista järjestyy uudelleen, kun kirje on avattu, joten valitse kirjeet nimen
perusteella, ei sijainnin. Palaa avatusta kirjeestä listaan sivun "Palaa heti"
-linkillä, ei `history.back()`-kutsulla.

## omavero-lomakekentta-syotto — Arvon syöttäminen lomakekenttään

- **Palvelu ja sivu:** OmaVero › lomakkeiden syöttökentät
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-11-19 · **Vahvistettu:** 2026-11-19
- **Rajapinta:** ALV-ilmoitus `aggountant file-vat`; käsin annettu ilmoitus kirjataan `complete-manual`-komennolla

Kenttä hyväksyy arvon vain, kun se syötetään
`document.execCommand('insertText')`-kutsulla kenttään kohdistettuna. Täytä
lomaketta vain käyttäjän nimenomaisesta pyynnöstä, äläkä koskaan ohjelman esteen,
selvittämättömän lähetyksen tai odottavan hyväksynnän ohi.

## omavero-lahetys-vahvistus — Lähetys pyytää erillisen vahvistuksen

- **Palvelu ja sivu:** OmaVero › lomakkeen lähetys
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-11-19 · **Vahvistettu:** 2026-11-19
- **Rajapinta:** ALV-ilmoitus `aggountant file-vat`; käsin annettu ilmoitus kirjataan `complete-manual`-komennolla

Lähetyspainikkeen jälkeen avautuu vielä erillinen vahvistus, joka hyväksytään
OK-painikkeella; lähetys on tehty vasta sen jälkeen. Kumpikin on käyttäjän teko:
pysähdy ennen lähetyspainiketta, älä vasta vahvistuksen kohdalla.
