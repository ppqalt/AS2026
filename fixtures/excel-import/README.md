# Synteettiset Excel-testitiedostot

`synteettinen.xls` ja `synteettinen.xlsx` sisältävät samat täysin keksityt RATA- ja KILP-taulukot sekä ylimääräisen ÄLÄ_TUO-välilehden. Tiedostoissa ei ole oikeita henkilötietoja, makroja, kaavoja tai ulkoisia linkkejä.

Molemmat valmiit testityökirjat ovat mukana versionhallinnassa. Niitä käytetään suoraan Rustin `tests/excel_import.rs`-testeissä. Testien suorittaminen, AS2026:n rakentaminen ja Excel-tuonti eivät tarvitse Pythonia tai erillistä testitiedostojen generaattoria.

Excel-tuonti on toteutettu Rustilla tiedostossa `src/excel_import.rs` käyttäen Rustin `calamine`-kirjastoa. `.xls`- ja `.xlsx`-tiedostoja ei käsitellä Pythonilla.
