# Synteettiset Excel-testitiedostot

`synteettinen.xls` ja `synteettinen.xlsx` sisältävät samat täysin keksityt RATA- ja KILP-taulukot sekä ylimääräisen ÄLÄ_TUO-välilehden. Tiedostoissa ei ole oikeita henkilötietoja, makroja, kaavoja tai ulkoisia linkkejä.

XLS on luotu kehitysvaiheessa `xlwt 1.3.0`:lla; XLSX on tavallinen ZIP/XML-työkirja. Sovellus tai testien suorittaminen ei tarvitse näitä generaattorin riippuvuuksia.

Testiaineiston uudelleenluonti tässä hakemistossa:

```sh
uv run --no-project --with xlwt==1.3.0 generate.py
```

Generaattori korvaa vain nämä kaksi synteettistä testitiedostoa. Käyttäjän alkuperäinen AS2014A-työkirja ei kuulu testiaineistoon.
