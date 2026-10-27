# AS2026:n käyttöliittymän kehitysohje

Fluent 2, suomenkielinen työpöytäsovellus. Sovellus tarjoaa järjestelmätilan, vaalean ja tumman perusulkoasun sekä [7 tummaa väriteemaa](TEEMAT.md). Toteutus säilyy Rustissa
ja Slintissä. Kilpailutietojen tarkistukset ja tallennus kulkevat yhteisen ytimen kautta.

> **Fluent 2 -betan kehittäjälle:** uusi [komponenttistandardi ja esiasetuskatalogi](FLUENT2_KOMPONENTTISTANDARDI.md) on beta-haaran ensisijainen lähde mitoille, varianteille ja vuorovaikutusmalleille. Tämä ohje kuvaa betan yhteisiä toimintarajoja; tarkat presetit ja toteutuksen rajaus löytyvät komponenttistandardista. Ristiriidoissa komponenttistandardi voittaa beta-haarassa. `main` ei muutu.

## Arkkitehtuuri

`ui/app.slint` kokoaa sivupalkin, seitsemän pääsivua, Toiminnot-valikot ja
valintaikkunat. `ui/presets.slint` tarjoaa yhteiset kontrollit. `ui/components.slint` sisältää yhteiset taulukot, editorit ja
paneelit. `ui/theme.slint` omistaa Fluent 2 / Tumma kuusi -mitat ja värit.
Paikallinen Fluent-sovitus ja sen lisenssit ovat `ui/fluent`-hakemistossa.

`src/gui` sitoo Slintin tapahtumat yhteiseen Rustin Workspace-, tallennus- ja
laskentakoodiin. Slint ei toteuta rinnakkaista pistelaskentaa tai LAN-protokollaa.
Säilytä Mikky100:n kilpailusäännöt ja tietomallit; tarkista ajantasainen main
ennen muutosta. Näyttöasetusten muutos ei saa poistaa hyväksyntöjä.

## Tallennus ja editorit

Uusi tietue tallentuu pakollisten kenttien valmistuttua. Virheellinen luonnos
säilyy korjattavana; tallennusvirhettä ei esitetä onnistumisena.
Kilpailijan muokkaus on Tulokset-sivulla keskitetty modaali. Valmis, Esc ja
sama muokkauskutsu sulkevat sen vain validoinnin salliessa. Uudet Emit-luvut
tallentuvat, mutta eivät vaihda aktiivisen editorin kilpailijaa kesken muokkauksen.
Koko kortin leimat korjataan Emit-sivun **Muokkaa leimauksia** -toiminnolla.

## Yhteiset säännöt

1. **Säätimet:** korkeus 36 px, nimetty painike vähintään 76 px, painikkeiden väli
   8 px. Tavallinen ja ensisijainen painike käyttävät samaa Fluent-komponenttia,
   näppäimistökohdistusta ja käytöstäpoistotilaa. Leveys seuraa tekstiä; vain
   tarkoitukselliset koko rivin toiminnot venyvät. Nimeä toiminto verbillä ilman
   kolmea pistettä. Kuvakepainikkeella on suomenkielinen saavutettava nimi.
2. **Toiminnot:** yksi teeman korostusväriä käyttävä ensisijainen toiminto toimintoryhmässä. Toissijaiset
   ovat neutraaleja. Peruuta vasemmalla, vahvistus oikealla, molemmat ryhmänä
   valintaikkunan oikeassa alakulmassa. Kilpailutietoja korvaava toiminto kertoo
   seurauksensa ja säilyttää nykyisen vahvistuksen; älä lisää uusia poistotoimintoja.
3. **Vieritys:** pystyvierityspalkki oikealla omassa 14 px:n kaistassaan,
   näkyvissä tarpeen mukaan. Vaakasuunta vain leveissä taulukoissa, otsikko ja
   rivit yhdessä. Pitkällä lomakealueella yksi pystyvierityksen omistaja.
4. **Valintaikkunat:** yhteinen pinta, 12 px etäisyys ikkunan reunasta ja 24 px
   sisämarginaali. Tavallinen leveys enintään 560 px, Excel-esikatselu 1100 px.
   Korkeus seuraa sisällön omaa mittaa näkymäkohtaisen vähimmäis- ja enimmäiskorkeuden
   välillä. Pitkän editorin yläraja on 1200 px ja aina sovellusikkunaa pienempi.
   Otsikko ja toimintorivi pysyvät näkyvissä; pitkä sisältö vierii niiden välissä.
5. **Lomakkeet:** nimi kentän yläpuolella, samat kellokentät ja valinnat kaikkialla.
   Enter tai poistuminen kentästä tallentaa kelvollisen arvon. Tavallisen
   muokkauskentän Enter poistaa kentän kohdistuskorostuksen vasta kelvollisen
   tallennuksen jälkeen. Erillisten toimintojen, kuten LAN-liittymisen, omat
   Enter-kutsut säilyvät. Tarkka syötevirhe
   näkyy kentän alla; syötettä tai viimeksi tallennettua tietoa ei hukata.
   Uusi valmis tietue tallentuu automaattisesti nykyisen toimintatavan mukaan.
6. **Typografia ja pinnat:** Theme määrää värit, tekstikoot ja 4 px:n väliasteikon.
   Sivun otsikko 22 px, osion otsikko 16 px, perusteksti 14 px, apu- ja
   kenttäteksti 12 px. Paneelin sisämarginaali 16 px, sivun 20 px. Valinnaiset häivytykset ja värisiirtymät noudattavat yhteistä tehosteasetusta. Ei rinnakkaista väripalettia.
7. **Palaute:** alareunassa yksi tiivis tilatieto. Kenttävirheet kuuluvat kenttään,
   tuontivirheet tuontiin ja tuloksen ongelmat kilpailijan tarkasteluun.
   Koko kilpailun huomautukset avataan tarvittaessa, eikä niitä ladota aina näkyviin.
   Tallennus- ja laitevirheitä ei piiloteta tai kuitata onnistumisiksi.
8. **Sivut:** sama otsikko, hakurivi, toimintojen ryhmittely ja työalue.
   Usein tarvittavat toiminnot ovat näkyvissä; tiedostomuodot ja tekniset
   lisätoiminnot omissa nimetyissä valikoissaan. Navigaatio pysyy näkyvissä ja
   vierii lyhyessä ikkunassa. Tuettu vähimmäiskoko on 900 × 600 loogista pikseliä.

Työalueen asettelu: `Workspace` erottaa luettelon ja tiedot, kun molemmille
ei riitä tilaa. Pääalueen rinnakkaisleveys on vähintään 540 px ja tietopaneeli
368–420 px. Päätaulukot saavat vähintään 180 px korkeutta. Alle 440 px korkeassa
työalueessa käytetään yhtä pääsisältöä kerrallaan. Tulosten kahta taulukkoa
ja Emit-historian leimaustietoja ei puristeta pieniksi alataulukoiksi.
Yleiskatsaus käyttää sivun leveyden. Asetusten lukuleveys rajataan 1680 pikseliin; leveässä ikkunassa kentät jakautuvat tasalevyisiin palstoihin. Nykyiset sivukohtaiset
ratkaisut ja komponentit: [käyttöliittymäopas](KAYTTOLIITTYMAOPAS.md).

Muutos tarkistetaan koko kilpailun synteettisellä GUI-kokeella, näppäimistöllä,
vierityksellä ja ikkunoiden koon vaihtamisella myös 150 %:n skaalauksella.
Kenttävirheen, tallennusvirheen ja keskeneräisen luonnoksen toiminta erotetaan
toisistaan. Windows-käännös ei yksin todista Windowsin fyysistä käyttökoetta.

Lähtöarvonta ja sen luokkavalinta avataan Kilpailijat-sivun Toiminnot-valikosta. Muokattavat kellot
ja kestot käyttävät yhteistä h : min : s -kenttää. Mallin kokonaisminuutteihin
perustuvissa kestoissa sekuntien on oltava 00. Pistekertoimet säilyvät numeroina.

Vientivalikkojen pelkkää alkukohdistusta tekevä FocusScope ei saa peittää
painikkeiden hiirialuetta. Valikko suljetaan valinnan jälkeen; sisäpuolinen
napsautus ei saa kadottaa toimintoa. Tulosviennillä on näkyvä tallennusikkuna
ja paikallinen virhepalaute myös ilman toimivaa työpöydän tiedostovalitsinta.
Huomautuksissa oletus on ryhmitelty yhteenveto, tarkat tiedot erikseen.

Pitkän tuloslomakkeen näppäimistökohdistus vierittää valitun kentän näkyviin.
Tab ja Shift+Tab kiertävät kentät sekä alapainikkeet näkyvässä järjestyksessä.
Näppäimistöllä kenttään siirryttäessä vanha arvo valitaan korvattavaksi.

Tulostuksen luokkavalinnassa käytetään samoja joukkovalintoja kuin lähtöarvonnassa.
Pitkän luokkalistan kaikki rivit ovat saavutettavissa näppäimistöllä myös
vieritysalueen ulkopuolelta; kohdistus vierittää rivin näkyviin.

Arvonnan luokkalista käyttää ChoiceRow-riviä. Tulostuksen ja live-julkaisun
luokkavalinta käyttää yhteistä ClassSelection-komponenttia: kaksi koko alaltaan
klikattavaa korttia rivillä, haku, valittujen määrä ja Vain valitut -rajaus.
Suodatus säilyttää alkuperäisen valintamallin indeksit eikä muuta piilossa olevia valintoja.
Joukkovalintapainikkeet ja valintojen yhteenveto pysyvät listan ulkopuolella
näkyvissä myös vieritettäessä; vain luokkalista vierii.

Yhdistetyn luokan ikkunassa vain luokkalista vierii; nimi, pisteasetukset,
virhepalaute ja alapainikkeet pysyvät näkyvissä. Myös BC:n tallennusvirhe
näytetään listan ulkopuolella tallennuspainikkeen lähellä.
Valintarivin ajastettu kohdistus tarkistaa pyynnön ajantasaisuuden ennen
kohdistamista, jotta vanha pyyntö ei vie kohdistusta nimikentästä.

BC:n tiedot/asemat ja osallistujat erotetaan välilehdille. Osallistujalistalla
on oma vieritysalue ja sama valintarivi kuin luokkavalinnoissa. Nopean
Tab-siirtymisen tavoite pidetään erillään viiveellä saapuvasta kohdistuspalautteesta,
jotta vanha kohdistus ei palauta listaa aiemmalle riville.

Lähiverkkoyhteys avataan Mikky100:n sijoittelun mukaisesti sivupalkin yhteystilasta. Tila kertoo yhteyden ja jonon ilman
pysyviä ohjetekstejä sivuilla. Etäpäivitys ei korvaa avoimen lomakkeen syötettä,
kohdistusta tai muokkauspohjaa. Muokkausristiriita näkyy kyseisessä lomakkeessa;
yhteysvirhe yhteysikkunassa. Yhteyskatkoa ei koskaan merkitä ajan tasalla olevaksi.

Valintaikkunan tausta on neutraali läpinäkyvä musta (`#00000080`, Slintin
RRGGBBAA-muoto). Taustaa ei sumenneta eikä kuvata. Kilpailijan muokkaus
avataan keskitettyyn ikkunaan; sama painike ja Esc sulkevat sen.

Ehdollisen popup-painikkeen toiminto suoritetaan ennen popupin sulkemista.
Toisen popupin avaaminen välitetään pääikkunan pysyvään käsittelijään, joka
sulkee ensin vanhan valikon ja avaa sitten uuden. Sulkemisen jälkeen tuhoutuvan
painikkeen vanhempiviittausta ei saa käyttää. Kokeile aina oikeaa valikkoketjua.

Komponentit, nykyinen sivurakenne ja tarkistuskuvat:
[Käyttöliittymäopas](KAYTTOLIITTYMAOPAS.md). Nykyinen opas
on historiallisten beta- ja versio-ohjeiden edellä.

## Tarkistukset ja lisenssit

[Käännös- ja GUI-testikomennot](BUILDING.md). CI ajaa natiivin Linux- ja
Windows-GUI:n 100/150 %:n skaalauksilla. Käytä `--demo`-kilpailua, erillisiä
asetuksia ja virtuaalinäyttöä. Kuvakaappaus yksin ei todista toimintoa:
varmista myös tallennus, uudelleenavaus ja tiedostojen sisältö.
Automaattikokeissa ei käytetä oikeita kilpailutietoja tai fyysistä Emit-lukijaa.

Paikallisen Fluent-sovituksen lisenssit ja alkuperäinen AboutSlint-maininta
säilytetään. Katso [kolmannen osapuolen tiedot](../THIRD_PARTY.md) ja
[sovituksen ylläpito-ohje](../ui/fluent/README.md).

## Ulkoasuasetukset ja gallerian eristys

`Preferences.appearance` tallentaa käyttäjän valinnan. Kaikki vaihtoehdot ja tallennusnimet ovat [teemaluettelossa](TEEMAT.md). Vanhassa asetustiedostossa puuttuva arvo tarkoittaa järjestelmän mukaista tilaa. Virheellinen arvo tai tallennuskonflikti ei korvaa tiedostoa. UI palauttaa viimeisen tallennetun arvon tallennusvirheessä. `effects_enabled` pysyy erillisenä, oletuksena epätotena.

Ainoastaan upstreamin `color-scheme.slint` lukee Slintin sisäistä järjestelmävärimallia. Älä levitä `SlintInternal`-kutsuja näkymiin. Teeman pinnat ovat staattisesti peittäviä. Windowsin alkuperäisiä taustamateriaaleja tai suurikontrastisen tilan täydellistä alustaintegraatiota ei saa kuvata valmiiksi.

`--fluent-gallery` palaa erilliseen `GalleryWindow`-ajoon ennen AppWindow-, PreferencesStore-, tulostin-, lukija- ja päivityspalvelujen alustusta. Galleria ei käytä demo-kilpailua eikä oikeita käyttäjäasetuksia. Kaappaus- ja testiajastimet ovat vain erikseen pyydettyä QA-ajoa varten; tavallinen galleria ei käynnistä niitä.

Nykyinen ulkoasuvalikoima: [10 vaihtoehtoa ja teemojen sävyt](TEEMAT.md).

Kohdistuskehys näkyy vain Tabilla aloitetussa näppäimistönavigoinnissa. Hiiren painallus tai kosketus piilottaa kehyksen, mutta ei poista todellista kohdistusta tai estä näppäimistökäyttöä. Kehys vastaa koko klikattavaa aluetta: luokkakortissa koko korttia, kytkimessä myös tekstiä. Tekstikenttien aktiivisen muokkauksen palaute säilyy.
