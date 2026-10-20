# AS2026 0.6.5

- Yhdistetyn luokan voi muodostaa valitsemalla olemassa olevat luokat ja antamalla yhteistulokselle nimen.
- NAC Beginners Cup lasketaan samasta kilpailijasta ja Emit-lukemasta omalla asemavalinnalla ja erillisillä virhepisteillä. Maalileiman voi jättää pois: viimeinen JAT toimii silloin BC:n maalina ja sen jälkeinen siirtymä jää laskennan ulkopuolelle.
- Emit-luku avaa käsiaikojen ja virhepisteiden täydennyksen. Tab etenee AT/JAT-ajoista puuttuviin RT-asemiin ja muihin virhepisteisiin. Tallenna ja tulosta on Enterin oletustoiminto; vieressä on pelkkä Tallenna.
- Kilpailijaparin tulosteessa on suuri kokonaistulos, virhepisteiden erittely ja tiivistetyt kilpailijatiedot. AT- ja Myöhä-sarakkeet näyttävät alkuperäisen aikaeron muodossa ±mm:ss ilman virhevapaata vähennystä.
- Luokka- ja asematulosteisiin voi valita yhden tai useamman luokan. Jokainen luokka alkaa omalta sivultaan, oletuksena yhdelle sivulle sovitettuna. Asematuloste on oletuksena vaakasuuntainen. Tulostin valitaan asetuksista.
- Luokkatulosteissa näkyvät ensin lasketut tulokset, sitten AJAA, keskeyttäneet, hylätyt ja peruneet. PER-, KESK- ja HYL-tulostus onnistuu ilman Emit-lukua.

Laskenta ja käyttöliittymä on tarkistettu Linuxissa ja Windowsissa. Näppäimistötyönkulut ja tulosteiden renderöinti on testattu synteettisellä aineistolla. Fyysistä tulostinta tai Emit-lukijaa ei käytetty automaattisissa testeissä.
