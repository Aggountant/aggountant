# Tee ALV-ilmoitus OmaVerossa

Kun kuukausi on suljettu, agentti laskee sen ALV-ilmoituksen luvut
kirjanpidosta. Syötät luvut itse OmaVeroon, ja agentti kirjaa ilmoituksen
aggountantiin tehdyksi. Toimi näin, jos suora ilmoittaminen ei ole käytössä
tai haluat asioida itse.

## Ennen kuin aloitat

- Kuukausi on suljettu, eli sen tila on `sealed`
  ([Sulje kuukausi](close-month.md)).
- Pääset kirjautumaan OmaVeroon yrityksen puolesta.

## Näin teet sen

1. **Pyydä agenttia näyttämään luvut.**

   > Näytä maaliskuun ALV-ilmoituksen luvut.

2. **Agentti esikatselee ilmoituksen** ja näyttää sen numeroidut kentät
   euroina, myynnin ja ostojen veron, nettosumman ja maksettavan määrän.
   Esimerkiksi kuukausi, jolla on vain yksi 124,00 euron ostokuitti, antaa:

   | Kenttä | Summa |
   |---|---|
   | 307 Kohdekauden vähennettävä vero | 25,20 € |
   | 308 Maksettava tai palautukseen oikeuttava vero | −25,20 € |
   | Maksettavaa | 0,00 € |

   Esikatselu myös täsmäyttää ALV-tilit ilmoitukseen: se vertaa kuukauden
   liikettä ALV-tileillä (esimerkiksi 2939 ja 1763) ilmoituksen myynnin ja
   ostojen veroon. Kun ne täsmäävät, tuloksen `vat_reconciliation.status` on
   `reconciled`. Muutaman sentin ero ALV-merkityissä tositteissa on normaalia,
   koska ohjelma laskee veron rivikohtaisesti.

   Tila `unreconciled` tarkoittaa, että ALV-tilillä on vientejä ilman
   ALV-merkintää ja ilmoituksesta puuttuu veroa. Esikatselu kertoo silloin
   koodin `VAT_ACCOUNTS_UNRECONCILED`, tositteiden numerot ja puuttuvan
   summan. Selvitä asia ennen kuin ilmoitat luvut
   ([Jos jokin menee pieleen](#jos-jokin-menee-pieleen)).

3. **Syötä kentät OmaVeroon** ja lähetä ilmoitus. Voit myös laatia
   Ilmoitin.fi-tiedoston itse, sillä aggountant ei laadi sitä.

4. **Maksa vero itse**, jos maksettavaa on, OmaVeron viitteellä pankissa tai
   OmaVerossa. aggountant ei maksa mitään.

5. **Tallenna OmaVerosta ilmoituksen vahvistus**, esimerkiksi PDF:nä, ja anna
   se agentille.

   > Tässä on maaliskuun ALV-ilmoituksen vahvistus OmaVerosta. Kirjaa
   > ilmoitus tehdyksi.

6. **Hyväksy merkintä.** Agentti arkistoi vahvistuksen ja merkitsee kuukauden
   ilmoitetuksi. Merkintä on pysyvä, joten hyväksy se vasta, kun ilmoitus on
   oikeasti lähetetty.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-vat preview --period 2026-03
aggountant artifact attach --file omavero-vahvistus-2026-03.pdf
aggountant file-vat complete-manual --period 2026-03 --channel omavero \
  --evidence-ref <vahvistuksen tunniste>
aggountant period show --period 2026-03
```

`file-vat preview` lukee vain kirjanpitoa: se ei käynnistä mitään, ei lue
tunnuksia eikä ota verkkoyhteyttä. Siksi sen voi ajaa myös jo ilmoitetulle
kuukaudelle ja tarkistaa sillä aiemmat ilmoitukset. Se ei tarkista, onko suora
ilmoittaminen käytössä, joten sen ohjeessa ovat molemmat reitit: suora
ilmoitus, kun `aggountant vero-fi status` kertoo valmiuden olevan kunnossa, ja
muuten tämän sivun käsin tehtävä reitti. Neljännesvuosikaudella kausi annetaan
muodossa `2026-Q1`.

Täsmäytys jättää pois ALV:n maksut, palautukset ja ALV-tilien väliset siirrot,
koska ne eivät kuulu ilmoitukseen. Ne jäävät pois, vaikka samalla tositteella
olisi myös korkoa tai myöhästymismaksu. Maksun tai palautuksen ohjelma
tunnistaa siitä, että sen rahapuoli kulkee pankkitilin tai verotilin kautta.
Tilikartassa nämä tilit on merkitty maksutileiksi: vakiotilikartassa ne ovat
1910 ja 1920 mutta ei käteistili 1900. Jos samalla tositteella on jokin muu
tase-erä, kuten saatu ennakko, käyttöomaisuus, ostovelka tai oma pääoma,
ohjelma ei pidä vientiä maksuna vaan tarkistaa sen.

`file-vat complete-manual` ei lähetä eikä maksa mitään. Kanava on `omavero`
tai `ilmoitin`. Jos samalle kuukaudelle on jäänyt epävarma suora lähetys,
komento vaatii lisäksi valitsimen `--remote-status-evidence-ref` ja sille
näytön Verohallinnon tilasta.
</details>

## Valmis, kun

Kuukauden tila näyttää `vat_filed true`:

```
period "2026-03" — "sealed" (…, vat_filed true)
```

## Rajoitukset

Ohjelma ei erota viivästyskorosta hyvitystä, joka on kirjattu ilman
ALV-merkintää samalle tositteelle ALV:n maksun kanssa. Hyvitys menee maksun
osana läpi, jos se on pienempi kuin maksettu vero ja kirjattu kulutilille
(myyntihyvitys) tai tulotilille (ostohyvitys), tai jos sen vastapuoli on
velkatili maksun puolella, kuten saatu ennakko. Kirjaa siksi hyvitykset omille
tositteilleen ja ALV-merkinnällä.

Muut ALV-ilmoituksen rajoitukset ovat sivulla
[Mitä on valmista ja mitä ei](../reference/status.md#alv-ilmoitus).

## Jos jokin menee pieleen

- `PERIOD_NOT_SEALED`: kuukautta ei ole suljettu tai sulku on kesken.
  [Sulje kuukausi](close-month.md) ensin.
- `VAT_PERIOD_NOT_SEALED`: esikatselit kuukautta, jota ei ole vielä suljettu.
  Luvut ovat tarkistusta varten tehty luonnos (`fileable: false`), eikä niitä
  voi ilmoittaa ennen sulkua.
- `VAT_ACCOUNTS_UNRECONCILED`: luetelluissa tositteissa on veroa ALV-tilillä
  ilman ALV-merkintää. Kirjanpidossa vero on oikein ALV-tilillä, mutta
  esikatselun luvuista se puuttuu. Pyydä agenttia näyttämään tositteet ja käy
  ne läpi lähdettä vasten.
  - Jos vero kuuluu tämän kuukauden ilmoitukseen, ilmoita kuukausi OmaVerossa
    korjatuin luvuin: lisää puuttuva vero ja sen peruste lähteen verokannan
    mukaiseen kenttään. Ilmoitukseen kuuluu esimerkiksi myyntilasku, josta
    ALV-merkintä on unohtunut, ja samoin ilman ALV-merkintää kirjattu ennakon
    palautus, käyttöomaisuushankinnan palautus tai yksityiskäytön ALV-oikaisu.
    Suljettua kuukautta ei voi avata, eikä suora ilmoitus lähetä korjattuja
    lukuja. Älä kirjaa samaa veroa enää ALV-merkinnällä seuraavalle
    kuukaudelle, koska se tulisi silloin ilmoitetuksi kahdesti. Jos kuukausi
    on jo ilmoitettu liian pienillä luvuilla, anna korvaava ilmoitus, josta
    kerrotaan tämän luettelon lopussa.
  - Jos vienti ei kuulu ilmoitukseen, luvut ovat oikein ja voit ilmoittaa ne.
    Näin on esimerkiksi, kun Verohallinnon palautus on kirjattu tilille 2939
    tai vienti on avaavan tositteen ALV-saldo. Suorassa ilmoituksessa
    [agentti kirjaa syyn ilmoitusajoon](file-vat-direct.md#jos-jokin-menee-pieleen).
- `VAT_SETTLEMENT_ROLES_MISSING`: näkyy esikatselussa edellisen koodin
  rinnalla. Luetellut tositteet on kirjattu tilikartalla, jonka maksutilejä ei
  ole merkitty, joten ohjelma ei erota ALV:n maksua tai palautusta
  korjauksesta. Kirjanpito voi silti olla kunnossa. Jos käytössä on
  aggountantin vakiotilikartta, pyydä agenttia
  [päivittämään kodin perustiedot](upgrade.md) ja ajamaan esikatselu
  uudelleen. Yrityksen omalla tilikartalla maksutilit merkitään tiedostoon
  `masterdata/reference/settlement-account-roles.json`. Jos tositteet ovat
  ALV:n maksuja tai palautuksia, agentti voi myös kuitata ne ilmoitusajossa
  kuten muut ilmoitukseen kuulumattomat viennit.
- Komento vaatii näytön Verohallinnon tilasta, koska samalle kuukaudelle on
  jäänyt suora lähetys, jonka lopputulos on epävarma. Selvitä ensin, menikö se
  perille, ettei ilmoitus mene kahdesti:
  [Ilmoituksen lopputulos on epävarma](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).
- Jo annetussa ilmoituksessa on virhe. Korjaa se korvaavalla ilmoituksella
  ([Korjaa jo annettu ilmoitus](file-vat-direct.md#korjaa-jo-annettu-ilmoitus)).
  Jos teet korvaavan ilmoituksen OmaVerossa, agentti kirjaa sen kuitteineen
  (`file-vat complete-manual --replacement`).

## Katso myös

- [Lähetä ALV-ilmoitus suoraan Verohallinnolle](file-vat-direct.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
