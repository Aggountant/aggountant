# Matkaennakot

Ohjelmassa ei ole matkaennakolle omaa reittiä, joten ennakko ja sen tilitys
kirjataan nykyisillä komennoilla oletustilikartan tileille. Ennakko on yhtiön
saaminen henkilöltä, ei kulu eikä tuloa: kulu syntyy vasta kululaskusta, ja
ennakko kuitataan sitä vastaan.

| Tapahtuma | Debet | Kredit | Miten |
|---|---|---|---|
| Ennakon maksu | `1730` Muut lyhytaikaiset saamiset | pankki | tiliotteen rivi, vastatili `1730`, ei ALV:tä |
| Kululasku | kulut | `2740` Muut lyhytaikaiset velat | kuten ilman ennakkoa, aina bruttona |
| Ennakon kuittaus | `2740` | `1730` | `voucher seal`, liitteenä matkalasku |
| Yhtiö maksaa erotuksen | `2740` | pankki | tiliotteen rivi |
| Henkilö palauttaa erotuksen tai koko ennakon | pankki | `1730` | tiliotteen rivi |

Kirjoita ennakon ja kuittauksen selitteeseen henkilö niin kuin pankki hänet
näyttää ja matka. Henkilötunnus ja tilinumero eivät kuulu selitteeseen, koska
tositteessa ei ole henkilökenttää ja selite on ainoa paikka, josta ennakon
omistaja näkyy.

Kululasku kirjataan samoin kuin ilman ennakkoa: kuitit
`capture-receipt record`illa `2740`:aa vastaan ja kilometrikorvaukset ja päivärahat
erikoiskirjauksena (`7300` Matkakulut, `7350` Päivärahat). Älä jaa kululaskun
kreditiä ennakon ja velan kesken. Kuittaus on maksu, ja omana tositteenaan sillä
on oma päivä, joka on Tulorekisterin maksupäivä.

Kuittaa enintään ennakon avoin osa ja enintään kululaskun summa. Päivää kuittaus
samalle päivälle kuin erotuksen maksu tai palautus; jos erotusta ei ole,
kululaskun hyväksymispäivälle. Silloin kuittaus ja pankkimaksu ovat samassa
kuukaudessa, eikä kysymystä siitä, kumpi päivä ilmoitetaan, synny. Johda
`natural_key` kululaskusta, jotta saman kuittauksen toinen sinetöinti palauttaa
olemassa olevan tositteen.

Ennakkoa ei ilmoiteta Tulorekisteriin koskaan, ei maksua eikä palautusta.
Kululaskun kilometrikorvaukset (311) ja päivärahat (331) ilmoitetaan kokonaan,
maksettiin ne pankista tai kuittaamalla. Ilmoitus on `tulorekisteri`-skillin
työtä.

Esimerkki: Esimerkki Oy maksaa Maija Meikäläiselle 500,00 € ennakon. Kululasku on
620,00 € (kuitit 260,00 €, korvaukset 360,00 €). Kuittaus D `2740` 500,00 /
K `1730` 500,00, ja erotus 120,00 € maksetaan samana päivänä tiliotteelta
`2740`:aa vastaan. Kun kaikki on kirjattu, `1730` on nolla ja `2740`:ssä ei ole
tästä laskusta mitään.

## Avoin ennakko

Selitä `1730`:n saldo täsmäytyksessä ennakkoina henkilöittäin. Kun ennakko on
avoinna vielä kuukauden kuluttua matkan päättymisestä, kerro se käyttäjälle ja
kirjaa se kodin `TODO.md`:hen. Jos yhtiöllä on oma tilitysaika, se on
`knowledge company` -tieto, ja käytä sitä.

Osakkaan ennakko tilitetään tai palautetaan aina ennen 31.12. Matkaan
käyttämätöntä ja palauttamatonta ennakkoa et kirjaa kuluksi, palkaksi tai
lainaksi itse. Kerro käyttäjälle, että rahat palautetaan heti, ja osakkaalle
myös, ettei ennakkoa voi jälkikäteen muuttaa osakaslainaksi: Verohallinnon
mukaan osakaslainasta sovitaan ennen nostoa, ja palauttamaton ennakko on lähempänä
peiteltyä osinkoa
([Peitelty osinko](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48147/peitelty-osinko11/),
luvut 6.9 ja 7.2.1).
