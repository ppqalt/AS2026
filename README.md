# AS2026

Versio 0.6.8 sisältää kilpailun kokonaismyöhästymisrajan, valittavat lähtöluettelon kentät, Emit-korjauksen Emit-sivulla sekä selkeytetyt toimintovalikot. Pohjana ovat uusimmat 0.6.7:n lähiverkko- ja tulostuskorjaukset. [Muutosten käyttöohje](docs/KAYTTOLIITTYMA_068.md).

Versio 0.6.7 lisää kilpailun yhteiskäytön lähiverkossa, yksittäisten Emit-asemarivien korjauksen sekä vientien ja huomautusten käytettävyysparannukset.
Esimerkkikilpailun voi avata komennolla `cargo run --features gui --bin as2026-gui -- --demo`.


Autosuunnistuskilpailujen hallinta ja tuloslaskenta Linuxille ja Windowsille.

- Kilpailijat, luokat, radat ja asemat samassa sovelluksessa.
- Kilpailun salattu lähiverkkoyhteys parituskoodilla, samat muokkausoikeudet kaikilla koneilla ja katkon aikana tallentuva Emit-lähetysjono; [käyttö- ja palautumisohje](docs/LAHIVERKKO.md).
- Fluent 2 / Dark Spruce koko käyttöliittymän suunnittelukielenä, ilman animaatioita ja koriste-efektejä.
- Kilpailijan tilat ja perustelut sekä molempien lisenssi- ja seuratiedot.
- Lähtöarvonnan luokkavalinnat Kilpailijat-sivun Toiminnot-valikossa, kellonaikojen esikatselu sekä HTML- ja CSV-lähtöluettelo.
- Kortinluvun jälkeinen käsiaikojen ja virhepisteiden syöttö Tab-näppäimellä sekä kilpailijaparin tulostus ilman erillistä hyväksymistä.
- Luokka- ja asematulosteet monivalinnalla ja käyttäjäkohtainen tulostinvalinta; [käyttöohje](docs/TULOSTUS.md).
- Pysyvät radan ja aseman muokkauspaneelit sekä tavoiteajan erilliset h/min/s-syöttökentät.
- Jatkuva Emit-kortinluku, automaattinen tallennus ja siirtyminen luetun kilpailijaparin tulokseen.
- RATA- ja KILP-Excel-tuonti (.xls/.xlsx) esikatselulla ja vahvistuksella.
- Tulosluettelot, asemakohtainen tarkastelu ja HTML-, CSV- sekä XLSX-viennit.
- Paikallinen live-HTML-tulosnäkymä ja yhdistettyjen luokkien hallinta.
- NAC Beginners Cup samasta Emit-lukemasta: oma asemavalinta, virhepisteet ja tuloksen hyväksyntä.
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

[Yhdistettyjen luokkien ja Beginners Cupin käyttö](docs/LUOKAT_JA_BEGINNERS_CUP.md).

Puuttuva käsikello estää tuloksen hyväksymisen. Käsimaalia ei päätellä viimeisestä leimasta, ja uusintaluku säilyttää aiemmat tallennetut korjaukset. Tarkemmat työnkulut ja rajat ovat kilpailutoimintojen ohjeessa.

Linux- ja Windows-yhteensopivuuden lisäksi käytännön varmennus vaatii fyysisen lukijan kokeet ja oikean vertailukilpailun; historiallisten AS2015A-tulosten täydellistä vastaavuutta ei väitetä.

## Lisenssi

AS2026:n oma koodi, dokumentaatio ja synteettiset esimerkit: [BSD-2-Clause](LICENSE). Riippuvuuksilla on omat lisenssinsä. Slint-maininta on ohjelman Tietoja-ikkunassa; lisätiedot ovat tiedostossa [THIRD_PARTY.md](THIRD_PARTY.md).
