# AS2026

Autosuunnistuskilpailujen hallinta ja tuloslaskenta Linuxille ja Windowsille.

- Kilpailijat, luokat, radat ja asemat samassa sovelluksessa.
- Jatkuva Emit-kortinluku ja automaattinen tallennus.
- Tulosluettelot, asemakohtainen tarkastelu ja HTML-raportit.
- Vanhojen AS2015A-kilpailutiedostojen luku.
- Suomenkielinen, tumma käyttöliittymä ja päivitykset sovelluksen asetuksista.

## Asennus

Ensimmäinen vakaa julkaisu on AS2026 0.5.4. Tulevatkin paketit löytyvät [Releases-sivulle](https://github.com/ppqalt/AS2026/releases): Linuxille `.tar.gz`, Windowsille `.zip`. Molemmat ovat 64-bittisille x86-tietokoneille.

Pura paketti, avaa `as2026-gui` (Windowsissa `as2026-gui.exe`) ja valitse **Asenna ja käynnistä**. AS2026 tulee sovellusvalikkoon. Päivitykset löytyvät kohdasta **sivupalkin Tarkista päivitykset -painike**.

Uusi kilpailu luodaan antamalla sen nimi. Kilpailut tallennetaan normaalisti Asiakirjat-kansion `AS2026/Kilpailut`-kansioon. Tallennuspaikan voi vaihtaa asetuksista. Ohjelmapäivitykset säilyttävät kilpailutiedot ja asetukset.

## Kehitys

```sh
cargo run --features gui --bin as2026-gui
cargo test --all-targets --features gui
cargo clippy --all-targets --features gui -- -D warnings
cargo build --release --features gui
cargo test --manifest-path tools/release/Cargo.toml
```

Rust 1.89 tai uudempi. Komentoriviohjelman voi kääntää ilman GUI-riippuvuuksia: `cargo build --release`.

[Kääntäminen](BUILDING.md) · [Komentorivi ja tiedostot](CLI.md) · [Käyttöliittymän muokkaaminen](UI.md) · [Julkaisujen tekeminen](RELEASES.md)

## Lisenssi

AS2026:n oma koodi, dokumentaatio ja synteettiset esimerkit: [BSD-2-Clause](LICENSE). Riippuvuuksilla on omat lisenssinsä. Slint-maininta on ohjelman Tietoja-ikkunassa; lisätiedot ovat tiedostossa [THIRD_PARTY.md](THIRD_PARTY.md).
