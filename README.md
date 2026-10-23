# AS2026

**AS2026 0.7.0** on suomenkielinen autosuunnistuskilpailujen hallinta- ja
tuloslaskentasovellus Linuxille ja Windowsille. Se toimii yhdellä koneella ilman
internetiä tai usean toimitsijan yhteisenä kilpailuna lähiverkossa.

## Lataa ja asenna

Lataa oma pakettisi [version 0.7.0 julkaisusivulta](https://github.com/ppqalt/AS2026/releases/tag/v0.7.0):

- Linux x86-64: `AS2026-0.7.0-linux-x86_64.tar.gz`
- Windows x86-64: `AS2026-0.7.0-windows-x86_64.zip`

Pura **koko paketti** omaan kansioon. Avaa `as2026-gui` tai Windowsissa
`as2026-gui.exe` ja valitse **Asenna ja käynnistä**. Linuxissa ohjelmalla pitää
olla suoritusoikeus. Valmis sovellus ei tarvitse Rustia eikä erillistä Slint-asennusta.
Paketit sisältävät allekirjoitetun tiedostoluettelon, käynnistimen ja riippuvuuksien
lisenssit. Päivitykset tarkistetaan sivupalkin **Päivitykset**-painikkeesta.

Pidä kaikilla saman kilpailun LAN-koneilla sama versio. Päivitä ja kokeile laitteet
ennen kilpailua; ohjelmapäivitys ei siirrä tai poista kilpailutiedostoja.

## Kilpailu alusta tuloksiin

1. Avaa Yleiskatsauksen **Toiminnot**: avaa kilpailu, luo uusi tai tuo Excelistä.
2. Tarkista kilpailijat, luokkien radat, asemat, Emit-kortit ja kilpailuasetukset.
   Excel-tuonnissa kilpailijat ja radat voi valita erikseen.
3. Valitse Kilpailijat → **Toiminnot → Arvo lähtöajat** ja tarkista lähtöluettelo.
4. Lue kortit Emit-sivulla. Täydennä käsikellot ja virhepisteet, tarkasta ja hyväksy
   tulokset. BC-osallistujan NAC-tallennusta seuraa erillinen Beginners Cupin syöttö.
5. Tulosta tai vie luokka- ja asematulokset Tulokset-sivun toiminnoista. Tarkista
   lopullinen luettelo ja julkaise livetulokset lopullisina vasta kilpailun valmistuttua.

Kelvolliset muutokset tallentuvat automaattisesti. Kilpailut luodaan normaalisti
Asiakirjat-kansion `AS2026/Kilpailut`-hakemistoon; oletuspaikan voi vaihtaa
Asetuksissa. [Kilpailupäivän tarkistuslista](docs/KILPAILUPAIVA.md) auttaa varmistamaan
varmuuskopion, kortinluvun, tulostuksen ja verkon ennen ensimmäistä lähtöä.

## Toiminnot

- Kilpailijat, luokat, yhdistetyt luokat, radat ja asemat sekä lähtöluettelon sarakevalinta.
- RATA/KILP-Excel-tuonti (`.xls`, `.xlsx`) esikatselulla ja erillisillä tuontivalinnoilla.
- Jatkuva Emit-luku, lukuhistoria ja aktiivisen kortin leimausten korjaus.
  Alkuperäiset leimat säilyvät jäljitettävinä.
- Käsikellot, kilpailijan tilat, AT/JAT, NAC ja Beginners Cup sekä erilliset hyväksynnät.
- Luokka-, asema- ja kilpailijatulosteet sekä HTML-, CSV- ja XLSX-tulosvienti.
- Lähiverkon yhteiskäyttö, paikallinen live-esikatselu ja SFTP/FTP-livetulokset.

## Yhteiskäyttö lähiverkossa

Avaa kilpailu isäntäkoneella ja valitse sivupalkin yhteystila → **Jaa tämä kilpailu**.
Muut koneet liittyvät saman lähiverkon kautta 12-merkkisellä parituskoodilla.
Automaattisen haun vaihtoehtona on isännän IP-osoite. Kaikilla on samat muokkausoikeudet.
Muutokset välittyvät automaattisesti, ja yhteyskatkon Emit-luvut säilyvät paikallisessa
jonossa. Internetiä, pilvipalvelua tai jaettua kansiota ei tarvita.

[Yhdistäminen, ristiriidat ja palautuminen](docs/LAHIVERKKO.md) ·
[Windowsin ja UFW:n palomuuri](docs/PALOMUURI.md)

## Livetulokset

Tulokset → **Toiminnot → Live-tulosten julkaisu** antaa valita julkaistavat luokat.
Paikallinen live päivittää sivut omalle koneelle. Palvelinjulkaisu siirtää samat
luokkakohtaisesti kiertävät sivut SFTP:llä tai FTP:llä. Valitse vain yksi julkaiseva
kone kilpailua kohti. [Käyttöönotto ja palvelimen vaatimukset](docs/LIVE_JULKAISU.md).

## Ohjeet

- [Kaikki käyttö- ja kehitysohjeet](docs/README.md)
- [Kilpailupäivän tarkistuslista](docs/KILPAILUPAIVA.md)
- [Kilpailun valmistelu ja tuloslaskenta](docs/KILPAILUTOIMINNOT.md)
- [Kuvallinen Slint-käyttöliittymäopas](docs/KAYTTOLIITTYMAOPAS.md)
- [Version 0.7.0 julkaisutiedot](RELEASE_NOTES.md)

Linux- ja Windows-CI tarkistavat Rust-/Slint-käännöksen, laskennan ja natiivin GUI:n
synteettisellä aineistolla. Fyysisen Emit-lukijan, oikean kilpailuverkon ja
paperitulostimen toiminta tarkistetaan aina käytettävällä kilpailukalustolla.
Automaattikokeet eivät todista laitteiston toimintaa tai täydellistä vastaavuutta
historiallisiin AS2015A-tuloksiin.

## Lisenssi

Oma koodi, dokumentaatio ja synteettiset esimerkit: [BSD-2-Clause](LICENSE).
Riippuvuuksilla on omat lisenssinsä: [THIRD_PARTY.md](THIRD_PARTY.md).
Sovellus on toteutettu Rustilla ja Slintillä, Fluent 2 / Dark Spruce -ulkoasulla.
Slint-maininta säilyy ohjelman Tietoja-ikkunassa.
