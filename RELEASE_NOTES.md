# AS2026 0.7.6

Versio yhdistää 0.7.4:n korjaukset aiemmin valmisteltuihin käyttöliittymä- ja luotettavuusparannuksiin sekä rajattuun teemavalikoimaan. Kaikki 0.7.4:n kilpailu-, KITI-, PDF-, Emit- ja livetuloskorjaukset säilyvät.

- **Kymmenen ulkoasuvaihtoehtoa:** Järjestelmän mukaan, Vaalea, Tumma, Tumma kuusi, Hiillos, Shanghai, Yömeri, Kanerva (laventeli), Ruusu ja Revontuli.
- **Hillitty valikoima:** keskenään liian samankaltaiset paletit on karsittu. Viisi ensimmäistä vaihtoehtoa ja laventelinsävyinen Kanerva säilyvät.
- **Vanhat ulkoasuasetukset:** poistuneen teeman tallennettu nimi avautuu samansävyisessä säilytetyssä teemassa. Muut asetukset ja kilpailutiedot säilyvät.
- **Yhtenäinen valinta:** samat vaihtoehdot sovelluksessa ja komponenttigalleriassa. Valinta tallentuu vain omalle tietokoneelle; animaatiot ja tehosteet ovat oletuksena pois.

- **Luokkavalinta:** tulostuksessa ja live-julkaisussa kaksi koko alaltaan klikattavaa korttia rivillä, haku, valittujen määrä ja joukkovalinnat. PDF-viennin järjestysnumerot säilyvät 0.7.4:n mukaisina.
- **Tulosten numerohaku:** pieni Nro-kenttä hakee vain täsmällistä kilpailunumeroa muun haun ja luokkarajauksen rinnalla.
- **Kohdistuskehys:** näkyy vain Tabilla aloitetussa näppäimistönavigoinnissa. Hiiriklikkaus piilottaa sen. Kehys vastaa koko klikattavaa aluetta, myös luokkakorteissa ja kytkimissä.
- **Valintaikkunat:** korkeus mukautuu sisältöön ja ikkunan kokoon; otsikko ja toiminnot pysyvät näkyvissä.
- **Live-julkaisun asetukset:** harvemmin tarvittavat asetukset on koottu lisäasetuksiin. Yhteystavan vaihto vaihtaa myös oletusportin, ja tulostiedoston oletusnimi muodostetaan kilpailun oikeasta nimestä pienaakkosilla. FTP-vastausten käsittelyä ja julkaisun herätejonoa on vahvistettu.
- **Tallennus ja päivitykset:** tuntemattomat sovellusasetukset säilyvät, tilapäistiedostot luodaan suojattuina, päivityspakettien nimet ja tarkistussummat validoidaan tarkasti. Windowsin asennuksen hetkellisiä tiedostolukkoja yritetään uudelleen.
- **Lähiverkon palautuminen:** taustatöiden virhetilat ja LAN-liittymisen aloitus käsitellään turvallisemmin. Kahden GUI:n palautumiskoe varmistaa Emit-lukujen, korjausten ja hyväksyntöjen säilymisen yhteyskatkossa.

Ulkoasu vaihdetaan kohdasta **Tietoja → Ulkoasu**. [Teemat ja sävyt](docs/TEEMAT.md).

[Version 0.7.4 kilpailukäyttöön liittyvät korjaukset](docs/julkaisut/0.7.4.md) säilyvät: KITI-luokkakohdistukset ja sijoitukset, turvallinen vientitiedostonimi, PDF-luokkajärjestys ja logo, Emit-korjauspohja, täsmennetyt ajanottotermit sekä livetulosten luokkaotsikot.

Pura koko käyttöjärjestelmäsi paketti ja käynnistä **as2026-gui** tai **as2026-gui.exe**. Pakettiin kuuluvat varmennettava manifesti, käynnistin ja lisenssit.

Fyysinen Emit-lukija, paperitulostin ja kilpailupaikan verkko on tarkistettava käytettävällä kalustolla. Automaattiset testit käyttävät erillisiä esimerkkikilpailuja.

[Käyttöohje](docs/KAYTTOLIITTYMAOPAS.md) · [Vienti](docs/VIENTI_JA_HUOMAUTUKSET.md) · [Emit-korjaus](docs/EMIT_RIVIN_KORJAUS.md)
