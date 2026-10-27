# Emit-kortin leimausten korjaus

Avaa Emit-sivun lukutapahtumasta **Muokkaa leimauksia** tai Kilpailijat-sivun
valitun kilpailijan tiedoista **Emit-leimat / Luo leimarivi**.

- Valitse kilpailija. Kohdistamattomassa, vääränumeroisessa korttiluvussa valinta
  on tehtävä erikseen ennen tallennusta.
- Korjaa tarvittaessa Emit-kortin numero. Alkuperäinen lukema säilyy historiassa;
  numerokorjauksesta syntyy jäljitettävä lukema.
- Taulukossa on yksi leimasin ja kellonaika per rivi. Kellonaika käy muodossa
  hhmmss tai hh:mm:ss. Korttiaika lasketaan lähtöajasta ja Emit-korjauksesta.
- **Poista** poistaa rivin luonnoksesta heti. **Lisää leima** lisää rivin kyseisen
  rivin jälkeen. **Lisää alkuun** lisää ensimmäisen rivin.
- Valitse rata ja **Tuo radan Emit-numerot**: radan leimasimet tulevat
  luonnoksen pohjaksi ja kellonajat jäävät täytettäviksi kilpailukortista.
  Toiminto käy myös jo luetun kortin korjauksessa. Pohjan tuonti korvaa
  luonnoksen rivit; **Palauta alkuperäiset leimat** tai **Peruuta** palauttaa
  alkuperäiseen. Tyhjiä kellonaikoja ei voi tallentaa.
- **Tallenna korjaus** laskee tulokset uudelleen. **Peruuta** hylkää luonnoksen.
  Alkuperäiseen palautus muuttaa luonnosta, ei poista korjaushistoriaa.

Saman leimasimen ja 99/999-koodin vuorottelu yhdistetään laskennassa yhdeksi
leimaksi, jonka aika on ensimmäisen oikean leiman aika. Raakaluku säilyy.
Puuttuvan JAT:n tai maalin lisääminen poistaa puuttumiseen perustuvan
keskeytyksen uudelleenlaskennassa. Erillisestä syystä käsin asetettu tila säilyy.

NAC- ja BC-tulokset lasketaan yhteisistä korjatuista leimoista. Molempien hyväksyntä
poistuu. Manuaaliset ajat ja erilliset RT-/muut lisävirheet säilyvät rata-aseman perusteella;
asemarivien pistekorjaukset korvataan uudella laskennalla. Virheellinen muutos ei
tallenna osittaista tulosta. Vanhat kilpailutiedostot avautuvat edelleen.

Regressiot: `tests/emit_corrections.rs`, `tests/release_072.rs` sekä natiivi
`AS2026_EMIT_CORRECTION_TEST=1`-GUI-koe. Kokeet käyttävät synteettistä kilpailua.
