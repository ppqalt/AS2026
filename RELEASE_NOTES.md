# AS2026 0.6.3

## Korjaukset ja parannukset

- Fluent 2 / Dark Spruce -ulkoasun painikkeet, kentät, näppäimistökohdistus, tekstikoot ja välit noudattavat yhteisiä sääntöjä. Painikkeilla on selkeät nimet ilman turhia kolmea pistettä. Harvemmin tarvittavat toiminnot ja vientimuodot löytyvät nimetyistä valikoista.
- Valintaikkunoiden otsikot ja toimintopainikkeet pysyvät näkyvissä pitkän sisällön vieriessä. Peruuta ja vahvistus ovat yhdessä oikeassa alakulmassa. Vierityspalkit ovat oikealla, eivät kenttien päällä; pitkällä lomakkeella on yksi pystyvieritys.
- Tulokset ja muut sivut käyttävät samaa tiivistä sivuasettelua. Huomautukset avataan tarvittaessa, ja tyhjä tulosluettelo käyttää koko työalueen. Puuttuvat arvot näytetään merkillä `-`.
- Kenttäkohtainen virhe näkyy kyseisen kentän yhteydessä. Kelvolliset muutokset ja valmiit uudet tietueet tallentuvat automaattisesti. Keskeneräinen luonnos säilyy saman käyttökerran sivunvaihdoissa. Luokan lähtöaika käyttää samoja tunti-, minuutti- ja sekuntikenttiä kuin muut kellokentät.
- Excel-kilpailun tuonti löytyy Yleiskatsauksesta. Esikatselusta voi tuoda koko kilpailun tai pelkät radat, ja luokkien radat voi täydentää myöhemmin. AS2014A-pohjan kaikki 70 kilpailijaa säilyvät tuonnissa myös puuttuvilla Emit-numeroilla tai lähtöajoilla; valinnaisen tiedon virhe ei poista koko kilpailijaa.
- Emit-lukijan käynti tai keskeneräinen lomake ei enää tarpeettomasti estä muita toimintoja. Kilpailua vaihdettaessa lukija pysäytetään hallitusti ja tallentamattomat lukemat tarkistetaan edelleen. Lähtöarvonnan valitut luokat, jo luettujen kilpailijoiden suojaus ja tulosten hyväksyntärajat säilyvät.
- Kilpailun laskentalogiikka, tiedostomuodot sekä Rust- ja Slint-toteutus säilyvät. Yhteiset käyttöliittymäsäännöt on kirjattu tiedostoon `docs/UI_PERIAATTEET.md`.

## Varmennuksen rajat

Rust-testit ja Linux/Windows CI tarkastavat muutoksia automaattisesti ennen julkaisemista. Synteettiset kilpailukokeet eivät korvaa oikean Windows-koneen ja fyysisen Emit-lukijan käyttökoetta tai vertailua historiallisiin kilpailutuloksiin. Suosittelemme harjoituskilpailua ennen virallista käyttöönottoa.
