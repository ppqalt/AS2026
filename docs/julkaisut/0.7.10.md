# AS2026 0.7.10

- Ponnahdusikkunan hiirirulla ei enää vieritä alla olevaa näkymää.
- Live-tulosten paikallisen kansion polun voi kopioida myös julkaisun ollessa käynnissä.
- Tulosten esikatselu näyttää kuljettajan ja kartanlukijan nimet samalla tyylillä. Kartanlukijan nimi siirtyy tarvittaessa kokonaan seuraavalle riville.
- Oikean rivin tuonti tarkistaa AT-, JAT-, ULA- ja maaliasemien määrät radan tiedoista ennen tallennusta.
- Luokan lähtötiedot ja myöhästymisraja ovat muokattavissa. Minuutteina annettu myöhästymisraja on koko kilpailun yhteinen laskenta-arvo: Luokat ja Asetukset päivittyvät molempiin suuntiin. Tallennetut tulokset lasketaan uudelleen ja hyväksytään uudelleen.
- Asetusten Tulostin-osion alla on Automatiikka. Tulkin tuloslaput sekä valmiin luokan luokka- ja asematulosteet voidaan automatisoida erikseen. Samaa muuttumatonta koontia ei tulosteta toistuvasti.
- Windowsin rinnakkaisten testien väliaikaisten kansioiden nimien törmäys on korjattu.

Pura koko käyttöjärjestelmäsi paketti. Käynnistä Windowsissa
**as2026-gui.exe** tai Linuxissa **as2026-gui**.

Automatiikan valinnat koskevat avointa kilpailua pääkoneella. Tarkista tulostin ennen automatiikan käyttöönottoa.

[Muutosohje](docs/ISSUE_63.md) ·
[Tulostusohje](docs/TULOSTUS.md) ·
[AS tulkin ohje](docs/AS_TULKKI_YHTEYS.md)