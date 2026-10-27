# Tulosvienti ja huomautukset

**Tulokset → Toiminnot → Vie tulokset** avaa Excel-, CSV- ja HTML-valinnat sekä hiirellä
että näppäimistöllä. Valinta avaa sovelluksen oman tallennusikkunan. Oletuksena
tiedosto tallentuu kilpailun kansioon. Sijaintia voi muuttaa kentässä tai
Selaa-painikkeesta. Jos nimi on jo käytössä, oletukseen lisätään numero.
Olemassa olevaa tiedostoa ei korvata; virhe näkyy tallennusikkunassa.

Esikatselun avaaminen tai tiedoston vieminen ei hyväksy tuloksia. Vain hyväksytyt tulokset
saavat virallisen sijoituksen ja pisteet.

**Huomautukset** näyttää yhteenvedon. Saman huomautuksen kilpailijat ovat
samalla rivillä, ja pitkistä numerolistoista näytetään kahdeksan numeroa sekä
muiden määrä. Hyväksymättömät tulokset kerrotaan yhtenä valmistelutietona,
eikä normaalia hyväksymättömyyttä nimetä tuntemattomaksi tarkastustulokseksi.
Luokkakoodien tavalliset etuliitteet eivät vaadi toimitsijalta toimenpiteitä.
Nousu- ja sarjapisteiden odotus näytetään vain, kun pisteitä käytetään.

**Näytä lisätiedot** avaa alkuperäisen, täydellisen diagnostiikan.
Huomautusikkuna sulkeutuu myös Esc-näppäimellä.

Tiedostonimen voi kirjoittaa ilman päätettä: valitun vientimuodon pääte lisätään
automaattisesti. Pelkkä nimi ja suhteellinen polku tulkitaan kilpailun kansion
suhteen. Väärän tiedostopäätteen virhe näytetään tallennusikkunassa, eikä tiedostoa
luoda väärällä muodolla.

## Julkinen tulosluettelo

Versiosta 0.7.1 alkaen HTML-tulosvienti ja sen tulostusasu sisältävät kilpailun,
luokkien ja ratojen tulokset ilman toimitsijan huomautuksia tai teknisiä
hyväksyntäselostuksia. Sama koskee yhdistettyjä luokkia ja Beginners Cupia.
Keskeneräisen luokan yhteydessä näkyy lyhyt **Alustavat tulokset**; hyväksyntää
odottavia kilpailijoita ei julkaista hyväksyttyinä eikä puuttuvia pisteitä täytetä.
Toimitsijan yllä kuvatut huomautukset ja täydelliset lisätiedot säilyvät saatavilla.

Komentorivin `tulokset` tulostaa luettelon stdout-virtaan ja toimitsijan
huomautukset stderr-virtaan. Tiedostoon ohjattu tulosluettelo (`> tulokset.md`)
ei siten sisällä raakadiagnostiikkaa.


## Version 0.7.2 vientivalinnat

Viennin XLSX-, CSV-, HTML- ja PDF-muodoissa valitaan vietävät luokat sekä
haluttaessa **Asemakohtaiset tulokset**. Otsikko on kilpailun nimi ja
luokkaotsikko `luokka: luokan nimi`; yhdistetyn luokan sisäinen Y-tunnus ei näy.
PDF syntyy ilman virtuaalitulostinta. Asetusten PNG-logo tallentuu sovellusasetuksiin,
kuvan alkuperäistä tiedostoa ei tarvita myöhemmin. PDF:ssä logo on vasemmassa
ylänurkassa 10 mm:n marginaalien sisällä. Enimmäiskorkeus on 20 mm ja
enimmäisleveys 60 mm; kuvasuhde säilyy ja otsikot väistävät logoa.

PDF-viennissä anna valituille luokille eri positiiviset järjestysnumerot.
Luokat tulevat pienimmästä numerosta suurimpaan myös silloin, kun mukana on
Beginners Cup. Valitsemattoman luokan numeroa ei tarkisteta.

**KITI Excel-pohjaan** pyytää alkuperäisen KITIstä ladatun XLSX-pohjan ja erillisen
tulostiedoston. Oletusnimi on pohjan nimi + **-valmis.xlsx**, pohjan kansiossa.
Olemassa olevaa kohdetiedostoa ei korvata. Results-työlehden luokkalohkot ja A–R-tiedot säilyvät, tulokset
täytetään S–Y-sarakkeisiin. Sijoitus ja luokkasijoitus käsitellään erikseen.
Kohdistus käyttää kilpailunumeroa ja luokkayhteyttä; nimiä ei käytetä arvaukseen.

Valitse jokaiselle pohjan luokalle pakollinen **Perusluokka** ja halutessasi
**Yhdistetty luokka**. Molemmissa valinnoissa ovat kaikki tulosluokat.
Perusluokan sijoitus menee aina **Lkasij. / Class position** -sarakkeeseen.
Yhdistetyn luokan sijoitus menee **Sij. / Position** -sarakkeeseen; ilman
yhdistettyä valintaa siihen tulee perusluokan sijoitus. Myös pisteet ja tila
tulevat perusluokasta. Molemmat kohdistukset muistetaan asetuksissa.
Tarkista ehdotukset ennen vientiä.

KITI-pohjan tulosselitesarakkeen kommentin mukaiset oletukset ovat osallistunut 1,
keskeyttänyt 2, hylätty 3, kilpailusta suljettu 4 ja peruuttanut 0. Myös lähtemätön
viedään koodilla 0. Tarvittaessa eri pohjaversion tilakoodit voi muuttaa valinnasta
**Muokkaa KITI-tilakoodeja** (TRK, KESK, HYL, SULJ, PER ja EILÄ).
Puuttuva tai moniselitteinen kohdistus estää viennin selkeällä virheilmoituksella.

Älä liitä oikeita kilpailijatietoja GitHubiin. Käytä testiaineistona synteettisiä
nimiä, tunnisteita ja lisenssejä myös piilotetuissa soluissa ja metatiedoissa.
