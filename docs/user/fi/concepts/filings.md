# Viranomaisilmoitukset ja epävarma lopputulos

aggountant lähettää ALV-ilmoituksen Verohallinnolle ja matkakorvausilmoituksen
Tulorekisteriin. Jos yhteys katkeaa kesken lähetyksen, ohjelma ei lähetä samaa
ilmoitusta itsestään uudelleen, vaan agentti selvittää ensin, menikö
ensimmäinen perille.

## Miksi ilmoitus on erilainen kuin tosite

Väärin sinetöidyn tositteen korjaat uudella tositteella omassa
kirjanpidossasi. Lähetettyä ilmoitusta et voi korjata itse, koska se on
oikeusvaikutteinen ja jää viranomaisen järjestelmään. Toinen lähetys samalta
kaudelta ei peruisi ensimmäistä, vaan olisi Verohallinnossa korvaava ilmoitus.
Siksi jokainen lähetys vaatii hyväksyntäsi, ja hyväksyntä koskee täsmälleen
sitä sisältöä, joka lähtee. ALV-ilmoituksen luvut ohjelma laskee vielä kerran
kirjanpidosta ennen lähetystä.

Ilmoitukset lähtevät yrityksesi omilla tunnuksilla. Toisen yrityksen puolesta
ilmoittamista ei tueta.

## Kuittaus ei ole käsittely

Kun Verohallinto vastaanottaa ALV-ilmoituksen, se antaa kuittauksen eli
tunnisteen ja aikaleiman, ja aggountant merkitsee kauden ilmoitetuksi.
Verohallinto on silloin vastaanottanut ilmoituksen mutta ei välttämättä vielä
käsitellyt sitä. OmaVerossa rajapinnan kautta tullut ilmoitus on näkynyt vasta
noin puolen vuorokauden kuluttua, ja Verohallinnon tuen mukaan noin 12 tunnin
viive on normaali. Aika perustuu käytännön havaintoon, eikä sitä ole luvattu.

## Epävarma lopputulos

Joskus ohjelma ei tiedä, menikö ilmoitus perille: yhteys katkesi kesken
lähetyksen, palvelu vastasi virheellä, palvelu ilmoitti onnistumisesta mutta ei
lähettänyt luettavaa kuittia, tai vastaus oli ristiriitainen. Ilmoitus on
voinut mennä perille, vaikka selvää vastausta ei tullut, joten ohjelma ei
tulkitse onnistumisvastausta ilman kuittia hylkäykseksi.

Silloin ohjelma ei lähetä uudelleen, koska viranomaisella olisi kaksi
ilmoitusta, jos ensimmäinen lähetys olikin mennyt perille. Ajo jää huomiota
vaativaan tilaan, ja agentti kysyy viranomaiselta, mitä sillä on:

- Jos viranomaisella on täsmälleen sama ilmoitus, ajo päättyy ilman uutta
  lähetystä.
- Jos ilmoitus poikkeaa tai viranomainen näyttää kaudelle jotain muuta, ajo
  jää odottamaan. Vertaa ilmoitusta OmaVerossa.
- Jos viranomainen vastaa, ettei sillä ole kaudelle ilmoitusta, se ei vielä
  todista mitään, koska ilmoituksen näkyminen voi kestää päiviä. Vastaus
  kirjataan ajoon, ja agentti kysyy myöhemmin uudelleen.

ALV-ilmoituksen ajo suljetaan ilmoittamattomana, kun Verohallinto on vastannut
"ei löydy" kahdesti vähintään yhden pankkipäivän välein. Toinen kysely
lasketaan aikaisintaan samasta kellonajasta seuraavana pankkipäivänä.
Pankkipäiviä ovat maanantaista perjantaihin, paitsi suomalaiset pankkien
vapaapäivät: uudenvuodenpäivä, loppiainen, pitkäperjantai, toinen
pääsiäispäivä, vappu, helatorstai, juhannusaatto, itsenäisyyspäivä,
jouluaatto, joulupäivä ja tapaninpäivä. Epäonnistunutta kyselyä tai vastausta, jota
ohjelma ei pysty lukemaan, ei lasketa. Kun ajo on suljettu, uusi ilmoitusajo
saa lähettää kauden ilmoituksen. Siihen asti aggountant kieltäytyy
aloittamasta tai hyväksymästä samalle kaudelle uutta ilmoitusta
(`VAT_FILING_UNRESOLVED`).

Tulorekisteri-ilmoituksen ajo ei sulkeudu "ei löydy" -vastauksista. Se pysyy
auki, kunnes tilakysely löytää ilmoituksen tai kertoo Tulorekisterin
hylänneen sen, tai kunnes agentti kirjaa asiointipalvelussa tehdyn
ilmoituksen tunnisteet ajoon.

Mitä teet itse, on kohdassa
[Ilmoituksen lopputulos on epävarma](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).

Jos samalle kaudelle on jäänyt epävarma rajapintalähetys ja teet ilmoituksen
käsin, sen kirjaaminen vaatii lisäksi näytön siitä, että viranomaisen tila on
selvitetty. Muuten käsin tehty ilmoitus voisi huomaamatta olla
kaksoisilmoitus.

## Tulorekisterin erityispiirteet

Tulorekisteri-ilmoituksessa on henkilötunnus. Ilmoituksen tarkiste on johdettu
myös siitä, joten tarkistetta käsitellään arkaluonteisena: sitä ei tallenneta
gitiin, muistiinpanoihin eikä keskustelulokiin. Tulorekisterin virhetekstejä
ohjelma ei näytä, koska niissäkin voi olla henkilötunnus. Tarkat virheet näet
Tulorekisterin asiointipalvelusta.

Maksun tunnistetta tai maksupäivää ei vaihdeta hylkäyksen tai epävarmuuden
kiertämiseksi, koska uusi tunniste olisi Tulorekisterille uusi ilmoitus.

## Mitä aggountant ei tee

aggountant ei maksa veroa, vaan maksat ALV:n itse OmaVeron viitteellä. Se ei
myöskään mitätöi työnantajan erillisilmoitusta. Mitätöinti tehdään
Tulorekisterin asiointipalvelussa.

Korvaava ALV-ilmoitus, matkakorvausilmoituksen korvaava ilmoitus ja mitätöinti
sekä erillisilmoituksen korvaava ilmoitus tehdään aggountantilla, kukin omalla
esikatselullaan ja hyväksynnällään.

## Mitä tämä tarkoittaa sinulle

- Hyväksy ilmoitus vasta, kun olet katsonut sen kentät.
- Älä lähetä ilmoitusta uudelleen sen takia, että se ei vielä näy OmaVerossa.
- Kun lopputulos on epävarma, odota ja anna agentin kysyä tilaa. Älä tee
  ilmoitusta käsin ennen kuin tiedät, menikö ensimmäinen perille.

## Katso myös

- [Lähetä ALV-ilmoitus suoraan Verohallinnolle](../tasks/file-vat-direct.md)
- [Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md)
- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
