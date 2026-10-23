# Emit-kortin leimausten korjaus

1. Valitse Emit-sivulta aktiivinen korttiluku ja **Muokkaa leimauksia**.
2. Muokkaa koko kortin listasta koodia, korttiaikaa tai hh:mm:ss-kellonaikaa.
   Kellonaika ja korttiaika lasketaan toistensa perusteella käyttäen kilpailijan
   lähtöaikaa ja Emit-korjausta.
3. Lisää puuttuva leima tai poista ylimääräinen leima editorin toiminnoilla.
4. Tallenna. Virheellinen syöte jää korjattavaksi. Peruuta tai Esc hylkää luonnoksen.

Perustelu ei ole pakollinen Mikky100:n nykyisessä editorissa. Alkuperäisiä
lukemia ei korvata. **Palauta alkuperäiset** palauttaa alkuperäisen korttisisällön
uutena korjauksena. Käsikellot ja muut virhepisteet täydennetään tuloksen
syöttöikkunassa; ne eivät ole Emit-kortin alkuperäisiä leimoja.

## Seuraukset ja jäljitettävyys

- NAC ja Beginners Cup lasketaan samasta korjatusta Emit-syötteestä. Molempien
  hyväksyntä poistuu myös silloin, kun BC:n pisteet eivät muutu. Yhdistetyt
  luokat käyttävät päivitettyjä NAC-tuloksia ja samaa hyväksyntäporttia.
- Käsin annetut kellot säilytetään rata-aseman perusteella muuttuvien
  rivinumeroiden sijaan. Erilliset RT- ja muut lisävirheet sekä HYL/PER/KESK-tila
  säilyvät. Asemariveille käsin kirjoitetut pisteet korvataan uudella laskennalla.
- `leimaluvut` ei muutu. `emit_korjaukset` sisältää alkuperäisen lukeman
  viittauksen, kilpailijan, aseman, perustelun, ajan sekä ennen/jälkeen-leimat.
  Viimeiset korjaukset näkyvät myös korjausikkunassa; koko historia on tiedostossa.
- **Palauta alkuperäiset leimat** kumoaa kaikki saman korttisisällön korjaukset
  uutena tapahtumana. Aikaisempia korjauksia ei poisteta.
- Saman korttisisällön uusintaluku säilyttää korjauksen. Eri korttisisältö ei
  peri vanhan lukeman korjauksia. Aiemmat tallennetut tulokset säilyvät kuten
  ennenkin, kunnes toimitsija käyttää Tarkista-toimintoa.
- Tallennus on yksi atominen muutos. Myös BC-kohdistuksen epäselvyys tai
  toisessa ikkunassa tehty tallennus estää koko muutoksen.

Vanhojen kilpailujen avaaminen säilyy: puuttuva korjaushistoria on tyhjä.
Uusia asetuksia tai korjauksia sisältävän kilpailutiedoston avaaminen vanhemmalla ohjelmaversiolla ei ole tuettu.

## Toistettava varmennus

`tests/emit_corrections.rs` tarkistaa laskennan, NAC/BC-hyväksynnät,
käsikellojen säilymisen, puuttuvan leiman, ylimääräisen kohdistuksen ja poiston,
alkuperäiseen palautuksen, virheelliset syötteet, tallennusristiriidan,
uusintaluvun, keskiyön, historian muuttamisen torjunnan ja uudelleenavaamisen.

Todellinen ikkunallinen GUI-koe käynnistyy vain erillisessä esimerkkikilpailussa:

```sh
AS2026_EMIT_CORRECTION_TEST=1 target/debug/as2026-gui --demo --capture-dir KUVAKANSIO
```

Koe odottaa lopuksi, että oma testi-ikkuna asetetaan 900 × 600 loogiseen
kokoon. Paikallinen raporttihakemiston `emit-korjaus-gui.sh` hoitaa koonvaihdon
Nirissä ja toistaa kokeen 100 %:n ja 150 %:n skaalalla. Kuvakaappaukset ovat
käynnissä olevan Slint-ikkunan kuvia. Ne eivät ole Windowsin GUI- tai
fyysisen Emit-lukijan kokeita.
