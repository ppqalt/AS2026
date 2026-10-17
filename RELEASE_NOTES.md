# AS2026 0.5.6

Tämä versio laajentaa kilpailujen tuloslaskentaa, luokkien hallintaa ja
tulosten vientiä. Sovelluksen pääkieli on edelleen suomi, ja Linux- sekä
Windows-versiot käyttävät yhteistä Rust-laskentaa.

## Uutta

- Kilpailijan virhepisteet koostuvat viidestä erikseen säilytettävästä
  osasta: asemien aika- ja reittivirheet, puuttuvat RT-asemat, muut virheet
  ja ylimääräiset RT-asemat. RT-asemien määrät ja pistesummat pysyvät erillään.
- Ajanottotavat 0, 1 ja 2 ovat valittavissa, ja leimatun sekä käsin
  kellotetun maalin laskentaa on täydennetty staattisella AS2015A-tutkimuksella.
- Luokkien nousupisteet (Np) ja sarjapisteiden (Sp) menetelmät 0–6,
  mukaan lukien oma Sp 4 -pistetaulukko, tallentuvat kilpailuun.
- Yhdistetyt tulosluokat voi määrittää ja muokata kilpailun asetuksissa.
  Luokan tunnuksen vaihtaminen päivittää myös yhdistettyjen luokkien jäsenet.
- Uudet CSV- ja XLSX-viennit sekä olemassa oleva HTML-tulosraportti jakavat
  saman sijoituslaskennan. Excel-vienti ei sisällä makroja tai kaavoja.
- `as2026 tulokset KANSIO --live 8765` avaa vain omalle koneelle näkyvän
  tulosnäkymän, joka päivittää sivun viiden sekunnin välein.
- Vanhoista AS2015A-tiedostoista tunnistetaan myös K-otsakkeiset kilpailijat
  sekä varhaisempi ennen PE-otsaketta käytetty kilpailun perustietomuoto.
- Leimatun maalin jälkeisille ylimääräisille Emit-leimoille on erillinen
  AS2026:n turvallinen laskentakäytäntö. Maalikellona käytetään varsinaista
  maalileimaa ja jälkileimat säilytetään.

## Rajaukset

AS2015A:sta Ghidralla palautetut piste- ja aikakäytännöt perustuvat
staattiseen kooditutkimukseen ja synteettisiin testeihin. Niiden täydellistä
vastaavuutta alkuperäisen AS2015A:n suorittamiin kilpailutuloksiin ei ole
vahvistettu. Vanhojen tallennettujen P-asemarivien täysi context-1-
uudelleenpisteytys, kaikkien käsikorjausten semantiikka ja osa
poikkeuksellisista tiloista ovat edelleen keskeneräisiä. Maalin jälkeisten
leimojen uusi AS2026-käytäntö ei väitä vastaavansa AS2015A:n toimintaa.

Tuodut vanhat kilpailut avataan edelleen vain luettaviksi. Ohjelma ei korvaa
tiedostoja, joissa on jo käyttäjän tallentama vienti tai kilpailun tiedot.

0.5.6 jaetaan aiempien versioiden tapaan allekirjoitettuina Linux- ja
Windows-paketteina, joissa on sama Ed25519-päivitysten luottamusketju.
