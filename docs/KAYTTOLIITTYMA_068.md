# Kilpailun valmistelu ja Emit-korjaukset 0.6.8:ssa

> Historiallinen kuvaus aikaisemmasta työvaiheesta tai versiosta. Nykyinen 0.7.0: [käyttöliittymäopas](KAYTTOLIITTYMAOPAS.md).
## Asetukset

Maksimi kokonaismyöhästymisraja ja maksimi jaksomyöhästymisraja annetaan kokonaisina minuutteina. Tyhjä käyttää 60 minuuttia. Kokonaisraja koskee kaikkia luokkia ja korvaa vanhan luokkakohtaisen kokonaisrajan. Täsmälleen rajalla oleva myöhästyminen ei keskeytä, mutta yhden sekunnin ylitys keskeyttää. Puuttuva JAT aiheuttaa keskeytyksen. Aikavirheen toleranssi (oletus 10 s) on näistä erillinen pisteytysasetus.

Valitse lähtöluettelon kentät ja järjestys -painike avaa valintaruudut ja siirtonuolet. Valitse vähintään yksi kenttä. Peruuta säilyttää entisen valinnan. Oletusjärjestys: numero, luokka, Emit, lähtöaika, kuljettaja, kuljettajan seura, kartturi, kartturin seura, auto. Valinta tallennetaan kilpailuun ja sitä käytetään myös HTML- ja CSV-vienneissä. Tila kuuluu tuloksiin ja kilpailijan tietoihin.

Oletus Emit-korjaus (s) on 120. Se täytetään uuden kilpailijan lomakkeelle. Oletuksen vaihtaminen ei muuta olemassa olevien kilpailijoiden korjauksia tai hyväksyttyjä tuloksia. Kilpailijan tiedoissa arvo voidaan vaihtaa, myös negatiiviseksi tai nollaksi. Vanhojen tiedostojen nolla-arvo tulkitaan edelleen 120 sekunnin oletukseksi; tässä versiossa erikseen annettu nolla tarkoittaa nollausta lähtöhetkellä.

## Kilpailijat ja lähtöarvonta

Kilpailijat → Toiminnot → Lisää kilpailija tai Arvo lähtöajat. Arvonta avaa luokkavalinnan, josta Arvo lähtöajat siirtyy esikatseluun. Lähtöajat tallentuvat vasta esikatselun hyväksymisestä. Luettujen korttien ja tallennettujen tulosten lähtöaikoja ei arvota uudelleen.

## Emit

Valitse lukutapahtuma ja sen sisältä korjattava rivi. Muokkaa avaa leiman koodin, sekuntiajan ja kellonajan muokkaamisen. Kellonaika = kilpailijan lähtöaika + korttiaika − kilpailijan Emit-korjaus. Näytön 0/lähtörivi näyttää kilpailijan virallisen lähtöajan. Kellonaika vaihtuu keskiyön yli; sekuntikenttä säilyttää koko kuluneen ajan.

Anna korjaukselle perustelu ja tallenna. Muutos laskee sekä perusluokan että BC:n tuloksen uudelleen ja poistaa niiden hyväksynnät. Käsikellot ja erilliset lisävirheet säilyvät. Alkuperäiset leimat säilyvät historiassa. Vanhaa erilaista korttilukua voi tarkastella, mutta korjaus tehdään aktiiviseen lukutapahtumaan. Käsikello korjataan edelleen Tulokset-sivun käsikellokentässä.

## Tulokset ja luokat

Tulosten Luokka-valinta näyttää yhden luokan; ensimmäinen Luokka-valinta näyttää kaikki perusluokat. Beginners Cup on mukana, kun se on määritelty kilpailuun. Tulosten viennit sekä luokka- ja asematulosteet löytyvät Toiminnot-valikosta.

Luokat → Toiminnot sisältää Lisää luokka, Lisää yhdistetty luokka ja Beginners Cup. Excel-tuonti löytyy yleiskatsauksesta ja Kilpailijat-sivulta. Lähtöluettelo viedään Kilpailijat-sivulta.
