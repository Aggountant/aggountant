# Tulorekisteri-yhteyden käyttöönotto

Tehdään kerran. Sen jälkeen ilmoitukset kulkevat [SKILL.md](SKILL.md)-tiedoston mukaan.

## Rajapintaoikeus

Oikeuden hakee käyttäjä Verohallinnon varmennepalvelussa: Tuotantovarmenteet →
organisaatio → Näytä tiedot → **Tee korvaava hakemus**. Lomake luettelee
Tulorekisterin rajapinnat neljänä valintana: Palkkatiedot (SFTP), Palkkatiedot (Web
service), Etuustiedot (SFTP) ja Etuustiedot (Web service). `file-wage` tarvitsee
**Palkkatiedot (Web service)** käyttötarkoituksella "ilmoitan itse maksamiani
tuloja". Etuustiedot ovat etuuksien maksajille, eikä SFTP ole aggountantin kanava.
Jos yritys käyttää Vero API:a, sen valinta jätetään lomakkeella rastitetuksi. Uutta varmennetta ei tarvita: sama kelpaa molempiin
kanaviin, kun oikeus on myönnetty. Erillinenkin varmenne on sallittu.

Hakemuksen voi hyväksyä heti. Myönnetyt oikeudet näkyvät vahvistuksessa nimillä
"Palkkatiedot tuottaja (Web service)" ja, jos Vero API pidettiin, "Vero API (Web
service)". Lähetetty hakemus ei vielä ole myönnetty oikeus: profiiliin se merkitään
vasta, kun käyttäjä kertoo vahvistuksen näyttävän Palkkatiedot tuottaja -oikeuden.

## Profiili

Profiilin kirjoittaa `aggountant tulorekisteri configure`; syötteen muoto on sen
ohjeessa, ja `aggountant tulorekisteri status` erittelee verkotta, mikä valmiudesta
puuttuu ja kenelle se kuuluu. `reporting_permission_active` on käyttäjän lausuma siitä,
että oikeus on myönnetty; ohjelma ei voi tarkistaa sitä. Ilman `credentials_file`-polkua
käytetään samaa tunnustiedostoa kuin ALV-ilmoituksessa
([vero-fi: tunnukset](../vero-fi/kayttoonotto.md)). Tunnustiedostossa pitää olla myös
`software_key`, vaikka Tulorekisteri ei käytä sitä. Tunnustiedosto on kodin
ulkopuolella, eikä sen sisältö kuulu keskusteluun. Valtuutuksella toisen puolesta
asiointia ei tueta.

Profiilin `employer_report`-lohkon kirjoittaa oma komentonsa (alla).

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
