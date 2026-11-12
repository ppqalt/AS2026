# Kilpailu ilman Emit-kortteja

## Kilpailun valmistelu

1. Luo uusi kilpailu ja valitse Asetukset → Ajanottotapa → Manuaalinen.
2. Lisää luokat, kilpailuparit ja lähtöluettelon lähtöajat. Emit-kortin numeroa ei tarvita.
3. Lisää Radat ja asemat -näkymään laskennan L-, AT-, JAT-, ULA- ja M-asemat sekä tavoiteajat ja jaksojen ajat. Emit-koodit eivät vaikuta paperikortin tulospohjaan.
4. Tuo Kilvet-näkymään oikeat rivit. RT-kilvet käsitellään erillisinä puutteina; aika-asemien AT/JAT/ULA/M-tyyppien ja järjestyksen tulee vastata laskentarataa. Nimien, kuten AT1 ja AT31, ei tarvitse olla samoja.
5. Lähetä kilpailutiedot AS tulkki -painikkeesta. Manuaalisessa kilpailussa kaikki aika-asemat julkaistaan ajan syöttöä varten.

Jos verkkokilpailuun on jo tallennettu tulkintoja, lisäosa ei salli oikean rivin muuttamista. Aloita manuaalisen ajanoton kokeilu uutena kilpailuna ennen ensimmäistä lähetystä ja tulkkausta.

## Aikojen syöttö AS2026:ssa

Valitse pari Tulokset-näkymästä ja paina Tulosta tulos. Aikojen syöttö aukeaa ilman kortinlukua. Syötä kellot muodossa hhmmss (esim. 183403). Kortista puuttuva merkintä merkitään viivalla (-). Tallenna laskee ja hyväksyy valmiin tuloksen. Tallenna ja tulosta lähettää lisäksi tuloslapun tulostimelle.

Tarkista muodostaa uuden tyhjän tulospohjan ja poistaa aiemmat ajat sekä aikapuutteiden automaattiset AT-pisteet vahvistuksen jälkeen. Käytä puhelimelta saatujen aikojen tarkastamiseen Tulosta tulos -painiketta. Tutki laskee paikallisesti muokatut ajat uudelleen.

Lähtöluettelon lähtöaika tarvitaan laskennan perustaksi. Jos sitä ei ole, laskenta odottaa sen syöttämistä. Puuttuvaa lähtöaikaa ei korvata keksityllä ajalla.

## Puhelimen ajat

Valmis-painikkeella lähetetyt ajat tulevat AS2026:een automaattisesti. Kohdistus tehdään aika-asemien järjestyksen ja tyypin mukaan, RT-kilvet ohittaen. Epäyhtenäinen asemaluettelo tai ristiriitainen aikajärjestys estää siirron ja näyttää virheen. Tietoja ei siirretä osittain kilpailutiedostoon.

Uusi kellonaika päivittää laskennan ja poistaa vanhan hyväksynnän. Tuloslaskija hyväksyy/tulostaa tuloksen edelleen itse. Pelkkä saman tulkinnan uudelleen vastaanottaminen ei lisää pisteitä eikä poista hyväksyntää. Keskeneräistä paikallista muokkausta ei korvata taustapäivityksellä.

## Puuttuvat aikamerkinnät

- Puuttuva AT-käynti: oletuksena 1800 vp nykyiseen Muut virhepisteet -kenttään. Määrä tulee kilpailun puuttuvan AT:n pisteasetuksesta.
- Puuttuva JAT tai maali: KESK, eli keskeyttänyt. Korjattu puhelintulkinta poistaa automaattisen puutesyyn uuden laskennan yhteydessä.
- Puuttuva ULA-aika: tulos odottaa selvitystä. Sitä ei voi hyväksyä sellaisenaan.
- Vielä tulkkaamaton jakso tai syöttämätön aika: odottaa täydentämistä; ei automaattista puuterangaistusta.

AKK:n 2026 sääntöjen 10.11 mukaan käymättä jäänyt JAT tai maali tarkoittaa keskeyttämistä, ei HYL-tilaa. Kohdan 10.14 ULA:n 600 vp:n säännöt koskevat Emit-ajanottoa. Miehitetyn aseman korttimerkinnän puuttuessa asemapöytäkirjasta löytyvä käynti on erillinen 300 vp:n tapaus: selvitä pöytäkirja, syötä oikea aika ja kirjaa päätöksen mukainen muu virhe käsin.

Lähde: https://www.autourheilu.fi/site/assets/files/1930/akk_autosuunnistus_2026.pdf

Tämä muutos koskee koko kilpailun manuaalista ajanottoa. Emit-kilpailun puhelinaikoja ei siirretä Emit-laskennan päälle. Beginners Cupin erillinen laskenta ja sen puhelinsiirto eivät kuulu tähän muutokseen.
