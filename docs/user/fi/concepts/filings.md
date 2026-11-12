# Viranomaisilmoitukset ja epävarma lopputulos

aggountant lähettää ALV-ilmoituksen Verohallinnolle ja matkakorvausilmoituksen
Tulorekisteriin. Lähetyksissä ohjelma toimii varovaisemmin kuin muualla: se ei
koskaan lähetä samaa ilmoitusta uudelleen automaattisesti, vaikka yhteys
katkeaisi kesken. Tämä sivu kertoo miksi ja mitä se tarkoittaa sinulle.

## Miksi ilmoitus on erilainen kuin tosite

Väärin sinetöity tosite korjataan uudella tositteella omassa kirjanpidossasi.
Lähetettyä ilmoitusta et voi korjata itse: se on oikeusvaikutteinen ja jää
viranomaisen järjestelmään. Toinen lähetys samalta kaudelta olisi
Verohallinnossa korvaava ilmoitus, ei peruutus. Siksi jokainen lähetys vaatii
hyväksyntäsi, ja hyväksyntä on sidottu juuri siihen sisältöön, joka lähtee.
ALV-ilmoituksessa ohjelma laskee luvut vielä kerran kirjanpidosta ennen
lähetystä.

Ilmoitukset lähtevät aina yrityksesi omilla tunnuksilla. Toisen yrityksen
puolesta ilmoittamista ei tueta.

## Kuittaus ei ole käsittely

Kun Verohallinto vastaanottaa ALV-ilmoituksen, se antaa kuittauksen
(tunniste ja aikaleima), ja aggountant merkitsee kauden ilmoitetuksi.
Kuittaus tarkoittaa, että ilmoitus on vastaanotettu, ei sitä, että se olisi
jo käsitelty. Käytännössä OmaVero on näyttänyt rajapinnan kautta tulleen
ilmoituksen vasta noin puolen vuorokauden kuluttua, ja Verohallinnon tuen
mukaan noin 12 tunnin viive on normaali. Tämä on käytännön havainto, ei
luvattu palveluaika.

## Epävarma lopputulos

Joskus ohjelma ei tiedä, menikö ilmoitus perille: yhteys katkesi kesken
lähetyksen, palvelu vastasi virheellä tai vastaus oli ristiriitainen. Ilmoitus
on voinut mennä perille, vaikka vastausta ei tullut.

Silloin ohjelma ei lähetä uudelleen. Jos se lähettäisi, ja ensimmäinen
lähetys olikin mennyt perille, viranomaisella olisi kaksi ilmoitusta.
Sen sijaan ajo jää huomiota vaativaan tilaan, ja agentti kysyy viranomaiselta,
mitä sillä on:

- Jos viranomaisella on täsmälleen sama ilmoitus, ajo päättyy ilman uutta
  lähetystä.
- Jos ilmoitusta ei näy tai se poikkeaa, ajo jää odottamaan, koska viranomainen
  ei välttämättä vielä näytä uusinta tietoa. Kysytään myöhemmin uudelleen.

Ilmoitusta ei koskaan lähetetä uudelleen sokkona. Toimintaohjeet ovat sivulla
[Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).

Sama varovaisuus koskee käsin tehtyä ilmoitusta. Jos samalle kaudelle on
jäänyt epävarma rajapintalähetys, käsin tehdyn ilmoituksen kirjaaminen vaatii
lisäksi näytön siitä, että viranomaisen tila on selvitetty. Muuten käsin tehty
ilmoitus voisi huomaamatta olla kaksoisilmoitus.

## Tulorekisterin erityispiirteet

Tulorekisteri-ilmoituksessa on henkilötunnus. Siksi:

- Ilmoituksen tarkiste on johdettu myös henkilötunnuksesta, ja sitä
  käsitellään arkaluonteisena: sitä ei tallenneta gitiin, muistiinpanoihin
  eikä keskustelulokiin.
- Ohjelma ei näytä Tulorekisterin virhetekstejä, koska niissä voi olla
  henkilötunnus. Tarkat virheet näet Tulorekisterin asiointipalvelusta.
- Maksun tunnistetta tai maksupäivää ei vaihdeta hylkäyksen tai epävarmuuden
  kiertämiseksi. Uusi tunniste olisi Tulorekisterille uusi ilmoitus.

## Mitä aggountant ei tee

- Se ei maksa veroa. ALV maksetaan OmaVeron viitteellä itse.
- Se ei tee korvaavia ilmoituksia eikä mitätöintejä. Ne tehdään OmaVerossa tai
  Tulorekisterin asiointipalvelussa.

## Mitä tämä tarkoittaa sinulle

- Hyväksy ilmoitus vasta, kun olet katsonut sen kentät.
- Älä lähetä ilmoitusta uudelleen sen takia, että se ei vielä näy OmaVerossa.
- Kun lopputulos on epävarma, odota ja anna agentin kysyä tilaa. Älä tee
  ilmoitusta käsin ennen kuin tiedät, menikö ensimmäinen perille.

## Katso myös

- [Lähetä ALV-ilmoitus suoraan Verohallinnolle](../tasks/file-vat-direct.md)
- [Ilmoita matkakorvaukset Tulorekisteriin](../tasks/report-travel-allowances.md)
- [Sinetöinti ja hyväksyntä](sealing-and-approval.md)
