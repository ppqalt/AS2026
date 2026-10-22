# AS2026 0.6.9

- Kilpailun nimi ja Automaattinen tallennus -teksti pois sivujen yläreunasta.
- Kilpailun avaus, luonti ja Excel-tuonti Yleiskatsauksen Toiminnot-valikkoon; kilpailijoiden Excel-tuonti ja HTML-lähtöluettelo sekä ratojen lisäys ja tuonti sivujensa Toiminnot-valikkoihin.
- HTML-lähtöluettelo lähtöaikajärjestykseen kaikkien luokkien kesken.
- Kilpailijaparin ATp-sarake sisältää aseman kaikki AT-virhepisteet.
- Tulostaulukkoon Emit-numero, luokkavalintaan yhdistetyt luokat. Luokka- ja asematulostus käyttää valittua luokkaa ja sisältää myös laskemattomat kilpailijat.
- Koko Emit-kortin editori: kellonajan syöttö hh:mm:ss, puuttuvan leiman lisäys, minkä tahansa leiman poisto sekä korttiajan automaattinen laskenta. Perustelua ei vaadita. Akun häiriöleimat 99/999 sivuutetaan laskennassa.
- Luokan poistaminen sen tiedoista. Luokkien ja ratojen muokkaaminen laskennan jälkeen sekä ratakohtainen uudelleenlaskentakysely.
- Radoista näytetään nimi sisäisen numeron sijaan.

[Käyttöohje](docs/KAYTTOLIITTYMA_069.md).

# AS2026 0.6.8

Kilpailun valmistelun ja Emit-korjausten työnkulut on selkeytetty uusimman 0.6.7-päähaaran pohjalta. Lähiverkon ja Windows-tulostuksen korjaukset säilyvät mukana.

- Sivupalkin otsikkona kilpailun nimi, sen alla yhteyden tila. Avaa ja Uusi ovat yleiskatsauksessa.
- Asetuksissa kilpailun maksimi kokonaismyöhästymisraja ja maksimi jaksomyöhästymisraja minuutteina, molempien oletus 60 min. Aikavirheen 10 sekunnin toleranssi on nimetty erikseen.
- Kilpailun numero, puuttuvan JAT:n pisteasetus ja JAT:n pistekatto on poistettu asetuslomakkeesta. Puuttuva JAT ja myöhästymisrajan ylitys aiheuttavat keskeytyksen. JAT:n ja maalin aikavirhettä ei katkaista pistekattoon toimitsijalaskennassa.
- Lähtöluettelon kentät valitaan Asetuksissa valintaruuduilla ja järjestetään nuolilla. Sama järjestys koskee kilpailijataulukkoa, HTML:ää ja CSV:tä. Tila-sarake on poistettu lähtöluettelosta.
- Oletus Emit-korjaus on 120 s. Uuden kilpailijan lomake käyttää kilpailun oletusta, ja kilpailijakohtainen korjaus voidaan vaihtaa. Erillinen 0 s tarkoittaa nollausta lähtöhetkellä; vanhan tiedoston 0 säilyttää vanhan 120 s oletuksen.
- Kilpailijoiden Toiminnot-valikossa Lisää kilpailija ja Arvo lähtöajat. Arvonta avaa luokkavalinnan ja sen jälkeen lähtöaikojen esikatselun.
- Emit-sivulla Muokkaa avaa valitun leiman korjauksen. Sekuntiaika ja lähtöaikaan sidottu hh:mm:ss-kellonaika päivittyvät toistensa mukana. Alkuperäiset leimat ja korjaushistoria säilyvät.
- Tulosten otsikon vieressä kilpailijahaku ja Luokka-valinta, jossa myös kilpailuun määritelty Beginners Cup. Vienti ja luokka- sekä asematulosteet ovat Toiminnot-valikossa.
- Luokkien lisääminen, yhdistetty luokka ja Beginners Cup ovat Luokat-sivun Toiminnot-valikossa.

Vanhoja kilpailutiedostoja voi edelleen avata. Uusien asetusten tallennuksen jälkeen kilpailutiedoston avaaminen vanhalla ohjelmaversiolla ei ole tuettu.
