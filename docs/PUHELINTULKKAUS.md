# Puhelintulkkauksen toteutuksen tila

Historiallinen tilannekuva 10.11.2026. Nykyinen toteutus ja rajat on kuvattu
tiedostossa [AS_TULKKI_MUUTOKSET.md](AS_TULKKI_MUUTOKSET.md).

10.11.2026. Oikean rivin tuonti ja tallennus on lisätty kehitysversioon.
Puhelinyhteys ei ole vielä valmis.

## Toteutettu lähdekoodiin

`src/interpretation.rs` määrittää paperikortin oikean rivin erilliset
merkintälajit ja Excel-tuonnin esikatselun. Moduuli ei muuta nykyisen
laskennan asemalajeja: oikean rivin L on leima, laskennan L on lähtö.

`preview_workbook` lukee Syotto_-alkuisten välilehtien A-sarakkeet
calamine-kirjastolla suorittamatta makroja. Se ei muuta työkirjaa eikä
kilpailua. Tulos sisältää lähderivin, jakson, normalisoidun tunnuksen ja
merkintälajin. Toistuvat tunnukset säilyvät erillisinä esiintyminä.

JAT ja sitä seuraava ULA kuuluvat päättyvään jaksoon. RT-tunnukset ovat
00–99. Tuntemattomat ja keskeneräiset tunnukset sekä tyhjät välirivit
estävät esikatselun julkaisemiskelpoisuuden. ULA ilman edeltävää JATia ja
maalin jälkeiset merkinnät raportoidaan tarkistettaviksi. Nämä ovat
esikatselun tarkistuksia, eivät uusia kilpailun pistelaskentasääntöjä.

Pysyviä asematunnisteita ei johdeta Excel-rivinumeroista. Ne lisätään
vasta kilpailuun tallennuksen yhteydessä. Nykyiset esikatselun lähderivit
eivät vielä ole rajapinnan asematunnisteita.

## Testaus

`tests/interpretation.rs` sisältää jakorajan, toistuvien RT-tunnusten,
alkunollien, virheellisten rivien, L-merkinnän ja Emitin vaikutuksen kokeet.

```sh
cargo fmt --all -- --check
cargo test --locked --test interpretation
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

Rust/Cargo 1.99.1 sekä rustfmt ja clippy on asennettu Codexin
toolchains-kansioon. Muotoilu ja sen tarkistus onnistuivat 10.11.2026.
Microsoft C++ Build Tools on asennettu ja kääntäminen toimii.
Tulkkausmoduulin kaikki viisi testiä ja ytimen kaikkien kohteiden Clippy
(`-D warnings`) läpäisivät tarkistukset 10.11.2026.

Mallikorttirivi.xlsm luettiin uudella Rust-moduulilla: Syotto_C sisältää
24 tunnistettua merkintää ja viisi keskeneräistä ??-kohtaa riveillä
17–21. Syotto_B sisältää 42 ja Syotto_MA 60 merkintää, ilman
tuontihuomautuksia. Alkuperäistä työkirjaa ei muutettu.

Koko ytimen testiajo ei ole hyväksytty: live-sivujen tallennuskoe
epäonnistui ja erillisellä projektin sisäisellä tilapäiskansiolla
beginners-testien tallennukset palauttivat käyttöoikeusvirheen 5.
Näiden syytä ei ole vielä ratkaistu. GUI-, LAN- ja fyysisten laitteiden
kokeita ei ole suoritettu tällä muutoksella.

## Seuraavat työvaiheet

1. Ratkaise laajemman testiajon tallennusvirheiden syy ja varmista perustestit.
2. Laajenna oikean rivin muutosten käsittely versiointiin ja tulkkausten
   sovittamiseen. Ensituonnin pysyvät tunnisteet ja tallennus ovat mukana.
3. Varmenna uuden oikean rivin tallennuksen LAN-synkronointi kahdella
   koneella. Tallennus käyttää Workspace/ProjectStore-polun olemassa olevaa
   muutosten välitystä.
4. Toteuta tulkkausten vastaanotto, aikarivien kohdistus nykyisiin
   käsikelloihin ja hyväksyttyjen tulosten korjausten käsittely.
5. Toteuta uusi palvelinrajapinta ja puhelinsovellus sekä niiden yhteinen
   varaus- ja kuittausprotokolla.

Nykyisessä ohjelmassa on jo LAN-isäntä ja asiakkaat. Puhelinsynkronoinnin
on kuljettava saman muutosten omistajuuden kautta eikä kilpailutiedostoa
saa kirjoittaa rinnakkaisella tallennusmekanismilla. Usean koneen käyttöä
ei tule toteuttaa erillisinä toisistaan riippumattomina lataajina.

Oikea rivi tallentuu kilpailun oikeat_rivit-kenttään. Tuo oikea rivi
ja Näytä tuonnin tiedot -toiminnot ovat Kilvet-välilehden Toiminnot-valikossa.
Kilvet näyttää radan oikean rivin, jaksot ja kilpihaun. Muuttuneen rivin
korvaamista, puhelinverkkoyhteyttä, puhelinnäkymiä tai laskennan muutoksia
ei ole vielä lisätty. Tarkat käyttöohjeet: EXCEL_TUONTI.md.
