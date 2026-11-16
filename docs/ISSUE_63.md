# Issue 63: käyttöliittymä ja automatiikka

Kehitysmuutokset version 0.7.9 pohjalta: issue #63.

- Modaali pysäyttää taustan hiirirullan. Modaalin oma sisältö saa edelleen vieriä; myös vieritysalueen rajalla rulla pysähtyy modaaliin.
- Live-tulosten julkaisuikkunassa paikallinen kansiopolku näkyy ilman lisäasetusten avaamista. Polun voi valita ja kopioida kentästä tai Kopioi kansion polku -painikkeella myös julkaisun ollessa käynnissä.
- Tulosten oikean reunan esikatselussa kuljettaja ja kartanlukija esitetään samalla tyylillä. Nimet erotetaan sanalla ja, jos ne mahtuvat riville. Muuten kartanlukija alkaa omalta riviltään.
- Oikean rivin tuonnin esikatselu ja tallennus vertaavat AT-, JAT-, ULA- ja M-merkintöjen määriä valitun radan ajanottoasemiin. Numeroitu tunnus (esim. AT1) lasketaan tyypin mukaan. Virhe kertoo kummankin määrän. Myös korvaava tuonti tarkistetaan. RT:t ja lähtö L eivät kuulu vertailuun; Emit-/käsiaikavalinta ei muuta määrää.
- Luokan ensimmäisen parin lähtöaika ja lähtöväli näkyvät ja ovat muokattavissa. Luettelon lähtöväli näytetään sekunteina, jolloin myös minuutista poikkeava lähtöväli näkyy oikein. Myöhästymisrajan puuttunut muokkauskenttä on palautettu. Raja käyttää kokonaisminuutteja, ei pyöristä sekunteja.
- Asetukset → Tulostin -osion alla on Automatiikka. Tulkin automaattinen hyväksyntä ja parin tuloslaput käyttävät samaa kilpailukohtaista valintaa kuin AS tulkin yhteysikkuna. Verkkokilpailu pitää luoda ennen tulkin automatiikan käyttöönottoa.
- Erillinen valinta tulostaa luokan valmistuttua sekä luokka- että asematulokset yhtenä työnä. Se ei edellytä puhelintulkkausta. Tyhjä luokka tai keskeneräinen pari estää automaattisen koonnin. Hyväksytty tulos, vahvistettu keskeytys ja muut lopulliset toimitsijatilat noudattavat olemassa olevaa julkaisukäytäntöä.
- Valinnat ovat oletuksena pois, kilpailukohtaisia ja käytettävissä vain pääkoneella. Beginners Cupin erillinen tulos ei kuulu tähän luokkakoontiautomatiikkaan.
- Koonnin sisältötunniste tallennetaan ennen jonoon lähettämistä. Sama sisältö ei tulostu uudelleen ohjelman käynnistyessä tai taustakierroksella. Muuttunut, uudelleen hyväksytty tulos muodostaa uuden koonnin. Fyysisen tulostimen virheen tai keskeytyneen lähetyksen jälkeen tulosta koonti tarvittaessa käsin nykyisestä luokkatulostuksesta.

## Tarkistaminen

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo test --locked --features desktop --test interpretation --test printing --test preferences
cargo clippy --locked --features gui --all-targets -- -D warnings
cargo build --locked --features gui --bin as2026-gui
```

Windowsin eristetty GUI-koe (PowerShell; ei käytä oikeaa kilpailua tai fyysistä tulostinta):

```powershell
$env:AS2026_ISSUE63_GUI_TEST = '1'
$env:SLINT_BACKEND = 'winit-software'
$env:SLINT_SCALE_FACTOR = '1'
target/debug/as2026-gui.exe --demo --smoke-test --capture-dir target/issue63-100
$env:SLINT_SCALE_FACTOR = '1.5'
target/debug/as2026-gui.exe --demo --smoke-test --capture-dir target/issue63-150
```

Koe ajaa ensin nykyisen koko kilpailun GUI-työnkulun ja lisää siihen luokka-asetusten tallennuksen, virheellisen minuuttirajan hylkäyksen, automaattisen koonnin molemmat osat, toistotulostuksen eston ja asetusten uudelleenavauksen. Ajastettu osa todentaa aidot rullatapahtumat taustalla ja modaalissa sekä ottaa tarkistuskuvat. Olemassa olevat LAN-/verkkokatkokokeet säilyvät ennallaan. Fyysisen tulostimen ja leikepöydän toiminta kannattaa kokeilla vielä käyttäjän normaalissa Windows-istunnossa.
## Myöhästymisrajan yhteinen laskenta

Luokat-sivun Myöhästymisraja (min) muuttaa koko kilpailun yhteistä kokonaismyöhästymisrajaa. Sama arvo näkyy kaikilla luokilla ja kilpailun asetuksissa. Muutos tallentaa uuden rajan ja laskee aiemmat tallennetut tulokset uudelleen yhtenä tapahtumana; käsin syötetyt ajat säilyvät ja aiemmat hyväksynnät poistuvat. Myös osallistuvien BC-parien tallennetut tulokset lasketaan uudelleen. Uutta rajaa käytetään myös myöhemmin laskettaviin tuloksiin. Virheellinen syöte ei muuta kilpailua. Pelkkä saman rajan tallentaminen ei poista hyväksyntöjä.

Rajan näyttö käyttää samaa arvoa molemmissa näkymissä. Asetuskentän tyhjentäminen näyttää heti oletuksen 60 min myös muokkauskentässä. GUI-koe vaihtaa Luokat → Asetukset → Luokat ja tarkistaa arvot 75, 90, 45 ja oletuksen 60 molempiin suuntiin.
