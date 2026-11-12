# Tee ALV-ilmoitus OmaVerossa

Kun kuukausi on suljettu, sen ALV-ilmoitus tehdään kirjanpidosta lasketuilla
luvuilla. Tällä sivulla syötät luvut itse OmaVeroon ja kirjaat ilmoituksen
aggountantiin tehdyksi. Näin toimit, jos suora ilmoittaminen ei ole käytössä
tai haluat asioida itse.

## Ennen kuin aloitat

- Kuukausi on suljettu, eli sen tila on `sealed`
  ([Sulje kuukausi](close-month.md)).
- Pääset kirjautumaan OmaVeroon yrityksen puolesta.

## Näin teet sen

1. **Pyydä agenttia näyttämään luvut.**

   > Näytä maaliskuun ALV-ilmoituksen luvut.

2. **Agentti esikatselee ilmoituksen.** Esikatselu lukee vain kirjanpitoa: se
   ei käynnistä mitään, ei lue tunnuksia eikä ota verkkoyhteyttä. Tuloksessa
   ovat ilmoituksen numeroidut kentät euroina, myynnin ja ostojen vero,
   nettosumma ja maksettava määrä. Esimerkiksi kuukausi, jolla on vain yksi
   124,00 euron ostokuitti, antaa:

   | Kenttä | Summa |
   |---|---|
   | 307 Kohdekauden vähennettävä vero | 25,20 € |
   | 308 Maksettava tai palautukseen oikeuttava vero | −25,20 € |
   | Maksettavaa | 0,00 € |

3. **Syötä kentät OmaVeroon** ja lähetä ilmoitus. Voit myös laatia
   Ilmoitin.fi-tiedoston itse. aggountant ei tee sitä.

4. **Maksa vero**, jos maksettavaa on, OmaVeron viitteellä pankissa tai
   OmaVerossa. aggountant ei maksa mitään.

5. **Tallenna vahvistus.** Ota OmaVerosta ilmoituksen vahvistus talteen, esimerkiksi
   PDF:nä, ja anna se agentille.

   > Tässä on maaliskuun ALV-ilmoituksen vahvistus OmaVerosta. Kirjaa
   > ilmoitus tehdyksi.

6. **Tarkista ja hyväksy.** Agentti arkistoi vahvistuksen ja merkitsee
   kuukauden ilmoitetuksi. Merkintä on pysyvä, joten hyväksy se vasta, kun
   ilmoitus on oikeasti lähetetty.

<details>
<summary>Komennot, joita agentti ajaa</summary>

```sh
aggountant file-vat preview --period 2026-03
aggountant artifact attach --file omavero-vahvistus-2026-03.pdf
aggountant file-vat complete-manual --period 2026-03 --channel omavero \
  --evidence-ref <vahvistuksen tunniste>
aggountant period show --period 2026-03
```

Kanava on `omavero` tai `ilmoitin`. Komento ei lähetä eikä maksa mitään. Jos
samalle kuukaudelle on jäänyt epävarma suora lähetys, komento vaatii lisäksi
valitsimen `--remote-status-evidence-ref` ja sille näytön Verohallinnon
tilasta.
Neljännesvuosikaudella kausi annetaan muodossa `2026-Q1`.
</details>

## Valmis, kun

Kuukauden tila näyttää `vat_filed true`:

```
period "2026-03" — "sealed" (…, vat_filed true)
```

## Rajoitukset

- Esikatselun ohjeteksti kehottaa aina syöttämään luvut käsin, myös silloin
  kun suora ilmoittaminen on käytössä. Teksti on vanhentunut.
- Korjattua tai korvaavaa ilmoitusta aggountant ei tee. Jos ilmoitettuihin
  lukuihin tulee korjaus, tee korvaava ilmoitus OmaVerossa.

Tarkemmin: [Mitä on valmista ja mitä ei](../reference/status.md#alv-ilmoitus).

## Jos jokin menee pieleen

- **`PERIOD_NOT_SEALED`.** Kuukautta ei ole suljettu tai sulku on kesken:
  [Sulje kuukausi](close-month.md).
- **Komento vaatii näytön Verohallinnon tilasta.** Samalle kuukaudelle on jäänyt
  suora lähetys, jonka lopputulos on epävarma. Selvitä ensin, menikö se perille,
  ettei ilmoitus mene kahdesti:
  [Kun jokin menee vikaan](../troubleshooting.md#ilmoituksen-lopputulos-on-epävarma).

## Katso myös

- [Lähetä ALV-ilmoitus suoraan Verohallinnolle](file-vat-direct.md)
- [Viranomaisilmoitukset ja epävarma lopputulos](../concepts/filings.md)
- [Kaudet, sulku ja tilikausi](../concepts/periods.md)
