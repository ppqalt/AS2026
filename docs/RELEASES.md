# AS2026:n julkaisujen tekeminen

Nykyinen lisenssi on **BSD-2-Clause**. Repositorio on julkinen. Pelkkä main-päivitys tai tagin lähettäminen ei julkaise GitHub Releaseä: julkaisutyönkulku käynnistetään erikseen ja sen julkaisuvaihe pitää valita nimenomaisesti. Oletuksena syntyvät vain Actions-testipaketit.

Paketointi tarkistaa projektin LICENSE-tiedoston koko standarditekstin, Cargo-lisenssitiedon sekä paketin lisenssimetadatan. Väärä oma lisenssi estää paketoinnin ja allekirjoituksen. Riippuvuuksien omat lisenssit säilytetään muuttamatta niitä.

Git-historia ja vanhat tagit säilytetään muuttumattomina.


Hyväksytyt muutokset kootaan `main`-haaraan. Tarpeettomat kehityshaarat poistetaan vasta, kun niiden muutokset on yhdistetty tai erillinen työ on säilytetty palautettavana. Julkaisut tehdään `vMAJOR.MINOR.PATCH`-tageista; esimerkiksi `v0.7.0`. Cargo.toml:n version on vastattava tagia ja tagin commitin on kuuluttava `main`-haaran historiaan. Kehityshaaran push ei julkaise ohjelmapäivitystä. Olemassa olevaa julkaisutagia tai jakelupakettia ei ylikirjoiteta.

## Paketit

`.github/workflows/release.yml` rakentaa Linux-paketin Ubuntu 22.04:llä ja Windows-paketin Windowsin omalla MSVC-työkaluketjulla. Kummallakin alustalla suoritetaan Cargo-testit, Clippy, muotoilutarkistus ja Rust-julkaisutyökalun testit ennen release-käännöstä. Ubuntu 22.04 rajaa Linuxin glibc-vaatimusta; ohjelma tarvitsee lisäksi järjestelmän tavalliset Wayland- tai X11-, EGL- ja fonttikirjastot. Windows-käännös ei todista käyttöliittymän tai päivityksen toimintaa fyysisellä Windows-koneella.

Julkaisu sisältää seuraavat tiedostot:

- `AS2026-VERSIO-linux-x86_64.tar.gz`
- `AS2026-VERSIO-windows-x86_64.zip`
- `release.json` ja sen 64 tavun binäärinen Ed25519-allekirjoitus `release.sig`

Arkistojen juuressa ovat `as2026-gui`, `as2026-launcher` ja `as2026` (Windowsissa `.exe`-päätteellä), samat `release.json` ja `release.sig`, projektin `LICENSE` ja `THIRD_PARTY.md` sekä `dependency-licenses/`. Puretun paketin GUI tarjoaa asennustoiminnon, kun allekirjoitettu manifesti on suoritettavan tiedoston vieressä. Latauksen jälkeen paketti puretaan ennen ohjelman avaamista.

Asennus päivittää myös pysyvän käynnistimen varmennetusta paketista atomisesti ennen aktiivisen version vaihtoa. Jos Windowsissa käynnissä oleva vanha käynnistin estää vaihdon, asennus antaa virheen ja ohjaa käyttämään uuden puretun paketin asennusohjelmaa; aktiivinen versio säilyy ennallaan. Aiemman version palautus varmentaa palautettavan paketin ja odottaa käynnistyskuittausta. Epäonnistuva palautus säilyttää alkuperäisen aktiivisen version.

`tools/release/`-hakemiston Rust-julkaisutyökalu kerää suoritettavat tiedostot ja käytössä olevan Cargo-riippuvuuspuun lisenssit. Mukaan kerätään myös rakennusaikaisia riippuvuuksia. `dependency-licenses/index.json` luettelee nimet, versiot, lisenssit, tekijät ja lähteet. Crateista puuttuvat lisenssit kopioidaan tarkistetuista, lähdecommitilla kiinnitetyistä `scripts/license-overrides/`-tiedostoista. `drm-fourcc` ja `simd_helpers` ilmoittavat lähdekoodissaan MIT-lisenssin ilman erillistä lisenssitiedostoa; niiden erikseen merkityissä ilmoituksissa yhdistetään tavallinen MIT-teksti Cargo-metatietojen tekijämainintoihin. REUSE-muotoiset SPDX-tekijämaininnat kerätään lisäksi lähdekoodin otsakkeista. Keruu pysähtyy puuttuvaan lisenssiin; riippuvuuden päivitys voi vaatia uuden tarkistetun lisenssikopion. Alkuperäisiä tekijämainintoja ei poisteta. Slintin maininta säilyy GUI:n **Tietoja**-ikkunassa.

## Aitous ja allekirjoitusavain

`release-public-key.hex` sisältää ohjelman hyväksymän Ed25519-julkisen avaimen. Yksityinen avain ei kuulu Git-repositorioon. GitHub Actionsin salaisuuden `AS2026_RELEASE_SIGNING_KEY` arvoksi asetetaan yksityinen PEM-avain. Avaimen paikallinen palautuskopio säilytetään erillään projektista ja varmennetaan turvallisesti. Samaa avainta tarvitaan tulevien päivitysten allekirjoituksiin. Avaimen vaihto vaatii erikseen suunnitellun luottamusketjun ja ohjelmapäivityksen.

Allekirjoitusvaihe saa kirjoitusoikeuden vain GitHub Releaseen. Rakennusvaiheilla on vain lukuoikeus. Allekirjoitussalaisuutta käytetään vasta, kun molempien alustojen tarkistukset ja paketointi ovat onnistuneet. Työnkulku käynnistetään manuaalisesti valitsemalla julkaisutagi. Oletus publish_release=false kokoaa vain testipaketit; allekirjoitus ja GitHub Release edellyttävät erillistä publish_release=true-valintaa. Pull request tai tagin push ei käynnistä julkaisua. GitHubin haarojen ja tagien suojaus kannattaa määrittää niin, että vain ylläpitäjä voi julkaista vakaita tageja.

Rust-julkaisutyökalun `sign`-komento varmistaa, että yksityisen avaimen julkinen osa vastaa repositorion avainta. Yksityinen avain käsitellään Rust-prosessin muistissa, eikä siitä kirjoiteta väliaikaista PEM-tiedostoa. Salaisuutta tai avaimen sisältöä ei tulosteta lokiin. Kumpikin alustan välipaketti tarkistetaan vielä uudelleen ennen allekirjoitusta. Julkaisu pysähtyy puuttuvaan avaimeen, väärään avaimeen tai muuttuneisiin tiedostoihin.

`release.json` on UTF-8-JSON, ja allekirjoitus koskee sen täsmällisiä tavuja. Rakenne on seuraava:

```json
{
  "schema": 1,
  "license": "BSD-2-Clause",
  "version": "0.7.0",
  "targets": {
    "x86_64-unknown-linux-gnu": {
      "asset": "AS2026-0.7.0-linux-x86_64.tar.gz",
      "files": [
        {"name": "as2026-gui", "size": 12345, "sha256": "..."}
      ]
    },
    "x86_64-pc-windows-msvc": {
      "asset": "AS2026-0.7.0-windows-x86_64.zip",
      "files": [
        {"name": "as2026-gui.exe", "size": 12345, "sha256": "..."}
      ]
    }
  }
}
```

Todellinen tiedostoluettelo sisältää jokaisen hyötytiedoston, myös lisenssit ja käynnistimen. Päivittäjä tarkistaa allekirjoituksen kiinnitetyllä julkisella avaimella sekä jokaisen puretun tiedoston koon ja SHA-256-tiivisteen; ylimääräisiä hyötytiedostoja ei hyväksytä. Arkiston omaa tiivistettä ei sijoiteta manifestiin, koska sama manifesti on arkiston sisällä. HTTPS-lataus ja purettujen tiedostojen allekirjoitetut tiivisteet muodostavat tarkistuksen. `release.json` ja `release.sig` toimitetaan sekä erillisinä ladattavina tiedostoina että pakettien sisällä.

## Tarkistukset paikallisesti

Rust-julkaisutyökalun testit käyttävät synteettisiä tiedostoja ja tarkistavat muun muassa arkistomuodot, tiedostonimet sekä tiivisteet:

```sh
cargo test --manifest-path tools/release/Cargo.toml
```

Valmiin paikallisen Linux-käännöksen hyötytiedostot voi koota ilman allekirjoitusavainta:

```sh
cargo run --manifest-path tools/release/Cargo.toml -- prepare --target x86_64-unknown-linux-gnu --output target/release-payload-linux
```

Allekirjoitus tehdään vasta molempien alustojen tarkistetuista hyötytiedostoista. Paikallinen yksittäisen alustan kehityskäännös ei ole allekirjoitettu ohjelmapäivitys. Älä lisää paikallisia kilpailuja, asetuksia, yksityisiä avaimia tai GitHub-tunnuksia julkaisupakettiin.

## Uuden version julkaiseminen

Versio [0.7.0](https://github.com/ppqalt/AS2026/releases/tag/v0.7.0) on jo julkaistu.
Sen tagia tai paketteja ei luoda uudelleen. Seuraavat vaiheet koskevat aina
**uutta, vielä käyttämätöntä versionumeroa**. Suorita komennot repositorion juuressa.

1. Yhdistä tarkistetut muutokset mainiin ja päivitä Cargo.toml, Cargo.lock sekä
   julkaisutiedot. Varmista Linuxin ja Windowsin vihreä CI sekä GUI-työnkulut.
2. Varmista Gitin allekirjoitusasetukset ennen tagin luomista. SSH-allekirjoituksessa
   Gitin `gpg.format` on `ssh` ja `user.signingkey` osoittaa käytettävään avaimeen.
   Lisää sen **julkinen avain** GitHub-tilille **Signing key** -tyyppisenä.
   GPG-allekirjoitus käy myös. Tagin tekijän sähköpostin pitää olla GitHub-tilillä
   varmennettu tai tilin oma GitHubin noreply-osoite. Yksityistä avainta ei
   lisätä repositorioon tai lähetetä GitHubiin.
3. Luo **allekirjoitettu annotated-tagi** juuri testattuun commitiin. Alla `vX.Y.Z`
   on paikkamerkki: korvaa se uudella versiolla ennen komentojen suorittamista.

```sh
release_tag=vX.Y.Z
git tag -s "$release_tag" -m "AS2026 ${release_tag#v}"
git verify-tag "$release_tag"
git push origin "refs/tags/$release_tag"
```

SSH:n paikallinen `verify-tag` tarvitsee luotetun julkisen avaimen
`gpg.ssh.allowedSignersFile`-asetuksen osoittamaan tiedostoon. Tämä paikallinen
varmennus ja GitHubin varmennus ovat erillisiä tarkistuksia.

4. Avaa GitHubin [Tags-sivu](https://github.com/ppqalt/AS2026/tags). Varmista tagin
   vihreä **Verified**-merkintä ja avaa sen tiedot: merkinnän pitää koskea tagin
   allekirjoitusta. Pelkkä Verified-commit, `release.sig` tai vihreä CI ei korvaa
   tagin varmennusta. Käytä allekirjoitusta mahdollisuuksien mukaan myös
   julkaisucommitissa. Jos avain puuttuu GitHubista, rekisteröi julkinen avain ja
   tarkista tila uudelleen; olemassa olevaa julkaisutagia ei siirretä.
5. Käynnistä varsinainen julkaisu vasta varmennusten jälkeen:

```sh
gh workflow run release.yml --ref main -f release_tag="$release_tag" -f publish_release=true
```

6. Odota julkaisutyönkulun valmistumista. Tarkista, että vakaa julkaisusivu on
   julkinen ja sisältää Linux- ja Windows-arkistot sekä `release.json`- ja
   `release.sig`-tiedostot. Lataa paketit julkisista osoitteista, varmista
   allekirjoitus ja manifestin tiedostot sekä kokeile ladattua sovellusta
   erillisellä esimerkkikilpailulla. Säilytä fyysisen kaluston testauksen rajat
   raportissa.

Käynnistäminen ei vielä tarkoita, että lataukset ovat saatavilla. Puuttuva
allekirjoitusavain tai epäonnistunut rakennus pysäyttää julkaisun; sitä ei kierretä
allekirjoittamattomilla paketeilla tai ohitetuilla testeillä.
