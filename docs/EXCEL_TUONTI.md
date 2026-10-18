# RATA- ja KILP-Excel-tuonti

Sovelluksen `excel_import` lukee `.xls`- ja `.xlsx`-tiedostoista RATA- ja KILP-välilehtien tunnistetut kilpailutiedot. Lukeminen muodostaa esikatselun. Käyttöliittymä vahvistaa erikseen uuden kilpailukansion luomisen; lukija ei kirjoita tiedostoja.

## Lähde ja integraatio

Kenttäkartan lähtökohtana on käyttäjän toimittama `AS2026-RATA-KILP-prototype.zip` (SHA-256 `21aa9135a6ced80bd3ab58067081290b5fd2bc263c1cf3e0de043103b31a2b0e`). Erillistä prototyyppisovellusta ei lisätty rinnakkaiseksi toteutukseksi, vaan lukeminen ja tietomallisovitus ovat AS2026:n yhteisessä kirjastossa.

AS2014A:n paikallinen tyhjä `as2014a.xls`-pohja tarkastettiin lukemalla. Sen SHA-256 on `3b78cf2b86f1d8708e5d87be34970321f27ed0bdfcdcc48bc35f3c7da24b4a6d`. Pohjaa tai sen makroja ei jaeta tässä projektissa. Pohjassa on C-, B-, W- ja Q-luokan tyhjät rataryhmät, erillinen MALLIRATA sekä viisi vajavaista kilpailijariviä. Se ei ole valmis kilpailu eikä sitä hyväksytä sellaisenaan tuontiin.

Prototyypin `calamine 0.31.0` kaatui aidon mallipohjan vanhaan kaavatunnisteeseen jo tiedostoa avattaessa. Käyttöön valittiin `calamine =0.36.1`, joka lukee pohjan onnistuneesti. Ylläpidon [muutosloki](https://github.com/tafia/calamine/blob/master/Changelog.md) kuvaa 0.35.0-version korjauksen tuntemattomien XLS-funktiotunnisteiden kaatumiseen. 0.36.1:n Rust-vähimmäisversio 1.88 mahtuu projektin 1.89-vaatimukseen. Oletusominaisuudet on poistettu käytöstä; Excel-asennusta, Pythonia tai erillistä ajonaikaista Excel-kirjastoa ei tarvita.

## Tulkinta

Yleiskatsauksen **Tuo kilpailu Excelistä…** toimii myös kilpailun ollessa avoinna.
Valitse tiedosto, tarkista kilpailija- ja asemataulukot, valitse puuttuvat
luokkien radat ja anna kilpailulle nimi. **Tallennuspaikka…** näyttää sijainnin
ja sallii sen vaihtamisen. Vahvistus luo aina uuden kilpailun omaan vapaaseen
alikansioonsa; saman nimistä kilpailua ei korvata. Peruutus ei kirjoita kilpailua.
Tallennusvirhe säilyttää esikatselun ja ratavalinnat korjaamista varten.

**Tuo vain radat** jättää kilpailijat tuomatta. Muussa tapauksessa puuttuva
Emit tai lähtöaika on valmistelutieto, joka täydennetään myöhemmin. Kilpailijoiden
nimet, luokat, seurat ja autot säilyvät. Puuttuva kortti ei ole fyysinen kortti 0,
eikä puuttuva lähtö tarkoita keskiyötä. Usealla kilpailijalla saa olla nämä
tiedot tyhjinä. Laskenta ja tavallisen tuloksen hyväksyntä edellyttävät kelvollisia
ajanottotietoja. Lähtöluetteloon puuttuvat ajat jätetään tyhjiksi.

- RATA: luokkaryhmä alkaa A-, F-, K-, P-sarakkeista viiden sarakkeen välein. Ryhmän ensimmäinen solu on `Luokka`, toinen luokan tunnus. Rivillä 3 ovat `EMIT no`, `Asemanimi`, `Ihanneaika`/`Ihannea`, `kdi` ja `Jakso`. Täsmälleen samanniminen kilpailijaluokka käyttää ryhmän rataa; muiden luokkien rata valitaan esikatselussa. MALLIRATA ei ole luokkaryhmä. Nimetön ryhmä ohitetaan vain, jos sen kaikki tietosolut ovat tyhjiä.
- Asematyyppi tunnistetaan vain tunnetuista tunnuksista/nimistä: L/Lähtö, M/Maali, ULA, JAT, AT, RT. Tuntematonta asemaa sisältävää rataa ei tuoda osittaisena.
- Tavallinen numeerinen ihanneaika säilyttää vanhan `Min.sek`-merkityksen: `7.28` on 7 min 28 s ja `13` on 13 min. Tekstinä hyväksytään myös `7,28`, `07:28` ja `hh:mm:ss`. Nimenomaisesti kellonajaksi/kestoksi muotoiltu Excel-solu tulkitaan vuorokauden osana. Yli kaksi sekuntinumeroa tai sekuntiosan arvo yli 59 hylätään.
- KDI säilyy sääntönä 0–7 ja Jakso kokonaisina minuutteina. Emit-numero 0 säilyy myös käsin kellotettavilla AT/JAT-asemilla.
- KILP: A numero, B luokka, C Emit, D/E lähtöaika, F kuljettaja, G seura, H kartanlukija, I seura ja J auto. D/E tukee mallipohjan erillisiä tunteja/minuutteja (`17`, `2` → `17:02:00`). D tukee myös tekstikelloa tai Excel-kelloa. Excel-kello tulkitaan ennen erillisten tuntien ja minuuttien tulkintaa; ristiriitainen E-solun minuuttitieto ilmoitetaan.
- Natiiviin kilpailuun tallentuvat myös molempien seuratiedot. Vanhoista kilpailutiedostoista puuttuva seura tarkoittaa tyhjää tietoa. Tietoja ei täydennetä oletetuilla henkilöillä, Emit-numeroilla tai radoilla.
- Ratanumerot 1, 2… ovat uuden kilpailun sisäisiä tunnisteita luokkaryhmien järjestyksessä. Luokkatunnuksia ei muuteta. Tuodun luokan lähtöarvonta on aluksi pois käytöstä; toimitsija voi valita osallistuvat luokat. Kilpailijoiden tuodut lähtöajat säilyvät.
- Työkirja ei sisällä kaikkia AS2026:n kilpailuasetuksia. Luokkien lähtöajan oletus on `00:00:00` ja lähtövälin `1 min`. Arvonta on aluksi pois käytöstä. Luokkavalinta näyttää kellot ja lähtövälit ennen valituille luokille tehtävää erikseen vahvistettavaa arvontaa.

## Ohitettavat ja estävät virheet

Vain pakollisilta tunnistetiedoiltaan viallinen kilpailijarivi tai kokonainen viallinen rata ohitetaan selitteen kanssa. Virheellinen tai keskeneräinen valinnainen Emit-numero tai lähtöaika ei pudota kilpailijaa pois: kortti jää asettamatta tai lähtöaika tyhjäksi, ja puute näkyy esikatselussa. Muut rivit säilyvät esikatselussa. Toistuva kilpailunumero tai rataluokan tunnus estää tuonnin. Toistuvan Emit-numeron kaikilta kilpailijoilta numero jätetään asettamatta, jotta kortinlukua ei kohdisteta väärin. Puuttuva Emit merkitään numerolla 0, puuttuva lähtöaika tyhjällä tekstillä ja puuttuva luokkarata numerolla 0. Ne on täydennettävä ennen kilpailua. KILP- tai RATA-välilehti saa puuttua kokonaan: yksinään olemassa oleva osa voidaan tuoda. Apulehdet ohitetaan eikä niitä tulkita kilpailutiedoiksi.

Pelkkien ratojen tuonnissa KILP-välilehden kilpailijavirheet eivät estä tuontia.
Ratojen omat estävät virheet tarkistetaan silti aina.

Muita välilehtiä ei tulkita eikä niiden sisältöä siirretä kilpailuun. VBA-rajapintaa ei kutsuta, eikä makroja tai kaavoja suoriteta. Excel-lukija voi lukea tiedoston sisäisiä kaava- ja välimuistitietueita formaatin jäsentämiseksi; tuonti käyttää solujen tallennettuja arvoja. Työkirjaa ei lasketa uudelleen, joten ennen tallennusta vanhentuneet kaava-arvot tulee päivittää lähdeohjelmassa.

## Todentaminen

`tests/excel_import.rs` lukee molemmat synteettiset tiedostomuodot ja tarkistaa molemmat välilehdet, seuratiedot, manuaaliset asemat, KDI:n/jaksot, ajan tulkinnan, tunnistekonfliktit, vialliset rivit ja lähdetiedoston muuttumattomuuden. Testit eivät edellytä Exceliä tai Pythonia.

`tests/excel_preparation.rs` tarkistaa valmistelurivien palautumisen, tyhjän
ratapohjan, ristiriitaiset kellot ja sen, etteivät puuttuvat lähtötiedot kaada
tuloslistaa tai muodosta hyväksyttyjä tuloksia. GUI:n `--smoke-test --capture-dir`
käy synteettisen kilpailun läpi tuonnista lähtöarvontaan, leimoihin, käsikelloihin,
pistekorjauksiin, hyväksymiseen, vientiin ja uudelleenavaukseen. Lisäksi se
testaa pelkät radat, ratavalinnan, tuonnin tallennusvirheen sekä perustellun
KESK-päätöksen ilman korttia tai aikoja. Sarjalaitetta ei käytetä.

Hyväksymättömät tulokset pysyvät esikatselussa. HTML-, CSV- ja XLSX-tulosvienti
sisältää vain julkaistavat tulokset ja vahvistetut tilapäätökset. Vahvistettu
KESK ilman ajanottoa ei saa aikoja, pisteitä tai sijoitusta.

Paikallinen mallipohjakoe on erikseen ohitettu tavallisessa testiajossa, koska alkuperäistä työkirjaa ei jaeta:

```sh
AS2026_EXCEL_MALLIPOHJA=/polku/as2014a.xls cargo test --test excel_import -- --ignored
```

Paikallinen valmistelutyökirja tarkistetaan erikseen ilman henkilötietojen
kopiointia testiaineistoon:

```sh
AS2026_EXCEL_VALMISTELU=/polku/as2014a.xlsx cargo test --test excel_preparation -- --ignored --nocapture
```

Kaikkien historiallisten työkirjojen tai vanhan ohjelman kilpailutulosten
täydellistä vastaavuutta ei ole osoitettu.
