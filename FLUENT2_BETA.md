# AS2026 — Fluent 2 / Dark Spruce

Fluent 2 on AS2026:n koko käyttöliittymän suunnittelukieli. Dark Spruce
on sen tumma väritys. Paikalliseen `main`-haaraan yhdistetty käyttöliittymä
pohjautuu v0.5.7:n toimintoihin ja aiempaan
`beta/fluent2-dark-spruce`-haaraan. Tämä yhdistäminen ei itsessään julkaise
uutta versiota tai muuta GitHubin haaroja.

Tiedoston nimi on säilytetty aiemmasta betasta, jotta olemassa olevat
viittaukset toimivat.

## Yhteiset säännöt

- Sama navigaatio, typografia, mittasuhteet, lomakkeet, taulukot,
  valintaikkunat ja toimintojen tilat kaikissa näkymissä.
- Lähes mustat, hieman vihertävät pinnat ja hillitty metsänvihreä korostus.
- Vihreä osoittaa ensisijaisesti valinnan, tärkeän toiminnon tai onnistumisen.
- Ohuet neutraalit rajat ja selkeä, tiivis asettelu.
- Sovelluksen komponenteissa ei animaatioita, siirtymiä,
  sumennusta, varjoja, liukuvärejä, hohtoa tai läpikuultavia peitepintoja.
- Kaikki käyttäjälle näkyvät tekstit ovat suomeksi. Vanhan AS2015A:n
  toimintojen yhteydessä käytetään sen alkuperäistä sanastoa.
- Toteutus säilyy Rustin ja Slintin varassa ilman verkkokäyttöliittymää.

## Komponentit ja värit

Slint 1.16:n Fluent-tyyli valitaan tiedostossa `build.rs`. Valmiiden
`Button`-, `LineEdit`-, `ComboBox`- ja `ScrollView`-komponenttien toteutukset
säilytetään paikallisessa, efektittömässä sovituksessa `ui/fluent`.
Ensisijaiselle vihreälle toimintopainikkeelle on oma pieni komponentti.

`ui/theme.slint` sisältää käyttöliittymän yhteiset värit ja välit:

| Tunniste | Väri | Käyttö |
|---|---|---|
| canvas | `#0D1210` | näkymän tausta |
| surface | `#151C19` | tavallinen paneeli |
| surface-raised | `#1C2722` | erotettu pinta |
| line | `#2C3931` | ohuet rajat |
| accent-fill | `#356F54` | tärkeä toiminto ja navigaation valintamerkki |
| accent-soft | `#233B2D` | valittu taulukkorivi tai navigaatiorivi |
| ink | `#E8EEE9` | tavallinen teksti |
| muted | `#A3B0A7` | täydentävä teksti |

Välit perustuvat neljän kuvapisteen asteikkoon. Painikkeiden peruskorkeus
on 36, taulukkorivien 38 ja navigaatiorivien 40 loogista kuvapistettä.
Kulmien pyöristys on 5–6 kuvapistettä. Navigaation vihreä valintamerkki
keskitetään rivin toteutuneen korkeuden perusteella, ja sen ylä- ja
alapuolelle jää sama tila myös näyttöskaalauksen muuttuessa.

Ensisijainen kirjasin on käyttäjän valitsema Google Sans. Sovellus pyytää
sitä käyttöjärjestelmältä, mutta ei sisällä eikä lataa kirjasintiedostoja.
Slint käyttää järjestelmän korvaavaa kirjasinta, jos Google Sans puuttuu.

## Säilytettävä toiminta

- Lomakkeiden, painikkeiden ja valintaikkunoiden näppäimistökäyttö säilyy.
- Taulukon rivit valitaan hiirellä. Taulukkoon ei lisätä omaa
  näppäimistökohdistusta tai suurta kohdistuskehystä. Kapeassa ikkunassa
  otsikot ja rivit vierivät vaakasuunnassa yhdessä; pystyvierityspalkki
  säilyy näkyvän taulukkoalueen reunassa.
- Radan tiedot pysyvät näkyvissä asemaa vaihdettaessa.
- Aseman tavoiteaika syötetään erillisiin tunti-, minuutti- ja
  sekuntikenttiin. Tallennusmuoto säilyy sekunteina.
- Editorin leveys määräytyy asettelun rajoitteista. Se ei riipu sellaisesta
  ikkunan leveydestä, jonka kokolaskenta puolestaan riippuu editorista.
- Laskenta, tiedostot, Emit-luku, kilpailutiedot ja tuonti/vienti eivät
  muutu pelkän ulkoasun yhdistämisen vuoksi.

## Tarkistaminen

Rust-testit tarkistavat muun muassa päätekstien ja valintojen luettavan
kontrastin sekä taulukoiden hiirivalinnan ja editorien asettelurajauksen.
GUI käännetään erikseen ja tarkistetaan todellisilla kuvakaappauksilla
myös kapeassa ikkunassa ja eri näyttöskaalauksilla. Linuxin tulos ei
itsessään todista Windowsin ulkoasua tai toimintaa.

```sh
cargo run --features gui --bin as2026-gui
```

Slintin alkuperäisten Fluent-vakiokomponenttien lyhyet animaatiot,
reunojen liukuvärit ja ponnahdusikkunoiden varjot on poistettu paikallisesta
sovituksesta. Testi tarkistaa myös nämä lähdetiedostot ja niiden paikalliset
riippuvuudet. Tausta-, teksti- ja valintavärit tulevat samasta Dark Spruce
-paletista. Käyttöjärjestelmän omat ikkunakehykset ja tiedostonvalitsimet
noudattavat käyttöjärjestelmän asetuksia.

Paikallisen Slint-kopion lisenssit, tehdyt muutokset ja versionpäivityksen
ylläpitotarve kuvataan tiedostossa `ui/fluent/README.md`. Tekstikentän oman
valikon vuoksi käännöksessä tarvitaan Slint 1.16:n sisäinen tyyppi;
rakennusskripti sallii sen vain omassa prosessissaan. Käyttäjän ei tarvitse
asettaa ympäristömuuttujia tai asentaa teemaa erikseen.
