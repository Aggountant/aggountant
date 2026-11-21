# Suoran ALV-ilmoittamisen käyttöönotto

Suora ilmoittaminen on yrityksen oma yhteys Verohallintoon: oma ohjelmistoavain, oma
sopimus ja oma varmenne. Käyttöönotto on valmis, kun `aggountant vero-fi status` kertoo
`ready` eikä esteitä ole. Siihen asti ilmoitus annetaan käsin ([SKILL.md](SKILL.md)).

## Kuka tekee mitä

Portaali-, sopimus- ja varmennetoimet ovat käyttäjän, koska ne ovat yrityksen
oikeustoimia: hän rekisteröi ohjelmistonsa Vero API -portaalissa ja saa oman
ohjelmistoavaimensa, yritys tekee rajapintasopimuksen, hankkii tuotantovarmenteen
Verohallinnon varmennepalvelusta ja aktivoi tuotantoluvan. Aggountantilla ei ole
yhteistä avainta. Sinä kokoat ei-salaiset tiedot, ajat valmiusarvion ja verifioinnin
ja kerrot, kenen vuoro kunkin esteen kohdalla on.

## Tunnukset

Käyttäjä tallentaa tunnukset itse JSON-tiedostoon kirjanpitokodin ulkopuolelle,
esimerkiksi hakemistoon `~/.config/aggountant/credentials/`, ja kertoo sinulle vain
polun. Tiedostossa ovat `target` (`production`), `business_id`, `software_key` sekä
PEM-muodossa `client_certificate_pem`, `client_private_key_pem` ja
`ca_certificate_pem`. Kodin alla oleva tunnustiedosto on valmiuden este. Polku
tallentuu koneen omiin asetuksiin, ei kotiin, joten jokaisella koneella on omansa.

Kun varmenne pitää yksilöidä jollekulle, esimerkiksi Verohallinnon tuelle, valmiusarvion
näyttämä SHA-256-sormenjälki riittää eikä ole salainen.

## Asetukset ja attestoinnit

Ei-salaiset asetukset kirjoittaa `aggountant vero-fi configure`; sen ohje kertoo
syötteen. Komento yhdistää annetut kentät tallennettuihin ja näyttää muutokset, joten
anna vain se, mikä muuttuu. Verifioinnin tallentamaa näyttöä se ei suostu pudottamaan.
`--replace` kirjoittaa koko tiedoston uudelleen ja pudottaa näytön: sitä tarvitaan vain
rikkinäisen asetustiedoston korvaamiseen.

Neljä attestointia ovat käyttäjän lausumia asioista, joita ohjelma ei voi tarkistaa:
ohjelmisto on rekisteröity, rajapintasopimus on voimassa, tuotantovarmenteen
verifiointi on hyväksytty ja tuotantolupa on aktiivinen. Ohjelma luottaa niihin, joten
väärä `true` tekee valmiusarviosta väärän. Kirjaa siis kukin todeksi vasta, kun
käyttäjä vahvistaa toimen tehdyksi, ei silloin kun hakemus on lähetetty. Verifioinnin attestoinnin kirjaa onnistunut `vero-fi verify-portal` itse.

## Järjestys

1. Tunnustiedosto paikalleen ja asetukset tilassa `disabled`, mukana Y-tunnus ja
   yhteyshenkilö. Verifiointi lukee nämä.
2. Käyttäjä valitsee portaalissa ALV:n hyväksyntätestit.
3. `aggountant vero-fi verify-portal` ajaa Verohallinnon kolme kiinteää testitapausta
   tuotantovarmenteella. Se ei anna ilmoitusta eikä muuta tilaa, mutta se on
   ensimmäinen yhteys Verohallinnon tuotantoon, joten pyydä siihen käyttäjän päätös.
   Onnistunut ajo tallentaa näytön asetuksiin.
4. Tuotantotila (`mode: production`) käyttäjän päätöksellä, kun attestoinnit on
   vahvistettu. Aktivointi sidotaan nykyisen varmenteen sormenjälkeen, jonka
   valmiusarvio näyttää. Ohjelma kirjoittaa tuotantotilan vain, jos valmius on kunnossa;
   muuten mikään ei muutu.

`aggountant vero-fi verify --target test` on vapaaehtoinen vianetsintä erillisillä
testitunnuksilla. Tuotanto ei tarvitse sitä.

## Varmenteen uusinta

Varmenne on voimassa kaksi vuotta, ja valmiusarvio varoittaa ennen umpeutumista.
Uudella varmenteella on uusi sormenjälki, ja verifioinnin näyttö ja aktivointi on
sidottu vanhaan. Kun käyttäjä on päivittänyt tunnustiedoston, verifiointi ja
tuotantoaktivointi tehdään uudella sormenjäljellä uudelleen, ja käyttäjältä
vahvistetaan ne attestoinnit, jotka koskevat varmennetta. Vanhan näytön siirtäminen
uudelle varmenteelle tekisi valmiusarviosta väärän.
