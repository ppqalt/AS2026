# Slintin Fluent-komponentit — paikallinen sovitus

Lähde: Slint **1.16.0**, paketti `i-slint-compiler`, hakemisto `widgets`.
**Beta-haaran Rust Slint- ja slint-build-versio on nyt 1.18.1**, mutta tämä hakemisto
sisältää edelleen **1.16.0-pohjaisen vendoroidun Fluent-sovituksen**. Niitä ei saa
kutsua saman upstream-version tiedostoiksi ennen oikeaa migraatiota ja testejä. Alkuperäisten tiedostojen nimet ja tiivisteet ovat tiedostossa
`UPSTREAM.sha256`.

Tässä on vain sovelluksen tarvitsemien Fluent-komponenttien riippuvuudet:
Button, LineEdit, ComboBox, CheckBox, ScrollView, ListView, tekstikentän valikko ja
AboutSlint. Käyttämättömiä säätimiä, päivämäärävalitsimia tai taulukoita ei
ole kopioitu. Käyttöliittymän varsinainen taulukko säilyy AS2026:n omana
hiirellä valittavana komponenttina.

`build.rs` lisää `style`-hakemiston Slint-kääntäjän julkiseen hakemistopolkuun.
Tällöin `std-widgets.slint` ja sen riippuvuudet löytyvät tästä ennen
sisäänrakennettua oletustyyliä. `.with_style("fluent")` säilytetään.
Kuvakkeet ja Slint-tunnus sisällytetään ohjelmatiedostoon; erillistä
teemakansiota ei tarvita käyttäjän koneella.

## Paikalliset muutokset

- Oletusarvoisesti kaikki valinnaiset tehosteet ovat pois käytöstä (`Theme.effects-enabled: false`). Betassa Button-tyylin taustavärisiirtymä on palautettu **vain opt-in-portilla**; tehostetta ei saa käynnistää ilman käyttäjän Tietoja-ikkunassa tekemää valintaa.
- Varjojen määritykset on poistettu, myös valikoiden ponnahduspinnoista.
- FluentPalette käyttää pelkästään `ui/theme.slint`-tiedoston peittäviä
  Dark Spruce -värejä. Ei järjestelmän sinistä korostusta, läpikuultavia
  täyttöjä tai reunojen liukuvärejä.
- Button, LineEdit ja ComboBox käyttävät oletuksena yhteistä 36 pikselin säätimen
  korkeutta, tekstikokoa, kulmasädettä ja sisennyksiä. Korostettu toiminto
  on saman Fluent-painikkeen `primary`-tila: myös sen näppäimistökohdistus,
  käytöstä poistaminen ja esteettömyystiedot säilyvät samoina.
- ComboBoxin valikko avautuu kentän alapuolelle. Näkyvissä on enintään kuusi
  tasakorkuista vaihtoehtoa; pidempi lista vierii pystysuunnassa. Vaakasuuntaa
  ei käytetä, ja sisällön leveys seuraa valikon leveyttä.
- Vierityspalkin nuolten piilotus käyttää välitöntä `visible`-tilaa.
  Käytöstä poistetun valikkorivin teksti saa peittävän vaimennetun värin.
- ScrollView varaa vierityspalkeille oman `Theme.scrollbar-width`-kaistan
  (14 pikseliä). Pystykaista on oikealla ja vaakakaista alhaalla. Kentät eivät
  jää palkkien alle; `always-off` vapauttaa kyseisen kaistan sisällölle.
  Varattu tila ei riipu palkin hetkellisestä näkyvyydestä, jotta sisällön
  koko ja vieritystarve eivät muodosta asettelun takaisinkytkentää.
- Nuolikuvakkeiden läpikuultavat maskit on muutettu peittäviksi;
  lopullinen kuvakeväri tulee yhteisestä paletista.
- LineEditin sisäinen `interface`/`uses`-syntaksi on korvattu samoja
  ominaisuuksia, kutsuja ja tapahtumia välittävillä tavallisilla
  Slint-määrityksillä. Tekstikentän käytös on säilytetty.
- Tekstikentän valikon tekstit ovat suomeksi. AboutSlintin version
  otsikko on `Versio`; Slint-tunnuskuvat säilyvät muuttamattomina.

Tekstikentän valikko käyttää Slint 1.16:n sisäistä
`ContextMenuInternal`-tyyppiä. Siksi rakennusskripti asettaa vain omaan
prosessiinsa `SLINT_ENABLE_EXPERIMENTAL_FEATURES=1` ennen kääntäjän luontia.
Tämä ei aseta käyttäjän ympäristömuuttujia eikä tuo uusia kirjastoja.

## Ylläpito

Paikallinen kopio lisää ylläpidettävää lähdekoodia. Slint-versiota
päivitettäessä vertaa jokainen tässä lueteltu tiedosto uuden version
alkuperäiseen: sisäisten valikkojen, kohdistuksen ja tekstikentän rajapinnat
voivat muuttua. Säilytä omat efektittömyysmuutokset, käännä käyttöliittymä,
aja `fluent2_theme`-testit ja GUI-työnkulkukoe sekä tarkista valikot,
leikepöytä, vieritys ja näyttöskaalaus Linuxissa ja Windowsissa.

## Lisenssit

Alkuperäiset tekijänoikeus- ja SPDX-merkinnät on säilytetty. Muokattuihin
Slint-tiedostoihin on lisätty AS2026-merkintä. Tämä hakemisto **ei muutu
AS2026:n BSD-2-Clause-lisenssin alaiseksi**.

Slint-lähteiden tarjoamat lisenssivaihtoehdot ovat hakemistossa `licenses`:
GPL-3.0-only, LicenseRef-Slint-Royalty-free-2.0 ja
LicenseRef-Slint-Software-3.0. Muuttamattomiin MadeWithSlint-tunnuksiin
liittyvä CC-BY-ND-4.0-teksti on samassa hakemistossa.

## Beta-haaran preset-kirjasto

`ui/presets.slint` tarjoaa valmiita 28/36/44 px:n kokoja. Buttonin `control-height` ja `visual-variant` sekä LineEditin/ComboBoxin `control-height` ovat AS2026:n lisäyksiä tähän Slint 1.16.0 -widgetkopioon. Oletus säilyy ennallaan vanhoille call-siteille. `danger` ja `subtle` ovat uudet semanttiset variantit. Ohjeet ja testikäynnistys: `docs/FLUENT2_GALLERIA.md` ja `docs/FLUENT2_KOMPONENTTISTANDARDI.md`.
