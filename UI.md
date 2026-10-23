# AS2026:n käyttöliittymän kehittäminen

Nykyisen 0.6.10-käyttöliittymän komponentit ja todelliset tarkistuskuvat ovat
[Slint-oppaassa](docs/KAYTTOLIITTYMAOPAS.md). Sitovat mitoitus-, väri- ja
vuorovaikutussäännöt: [UI_PERIAATTEET.md](docs/UI_PERIAATTEET.md).

## Arkkitehtuuri

`ui/app.slint` kokoaa sivupalkin, seitsemän pääsivua, Toiminnot-valikot ja
valintaikkunat. `ui/components.slint` sisältää yhteiset taulukot, editorit ja
paneelit. `ui/theme.slint` omistaa Fluent 2 / Dark Spruce -mitat ja värit.
Paikallinen Fluent-sovitus ja sen lisenssit ovat `ui/fluent`-hakemistossa.

`src/gui` sitoo Slintin tapahtumat yhteiseen Rustin Workspace-, tallennus- ja
laskentakoodiin. Slint ei toteuta rinnakkaista pistelaskentaa tai LAN-protokollaa.
Säilytä Mikky100:n kilpailusäännöt ja tietomallit; tarkista ajantasainen main
ennen muutosta. Erityisesti näyttöasetusten muutos ei saa poistaa hyväksyntöjä.

## Toiminta ja tallennus

Kelvolliset kentät tallentuvat Enterillä tai kohdistuksen poistuessa.
Uusi tietue tallentuu pakollisten kenttien valmistuttua. Virheellinen luonnos
säilyy korjattavana; tallennusvirhettä ei esitetä onnistumisena.

Kilpailijan muokkaus on Tulokset-sivulla keskitetty modaali. Muokkauskutsu,
Valmis ja Esc sulkevat sen validoinnin salliessa. Uudet Emit-luvut tallentuvat,
mutta eivät vaihda aktiivisen editorin kilpailijaa kesken muokkauksen.
Koko kortin leimat korjataan Emit-sivun **Muokkaa leimauksia** -toiminnolla.

Toiminnot-valikon ehdollinen painike voi tuhoutua popupin sulkeutuessa:
suorita toiminto ennen sulkemista. Kun avataan toinen popup, käytä pääikkunan
pysyvää käsittelijää. Tarkista aina todellinen päävalikko → alavalikko →
toiminto -ketju; suora Rust-callback ei yksin testaa valikkoa.

## Ulkoasu ja saavutettavuus

Suomenkieliset tekstit, yhteiset Fluent-säätimet ja näkyvä näppäimistökohdistus.
Ei snapshot-bluria: modaalin alla neutraali `#00000080`-peite (Slint: RRGGBBAA).
Säilytä otsikot ja alapainikkeet vierityksen ulkopuolella. Vähimmäiskoko on
900 × 600 loogista pikseliä; kapea työalue käyttää välilehtiä tai vieritystä.
Leveän näytön asetukset jakautuvat palstoihin.

## Tarkistus

[Käännös- ja GUI-testikomennot](BUILDING.md). CI ajaa natiivin Linux- ja
Windows-GUI:n 100/150 %:n skaalauksilla. Käytä `--demo`-kilpailua, erillisiä
asetuksia ja virtuaalinäyttöä; älä muuta oikeita kilpailutietoja tai käytä
fyysistä lukijaa automaattikokeessa. Kuvakaappaus yksin ei todista toimintoa:
varmista myös tallennus, uudelleenavaus ja tiedostojen sisältö.

Lisenssit ja alkuperäinen AboutSlint-maininta säilytetään. Vanhojen beta-ohjeiden
asettelut eivät syrjäytä nykyistä käyttöliittymäopasta.
