# Historia: käyttöliittymän muutokset 0.6.8–0.6.9

> Nämä ohjeet kuvaavat julkaistujen versioiden 0.6.8 ja 0.6.9 toimintaa. Myöhemmän version korjauksia ei ole kirjoitettu niiden tilalle. Nykyinen [käyttöliittymäopas](../KAYTTOLIITTYMAOPAS.md) ja [Emit-korjauksen ohje](../EMIT_RIVIN_KORJAUS.md).

## Kilpailun valmistelu ja Emit-korjaukset 0.6.8:ssa

### Asetukset

Maksimi kokonaismyöhästymisraja ja maksimi jaksomyöhästymisraja annetaan kokonaisina minuutteina. Tyhjä käyttää 60 minuuttia. Kokonaisraja koskee kaikkia luokkia ja korvaa vanhan luokkakohtaisen kokonaisrajan. Täsmälleen rajalla oleva myöhästyminen ei keskeytä, mutta yhden sekunnin ylitys keskeyttää. Puuttuva JAT aiheuttaa keskeytyksen. Aikavirheen toleranssi (oletus 10 s) on näistä erillinen pisteytysasetus.

Valitse lähtöluettelon kentät ja järjestys -painike avaa valintaruudut ja siirtonuolet. Valitse vähintään yksi kenttä. Peruuta säilyttää entisen valinnan. Oletusjärjestys: numero, luokka, Emit, lähtöaika, kuljettaja, kuljettajan seura, kartturi, kartturin seura, auto. Valinta tallennetaan kilpailuun ja sitä käytetään myös HTML- ja CSV-vienneissä. Tila kuuluu tuloksiin ja kilpailijan tietoihin.

Oletus Emit-korjaus (s) on 120. Se täytetään uuden kilpailijan lomakkeelle. Oletuksen vaihtaminen ei muuta olemassa olevien kilpailijoiden korjauksia tai hyväksyttyjä tuloksia. Kilpailijan tiedoissa arvo voidaan vaihtaa, myös negatiiviseksi tai nollaksi. Vanhojen tiedostojen nolla-arvo tulkitaan edelleen 120 sekunnin oletukseksi; tässä versiossa erikseen annettu nolla tarkoittaa nollausta lähtöhetkellä.

### Kilpailijat ja lähtöarvonta

Kilpailijat → Toiminnot → Lisää kilpailija tai Arvo lähtöajat. Arvonta avaa luokkavalinnan, josta Arvo lähtöajat siirtyy esikatseluun. Lähtöajat tallentuvat vasta esikatselun hyväksymisestä. Luettujen korttien ja tallennettujen tulosten lähtöaikoja ei arvota uudelleen.

### Emit

Valitse lukutapahtuma ja sen sisältä korjattava rivi. Muokkaa avaa leiman koodin, sekuntiajan ja kellonajan muokkaamisen. Kellonaika = kilpailijan lähtöaika + korttiaika − kilpailijan Emit-korjaus. Näytön 0/lähtörivi näyttää kilpailijan virallisen lähtöajan. Kellonaika vaihtuu keskiyön yli; sekuntikenttä säilyttää koko kuluneen ajan.

Anna korjaukselle perustelu ja tallenna. Muutos laskee sekä perusluokan että BC:n tuloksen uudelleen ja poistaa niiden hyväksynnät. Käsikellot ja erilliset lisävirheet säilyvät. Alkuperäiset leimat säilyvät historiassa. Vanhaa erilaista korttilukua voi tarkastella, mutta korjaus tehdään aktiiviseen lukutapahtumaan. Käsikello korjataan edelleen Tulokset-sivun käsikellokentässä.

### Tulokset ja luokat

Tulosten Luokka-valinta näyttää yhden luokan; ensimmäinen Luokka-valinta näyttää kaikki perusluokat. Beginners Cup on mukana, kun se on määritelty kilpailuun. Tulosten viennit sekä luokka- ja asematulosteet löytyvät Toiminnot-valikosta.

Luokat → Toiminnot sisältää Lisää luokka, Lisää yhdistetty luokka ja Beginners Cup. Excel-tuonti löytyy yleiskatsauksesta ja Kilpailijat-sivulta. Lähtöluettelo viedään Kilpailijat-sivulta.

## Käyttöliittymän korjaukset 0.6.9

Sivujen yläreunan kilpailuotsikko ja Automaattinen tallennus -teksti on poistettu. Kilpailun nimi näkyy edelleen sivupalkissa.

Yleiskatsauksen Toiminnot-valikossa ovat Avaa, + Uusi ja Tuo kilpailu Excelistä. Kilpailijoiden Toiminnot-valikossa ovat myös Tuo Excel ja Tallenna html-lähtöluettelo. Lähtöluettelo järjestetään lähtökellon mukaan kaikkien luokkien kesken, myös keskiyön yli; samaan aikaan lähtevät kilpailunumeron mukaan ja puuttuvat lähtöajat viimeiseksi. Radat ja asemat -sivun Lisää rata ja Tuo Excel löytyvät Toiminnot-valikosta.

### Emit-leimausten muokkaaminen

Valitse Emit-sivulla kilpailijan lukutapahtuma ja Muokkaa leimauksia. Editorissa ovat kaikki aktiivisen kortin leimaukset. Anna koodi ja kilpailukortista luettu kellonaika hh:mm:ss. Korttiaika lasketaan kaavalla kellonaika − lähtöaika + kilpailijan Emit-korjaus, myös keskiyön yli.

Lisää leima lisää uuden leimauksen. Poista leima -valinta poistaa kyseisen leimauksen tallennettaessa; myös radan asemaan kohdistuneen leiman voi poistaa. Tallennus järjestää leimat korttiajan mukaan. Koodit 99 ja 999 eivät tuota ylimääräisen AT:n virhepisteitä eivätkä kelpaa radan aseman leimaukseksi. Perustelua ei vaadita. Alkuperäinen korttiluku säilyy historiassa, ja Palauta alkuperäiset leimat täyttää editoriin alkuperäisen sisällön. Peruuta hylkää editorin muutokset.

Tallennus laskee kilpailijan NAC- ja BC-tulokset uudelleen. Asemien pistekorjaukset ja hyväksynnät poistuvat; samannimisten ja samantyyppisten käsikelloasemien ajat sekä erilliset lisävirheet säilyvät. Tarkista tulos ennen hyväksymistä.

### Tulokset ja tulosteet

Tulostaulukossa näkyy Emit-numero. Luokkavalinnassa ovat myös yhdistetyt luokat. Valitun luokan Tulosta luokkatulokset ja Tulosta asematulokset lähettävät kyseisen luokan tulosteen tulostimeen. Kun luokkaa ei ole rajattu, tulostus avaa monivalinnan. Tulosteissa ovat myös vielä laskemattomat kilpailijat ilman sijoitusta tai laskettua pistesummaa. Niiden tilana on AJAA. Asematulosteen sarakkeet tulevat radasta myös ennen ensimmäistä korttilukua.

Kilpailijaparin tulosteen ATp (AT pisteet) sisältää aseman aikavirhepisteiden itseisarvon sekä puuttuvan tai ylimääräisen leiman virhepisteet. Puuttuvan AT:n oletus on 1800 pistettä; kilpailun pisteasetuksia noudatetaan.

### Luokat ja radat

Luokkia ja yhdistettyjä luokkia voi lisätä ja muokata laskettujen tulosten jälkeenkin. Luokan tiedoissa on Poista luokka. Luokan kilpailijat siirretään ensin toiseen luokkaan. Poisto poistaa luokan myös yhdistettyjen luokkien jäsenvalinnoista; tyhjentyvä yhdistelmä poistuu.

Radoista näytetään nimi. Sisäinen yksilöivä ratatunniste säilyy kilpailutiedostossa ja vanhojen tiedostojen yhteensopivuus säilyy.

Radan asemien tai luokan ratavalinnan muuttamisen jälkeen ohjelma kysyy, päivitetäänkö lasketut tulokset. Kyllä laskee uudelleen muokattuja ratoja käyttävien kilpailijoiden tulokset ja säilyttää vastaavien käsikelloasemien ajat. Peruuta säilyttää aiemmat pisteet. Muuttunutta rataa vastaamatonta vanhaa tulosta ei julkaista hyväksyttynä.
