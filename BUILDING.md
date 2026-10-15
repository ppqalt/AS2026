# Kääntäminen Linuxille ja Windowsille

Laskentakoodi käyttää Rustin vakiokirjastoa. Sarjaporttiluvussa käytetään yhtä suoraa Cargo-riippuvuutta (`serialport`, ilman oletusominaisuuksia).
Käyttäjän ei tarvitse asentaa Pythonia, Javaa, Ghidraa, selainta,
tietokantapalvelinta tai käyttöliittymäkirjastoa. Valmis ohjelma toimii ilman
lähdekoodihakemistoa. Esimerkkikilpailun tiedot ovat ohjelman sisällä, ja omat
kilpailutiedostot luetaan käyttäjän valitsemasta kansiosta.

## Käyttäjälle

Valitse omalle käyttöjärjestelmällesi käännetty ohjelma:

- Linux: `as2015-rs`
- Windows: `as2015-rs.exe`

Käynnistä ohjelma päätteessä ilman lisävalintoja. Suomenkielinen valikko opastaa
eteenpäin. Windowsissa voit myös avata `as2015-rs.exe`-tiedoston, jolloin valikko
avautuu pääteikkunaan. Liitä kilpailukansion polku pyydettäessä. Polussa saa olla
välilyöntejä ja ääkkösiä, ja polun ympärillä saa olla lainausmerkit. Kansion voi
vetää päätteeseen, jos käyttämäsi pääte osaa liittää sen polun. Alkuperäisten
AS2015-tiedostojen merkistökysymykseen voi vastata oletuksen mukaisesti ei.
Valitse UTF-8 vain, jos tiedät tiedostojen olevan siinä muodossa.

Valinta 0 sulkee ohjelman. Tuloksen jälkeen voit valita uuden toiminnon.
Komentorivitoiminnot ovat edelleen käytettävissä esimerkiksi toistuvien tehtävien
ajamiseen. Ne on esitelty tiedostossa `README.md`.

Rustia ja käännöstyökaluja tarvitaan vain ohjelman kääntämiseen. Tavallinen
Linux-versio käyttää käyttöjärjestelmän C-ajonaikaiskirjastoa, joten se ei ole
kokonaan staattinen ohjelma. Käännä ohjelma vanhimmalla tukemallasi
Linux-jakelulla tai varmista toiminta erikseen jokaisella kohdejakelulla ennen
jakelua.

Windowsin MSVC-käännöksissä `.cargo/config.toml` ottaa käyttöön Rustin
`+crt-static`-valinnan. Sen tarkoitus on sisällyttää C-ajonaikaiskirjasto
ohjelmatiedostoon, jotta käyttäjä ei tarvitse erillistä Visual C++
Redistributable -asennusta tätä sovellusta varten. Windowsin omia
järjestelmäkirjastoja käytetään silti. Menetelmä on kuvattu
[Rustin linkitysohjeessa](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes).
Tarkista valmiin ohjelman tarvitsemat DLL-kirjastot ja kokeile ohjelmaa
tukemillasi Windows-versioilla. Windows-käännöstä ei ole vielä tehty tällä
kehityskoneella.

## Kääntäminen Linuxilla

Tarvitset vakaan Rust-työkaluketjun ja käyttöjärjestelmän C-linkittäjän.
Sarjaporttikirjasto rakennetaan ilman Linuxin libudev-kehityspakettia. Ensimmäinen käännös tarvitsee verkkoyhteyden Cargo-riippuvuuksien lataamiseen.
Avaa pääte hakemistossa, jossa `Cargo.toml` sijaitsee, ja suorita:

```sh
cargo build --release
./target/release/as2015-rs
```

Valmis ohjelma on `target/release/as2015-rs`. Jos ympäristömuuttuja
`CARGO_TARGET_DIR` on asetettu, tuloshakemisto määräytyy sen mukaan.

## Kääntäminen Windowsilla

Tarvitset vakaan Rust-työkaluketjun ja käännöskohteen
`x86_64-pc-windows-msvc`. MSVC-kääntäminen tarvitsee myös Microsoftin C++
Build Tools -työkalut ja Windows SDK:n, mukaan lukien C++-kääntäjän ja
linkittäjän. Nämä ovat kehittäjän työkaluja. Valmiin ohjelman käyttäjä ei
tarvitse Rustia eikä käännöstyökaluja. Rustin asennusohjelma neuvoo tarvittavien
työkalujen hankinnassa:
[Rustin virallinen asennusohje](https://doc.rust-lang.org/book/ch01-01-installation.html).

Avaa PowerShell hakemistossa, jossa `Cargo.toml` sijaitsee, ja suorita:

```powershell
cargo build --release
.\target\release\as2015-rs.exe
```

Valmis ohjelma on `target\release\as2015-rs.exe`. Suorita komennot projektin
hakemistosta, jotta Cargo lukee `.cargo/config.toml`-tiedoston
ajonaikaiskirjastoasetuksen. Ohjelma käännetään Windowsissa Windowsille.
Linuxissa käännetty ohjelmatiedosto ei toimi Windows-ohjelmana.

## Tarkistukset ja automaattinen Windows-käännös

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Tiedosto `.github/workflows/ci.yml` määrittelee nämä tarkistukset,
julkaisuversion kääntämisen ja ohjelman sisäisen esimerkkiajon sekä Linuxille
että Windowsille. GitHubin suoritusympäristöt ovat `ubuntu-latest` ja
`windows-latest`. Työnkulku toimii, kun tätä projektihakemistoa käytetään
GitHub-tietovaraston juurihakemistona. Lähdekoodi noudetaan
[GitHubin virallisella toiminnolla](https://github.com/actions/checkout), ja
Rust asennetaan suoritusympäristön Rust-asentimella.

Työnkulun määrittely on valmisteltu paikallisesti. Tämä ei tarkoita, että
GitHubin tarkistukset olisi jo ajettu. Näitä muutoksia ei ole lähetetty
GitHubiin eikä etäajoa ole käynnistetty.

`--offline` koskee Cargoa: Rust-työkaluketjun asentamisen jälkeen projekti ei
nouda Cargo-paketteja verkosta. Työkaluketjun asentaminen tai päivittäminen sekä
GitHubin etäajot tarvitsevat verkkoyhteyden.

Testeissä käytetään keinotekoista aineistoa. Niissä ei käynnistetä
AS2015A-ohjelmaa eikä käytetä sarjaporttilaitteita. Käyttövalikkoa testataan
muistissa olevalla syötteellä ja tulosteella. Syötteen loppuminen kokeillaan
jokaisen kysymyksen kohdalla, joten testit eivät jää odottamaan näppäimistöä.
Näiden testien läpäisy ei tarkoita vertailua oikeisiin vanhoihin
kilpailutuloksiin.

## Kehityskoneella varmistettu

Tarkistetulla kehityskoneella ovat Rust ja Cargo 1.98.1. Asennettuna on vain
koneen oma käännöskohde `x86_64-unknown-linux-gnu`. MinGW-ristikääntäjää ei
löytynyt. Windows-kääntämistä tai ohjelman toimintaa Windowsissa **ei ole
varmistettu paikallisesti**. Windows-työnkulku on ajettava ennen kuin
Windows-käännöstä voi sanoa testatuksi. Näiden muutosten yhteydessä ei ole
asennettu järjestelmäpaketteja, Rustin käännöskohteita tai työkaluketjuja.


TOML-kilpailujen käsittelyyn käytetään `serde`- ja `toml`-Rust-kirjastoja. Ne linkitetään valmiiseen ohjelmaan, eivätkä vaadi erillisiä käyttäjän asentamia sovelluksia.
