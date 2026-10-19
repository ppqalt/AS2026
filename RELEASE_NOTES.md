# AS2026 0.6.4

- Käyttöliittymäbetan uudistukset ovat nyt mukana vakaassa versiossa. Fluent 2 / Dark Spruce -ulkoasu säilyy, ja taulukoiden, lomakkeiden sekä painikkeiden tilankäyttö on yhtenäistetty.
- Pienissä ikkunoissa Luettelo- ja Tiedot-näkymät antavat sisällölle riittävästi tilaa. Suurissa ikkunoissa taulukko ja rajatun levyinen tietopaneeli näkyvät rinnakkain. Navigaatio ja lomakkeet pysyvät käytettävissä vierittämällä.
- Radat ja asemat -sivulla ratavalinta, asemataulukko ja editorit on järjestetty selkeämmiksi. Ratataulukkoon varataan tila kaikille näkyville riveille. Tulokset ja Emit-lukeminen käyttävät vastaavia mukautuvia työalueita.
- Yleiskatsaus käyttää koko sivun leveyden. Lähtöarvonta on toimintorivin oikealla reunalla, ja arvottavien luokkien valinta avautuu sen viereisestä rataskuvakkeesta.
- Muokattavat kellonajat ja kestot käyttävät yhteisiä tunti-, minuutti- ja sekuntikenttiä. Alkuperäiset tallennusyksiköt, kokonaisminuuttien tarkistukset, tyhjät valinnaiset arvot ja negatiivinen Emit-korjaus säilyvät.
- Excel-tuonnissa voi valita koko kilpailun, vain radat tai vain kilpailijat. Pelkkien kilpailijoiden mukana muodostetaan heidän luokkansa ilman ratoja. Esikatselu, huomautukset ja vahvistuspainike seuraavat valintaa. Tuonti luo uuden kilpailun eikä korvaa avoinna olevaa kilpailua.
- Aiemmat tulosten hyväksynnän, KESK/HYL/PER-tilojen, tallennuksen ja lähtöarvonnan suojaukset säilyvät. Kilpailutiedostojen formaattia tai laskentasääntöjä ei muutettu.

Linuxin käyttöliittymää on kokeiltu synteettisellä kilpailutyönkululla, eri ikkunakoissa ja aidossa koko näytön tilassa. Julkaisutyönkulku tarkistaa ja kokoaa Linux- ja Windows-paketit. Fyysistä Emit-lukijaa tai Windowsin käyttöliittymää ei ole testattu tässä muutoksessa.
