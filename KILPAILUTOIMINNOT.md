# Kilpailun tarkastus ja lähtöjärjestelyt

Tässä kuvataan paikallisen main-kehitysversion lisäykset v0.5.7:n päälle.
Niistä ei vielä ole tehty uutta GitHub-julkaisua.

## Tarkista ja Tutki

Tulokset-näkymässä kilpailijan valinta näyttää asemarivit. Valitse asema
korjataksesi sen virhepisteitä. Käsiaika-asemalla voi muuttaa myös kelloa.
Leimatun aseman kello johdetaan kortista, ja lähtökello muutetaan
Kilpailijat-näkymän lähtöajasta.

- **Tarkista…** muodostaa asemarivit uudelleen viimeisestä aktiivisesta
  korttiluvusta. Vahvistus kertoo, että asemarivien käsikorjaukset korvataan.
  Erilliset RT-lisävirheet, muut kilpailijakohtaiset virheet ja käsin valittu
  tila säilyvät.
- **Kellon muuttaminen** tallentaa käsikellon. Se ei muuta toteutunutta
  aikaa eikä virhepisteitä ennen Tutki-toimintoa.
- **Pisteiden muuttaminen** tallentaa asemarivin pisteet ja päivittää summan.
  Aikavirheen ennakko on positiivinen, myöhä negatiivinen. Kokonaissummaan
  lasketaan aikavirheen itseisarvo sekä muut virhepisteet.
- **Tutki** laskee tallennetuista asemariveistä ja käsikelloista.
  Se korvaa asemarivien pistekorjaukset laskennalla.

Tallennettu asematulos säilyy myös uuden korttiluvun jälkeen. Kortinlukuhistoriaa
ei muuteta korjauksilla. Radan muuttaminen ei kohdista vanhoja korjauksia
uusiin asemiin: kyseinen kilpailija jää ilman sijoitusta ja saa huomautuksen,
kunnes toimitsija valitsee Tarkista. Muiden kilpailijoiden tarkastelu jatkuu.

Leimaamattoman maalin Tutki-laskentaa ei vielä tueta Emit-ajanottotavoilla 1 ja 2.
Toiminto ei ole silloin käytettävissä. Kellonaikapohjainen tapa 0 ja Emit-tapojen
leimatut maalit on toteutettu olemassa olevan staattisen tutkimuksen mukaan.
Kilpailun ajanottotapaa ei pidä vaihtaa vain tämän rajoituksen kiertämiseksi.

## Kilpailijan tila ja lisenssit

Kilpailijat-näkymässä ovat alkuperäiset tilat **TRK**, **KESK**, **PER**,
**HYL**, **SULJ**, **ILM** ja **EILÄ** suomenkielisine nimineen.
**Automaattinen** antaa laskennan määrätä tilan. Käsin valittu tila ja sen syy
säilyvät uusissa korttiluvuissa. Syy viedään HTML-, CSV- ja XLSX-tulosluetteloihin.
Puuttuvaa tai vanhentunutta tulosta ei voi julkaista nollapisteisenä TRK/KESK-valinnalla.

Kuljettajan ja kartanlukijan lisenssit tallentuvat erikseen. Sarjapistekelpoisuus
käyttää vanhasta ohjelmasta palautettua X- ja numeromerkkien käsittelyä, myös
merkkijärjestyksen poikkeuksia. Lisenssitieto ei itsessään vaihda kilpailijan tilaa.

## Lähtöajat ja lähtöluettelo

Aseta luokille lähtöaika ja lähtöväli. **Arvo lähtöajat…** löytyy Kilpailijat-
ja Luokat-näkymistä. Esikatselu näyttää kilpailunumerot ja arvotut kellot.
**Peruuta** ei muuta kilpailua; **Hyväksy** korvaa esikatselun lähtöajat yhtenä
tallennuksena. Kilpailunumerot, kortit ja muut kilpailijatiedot säilyvät.

Arvontaan osallistuvat automaattiset/ILM-kilpailijat. Kortin jo luettu tai
tallennetun asematuloksen saanut osallistuja estää arvonnan. Keskeyttäneet,
hylätyt ja muut erikseen asetetut tilat eivät osallistu. Muuttunut kilpailu
mitätöi vanhan esikatselun. Arvonta on luokkakohtainen eikä yritä ratkaista eri
luokkien kesken mahdollisesti samoja lähtökelloja.

**Lähtöluettelo…** tallentaa HTML:n selaimella tulostettavaksi tai CSV:n
jatkokäsittelyyn. Lista on luokkajärjestyksessä ja luokan sisällä lähtöjärjestyksessä,
myös keskiyön yli. Kaikki kilpailijat näkyvät tilatietoineen.

## Aineiston yhteensopivuus

Uudet kentät ovat valinnaisia. Aiemmat kilpailu.toml-tiedostot avautuvat ilman
muunnosta. Vanhojen muutostiedostojen kilpailijamuutokset säilyttävät uudet
lisenssit, tilan, lisävirheet ja tallennetun tuloksen, jos niitä ei ole annettu
muutoksessa. Vanhaa sovellusversiota ei pidä käyttää uuden version kirjoittamien
lisäkenttien muokkaamiseen. Varmuuskopio syntyy tavallisen tallennuksen yhteydessä.

Tarkistukset käyttävät synteettisiä aineistoja. Repositorion aineistoissa ei ole
oikean kilpailun lähtötietoja ja alkuperäisen ohjelman varmennettua tulosluetteloa.
Täydellistä historiallisten tulosten vastaavuutta tai fyysisen lukijan/Windowsin
käytännön varmennusta ei siten väitetä.
