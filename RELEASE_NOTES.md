# AS2026 0.7.1

Versio 0.7.1 selkeyttää kilpailijoille julkaistavat tulokset ([issue #27](https://github.com/ppqalt/AS2026/issues/27)).

- Julkisista luokka-, yhdistettyjen luokkien, rata- ja BC-tuloksista poistettiin
  kaikki toimitsijan varoitus- ja infolaatikot, hyväksyntä- ja tasapisteselostukset,
  julkaisematta jääneiden tietueiden laskurit sekä luokkien osallistujalaskurit.
- Keskeneräisen luokan yhteydessä näkyy vain lyhyt **Alustavat tulokset**.
  Julkaisemattomat nousu- ja sarjapisteet näkyvät edelleen viivana.
- HTML-vienti, selaimen tulostus ja paikallinen HTML-live käyttävät samaa
  pelkistettyä tulossisältöä. Tiiviit kiertävät live-sivut säilyvät ennallaan.
- Komentorivin tekstimuotoisessa tulosluettelossa ei ole diagnostiikkaa.
  Toimitsijan tarkistustiedot tulostuvat erilliseen stderr-virtaan.
- Pistelaskenta, hyväksyntävaatimukset, sijoitukset, keskeytykset ja tietueet
  säilyvät ennallaan. Täydellinen diagnostiikka on edelleen toimitsijan
  Huomautukset / Näytä lisätiedot -näkymässä ja laskennan esikatselussa.

Esimerkiksi 13 käsikellohuomautuksen ja 44 hyväksymättömän tietueen tilanteessa
julkinen sivu alkaa nyt luokan otsikosta ja tulostaulukosta. Hyväksymättömät
kilpailijat odottavat edelleen toimitsijan tarkastusta eivätkä saa sijoitusta.

Lataa ja pura koko oman käyttöjärjestelmäsi paketti. Käynnistä `as2026-gui`
tai `as2026-gui.exe` ja valitse asennustoiminto. Linux- ja Windows-pakettien
mukana ovat allekirjoitettu manifesti, käynnistin ja riippuvuuksien lisenssit.

[Kilpailupäivän ohje](docs/KILPAILUPAIVA.md) · [Tulosvienti](docs/VIENTI_JA_HUOMAUTUKSET.md)
