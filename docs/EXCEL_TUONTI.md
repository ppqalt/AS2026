# RATA- ja KILP-Excel-tuonti

Sovelluksen `excel_import` lukee `.xls`- ja `.xlsx`-tiedostoista RATA- ja KILP-välilehtien tunnistetut kilpailutiedot. Lukeminen muodostaa esikatselun. Käyttöliittymä vahvistaa erikseen uuden kilpailukansion luomisen; lukija ei kirjoita tiedostoja.

## Lähde ja integraatio

Kenttäkartan lähtökohtana on käyttäjän toimittama `AS2026-RATA-KILP-prototype.zip` (SHA-256 `21aa9135a6ced80bd3ab58067081290b5fd2bc263c1cf3e0de043103b31a2b0e`). Erillistä prototyyppisovellusta ei lisätty rinnakkaiseksi toteutukseksi, vaan lukeminen ja tietomallisovitus ovat AS2026:n yhteisessä kirjastossa.

AS2014A:n paikallinen tyhjä `as2014a.xls`-pohja tarkastettiin lukemalla. Sen SHA-256 on `3b78cf2b86f1d8708e5d87be34970321f27ed0bdfcdcc48bc35f3c7da24b4a6d`. Pohjaa tai sen makroja ei jaeta tässä projektissa. Pohjassa on C-, B-, W- ja Q-luokan tyhjät rataryhmät, erillinen MALLIRATA sekä viisi vajavaista kilpailijariviä. Se ei ole valmis kilpailu eikä sitä hyväksytä sellaisenaan tuontiin.

Prototyypin `calamine 0.31.0` kaatui aidon mallipohjan vanhaan kaavatunnisteeseen jo tiedostoa avattaessa. Käyttöön valittiin `calamine =0.36.1`, joka lukee pohjan onnistuneesti. Ylläpidon [muutosloki](https://github.com/tafia/calamine/blob/master/Changelog.md) kuvaa 0.35.0-version korjauksen tuntemattomien XLS-funktiotunnisteiden kaatumiseen. 0.36.1:n Rust-vähimmäisversio 1.88 mahtuu projektin 1.89-vaatimukseen. Oletusominaisuudet on poistettu käytöstä; Excel-asennusta, Pythonia tai erillistä ajonaikaista Excel-kirjastoa ei tarvita.

## Tulkinta

- RATA: luokkaryhmä tunnistetaan Luokka-otsikon perusteella, tavallisesti A-, F-, K-, P-sarakkeista viiden sarakkeen välein. Ryhmän ensimmäinen solu on `Luokka`, toinen luokan tunnus. Rivillä 3 ovat `EMIT no`, `Asemanimi`, `Ihanneaika`/`Ihannea` sekä valinnaiset `kdi` ja `Jakso`. Tyhjä ratalohko ei estä tuontia. Luokan oma ryhmä määrää rata–luokka-yhteyden. MALLIRATA ei ole luokkaryhmä.
- Asematyyppi tunnistetaan vain tunnetuista tunnuksista/nimistä: L/Lähtö, M/Maali, ULA, JAT, AT, RT. Tuntematonta asemaa sisältävää rataa ei tuoda osittaisena.
- Tavallinen numeerinen ihanneaika säilyttää vanhan `Min.sek`-merkityksen: `7.28` on 7 min 28 s ja `13` on 13 min. Tekstinä hyväksytään myös `7,28`, `07:28` ja `hh:mm:ss`. Nimenomaisesti kellonajaksi/kestoksi muotoiltu Excel-solu tulkitaan vuorokauden osana. Yli kaksi sekuntinumeroa tai sekuntiosan arvo yli 59 hylätään.
- KDI säilyy sääntönä 0–7 ja Jakso kokonaisina minuutteina. Emit-numero 0 säilyy myös käsin kellotettavilla AT/JAT-asemilla.
- KILP: otsikot tunnistetaan nimien perusteella ensimmäisten kymmenen rivin joukosta. Vain nro, Lka ja Kuski ovat pakollisia. Muut sarakkeet (Emit, lähtöaika, seurat, kartanlukija, auto) ovat valinnaisia. Tavallinen pohja käyttää A numero, B luokka, C Emit, D/E lähtöaika, F kuljettaja, G seura, H kartanlukija, I seura ja J auto. D/E tukee mallipohjan erillisiä tunteja/minuutteja (`17`, `2` → `17:02:00`). D tukee myös tekstikelloa tai Excel-kelloa, kun E on tyhjä.
- Natiiviin kilpailuun tallentuvat myös molempien seuratiedot. Vanhoista kilpailutiedostoista puuttuva seura tarkoittaa tyhjää tietoa. Tietoja ei täydennetä oletetuilla henkilöillä, Emit-numeroilla tai radoilla.
- Ratanumerot 1, 2… ovat uuden kilpailun sisäisiä tunnisteita luokkaryhmien järjestyksessä. Luokka, jolta puuttuu rata, saa arvon 0, eikä sille luoda kuvitteellisia asemia. Yksinkertainen SM-M voidaan yhdistää olemassa olevaan M-rataan; moniosaisia luokkia ei arvata automaattisesti. Luokkatunnuksia ei muuteta. Tuodun luokan lähtöarvonta on aluksi pois käytöstä; toimitsija voi valita osallistuvat luokat. Kilpailijoiden tuodut lähtöajat säilyvät.
- Työkirja ei sisällä kaikkia AS2026:n kilpailuasetuksia. Uuden kilpailun muut asetukset saavat sovelluksen tavalliset oletukset; tämä kerrotaan esikatselussa.

## Ohitettavat ja estävät virheet

Viallinen kilpailijarivi tai kokonainen viallinen rata ohitetaan selitteen kanssa. Muut rivit säilyvät esikatselussa. Toistuva kilpailunumero tai rataluokan tunnus estää tuonnin. Toistuvan Emit-numeron kaikilta kilpailijoilta numero jätetään asettamatta, jotta kortinlukua ei kohdisteta väärin. Puuttuva Emit merkitään numerolla 0, puuttuva lähtöaika tyhjällä tekstillä ja puuttuva luokkarata numerolla 0. Ne on täydennettävä ennen kilpailua. KILP- tai RATA-välilehti saa puuttua kokonaan: yksinään olemassa oleva osa voidaan tuoda. Apulehdet ohitetaan eikä niitä tulkita kilpailutiedoiksi.

Muita välilehtiä ei tulkita eikä niiden sisältöä siirretä kilpailuun. VBA-rajapintaa ei kutsuta, eikä makroja tai kaavoja suoriteta. Excel-lukija voi lukea tiedoston sisäisiä kaava- ja välimuistitietueita formaatin jäsentämiseksi; tuonti käyttää solujen tallennettuja arvoja. Työkirjaa ei lasketa uudelleen, joten ennen tallennusta vanhentuneet kaava-arvot tulee päivittää lähdeohjelmassa.

## Todentaminen

`tests/excel_import.rs` lukee molemmat synteettiset tiedostomuodot ja tarkistaa molemmat välilehdet, seuratiedot, manuaaliset asemat, KDI:n/jaksot, ajan tulkinnan, tunnistekonfliktit, vialliset rivit ja lähdetiedoston muuttumattomuuden. Testit eivät edellytä Exceliä tai Pythonia.

Paikallinen mallipohjakoe on erikseen ohitettu tavallisessa testiajossa, koska alkuperäistä työkirjaa ei jaeta:

```sh
AS2026_EXCEL_MALLIPOHJA=/polku/as2014a.xls cargo test --test excel_import -- --ignored
```

Molemmat synteettiset tiedostomuodot ja paikallinen alkuperäispohjan koe on ajettu onnistuneesti. Importteria on laajennettu käyttäjän toimittaman täytetyn as2014a.xlsx-rakenteen perusteella. Alkuperäistä työkirjaa henkilötietoineen ei lisätä GitHubiin. Fyysisen Emit-lukijan ja historiallisten tulosten todentaminen ovat edelleen erillisiä testejä.
