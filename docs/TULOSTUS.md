# Aikojen täydentäminen ja tulostus

Tunnetun Emit-kortin lukeminen avaa Tulokset-sivun ja **Täydennä tulos**
-ikkunan. Jos toinen syöttö on kesken, lukema tallentuu ja seuraavan kilpailijan
ikkuna avautuu nykyisen syötön valmistuttua tai peruutuksen jälkeen.

Syötä leimasinnumeron 0 asemien kellonajat radan järjestyksessä. Myös käsin
otettava maaliaika kysytään, jos maalissa ei ole leimasinta. Lähtöaika tulee
kilpailijan tiedoista. Kellon voi antaa muodossa `18:03:25`, `18.03.25` tai
`180325`. Tyhjä kenttä on puuttuva aika; `00:00:00` on annettu keskiyön aika.

**Tab** siirtyy aikakentästä seuraavaan, viimeisestä ajasta puuttuvien RT-asemien
määrään, siitä muihin virhepisteisiin ja lopuksi **Tallenna**- ja **Tallenna ja tulosta** -painikkeisiin.
**Tallenna** hyväksyy kelvollisen tuloksen ilman tulostamista. **Tallenna ja tulosta** lähettää lisäksi tulosteen.
Enter syöttökentässä käynnistää **Tallenna ja tulosta** -toiminnon. Kun kohdistus on
**Tallenna**- tai **Peruuta**-painikkeessa, Enter suorittaa kyseisen painikkeen toiminnon.
Aiemmin syötetyt ylimääräisten RT-asemien
pisteet säilyvät; niitä voi muuttaa kilpailijan tiedoista.

**Tallenna ja tulosta** tallentaa ajat ja lisävirheet sekä laskee aikavirheet. Tulos tulee
tulosluetteloon ilman erillistä hyväksymispainiketta. Tulostinvirhe ei peru
tietojen tallennusta; tulostamisen voi uusia. Saman kilpailijan **Tulosta tulos**
avaa ikkunan myös myöhemmin. Peruuta tai Esc sulkee syöttämättä uusia arvoja.

Kilpailijaparin A4-pystytulosteessa ovat kilpailijatiedot, lähtö- ja maaliaika,
käytetty aika, kokonaismyöhä, AT-, ATp/ATy-, RT- ja muut pisteet sekä asemien
ihanneajat, toteutuneet ajat, kellot ja virhepisteet. ULA on aikaviite, joten
sitä ei tulosteta omana virhepisterivinään. Asemarivit sovitetaan yhdelle sivulle.
NAC- ja BC-laskenta käsitellään omissa tulosnäkymissään: BC:n syöttö ja tulostus
käyttävät BC:n omia virhepisteitä ja asemavalintaa. Raaka Emit-aineisto säilyy.

## Luokka- ja asematulokset

Avaa **Tulokset → Toiminnot → Tulosta luokkatulokset** tai
**Tulosta asematulokset**. Valitse yksi tai useampi perusluokka, yhdistetty
luokka tai määritetty Beginners Cup. Koko luokkakorttia voi klikata.
Haku rajaa näkyvät kortit, mutta säilyttää muut valinnat. **Vain valitut** näyttää
valitut luokat; **Valitse kaikki** ja **Tyhjennä** koskevat koko luetteloa.
Valittujen määrä näkyy hakurivin yläpuolella. Muiden luokkien tuloksia ei lisätä tulosteeseen.
Jokainen valittu luokka alkaa omalta sivultaan. Oletuksena kukin luokka sovitetaan
yhdelle sivulle. Asematuloste on oletuksena A4-vaaka, luokkatulos A4-pysty.
Suunnan voi vaihtaa. Sovituksen poistaminen antaa pitkän luokan jatkua uusilla
sivuilla, joilla taulukon otsikot toistetaan. Tiheässä yhden sivun tulosteessa
teksti pienenee, joten suurelle kilpailijamäärälle kannattaa poistaa sovitus.

Luokkatuloste sisältää sijan, numeron, nimet, seurat, auton, tuloksen, mahdolliset
nousu- ja sarjapisteet sekä tilan syyn. Asematulosteessa näkyvät asemakohtaiset
aikavirheet ja pistesummat. Keskeneräiselle tulokselle ei tulosteta sijoitusta
tai esikatselun pisteitä. Luokkien sijoitukset tulevat samasta laskennasta kuin
ohjelman muissa tulosvienneissä.

## Tulostin

Valitse **Asetukset → Tulostin → Päivitä tulostinluettelo** ja valitse tulostin.
Valinta tallennetaan käyttäjän sovellusasetuksiin, ei kilpailutiedostoon.
**Käyttöjärjestelmän oletus** käyttää järjestelmän oletustulostinta.

Windows lähettää sivumallin suoraan valitun tulostimen GDI-jonoon Rustista
Windowsin omilla rajapinnoilla. PowerShelliä, .NET-ajoympäristöä tai selainta ei tarvita.
Tulostimen pitää tukea A4-paperia. Myös virtuaalisen PDF-tulostimen voi valita;
sen oma tallennusikkuna voi kysyä tiedoston sijainnin.

Linux käyttää CUPS:n `lp`- ja `lpstat`-komentoja sekä saman sivumallin
PostScript-tulostetta. Toimiva käyttöjärjestelmän tulostusjono tarvitaan.
Ohjelman ilmoitus tarkoittaa työn lähettämistä jonoon; paperitulostimen virheet
ja jonon jatkokäsittely tarkistetaan käyttöjärjestelmästä.

## Varmennus

Ydintestit tarkistavat manuaalisten aikojen laskennan, tallennuksen, virheelliset syötteet,
luokkien rajauksen, pitkän taulukon sovituksen ja PostScriptin tekstiescapet.
GUI-koe käyttää oikeita Tab- ja Enter-tapahtumia AT → JAT → RT → muut → Tulosta
-järjestyksessä. Windowsin testit renderöivät kilpailija-, luokka- ja asematulosteet
samalla Rust/GDI-piirtäjällä PNG-kuviksi lähettämättä niitä tulostusjonoon.
Fyysisen tulostimen ja Emit-lukijan yhteiskoe tehdään erikseen.

Tulosviennin luokat valitaan samalla kahden sarakkeen korttivalitsimella kuin tulostuksessa ja live-julkaisussa. Haku ja Vain valitut rajaavat näkyvää listaa; piilossa olevat valinnat säilyvät. PDF:n luokkajärjestys annetaan korttien järjestysnumerokentissä.

HTML-, CSV-, Excel- ja PDF-vientien oletusnimissä on kilpailun nimi, esimerkiksi `testieksytys-tulokset.pdf`. Sama etuliite lisätään HTML-lähtöluetteloon ja KITI-pohjan tulostiedostoon. Oman tiedostonimen voi edelleen valita.
