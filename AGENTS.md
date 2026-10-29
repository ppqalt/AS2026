# AS2026:n kehityssäännöt

- Projektin oma lähdekoodi, testit, työkalut ja automaatio toteutetaan Rustilla.
  Pythonia ei lisätä lähdekoodiin, skripteihin, testeihin, työkaluihin tai
  workflow-tiedostoihin. Ei Python-tulkkia, -sidoksia tai Pythonia käynnistäviä
  rakennus- ja testikomentoja. Historiallinen dokumentaatio säilytetään.
- `tools/checks` tarkistaa koko versionhallintaan kuuluvan repositorion.
  `check-no-python` on pakollinen Linuxin ja Windowsin CI-tarkistus.
- Säilytä kahden erillisen GUI-prosessin LAN-, verkkokatko- ja palautumiskokeen
  kattavuus. Tarkistuskomennot ja eristetyt testit kuvataan `docs/BUILDING.md`:ssä.
- Mikky100:n uusimmat kilpailusäännöt ja toteutukset ovat ensisijaisia.
  Älä heikennä testejä tai muuta julkaistuja tageja.
- Käyttöliittymä ja käyttäjädokumentaatio ovat suomeksi. Lue
  `docs/UI_PERIAATTEET.md` ennen käyttöliittymämuutoksia.
