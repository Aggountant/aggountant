# ADR-8 kirjanpitäjälle: vanha tilikartta tuodaan sellaisenaan SIE4:stä

Tämä on yhteenveto ADR-8:n päätöksestä kirjanpidon ammattilaiselle. Varsinainen päätös on [ADR-8](adr-8-chart-adoption); tämä sivu ei päätä mitään uutta.

## Mitä päätettiin

Aggountantissa on kaksi polkua. Uusi yritys saa oletuksena aggountantin oman standarditilikartan. Jos yrityksellä on jo kirjanpito toisessa järjestelmässä, se saa jatkaa **omalla tilikartallaan**.

Oma kartta otetaan käyttöön vanhan kirjanpito-ohjelman **SIE4-viennistä**. Sen `#KONTO`-osa sisältää tilit koneellisesti luettavana, myös taseen tilit. Tämä on parempi kuin se, että `init`-komento yritettäisiin tehdä yhden yrityksen vanhan kartan näköiseksi: standardikartta pysyy standardina ja yrityksen kartta säilyy yrityksen omana.

## Mitä tämä tarkoittaa käytännössä

- Yrityksen vanha kartta tuodaan omana versionaan. Sen kaikki tilit säilyvät omilla numeroillaan eivätkä katoa geneerisen kartan alle.
- Kartta tarkistetaan ennen käyttöä. Erityisesti ALV-tileille merkitään niiden rooli: esimerkiksi `1798` on käännetyn veron vähennettävä osto-ALV ja `2992` vastaava myynti-ALV. Ohjelma ei saa päätellä tätä pelkästä tilinumerosta eikä tehdä nollailmoitusta, jos roolia ei tunneta.
- Jokaisesta karttaversiosta jää oma jäljitettävä snapshot. Jos karttaan lisätään tili seuraavana vuonna, vanhan kauden tositteet säilyvät vanhan kartan mukaisina.
- Jos yritys vaihtaa myöhemmin standardikarttaan, se on hallittu **katkos**: vanha kartta jää talteen, tilit mapataan ja alkusaldot käsitellään erikseen. Vanhaa karttaa ei kirjoiteta yli.

## Mitä tämä ei vielä ratkaise

SIE4 sisältää myös vanhat tositteet. ADR-8 päättää kartan tuonnin, ei sitä, tehdäänkö vanhasta historiasta aggountantin sinetöityä kirjanpitoa. Se ratkaistaan erillisessä onboarding-auktoriteetin ADR:ssä. Nykyinen suositus on, että vanhan järjestelmän historia pysyy vertailu- ja analyysiaineistona ja uusi sinetöity kirja alkaa ensimmäisestä live-kaudesta alkusaldoilla.

**Lopputulos:** uusi yritys pääsee nopeasti alkuun standardikartalla. Olemassa oleva yritys jatkaa tutuilla tileillään, ja ALV-tilien merkitys tarkistetaan ennen kuin niitä käytetään kirjanpidossa.
