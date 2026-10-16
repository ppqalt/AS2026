# AS2026 — autosuunnistuksen kilpailunhallinta

AS2026 on suomenkielinen Rust-ohjelma Linuxille ja Windowsille. Se jatkaa olemassa olevaa AS2015A-ohjelman tutkimukseen perustuvaa laskentatoteutusta. Mukana ovat graafinen käyttöliittymä, komentorivikäyttö, vanhojen kilpailuaineistojen luku, uusi TOML-kilpailumuoto, kilpailutietojen muokkaus, jatkuva Emit-luku, automaattinen tallennus ja tulosten esikatselu.

Ohjelma on kehitysversio. Laskennan täydellistä vastaavuutta AS2015A:n kilpailutuloksiin ei ole varmennettu. Synteettiset regressiotestit ja oikean lukijan kokeet eivät korvaa tätä vertailua. Repositorio säilyy toistaiseksi yksityisenä.

## Lisenssi

AS2026:n oma lähdekoodi, dokumentaatio ja synteettiset esimerkit ovat [BSD-2-Clause-lisenssillä](LICENSE). Lähdekoodia ja ohjelmatiedostoja jaettaessa säilytetään tekijänoikeusilmoitus, lisenssiehdot ja vastuuvapauslauseke. Ohjelma tarjotaan ilman takuuta. Lisenssi ei koske alkuperäistä AS2015A-ohjelmaa, kilpailuaineistoja eikä ulkopuolisia kirjastoja.

Riippuvuudet säilyttävät omat lisenssinsä. Slintiä käytetään työpöytäsovelluksessa sen maksuttomalla Royalty-free 2.0 -lisenssillä, ja sen edellyttämä maininta on ohjelman **Tietoja**-ikkunassa. Katso [THIRD_PARTY.md](THIRD_PARTY.md).

## Graafinen käyttöliittymä

```sh
cargo run --features gui --bin as2026-gui
cargo run --features gui --bin as2026-gui -- --demo
cargo run --features gui --bin as2026-gui -- oma-kilpailu
cargo build --release --features gui --bin as2026-gui
./target/release/as2026-gui
```

Windowsissa ohjelmatiedosto on `as2026-gui.exe`. Käyttöliittymä toteutetaan Slintillä. Sen seitsemän näkymää ovat **Yleiskatsaus**, **Kilpailijat**, **Emit-lukeminen**, **Tulokset**, **Luokat**, **Radat ja asemat** sekä **Kilpailun asetukset**. Uuden kilpailun voi perustaa ja aiemman avata tiedostonvalinnalla. Vanhoja AS2015A-aineistoja voi tarkastella muuttamatta lähdetiedostoja.

Kilpailun voi avata myös antamalla käynnistyksessä kansion tai `kilpailu.toml`-tiedoston polun. `--demo` avaa synteettisestä kilpailusta erillisen muokattavan väliaikaiskopion; lähdekoodin esimerkkitiedostot säilyvät ennallaan.

Uuden TOML-kilpailun kilpailijoita, luokkia, ratoja ja asemia muokataan yhteisessä kilpailumallissa. Olemassa olevan tietueen muutos tallentuu, kun kenttä hyväksytään Enterillä tai kentästä siirrytään pois. Uusi tietue täytetään ensin kokonaan ja lisätään **Lisää**-toiminnolla. Virheellinen tai vielä keskeneräinen syöte ei korvaa tallennettua kilpailua. Näkymän tallennustila kertoo, onnistuiko tallennus.

Kilpailijoita voi hakea luettelosta. Luokka, rata, asematyyppi ja laskentasääntö valitaan nimetyistä vaihtoehdoista, jotta sisäisiä koodeja ei tarvitse muistaa.

Emit-näkymässä valitaan automaattisesti tunnistettu tai itse annettu portti ja käynnistetään jatkuva luku. Lukija toimii taustalla, joten muut näkymät säilyvät käytettävissä. Korttien hyväksytyt lukemat tallentuvat samaan kilpailuun. Tulokset muodostetaan ajantasaisista tiedoista ja raportin voi tallentaa HTML-tiedostoksi.

Ulkoasua, komponentteja ja muokkausperiaatteita käsittelee [UI.md](UI.md). Alustojen käännös- ja käynnistysvaatimukset ovat tiedostossa [BUILDING.md](BUILDING.md). GUI on ensimmäinen käyttökelpoinen versio: se ei lisää vielä puuttuvia laskentasääntöjä tai kaikkia AS2015A:n erikoistoimintoja.

## Komentorivin käynnistäminen

Tarvitset kääntämiseen Rustin version 1.89 tai uudemman. Tarkemmat alustakohtaiset ohjeet ovat tiedostossa [BUILDING.md](BUILDING.md).

```sh
cargo run
cargo run -- esimerkki
cargo build --release
./target/release/as2026
```

`as2026` avaa ilman argumentteja suomenkielisen päätekäyttövalikon. Siinä voi tarkastaa kilpailun, laskea esikatselun, näyttää tulokset, purkaa kehystiedoston, kokeilla esimerkkikilpailua, avata jatkuvan Emit-luvun tai perustaa uuden kilpailun. Valinta 0 sulkee ohjelman. Tulosraportin voi tallentaa HTML-tiedostoksi ja tulostaa selaimesta. Olemassa olevaa raporttitiedostoa ei korvata. CLI voidaan kääntää myös ilman `gui`-ominaisuutta ja sen riippuvuuksia.

Windowsin ohjelmatiedosto on `as2026.exe`. Valmis ohjelma ei tarvitse Rustia tai erillistä palvelinta. Sarjaportin käyttö toteutetaan samalla Rust-rajapinnalla molemmilla alustoilla; Windows-käännös, testit, Clippy ja ohjelman sisäinen esimerkki on varmennettu [GitHubin Windows-ajossa](https://github.com/ppqalt/AS2026/actions/runs/38806074766). Fyysistä Windows-lukijaa ei ole testattu.

## Uusi kilpailu ja yhteiset tiedot

```sh
cargo run -- uusi oma-kilpailu "Harjoituskilpailu"
cargo run -- tarkista oma-kilpailu
```

`uusi` luo kansioon UTF-8-muotoisen `kilpailu.toml`-tiedoston. Olemassa olevaa kilpailutiedostoa ei korvata. Pohjassa ovat harjoitusluokka H, rata 1 sekä lähtö ja maali. Kilpailijan voi lisätä alla kuvatulla muutostiedostolla.

Kilpailulla on yksi yhteinen `native::Project`-malli. Kilpailija viittaa luokan tunnukseen, luokka radan numeroon ja rata sisältää asemansa. Kortti yhdistetään kilpailijaan Emit-numeron perusteella. Luokan, radan, nimen tai leimausten muutos tulee mukaan seuraavaan laskentaan samasta mallista; erillistä vanhentuvaa tuloskopiota ei ylläpidetä. Saman kilpailijanumeron tai Emit-numeron ristiriitainen käyttö hylätään.

Tiedostomuoto säilyy versiona `versio = 1`. Aiemmat version 1 tiedostot avautuvat edelleen. Kilpailijalle on lisätty vapaaehtoinen `auto` ja lukutapahtumalle vapaaehtoinen `luettu_unix_ms`. Jälkimmäinen kertoo uuden lukutapahtuman tallennusajan Unix-ajan millisekunteina; aiemmista tiedostoista kenttä voi puuttua. Luettava esimerkki on [fixtures/native-demo/kilpailu.toml](fixtures/native-demo/kilpailu.toml).

## Kilpailijan, luokan, radan ja aseman muuttaminen

Kirjoita esimerkiksi `muutokset.toml`:

```toml
[[kilpailijat]]
numero = 1
luokka = "H"
kuljettaja = "Koe"
kartanlukija = "Testi"
auto = "Harjoitusauto"
emit = 538747
lahtoaika = "15:00:00"
emit_korjaus_sekuntia = 0
```

Käytä muutosta:

```sh
cargo run -- paivita oma-kilpailu muutokset.toml
```

Komento tarkistaa koko muuttuneen kilpailun ja tallentaa sen automaattisesti. Virheellinen muutos ei korvaa kilpailua. Erillistä Talleta-painiketta tai lopetukseen asti odottavaa tallennusta ei ole.

Muutostiedoston osioiden merkitys:

| Osio | Tunniste ja vaikutus |
| --- | --- |
| `[[kilpailijat]]` | Sama `numero` korvaa koko kilpailijatietueen; uusi numero lisää kilpailijan. |
| `[[luokat]]` | Sama `tunnus` korvaa koko luokan; uusi tunnus lisää luokan. |
| `[[radat]]` | Sama `numero` korvaa koko radan ja **kaikki sen asemat**; uusi numero lisää radan. |
| `[kilpailu]` | Korvaa koko kilpailun perustieto-osion. |
| `[virhepisteet]` | Korvaa koko kilpailukohtaisten virhepisteasetusten osion. |

Vain mukana olevat tietueet ja osiot vaihdetaan. Muut kilpailijat, luokat ja radat säilyvät. Yksittäisen mukana olevan tietueen kenttiä ei yhdistetä vanhoihin: anna kaikki tarvittavat arvot. Jos jätät vapaaehtoisen kentän pois korvattavasta tietueesta, sen oletusarvo tulee käyttöön. Esimerkiksi tyhjä `[virhepisteet]` palauttaa kilpailukohtaiset poikkeukset laskentaytimen oletuksiin. Toistuvat tunnisteet saman muutostiedoston sisällä ja tuntemattomat kentät hylätään. Muutostiedostolla ei poisteta tietueita.

Rataa muuttaessa ilmoita myös kaikki säilytettävät asemat. Esimerkiksi uuden kilpailupohjan rata voidaan korvata näin:

```toml
[[radat]]
numero = 1
nimi = "Harjoitusrata"

[[radat.asemat]]
tyyppi = "L"
nimi = "Lähtö"
emit_koodi = 0
tavoiteaika_sekuntia = 0

[[radat.asemat]]
tyyppi = "M"
nimi = "Maali"
emit_koodi = 0
tavoiteaika_sekuntia = 300
jakso_minuutteina = 5
```

Tiedostoa voi muokata myös Zedissä tai muussa tekstieditorissa ohjelman ollessa suljettuna. Suora editorimuutos ei kulje sovelluksen tarkistuksen ja varmuuskopioinnin kautta; `paivita` käyttää turvallista tallennuspolkua.

## Jatkuva Emit-luku ja automaattinen tallennus

```sh
cargo run -- lue oma-kilpailu
cargo run -- lue oma-kilpailu /dev/ttyUSB0
```

Windowsissa portti voi olla esimerkiksi `COM3`. Automaattinen tunnistus etsii ensisijaisesti FTDI-porttia; Linuxissa käytetään myös `/dev/serial/by-id`-nimiä ja tarvittaessa `/dev/ttyUSB0`-porttia. Portin voi aina valita itse. Linuxissa käyttäjällä pitää olla oikeus käyttää valittua porttia.

Jatkuva toiminto avaa portin kerran asetuksilla **9600 baud, 8 databittiä, ei pariteettia, 2 stop-bittiä (8N2)**. Ohjelma odottaa kortteja samassa portissa, tallentaa jokaisen hyväksytyn lukutapahtuman ja näyttää tunnetun kilpailijan laskennan. Seuraavaa korttia voi lukea heti. Ctrl+C lopettaa jatkuvan luvun: valikkokäytössä palataan valikkoon, komentorivikäytössä ohjelma päättyy.

Lukuhistoria, kortin leimaussisältö ja laskentatulos ovat eri asioita:

- Hyväksytyt lukijakehykset säilyvät `[[leimaluvut]]`-historiassa, myös samaa sisältöä toistettaessa. Lukija voi lähettää uuden kehyksen kortin pysyessä paikallaan; historiarivi ei yksin todista erillistä kortin nostoa ja uudelleenasetusta.
- Kortin varsinaista sisältöä verrataan ensimmäistä 250-lukumerkkiä edeltävistä leimauksista, ankkuri mukaan lukien. Pelkkä 250-merkin ajan muutos ei tarkoita muuttunutta leimaussisältöä.
- Laskenta käyttää historian viimeisintä kelvollista lukemaa kullekin kortille. Kilpailijasta muodostetaan yksi aktiivinen tulos, joten uusintaluku ei tee kaksoistulosta. Lukukello ja lukumerkin aika voivat silti vaikuttaa maalikellon johtamiseen.
- Muuttuneet varsinaiset leimaukset havaitaan. Eri kortteja voi lukea peräkkäin.
- Tuntematon kortti tallentuu historiaan ja yhdistyy myöhemmin lisättyyn kilpailijaan Emit-numerolla.
- Hylättävä uusi lukema ei poista aiempaa kelvollista lukemaa. Vanhan TOML-historian virheellinen lukema ohitetaan laskennassa ja siitä ilmoitetaan.

Kehyksen osat säilyvät sarjaportin aikakatkaisujen yli, ja samassa tavupuskurissa saapuneet useat kehykset käsitellään järjestyksessä. Rakenteeltaan hylättävän aineiston jälkeen etsitään seuraavaa kehystä. Hyötydatassa oleva synkkatavupari ei katkaise ehjää kehystä. Protokollalle ei ole varmennettu tarkistussummaa, joten kaikkea satunnaista tavuhävikkiä tai vioittumista ei voida tunnistaa.

Pelkkä `as2026 lue` tai `as2026 lue /dev/ttyUSB0` on edelleen **kertaluonteinen laitetarkistus**. Se odottaa korkeintaan 30 sekuntia, näyttää yhden kortin eikä tallenna kilpailuun. Automaattinen tallennus edellyttää kilpailukansion antamista.

Tallennuspolkua voi käyttää ilman fyysistä lukijaa:

```sh
as2026 tallenna-kehys KANSIO KEHYSTIEDOSTO HH:MM:SS
```

Komento lukee yhden 217 tavun tiedoston ja käyttää samaa kilpailumallin ja tallennuksen polkua kuin jatkuva luku. Kellonaika on kortin lukukello. Synteettisen kehyksen käsittely tällä komennolla ei todista fyysisen lukijan toimintaa.

## Tallennuksen turvallisuus ja palautuminen

Tallennus validoi ehdokkaan, kirjoittaa valmiin tilapäistiedoston samaan kansioon ja korvaa varsinaisen tiedoston vasta kirjoituksen jälkeen. Aiempi sisältö säilytetään tiedostossa `kilpailu.toml.bak`. Se sisältää yhden edellisen tallennetun version, eikä ole koko kilpailun pitkäaikainen varmuuskopiohistoria. TOML tallennetaan uudelleen, joten editorikommentit ja alkuperäinen asettelu eivät säily.

Uuden kilpailutiedoston luominen tarvitsee kovat linkit sallivan tiedostojärjestelmän. Käytä tavallista paikallista kilpailukansiota: esimerkiksi FAT- ja exFAT-taltiot eivät tue tätä luontipolkua. Tallennusvirheestä ilmoitetaan, eikä olemassa olevaa kilpailua korvata uuden kilpailun luonnissa.

`kilpailu.toml.lukko`-sivutiedoston käyttöjärjestelmälukko estää tämän sovelluksen paikalliset päällekkäiset kirjoitukset. Lukko vapautuu prosessin sulkeutuessa; sivutiedoston olemassaolo ei tarkoita, että lukko olisi varattu. Sovellus vertaa myös tiedoston sisältöä avaamisen yhteydessä luettuun versioon ja ilmoittaa havaitusta ristiriidasta ennen korvaamista. Tallennusvirhe ilmoitetaan käyttäjälle. Jos tiedosto ehti vaihtua mutta pysyvyyden varmistus epäonnistui, tästä annetaan erillinen ilmoitus ja kilpailu pitää avata uudelleen tarkistusta varten.

Ulkoinen tekstieditori ei noudata sovelluksen lukkoa. Sisältövertailu havaitsee monia ulkoisia muutoksia, mutta ei tee editorin ja sovelluksen samanaikaisesta kirjoituksesta turvallista. Sulje ohjelma ennen suoria editorimuutoksia. Tämä on paikallinen tiedostovarasto; jaettu verkkolevy ei muuta sitä usean koneen tietokannaksi.

Palautus tehdään ohjelman ollessa suljettuna: säilytä ensin kopio nykyisestä `kilpailu.toml`-tiedostosta, tarkasta `kilpailu.toml.bak` editorissa ja kopioi tarvittaessa sen sisältö varsinaisen tiedoston tilalle. Avaa kilpailu uudelleen ja tarkasta tiedot. Säilytä lisäksi erilliset kilpailukansion varmuuskopiot.

## Vanhojen AS2015A-kilpailujen luku

Jos kansiossa on `kilpailu.toml`, käytetään sitä. Muuten luetaan vanhat `Aspara.txt`, `Asperus.txt`, `Radat.txt` ja `Kilp.txt`. Korttilaskenta tarvitsee myös `emit.txt`-tiedoston; sarjapisteiden oma taulukko eli Sp 4 tarvitsee `sarjap.txt`-tiedoston.

Tiedostonimien kirjainkoko ei vaikuta vanhojen tiedostojen hakuun. Moniselitteiset samannimiset tiedostot hylätään. Oletusmerkistö on Windows-1252; `--utf8` valitsee UTF-8:n. Puolipistekentät ja kiinteälevyiset tiedot käsitellään vanhan tiedostomuodon lukijassa. Lähdetiedostoja ei kirjoiteta uudelleen.

Vanhojen aineistojen tuonti ja uuden kilpailun tallennus ovat erillisiä polkuja. Vanhan kilpailun tarkastaminen ei tee automaattista TOML-migraatiota. Jatkuva tallentava Emit-luku ja `paivita` tarvitsevat uuden muodon `kilpailu.toml`-tiedoston.

## Laskenta, tulokset ja komennot

```sh
as2026 tarkista KANSIO [--utf8]
as2026 laske KANSIO [--utf8] [--vaiheet] [--raportti laskenta.html]
as2026 tulokset KANSIO [--utf8] [--raportti tulokset.html]
as2026 tulokset KANSIO --ratkaise-tasatulokset
as2026 pura KEHYSTIEDOSTO [HH:MM:SS]
as2026 esimerkki
as2026 ohje
```

Näissä esimerkeissä `as2026` tarkoittaa oman alustan ohjelmatiedostoa. Kehityshakemistossa sama komento on esimerkiksi `cargo run -- tarkista fixtures/synthetic`. Laita välilyöntejä sisältävä polku lainausmerkkeihin.

`laske` näyttää korttiaineistosta lasketut asemarivit ja huomautukset. TOML-kilpailun `tulokset` laskee tulosluettelon ajantasaisesta kilpailumallista; se on kehitysversion asemalaskennan tulos. Vanhan kilpailun `tulokset` käyttää `Kilp.txt`-tiedoston tallennettuja pisteitä. Esikatselua ei kirjoiteta vanhojen tallennettujen tulosten päälle. Uuden kilpailun johdettuja tuloksia ei myöskään ylläpidetä erillisenä pysyvänä kopiona.

Toteutettu laskenta kattaa leimausten kohdistuksen, AT/JAT/ULA-jaksot, puuttuvien ja ylimääräisten asemien virheet, ULA-pyöristykset, enimmäissakot, myöhästymisrajat sekä säännön 4 käsittelyn. Tulosluettelossa on luokka- ja ratakohtainen järjestys. Vanha aineistopolku tukee lisäksi yhdistettyjä luokkia, osallistumistilastoa sekä nousu- ja sarjapisteitä. Uuden TOML-muodon tietomalli ei vielä kata kaikkia vanhan kilpailun asetuksia.

Tasapisteet säilyvät oletuksena jaettuina. Asematietoihin perustuvan ratkaisun voi pyytää erikseen, mutta se tehdään vain riittävillä, samoihin asemiin kohdistetuilla tiedoilla. Avoimet tarkastustulokset estävät nousu- ja sarjapisteiden vahvistamisen.

Laskennan olennaiset rajat säilyvät:

- Esikatselun summa sisältää asemarivien virhepisteet. Erillisiä kilpailijakohtaisia RT-lisäyksiä ja muita virhepisteitä ei vielä yhdistetä siihen.
- Korttilaskenta tukee ajanottotapoja 0 ja 1. Ajanottotavan 0 polku käyttää korttiaineistoa; käsin syötettyjen asemakellojen yleistä uudelleenlaskentaa ei ole.
- Ajanottotapaa 2 ja kaikkia lähtö-/maalikellon poikkeustapauksia ei tueta. Käsimaalin johtamisesta viimeisestä leimasta annetaan huomautus.
- Epäselvät tai tukemattomat polut ilmoitetaan; laskentasääntöjä ei päätellä puuttuvan vertailuaineiston tilalle.

## Jatkokehityksen kattavuus

Alkuperäisen AS2015A:n kahdeksan päätoimintoa ohjaavat kattavuutta, eivät uuden käyttöliittymän välilehtijakoa:

| Alkuperäinen toiminto | Nykyinen toteutus ja jäljellä oleva työ |
| --- | --- |
| Parametrit ja perustiedot | Vanhojen tietojen luku, TOML-perustiedot ja virhepisteasetukset sekä kilpailun asetusten GUI-näkymä. Kaikkien alkuperäisten asetusten kattavuus puuttuu. |
| Luokkien tiedot | Luokan lisäys ja korvaus sekä yksiselitteinen rataviittaus. Kaikkia vanhoja luokka-, yhdistely- ja pisteasetuksia ei voi määrittää TOML-muodossa. |
| Ratatiedot | Ratojen ja asemien luku, kokonaisen radan muutos sekä graafinen rata- ja asemaeditori. Kaikkien vanhojen poikkeussääntöjen kattavuus on kesken. |
| Emit-leimantarkastus | Kehyksen purku, jatkuva lukija, historia ja kohdistus. Laajemmat fyysiset laitekokeet ja laskennan vastaavuusvertailu jatkuvat. |
| Asemakohtaiset tiedot | Laskennan asemarivit, huomautukset ja HTML-raportti. Käsin syötettyjen asemakellojen ja kaikkien korjausten työnkulku puuttuu. |
| Tuloslaskenta | Laskentaydin ja luettelot olemassa. Kattava AS2015A-vertailu, ajanottotapa 2 ja erilliset lisävirhepisteet ovat kesken. |
| Ilmoittautumiset | Kilpailijan lisäys ja korvaus yhteiseen malliin, auto, Emit-kohdistus ja graafinen kilpailijaeditori. Lähtöaikojen arvonta puuttuu. |
| Emit-rivin teko | Kehyksestä muodostetaan yhteisen mallin lukutapahtuma. Vanhan ohjelman koko erillistä rivinmuodostus- ja vientitoimintoa ei ole toteutettu. |

Seuraavien ominaisuuksien tulee käyttää `ProjectStore::update`-rajapintaa ja samaa kilpailumallia kuten GUI:ssa ja komentorivillä nyt. Verkkopalvelinta tai monen koneen synkronointia ei ole toteutettu.

## Tarkistukset

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
cargo test --all-targets --features gui
cargo clippy --all-targets --features gui -- -D warnings
cargo build --release --features gui --bin as2026-gui
```

Testit kattavat vanhojen tiedostojen lukua, TOML-yhteensopivuutta, kohdistuksia, laskentaa ja järjestystä, uusintalukuja, jatkuvan lukijan tavupuskurointia sekä tallennuksen epäonnistumista ja paikallisia ristiriitoja. Testiaineisto on synteettistä. Erillisessä Linux-laitetestissä oikea kortti 538747 luettiin toistuvasti samassa jatkuvassa istunnossa, historia tallentui ja kilpailu avautui uudelleen. Tämä ei varmista kaikkia laite- ja kilpailutilanteita eikä laskennan täydellistä vastaavuutta alkuperäisiin kilpailutuloksiin.
