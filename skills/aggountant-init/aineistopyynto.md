# Esimerkki: aineistopyyntö

Kuvitteellinen tilanne, jolla [SKILL.md](SKILL.md):n yksi aineistopyyntö näytetään
käytännössä. Esimerkki Oy (Y-tunnus 1234567-1) siirtyy toisesta kirjanpito-ohjelmasta.
Tilikausi on kalenterivuosi, vanha ohjelma kattaa helmikuun 2026 loppuun ja
aggountant alkaa 1.3.2026. Yhtiöllä on yksi pankkitili, ei palkansaajia, ja
saldoluettelossa on verotilin velkaa. Valmiuslistalta tunnistetiedot, tilikausi ja
katkaisu ovat valmiita; muut kohdat odottavat aineistoa.

Pyyntö kirjoitetaan käyttäjälle hänen kielellään, ja se on yksi viesti:

> Jotta Esimerkki Oy:n kirjanpito voi alkaa 1.3.2026, tarvitsen kerralla seuraavat.
> Kaikki muu on jo kunnossa.
>
> 1. **Vanhasta ohjelmasta tase ja tuloslaskelma 28.2.2026, tililuettelo sekä
>    pääkirja 1.1.–28.2.2026**, PDF:nä tai CSV:nä. Niistä tehdään avaava tosite ja
>    tilimuunnos, ja pääkirjasta näkee, mistä kunkin tilin saldo koostuu. Hae ne ennen
>    kuin vanhan ohjelman käyttö päättyy.
> 2. **Pankkitilin tiliote 1.3.2026 alkaen** sähköisenä tiliotetiedostona (camt),
>    jos pankki antaa sellaisen, sekä tilin saldo 28.2.2026. Tiliote on kuukauden
>    kirjausten lähde, ja helmikuun lopun saldon pitää täsmätä taseeseen.
> 3. **OmaVerosta yhtiön verotilin kaikki tapahtumat 1.1.2026 alkaen.** Taseessa on
>    verotilin velkaa, ja sen saldo selviää vain katkeamattomasta otteesta, koska
>    korot ja maksut on voitu kirjata vanhassa ohjelmassa eri tavalla.
> 4. **Lista avoimista myyntilaskuista ja maksamattomista ostolaskuista 28.2.2026**,
>    jos tase ei erittele niitä laskuittain. Muuten maksua ei voi myöhemmin kohdistaa.
> 5. **Vastaukset kolmeen kysymykseen:** Onko yhtiön kuluja maksettu omalta tililtäsi
>    tai kortiltasi, ja jos on, kuitit niistä? Käytetäänkö yhtiön työhön omaa autoa
>    tai kotia? Mihin kuitit ja laskut nykyään tulevat (sähköposti, kansio)?
>
> Palkka-aineistoa en tarvitse, koska yhtiöllä ei ole palkansaajia. Suoraa
> ALV-ilmoittamista ei tarvitse ottaa käyttöön nyt; se voidaan tehdä myöhemmin.

Pyyntö kirjataan heti Esimerkki Oy:n kodin `TODO.md`-tiedostoon muilta odotettavaksi,
kohta kohdalta. Kun käyttäjä toimittaa osan, toimitettu poistetaan ja jatkoistunnossa
pyydetään vain loput.
