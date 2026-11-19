# Puhelimen ajat Emit-kilpailun väliaikatuloksiin

Emit-kilpailussa AS tulkin käsin syötetyistä ajoista muodostetaan väliaikatulos,
kun kilpailuparille ei vielä ole kelvollista Emit-kortin lukua. Kilpailun
ajanottotapaa ei tarvitse vaihtaa. Asemat kohdistetaan tyypin ja järjestyksen
mukaan kuten paperikorttien ajanotossa.

Väliaikatulos tallennetaan `tulkin_valiaikatulos`-tunnisteella. Sitä ei voi
hyväksyä lopputulokseksi eikä automaattinen hyväksyntä tai tulostus käsittele sitä.
Tuloksessa näkyy tieto puhelimen ajoista. Puuttuva käsiaika on täydennettävä
tieto, ellei tarkastaja ole merkinnyt korttimerkintää puuttuvaksi.

Tulokset-sivun taulukko ja esikatselu näyttävät jo laskettujen puhelinaikojen
pisteet myös keskeneräiselle väliaikatulokselle. Näyttämistä ei sidota
lopputuloksen hyväksymiskelpoisuuteen. Puuttuvan maaliajan paikalla sekä
kokonaiskestossa näkyy viiva, kunnes maaliaika on tiedossa.

Kelvollisen Emit-luvun jälkeen Emit-laskenta on ensisijainen sekä tulosnäkymässä
että tulosta avattaessa. Taustasiirto korvaa väliaikatuloksen Emit-tuloksella.
Puhelimen myöhemmät aikakirjaukset eivät korvaa Emit-aikoja. RT-puutteiden ja
muiden virheiden siirto jatkuu normaalisti.

Ilman Emitiä ajettavan kilpailun aikaisempi käsiajanotto säilyy. Julkaistun
kilpailun siirtyminen Emitistä paperikortteihin sallitaan tiedonsiirrossa;
uudet vaaditut ajat jäävät täydennettäviksi. Vastakkainen muutos vaatii erillisen
ratkaisun, jotta kirjattuja aikoja ei hylätä huomaamatta.

Ketjutesti: `cargo test --features desktop --test tulkki_interim`.
Testi kattaa väliaikatuloksen, livetuloksen, automaattisen hyväksynnän eston,
uudelleenhaun toistettavuuden sekä Emitin ensisijaisuuden kortin lukemisen jälkeen.
