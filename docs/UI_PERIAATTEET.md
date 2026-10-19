# AS2026:n käyttöliittymän yhteiset säännöt

Fluent 2 / Dark Spruce, suomenkielinen työpöytäsovellus. Toteutus säilyy Rustissa
ja Slintissä. Kilpailutietojen tarkistukset ja tallennus kulkevat yhteisen ytimen kautta.

1. **Säätimet:** korkeus 36 px, nimetty painike vähintään 76 px, painikkeiden väli
   8 px. Tavallinen ja ensisijainen painike käyttävät samaa Fluent-komponenttia,
   näppäimistökohdistusta ja käytöstäpoistotilaa. Leveys seuraa tekstiä; vain
   tarkoitukselliset koko rivin toiminnot venyvät. Nimeä toiminto verbillä ilman
   kolmea pistettä. Kuvakepainikkeella on suomenkielinen saavutettava nimi.
2. **Toiminnot:** yksi vihreä ensisijainen toiminto toimintoryhmässä. Toissijaiset
   ovat neutraaleja. Peruuta vasemmalla, vahvistus oikealla, molemmat ryhmänä
   valintaikkunan oikeassa alakulmassa. Kilpailutietoja korvaava toiminto kertoo
   seurauksensa ja säilyttää nykyisen vahvistuksen; älä lisää uusia poistotoimintoja.
3. **Vieritys:** pystyvierityspalkki oikealla omassa 14 px:n kaistassaan,
   näkyvissä tarpeen mukaan. Vaakasuunta vain leveissä taulukoissa, otsikko ja
   rivit yhdessä. Pitkällä lomakealueella yksi pystyvierityksen omistaja.
4. **Valintaikkunat:** yhteinen pinta, 12 px etäisyys ikkunan reunasta ja 24 px
   sisämarginaali. Tavallinen leveys enintään 560 px, Excel-esikatselu 1100 px.
   Otsikko ja toimintorivi pysyvät näkyvissä; pitkä sisältö vierii niiden välissä.
5. **Lomakkeet:** nimi kentän yläpuolella, samat kellokentät ja valinnat kaikkialla.
   Enter tai poistuminen kentästä tallentaa kelvollisen arvon. Tarkka syötevirhe
   näkyy kentän alla; syötettä tai viimeksi tallennettua tietoa ei hukata.
   Uusi valmis tietue tallentuu automaattisesti nykyisen toimintatavan mukaan.
6. **Typografia ja pinnat:** Theme määrää värit, tekstikoot ja 4 px:n väliasteikon.
   Sivun otsikko 22 px, osion otsikko 16 px, perusteksti 14 px, apu- ja
   kenttäteksti 12 px. Paneelin sisämarginaali 16 px, sivun 20 px. Ei uusia
   koristeita, efektejä tai rinnakkaista väripalettia.
7. **Palaute:** alareunassa yksi tiivis tilatieto. Kenttävirheet kuuluvat kenttään,
   tuontivirheet tuontiin ja tuloksen ongelmat kilpailijan tarkasteluun.
   Koko kilpailun huomautukset avataan tarvittaessa, eikä niitä ladota aina näkyviin.
   Tallennus- ja laitevirheitä ei piiloteta tai kuitata onnistumisiksi.
8. **Sivut:** sama otsikko, hakurivi, toimintojen ryhmittely ja työalue.
   Usein tarvittavat toiminnot ovat näkyvissä; tiedostomuodot ja tekniset
   lisätoiminnot omissa nimetyissä valikoissaan. Navigaatio pysyy näkyvissä ja
   vierii lyhyessä ikkunassa. Tuettu vähimmäiskoko on 900 × 600 loogista pikseliä.

Beta-haaran tarkennus: `Workspace` erottaa luettelon ja tiedot, kun molemmille
ei riitä tilaa. Pääalueen rinnakkaisleveys on vähintään 540 px ja tietopaneeli
368–420 px. Päätaulukot saavat vähintään 180 px korkeutta. Alle 440 px korkeassa
työalueessa käytetään yhtä pääsisältöä kerrallaan. Tulosten kahta taulukkoa
ja Emit-historian leimaustietoja ei puristeta pieniksi alataulukoiksi.
Yleiskatsaus käyttää sivun leveyden. Asetusten lukuleveys rajataan 1120 pikseliin. Betan tarkemmat
sivukohtaiset ratkaisut: `KAYTTOLIITTYMA_BETA.md`.

Muutos tarkistetaan koko kilpailun synteettisellä GUI-kokeella, näppäimistöllä,
vierityksellä ja ikkunoiden koon vaihtamisella myös 150 %:n skaalauksella.
Kenttävirheen, tallennusvirheen ja keskeneräisen luonnoksen toiminta erotetaan
toisistaan. Windows-käännös ei yksin todista Windowsin fyysistä käyttökoetta.

Lähtöarvonta sijoitetaan toimintorivin oikeaan reunaan; luokkavalinnan
rataskuvake on välittömästi arvontapainikkeen vieressä. Muokattavat kellot
ja kestot käyttävät yhteistä h : min : s -kenttää. Mallin kokonaisminuutteihin
perustuvissa kestoissa sekuntien on oltava 00. Pistekertoimet säilyvät numeroina.
