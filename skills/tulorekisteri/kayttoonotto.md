# Tulorekisteri-yhteyden käyttöönotto

Tehdään kerran. Sen jälkeen ilmoitukset kulkevat [SKILL.md](SKILL.md)-tiedoston mukaan.

## Rajapintaoikeus

Oikeuden hakee käyttäjä Verohallinnon varmennepalvelussa: Tuotantovarmenteet →
organisaatio → Näytä tiedot → Tee korvaava hakemus. Tulorekisterin rajapinnoista
valitaan Palkkatiedot (Web service) käyttötarkoituksella "ilmoitan itse maksamiani
tuloja", ja jos yritys käyttää Vero API:a, sen valinta pidetään ennallaan. Uutta
varmennetta ei tarvita: sama kelpaa molempiin kanaviin, kun oikeus on myönnetty.
Erillinenkin varmenne on sallittu. Hakemus voi tulla hyväksytyksi heti, mutta lähetetty
hakemus ei vielä ole myönnetty oikeus: profiiliin se merkitään vasta, kun käyttäjä
kertoo vahvistuksen näyttävän oikeuden.

## Profiili

Profiilille ei ole komentoa, joten kirjoita kodin tiedosto
`.aggountant/tulorekisteri.json` itse. Siinä ei ole salaisuuksia. Esimerkin tiedot ovat
kuvitteellisia:

```json
{
  "schema_version": 1,
  "mode": "production",
  "business_id": "1234567-1",
  "reporting_permission_active": true,
  "credentials_file": null
}
```

`reporting_permission_active` on käyttäjän lausuma siitä, että oikeus on myönnetty;
ohjelma ei voi tarkistaa sitä. `credentials_file: null` käyttää samaa tunnustiedostoa
kuin ALV-ilmoitus ([vero-fi: tunnukset](../vero-fi/kayttoonotto.md)); erilliselle
varmenteelle annetaan oman, samanmuotoisen tiedoston absoluuttinen polku. Tiedostossa
pitää olla myös `software_key`, vaikka Tulorekisteri ei käytä sitä. Tunnustiedosto on
kodin ulkopuolella, eikä sen sisältö kuulu keskusteluun.

Ohjelma ei erittele, mikä valmiudesta puuttuu: se vaatii tuotantotilan, oikeuden
lausuman, tunnustiedoston tuotantotargetin ja saman Y-tunnuksen sekä voimassa olevan
varmenteen. Valtuutuksella toisen puolesta asiointia ei tueta.

Tiedostossa voi olla myös `employer_report`-lohko. Sitä ei kirjoiteta käsin, vaan sen
tekee komento (alla), joka jättää muun tiedoston ennalleen.

Jos koti on git-repositorio, katso ennen ensimmäistä lähetystä, ettei gitin seurantaan
ole aiemmin lisätty lähetysten outbox-tietokantaa: ohjelman `.gitignore` ei poista jo
seurattua tiedostoa.

## Tulonsaaja

Tulonsaaja lisätään työntekijärekisteriin komennolla `aggountant employee add`. Syöte
sisältää henkilötunnuksen, joten käyttäjä laatii sen itse tiedostoon, antaa sinulle
vain tiedoston nimen ja poistaa tiedoston lisäyksen jälkeen. `aggountant employee show`
tulostaa henkilötunnuksen ja tilinumeron selväkielisinä; älä aja sitä tarpeettomasti.

## Erillisilmoituksen asetukset

Työnantajarekisterissä olevalle yritykselle `aggountant file-wage employer-report
configure` kirjoittaa profiiliin lohkon `employer_report`. Satunnaiselle työnantajalle
lohkoa ei tehdä. Komento vertaa rekisteröinnin alkukuukautta YTJ:n
työnantajarekisteriin ja kieltäytyy, jos ne eroavat. Ehdota silloin rekisterin
kuukautta; poikkeama hyväksytään vain, kun käyttäjä vahvistaa oman kuukautensa, ja
tarkistuksen ohitus (`--offline`) vain hänen luvallaan.

Yhteyshenkilö ja eläkevakuutukset ovat käyttäjän antamia tietoja. Eläkevakuutus
lähetetään "Ei palkanmaksua" -ilmoituksen mukana. Tyhjä lista on oikein vain, jos
yrityksellä ei ole vakuuttamisvelvollisuutta, esimerkiksi kun ainoa työskentelevä on
YEL-vakuutettu omistaja. Kysy tämä; kumpaakaan vaihtoehtoa ei oleteta. Jos ohjelma
hylkää arvon muodon, korjaa se käyttäjän kanssa äläkä muotoile sitä itse kelpaavaksi.
