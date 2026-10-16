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

## Nykyinen lisenssi: BSD-2-Clause

Uusimman dev-version oma koodi, dokumentaatio ja synteettiset esimerkit käyttävät BSD-2-Clause-lisenssiä. LICENSE sisältää standardin koko tekstin ja nykyisen tekijänoikeusilmoituksen: `Copyright (c) 2026 AS2026 contributors`. Henkilötietoja ei ole lisätty. Cargo-paketin SPDX-tunnus on `BSD-2-Clause`. Riippuvuuksien lisenssit ja Slintin AboutSlint-maininta säilyvät erillisinä.

Aiempi commit `50baf9d` käytti 0BSD-lisenssiä. Siihen jo myönnettyjä oikeuksia ei peruuteta eikä sen Git-historiaa muuteta. Nykyinen lisenssivalinta koskee uutta dev-versiota ja siitä tehtäviä tulevia julkaisuja.

GitHub-repositorio säilyy yksityisenä. Tarkistuksessa Releases-rajapinta palautti nolla julkaisua (mukaan lukien luonnokset ja esijulkaisut), eikä Actions-latauspaketteja ollut. Release-julkaisuja tai tageja ei siksi poistettu. `codex-original` säilyy commitissa `944ee4f`. Uutta GitHub Releaseä ei tehty.

## Asennus, kilpailukansiot ja lisenssitarkistukset 16.10.2026

Versio 0.5.0 lisää pysyvän käyttäjäkohtaisen asennuksen ja kilpailujen oletuskansion. Uusi kilpailu perustetaan nimellä; ohjelma luo sille yksilöllisen kansion Asiakirjat-kansion `AS2026/Kilpailut`-hakemistoon tai käyttöjärjestelmän pysyvään sovellusdatakansioon. Sijainnin voi vaihtaa. Viimeksi avattu kilpailu muistetaan, ja ohjelmatiedostot, asetukset sekä kilpailut säilyvät erillään. Esimerkkitila käyttää erillistä väliaikaista kopiota ja erillisiä asetuksia.

Tämän vaiheen Rust-muutokset:

| Tiedosto | Tarkoitus |
| --- | --- |
| `src/preferences.rs` | Käyttöjärjestelmän kansiot, turvalliset kilpailukansioiden nimet, asetusten atominen tallennus, lukitus ja viimeksi avattu kilpailu. |
| `src/updates.rs` | Manuaalinen HTTPS-päivitystarkistus, Ed25519-allekirjoitus, tiedostojen tiivisteet, BSD-2-Clause-metatieto ja rajattu turvallinen arkiston purku. |
| `src/installation.rs` | Versionoitu asennus, pysyvä käynnistin, paikallinen asennuslukko, käynnistyskuittaus ja epäonnistuneen päivityksen palautus. Linuxin sovellusvalikkotiedosto ja Windowsin Käynnistä-valikon pikakuvake. |
| `src/bin/as2026-launcher.rs` | Asennus- ja käynnistystoimintojen erillinen ohjelma; Windowsissa ei konsoli-ikkunaa. |
| `src/bin/as2026-gui.rs` | Windowsin graafisen sovelluksen käynnistys ilman konsoli-ikkunaa. |
| `src/gui/desktop.rs` | Päivitysten taustatyö, asennuksen hallittu käynnistys sekä tallennusten tarkistus ennen sovelluksen sulkemista. |
| `src/gui/mod.rs` | Oletuskansio, nimellä perustaminen, muistettu kilpailu, asennuksen käynnistyskuittaus ja uusien toimintojen yhdistäminen käyttöliittymään. |
| `src/gui/smoke.rs` | Nimellä perustamisen, pysyvän tallennuksen, muistamisen ja näppäimistön toimintakokeet. |
| `src/gui/tables.rs` | Käyttäjälle olennaiset laskentahuomautukset yhteisen esityspolun kautta. |
| `src/report.rs` | Yleiset tutkimus- ja kehitystekstit pois käyttäjän raporteista; toimintaan vaikuttavat tiedot säilyvät. |
| `src/main.rs` | CLI:n yleisten kehitys- ja tutkimushuomautusten siivous samalla esityspolulla. |
| `src/lib.rs` | Valinnaisten työpöytämoduulien vienti. |

Testitiedostot `tests/preferences.rs`, `tests/installation.rs`, `tests/report.rs` ja `tests/cli.rs` kattavat kansiot, asetusten konfliktit, pikakuvakkeiden muodostamisen sekä raporttien käyttäjätekstit. Asennuksen ja päivittäjän sisäiset testit kattavat lisäksi virheellisen allekirjoituksen, muuttuneet tiedostot, vaaralliset arkistot, kesken jääneen asennuksen ja palautuksen. Laskentaytimen sääntöjä ei muutettu.

Lisenssimuutos on omassa commitissaan `79d0a35`. Se muutti tiedostoja `LICENSE`, `Cargo.toml`, `README.md`, `THIRD_PARTY.md`, `ui/app.slint` ja tätä raporttia. LICENSE käyttää [SPDX:n BSD-2-Clause-vakiotekstiä](https://spdx.org/licenses/BSD-2-Clause.html) ja aiempaa yhteistä tekijämainintaa. Tulevien pakettien lisenssivarmistukset ovat tiedostoissa `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `scripts/package_release.py`, `scripts/sign_release.py`, `scripts/tests/test_release.py` ja `src/updates.rs`.

Muut tämän vaiheen tiedostot ovat `Cargo.lock`, `BUILDING.md`, `CLI.md`, `UI.md`, `RELEASES.md`, `RELEASE_NOTES.md`, `assets/as2026.svg`, `assets/as2026.ico`, `release-public-key.hex`, `scripts/.gitignore` ja `scripts/license-overrides/`. Viimeksi mainittu hakemisto sisältää lähdecommitilla kiinnitettyjä kolmannen osapuolen lisenssitekstejä ja niiden lähdeluettelon; niiden lisenssejä ei muutettu BSD-2-Clauseksi. Julkinen allekirjoitusavain kuuluu lähdekoodiin, yksityinen avain säilyy erillään.

Paikallisesti hyväksytyt tarkistukset:

- `cargo fmt --all --check`.
- `cargo test --all-targets`: 160 testiä, ei epäonnistuneita.
- `cargo test --all-targets --features gui`: 203 testiä, ei epäonnistuneita.
- Clippy ilman GUI:ta ja GUI-ominaisuudella: ei varoituksia.
- Release-käännös ilman GUI:ta ja GUI-ominaisuudella: hyväksytty.
- `python3 scripts/package_release.py check-license`: Cargo ja koko BSD-2-Clause-teksti hyväksytty.
- Paketoinnin ja allekirjoituksen 25 Python-testiä: hyväksytty. Vanha oma 0BSD-lisenssi, vajaa lisenssiteksti ja vanha metadata hylätään; kolmannen osapuolen 0BSD-lisenssi säilyy.
- Release-GUI:n työnkulku- ja näppäimistökokeet: hyväksytty. Seitsemän näkymän, uuden kilpailun ja Tietoja-ikkunan kuvat tarkastettiin; Tietoja näyttää BSD-2-Clausen ja Slint-maininnan.
- Todellinen Linuxin hyötytiedostopaketti: 988 tiedostoa, mukaan lukien oma lisenssi ja riippuvuuksien lisenssit. Paikallinen paketti allekirjoitettiin, asennettiin ja sen käynnistyskuittaus onnistui. Asennetun LICENSE vastasi projektin LICENSEä.
- `desktop-file-validate` hyväksyi pikakuvakkeen, ja `gtk-launch` avasi ohjelman pysyvästä `versions/0.5.0`-asennuskansiosta.

GitHubin Releases-, Actions-artifacts- ja Packages-lukumäärät olivat alkutarkistuksessa nolla. Vanhoja julkaisuja ei siis ollut poistettavaksi. Uusi CI tuottaa ainoastaan yksityisiä BSD-2-Clause-testipaketteja. Release-työnkulku edellyttää manuaalista käynnistystä ja erillistä julkaisuvaiheen valintaa; se tarkistaa lisenssin ennen rakentamista ja uudelleen ennen allekirjoittamista. Uutta Releaseä tai tagia ei tehty.

GitHubin yleinen lisenssimetadata pysyy toistaiseksi tyhjänä, koska oletushaara `main` on alkuperäinen `944ee4f` ilman LICENSE-tiedostoa. Dev-haarassa BSD-2-Clause on käytössä. Lisenssin automaattinen tunnistus päivittyy, kun hyväksytty uusi versio myöhemmin viedään main-haaraan. Mainin ja `codex-original`-tagin sisältöä tai aiempaa Git-historiaa ei muutettu.

Fyysisen Windows-koneen asennusta tai päivitystä ei testattu. GitHubista tehtävää oikean julkaistun päivityksen latausta ei voitu kokeilla, koska repositorio on yksityinen eikä Releaseä julkaista tässä vaiheessa. Latauksen, allekirjoituksen ja purun polut on testattu korvattavalla HTTP-kuljetusrajapinnalla; tämä ei korvaa tulevan oikean julkaisun latauskoetta. Fyysiseen Emit-lukijaan ei koskettu tässä vaiheessa.

## Emit-näkymän viimeistely, versio 0.5.1

Muiden muutosten jälkeen Emit-näkymän suuri viimeisen kortin paneeli korvattiin pienellä kolmen sekunnin onnistumiskuittauksella. Lukuhistoria ryhmittelee vain peräkkäiset saman kortin samat varsinaiset leimat. Lukukertojen määrä näkyy Lukuja-sarakkeessa, ryhmän rivivalinta säilyy ja yksityiskohdat näyttävät viimeisimmän lukeman. Eri kortit, muuttuneet leimat ja virheelliset tapahtumat säilyvät erillisinä. Kaikki alkuperäiset lukemat säilytetään edelleen tallennuksessa.

Tämä viimeistely muutti tiedostoja `src/gui/mod.rs`, `src/gui/tables.rs`, `src/gui/smoke.rs` ja `ui/app.slint`. Versionumero päivitettiin Cargo-tiedostoihin ja käyttö dokumentoitiin UI- ja julkaisuteksteihin. Laskentaytimen, sarjaliikenteen tai kilpailutiedoston muotoa ei muutettu.

Version 0.5.1 koko GUI-testisarja sisältää 205 hyväksyttyä testiä; lisätestit suojaavat eri korttien erottelua, myöhempää kilpailijaan yhdistämistä sekä sadan uusintakehyksen ryhmittelyä. Release-GUI:n työnkulku-, näppäimistö- ja oikea 3,2 sekunnin ajastinkoe hyväksyttiin. Kuvasta tarkastettiin myös pienen lukukuittauksen sijainti. Fyysistä lukijaa ei tarvittu näihin kokeisiin.

Paikallinen allekirjoitettu päivitys 0.5.0-versiosta 0.5.1-versioon ja edellisen 0.5.0-version palautus kokeiltiin oikeilla ohjelmilla. Käynnistyskuittaus ja version osoittimen palautus onnistuivat. Julkaisemattoman väliversion asennus säilytettiin palautuskansiossa, kun kuvantarkastuksessa viimeisteltiin kuittauksen välistystä. GitHub Releaseä tai tagia ei tehty tässäkään kokeessa.
