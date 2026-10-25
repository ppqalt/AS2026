# Slintin Fluent-komponentit — AS2026:n beta-sovitus

## Lähde ja rajaus

Lähde on **Slint 1.18.1**, virallinen commit
`372cf0ee5577c3dfec309a45e7b778ba4e81b734`, paketin `i-slint-compiler`
hakemisto `widgets`. Jokaisen kopioidun Slint- ja SVG-tiedoston alkuperäisen
1.18.1-lähteen SHA-256 on `UPSTREAM.sha256`:ssä. Lähdetavut on verrattu myös
virallisen Git-tagin Git-puuhun. Hashit kuvaavat alkuperäisiä, eivät muokattuja tiedostoja.

Beta käyttää samaa 1.18.1-versiota runtimessa ja kääntäjässä. Vanhaa 1.16.0-
pohjaa on verrattu tiedosto kerrallaan ja projektin sovitukset siirretty uuteen
lähteeseen. `style-base.slint` on versioiden välillä muuttumaton; sekin on
verrattu 1.18.1-lähteeseen. Tämä ei ole pelkkä versionumeron vaihto.

`build.rs` lisää `style`-hakemiston hakupolkuun ja säilyttää Fluent-tyylin.
Näin sovellus käyttää samaa kontrollia kaikilla sivuilla. Paikallinen kopio
on tarpeen yhteiselle tehosteportille, suomenkieliselle tekstivalikolle,
sovelluksen preset-mitoille ja vakaalle vierityskaistalle. Standardin
käyttölogiikka kopioidaan mahdollisimman muuttamattomana upstreamista.

## Sovitukset ja 1.18.1-migraatio

- `std-widgets-impl.slint` vie nyt `MenuBarImpl`- ja `PopupMenuImpl`-toteutukset
  kääntäjän 1.18.1:n edellyttämästä paikasta. `common/menus.slint` käyttää
  uutta eriytettyä tuontia ja `close-popup`-callbackia; rekursiivista tuontia ei ole.
- `LineEdit` ja `LineEditBase` käyttävät upstreamin varsinaista
  `LineEditInterface`-rajapintaa. Vanhaa käsin kopioitua välitysrajapintaa
  ei ylläpidetä rinnalla. Mukana ovat kumoa/tee uudelleen sekä saavutettava
  tekstivalinnan asetus. Tekstivalikon komennot ovat suomeksi.
- `ComboBoxBase` sisältää 1.18.1:n `current-value`/`current-index`-synkronoinnin.
  Valittu teksti katkeaa hallitusti. Ponnahduslista avautuu AS2026:ssa
  säätimen alapuolelle ja näyttää enintään kuusi riviä.
- `ScrollView` tarjoaa 1.18.1:n `content-*`-rajapinnan. Sovelluksen kutsut
  käyttävät sitä; upstreamin vanhat `viewport-*`-yhteensopivuusaliakset
  ovat edelleen tarjolla. Palkille varataan oma 14 px kaista. Kaistan varaus
  ei riipu vieritystarpeesta, jotta asettelu ei muodosta takaisinkytkentää.
  `maximum`-muutos sekä ScrollView:n sisältö- ja näkyvän alueen muutokset rajaavat nykyisen vierityskohdan sallitulle alueelle myös palkin ollessa piirtoalueen ulkopuolella. Sisällön
  laajentuminen säilyttää kelvollisen kohdan; nollapituinen alue ei jaa nollalla.
- `Button`, `CheckBox`, `ListItem` ja valikkopohjat sisältävät uudemmat
  näppäin-, saavutettavuus- ja rajapintakorjaukset. Buttonin
  `control-height` ja `visual-variant` sekä syötteiden `control-height`
  ovat dokumentoituja AS2026-lisäyksiä. Vanhojen kutsujen oletuskoko säilyy.
- Värien ja mittojen projektisovitus tehdään `ui/theme.slint`-tokeneilla ja
  `styling.slint`-kartoituksella. Valinnaiset animaatiot käyttävät aina
  `Theme.effects-enabled`-porttia, jonka oletus on pois.
- SVG-kuvakkeet ja Slintin tunnuskuvat ovat muuttamattomia 1.18.1-resursseja.
  Lopullinen kuvakkeen väri määräytyy kontrollin tilasta. AboutSlint kertoo
  oikean kirjastoversion ja säilyttää alkuperäisen tunnuksen ja linkin.

## Päivittäminen ja tarkistus

Päivitä runtime ja kääntäjä yhdessä. Vertaa jokaista SHA-luettelon tiedostoa
uuteen lähteeseen, siirrä vain yllä kuvatut projektisovitukset ja päivitä
alkuperäislähteiden hashit. Tarkista erityisesti impl-viennit, tekstivalikot,
interface-rajapinnat, saavutettavuus ja popupin sulkemisjärjestys.

Aja Rust- ja Slint-käännös, regressiot, Clippy sekä galleria ja oikeat
GUI-työnkulut Linuxissa ja Windowsissa. Testaa 100/150 % skaalaus, hiiri,
Tab/Shift+Tab, Enter/Space/Esc, leikepöytä ja sisällön kutistuva vieritys.
Käännös tai lähdetekstin tarkistus ei yksin todista visuaalista oikeellisuutta.

## Lisenssit

Alkuperäiset tekijänoikeus- ja SPDX-merkinnät säilyvät. Nämä tiedostot eivät
muutu AS2026:n BSD-2-Clause-lisenssin alaisiksi. Alkuperäiset lisenssivaihtoehdot
ovat `licenses`-hakemistossa: GPL-3.0-only, LicenseRef-Slint-Royalty-free-2.0 ja
LicenseRef-Slint-Software-3.0. MadeWithSlint-tunnusten CC-BY-ND-4.0 säilyy.

Betan komponenttien käyttö: [preset-katalogi](../../docs/FLUENT2_KOMPONENTTISTANDARDI.md)
ja [kehitysgalleria](../../docs/FLUENT2_GALLERIA.md).

## Laajennettu perusvalikoima ja järjestelmätila

Slintin alkuperäiset Slider-, Switch-, SpinBox-, TextEdit-, RadioGroup-, TabWidget-, ProgressIndicator- ja ToolTip-pohjat on lisätty samasta 1.18.1-tagista. Animaatioiden `enabled` käyttää yhteistä tehosteporttia; indeterminaattinen etenemispalkki ei lue `animation-tick`-arvoa portin ollessa pois. Sliderin askel pyöristyy suhteessa minimiin, disabled estää muutokset ja pelkkä hiiren liike ei muuta arvoa. Nollapituiset liukualueet on suojattu. Sliderillä on näkyvä fokusreunus.

`color-scheme.slint` on muuttamaton upstream-silta käyttöjärjestelmän värimalliin. Vain tämä tiedosto lukee `SlintInternal.color-scheme`-arvoa. `build.rs`:n kokeellisten rajapintojen lippu on tarpeen sekä tälle sillalle että `LineEditInterface`-rajapinnalle. Se ei ota käyttöön valinnaisia visuaalisia tehosteita.

Buttonin `icon-only` ja `menu-button` ovat pieniä projektisovituksia; jälkimmäinen lisää tekstin jälkeen avausnuolen. Väripaletti on keskitetty: järjestelmän mukana vaihtuva neutraali vaalea/tumma tai käyttäjän valitsema Dark Spruce.

Valikkojen ja yhdistelmäruutujen ulkokehys käyttää `Theme.radius-large`-tokenia (8 px); sisäisten valintojen 4 px:n säde säilyy. Työkaluvihjeen alkuperäinen 4 px:n säde vastaa erillistä tooltip-sääntöä.
