# AS2026 0.7.5-2

Korjausversio siirtää viimeisen oman testityökalun Rustiin ja vahvistaa
projektin automaattisia tarkistuksia. Kilpailulaskenta, käyttöliittymä ja
version 0.7.5-1 luokkavalinnat sekä vientitiedostojen nimet säilyvät.

- Kahden erillisen GUI-prosessin LAN-, verkkokatko- ja palautumiskoetta
  koordinoidaan nyt Rustilla. Emit, käsikellot, pisteet, hyväksynnät,
  tulostusesikatselu ja pysyvä jono tarkistetaan kuten ennenkin.
- Koordinaattori tarkistaa molemmat prosessit, virhetiedostot ja
  palautuneiden kilpailutiedostojen tarkan yhtäläisyyden. Aikakatkaisu
  lopettaa molemmat lapsiprosessit ja säilyttää lokit sekä kuvat.
- Projektin omissa työkaluissa ja automaatiossa ei käytetä Pythonia.
  Versionhallinnan läpikäyvä Rust-tarkistin estää sen lisäämisen takaisin.
  Tarkistimen sekä LAN-koordinaattorin virhepolut testataan erikseen.
- Linuxin ja Windowsin CI käyttävät Rust-koordinaattoria. Dokumentaatio
  ja kehityssäännöt on päivitetty.

**Asennus:** pura koko käyttöjärjestelmäsi paketti ja käynnistä ohjelma.
0.7.5-1 tukee tämän korjausversion automaattipäivitystä. Vanha 0.7.5 ei tue
numeroituja korjausversioita, joten siitä siirryttäessä lataa paketti käsin.
Kilpailutietoja tai käyttäjäasetuksia ei tarvitse poistaa.

[Version 0.7.5-1 muutokset](docs/julkaisut/0.7.5-1.md) ·
[Käyttöohje](docs/KAYTTOLIITTYMAOPAS.md) · [Rakentaminen](docs/BUILDING.md)
