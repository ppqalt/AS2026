# AS2026

**Käyttöliittymän kokeiluhaara:** `beta/kayttoliittyma`. Käynnistä erillinen
esimerkkikilpailu komennolla `./scripts/kokeile-beta.sh`.
[Betan tilanjako ja kokeiluohje](docs/KAYTTOLIITTYMA_BETA.md).
Tämä haara ei ole uusi vakaa julkaisu.

Autosuunnistuskilpailujen hallinta ja tuloslaskenta Linuxille ja Windowsille.

- Kilpailijat, luokat, radat ja asemat samassa sovelluksessa.
- Fluent 2 / Dark Spruce koko käyttöliittymän suunnittelukielenä, ilman animaatioita ja koriste-efektejä.
- Kilpailijan tilat ja perustelut sekä molempien lisenssi- ja seuratiedot.
- Lähtöarvonnan luokkavalinnat arvontapainikkeen vieressä, kellonaikojen esikatselu sekä HTML- ja CSV-lähtöluettelo.
- Usean käsiaika-aseman kellot, pistekorjaukset sekä erilliset Tarkista-, Tutki- ja Hyväksy tulos -toiminnot.
- Pysyvät radan ja aseman muokkauspaneelit sekä tavoiteajan erilliset h/min/s-syöttökentät.
- Jatkuva Emit-kortinluku, automaattinen tallennus ja siirtyminen luetun kilpailijaparin tulokseen.
- RATA- ja KILP-Excel-tuonti (.xls/.xlsx) esikatselulla ja vahvistuksella.
- Tulosluettelot, asemakohtainen tarkastelu ja HTML-, CSV- sekä XLSX-viennit.
- Paikallinen live-HTML-tulosnäkymä ja yhdistettyjen luokkien hallinta.
- Vanhojen AS2015A-kilpailutiedostojen luku.
- Suomenkielinen, tumma käyttöliittymä ja päivitykset sovelluksen asetuksista.

## Asennus

Julkaisupaketit ovat [Releases-sivulla](https://github.com/ppqalt/AS2026/releases): Linuxille `.tar.gz`, Windowsille `.zip`. Molemmat ovat 64-bittisille x86-tietokoneille. Paikalliset kehitysmuutokset tulevat paketteihin vasta erillisessä julkaisussa.

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

[Kilpailun tarkastus, Excel-tuonti ja lähtöjärjestelyt](KILPAILUTOIMINNOT.md) · [Kääntäminen](BUILDING.md) · [Komentorivi ja tiedostot](CLI.md) · [Käyttöliittymän muokkaaminen](UI.md) · [Julkaisujen tekeminen](RELEASES.md)

Puuttuva käsikello estää tuloksen hyväksymisen. Käsimaalia ei päätellä viimeisestä leimasta, ja uusintaluku säilyttää aiemmat tallennetut korjaukset. Tarkemmat työnkulut ja rajat ovat kilpailutoimintojen ohjeessa.

Linux- ja Windows-yhteensopivuuden lisäksi käytännön varmennus vaatii fyysisen lukijan kokeet ja oikean vertailukilpailun; historiallisten AS2015A-tulosten täydellistä vastaavuutta ei väitetä.

## Lisenssi

AS2026:n oma koodi, dokumentaatio ja synteettiset esimerkit: [BSD-2-Clause](LICENSE). Riippuvuuksilla on omat lisenssinsä. Slint-maininta on ohjelman Tietoja-ikkunassa; lisätiedot ovat tiedostossa [THIRD_PARTY.md](THIRD_PARTY.md).
