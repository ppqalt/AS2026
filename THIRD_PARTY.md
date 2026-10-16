# Ulkopuoliset kirjastot

AS2026:n [BSD-2-Clause-lisenssi](LICENSE) koskee projektin omaa koodia, dokumentaatiota ja synteettisiä esimerkkejä. Se ei muuta riippuvuuksien lisenssejä. Kirjastojen versiot lukitaan Cargo.lock-tiedostossa; niiden omat lisenssi- ja tekijätiedot säilyvät kyseisissä Cargo-paketeissa. Lähdekoodirepositorio ei sisällä näiden kirjastojen kopioita.

## Slint

GUI käyttää Slint 1.16.0:aa. Linuxin ja Windowsin työpöytäsovelluksen Slint-komponenteille valitaan [Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0](https://slint.dev/agreements/slint-royalty-free-license.pdf). Tämä ei muuta AS2026:n omaa BSD-2-Clause-lisenssiä.

Lisenssi edellyttää Slint-mainintaa jaettavassa sovelluksessa tai sen julkisella verkkosivulla. AS2026 näyttää Slintin alkuperäisen AboutSlint-komponentin sivuvalikon **Tietoja**-ikkunassa. Komponenttia ja pääsyä siihen on säilytettävä tämän Slint-lisenssin mukaista GUI-ohjelmaa jaettaessa.

Slintin riippuvuuksilla ja muilla Cargo-kirjastoilla on omat lisenssinsä. Kun valmiille ohjelmalle tehdään jakelupaketti, siihen liitetään myös mukana jaettavien riippuvuuksien tarvitsemat lisenssitekstit ja tekijämaininnat. Tämän repositorion BSD-2-Clause-lisenssi ei ole lupa poistaa niitä. CLI voidaan kääntää ilman Slintiä ja muita GUI-riippuvuuksia.

BSD-2-Clause:n virallinen teksti: [Open Source Initiative](https://opensource.org/license/bsd-2-clause).
