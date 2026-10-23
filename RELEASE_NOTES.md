# AS2026 0.7.0

0.7.0 kokoaa kilpailukäytön korjaukset Mikky100:n uusimman 0.6.11-toteutuksen
päälle. Aiemmat kilpailuasetukset, Emit- ja AT/JAT-laskenta, lähtöluettelon
sarakevalinta, hyväksyntöjen käsittely ja LAN-tilaviestit säilyvät mukana.

## Korjaukset

- Beginners Cupin maali ei enää anna aikarangaistusta ennakosta tai myöhästä.
  Korjaus koskee johdettua BC-rataa; NAC-lähderadan laskenta säilyy ennallaan
  ([issue #25](https://github.com/ppqalt/AS2026/issues/25)).
- Tavallisen muokkauskentän Enter vahvistaa kelvollisen arvon ja poistaa kentän
  kohdistuskorostuksen. Virheellinen syöte jää korjattavaksi. Erillisten liittymis-,
  tallennus- ja tulosikkunoiden omat Enter-toiminnot säilyvät.
- Rataluettelo ja radan tietopaneeli käyttävät samaa korkeutta rinnakkaisnäkymässä.
- Sivupalkin yhteyspainike käyttää yhteistä Fluent-painiketta ja on kohdistettu
  navigoinnin kanssa.
- Tilapalkin erillinen ”Näytä virhe” -painike on poistettu. Virheteksti säilyy
  tilapalkissa; tallennusvirheitä ei kuitata onnistumisiksi.

## Kilpailukäytön taustakorjaukset

- LAN-kuittauksen lähetysjonon tallennusvirhe säilyy näkyvänä ja tallennusta
  yritetään uudelleen. Tapahtumatunnusten kaksoiskäsittelysuoja säilyy.
- Paikallisen livejulkaisun pysäytys ja uudelleenkäynnistys eivät jätä vanhaa
  odottavaa kirjoitusta korvaamaan uudempaa sivua.
- FTP-palvelimen vaatima tiedostonvaihto säilyttää aiemman sivun varakopiona
  onnistuneeseen vaihtoon asti ja yrittää palauttaa sen vaihtovirheessä.
- Muuttumaton livejulkaisun tilanne ei enää kopioi koko kilpailua taustalla
  jokaisella tarkistuskierroksella.
- Tulostinluettelo haetaan taustalla. Hidas tulostuspalvelu ei pysäytä
  käyttöliittymää, ja haun aikana tehty tulostinvalinta säilytetään.

## Mukana säilyvät 0.6.11:n toiminnot

- BC:n aikojen ja virhepisteiden erillinen syöttö NAC-tuloksen tallentamisen jälkeen.
- Julkaistavien luokkien valinta, tiiviit luokkakohtaiset tulossivut ja paikallinen live.
- Alkuperäiset Emit-luvut, jäljitettävät leimakorjaukset ja hyväksyntöjen tarkistukset.
- Lähiverkon samanaikainen käyttö, muokkausristiriitojen suojaus ja katkon lähetysjono.
- Sisäänrakennettu SFTP/FTP-siirto sekä Windowsin rajattu tiedostolukitusten uudelleenyritys.

## Käyttöönotto

Lataa ja pura oman järjestelmäsi koko paketti, käynnistä `as2026-gui` /
`as2026-gui.exe` ja valitse asennustoiminto. Linux- ja Windows-arkistojen mukana
ovat allekirjoitettu `release.json`, `release.sig`, käynnistin ja riippuvuuksien
lisenssit. Aikaisempia julkaisupaketteja tai tageja ei korvata.

Varmuuskopioi koko kilpailukansio ennen päivitystä. Käytä kaikilla LAN-koneilla
samaa versiota. Kokeile kilpailukaluston Emit-lukija, tulostin ja lähiverkkoyhteys
erillisellä esimerkkikilpailulla ennen varsinaista kilpailua. Automaattikokeet
käyttävät synteettistä aineistoa, eivät oikeita kilpailuja tai käyttäjäasetuksia.

[Kilpailupäivän tarkistuslista](docs/KILPAILUPAIVA.md) ·
[Käyttöliittymäopas](docs/KAYTTOLIITTYMAOPAS.md) ·
[Livejulkaisu](docs/LIVE_JULKAISU.md) ·
[0.6.11:n julkaisutiedot](docs/historia/JULKAISU_0611.md)
