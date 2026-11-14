# AS tulkin verkkokilpailu

Kilpailun hallinta tehdään AS2026:ssa. WordPressiin asennetaan AS tulkki -tiedonsiirtolisäosa 0.7.1 tai uudempi; sen omiin asetuksiin ei syötetä mitään.

## Kilpailutietojen lähettäminen

1. Avaa kilpailu pääkoneella. Lisää kilpailuparit ja tuo jokaisen käytettävän radan oikea rivi **Kilvet**-välilehdellä. Tarkista **Ajan syöttö käsin** -valinnat.
2. Valitse **Kilvet → AS tulkki**.
3. Anna sivuston HTTPS-osoite. Oletus on `https://autosuunnistus.net/`.
4. Anna WordPressin ylläpitäjän käyttäjätunnus ja WordPressin **sovellussalasana**. Se luodaan WordPressin käyttäjäprofiilissa, ei lisäosan asetuksissa. Tavallinen kirjautumissalasana ei sovellu tähän.
5. Valitse **Luo verkkokilpailu**. AS2026 tallentaa paikalliset kilpailukohtaiset yhteystunnukset, luo verkkokilpailun ja lähettää kilpailutiedot. Lopuksi näkyvät liittymiskoodi ja palvelimen kuittaama tietoversio.
6. Myöhempi **Lähetä kilpailutiedot** käyttää kilpailun omaa yhteysavainta. WordPressin sovellussalasanaa ei silloin tarvita.

Sovellussalasana säilyy vain yhteysikkunan/taustapyynnön muistissa. Sitä ei kirjoiteta asetustiedostoon eikä anneta puhelimille. Liittymiskoodi, kilpailun yhteysavain, sivusto, WordPress-käyttäjätunnus ja verkkokilpailun tunniste säilyvät tämän koneen käyttäjäasetuksissa kilpailun polkuun sidottuina. Asetustiedosto on pidettävä tämän koneen käyttäjän hallussa. Ohjelma ei siirrä näitä tunnuksia kilpailun TOML-tiedostoon tai LAN-asiakkaille.

Yhteyskatkossa sama luontitunniste ja avaimet säilyvät. Uusi lähetys jatkaa luontia; älä luo korvaavaa verkkokilpailua toisella koneella. Jos palvelin ehti luoda kilpailun mutta kilpien lähetys epäonnistui, kilpailun tunniste tallennetaan silti ja sama lähetyspainike jatkaa siitä.

## Tulkintojen vastaanotto

Puhelimella avataan `https://autosuunnistus.net/as-tulkki/` ja liitytään AS2026:n näyttämällä koodilla sekä tarkastajan nimellä. Lisäosaversio 0.3.0 sisältää selainversion. Valmiiksi merkityt jaksot lähetetään palvelimelle; keskeneräiset luonnokset säilyvät puhelimella.

AS2026 vastaanottaa avoimen verkkokilpailun tulkinnat pääkoneella automaattisesti noin 15 sekunnin välein edellisen haun valmistumisesta. Avaa **Kilvet → Näytä tulkinnat**. Taulukossa näkyvät kilpailuparit, tulkattujen jaksojen tila, kilvipuutteet, aikapuutteet ja muut virheet. Kaikkien radan jaksojen valmistuttua parin tilana on **Tulkattu**. Osittainen tulkinta näyttää esimerkiksi **2/4 jaksoa**.

Avaa pari nähdäksesi jaksoittaiset puuttuvat kilvet, ajat, muut virheet, huomautukset ja tarkastajan. Saman jakson samannumeroiset kilvet erotetaan jakson rivinumerolla. Korjaus korvaa vain kyseisen jakson aiemman tulkinnan. Parin muut jaksot säilyvät, eikä virhemääriä lasketa uudelleenlähetyksestä kahteen kertaan.

Tulkinnat ja muutossyötteen noutokohta tallennetaan samoihin paikallisiin asetuksiin kuin yhteys. Katkossa aiemmat tiedot säilyvät ja yhteystila kertoo, ettei vastaanotto ole ajan tasalla. Päivitystä yritetään uudelleen automaattisesti. **Päivitä tulkinnat** käynnistää haun myös käsin. Aiemman kehitysversion jo luoma verkkokilpailu toimii ilman kilpailun uudelleenluontia tai uutta WordPress-kirjautumista.

## Tuloslaskenta ja rajaukset

**Tulokset**-näkymän Tulkkaus-sarake näyttää vastaanotetun tilan. **Näytä tulkinta** avaa jaksoittaiset puuttuvat kilvet, ajat ja huomautukset.

Puhelimelta saadut puuttuvien RT-asemien määrät siirtyvät automaattisesti olemassa olevaan **Puuttuvat RT-asemat** -laskentakenttään. Pisteet lasketaan kilpailun RT-virheen asetuksesta. AS tulkin 0.7.2:ssa sadan pisteen askelin kirjatut muut virhepisteet siirtyvät **Muut virhepisteet** -kenttään sellaisinaan. Aiemmat kappalemäärät pisteytetään ylimääräisen AT-merkinnän pistemäärällä (oletus 600 vp/kpl). Manuaalisessa ajanotossa myös puhelimen ajat ja puuttuvat aikamerkinnät siirretään laskentaan. Katso MANUAALINEN_AJANOTTO.md. Emit-kilpailun aikoja ei korvata puhelimesta.

Korjattu tulkinta korvaa aiemman puhelinosuuden. Toistuva vastaanotto ei lisää pisteitä uudelleen. Muiden virhepisteiden erillinen käsin lisätty osuus säilyy. Puhelimen RT-määrä on vastaanotetun tulkinnan mukainen. Siirtotunniste tallentuu kilpailutiedostoon. Pisteisiin vaikuttava muutos poistaa aiemman hyväksynnän; oletuksena lopputulos hyväksytään käsin. AS tulkki -yhteysikkunassa voi valita valmiiden tulosten automaattisen hyväksynnän ja tulostuksen. Keskeneräisen muokkauksen aikana laskentapäivitystä odotetaan muokkauksen päättämiseen.

Vastaanoton välimuisti on pääkoneen asetuksissa. Laskentakenttiin tallennetut tiedot kuuluvat kilpailutiedostoon. Beginners Cupin erillisen laskennan automaattinen siirto toteutetaan erikseen.
Oikean rivin kilpikorjaukset, mitätöinti ja ULA:n lisääminen JAT:n jälkeen ovat käytössä. Tulkatut jaksot ja kohdistuvat kirjaukset säilyvät. Uuden ULA:n tuntematon käsin syötettävä aika jää täydennettäväksi. Vanhojen aika-asemien ja jaksorajojen muutokset on suojattu. Katso AS_TULKKI_MUUTOKSET.md.

Ensin käytetään koekilpailua. Ensimmäinen julkaisu käynnistetään lähetyspainikkeesta. Julkaistun kilpailun sallitut kilpikorjaukset lähetetään taustalla. Verkkotyö tehdään taustasäikeessä; virhe ei kuittaannu onnistuneeksi. Lähetyksen aikana yhteysikkuna ei sulkeudu eikä kilpailua vaihdeta.
