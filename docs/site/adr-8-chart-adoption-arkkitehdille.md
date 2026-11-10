# ADR-8 arkkitehdille: SIE4 `#KONTO` karttasnapshotiksi, ei `init`-haaraksi

Tämä on yhteenveto ADR-8:n päätöksestä ohjelmistoarkkitehdille. Varsinainen päätös on [ADR-8](adr-8-chart-adoption); tämä sivu ei päätä mitään uutta.

## Päätös

Kahden käyttäjäpolun tuotelinja oli jo lukittu: aggountantin standardikartta on oletus, olemassa oleva yritys voi käyttää omaa karttaansa. Toteutustavaksi päätettiin **Chart-A**: deterministinen SIE4 `#KONTO` -tuonti yrityskohtaiseen, git-natiiviin `masterdata/reference`-karttaan. `aggountant init` jää idempotentiksi `fi-general-2026`-bootstrapiksi eikä sitä uudelleengeneroida yhden yrityksen kartaksi.

## Muoto ja invariantit

- Tuonti on eksplisiittinen onboarding-askel, ei `init`in sivuvaikutus.
- Parseri normalisoi ja validoi `#KONTO`-lähteen fail-closed. Tulos on ihmisen diffattava snapshot; `chart_version` on kanonisten JCS-tavujen full SHA-256, kuten nykyisessä masterdatassa.
- Aktiivinen kartta sidotaan tilikauteen/voimassaolorajaan. Semanttinen muutos luo uuden snapshotin, eikä historiallista snapshotia ylikirjoiteta. Jokainen sealed voucher pinnaa edelleen `chart_version`in provenanceen (O-19/O-26).
- Karttasnapshotiin rikastetaan karttakohtaiset ALV-roolit ja tarvittaessa tilin odotettu ALV-käsittely. Aktivoimaton tai roolittomaksi jäävä kartta ei kelpaa ALV-projektioon. Tämä on `vat-account-role-chart-metadata`-työn auktoritatiivinen syöte: `1798`/`2992` eivät ole globaalit vakiot vaan tuodun karttasnapshotin rooleja.

## Rajat

SIE4:n `#VER`/`#TRANS` ovat samalla hyödyllistä vertailudataa, mutta kartan importointi ei tee niistä aggountantin fiscal-auktoriteettia. `onboarding-authority-adr` ratkaisee erikseen Onb-A/Onb-B:n; Onb-A:ssa historia menee referenssi-/analytics-pintaan ja sealed fiscal alkaa live-kaudesta alkusaldojen kautta.

`sv-finding-default` avautuu: SIE4 tuo olemassa olevan yrityksen kartan uskollisesti, myös tase-tilit. Standardikarttaa ei muuteta yrityksen kartaksi.

## Miksi ei `init`-regen

Se sekoittaisi bootstrapin migraatioon, muuttaisi yhden yrityksen kartan valheelliseksi yleisstandardiksi, tekisi re-runista ylikirjoitusriskin eikä mallintaisi vuosittain muuttuvaa karttaa. Deterministinen importer + snapshotit on pienempi, testattavampi ja ajallisesti oikea pinta.

**Lopputulos:** F25 rakentaa yhden parseri/validation/activation-polun yrityskartalle; se avaa karttakohtaisen ALV-roolimetadatan ja olemassa olevan yrityksen uskollisen kartan, mutta jättää historian auktoriteetin erilliseen ADR:ään.
