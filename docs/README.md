# AS2026 0.7.1:n pohjalta kehitetyn Fluent 2 -betan ohjeet

Lataus ja asennus: [projektin etusivu](../README.md) ·
[version 0.7.1 julkaisutiedot](../RELEASE_NOTES.md).

> **Fluent 2 -beta on keskeneräinen.** Tällä haaralla oleva [beta-suunnitelma ja koko käyttöliittymän tarkastuslista](FLUENT2_BETA.md) kertoo suunnan ja todetut puutteet. Käyttöliittymäopas ja yhteiset periaatteet on päivitetty betan nykyisten teemojen, presettien ja valintaikkunoiden mukaisiksi. Suunniteltujen materiaalien ja WinUI-vertailun keskeneräisyys on erotettu toteutetuista ominaisuuksista.

## Kilpailun järjestäjälle

| Ohje | Sisältö |
|---|---|
| [Kilpailupäivän tarkistuslista](KILPAILUPAIVA.md) | Varmuuskopio, kalusto, verkko ja lopulliset tulokset |
| [Kilpailun valmistelu ja tuloslaskenta](KILPAILUTOIMINNOT.md) | Käsikellot, pisteet, hyväksynnät ja lähtöarvonta |
| [Kuvallinen käyttöliittymäopas](KAYTTOLIITTYMAOPAS.md) | Sivut, valintaikkunat ja Slint-komponentit |
| [Excel-tuonti](EXCEL_TUONTI.md) | RATA/KILP-kentät, esikatselu ja puuttuvat tiedot |
| [Luokat ja Beginners Cup](LUOKAT_JA_BEGINNERS_CUP.md) | Perusluokat, yhdistetyt luokat ja BC:n erillinen tulos |
| [Emit-leimausten korjaus](EMIT_RIVIN_KORJAUS.md) | Kortin editori, alkuperäiset leimat ja laskennan tarkistus |
| [Tulostus](TULOSTUS.md) | Kilpailija-, luokka- ja asematulosteet |
| [Tulosvienti ja huomautukset](VIENTI_JA_HUOMAUTUKSET.md) | HTML, CSV, XLSX ja tuloksen huomautukset |
| [Lähiverkon yhteiskäyttö](LAHIVERKKO.md) | Liittyminen, samanaikaiset muutokset ja palautuminen |
| [Palomuuriohje](PALOMUURI.md) | Windowsin ja UFW:n rajatut LAN-säännöt |
| [Livetulokset](LIVE_JULKAISU.md) | Paikallinen live, SFTP/FTP ja lopullinen julkaisu |
| [Komentorivi](CLI.md) | Kilpailutiedostot, komennot ja turvallinen tallennus |

## Fluent 2 -betan testattava komponenttigalleria

**[FLUENT2_GALLERIA.md](FLUENT2_GALLERIA.md)** dokumentoi `ui/presets.slint`-kirjaston tämänhetkiset kontrollit ja sen oman `--fluent-gallery`-käynnistysvalinnan. Galleria toimii erillään kilpailudatasta. Ohje erottaa vielä toteuttamattomat ominaisuudet jo lähdekoodiin lisätyistä preseteistä.

## Fluent 2 -betan komponenttistandardi

**[Fluent 2 / Windows 11 -komponenttistandardi ja preset-katalogi](FLUENT2_KOMPONENTTISTANDARDI.md)** on tämän beta-haaran ensisijainen ohje kaikille `ui/**/*.slint`-muutoksille. Se sisältää Microsoftin lähdeviitteet, painike-, slider-, scrollbar-, dialogi- ja muiden kontrollien variantit, ulkoasun mitoista ja behaviorista tehtävät projektipäätökset sekä tiedostokohtaiset toteutussäännöt. **Huom.: kaikki katalogin komponentit eivät ole vielä toteutettuja.** Vakaalla `main`-haaralla ei ole näitä muutoksia.

## Kehittäjälle ja ylläpitäjälle

- [Kääntäminen ja testaus](BUILDING.md)
- [Käyttöliittymän kehitysohje ja yhteiset periaatteet](UI_PERIAATTEET.md)
- [Julkaisuautomaatio, allekirjoitukset ja Verified-tagi](RELEASES.md)
- [Projektin lisenssi](../LICENSE) ja [riippuvuuksien tiedot](../THIRD_PARTY.md)

## Aiemmat versiot

[Historiakansio](historia/README.md) sisältää vanhat julkaisutiedot ja
kehitysvaiheiden raportit. Ne kuvaavat omaa ajankohtaansa; nykyversion
käyttöohjeet ovat yllä. Vanhojen julkaisujen lähteet säilyvät Git-tageissa.
