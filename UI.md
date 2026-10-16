# AS2026:n käyttöliittymän muokkaaminen

GUI on Slintillä toteutettu Rust-ohjelma Linuxille ja Windowsille. Se käynnistetään erillisenä `as2026-gui`-ohjelmana. Nykyinen `as2026`-komentoriviohjelma säilyy käytettävissä ilman GUI-riippuvuuksia.

```sh
cargo run --features gui --bin as2026-gui
cargo run --features gui --bin as2026-gui -- --demo
cargo run --features gui --bin as2026-gui -- oma-kilpailu
```

## Näkymät ja käyttö

Vasemman reunan navigaatio pysyy näkyvissä. Työalue käyttää tavallisia luetteloita, taulukoita ja lomakkeita; näkyvä tila kertoo avatun kilpailun ja tallennuksen onnistumisen. Ulkoasu on tumma ja perustuu hillittyihin väreihin, selkeään tekstihierarkiaan ja tiiviisiin väleihin. Suuret koristekortit, liukuvärit tai animaatiot eivät kuulu perusrakenteeseen.

| Näkymä | Tehtävä |
| --- | --- |
| Yleiskatsaus | Kilpailun avaaminen ja perustaminen sekä yhteenveto. |
| Kilpailijat | Haettava kilpailijaluettelo, uuden kilpailijan lisäys ja tietojen muokkaus. |
| Emit-lukeminen | Automaattinen tai manuaalinen portti, jatkuvan luvun käynnistys ja pysäytys sekä lukutapahtumien seuranta. |
| Tulokset | Ajantasainen tulosluettelo, laskennan huomautukset ja HTML-raportti. |
| Luokat | Luokan lisäys ja muokkaus sekä yhteys rataan. |
| Radat ja asemat | Ratojen ja niiden asemien lisäys ja muokkaus. |
| Asetukset | Kilpailujen tallennuspaikka, ohjelmapäivitykset ja avoimen kilpailun perustiedot. |

Olemassa olevien tietojen kelvolliset muutokset tallentuvat Enterillä tai poistuttaessa kentästä. Uusi tietue täytetään luonnoksena ja lisätään vasta ehjänä. Luonnoksen syötekentät ovat käyttöliittymän väliaikaista tilaa; ne eivät ole toinen itsenäinen kilpailutietokanta. Virheellinen syöte pitää voida korjata menettämättä viimeksi oikein tallennettua kilpailua. Tallennusvirhettä ei saa esittää onnistuneena tallennuksena.

Luokan ja radan viittaukset, aseman tyyppi ja laskentasääntö valitaan nimetyistä vaihtoehdoista. ULA-pyöristykset näkyvät vain ULA-asemalle. Uusi väliasema sijoitetaan ennen radan viimeistä maalia; järjestystä voi muuttaa nuolipainikkeilla. Sisäisten tunnusten vapaa kirjoittaminen ei ole näiden valintojen ensisijainen käyttötapa. Kentän muokkaaminen muodostaa ensin luonnoksen; jokaisesta näppäinpainalluksesta ei kirjoiteta kilpailutiedostoa uudelleen.

Vanhoja AS2015A-kilpailuja käsitellään lähdeaineistona. Niiden tarkastelu ei tee automaattista muunnosta eikä kirjoita vanhojen tiedostojen päälle. Uusien kilpailujen muokkaus käyttää `kilpailu.toml`-tiedostoa.

## Missä mitäkin muutetaan

| Kohde | Paikka |
| --- | --- |
| Värit, tekstikoot ja mitat | `ui/theme.slint`. |
| Kentät, painikkeet ja muut yhteiset komponentit | `ui/components.slint`. |
| Seitsemän näkymän asettelu ja käyttäjätekstit | `ui/app.slint`. |
| Slintin ja Rustin välillä kulkevat esitystyypit | `ui/types.slint`. |
| Rustin ja Slintin välinen tiedonsiirto sekä käyttöliittymän tapahtumat | `src/gui/mod.rs`. |
| Sovellusasetukset ja päivitysten GUI-taustatyö | `src/gui/desktop.rs`. |
| Pysyvät kilpailukansiot ja viimeksi avattu kilpailu | `src/preferences.rs`. |
| Versionoitu asennus, käynnistin ja palautus | `src/installation.rs`, `src/bin/as2026-launcher.rs`. |
| Allekirjoituksen tarkistus ja lataukset | `src/updates.rs`, `release-public-key.hex`. |
| GUI:n käynnistäminen | `src/bin/as2026-gui.rs`. |
| Lomakkeet ja nimetyt valinnat | `src/gui/forms.rs`. |
| Taulukot, haku ja tulosten esittäminen | `src/gui/tables.rs`. |
| GUI-sovittimen laitteeton työnkulkukoe | `src/gui/smoke.rs`. |
| Kilpailun yhteinen tietomalli | `src/native.rs`. |
| Turvallinen paikallinen tallennus | `src/storage.rs`, `ProjectStore::update`. |
| Kilpailutietojen muokkaus ja laskennan kokoaminen | `src/workspace.rs`. |
| Jatkuvan lukemisen taustatyö | `src/live_reader.rs`. |
| Sarjaportin ja kehysten käsittely | `src/reader.rs` ja `src/emit.rs`. |
| Laskenta, järjestys ja raporttien sisältö | `src/application.rs`, `src/timing.rs`, `src/scoring.rs`, `src/ranking.rs` ja `src/report.rs`. |
| Vakiokomponenttien suomennokset | `translations/fi/LC_MESSAGES/as2026.po`. |
| Slintin käännösasetukset | `build.rs`. |
| Projektin lisenssi ja ulkopuolisten kirjastojen ehdot | `LICENSE` ja `THIRD_PARTY.md`; Slint-maininta `ui/app.slint`-tiedoston Tietoja-ikkunassa. |

Ulkoasun muutoksessa aloita yhteisistä väreistä ja komponenteista. Muuta esimerkiksi kentän tai painikkeen kokoa yhteisessä komponentissa, jotta sama muutos tulee kaikkialle. Slint vastaa esittämisestä ja käyttäjän syötteestä. Laskentasääntöjä, tiedostojen kirjoittamista tai kilpailijoiden kohdistamista ei toteuteta uudelleen Slint-tiedostoissa.

Uuden lomakekentän lisääminen etenee näin:

1. Tarkista, onko kenttä jo yhteisessä kilpailumallissa. Jos on, käytä sitä.
2. Lisää lomakkeeseen suomenkielinen nimi, selkeä syöte ja tarvittaessa esimerkki.
3. Jäsennä ja tarkista arvo Rustissa samalla muokkauspolulla kuin muu kilpailutieto.
4. Tallenna muutos `ProjectStore::update`-rajapinnan kautta. Päivitä listat ja tulokset onnistuneesta yhteisestä mallista.
5. Tarkista myös virheellinen syöte ja tallennusvirhe, jos lisäys muuttaa tallennuskäyttäytymistä.

Kenttien välillä liikutaan Tabilla ja arvot voi hyväksyä Enterillä. Säilytä näkyvä näppäimistökohdistus ja kenttien tunnistettavat nimet, kun muutat komponentteja. Pitkien nimien ja polkujen sekä pienen ikkunan pitää säilyä käytettävinä.

## Emit-luku ja ajantasainen kilpailu

Sarjaportin odotus tapahtuu taustalla. Ikkunan tapahtumakiertoa ei saa pysäyttää laitteen odottamiseen. Fyysisen lukutapahtuman, kortin varsinaisten leimausten ja kilpailijalle lasketun tuloksen ero säilyy samana kuin komentorivillä.

Kilpailun vaihtaminen ei saa siirtää vanhan kilpailun lukutapahtumaa uuden kilpailun tiedostoon. Muokkaus, lukuhistorian tallennus ja tulosten uudelleenlaskenta käyttävät samaa ajantasaista kilpailua. Pelkkä muutoksen esittäminen Slintin listamallissa ei ole tallennus.

Onnistunut tallennettu kortinluku näkyy pienenä kolmen sekunnin kuittauksena lukuhistorian otsikkorivillä. Peräkkäiset saman kortin samat varsinaiset leimat esitetään yhdellä rivillä; Lukuja-sarake kertoo lukukertojen määrän ja yksityiskohdat näyttävät ryhmän viimeisimmän lukeman. Eri kortit, muuttuneet leimat ja virheelliset lukemat näkyvät erillisinä. Ryhmittely koskee vain esitystä: kaikki alkuperäiset lukemat säilyvät kilpailutiedostossa.

Linuxin tiedostonvalinta käyttää työpöydän XDG-portaalia ja Windowsin tiedostonvalinta Windowsin omaa valintaikkunaa. Käyttöjärjestelmän valintaikkunan kieli ja ulkoasu määräytyvät järjestelmän asetuksista. Sovelluksen omat tekstit sekä Slintin tekstikenttien Leikkaa-, Kopioi-, Liitä- ja Valitse kaikki -toiminnot ovat suomeksi.

## Tarkistaminen

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo test --all-targets --features gui
cargo clippy --all-targets --features gui -- -D warnings
cargo build --release --features gui --bin as2026-gui
```

Rust-testit suojaavat kilpailumallia, tallennusta ja sovelluslogiikkaa. Kääntymistarkistus ei varmista ulkoasua. Tarkista käynnistetystä ikkunasta myös kaikkien seitsemän näkymän asettelu, näppäimistökäyttö, pitkät tekstit ja tallennustila. Älä käytä lähdekoodin esimerkkikilpailua oikeiden tietojen tallennuspaikkana; luo erillinen kokeilukansio tai kopioi synteettinen esimerkki sinne.

Esimerkin voi avata turvallisesti `--demo`-valinnalla. Se luo oman väliaikaisen kilpailukopion, jota voi muokata muuttamatta repositorion esimerkkejä. Kehittäjän kuvakaappausajo avaa saman esimerkin, ottaa kuvat seitsemästä näkymästä ja päättyy:

```sh
cargo run --features gui --bin as2026-gui -- --demo --capture-dir /tmp/as2026-kuvat
```

Kuvakaappausajo tarvitsee tässä toteutuksessa toimivan graafisen istunnon. Se auttaa tarkastamaan asettelua, mutta ei itsessään todista kaikkia hiiri- ja näppäimistötyönkulkuja toimiviksi.

GUI käyttää nykyistä laskentaytimen toteutusta ja sen tunnetut rajat säilyvät. Laskennan täydellinen AS2015A-vastaavuus, kaikki vanhan ohjelman erityistoiminnot ja Windowsin fyysinen Emit-luku vaativat edelleen erilliset varmennukset. Paikallinen tiedostotallennus ei ole usean koneen tietokanta.

## Tässä työvaiheessa varmennetut asiat

Tumma teema tarkastettiin käynnissä olevasta Linux-ikkunasta ja kaikkien seitsemän näkymän kuvakaappauksista. Myös Slintin vakiokenttien tumma värimaailma asetetaan yhteisen Theme-määrityksen kautta, jotta kenttien tekstit eivät jää vaalealle taustalle näkymättömiksi.

Laitteeton työnkulkukoe käyttää oikeita GUI-callbackeja omassa synteettisessä kilpailukopiossaan. Se tarkistaa luonnoksen hyväksymisen näkymän vaihdossa, virheellisen kentän säilyttämisen, uuden kilpailijan, luokkatunnuksen viittaukset, aseman muutoksen vaikutuksen tulokseen, asematyypin mukaiset sääntövalinnat ja tallentamatta jääneen Emit-lukeman uudelleenyrityksen:

```sh
cargo run --features gui --bin as2026-gui -- --demo --smoke-test --capture-dir /tmp/as2026-kuvat
```

Kuvakaappausajoon yhdistetty koe lähettää myös näppäintapahtumat käynnissä olevalle ikkunalle: Ctrl+N avaa uuden kilpailun ikkunan, teksti menee nimi- ja polkukenttiin, Tab siirtää kentästä toiseen ja Esc sulkee ikkunan. Tämä ajo tallentaa lisäksi kuvat uuden kilpailun ikkunasta ja Tietoja-ikkunasta. Tietoja-ikkunan alkuperäinen AboutSlint-komponentti säilytetään Slintin Royalty-free-lisenssin mainintavaatimuksen vuoksi.

Koe ei avaa fyysistä lukijaa. Fyysistä Emit-testiä ei tässä GUI-vaiheessa toistettu. Aiempi CLI-laitetesti ei yksin todista GUI:n fyysistä lukijatyönkulkua.

Tulosten tallennuspaikan valinta tehdään erillisessä tehtävässä, joten raportin valintaikkuna ei pysäytä Emit-tapahtumien käsittelyä. Tallennusvirheessä lukija pysäytetään ja jo luetut kehykset säilytetään muistissa; käyttöliittymä tarjoaa uudelleenyrityksen ennen sulkemista. Lukuhistorian luettelo näyttää viimeiset 50 lukemaa, mutta koko historia säilyy kilpailutiedostossa.

## Pysyvä asennus ja kilpailukansiot

Uuden kilpailun luomisessa riittää nimi. Ohjelma tekee sen alle yksilöllisen turvallisen kansion Asiakirjat-kansion AS2026/Kilpailut-juureen. Jos Asiakirjat-kansiota ei ole määritelty, käytetään käyttöjärjestelmän pysyvää sovellusdataa. Asetuksista vaihdettu oletusjuuri ja kilpailukohtainen Vaihda sijaintia -valinta eivät siirrä vanhoja kilpailuja. Viimeksi onnistuneesti avattu pysyvä kilpailu muistetaan seuraavaa käynnistystä varten. --demo käyttää erillisiä tilapäisiä koetietoja ja omia asetuksia; se ei muuta viimeksi avattua oikeaa kilpailua.

Asetukset-näkymän sovellusasetuksia voi käyttää myös ilman avointa kilpailua. Päivitysten tarkistus ja lataus tehdään käyttäjän pyynnöstä taustalla. Asennus odottaa keskeneräisten muokkausten hyväksymistä ja odottavien Emit-lukemien tallennusta ennen ohjelman vaihtoa. Kortinlukija pysäytetään hallitusti asennuksen ajaksi.

Projektin oma lisenssi on BSD-2-Clause. Alkuperäinen Slint-maininta säilytetään Tietoja-ikkunassa. Esitystasolla näytetään toimitsijan työhön liittyvät tiedot ja virheet; kehityksen ja laskennan tutkimushistoria kuuluu tekniseen dokumentaatioon.
