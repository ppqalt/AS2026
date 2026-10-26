# AS2026:n komentorivi ja kilpailutiedostot

## Komentorivin käynnistäminen

Valmis komentoriviohjelma tulee julkaisupaketissa. Kehityskäännöksessä käytä ajantasaista vakaata Rustia ja lukittuja riippuvuuksia; Cargo.toml ilmoittaa paketin vähimmäisversion. Komennot suoritetaan repositorion juuressa. [Kääntäminen ja testaus](BUILDING.md).

```sh
cargo run
cargo run -- esimerkki
cargo build --release
./target/release/as2026
```

`as2026` avaa ilman argumentteja suomenkielisen päätekäyttövalikon. Siinä voi tarkastaa kilpailun, laskea esikatselun, näyttää tulokset, purkaa kehystiedoston, kokeilla esimerkkikilpailua, avata jatkuvan Emit-luvun tai perustaa uuden kilpailun. Valinta 0 sulkee ohjelman. Tulosraportin voi tallentaa HTML-tiedostoksi ja tulostaa selaimesta. Olemassa olevaa raporttitiedostoa ei korvata. CLI voidaan kääntää myös ilman `gui`-ominaisuutta ja sen riippuvuuksia.

Windowsin ohjelmatiedosto on `as2026.exe`. Valmis ohjelma ei tarvitse Rustia tai erillistä palvelinta. Sarjaportti käyttää samaa Rust-rajapintaa Linuxissa ja Windowsissa. Fyysisen lukijan ja Windowsin käytännön työnkulut tarvitsevat erilliset laitekokeet.

## Uusi kilpailu ja yhteiset tiedot

```sh
cargo run -- uusi oma-kilpailu "Harjoituskilpailu"
cargo run -- tarkista oma-kilpailu
```

`uusi` luo kansioon UTF-8-muotoisen `kilpailu.toml`-tiedoston. Olemassa olevaa kilpailutiedostoa ei korvata. Pohjassa ovat harjoitusluokka H, rata 1 sekä lähtö ja maali. Kilpailijan voi lisätä alla kuvatulla muutostiedostolla.

Kilpailulla on yksi yhteinen `native::Project`-malli. Kilpailija viittaa luokan tunnukseen, luokka radan numeroon ja rata sisältää asemansa. Kortti yhdistetään kilpailijaan Emit-numeron perusteella. Kortinlukuhistoria ja tallennettu tarkastustulos ovat erillisiä: uusintaluku säilyttää käsikorjaukset, kunnes ne korvataan erikseen Tarkista-toiminnolla. Muuttunut rata vaatii uuden tarkastuksen. Ristiriitaiset kilpailija- tai Emit-numerot hylätään.

Tiedostomuoto säilyy versiona `versio = 1`. Aiemmat tiedostot avautuvat edelleen. Kilpailijan auto, lisenssit, seurat, käsin valittu tila ja tallennettu asematulos ovat valinnaisia tietoja. Luokan `osallistuu_lahtoarvontaan` on vanhoissa tiedostoissa oletuksena tosi; Excel-tuonnissa se on aluksi epätosi. Lukutapahtuman valinnainen `luettu_unix_ms` kertoo tallennusajan Unix-ajan millisekunteina. Esimerkki on [fixtures/native-demo/kilpailu.toml](../fixtures/native-demo/kilpailu.toml).

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

Vain mukana olevat tietueet ja osiot vaihdetaan; muut kilpailijat, luokat ja radat säilyvät. Anna korvattavan tietueen tarvittavat perustiedot. Yhteensopivuuden vuoksi pois jätetyt lisenssit, seurat, tila, lisävirheet, tallennettu tulos ja luokan arvontavalinta säilyvät vanhoista tiedoista. Nimenomainen tyhjä lisenssi tai seura tyhjentää tiedon. Muutostiedoston `tila = { tunnus = "" }` palauttaa automaattisen tilan; varsinaisesta kilpailutiedostosta tila poistetaan kokonaan. Muiden vapaaehtoisten kenttien poisjättö käyttää oletusta. Tyhjä `[virhepisteet]` palauttaa kilpailukohtaiset poikkeukset oletuksiin. Toistuvat tunnisteet ja tuntemattomat kentät hylätään. Muutostiedostolla ei poisteta tietueita.

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
- Uusi korttilaskenta käyttää historian viimeisintä kelvollista lukemaa. Kilpailijalla on yksi aktiivinen tulos; uusintaluku ei tee kaksoistulosta eikä korvaa tallennettuja käsikorjauksia. Uuden kilpailun käsimaali täydennetään erillisellä kellolla, ei johtamalla se viimeisestä leimasta.
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
as2026 tulokset KANSIO [--utf8] [--raportti tulokset.html] [--csv tulokset.csv] [--xlsx tulokset.xlsx] [--live PORTTI]
as2026 tulokset KANSIO --ratkaise-tasatulokset
as2026 pura KEHYSTIEDOSTO [HH:MM:SS]
as2026 esimerkki
as2026 ohje
```

Näissä esimerkeissä `as2026` tarkoittaa oman alustan ohjelmatiedostoa. Kehityshakemistossa sama komento on esimerkiksi `cargo run -- tarkista fixtures/synthetic`. Laita välilyöntejä sisältävä polku lainausmerkkeihin.

`laske` näyttää TOML-kilpailun samat tallennetut asemakorjaukset ja täydennettävät käsikellot kuin käyttöliittymä. Jos tallennettua asematulosta ei ole, rivit muodostetaan korttiluvusta. Puuttuva käsikello näkyy viivana ja tulos odottaa tarkastusta. `tulokset` käyttää samaa ajantasaista kilpailumallia. Komennot eivät itsessään hyväksy tulosta tai muuta kilpailutiedostoa.

Vanhan AS2015A-aineiston `laske` on erillinen raakaleimausten staattinen esikatselu. Sen `tulokset` käyttää `Kilp.txt`-tiedoston tallennettuja pisteitä. Kumpikaan ei korvaa vanhoja lähdetiedostoja. Käyttöliittymän Tarkista, Tutki, käsikorjaukset ja Hyväksy tulos on kuvattu [kilpailutoiminnoissa](KILPAILUTOIMINNOT.md).

AS2026-kilpailun toimitsijalaskenta tukee ajanottotapoja 0, 1 ja 2 sekä erikseen annettuja käsikelloja. Kokonaispisteet yhdistävät asemien aikavirheet ja muut virheet, puuttuvien ja ylimääräisten RT-asemien erilliset pisteet sekä muut kilpailijakohtaiset pisteet. Oletuksena puuttuva RT antaa 600 pistettä; kilpailun asetus voi muuttaa kerrointa.

Tulosluettelossa ovat luokka- ja ratakohtainen järjestys, yhdistetyt luokat sekä nousu- ja sarjapisteet. Syy säilyy HTML-, CSV- ja XLSX-viennissä. CSV on puolipistein erotettua UTF-8:aa ja XLSX tavallinen työkirja ilman makroja tai suoritettavia kaavoja. Raportit eivät korvaa olemassa olevaa tiedostoa.

`--live 8765` avaa tulosnäkymän vain samalla koneella osoitteessa `http://127.0.0.1:8765/`. Kilpailu luetaan uudelleen selaimen pyynnöillä ja näkymä päivittyy viiden sekunnin välein. Ctrl+C lopettaa palvelimen. Se ei ole usean koneen synkronointi.

Versiosta 0.7.2 alkaen tasapisteet ratkaistaan RT-määrän ja aika-asemien parivertailulla, kun tiedot ovat riittävät ja kohdistuvat samoihin asemiin. Tasainen, puutteellinen tai ristiriitainen vertailu jää jaetuksi. Valitsin `--ratkaise-tasatulokset` käyttää yhteensopivuuden vuoksi vaihtoehtoista vanhaa parhaiden rivien vertailua. Avoimet tarkastustulokset estävät nousu- ja sarjapisteiden vahvistamisen.

## Käsikellot, Excel-tuonti ja lähtöjärjestelyt

Käyttöliittymän **Tulokset** täydentää Emit-numerolla 0 merkityt asemat muodossa `hh:mm:ss`. Myös käsimaalin aika annetaan erikseen. **Tutki** laskee tallennetut kellot uudelleen; **Hyväksy tulos** säilyttää asemien käsin korjatut pisteet. Keskeneräinen, virheellinen tai vanhentunut tulos ei kelpaa hyväksyttäväksi. Nämä toiminnot eivät ole erillisiä komentorivikomentoja.

Tunnetun kilpailijaparin korttiluku avaa käyttöliittymässä automaattisesti Tulokset-näkymän. Keskeneräinen muokkaus saa valmistua ensin. Uusintaluku säilyttää tallennetun tarkastuksen; **Tarkista…** korvaa sen vasta vahvistuksen jälkeen.

**Tuo Excel…** lukee `.xls`- tai `.xlsx`-pohjan RATA- ja KILP-välilehdet esikatseluun. Hyväksyntä luo uuden kilpailun valittuun kansioon. Seuratiedot säilyvät, eikä puuttuvia luokkia tai ratoja arvata. Tuonti ja sen vahvistus ovat käyttöliittymässä. Katso [Excel-kenttäkartta](EXCEL_TUONTI.md).

**Kilpailijat → Toiminnot → Arvo lähtöajat** avaa kaikkien luokkien valintaruudut. Ikkunan **Arvo lähtöajat** tallentaa luokkien **Osallistuu lähtöarvontaan** -valinnat ja siirtyy kellonaikojen esikatseluun. Luokkavalinnan peruuttaminen ei muuta tietoja; lähtöajat tallentuvat vasta esikatselun hyväksymisestä. Ulkopuoliset luokat sekä jo luetut tai asematuloksen saaneet kilpailijat ohitetaan ja säilyttävät aikansa. Muut valitut lukemattomat arvotaan normaalisti. Lähtöluettelon voi tallentaa HTML:nä tai CSV:nä.

Vanhan AS2015A-aineiston staattinen laskentapolku on erillään uuden kilpailun toimitsijalaskennasta. Kaikkia vanhan ohjelman epäselviä poikkeustapauksia tai historiallisten tulosten täydellistä vastaavuutta ei ole varmennettu.

## Tarkistukset

Yhteiset Rust-, Slint- ja GUI-tarkistukset on koottu
[kääntämis- ja testausohjeeseen](BUILDING.md#rust--slint--ja-paketointitarkistukset).
Ne kattavat vanhat tiedostot ja Excel-tuonnin, TOML-yhteensopivuuden,
käsikellot, tilasiirtymät, uusintaluvut, hyväksynnän esteet sekä tallennuksen
virheet ja ristiriidat. Paikallisen alkuperäisen Excel-mallipohjan erillinen
koe on kuvattu [Excel-tuonnin ohjeessa](EXCEL_TUONTI.md).
