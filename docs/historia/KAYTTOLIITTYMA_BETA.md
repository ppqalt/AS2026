# Historia: käyttöliittymän beta

> Historiallinen kuvaus beta-haaran työvaiheesta. Haaran nimi, mitat ja beta-tunnisteet eivät kuvaa nykyversiota. Nykyinen [käyttöliittymäopas](../KAYTTOLIITTYMAOPAS.md).

Haara: `beta/kayttoliittyma`. Pohjana nykyisen toimivan version commit
`00a9fdb`. Muutokset arvioidaan tässä haarassa ennen mahdollista yhdistämistä.
Rust, Slint, kilpailumalli ja Fluent 2 / Dark Spruce säilyvät.

## Tilanjako

- Luettelo ja tiedot ovat rinnakkain, kun työalueella riittää vähintään
  540 px pääsisällölle sekä 368 px tietopaneelille ja paneelien välille.
  Tietopaneeli kasvaa suurella näytöllä hallitusti enintään 420 pikseliin.
- Kapeassa tai alle 440 px korkeassa työalueessa Luettelo–Tiedot-valinta
  antaa kummallekin koko työalueen. Ikkunan vähimmäiskoko säilyy 900 × 600:ssa.
  Vaihto ei muuta kilpailutietoja tai poista keskeneräisen syötteen suojausta.
- Taulukon käyttökelpoinen vähimmäiskorkeus on 180 px. Jos kahta taulukkoa
  ei mahdu järkevästi päällekkäin, ne avataan erikseen. Pitkät taulukot
  vierivät, eikä rivimäärä kasvata ikkunaa rajattomasti.
- Säätimet ovat edelleen 36 px korkeita. Välit ja tekstikoot tulevat Themestä.
  Pitkä kentän nimi saa rivittyä; syöttösäädin ei pienene sen vuoksi.

## Sivujen erot

**Kilpailijat ja Luokat:** luettelo vasemmalla, sama rajattu tietopaneeli oikealla.
Kapeassa ikkunassa tiedot avataan omana näkymänään. Lisääminen avaa lomakkeen.

**Tulokset:** leveässä ikkunassa tulosluettelo ja asematulokset jakavat pääalueen.
Kapeassa ikkunassa niillä on omat valintansa, jotta kumpikaan taulukko ei jää
pieneksi kaistaleeksi. Valitun tuloksen yhteenveto ja korjauslomake vierivät
yhdessä. Koko kilpailun huomautukset avataan samaa valintaikkunapohjaa käyttävään
ikkunaan; ne eivät vie taulukoiden korkeutta.

**Radat ja asemat:** leveässä ikkunassa ratavalinta on tiivis ja asemaluettelo
käyttää loppukorkeuden. Kapeassa ikkunassa rata valitaan erillisestä luettelosta.
Radan ja aseman lomakkeilla on yksi yhteinen vieritys. Kapean tilan leveässä
tietonäkymässä aseman kuusi kenttää voivat asettua kahteen palstaan.
Radan tiedot ja aseman tiedot valitaan tällöin erikseen, jotta valitun aseman
muokkaus ei jää radan tietojen alapuolelle. Leveässä näkymässä molemmat säilyvät.

**Emit-lukeminen:** leveässä ikkunassa historia ja valitun kortin leimaukset
ovat rinnakkain. Leimaustaulukolle varataan tavallista lomakepaneelia enemmän
leveyttä. Kapeassa tilassa kumpikin saa koko työalueen. Kokeet eivät käynnistä
fyysistä lukijaa.

**Yleiskatsaus:** kortit käyttävät sivun leveyden. **Asetukset:** tietojen lukuleveys on rajattu 1120 pikseliin.
Työvaiheiden painikkeet jakautuvat kapeassa ikkunassa kahdelle riville.
Asetusten tallennuspaikka siirtyy kapeassa ikkunassa lomakkeen alle; koko sisältö
vierii yhdessä. Sivupalkin navigaatio käyttää vapaan korkeuden ilman toista
venyvää tyhjää aluetta.

## Kokeilu

`cargo run --locked --features gui --bin as2026-gui -- --demo` avaa projektihakemistosta
erillisen esimerkkikilpailun sekä Linuxissa että Windowsissa. Käyttäjän nykyinen
asennus ja alkuperäiset kilpailutiedot eivät kuulu tähän kokeiluun.
Beta tunnistetaan ikkunan otsikosta ja sivupalkin AS2026 Beta -nimestä.

Todelliset Slint-kuvakaappaukset ja ajolokit tallennetaan paikalliseen
raporttikansioon. Kokokoe varmistaa käyttöjärjestelmältä toteutuneet mitat,
ei pelkästään pyydettyä ikkunakokoa. Koe kattaa kaikki seitsemän pääsivua,
tietopaneelit sekä uudet tietueet ja huomautusikkunan useassa koossa ja
100 %:n / 150 %:n skaalauksilla. Lisäksi koe siirtää oikean sovellusikkunan
koko näytölle ja varmistaa sen tilan sekä näyttöä vastaavat pikselimitat.

Excel-tuonnin **Tuotavat tiedot** -valinta rajaa uuden kilpailun sisällön:
Koko kilpailu, Tuo vain radat tai Tuo vain kilpailijat. Pelkkien kilpailijoiden
mukana muodostetaan heidän luokkansa ilman ratoja. Ratavalinnat täydennetään
myöhemmin Luokat-sivulla. Rajauksen ulkopuolisen välilehden virheet eivät estä
tuontia, mutta tuotavien tietojen tarkistukset säilyvät. Lähde ja avoinna oleva
kilpailu säilyvät ennallaan; tuonti tallennetaan uutena kilpailuna.
