# AS2026:n tulevien julkaisujen tekeminen

Nykyinen lisenssi on **BSD-2-Clause**. Repositorio pidetään yksityisenä, eikä uusi dev-push tai tagi yksin julkaise GitHub Releaseä. Julkaisutyönkulku käynnistetään tarkoituksella ja sen julkaisuvaihe pitää valita erikseen. Oletuksena syntyvät vain tarkistetut yksityiset Actions-testipaketit.

Paketointi tarkistaa projektin LICENSE-tiedoston koko standarditekstin, Cargo-lisenssitiedon sekä paketin lisenssimetadatan. Väärä oma lisenssi estää paketoinnin ja allekirjoituksen. Riippuvuuksien omat lisenssit säilytetään muuttamatta niitä.

Aiemman 0BSD-koodin oikeuksia ei peruuteta. Vanhoja committeja ja tageja ei muuteta.


Kehitys tehdään `dev`-haarassa. Hyväksytty, testattu versio viedään hallitusti `main`-haaraan. Julkaisut tehdään `vMAJOR.MINOR.PATCH`-tageista; esimerkiksi `v0.5.0`. Cargo.toml:n version on vastattava tagia ja tagin commitin on kuuluttava `main`-haaran historiaan. Kehityshaaran push ei julkaise ohjelmapäivitystä. Olemassa olevaa julkaisutagia tai jakelupakettia ei ylikirjoiteta.

## Paketit

`.github/workflows/release.yml` rakentaa Linux-paketin Ubuntu 22.04:llä ja Windows-paketin Windowsin omalla MSVC-työkaluketjulla. Kummallakin alustalla suoritetaan Cargo-testit, Clippy, muotoilutarkistus ja paketointiskriptien testit ennen release-käännöstä. Ubuntu 22.04 rajaa Linuxin glibc-vaatimusta; ohjelma tarvitsee lisäksi järjestelmän tavalliset Wayland- tai X11-, EGL- ja fonttikirjastot. Windows-käännös ei todista käyttöliittymän tai päivityksen toimintaa fyysisellä Windows-koneella.

Julkaisu sisältää seuraavat tiedostot:

- `AS2026-VERSIO-linux-x86_64.tar.gz`
- `AS2026-VERSIO-windows-x86_64.zip`
- `release.json` ja sen 64 tavun binäärinen Ed25519-allekirjoitus `release.sig`

Arkistojen juuressa ovat `as2026-gui`, `as2026-launcher` ja `as2026` (Windowsissa `.exe`-päätteellä), samat `release.json` ja `release.sig`, projektin `LICENSE` ja `THIRD_PARTY.md` sekä `dependency-licenses/`. Puretun paketin GUI tarjoaa asennustoiminnon, kun allekirjoitettu manifesti on suoritettavan tiedoston vieressä. Latauksen jälkeen paketti puretaan ennen ohjelman avaamista.

`scripts/package_release.py` kerää suoritettavat tiedostot ja käytössä olevan Cargo-riippuvuuspuun lisenssit. Mukaan kerätään myös rakennusaikaisia riippuvuuksia. `dependency-licenses/index.json` luettelee nimet, versiot, lisenssit, tekijät ja lähteet. Crateista puuttuvat lisenssit kopioidaan tarkistetuista, lähdecommitilla kiinnitetyistä `scripts/license-overrides/`-tiedostoista. `drm-fourcc` ja `simd_helpers` ilmoittavat lähdekoodissaan MIT-lisenssin ilman erillistä lisenssitiedostoa; niiden erikseen merkityissä ilmoituksissa yhdistetään tavallinen MIT-teksti Cargo-metatietojen tekijämainintoihin. REUSE-muotoiset SPDX-tekijämaininnat kerätään lisäksi lähdekoodin otsakkeista. Keruu pysähtyy puuttuvaan lisenssiin; riippuvuuden päivitys voi vaatia uuden tarkistetun lisenssikopion. Alkuperäisiä tekijämainintoja ei poisteta. Slintin maininta säilyy GUI:n **Tietoja**-ikkunassa.

## Aitous ja allekirjoitusavain

`release-public-key.hex` sisältää ohjelman hyväksymän Ed25519-julkisen avaimen. Yksityinen avain ei kuulu Git-repositorioon. GitHub Actionsin salaisuuden `AS2026_RELEASE_SIGNING_KEY` arvoksi asetetaan yksityinen PEM-avain. Avaimen paikallinen palautuskopio säilytetään erillään projektista ja varmennetaan turvallisesti. Samaa avainta tarvitaan tulevien päivitysten allekirjoituksiin. Avaimen vaihto vaatii erikseen suunnitellun luottamusketjun ja ohjelmapäivityksen.

Allekirjoitusvaihe saa kirjoitusoikeuden vain GitHub Releaseen. Rakennusvaiheilla on vain lukuoikeus. Allekirjoitussalaisuutta käytetään vasta, kun molempien alustojen tarkistukset ja paketointi ovat onnistuneet. Työnkulku käynnistetään manuaalisesti valitsemalla julkaisutagi. Oletus publish_release=false kokoaa vain testipaketit; allekirjoitus ja GitHub Release edellyttävät erillistä publish_release=true-valintaa. Pull request tai tagin push ei käynnistä julkaisua. GitHubin haarojen ja tagien suojaus kannattaa määrittää niin, että vain ylläpitäjä voi julkaista vakaita tageja.

`scripts/sign_release.py` varmistaa, että yksityisen avaimen julkinen osa vastaa repositorion avainta. Avain kirjoitetaan rajatuin oikeuksin väliaikaiseen tiedostoon vain allekirjoittamisen ajaksi ja poistetaan sen jälkeen. Salaisuutta tai OpenSSL:n yksityiskohtaisia avainvirheitä ei tulosteta lokiin. Kumpikin alustan välipaketti tarkistetaan vielä uudelleen ennen allekirjoitusta. Julkaisu pysähtyy puuttuvaan avaimeen, väärään avaimeen tai muuttuneisiin tiedostoihin.

`release.json` on UTF-8-JSON, ja allekirjoitus koskee sen täsmällisiä tavuja. Rakenne on seuraava:

```json
{
  "schema": 1,
  "license": "BSD-2-Clause",
  "version": "0.5.0",
  "targets": {
    "x86_64-unknown-linux-gnu": {
      "asset": "AS2026-0.5.0-linux-x86_64.tar.gz",
      "files": [
        {"name": "as2026-gui", "size": 12345, "sha256": "..."}
      ]
    },
    "x86_64-pc-windows-msvc": {
      "asset": "AS2026-0.5.0-windows-x86_64.zip",
      "files": [
        {"name": "as2026-gui.exe", "size": 12345, "sha256": "..."}
      ]
    }
  }
}
```

Todellinen tiedostoluettelo sisältää jokaisen hyötytiedoston, myös lisenssit ja käynnistimen. Päivittäjä tarkistaa allekirjoituksen kiinnitetyllä julkisella avaimella sekä jokaisen puretun tiedoston koon ja SHA-256-tiivisteen; ylimääräisiä hyötytiedostoja ei hyväksytä. Arkiston omaa tiivistettä ei sijoiteta manifestiin, koska sama manifesti on arkiston sisällä. HTTPS-lataus ja purettujen tiedostojen allekirjoitetut tiivisteet muodostavat tarkistuksen. `release.json` ja `release.sig` toimitetaan sekä erillisinä ladattavina tiedostoina että pakettien sisällä.

## Tarkistukset paikallisesti

Paketointitestit käyttävät synteettisiä tiedostoja ja testissä muodostettua kertakäyttöistä Ed25519-avainta:

```sh
python3 -m unittest discover -s scripts/tests -v
```

Valmiin paikallisen Linux-käännöksen hyötytiedostot voi koota ilman allekirjoitusavainta:

```sh
python3 scripts/package_release.py prepare --target x86_64-unknown-linux-gnu --output target/release-payload-linux
```

Allekirjoitus tehdään vasta molempien alustojen tarkistetuista hyötytiedostoista. Paikallinen yksittäisen alustan kehityskäännös ei ole allekirjoitettu ohjelmapäivitys. Älä lisää paikallisia kilpailuja, asetuksia, yksityisiä avaimia tai GitHub-tunnuksia julkaisupakettiin.
