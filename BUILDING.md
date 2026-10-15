# AS2026:n kääntäminen ja tarkistaminen

AS2026:n ohjelmatiedosto on Linuxissa `as2026` ja Windowsissa `as2026.exe`. Laskenta, kilpailumalli ja komentorivitoiminnot ovat yhteistä Rust-koodia. Graafista käyttöliittymää tai palvelinta ei tarvitse asentaa.

## Tarvittavat työkalut

Kääntäminen tarvitsee Rustin version **1.89 tai uudemman**, C-linkittäjän ja kohdealustan kehitystyökalut. Ensimmäinen käännös tarvitsee verkkoyhteyden Cargo-riippuvuuksien lataamiseen. `Cargo.lock` pidetään versionhallinnassa sovelluksen riippuvuuksien toistettavuutta varten.

Riippuvuuksien tehtävät ovat rajattuja: `serialport` avaa lukijan Linuxissa ja Windowsissa, `serde` ja `toml` käsittelevät kilpailutiedostot, `chrono` antaa paikallisen lukukellon ja `ctrlc` lopettaa jatkuvan lukemisen hallitusti. Sarjaporttikirjaston Linuxin oletusominaisuudet on poistettu käytöstä, joten libudev-kehityspakettia ei tarvita.

Valmiin ohjelman käyttäjä ei tarvitse Rustia, Pythonia, Javaa, Ghidraa tai tietokantapalvelinta. HTML-raportin katseluun ja tulostamiseen voi käyttää selainta. Omat kilpailukansiot säilytetään erillään ohjelmatiedostosta.

## Linux

Avaa pääte projektihakemistossa, jossa `Cargo.toml` sijaitsee:

```sh
cargo build --release
./target/release/as2026
./target/release/as2026 esimerkki
```

Kehitysversion voi käynnistää myös suoraan:

```sh
cargo run
cargo run -- tarkista fixtures/native-demo
cargo run -- laske fixtures/native-demo --vaiheet
```

Valmis ohjelma on `target/release/as2026`, ellei `CARGO_TARGET_DIR` ohjaa tulosta muualle. Tavallinen GNU/Linux-käännös käyttää käyttöjärjestelmän C-ajonaikaiskirjastoa. Kääntäminen CachyOS:ssa ei yksin takaa yhteensopivuutta kaikkien vanhempien Linux-jakelujen kanssa; jakeluun tarkoitettu ohjelma pitää kokeilla kohdeympäristöissä.

## Windows

Tarvitset Windowsissa Rustin ja käännöskohteen `x86_64-pc-windows-msvc` sekä Microsoftin C++ Build Tools -työkalut ja Windows SDK:n. Avaa PowerShell projektihakemistossa:

```powershell
cargo build --release
.\target\release\as2026.exe
.\target\release\as2026.exe esimerkki
```

Valmis tiedosto on `target\release\as2026.exe`. Suorita käännös projektihakemistosta, jotta Cargo lukee `.cargo/config.toml`-asetukset. MSVC-kohteelle on asetettu `+crt-static`, jolla C-ajonaikaiskirjasto pyritään sisällyttämään ohjelmatiedostoon. Windowsin omia järjestelmäkirjastoja käytetään silti. Tarkista valmiin ohjelman DLL-riippuvuudet ja toiminta tuetuilla Windows-versioilla.

Linuxissa tuotettu GNU/Linux-ohjelma ei ole Windows-ohjelma. Windows-käännös, automaattiset testit, Clippy ja ohjelman esimerkkiajo on varmennettu GitHubin Windows-ympäristössä. Fyysistä Emit-lukua Windowsissa ei ole testattu.

## Tarkistuskomennot

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Muotoilun voi korjata komennolla `cargo fmt --all`. Tarvittaessa `cargo check --all-targets` tarkistaa kääntymisen ilman ohjelmatiedoston linkittämistä. Kun tarvittavat riippuvuudet ovat jo välimuistissa, Cargon komentoihin voi lisätä `--offline`; työkaluketjun asentaminen ja puuttuvien pakettien lataaminen vaativat edelleen verkon.

Tiedosto `.github/workflows/ci.yml` määrittelee muotoilu-, käännös-, testi- ja Clippy-tarkistukset, julkaisuversion käännöksen sekä ohjelman sisäisen esimerkin ajon `ubuntu-latest`- ja `windows-latest`-ympäristöissä. Työnkulku käyttää vakaata Rustia. [Tarkistusajo](https://github.com/ppqalt/AS2026/actions/runs/38806074766) commitille `ecf88a0` valmistui hyväksytysti sekä Linuxissa että Windowsissa: muotoilu, kääntymistarkistus, testit, Clippy, release-käännös ja esimerkkiajo onnistuivat. Paikallisen kehityskoneen Rust oli 1.98.1; CI käytti sen hetkistä vakaata Rustia. Viimeinen dokumentaatio- ja Git-ohitusmuutos ei muuttanut tätä testattua Rust-koodia eikä käynnistä samoja tarkistuksia uudelleen. Projektin on oltava repositorion juuressa, jotta suhteelliset polut toimivat.

## Tallennuspolun kokeilu ilman fyysistä laitetta

Luo kokeita varten erillinen kilpailukansio ja käytä synteettistä kehystä:

```sh
cargo run -- uusi kokeilu "Tallennuskokeilu"
cargo run -- tallenna-kehys kokeilu fixtures/synthetic/emit-frame.raw 12:11:00
cargo run -- tarkista kokeilu
```

Kehyksen kortti on synteettinen `123456`. Se tallentuu myös ilman ilmoittautunutta kilpailijaa. Kilpailijan voi liittää myöhemmin `paivita`-komennolla, kun luokka ja rata on määritetty. Komento käyttää samaa tallennusrajapintaa kuin jatkuva sarjaporttiluku. Se ei avaa fyysistä porttia eikä todista lukijan toimintaa.

Kilpailutietojen muutoksia käsitellään näin:

```sh
cargo run -- paivita kokeilu muutokset.toml
```

Muutostiedoston täydelliset tietueet korvaavat saman tunnisteen kilpailijan, luokan tai radan. Radan mukana korvataan sen kaikki asemat. `[kilpailu]` ja `[virhepisteet]` korvaavat mukana ollessaan koko vastaavan osion. Poistotoimintoa ei ole. Käyttöesimerkit ja kenttien merkitys ovat tiedostossa [README.md](README.md).

## Fyysisen Emit-lukijan kokeilu

Käytä kokeiluun omaa TOML-kilpailua, jotta oikeaa kilpailuaineistoa ei muuteta:

```sh
cargo run -- lue kokeilu /dev/ttyUSB0
```

Windowsissa vastaava portti voi olla `COM3`. Ilman porttiargumenttia ohjelma yrittää automaattista tunnistusta. Asetukset ovat 9600 8N2 ilman vuonohjausta. Ohjelma säilyttää portin auki ja tallentaa hyväksytyt lukutapahtumat automaattisesti. Ctrl+C lopettaa luvun. Komento `cargo run -- lue /dev/ttyUSB0` on erillinen kertaluvun laitetarkistus eikä tallenna kilpailuun.

Linuxin kehityskoneella on tässä työvaiheessa varmennettu oikean kortin 538747 kertaluku sekä jatkuva luku samalla auki pysyvällä portilla. Kortti nostettiin ja asetettiin uudelleen, hyväksyttyjä kehyksiä tallentui useita ja kilpailun uudelleenavaus onnistui. Leimaukset 47/145, 48/207, 49/238, 50/267 ja 100/396 säilyivät samoina lukumerkin ajan muuttuessa; kilpailijalle muodostui yksi aktiivinen tulos. Lukija lähettää uusintakehyksiä myös kortin ollessa paikallaan, joten jokainen historiarivi kertoo hyväksytystä lukemasta, ei varmennetusta erillisestä fyysisestä nostotapahtumasta.

Eri fyysiset kortit peräkkäin, muuttunut varsinainen leimaussisältö ja Windows-lukija tarvitsevat vielä omat laitekokeensa. Simuloidut testit eivät korvaa niitä. Kehyksen tunnettu rakenne ei sisällä varmennettua tarkistussummaa, joten kaikkea vioittumista ei voida tunnistaa.

Linuxissa portin avaaminen vaatii käyttöoikeuden valittuun laitetiedostoon. Tarkista tarvittaessa laitteen ryhmä ja jakelun sarjaporttikäytäntö. Windowsissa portin pitää olla vapaa muiden ohjelmien käytöstä, kuten Linuxissakin.

## Testien rajat ja tiedostojen säilyttäminen

Automaattiset testit käyttävät synteettisiä kilpailijoita, kilpailuja ja tavujonoja. Ne eivät käynnistä AS2015A-ohjelmaa eivätkä käytä fyysistä sarjaporttia. Testit suojaavat palautetun laskentamallin regressioita, mutta täysi vastaavuus alkuperäisiin kilpailutuloksiin on varmistettava vertailuaineistolla erikseen.

Pidä `kilpailu.toml`, lukuhistoria ja varmuuskopiot poissa lähdekoodirepositoriosta, jos niissä on oikeita kilpailijoiden tietoja. Lähdekoodin `fixtures`-aineisto on synteettistä. Ohjelman paikallinen `.lukko`-lukitus ja sisältövertailu eivät ole usean tietokoneen synkronointia. Ulkoiset editorimuutokset ja `.bak`-palautus tehdään sovelluksen ollessa suljettuna; tarkemmat palautusohjeet ovat README-tiedostossa.

Uuden kilpailun luonti julkaisee valmiin tiedoston kovalla linkillä, jotta olemassa olevaa kilpailua ei voida korvata kilpailevalla luontiyrityksellä. Valitse kovat linkit salliva paikallinen tiedostojärjestelmä. FAT- ja exFAT-taltiot eivät tue tätä luontipolkua; niitä ei pidä käyttää suoraan kilpailun tallennuskansiona tässä toteutuksessa.
