# AS2026:n kääntäminen ja testaus

Valmiin [julkaisupaketin](https://github.com/ppqalt/AS2026/releases/latest)
käyttäjä ei tarvitse käännöstyökaluja. Kehitys tarvitsee vakaan Rustin
(Cargo.toml ilmoittaa vähimmäisversion), C/C++-linkittäjän ja alustan kehityskirjastot.
Käytä ajantasaista stable-työkaluketjua ja versionhallittua Cargo.lock-tiedostoa.
Suorita tämän ohjeen komennot repositorion juuressa, jossa Cargo.toml sijaitsee.

## Linux

Ubuntu-pohjaisessa kehitys- ja CI-ympäristössä:

```sh
sudo apt-get install pkg-config libssl-dev libfontconfig1-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev libegl1-mesa-dev xvfb
cargo build --locked --release --features gui
./target/release/as2026-gui
```

CLI ilman GUI:ta: `cargo build --locked --release --bin as2026`.
GUI tarvitsee Wayland- tai X11-istunnon sekä fontti- ja ikkunakirjastot.
Tiedostonvalintaan tarvitaan työpöydän XDG-portaali toteutuksineen.
Sarjaportin käyttöoikeus määräytyy jakelun laitetiedoston oikeuksista;
libudev-kehityspakettia ei tarvita valitulla serialport-sovituksella.

Tarvittaessa ohjelmistorenderöinti:

```sh
SLINT_BACKEND=winit-software ./target/release/as2026-gui
```

Julkaisu rakennetaan Ubuntu 22.04:llä glibc-yhteensopivuuden rajaamiseksi.
Ohjelma ei ole täysin staattinen Linux-binääri.

## Windows

Asenna Rustin MSVC-työkaluketju, Microsoft C++ Build Tools ja Windows SDK.
Avaa PowerShell repositorion juuressa:

```powershell
cargo build --locked --release --features gui
.\target\release\as2026-gui.exe
```

`.cargo/config.toml` sisällyttää MSVC-ajonaikaiskirjaston ohjelmaan;
Windowsin järjestelmäkirjastoja käytetään silti. Ohjelmistorenderöinti tarvittaessa:

```powershell
$env:SLINT_BACKEND = 'winit-software'
.\target\release\as2026-gui.exe
```

## Rust-, Slint- ja paketointitarkistukset

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets --features gui
cargo clippy --locked --all-targets --features gui -- -D warnings
cargo build --locked --features gui --bin as2026-gui
cargo test --locked --manifest-path tools/release/Cargo.toml
```

`CARGO_TARGET_DIR` voi ohjata yhteiseen välimuistiin. Paikallisessa AS2026-
työkansiossa käytetään olemassa olevaa `sovellus/target`-kansiota; älä luo
vanhojen työpuunimien perusteella uusia suuria välimuisteja.

## Oikean GUI:n automaattikokeet

Demovalinta luo oman synteettisen kilpailun ja asetukset. Käytä joka ajossa
uutta kuvakansiota: tiedostovienti ei korvaa olemassa olevia tiedostoja.

```sh
cargo run --locked --features gui --bin as2026-gui -- --demo --smoke-test --capture-dir /tmp/as2026-gui-koe
AS2026_MENU_GUI_TEST=1 cargo run --locked --features gui --bin as2026-gui -- --demo --capture-dir /tmp/as2026-valikkokoe
AS2026_EMIT_CORRECTION_TEST=1 cargo run --locked --features gui --bin as2026-gui -- --demo --capture-dir /tmp/as2026-emit-koe
```

Linuxissa Xvfb mahdollistaa kokeet käyttäjän työpöydästä erillään:

```sh
AS2026_MENU_GUI_TEST=1 SLINT_SCALE_FACTOR=1 xvfb-run -a --server-args='-screen 0 2560x1440x24' target/debug/as2026-gui --demo --capture-dir /tmp/as2026-valikot
```

Windowsissa aseta ympäristömuuttuja PowerShellissä ennen samaa ohjelma-ajoa.
CI käyttää .NETin prosessikahvaa ja tarkistaa päättymiskoodin. Katso täsmälliset
Linux/Windows-komennot [.github/workflows/ci.yml](../.github/workflows/ci.yml).

CI **avaa natiivin GUI:n**, lähettää hiiri- ja näppäintapahtumia, suorittaa
kilpailun työnkulut ja tallentaa kuvat 100/150 %:n skaalauksilla. Valikkokoe
kattaa kaikki päävalikkovalinnat, vientien takaisinluvun ja luokkien
uudelleenavaamisen. Natiivin tiedostovalitsimen vastaus syötetään synteettisesti;
varsinainen avaus-/tuonti-/tallennuskoodi suoritetaan. Tuloste muodostetaan
lähettämättä sitä paperitulostimelle.

Testit eivät käytä fyysistä Emit-lukijaa tai AS2015A-ohjelmaa. Windowsin
fyysinen lukija, paperitulostus ja historiallisten tulosten täydellinen
vastaavuus edellyttävät erillisiä kokeita. Aiemmat paikalliset laitekokeet
ovat historiallista aineistoa, eivät tämän version hyväksyntätesti.

## Tiedot ja julkaiseminen

Pidä oikeat kilpailut, asetukset ja yksityiset avaimet repositorion ulkopuolella.
`fixtures` sisältää synteettisiä esimerkkejä. Paikallinen tiedostolukko suojaa
saman koneen tallennusta; usean koneen käyttö tehdään [LAN-toiminnolla](LAHIVERKKO.md).
Julkaisuautomaatio, allekirjoitukset ja paketit: [RELEASES.md](RELEASES.md).
