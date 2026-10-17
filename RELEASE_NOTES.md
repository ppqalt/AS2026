# AS2026 0.6.0

AS2026 0.6.0 tuo **Fluent 2 / Dark Spruce** -käyttöliittymän koko ohjelmaan
ja lisää kilpailutoimistolle olennaisia tarkastus- ja lähtöjärjestelytoimintoja.
Sovellus toimii edelleen natiivisti Rustilla ja Slintillä Linuxissa ja Windowsissa.
Kaikki käyttäjälle näkyvät tekstit säilyvät suomenkielisinä.

## Fluent 2 ja käyttöliittymä

- Lähes musta, hillityn metsänvihreä Dark Spruce -teema on kaikkien seitsemän
  näkymän yhteinen suunnittelukieli. Valinnat ja ensisijaiset toiminnot
  korostuvat vihreällä, eivät koristeellisilla tehosteilla.
- Slint 1.16:n Fluent-vakiokomponenteista on mukana paikallinen sovitus,
  josta on poistettu animaatiot, läpikuultavuus, varjot ja liukuvärit.
  Mukana ovat sovituksen alkuperäiset lisenssit ja tekijämaininnat.
- Sivupalkin valintamerkki on keskitetty ja taulukoiden otsikot ja rivit
  vierivät yhdessä kapeissa ikkunoissa. Lomakkeet, editorit ja painikkeet
  käyttävät yhteisiä mittoja ja värejä. Valintarivin suuri näppäimistökehys
  ei palaudu; lomakkeiden näppäimistökäyttö säilyy.
- Radan tiedot pysyvät näkyvissä asemaa valittaessa. Asemien tavoiteaika
  syötetään tunneissa, minuuteissa ja sekunneissa, mutta tallennetaan
  edelleen sekunteina.
- Ikkunoiden asettelussa vältetään Slintin sisäkkäisistä leveyksistä
  syntyvät riippuvuussilmukat. Käyttöjärjestelmän omat ikkunakoristeet ja
  tiedostonvalitsimet noudattavat järjestelmän valintoja.

## Kilpailun hallinta

- **Tarkista…** palauttaa asemarivit aktiivisen kortinlukulokin perusteella
  vahvistuksen jälkeen. **Tutki** laskee tallennetuista asemariveistä ja
  käsikelloista; erillisiä kellon ja pisteiden käsikorjauksia voidaan tehdä.
- Kilpailijoille voi asettaa alkuperäisiä kilpailutiloja (TRK, KESK, PER,
  HYL, SULJ, ILM, EILÄ), perustelut sekä kuljettajan ja kartanlukijan
  lisenssitiedot. Tilat ja niiden syyt näkyvät tulosviennissä.
- **Arvo lähtöajat…** näyttää ennen tallennusta esikatselun, joka voidaan
  hyväksyä tai perua. HTML- ja CSV-lähtöluettelot tukevat käytännön
  kilpailujärjestelyjä.
- Vanhojen kilpailu.toml-tiedostojen valinnaiset lisäkentät säilyvät
  yhteensopivina. Tallennus säilyttää aiemman version varmuuskopion.

## Varmennuksen rajat

Pistelaskennan ja kilpailutoimintojen testit perustuvat pääosin synteettisiin
vertailuihin sekä aiemmasta AS2015A:sta palautettuihin sääntöihin.
Täyttä tulosvertailua oikeaan historialliseen kilpailuun ei ole vielä tehty.
Emit-ajanottotapojen 1 ja 2 leimaamattoman maalin Tutki-laskenta ei ole
käytettävissä ilman vahvistettua alkuperäisen ohjelman laskentasääntöä.
Fyysisen lukijan ja valmiin Windows-paketin toiminta on lisäksi hyvä tarkistaa
todellisella kilpailulaitteistolla.

Linux- ja Windows-julkaisut tarkistetaan GitHub Actionsissa. Hyväksytyt
paketit allekirjoitetaan aiemmalla Ed25519-päivitysten luottamusketjulla.
