# Ulkopuoliset kirjastot

AS2026:n [BSD-2-Clause-lisenssi](LICENSE) koskee projektin omaa koodia, dokumentaatiota ja synteettisiä esimerkkejä. Se ei muuta riippuvuuksien lisenssejä. Kirjastojen versiot lukitaan Cargo.lock-tiedostossa; niiden omat lisenssi- ja tekijätiedot säilyvät kyseisissä Cargo-paketeissa. Poikkeuksena repositorio sisältää alla kuvatun pienen Slintin Fluent-komponenttien paikallisen sovituksen.

## Slint

GUI käyttää Slint 1.18.1:aa. Linuxin ja Windowsin työpöytäsovelluksen Slint-komponenteille valitaan [Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0](https://slint.dev/agreements/slint-royalty-free-license.pdf). Tämä ei muuta AS2026:n omaa BSD-2-Clause-lisenssiä.

Lisenssi edellyttää Slint-mainintaa jaettavassa sovelluksessa tai sen julkisella verkkosivulla. AS2026 näyttää Slintin alkuperäisen AboutSlint-komponentin sivuvalikon **Tietoja**-ikkunassa. Komponenttia ja pääsyä siihen on säilytettävä tämän Slint-lisenssin mukaista GUI-ohjelmaa jaettaessa.

### Paikallinen Fluent-sovitus

`ui/fluent` sisältää AS2026:n käyttämät Slint 1.18.1:n Fluent-komponentit ja niiden välttämättömät riippuvuudet paketista `i-slint-compiler`. Näiden lähteiden ja kuvakkeiden lisenssit säilyvät Slintin omina; AS2026:n työpöytäsovellukseen integroidussa sovituksessa käytetään samaa edellä valittua Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0 -lisenssiä. Tämä ei vaihda projektin omaa lisenssiä GPL:ksi eikä muuta Slint-lähteitä BSD-lisensoiduiksi.

Sovitus käyttää peittäviä teemapintoja ilman varjoja ja liukuvärejä. Valinnaiset animaatiot ovat yhden, oletuksena pois olevan käyttäjäasetuksen takana. Värit on sidottu yhteiseen vaaleaan, tummaan ja Dark Spruce -teemaan ja tekstikentän valikko on suomennettu. Slintin tunnuskuvat säilyvät muuttamattomina. Lähdeversio, muutokset ja päivitysohjeet ovat tiedostossa `ui/fluent/README.md`; alkuperäisten tiedostojen tiivisteet ovat tiedostossa `ui/fluent/UPSTREAM.sha256`.

Alkuperäiset tekijämaininnat on säilytetty lähteissä:

- Copyright © SixtyFPS GmbH <info@slint.dev>
- Copyright © 2025 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>, author Nathan Collins <nathan.collins@kdab.com>

Alkuperäiset Slint-lisenssivaihtoehdot ja tunnuskuvien CC-BY-ND-4.0-lisenssi ovat hakemistossa `ui/fluent/licenses`. Julkaisupaketin olemassa oleva lisenssikerääjä sisällyttää samat tekstit Slint 1.18.1:n Cargo-pakettien `LICENSES`-hakemistoista hakemistoon `dependency-licenses`; tämä tekijä- ja muutosmaininta kulkee mukana tiedostossa `THIRD_PARTY.md`.

Slintin riippuvuuksilla ja muilla Cargo-kirjastoilla on omat lisenssinsä. Kun valmiille ohjelmalle tehdään jakelupaketti, siihen liitetään myös mukana jaettavien riippuvuuksien tarvitsemat lisenssitekstit ja tekijämaininnat. Tämän repositorion BSD-2-Clause-lisenssi ei ole lupa poistaa niitä. CLI voidaan kääntää ilman Slintiä ja muita GUI-riippuvuuksia.

BSD-2-Clause:n virallinen teksti: [Open Source Initiative](https://opensource.org/license/bsd-2-clause).
