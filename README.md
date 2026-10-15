# AS2015 — autosuunnistuksen tulostyökalu

Suomenkielinen Rust-sovellus, jonka pohjana on vanhan AS2015A-ohjelman staattinen tutkimus. Käyttöliittymän tutut termit ja asemahuomiot on poimittu alkuperäisestä ohjelmasta. Uusille esikatselu- ja tarkistustoiminnoille on kirjoitettu suomenkieliset ohjeet.

Tämä on vielä kehitysversio. Vanhaa Windows-ohjelmaa ei ole suoritettu. Uuden laskennan vastaavuus oikeisiin vanhoihin kilpailutuloksiin on vielä varmistettava.

## Käynnistäminen

Linux-versio on tiedostossa `bin/as2015-rs`. Avaa pääte tämän kansion kohdalle ja käynnistä:

```sh
./bin/as2015-rs
```

Saat numeroidun valikon. Ohjelma pyytää kilpailukansion ja opastaa tarvittavissa valinnoissa. Vanhoille tiedostoille sopii oletusarvoinen merkistö. Voit ensin valita **5 — Kokeile esimerkkikilpailua** ilman omia tiedostoja.

Valikon toiminnot:

1. **Tarkastus** näyttää kilpailun tiedot, radat, kilpailijat ja tallennetut pisteet.
2. **Tuloslaskenta** laskee esikatselun `emit.txt`-korttiaineistosta ja näyttää asemakohtaiset virhepisteet sekä laskennan huomautukset.
3. **Tulosluettelot** näyttää luokka-, yhdistettyjen luokkien ja ratakohtaiset sijoitukset tallennetuista pisteistä. Nousu- ja sarjapisteet noudattavat kilpailun valintoja.
4. **Emit-kehystiedoston purku** lukee yhden 217 tavun tiedoston. Se ei vielä lue laitteesta.
5. **Esimerkkikilpailu** näyttää koko toteutetun korttilaskennan. Käsin laaditun esimerkin tulos on 645 pistettä.

Tuloslaskennan ja tulosluettelon jälkeen voit tallentaa raportin HTML-tiedostoksi. Sen voi avata tavallisella selaimella ja tulostaa selaimen tulostustoiminnolla. Verkkoa tai erillistä palvelinta ei tarvita. Olemassa olevaa tiedostoa ei korvata.

## Linux ja Windows

Yhteinen lähdekoodi käyttää vain Rustin vakiokirjastoa: kolmansien osapuolten kirjastoja ei ole. Valmis ohjelma ei tarvitse Rustia, Pythonia, Javaa tai Ghidraa. Raportin katseluun tarvitaan selain.

Linux-julkaisu on käännetty ja kokeiltu tällä CachyOS-koneella. Se käyttää tavallisia käyttöjärjestelmän kirjastoja; toimintaa kaikissa vanhoissa Linux-jakeluissa ei ole vielä varmistettu.

Windowsissa ohjelma käännetään omaksi `as2015-rs.exe`-tiedostokseen. Windowsin C-ajonaikaiskirjasto liitetään ohjelmaan staattisesti. Lähdekoodin mukana on sekä Windowsin että Linuxin automaattinen tarkistusohje työnkulussa, mutta Windows-käännöstä ja käyttöä ei ole vielä testattu tällä koneella. Windows-valmisohjelmaa ei siksi ole tässä paketissa. Tarkemmat käännösohjeet: [Rakentaminen](BUILDING.md).

## Kilpailuaineisto

Kansion tulee sisältää `Aspara.txt`, `Asperus.txt`, `Radat.txt` ja `Kilp.txt`. Korttilaskenta tarvitsee myös `emit.txt`-tiedoston. Sarjapisteiden oma taulukko, eli valinta Sp 4, tarvitsee `sarjap.txt`-tiedoston.

Tiedostonimien kirjainkoko ei vaikuta hakuun. Moniselitteiset samannimiset tiedostot hylätään. Lähdetiedostoja ei muuteta. Alkuperäiset tavut säilyvät tuodussa aineistossa; esikatselu ja tallennetut tulokset pidetään erillään.

Tasapisteet säilyvät oletuksena jaettuina. Asematietoihin perustuvan ratkaisun voi pyytää erikseen, mutta se tehdään vain riittävillä ja samoihin asemiin kohdistetuilla tiedoilla. Avoimet tarkastustulokset estävät nousu- ja sarjapisteiden vahvistamisen.

## Komentojen käyttäminen

Valikon lisäksi käytettävissä ovat seuraavat komennot:

```sh
./bin/as2015-rs tarkista fixtures/synthetic
./bin/as2015-rs laske fixtures/synthetic --vaiheet --raportti laskenta.html
./bin/as2015-rs tulokset fixtures/synthetic --raportti tulokset.html
./bin/as2015-rs pura fixtures/synthetic/emit-frame.raw 12:11:00
./bin/as2015-rs esimerkki
./bin/as2015-rs ohje
```

Lisävalinta `--utf8` valitsee UTF-8-merkistön. Oletus on Windows-1252. Kellonaika annetaan muodossa `12:11:00`. Välilyöntejä sisältävä kansion polku laitetaan komentoa käytettäessä lainausmerkkeihin.

Komennon `tulokset` lisävalinta `--ratkaise-tasatulokset` ottaa käyttöön edellä kuvatun erillisen tasatulosvertailun. Laskennan esikatselua ei siirretä automaattisesti tallennettujen tulosten tilalle.

## Toteutettu laskenta ja rajat

Toteutettuna ovat Emit-kehyksen ja tallennetun korttiluvun purku, leimausten kohdistus radalle, AT/JAT/ULA-jaksolaskenta, puuttuvien ja ylimääräisten asemien virheet, ULA-pyöristykset, enimmäissakot, myöhästymisrajat ja säännön 4 käsittely. Tulosluettelossa ovat luokka- ja ratakohtaiset sijoitukset, yhdistetyt luokat, osallistumistilasto sekä nousu- ja sarjapisteet.

Esikatselun yhteissumma sisältää vain asemarivien virhepisteet. Kilpailijalle erikseen kirjattuja RT-lisäyksiä ja muita virhepisteitä ei vielä yhdistetä siihen. Tallennettu kokonaistulos näytetään erillisenä vertailuarvona.

Korttilaskenta on rajattu ajanottotapoihin 0 ja 1. Ajanottotavan 0 tässä toteutettu polku käyttää korttiaineistoa; käsin syötettyjen asemakellojen uudelleenlaskenta ei ole vielä käytettävissä. Ajanottotapa 2, suora sarjaportin lukeminen, kilpailutietojen muokkaus, lähtöaikojen arvonta ja varsinainen graafinen käyttöliittymä ovat vielä toteuttamatta.

Vanhan ohjelman ristiriitaisia kohtia ei peitetä. Esimerkiksi käsimaalin johtaminen viimeisestä leimasta ilmoitetaan laskennan huomautuksena. Epäselvä tai vielä tukematon laskentapolku tuottaa ilmoituksen tuloksen arvaamisen sijaan. Kokonaisvastaavuutta vanhaan ohjelmaan ei väitetä.

## Tarkistukset

```sh
cargo fmt --all -- --check
cargo test --offline --locked --all-targets
cargo clippy --offline --locked --all-targets -- -D warnings
cargo build --offline --locked --release
```

Testiaineisto on käsin laadittua. Testit tarkistavat palautetun laskentamallin ja tiedostojen käsittelyn; ne eivät korvaa vertailua oikeaan kilpailuaineistoon ja tunnettuihin tuloksiin.
