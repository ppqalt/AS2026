# Version 0.6.10 muutosten ominaisuuskohtainen vertailu

Pohja: `ac686bf079342bd6980ae497b4899f0b75cc3f0a`, Mikky100:n
uusin main ja julkaistu v0.6.9 tarkistettaessa 23.10.2026. Haara:
`fix/v069-ui-taydennykset`. PR #11 on yhdistetty mainiin. Täydennykset toimitetaan version 0.6.10 mukana. Aikaisemmat tagit säilyvät.
Vanhaa kehityshaaraa ei yhdistetä mainiin.

## Ominaisuuskohtainen vertailu

| Pyyntö / vanha patch | Uusimmassa jo oleva ratkaisu | Tämän haaran päätös |
|---|---|---|
| Kilpailun Emit-aikakorjaus | Overrides-oletus 120 s | Säilytetään Mikky100:n malli ja sekuntikenttä |
| Kilpailijakohtainen poikkeus | `emit_korjaus_annettu`, myös nolla erotetaan oletuksesta | Ei vanhaa rinnakkaista kenttää tai h:min:s-muutosta |
| Emit-rivin korjaus | Koko aktiivisen kortin editori, lisää/poista/palauta ja kellonaika | Säilytetään; vanhaa tulossivun korjausdialogia ei tuoda |
| Laskenta ja hyväksynnät | AT/JAT, kokonaismyöhästyminen, `same_scoring`, raakalukemien jäljitettävyys | Ei muutoksia laskentaan tai tallennusmalliin |
| Lähtöluettelo | Valittavat sarakkeet ja järjestys, näppäimistö | Säilytetään painike ja kaikki toiminnot |
| Toimintojen sijoittelu | Mikky100:n yhteinen Toiminnot-valikko ja tulossivun yksi rivi | Säilytetään, korjataan popupin elinkaari kaikissa valinnoissa (#10) |
| Luokkien/ratojen lisäys laskennan jälkeen | Sallittu, poistot ja ratamuutoksen uudelleenlaskentavahvistus | Säilytetään; lisäyspainikkeiden korjaus varmistetaan tallennus/uudelleenavauskokeella |
| Kilpailijan muokkaus | Sama kenttämalli ja automaattinen tallennus inline-editorissa | Keskitetty modaali, sulkeminen samalla kutsulla / Valmis / Esc |
| Beginners Cup | Mikky100:n laskenta ja erillinen tulosnäkymä | Sama painike palauttaa perusnäkymään |
| LAN-tila | Sivupalkki, jonon ja katkon viestit | Säilytetään sijainti ja viestit; yhteysikkunan code/osoite erotetaan |
| Parituskoodi | 12-merkkinen salaisuus ja Noise-kättely | Ei lyhennetä neljään merkkiin eikä korvata verkkototeutusta |
| Enter liittymisessä | Liittymiskutsu ja validointi jo olemassa | Enter osoite- ja koodikentistä samaan kutsuun; todellinen kenttäkohdistus testataan |
| Snapshot-blur | Ei nykyisessä Mikky100:n lähteessä | Ei vanhaa snapshot-koodia; yhteinen peite neutraali `#00000080` (RRGGBBAA) |
| Asetukset | Kaikki Mikky100:n kilpailu- ja tulostusasetukset | Tasalevyiset responsiiviset palstat, lukuleveys 1680; pieni ikkuna vierii |
| Valikot | Paikallinen Fluent 2 -sovitus | ComboBox kentän alle, enintään kuusi näkyvää riviä |
| Radat/Emit | Eri korkeudella alkavat rinnakkaispaneelit | Radan tiedot taulukon tasolle; Emit-taulukoille yhteinen otsikkorivi |
| Virhepalaute / aloitus | Nykyiset virhepolut ja aloitustoiminnot | Tilapalkkiin sisämarginaali ja Näytä virhe; rajattu aloituskortti |
| Ohjeet | 0.6.8/0.6.9:n käyttöohjeet | Uusi kuvallinen Slint-komponenttiopas ja Windows/UFW-palomuuri |

Vanhan `julkaisu-0.6.6`-työpuun keskeneräinen työ on säilytetty. Sen vanhaa
Emit-tietomallia, nelimerkkistä parituskoodia, snapshot-bluria, vanhaa
tulostus-/laskentakoodia, ylimääräisiä työkalurivejä ja julkaisuskriptiä ei tuoda.
Myöskään nykyisiä testejä ei poisteta tai heikennetä.

## Löydetty varsinainen vika

Mikky100:n [ilmoitus #10](https://github.com/ppqalt/AS2026/issues/10)
on korkein korjausprioriteetti. Popupin sulkeminen tuhosi ehdollisen
painikkeen vanhempiviittauksen ennen seuraavaa toimintoa. Tavalliset valinnat
suorittavat toiminnon ennen sulkemista. Vientialavalikko käyttää pysyvää
pääikkunan käsittelijää: vanha popup suljetaan ja uusi avataan vasta sen jälkeen.
Korjaus kattaa kaikki sivut, ei vain kahta Lisää luokka -painiketta.

## Paikallinen varmennus

- Rust + Slint kääntyvät; Clippy ilman varoituksia ja muotoilutarkistus.
- 419 testiä / 37 testisarjaa, 0 epäonnistumista (`--all-targets --features gui`).
- Olemassa olevat GUI-työnkulut Linuxin virtuaalinäytössä 100 % ja 150 %.
- Käännetyn GUI:n valikkokoe: 23 päävalikkovalintaa molemmilla syöttötavoilla,
  lisäksi CSV/HTML-alavalinnat (50 aktivointia), todellinen tiedostojen tallennus,
  luokkien tallennus/uudelleenavaus ja molempien tulosteiden muodostus.
- Excelin esikatselu tuotannon tuontikoodilla; tiedostovalitsimen vastaus
  syötetään synteettisesti. Sama aloituspolku myös ilman kilpailua.
- Seitsemän sivua ko'oissa 900×600, 1366×768, 1920×1080 ja 2560×1440.
- BC päälle/pois, modaali sulkeutuu, neutraalin peitteen pikselivertailu,
  keskeneräinen tietue, vain luku -käyttöliittymä ja tulostus käynnissä.
- Kaksi erillistä GUI-prosessia: Enter-liittyminen, kaksi Emit-lukua,
  käsikellot, virhepisteet, hyväksyntä, tuloste sekä yhteyskatkon pysyvä
  Emit-jono ja palautuminen. Fyysistä lukijaa/tulostinta ei käytetty.

Linux- ja Windows-CI suorittavat myös vanhat testit sekä uuden valikkokokeen.
Ajantasainen alustatulos näkyy tämän haaran pull requestin tarkistuksissa.
Käännös tai callback-tarkkailija yksin ei ole väite fyysisestä laitekokeesta.
Natiivi tiedostovalitsin ja paperitulostus on rajattu automaattikokeen ulkopuolelle.

[Kuvallinen komponenttiopas](KAYTTOLIITTYMAOPAS.md) ·
[Palomuuriohje](PALOMUURI.md) · [Mikky100:n 0.6.9-työnkulut](KAYTTOLIITTYMA_069.md)
