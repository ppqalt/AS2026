# AS2026:n kunnostuksen tarkastusraportti

Alkuosa kuvaa CLI-kunnostuksen valmistumishetkeä. Sen jälkeen toteutettu GUI-vaihe ja sen tarkistukset ovat tämän raportin lopussa. Ajantasaiset käyttöohjeet ovat README-, BUILDING- ja UI-tiedostoissa.

Työ tehtiin olemassa olevaan `dev`-työpuuhun. Lähtötila oli puhdas commit `5f869c4` (TOML-kilpailutuki), ei aiemmin mainittu `1bfc541`. Valmisteilla ollutta kunnostuspatchia ei käytetty. `main` ja `codex-original` säilyvät alkuperäisessä commitissa `944ee4f`.

Ennen muutoksia tehtiin Git-bundle ja lähdekoodiarkisto työpuiden viereiseen `as2026-palautus-20261015`-hakemistoon. Laitetestin oikeat lukemat ja lokit säilyvät sen alihakemistossa `fyysinen-laitetesti`; ne eivät kuulu Git-repositorioon.

## Muutetut Rust-lähdetiedostot

| Tiedosto | Muutoksen tarkoitus |
| --- | --- |
| `src/application.rs` | Tallennetun lukeman ja suoran kehyksen yhteinen laskentapolku. Vanhan radan nimiviite ja uuden radan numeroviite erotettu. TOML-kilpailun tulokset lasketaan nykyisestä mallista ja laskentahuomautukset säilyvät. |
| `src/changes.rs` (uusi) | Kilpailijan, luokan, radan/asemien ja asetusten lisäys tai korvaus; virheellinen muutos ei muuta muistissa olevaa mallia. |
| `src/emit.rs` | Ensimmäisen lukumerkin yhteinen tarkistus ja varsinainen leimaussisältö ilman 250-merkin vaihtuvaa lukuaikaa. |
| `src/legacy.rs` | Toistuva luokkatunnus hylätään, jotta rataviitettä ei päätellä tiedostojärjestyksestä. Muu vanhan tiedostomuodon tuonti säilyy. |
| `src/lib.rs` | AS2026-kirjaston nimi ja uusien yhteisten moduulien vienti. |
| `src/main.rs` | Suomenkielinen AS2026-CLI, jatkuva tallentava Emit-luku, hallittu keskeytys, `paivita` ja `tallenna-kehys`; TOML-tulosten ajantasainen muodostus. |
| `src/native.rs` | Serialisoitava yhteinen TOML-malli, version 1 yhteensopivuus, tyypitetty uuden kilpailun pohja, auto ja lukutapahtuman aikaleima, validointi ja lukuhistoria. |
| `src/reader.rs` | Pysyvä 9600 8N2 -portti, keskeneräisten ja useiden saapuneiden kehysten puskurointi sekä ehjän hyötydatan synkkatavuparin säilyttäminen. |
| `src/report.rs` | AS2026 ja suomalaiset otsikot; vanhojen tallennettujen tulosten ja uusien laskettujen tulosten oikea lähdekuvaus. |
| `src/storage.rs` (uusi) | Atominen paikallinen tallennus, `.bak`, käyttöjärjestelmälukko, sisältökonfliktit, polkualiaksien yhtenäistäminen ja käyttöoikeuksien säilyttäminen. |
| `src/terminal.rs` | AS2026-nimi ja toimivan suomenkielisen käyttövalikon säilyttäminen. |

Myös `matching.rs`, `scoring.rs`, `timing.rs` ja `ranking.rs` luettiin ja katselmoitiin. Niiden laskentasääntöjä ei muutettu. Vanhan ohjelman epäselviä sääntöhaaroja ei korvattu oletuksilla.

Testitiedostot: `application.rs`, `cli.rs`, `core_behavior.rs`, `native.rs`, `reader.rs`, `report.rs`, `storage.rs` ja `emit.rs` kattavat uusia korjauksia. `ranking.rs`, `timing.rs` ja `terminal.rs` päivitettiin myös pakettinimeen `as2026`. `tests/emit.rs` ja `tests/storage.rs` ovat uusia.

Muut tiedostot: Cargo-paketin ja ohjelmatiedoston nimi on `as2026`, versio 0.3.0 ja Rustin vähimmäisversio 1.89. Riippuvuudet ovat `serialport`, `serde`, `toml`, `chrono` (paikallinen lukukello) ja `ctrlc` (hallittu keskeytys). `.gitignore`-päällekkäisyys poistettiin ja paikalliset kilpailu-, lukko-, varmuuskopio- ja tilapäistiedostot ohitetaan synteettisiä fixtures-tiedostoja lukuun ottamatta. README ja BUILDING vastaavat toteutusta.

## Todennetut korjaukset

- Kertaluku ei aiemmin tallentanut kilpailuun. Jatkuva toiminto tallentaa hyväksytyn lukeman ennen onnistumisilmoitusta ja odottaa seuraavaa korttia samassa portissa.
- Hyötydatassa oleva synkkatavupari saattoi katkaista ehjän kehyksen. Skanneri ei enää hylkää sitä tästä syystä.
- Usean kehyksen yhteinen tavupuskuri ja aikakatkaisun yli jatkuva osittainen kehys käsitellään menettämättä seuraavaa korttia.
- Koodin 250 vaihtuva aika ei tarkoita uusia varsinaisia leimauksia. Historia ja yksi aktiivinen korttisisältö erotetaan; yhden kilpailijan tulos ei monistu.
- Tuntemattoman kortin lukema säilyy myöhempää ilmoittautumista varten. Kelvoton uusi lukema ei korvaa kelvollista sisältöä.
- Kilpailijan nimi, luokan rata ja radan asemat vaikuttavat seuraavaan laskentaan samasta ajantasaisesta mallista. Erillistä pysyvää tulosvälimuistia ei tarvita.
- Tallennuksen epäonnistuminen, vanhentunut ikkuna ja toinen paikallinen kirjoittaja tuottavat virheen. Vanha sisältö ja muistissa oleva onnistuneesti tallennettu tila säilyvät.
- Vanhan kilpailun numerolta näyttävä radan nimi ei enää valitse toista rataa sen indeksin perusteella. AS2015A:n luokkasarakkeen tekstivertailu radan nimeen varmistettiin olemassa olevasta staattisesta analyysistä (vertailukohta `0x41553e`). Korjausta suojaavat erilliset nimiviitteen ja numeroviitteen regressiotestit.
- Uuden kilpailun HTML-tulos ei väitä laskettuja pisteitä tuodun `Kilp.txt`:n pisteiksi. Laskennan erityishuomautukset näkyvät myös tulosluettelossa.

## Todella suoritetut tarkistukset

Paikallisesti CachyOS/Linuxissa, Rust 1.98.1:

| Tarkistus | Tulos |
| --- | --- |
| `cargo fmt --all --check` | Hyväksytty. |
| `cargo test --all-targets` | 138 testiä hyväksytty, ei epäonnistuneita. |
| `cargo clippy --all-targets -- -D warnings` | Hyväksytty ilman varoituksia. |
| `cargo build --release` | Hyväksytty; ohjelma `target/release/as2026`. |
| README:n TOML-esimerkit ja BUILDINGin tallennusesimerkki | Ajettu erillisessä tilapäiskansiossa. |
| Fyysinen Emit-kertaluku Linuxissa | Kortti ja viisi odotettua varsinaista leimaa tunnistettiin. |
| Fyysinen jatkuva Emit-luku Linuxissa | Automaattinen FTDI-tunnistus, sama avoin portti, kortin uudelleenasetus, vaihtuva lukumerkki ja tallennus onnistuivat. Yhdessä 55 sekunnin istunnossa tallentui 49 lisäkehystä. Yhteensä testikansiossa 51 lukemaa. |
| Kilpailun uudelleenavaus fyysisen luvun jälkeen | Historia avautui ja kilpailijalle muodostui yksi aktiivinen tulos. |
| Git-historian ja työpuun julkaisutarkastus | Ei havaittuja tokeneita, yksityisavaimia, oikeita kilpailija-aineistoja eikä suljetun ohjelman binäärejä. |

Fyysinen testi ei todista, että jokainen kehys olisi erillinen kortin asetus: laite lähettää kehyksiä myös kortin pysyessä paikallaan. Eri oikeiden korttien peräkkäistä lukua, muuttunutta todellista korttisisältöä tai Windowsin fyysistä lukijaa ei testattu. Näiden ohjelmallisia polkuja suojaavat simuloidut testit.

[GitHub Actions -tarkistusajo](https://github.com/ppqalt/AS2026/actions/runs/38806074766) commitille `ecf88a0` hyväksyttiin sekä `ubuntu-latest`- että `windows-latest`-ympäristössä. Molemmissa onnistuivat muotoilu, `cargo check`, testit, Clippy, release-käännös ja ohjelman sisäinen esimerkki. Paikallista Windows-käännöstä ei tehty. Viimeinen dokumentaatio- ja Git-ohitusmuutos ei muuttanut tarkistettua Rust-koodia; samojen tarkistusten uusinta jätettiin siinä pois.

## GitHub ja Git-tila

Repositorio: [ppqalt/AS2026](https://github.com/ppqalt/AS2026), näkyvyys **PRIVATE**, oletushaara **main**. GitHub CLI:n oikea käyttäjä tarkistettiin kirjautumisen uusimisen jälkeen. Ennestään samannimistä repositoriota ei ollut.

GitHubiin lähetettiin `main`, `dev` ja `codex-original`. Main ja tagi osoittavat edelleen alkuperäiseen commitiin `944ee4f`; niiden sisältöä ei muutettu. Dev sisältää kunnostuscommitin `c45911b`, dokumentaatiocommitin `ecf88a0` ja lopuksi etätarkistusten kirjauksen sekä paikallisten kilpailutiedostojen Git-ohitukset. Lähdekoodia ei lisensoitu uudelleen eikä repositoriota tehty julkiseksi.

## Keskeneräiset toiminnot ja laskennan vastaavuus

Täydellistä AS2015A-vastaavuutta ei väitetä. Saatavilla ei ollut kattavia oikeita kilpailuaineistoja, joille olisi alkuperäisohjelman vahvistamat tulokset. Nykyiset 645/600-pisteiset regressiot ovat synteettisiä. Fyysisen lukijan onnistuminen ei varmista pistekaavojen vastaavuutta.

Edelleen puuttuvat tai ovat rajattuja:

- Ajanottotapa 2, käsin syötettyjen asemakellojen yleinen uudelleenlaskenta sekä osa poikkeavista lähtö-/maalitilanteista.
- Erillisten kilpailijakohtaisten RT-lisäysten ja muiden lisävirhepisteiden yhdistäminen uuteen asemalaskentaan.
- Kaikkien vanhojen kilpailu-, luokka-, yhdistely-, nousu- ja sarjapisteasetusten määrittäminen uudessa TOML-muodossa.
- Vanha K-otsikko ja PE:tä edeltävä perustietomuoto; epäselviä tuonteja ei arvata.
- Ohjatut muokkausnäkymät, poistot, lähtöaikojen arvonta ja vanhan Emit-rivinmuodostuksen koko vientityönkulku.
- Slint-GUI ja usean koneen yhteiskäyttö. Verkkopalvelinta tai synkronointia ei rakennettu.

Alkuperäisten kahdeksan toiminnon kattavuustaulukko on README:ssä. Paikallinen TOML-tallennus ei ole usean koneen tietokanta. Ulkoinen editori ei noudata sovelluksen lukkoa. Uuden tiedoston atominen luonti tarvitsee kovat linkit sallivan tiedostojärjestelmän, joten FAT/exFAT ei sovellu tähän luontipolkuun.

Seuraava perusteltu työ on kerätä alkuperäisellä AS2015A:lla muodostettu anonymisoitu vertailukilpailu ja sen vahvistetut tulokset. Sen avulla voidaan testata koko laskentaketjua ja toteuttaa erilliset lisäpisteet muuttamatta sääntöjä oletuksilla. Tämän jälkeen ohjattu kilpailija- ja rataeditori voi käyttää nykyistä tallennusrajapintaa.

## GUI-vaihe 16.10.2026

Versio 0.4.0 lisää erillisen `as2026-gui`-ohjelman ja seitsemän suomenkielistä näkymää. Käyttäjän kuvassa näkyneet valkoiset pinnat muutettiin lähes mustiksi ja Slintin vakiokentille asetettiin sama tumma värimaailma. Tekstit, syötteet, valinnat ja painikkeet tarkastettiin oikeasta Linux-ikkunasta. Värit ja mitat ovat yhteisessä `ui/theme.slint`-tiedostossa, komponentit `ui/components.slint`-tiedostossa ja näkymät `ui/app.slint`-tiedostossa.

Tämän vaiheen Rust-muutokset:

| Tiedosto | Tarkoitus |
| --- | --- |
| `build.rs` (uusi) | Valinnaisen Slint-käyttöliittymän ja sen suomennosten käännös. |
| `src/bin/as2026-gui.rs` (uusi) | Graafisen ohjelman käynnistys ja suomenkielinen virheilmoitus. |
| `src/gui/mod.rs` (uusi) | Ikkunan tapahtumat, luonnosten hyväksyminen, tallennuksen tilailmoitukset, jatkuvan lukijan käsittely, tallentamattomien kehysten uudelleenyritys ja raportin vienti. |
| `src/gui/forms.rs` (uusi) | Lomakkeet, nimetyt valinnat ja kenttäkohtaiset muutokset yhteiseen kilpailuun. ULA-valinnat seuraavat asematyyppiä. |
| `src/gui/tables.rs` (uusi) | Haettavat taulukot, lukuhistoria, tulosjärjestys ja asemakohtaiset laskentatiedot. Ilmoittautuneelle ilman tulosta ei näytetä nollapistettä valmiina tuloksena. |
| `src/gui/smoke.rs` (uusi) | Oikean GUI-sovittimen työnkulkukoe ja ikkunan näppäintapahtumakoe synteettisellä kopiolla. |
| `src/workspace.rs` (uusi) | Kilpailun avaus ja kenttämuutokset ProjectStore-rajapinnalla. Luokka- ja ratatunnisteen muutos säilyttää viittaukset. Väliasema lisätään ennen maalia. |
| `src/live_reader.rs` (uusi) | Yksi jatkuvasti avoin lukija taustasäikeessä, rajattu tapahtumajono ja jo luettujen kehysten hallittu tyhjennys pysäytyksessä. |
| `src/reader.rs` | Puskuroidun kokonaisen kehyksen nouto ilman uutta laiteodotusta. |
| `src/legacy.rs` | Arkiston Clone-tuki. Vanhaa tiedostomuotoa ja laskentasääntöjä ei muutettu. |
| `src/lib.rs` | Uusien yhteisten moduulien vienti. |

Lisäksi `tests/workspace.rs` sisältää 14 tallennuksen, viittausten ja asemamuutosten testiä ja `tests/live_reader.rs` kuusi taustalukijan testiä. GUI:n taulukoita suojaa kolme testiä. Cargo-riippuvuudet ovat GUI:n osalta valinnaisia; CLI pysyy käännettävänä erikseen. TOML-muodon versio säilyy ennallaan.

Paikallisesti hyväksyttiin muotoilu, 158 testiä ilman GUI-ominaisuutta, 161 testiä GUI-ominaisuudella, Clippy GUI-ominaisuudella ilman varoituksia sekä CLI:n ja GUI:n release-käännös. Release-ohjelman työnkulkukoe tarkisti automaattitallennuksen, keskeneräisen syötteen, viittausten muutokset, aseman vaikutuksen laskentaan, ULA-valinnat ja tallennusvirheen jälkeisen Emit-uudelleenyrityksen. Näppäimistökokeessa toimivat Ctrl+N, tekstin syöttäminen, Tab ja Esc. Seitsemästä näkymästä sekä uuden kilpailun ikkunasta otettiin kuvat.

Fyysistä Emit-lukijaa ei avattu GUI-vaiheessa. Windowsin visuaalinen käyttö ja fyysinen lukija ovat edelleen erikseen testattavia. Laskentaytimen tunnetut rajat ja AS2015A-vastaavuuden puuttuva kattava varmennus säilyvät. Poistoja, lähtöaikojen arvontaa tai monen koneen järjestelmää ei toteutettu. Ohjatut kilpailija-, luokka-, rata- ja asemaeditorit ovat nyt käytettävissä.

Seuraava kehitysaskel on kokeilla kilpailun perustamisen ja tulostuksen kokonainen työnkulku järjestäjän kanssa sekä kerätä alkuperäisellä AS2015A:lla vahvistettu vertailukilpailu laskentaytimen varmennukseen.

## Lisenssivalinta

Käyttäjän pyynnöstä AS2026:n oma koodi, dokumentaatio ja synteettiset esimerkit lisensoitiin 0BSD-lisenssillä. Virallinen teksti on LICENSE-tiedostossa ja Cargo-paketin SPDX-tunnus on `0BSD`. README:n aiempi avoin lisenssivalinta päivitettiin. Ulkopuoliset kirjastot säilyttävät omat ehtonsa; Slintin Royalty-free 2.0 -vaihtoehto ja Tietoja-ikkunan alkuperäinen AboutSlint-maininta dokumentoidaan THIRD_PARTY-tiedostossa. Repositorio säilyy yksityisenä, main-haaran historiaa ei muuteta.
