# AS2026 0.7.3 (valmistelussa)

Päivitys yhdistää uusimman 0.7.2-kilpailutoiminnallisuuden Fluent 2 -käyttöliittymään. Kilpailusäännöt, AT/JAT-laskenta, Emit-historia, hyväksynnät, tulostus, vienti ja LAN-yhteiskäyttö säilyvät Mikky100:n toteutuksen pohjalta.

## Käyttöliittymä

- Yhtenäiset painikkeet, valikot, lomakkeet ja valintaikkunat sekä selkeämmät virhe- ja huomautusnäkymät.
- Kilpailijan ja aseman muokkaus avautuu keskitettyyn ikkunaan. Radan tiedot avataan omasta painikkeesta; radat-lista on asemien alapuolella.
- Emit-editorin kenttien ja painikkeiden välit sekä tulosyhteenvedon pitkien tekstien tilankäyttö on korjattu. Valintalistojen sisältö ei leikkaudu alareunasta.
- Tietoja-ikkunassa on erilliset Lisenssit- ja Ulkoasu-näkymät. Oma lisenssi avautuu linkistä ja Slint-maininta säilyy näkyvissä.
- Teemat: järjestelmän mukainen, vaalea, tumma, Tumma kuusi, Hiillos ja Shanghai. Valinta tallentuu vain tämän koneen sovellusasetuksiin.
- Valinnaiset lyhyet häivytykset ja värisiirtymät. Animaatiot ovat oletuksena pois, eivätkä ne viivästytä kilpailutietojen päivityksiä.
- Ajanottotapojen selitteitä on selkeytetty: Manuaalinen, Emit ja Osittain manuaalinen. Tallennetut arvot ja laskentatavat säilyvät ennallaan.

## Päivitykset ja paketointi

- Päivitykset tarkistetaan taustalla käynnistyksen jälkeen. Päivitystä ei ladata tai asenneta automaattisesti.
- Uusien riippuvuusversioiden lisenssit ja tekijämaininnat toimitetaan molempien käyttöjärjestelmien paketeissa.

Pura koko käyttöjärjestelmäsi paketti. Käynnistä **as2026-gui** tai **as2026-gui.exe** ja valitse asennustoiminto. Pakettiin kuuluvat varmennettava manifesti, käynnistin ja lisenssit.

Fyysinen Emit-lukija, paperitulostin ja kilpailupaikan verkko on tarkistettava käytettävällä kalustolla. Automaattiset testit käyttävät erillisiä esimerkkikilpailuja.

[Käyttöohje](docs/KAYTTOLIITTYMAOPAS.md) · [Lähiverkko](docs/LAHIVERKKO.md) · [Version 0.7.2 muutokset](docs/julkaisut/0.7.2.md)
