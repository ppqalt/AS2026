# Issue 68 – käyttöliittymän korjaukset

- Oikean rivin onnistunut tallennus näkyy tuonti-ikkunassa vihreänä. Vahvistuksen väri palautuu neutraaliksi, kun valitaan uusi tuonti tai tallennus epäonnistuu. Yhteiset onnistumisilmoitukset ja AS tulkin onnistunut tiedonsiirto näkyvät teeman vihreällä; virheet punaisella.
- Verkkokilpailun luonti erottaa luokalta puuttuvan radan ja radalta puuttuvan oikean rivin. Viesti ohjaa Luokat- tai Kilvet-näkymään.
- Liittymisen QR-ikkunassa on Tulosta QR-koodi. Ohjelman normaali tulostusjono tuottaa yhden A4-sivun, jolla ovat kilpailun nimi, suuri QR-koodi, liittymiskoodi, liittymisohje ja linkki. Mukana on QR-koodin neljän ruudun suoja-alue.

QR muodostetaan paikallisesti ja sisältää saman julkisen liittymislinkin kuin puhelimelle näytetty koodi. Verkkokilpailun hallinta-avainta tai WordPress-salasanaa ei tulosteta.

Tarkistukset: ratatietojen virheilmoitusten regressiotesti; QR-tulosteen dekoodaus Windowsin varsinaisen GDI-renderöijän tuottamasta kuvasta; tulostus- ja raporttitestit; natiivi GUI-koe 100/150 %:n skaalauksilla ja synteettisellä kilpailulla. Fyysistä tulostinta ei käytetä automaattikokeessa.
