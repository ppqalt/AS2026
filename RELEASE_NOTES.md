# AS2026 0.5.7

Tämä julkaisu selkeyttää **Radat ja asemat** -näkymän valintaa ja
muokkausta. Kilpailujen tietomalli ja tuloslaskennan aikayksikkö eivät muutu.

## Käyttöliittymä

- Radan tiedot pysyvät näkyvissä, vaikka radalta valitaan muokattava asema.
  Asemalistan otsikko `Rata N · nimi` ei enää hyppää ylös ja alas
  muokkauspaneelin kadotessa.
- Radan numeroa ja nimeä voi muokata samalla kun asema on valittuna.
  Radan numeron vaihtuminen päivittää myös avoimen asemamuokkauksen viitteen.
- Tavoiteaikaa muokataan erillisillä tunneilla, minuuteilla ja sekunneilla
  (h : min : s). Kilpailu tallentaa edelleen täsmälleen samat sekuntimäärät
  `tavoiteaika_sekuntia`-kenttään; muut aikakentät säilyvät ennallaan.
  Virheellisiä minuutti- ja sekuntiarvoja ei hyväksytä.
- Aseman kuusi kenttää näkyvät kahdessa sarakkeessa riittävän leveässä
  ikkunassa ja yhdessä sarakkeessa kapeissa ikkunalaatoissa.
- Ratataulukon koko seuraa tietueiden määrää. Asemalista ja muokkauspaneeli
  ovat erillisiä, jotta käyttöliittymä ei lisää keinotekoisia tyhjiä välejä.
- Emit-lukemisen sarjaporttikentän ylimääräinen "Sarjaportti"-otsikko
  poistettiin; automaattisen porttivalinnan tai käsin annetun COM-portin
  toimintaa ei muutettu.
- Taulukoista poistettiin suuri näppäimistökohdistuksen kehys, sen
  nuolinäppäinnavigaatio ja fokuksen siirtäminen riviklikkauksella.
  Hiirellä valittu rivi näkyy edelleen omalla taustavärillään.
  Tekstikentät toimivat näppäimistöllä normaalisti.

## Yhteensopivuus

AS2026 0.5.6:n Rust-laskenta, Emit-luku ja CSV/XLSX/HTML-viennit
säilyvät. Tämä julkaisu ei muuta AS2015A:n palautettujen laskentasääntöjen
todistusastetta: historiallista täyttä parity-varmennusta ei ole tehty,
ja vanhojen tallennettujen asematulosten täydellinen uudelleenpisteytys
on edelleen keskeneräinen. Vanhat kilpailutiedostot avataan edelleen
vain luettaviksi.

Linux- ja Windows-jakelupaketit rakennetaan, tarkistetaan ja
allekirjoitetaan aiemmalla Ed25519-päivitysten luottamusketjulla.
