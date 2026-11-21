# ALV-ilmoituksen häiriöt

Periaate on [SKILL.md](SKILL.md)-tiedostossa: ilmoitusta ei lähetetä uudelleen
arvauksen varassa, tila kysytään ohjelmalta ja `next_command`ia seurataan. Tässä on
se, mitä kussakin tilanteessa on hyvä tietää ohjelman vastauksen lisäksi.

## Kuitattu ilmoitus ei näy OmaVerossa

Ensimmäisten tuotantoilmoitusten jälkeen (13.9.2026) OmaVero näytti kaudet seuraavana
aamuna yhä ilmoittamattomina, ja niin näytti myös rajapinnan kausilista. Verohallinnon
puhelintuki kertoi 14.9.2026, että rajapinnan vastaanottamat ilmoitukset noudetaan
jatkokäsittelyyn väliajoin ja noin 12 tunnin viive on tavallinen; ohje oli katsoa
seuraavana päivänä. Tämä on havainto, ei luvattu palveluaika.

Kuittaus todistaa ilmoituksen, vaikka se ei näy ja vaikka tilakysely epäonnistuisi.
Kerro käyttäjälle odotusaika viikonloput huomioiden. Jos ilmoitus ei näy vuorokauden
jälkeen tai eräpäivä painaa, kokoa käyttäjälle Verohallinnon ALV-neuvontaan soittoa
varten kausi, Y-tunnus, kuittauksen tunniste ja vastaanottoaika sekä lähetetyt kentät.
Uusi lähetys tehdään vain Verohallinnon nimenomaisesta ohjeesta.

Jos `file-vat status` kertoo Verohallinnon käsitelleen ilmoituksen eri luvuin kuin
hyväksyttiin, vastauksen `differences` nimeää kentät. Pyydä käyttäjää vertaamaan juuri
niitä OmaVerossa; ero ratkaistaan siellä.

## Lähetyksen lopputulos jäi epävarmaksi

Aikakatkaisu, palvelinvirhe tai vastaus ilman luettavaa kuittausta ei ole hylkäys:
ilmoitus voi olla perillä. `aggountant file-vat reconcile` kysyy Verohallinnolta ja
kirjaa vastauksen ajoon. Jos Verohallinnolla on hyväksytyn ehdotuksen mukainen
ilmoitus, ajo päätetään sen perusteella ilman uutta lähetystä. Yksi "ei löydy" ei
käsittelyviiveen takia riitä: ohjelma sulkee ajon ilmoittamattomana vasta toisesta
samasta vastauksesta, joka on saatu aikaisintaan seuraavana pankkipäivänä (vastauksen
`next_check_from`). Siihen asti uusi lähetys on estetty. Kerro käyttäjälle,
milloin kysytään uudelleen, ja jos eräpäivä ehtii väliin, sano se suoraan: hän voi
katsoa OmaVerosta tai soittaa Verohallinnolle.

Jos kausi päädytään ilmoittamaan käsin, kun suoran lähetyksen lopputulos on yhä auki,
`file-vat complete-manual` vaatii näytön myös siitä, miten Verohallinnon tila
selvitettiin (`--remote-status-evidence-ref`). Ohjelma ei arvioi näytön sisältöä, joten
arvio on sinun ja käyttäjän: näyttö on Verohallinnon nimenomainen vastaus siitä, ettei
aiempaa ilmoitusta ole. Tyhjä OmaVeron näkymä tai yksi "ei löydy" ei ole sellainen.

## Ajo keskeytyi kesken lähetyksen

Jos ohjelma tai kone pysähtyi lähetyksen ja sen kirjaamisen välissä, ajoa ei jatketa
millään komennolla. `file-vat status` katsoo, ehtikö pyyntö lähteä, kysyy
Verohallinnolta ja nimeää komennon, joka kirjaa lopputuloksen lähettämättä mitään.

Kahdessa tapauksessa ohjelma ei nimeä komentoa, koska ratkaisu ei ole sen:

- Verohallinto näyttää kaudelta jotain muuta kuin hyväksytyn ilmoituksen. Käyttäjä
  vertaa OmaVerossa, ja tila kysytään myöhemmin uudelleen.
- Ajo ei lähettänyt mitään, mutta Verohallinnolla on samat luvut käsiteltyinä.
  Ilmoitus on silloin annettu muuta kautta. Ohjelma ei ota sitä ajon omaksi: käyttäjä
  vahvistaa, että hän tai joku muu antoi sen, ja se kirjataan käsin annettuna
  (`file-vat complete-manual`, Verohallinnon vastaus näyttönä).
