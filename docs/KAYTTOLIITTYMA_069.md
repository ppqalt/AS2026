# Käyttöliittymän korjaukset 0.6.9

> Historiallinen kuvaus aikaisemmasta työvaiheesta tai versiosta. Nykyinen 0.6.10: [käyttöliittymäopas](KAYTTOLIITTYMAOPAS.md).
Sivujen yläreunan kilpailuotsikko ja Automaattinen tallennus -teksti on poistettu. Kilpailun nimi näkyy edelleen sivupalkissa.

Yleiskatsauksen Toiminnot-valikossa ovat Avaa, + Uusi ja Tuo kilpailu Excelistä. Kilpailijoiden Toiminnot-valikossa ovat myös Tuo Excel ja Tallenna html-lähtöluettelo. Lähtöluettelo järjestetään lähtökellon mukaan kaikkien luokkien kesken, myös keskiyön yli; samaan aikaan lähtevät kilpailunumeron mukaan ja puuttuvat lähtöajat viimeiseksi. Radat ja asemat -sivun Lisää rata ja Tuo Excel löytyvät Toiminnot-valikosta.

## Emit-leimausten muokkaaminen

Valitse Emit-sivulla kilpailijan lukutapahtuma ja Muokkaa leimauksia. Editorissa ovat kaikki aktiivisen kortin leimaukset. Anna koodi ja kilpailukortista luettu kellonaika hh:mm:ss. Korttiaika lasketaan kaavalla kellonaika − lähtöaika + kilpailijan Emit-korjaus, myös keskiyön yli.

Lisää leima lisää uuden leimauksen. Poista leima -valinta poistaa kyseisen leimauksen tallennettaessa; myös radan asemaan kohdistuneen leiman voi poistaa. Tallennus järjestää leimat korttiajan mukaan. Koodit 99 ja 999 eivät tuota ylimääräisen AT:n virhepisteitä eivätkä kelpaa radan aseman leimaukseksi. Perustelua ei vaadita. Alkuperäinen korttiluku säilyy historiassa, ja Palauta alkuperäiset leimat täyttää editoriin alkuperäisen sisällön. Peruuta hylkää editorin muutokset.

Tallennus laskee kilpailijan NAC- ja BC-tulokset uudelleen. Asemien pistekorjaukset ja hyväksynnät poistuvat; samannimisten ja samantyyppisten käsikelloasemien ajat sekä erilliset lisävirheet säilyvät. Tarkista tulos ennen hyväksymistä.

## Tulokset ja tulosteet

Tulostaulukossa näkyy Emit-numero. Luokkavalinnassa ovat myös yhdistetyt luokat. Valitun luokan Tulosta luokkatulokset ja Tulosta asematulokset lähettävät kyseisen luokan tulosteen tulostimeen. Kun luokkaa ei ole rajattu, tulostus avaa monivalinnan. Tulosteissa ovat myös vielä laskemattomat kilpailijat ilman sijoitusta tai laskettua pistesummaa. Niiden tilana on AJAA. Asematulosteen sarakkeet tulevat radasta myös ennen ensimmäistä korttilukua.

Kilpailijaparin tulosteen ATp (AT pisteet) sisältää aseman aikavirhepisteiden itseisarvon sekä puuttuvan tai ylimääräisen leiman virhepisteet. Puuttuvan AT:n oletus on 1800 pistettä; kilpailun pisteasetuksia noudatetaan.

## Luokat ja radat

Luokkia ja yhdistettyjä luokkia voi lisätä ja muokata laskettujen tulosten jälkeenkin. Luokan tiedoissa on Poista luokka. Luokan kilpailijat siirretään ensin toiseen luokkaan. Poisto poistaa luokan myös yhdistettyjen luokkien jäsenvalinnoista; tyhjentyvä yhdistelmä poistuu.

Radoista näytetään nimi. Sisäinen yksilöivä ratatunniste säilyy kilpailutiedostossa ja vanhojen tiedostojen yhteensopivuus säilyy.

Radan asemien tai luokan ratavalinnan muuttamisen jälkeen ohjelma kysyy, päivitetäänkö lasketut tulokset. Kyllä laskee uudelleen muokattuja ratoja käyttävien kilpailijoiden tulokset ja säilyttää vastaavien käsikelloasemien ajat. Peruuta säilyttää aiemmat pisteet. Muuttunutta rataa vastaamatonta vanhaa tulosta ei julkaista hyväksyttynä.
