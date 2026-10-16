# AS2026 0.5.5

Julkaisupaketointi, lisenssitarkistukset, pakettien allekirjoitus ja niiden testit on siirretty Pythonista Rustiin. GitHub Actions käyttää nyt samaa Rust-työkalua Linuxissa ja Windowsissa.

Päivitys säilyttää allekirjoitetun `release.json` / `release.sig` -manifestin rakenteen, nykyisen Ed25519-avaimen ja päivittäjän yhteensopivuuden. AS2026:n kilpailujen käsittelyä, Emit-lukemista, käyttöliittymää tai tuloslaskentasääntöjä ei ole muutettu.

0.5.5 jaetaan allekirjoitettuina Linux- ja Windows-paketteina. Olemassa olevat kilpailut ja asetukset säilyvät asennuksen ulkopuolella.
