# AS2026

**AS2026 0.6.10** on suomenkielinen autosuunnistuskilpailujen hallinta- ja
tuloslaskentasovellus Linuxille ja Windowsille. Rust ja Slint, Fluent 2 / Dark Spruce.

## Lataa ja asenna

Lataa oma pakettisi [version 0.6.10 julkaisusivulta](https://github.com/ppqalt/AS2026/releases/tag/v0.6.10):

- Linux x86-64: `AS2026-0.6.10-linux-x86_64.tar.gz`
- Windows x86-64: `AS2026-0.6.10-windows-x86_64.zip`

Pura koko paketti omaan kansioon. Avaa `as2026-gui` tai Windowsissa
`as2026-gui.exe` ja valitse **Asenna ja käynnistä**. Linuxissa ohjelmalla pitää
olla suoritusoikeus. Valmis ohjelma ei tarvitse Rustia eikä erillistä Slint-asennusta.
Pakettien mukana ovat allekirjoitettu manifesti, käynnistin ja riippuvuuksien lisenssit.
Päivitykset tarkistetaan sivupalkin **Päivitykset**-painikkeesta.

## Aloita kilpailu

1. Avaa Yleiskatsauksen **Toiminnot**: avaa kilpailu, luo uusi tai tuo Excelistä.
2. Tarkista kilpailijat, luokat, radat ja asemat. Excel-tuonnissa voit valita
   kilpailijat ja radat erikseen.
3. Valitse Kilpailijat → **Toiminnot → Arvo lähtöajat** ja tarkista lähtöluettelo.
4. Lue kortit Emit-sivulla. Täydennä käsikellot ja virhepisteet sekä tarkasta tulokset.
5. Tulosta tai vie hyväksytyt tulokset Tulokset → **Toiminnot** -valikosta.

Kelvolliset muokkaukset tallentuvat automaattisesti. Kilpailut luodaan normaalisti
Asiakirjat-kansion `AS2026/Kilpailut`-hakemistoon; oletuspaikan voi vaihtaa
Asetuksissa. Ohjelmapäivitys ei siirrä kilpailuja. Kokeile esimerkkiä ennen oikeaa
kilpailua ja pidä kilpailukansiosta erillinen varmuuskopio.

## Mitä sovellus tekee?

- Kilpailijat, luokat, yhdistetyt luokat, radat ja asemat sekä lähtöluettelon sarakevalinta.
- RATA/KILP-Excel-tuonti (`.xls`, `.xlsx`) esikatselulla.
- Jatkuva Emit-luku, lukuhistoria ja aktiivisen kortin leimausten manuaalinen korjaus.
  Alkuperäiset leimat säilyvät jäljitettävinä.
- Käsikellot, kilpailijan tilat, AT/JAT-laskenta, NAC ja Beginners Cup sekä hyväksynnät.
- Luokka-, asema- ja kilpailijatulosteet sekä HTML-, CSV- ja XLSX-tulosvienti.
- Paikallinen offline-käyttö ja kilpailun yhteiskäyttö lähiverkossa.

0.6.10 korjaa koko Toiminnot-valikon, mukaan lukien luokkien lisäämisen ja
vientialavalikon. Mukana ovat keskitetty kilpailijaeditori, neutraalit modaalitaustat,
BC-valinnan päälle/pois-kytkentä sekä asetusten ja rinnakkaispaneelien asettelukorjaukset.
Mikky100:n uusimmat kilpailusäännöt ja laskenta säilyvät. [Julkaisutiedot](RELEASE_NOTES.md).

## Sama kilpailu lähiverkossa

Avaa kilpailu yhdellä koneella ja valitse sivupalkin yhteystila → **Jaa tämä kilpailu**.
Muut koneet liittyvät saman lähiverkon kautta 12-merkkisellä parituskoodilla.
Automaattisen haun vaihtoehtona on isännän IP-osoite. Kaikilla on samat muokkausoikeudet;
muutokset välittyvät automaattisesti ja katkon Emit-luvut säilyvät paikallisessa jonossa.
Internet, pilvipalvelu tai jaettu kansio ei ole tarpeen.

[Yhdistäminen, ristiriidat ja palautuminen](docs/LAHIVERKKO.md) ·
[Windowsin ja UFW:n palomuuri](docs/PALOMUURI.md)

## Ohjeet ja kehitys

- [Kilpailun valmistelu ja tuloslaskenta](KILPAILUTOIMINNOT.md)
- [Excel-tuonti](docs/EXCEL_TUONTI.md), [Emit-korjaus](docs/EMIT_RIVIN_KORJAUS.md),
  [luokat ja Beginners Cup](docs/LUOKAT_JA_BEGINNERS_CUP.md), [tulostus](docs/TULOSTUS.md)
- [Kuvallinen käyttöliittymäopas](docs/KAYTTOLIITTYMAOPAS.md)
- [Kääntäminen ja testaus](BUILDING.md), [komentorivi](CLI.md),
  [UI-kehitys](UI.md), [julkaisuautomaatio](RELEASES.md)

Linux- ja Windows-CI ajavat Rust-/Slint-tarkistukset, GUI-työnkulut 100/150 %:n
skaalauksella sekä valikko- ja Emit-editorikokeet. Fyysisen Emit-lukijan,
paperitulostuksen ja historiallisten AS2015A-tulosten täydellinen vastaavuus
vaatii erillisen laite- ja vertailukokeen; automaattinen koe ei todista niitä.

## Lisenssi

Oma koodi, dokumentaatio ja synteettiset esimerkit: [BSD-2-Clause](LICENSE).
Riippuvuuksilla on omat lisenssinsä: [THIRD_PARTY.md](THIRD_PARTY.md).
Slint-maininta säilyy ohjelman Tietoja-ikkunassa.
