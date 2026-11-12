# AS tulkki -integraatio: muutokset kehittäjälle

Lähdekoodin luovutus 12.11.2026. Pohja on AS2026 0.7.5-2, GitHubin
ZIP-kopion commit `d44a7c60e395909e2b0a7aa2ad58acc99a32e151`.
Kuvaus dokumentoi integraation alkuperäisen luovutuksen. Korjattu integraatio sisältyy AS2026 0.7.6:een.

## Käyttäjälle näkyvät muutokset

- Kilvet-välilehti sekä oikeiden rivien Excel-tuonti (`xls`, `xlsx`, `xlsm`).
- Usean radan tuonti samassa ikkunassa, numeroidut AT/JAT/ULA-tunnukset,
  toistuvat kilvet ja jaksokohtaiset pysyvät esiintymätunnisteet.
- Asemakohtainen Ajan syöttö käsin -valinta.
- AS tulkin kilpailutietojen julkaisu ja tulkintojen vastaanotto pääkoneella.
- Liittymiskoodi ja QR-koodi, yhteyden avaaminen/sulkeminen ja koodin vaihto.
- Tulkkaustila, puuttuvien kilpien ja aikojen katselu sekä muutoshistoria.
- RT-puutteiden, muiden virheiden ja käsin syötettyjen aikojen liittäminen
  olemassa olevaan tuloslaskentaan ilman kaksinkertaista kirjausta.
- Koko kilpailun ajanotto ilman Emitiä sekä kuuden numeron kellosyöte.
- Puuttuvat RT:t jaksokohtaisesti tuloslapulle ja erillinen puutteiden koonti.
- Kilven mitätöinti, muuttuneiden tulosten laskenta ja tuloslaput.
- Oikean rivin korvaamisen esikatselu. Tulkatut jaksot säilyvät, poistettu
  kilpi poistuu nykyisistä puutteista ja uusi kilpi on oletuksena löytynyt.
- ULA:n lisääminen JAT:n jälkeen myös julkaistuun riviin. Vanhat ajat
  säilyvät; uusi tuntematon käsin syötettävä aika jää täydennettäväksi.
- Valinnainen automaattinen valmiin tuloksen hyväksyntä ja tulostus.
- Livetuloksiin myös keskeneräisten parien pisteet ja sijoitukset sekä
  Tilanne / RT / Aika / Muut / Yhteensä -sarakkeet ja välituloksen selitys.
- AT:n maksimi-ennakon ja -myöhän etumerkkien korjaus.
- Mitätöinnin vahvistus sallii taustahaun kellonajan muuttumisen; varsinaiset
  kilpailu- ja tulkkaustietojen muutokset tarkistetaan edelleen.

## Toteutuksen keskeiset osat

| Osa | Lähdekoodi |
| --- | --- |
| Oikea rivi, Excel, pysyvät tunnisteet ja koonti | `src/interpretation.rs`, `src/gui/right_row.rs` |
| WordPress-asiakas, sovitus ja siirtotunnisteet | `src/tulkki_yhteys.rs`, `src/gui/tulkki.rs` |
| QR-liittyminen | `src/join_qr.rs`, `ui/tulkki-qr.slint` |
| Käsiajanotto ja hyväksyntä | `src/manual_timing.rs`, `src/workspace.rs`, `src/corrections.rs` |
| Tulosteet ja livetulokset | `src/printing.rs`, `src/live_pages.rs`, `src/gui/printing.rs` |
| GUI ja kokeet | `src/gui/tulkki_smoke.rs`, `ui/tulkki-*.slint`, `tests/interpretation.rs`, `tests/live_interim.rs`, `tests/tulkki_live.rs` |

Ulkoinen osa on erillinen WordPress-lisäosa **AS tulkki – tiedonsiirto 0.7.1**.
Se sisältää puhelimessa toimivan Flutter-selainversion. PHP- ja Dart-koodi
eivät kuulu AS2026:n Rust-repositorioon tai tämän paketin lähdekoodiin.

## Rajapinta ja pysyvät tiedot

- REST-nimiavaruus on `/wp-json/as-tulkki/v1`. HTTPS on pakollinen.
  Luonti/hallinta käyttää WordPressin ylläpitäjän sovellussalasanaa vain
  muistissa. Muu yhteys käyttää kilpailukohtaista avainta.
- AS2026 hakee noin viiden sekunnin välein edellisen haun valmistumisesta;
  puhelin hakee kymmenen sekunnin välein. Valmis lähettää heti.
- Asematunnisteita ei johdeta vaihtuvista Excel-rivinumeroista. Poistettuja
  tunnisteita ei käytetä uudelleen. Mitätöintiä ei peruta.
- Kilpikorjauksissa ja ULA-lisäyksissä nykyiset tulkinnat sovitetaan uuteen
  riviin. Vanhojen aika-asemien poisto/vaihto/järjestys sekä jaksorajat on
  suojattu. Numeron muutos muodostaa uuden kilven.
- Muutossyötteen `pendingTimeIds` kertoo lisätyn ULA:n vielä tuntemattoman
  käsin syötettävän ajan. Se ei ole puuttuva korttimerkintä eikä virhepiste.
  Uusi nykyversion tulkkaus vaatii silti ajan tai todellisen puuttumismerkinnän.
- `originalSections` ja `originalLabels` säilyttävät historian alkuperäisen
  sisällön myös poistuneille kilville. Uusinta samalla requestId:llä ei saa
  palauttaa vanhaa korjausta uuden päälle.
- Tulkki-yhteys, vastaanoton välimuisti ja historia ovat pääkoneen
  käyttäjäasetuksissa. Pisteet, siirtolähde ja oikeat rivit ovat kilpailussa.
- Vanhoissa tiedostoissa puuttuvat uudet kentät saavat serde-oletukset.
  `StoredRoute.seuraava_tunniste` estää tunnisteiden uudelleenkäytön.
  Ratakohtaiset laskentaversiot säilyttävät muiden ratojen käsin korjatut pisteet.
- Automaattinen hyväksyntä on oletuksena pois. Poikkeustiloja ei ratkaista
  automaattisesti. Tulostusyrityksen tunniste tallennetaan ennen jonoon siirtoa;
  tulostinvirheen tai kesken jääneen yrityksen jälkeen tulostetaan tarvittaessa käsin.
- Beginners Cupin erillistä tulosta ei hyväksytä tällä automatiikalla.

## Tarkistukset ennen yhdistämistä

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets --features gui
cargo clippy --locked --all-targets --features gui -- -D warnings
cargo test --manifest-path tools/release/Cargo.toml
cargo run --manifest-path tools/checks/Cargo.toml -- check-no-python
```

Windowsissa on tehty kohdennetut ydin-, GUI- ja selainkokeet sekä Clippy.
Synteettinen GUI-koe ajetaan `--demo --capture-dir <hakemisto>` ja
`AS2026_TULKKI_GUI_TEST=1`, skaalauksilla `SLINT_SCALE_FACTOR=1` ja `1.5`.
Se ei käytä oikeaa kilpailua, verkkosivustoa, kortinlukijaa tai tulostinta.
Tarkka koeyhteenveto ja jäljellä olevat tarkistukset ovat luovutuspaketissa.

Koko Linux/Windows-CI, varsinainen WordPress/MariaDB-integraatiokoe ja kahden
erillisen GUI-prosessin LAN-/katko-/palautumiskoe pitää ajaa ennen yhdistämistä.
Näiden olemassa olevia testejä ei ole poistettu. Fyysisten puhelinten yhteiskäyttö
sekä puuttuvien aikojen sääntöpoikkeustestit on siirretty myöhemmäksi käyttäjän päätöksellä.

Julkaisuversion päättää ylläpitäjä. Päivitä silloin Cargo.toml, Cargo.lock ja
julkaisutiedot normaalisti sekä käytä `docs/RELEASES.md`:n allekirjoitettua
työnkulkua. Nykyistä 0.7.5-2-tagia tai paketteja ei ylikirjoiteta.
