# AS2026 0.7.4

Versio toteuttaa issuet #45–#52 ja säilyttää 0.7.3:n Fluent 2 -käyttöliittymän.

- **KITI-vienti:** jokaiselle pohjan luokalle valitaan pakollinen perusluokka ja vapaaehtoinen yhdistetty luokka. Molemmissa listoissa ovat kaikki tulosluokat. Perusluokan sija menee aina Lkasij.-sarakkeeseen, yhdistetyn luokan sija Sij.-sarakkeeseen. Ilman yhdistettyä valintaa molemmat sijoitukset tulevat perusluokasta. Valinnat muistetaan.
- **KITI-tiedostonimi:** pohjan nimi + `-valmis.xlsx`, samassa kansiossa. Alkuperäistä pohjaa ja olemassa olevia tulostiedostoja ei korvata.
- **PDF-vienti:** valittujen luokkien järjestyksen voi määrätä järjestysnumeroilla. Järjestys koskee myös Beginners Cupia ja asemakohtaisia tuloksia.
- **PDF-logo:** logo sijoittuu 10 mm:n marginaalien sisälle kuvasuhteensa säilyttäen. Otsikot väistävät logoa myös vaakatulosteessa ja leveällä logolla.
- **Emit-korjaus:** radan Emit-numerot voi tuoda pohjaksi myös olemassa olevaa korttilukua korjattaessa. Kellonajat täydennetään kilpailukortista; alkuperäinen lukema säilyy historiassa.
- **Termit:** Manuaalinen aika, Kokonaismyöhästymisraja, Jaksomyöhästymisraja ja Virhevapaa aika ovat käytössä käyttöliittymässä ja ajantasaisissa ohjeissa.
- **Livetulokset:** yhdistettyjen luokkien otsikoissa näkyy luokan nimi ilman sisäistä Y1-/Y2-tunnistetta.

Pura koko käyttöjärjestelmäsi paketti ja käynnistä **as2026-gui** tai **as2026-gui.exe**. Pakettiin kuuluvat varmennettava manifesti, käynnistin ja lisenssit.

Fyysinen Emit-lukija, paperitulostin ja kilpailupaikan verkko on tarkistettava käytettävällä kalustolla. Automaattiset testit käyttävät erillisiä esimerkkikilpailuja.

[Käyttöohje](docs/KAYTTOLIITTYMAOPAS.md) · [Vienti](docs/VIENTI_JA_HUOMAUTUKSET.md) · [Emit-korjaus](docs/EMIT_RIVIN_KORJAUS.md) · [Version 0.7.3 muutokset](docs/julkaisut/0.7.3.md)
