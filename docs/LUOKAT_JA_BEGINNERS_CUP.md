# Yhdistetyt luokat ja NAC Beginners Cup

## Yhdistetyn luokan luonti

Luokat-sivun **Toiminnot → Lisää yhdistetty luokka** avaa valintaikkunan, jossa annetaan
nimi ja valitaan jäsenluokat valintaruuduilla. Nousupisteet ja sarjapistemenetelmä
voidaan valita samassa ikkunassa. **Lisää luokka** tallentaa määritelmän kerralla;
**Peruuta** tai Esc sulkee tallentamatta. Virheellinen syöte tai tallennusvirhe
jättää syötteen korjattavaksi ikkunaan.

Yhdistetyt luokat näkyvät Luokat-taulukossa Y1–Y4-riveinä. Rivin valitseminen
avaa muokkausikkunan. Y1–Y4 ovat taulukon merkintöjä, eivät uusia kilpailijaluokkia.
Enintään neljän yhdistetyn luokan raja säilyy. Kilpailijat, radat, lähtöarvonta
ja alkuperäiset luokat säilyvät. Yhdistetty tulos käyttää alkuperäisten luokkien
valmiita tuloksia ja sijoituslaskentaa, eikä laske virhepisteitä toisella radalla.

## Beginners Cupin määrittäminen

1. Lisää kilpailijat kerran heidän NAC-luokkiinsa ja määritä NAC-rata normaalisti.
2. Avaa **Luokat → Toiminnot → Beginners Cup**. Anna BC:n nimi ja erillinen tunnus,
   oletuksena `BC`, ja valitse NAC-lähderata.
3. Valitse BC-tulokseen kuuluvat asemat. Pois valitut asemat jätetään BC:ssä
   huomioimatta. Lähtö, maali ja ULA-aikaviitteet säilyvät mukana yhteisten aikojen
   tulkitsemiseksi. Pois jätetyn JAT:n tavoite- ja jaksoajat siirtyvät seuraavaan
   osuuteen; mukana oleva ULA aloittaa uuden aikaviitteen.
4. Valitse BC-osallistujat saman lähderadan kilpailijoista ja valitse
   **Tallenna BC**. Myös kilpailijan tiedoissa on BC-osallistujavalinta.

Eri NAC-luokat voivat käyttää samaa lähderataa. Niiden kilpailijat saavat
yhteisen BC-tuloksen; jokaisen laskennassa säilyy oman NAC-luokan myöhästymisraja.
BC on tässä toteutuksessa yhden NAC-lähderadan osajoukko. Omia tavoiteaikoja,
nousupisteitä ja sarjapisteitä ei määritetä tällä toiminnolla.

Osallistujavalinnan poistaminen säilyttää aiemmat BC:n virhepisteet, käsikellot
ja asematuloksen. Ne palautuvat käyttöön, kun kilpailija valitaan uudelleen.
Kilpailija lasketaan ilmoittautuneiden määrään ja lähtöarvontaan kerran.

## Tulosten käsittely

Tulokset-sivun **Beginners Cup** -painike avaa BC-näkymän ja uusi painallus
palauttaa perusluokkiin. Näkymän voi valita myös luokkavalikosta. Kortti luetaan kerran, ja ensimmäinen lukema voi muodostaa molemmat
asematulokset. Alkuperäinen Emit-historia säilyy muuttamattomana.

BC-näkymän kilpailijapaneelista avataan **Muokkaa kilpailijaa**. Puuttuvat
ja ylimääräiset RT:t, muut virhepisteet, tila ja tilan syy tallennetaan vain
BC-tulokseen. Myös asemakorjaukset, käsikellot, **Tutki**, **Tarkista** ja
**Tulosta tulos** kohdistuvat valittuun laskentaan. NAC-pisteitä ei kopioida BC:hen.
BC-pisteiden muutos ei muuta NAC-tulosta tai sen hyväksyntää ja päinvastoin.
Perusteltu keskeytys voidaan vahvistaa kummallekin erikseen.

Lähtöaika, Emit-kortin numero ja Emit-korjaus ovat yhteisiä kilpailijatietoja.
Niiden sekä yhteisten ajanottoasetusten muutos edellyttää molempien tulosten
tarkistamista. BC:n asemavalinnan muutos mitätöi vain BC-hyväksynnän.
NAC-radan asemien muuttuminen estää BC-laskennan, kunnes asemavalinnat on
tarkistettu ja tallennettu uudelleen. Asemien järjestys ei voi vaihtua
hiljaisesti vanhojen BC-valintojen alla.

Kortin uusintaluku noudattaa sovelluksen aiempaa toimintaa: tallennetut
tarkastukset ja käsikorjaukset säilyvät. **Tarkista** muodostaa valitun laskennan
asemat uudesta lukemasta vahvistuksen jälkeen. Kortinluku siirtyy perusluokan
tulokseen, josta BC-näkymään voi vaihtaa.

HTML-, CSV- ja XLSX-tulosviennit sekä paikallinen live-HTML sisältävät molemmat
laskennat. Hyväksynnät tarkistetaan erikseen, vaikka kilpailijanumero on sama.
Hyväksymätön BC-esikatselu ei saa NAC:n hyväksyntää eikä päinvastoin.

## BC:n maalin aikarangaistus

Versiossa 0.7.0 BC:n maali päättää ajanoton ilman
ennakko- tai myöhästymisrangaistusta. Tämä koskee myös NAC-radan JAT-asemaa,
johon lyhennetty BC päättyy. Maaliaika säilyy ajanoton päättymiseen ja tuloksen
tarkastamiseen; NAC:n saman aseman sääntöä tai pisteitä ei muuteta. Muut BC:n
asemat, puuttuvat leimat ja kilpailun keskeytysehdot käsitellään edelleen
nykyisen laskennan mukaan.

## Leimojen kohdistus ja tiedostot

BC-ohitukset perustuvat NAC-radan asemakohtiin, eivät pelkkään Emit-koodiin.
Saman koodin muu esiintymä ja oikeasti ylimääräiset leimat säilyvät laskennassa.
Saman leiman käyttävä ULA säilyttää tarvitsemansa aikaviitteen.
Jos sama koodi kuuluu sekä mukana olevaan että ohitettavaan asemakohtaan ja
lähderadan leima puuttuu, kohdistus voi olla epäselvä. BC-laskenta ilmoittaa
virheen eikä poista leimaa arvaamalla. NAC-aineisto säilyy.

`beginners_cup` sisältää määrityksen ja tarkistetun lähderadan. `bc_osallistujat`
sisältää kilpailijanumeroon viittaavat BC-tiedot. Kilpailijaa tai raakaleimoja
ei tallenneta toista kertaa. Puuttuvat lisäkentät tarkoittavat vanhaa kilpailua
ilman BC:tä, joten aiemmat tiedostot avautuvat edelleen.

## Varmennus

`tests/beginners.rs` kattaa osajoukon laskennan, toistuvat koodit, epäselvän
puuttuvan leiman, aidot ylimääräiset leimat, omat virhepisteet ja hyväksynnät,
ratamuutokset, osallistujavalinnan säilyttämisen, tallennuksen, lähtöarvonnan ja
molempien tulosten viennin. GUI:n työnkulkukokeet käsittelevät uudet valintaikkunat,
BC:n käsimaalin, pisteet ja hyväksynnän sekä näppäimistön ja vierityksen.

CI tarkistaa Rust-testit, Clippyn ja GUI-käännöksen Linuxissa ja Windowsissa.
Linuxissa GUI-työnkulkukoe suoritetaan myös 100 ja 150 prosentin skaalauksella
ja tarkistuskuvat tallennetaan Actions-artifactiksi. Synteettinen koe ei korvaa
fyysisen Emit-lukijan koetta tai oikeaan kilpailuaineistoon vertaamista.


Versiosta 0.6.11 alkaen BC-osallistujan NAC-tuloksen tallentaminen avaa automaattisesti
Beginners Cupin oman aikojen ja virhepisteiden syötön. Täytä BC:n puuttuvat RT:t ja
muut virhepisteet erikseen ja tallenna. Tallennetut BC-arvot näytetään uusintakäsittelyssä;
NAC-arvoja ei kopioida niiden tilalle.
