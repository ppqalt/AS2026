# Yhdistetyt luokat ja NAC Beginners Cup

Tutkittu 19.10.2026, lähtökohtana `main`-haaran commit `82f3ff3` (0.6.4).

## Yhdistetyn luokan luonti

Luokat-sivun **Lisää yhdistetty luokka** avaa valintaikkunan, jossa annetaan
nimi ja valitaan olemassa olevat jäsenluokat valintaruuduilla. Nousupisteet ja
sarjapistemenetelmä voidaan valita samassa ikkunassa. **Lisää luokka** tallentaa
koko määritelmän kerralla; **Peruuta** tai Esc sulkee tallentamatta. Virheellinen
syöte tai tallennusvirhe jättää syötteen korjattavaksi ikkunaan.

Yhdistetyt luokat näkyvät Luokat-taulukossa Y1–Y4-riveinä, joiden nimessä näkyvät
myös jäsenluokat. Rivin valitseminen avaa saman valintaikkunan muokkaamista
varten. Tunnukset Y1–Y4 ovat taulukon merkintöjä, eivät uusia kilpailijaluokkia.
Alkuperäiset luokat, radat, kilpailijat ja lähtöarvonta säilyvät.

Tallennus käyttää nykyistä `yhdistetyt_luokat`-mallia sekä yhteistä validoitua
tallennusta. Nykyinen enintään neljän yhdistetyn luokan raja säilyy.
Yhdistetty tulos käyttää alkuperäisten luokkien valmiita tuloksia ja nykyistä
`ranking`-laskentaa ja tulosvientiä. Rataa tai virhepisteitä ei lasketa uudelleen
yhdistetyn luokan perusteella.

## Beginners Cupin vahvistettu tarve

BC-osallistujalle tarvitaan sekä NAC- että BC-tulos samasta Emit-lukemasta.
BC-tuloksessa osa NAC-radan asemista jätetään huomioimatta. Käsin syötettävien
virhepisteiden pitää olla erillisiä, sillä BC:hen kuuluvat vain sen omalla
osuudella tulleet virheet. NAC:n virhepisteitä ei voi sellaisenaan kopioida BC:hen.

## Nykyisen toteutuksen rajoitukset

- `native::Project::record_readout` estää kortinluvun, jos sama Emit-numero on
  usealla kilpailijalla. Toisen kilpailijatietueen lisääminen ei toimi ratkaisuna.
- `matching::match_route` muodostaa radasta puuttuvista korttileimoista
  ylimääräisiä rivejä. `timing` pisteyttää ylimääräiset leimat. BC-radan
  lyhentäminen ilman rajausmääritystä voisi siis lisätä virhepisteitä.
- `Driver::lisavirheet` ja `Driver::tallennettu_tulos` ovat kilpailijakohtaisia.
  Niissä ei nykyisin ole erillistä paikkaa rinnakkaiselle BC-tulokselle.
- Yhdistetty luokka kokoaa tulokset, mutta ei muuta niiden laskentarataa.

## Suositeltu seuraava toteutus

Kilpailija ilmoitetaan kerran NAC-luokkaan. Kilpailijalle valitaan lisäksi
**Osallistuu Beginners Cupiin**. BC määritellään erilliseksi laskentaluokaksi,
jolla on oma rata ja osallistujavalinta. Pelkkä yhdistetty luokka ei riitä.

Yksi alkuperäinen Emit-lukema säilytetään muuttamattomana historiassa.
Samasta lukemasta muodostetaan kaksi tulosta, jotka tunnistetaan kilpailijan
ja laskentaluokan perusteella. Kummallakin tuloksella on omat asemarivit,
käsikellot, käsin syötettävät lisävirheet, korjaukset ja hyväksyntä.
BC-virheiden muutos ei muuta NAC-tulosta eikä sen hyväksyntää.

BC-radan asemat yhdistetään NAC-radan asemakohtiin ja vain erikseen
poissuljetuiksi määritetyt leimat jätetään BC-laskennassa huomiotta.
Pelkkä Emit-koodien suodatus ei riitä, koska sama aseman koodi voi esiintyä
radalla useasti. Muita ylimääräisiä leimoja ei saa sivuuttaa automaattisesti.
Alkuperäiset leimausajat ja ajanlaskennan ankkurit säilytetään.

Tulokset-sivulla valitaan NAC- tai BC-laskenta. Tulosvienti ja live-tulokset
sisältävät molemmat luokat, mutta ilmoittautuneiden määrä, Emit-kortin omistus
ja lähtöarvonta käsittelevät kilpailijan kerran. Uusi korttisisältö tai
BC-ratamuutos päivittää ja mitätöi hyväksynnän vain niiltä osin kuin tuloksen
laskentaperuste muuttuu.

Toteutukseen tarvitaan regressiokokeet ainakin ohitettavasta asemasta,
toistuvasta Emit-koodista, todellisesta ylimääräisestä leimasta, erillisistä
NAC/BC-virhepisteistä ja hyväksynnöistä sekä uusintaluvusta ja viennin
uudelleenavauksesta. BC-laskentaa ei ole toteutettu tässä muutoksessa.

## Varmennus tässä työtilassa

Koko `ui/app.slint` kääntyy Slint 1.16.0:n Node-sidonnalla ja projektin omalla
Fluent-tyylillä. Ikkuna ja valintaikkunan ominaisuudet alustuvat kokoluokissa
900 × 600 ja 1440 × 900 ilman näkyvän ikkunan avaamista. Tämä ei ole visuaalinen
tai näppäimistöllä tehty käytettävyyskoe eikä Rust-sovittimen käännöskoe.

Rustfmt-muotoilutarkistus ja `git diff --check` onnistuvat. Lisätyt Rust-testit
tarkistavat jäsenvalintojen atomisen tallentamisen, virheellisten jäsenten
hylkäämisen ja yhdistetyn luokan rivitunnisteen säilymisen haussa. GUI:n
elinkaarikokeeseen on lisätty luonti-, validointi-, peruutus- ja muokkausvaiheet.
Täyttä Rust-työkaluketjua ei ole asennettu tähän työtilaan, joten testejä ei ole
ajettu. Ennen yhdistämistä tarvitaan normaalit Rust/GUI-testit ja Clippy sekä
Windowsin näkyvän ikkunan, vierityksen, näppäimistön ja 150 % skaalauksen koe.
