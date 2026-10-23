# Kilpailun tarkastus ja lähtöjärjestelyt

Ohje koskee AS2026 0.6.10:tä ja Mikky100:n nykyisiä kilpailusääntöjä.

## Emit-kortista tulokseen

Tunnetun kilpailijaparin onnistunut korttiluku tallentuu alkuperäisine leimauksineen. Sovellus avaa **Tulokset** ja valitsee luetun kilpailijaparin. Asemat muodostuvat luokan radasta, ja leimatuista asemista saadut ajat täyttyvät automaattisesti.

Keskeneräistä lomakemuokkausta ei keskeytetä: korttiluku tallentuu, ja siirtyminen odottaa muokkauksen hyväksymistä tai peruuttamista. Tuntematon kortti säilyy lukuhistoriassa eikä kohdistu toiseen kilpailijaan.

Uusintaluku ei tee kaksoistulosta. Aiemmat tallennetut käsikellot, pistekorjaukset ja hyväksyntä säilyvät. Uuden korttiluvun ottaminen niiden tilalle tapahtuu erikseen **Tarkista…**-toiminnolla ja sen vahvistuksella.

## Käsikellot, tarkastus ja hyväksyntä

Radalla Emit-numerolla **0** merkityt asemat kellotetaan käsin. Lähtökello tulee kilpailijan lähtöajasta; muiden käsiaika-asemien kellot syötetään Tulokset-näkymässä muodossa **hh:mm:ss**. Käsiaika-asemia voi olla useita. Myös käsimaalin kello syötetään erikseen: sitä ei arvata viimeisestä leimasta. Puuttuva käsikello näkyy tyhjänä, ei valmiina nollakellona.

1. Valitse kilpailijapari ja asemarivi. Tarkasta automaattisesti täytetyt leimaukset ja syötä puuttuvat käsikellot.
2. Valitse **Tutki**. Se laskee tallennetuista asemariveistä ja käsikelloista. Kellon muuttaminen tallentuu heti, mutta ei vielä muuta laskettuja aikoja tai pisteitä.
3. Tee tarvittavat asemakohtaiset pistekorjaukset laskennan jälkeen. Pistekorjaus päivittää rivin ja summan. Aikavirheen ennakko on positiivinen ja myöhä negatiivinen; kokonaispisteisiin tulee itseisarvo.
4. Täydennä kilpailijakohtaiset lisävirheet ja tarkasta tila sekä Syy.
5. Valitse **Hyväksy tulos**. Hyväksyntä tallentaa tarkastuksen eikä laske käsin korjattuja pisteitä uudelleen.

**Tutki** korvaa aiemmat asemarivien pistekorjaukset uudella laskennalla. **Tarkista…** puolestaan muodostaa asemarivit uudelleen viimeisestä aktiivisesta korttiluvusta ja korvaa niiden käsikellot sekä pistekorjaukset. Sen vahvistus kertoo korvattavista tiedoista. Erilliset RT-lisävirheet, muut kilpailijakohtaiset virheet ja käsin valittu tila säilyvät.

Puuttuva käsikello, virheellinen aika, muuttuneeseen rataan kuuluva tulos tai ristiriitaiset pistesummat estävät hyväksymisen. Kellon, pisteiden ja muiden tulokseen vaikuttavien tietojen muutos poistaa aiemman hyväksynnän. Radan muuttaminen ei kohdista vanhoja käsikorjauksia uusiin asemiin automaattisesti: muodosta rivit uudelleen Tarkista-toiminnolla ja tarkasta ne.

## Pisteet ja tila

Kokonaispisteissä yhdistyvät viisi osaa: aikavirhepisteet, asemien muut virhepisteet, puuttuvien RT-asemien pisteet, ylimääräisten RT-asemien pisteet ja muut kilpailijakohtaiset virhepisteet.

**Puuttuvat RT-asemat (kpl)** hinnoitellaan kilpailun asetuksella: oletus on 600 pistettä asemalta, joten kolme puuttuvaa RT-asemaa antaa 1 800 pistettä. Näkymässä näkyvät sekä määrä että pistesumma. **Muut virhepisteet** syötetään suoraan pisteinä. Myös ylimääräisten RT-asemien nykyinen käsittely säilyy.

**Automaattinen** tila antaa laskennan määrätä tuloksen:

- **ILM**: tarvittava aika tai muu tarkastustieto on vielä kesken.
- **TRK**: laskenta on kelvollinen eikä keskeytysehtoja ole.
- **KESK**: esimerkiksi vaadittu JAT puuttuu tai jakson tai kilpailun myöhästymisraja ylittyy. Laskenta käyttää kilpailun ja luokan asetuksia. Olennaiset syyt näkyvät Syy-kentässä.

Toimitsija voi valita myös tilat **TRK**, **KESK**, **PER**, **HYL**, **SULJ**, **ILM** ja **EILÄ** suomenkielisine nimineen. Käsin valittu HYL tai PER säilyy uudessa korttiluvussa. Keskeneräisestä tai virheellisestä tuloksesta ei tehdä tarkastettua pelkällä TRK-valinnalla. Tila ja Syy kulkevat HTML-, CSV- ja XLSX-tulosluetteloihin.

Kuljettajan ja kartanlukijan lisenssit sekä seuratiedot tallentuvat erikseen. Lisenssien X- ja numeromerkit vaikuttavat vanhasta ohjelmasta palautetun säännön mukaan sarjapistekelpoisuuteen; ne eivät itsessään muuta kilpailijan tilaa.

## Excel-kilpailun tuonti

Valitse Yleiskatsaus → **Toiminnot → Tuo kilpailu Excelistä**, avaa `.xls`- tai `.xlsx`-tiedosto ja valitse uuden kilpailun kansio. Tarkasta esikatselun määrät, huomautukset ja tallennuspaikka ennen tuonnin vahvistamista. Olemassa olevaa kilpailutiedostoa ei korvata. Kilpailun nimi tulee työkirjan nimestä, ja sen voi vaihtaa asetuksissa.

Tuonti lukee vain tunnistetun **RATA**- ja **KILP**-pohjan: radat ja asemat asetuksineen sekä kilpailijaparit, Emit-numerot, lähtöajat, molempien seurat ja autot. Kelvottomat rivit selitetään; epäselviä luokkia tai ratoja ei arvata. Toistuvat kilpailija- tai Emit-numerot estävät vahvistuksen. Makroja tai kaavoja ei suoriteta eikä lähdetyökirjaa muuteta.

Tuodut luokat eivät aluksi osallistu lähtöarvontaan. Näin KILP-välilehden lähtöajat säilyvät, kunnes toimitsija valitsee luokat arvontaan. Kenttäkartta ja tarkemmat rajat ovat [Excel-tuonnin kuvauksessa](docs/EXCEL_TUONTI.md).

## Lähtöajat ja lähtöluettelo

Aseta luokan lähtöaika ja lähtöväli. Avaa **Kilpailijat → Toiminnot → Arvo lähtöajat**: samassa ikkunassa näkyvät kaikkien luokkien valintaruudut. Voit valita kaikki tai tyhjentää valinnan. **Arvo lähtöajat** tallentaa luokkavalinnat yhdessä ja avaa lähtöaikojen esikatselun; **Peruuta** jättää ne ennalleen. Luokkavalinta ei vielä muuta lähtöaikoja. Sama asetus näkyy luokan **Osallistuu lähtöarvontaan** -kentässä. A- ja M-luokille ei ole kiinteää poikkeussääntöä.

**Arvo lähtöajat** löytyy Kilpailijat-näkymän Toiminnot-valikosta. Esikatselu näyttää kilpailunumerot ja arvotut kellot. **Peruuta** ei muuta kilpailua; **Hyväksy** tallentaa näytetyt lähtöajat yhdessä. Arvonnan ulkopuolisten luokkien ajat sekä kilpailunumerot, kortit ja muut tiedot säilyvät.

Valituista luokista arvotaan automaattiset/ILM-kilpailijat, joiden korttia ei ole luettu eikä asematulosta tallennettu. Jo luetut, asematuloksen saaneet ja muut käsin valitut tilat ohitetaan: heidän lähtöaikansa ja tuloksensa säilyvät, mutta muiden arvonta jatkuu. Jos yhtään arvottavaa ei jää, ohjelma ilmoittaa asiasta. Kilpailun muuttuminen mitätöi vanhan esikatselun.

**Kilpailijat → Toiminnot → Tallenna html-lähtöluettelo** tallentaa selaimella tulostettavan lähtöluettelon. Lähtöluettelon sarakkeet ja järjestys valitaan Asetuksista. HTML on kaikkien luokkien yhteisessä lähtöaikajärjestyksessä; CSV-lähtöluettelo on käytettävissä komentoriviltä.

## Yhteensopivuus ja tarkistusten rajat

Uudet kentät ovat valinnaisia: aiemmat `kilpailu.toml`-tiedostot avautuvat ilman muunnosta. Vanhojen muutostiedostojen pois jättämät lisenssit, seurat, tila, lisävirheet, tallennettu tulos ja luokan arvontavalinta säilyvät. Vanhaa sovellusversiota ei pidä käyttää uuden version lisäkenttien muokkaamiseen. Tavallinen tallennus säilyttää edellisen version varmuuskopiona.

Työnkulkuja testataan synteettisillä tiedostoilla ja käyttöliittymäkokeilla. Oikean kilpailun täydellistä AS2015A-tulosvastaavuutta sekä fyysisen lukijan ja paperitulostuksen varmistamiseen tarvitaan erilliset vertailukokeet. Windowsin automaattiset GUI-työnkulut ajetaan CI:ssä.
