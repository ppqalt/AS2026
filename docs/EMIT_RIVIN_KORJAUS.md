# Emit-rivin korjaus

Versiossa 0.6.8 korjaus avataan Emit-sivulta. [Uusien toimintojen käyttöohje](KAYTTOLIITTYMA_068.md).

## Käyttö

1. Valitse **Emit**, aktiivinen lukutapahtuma ja korjattava asemarivi.
2. Valitse luetun sisällön otsikon vierestä **Muokkaa**.
3. **Muokkaa / lisää leima** korjaa valitun leiman koodin ja korttiajan.
   Sekuntiaika on aika Emit-kortin nollauksesta. Rinnalla oleva hh:mm:ss-kellonaika lasketaan kilpailijan lähtöajasta ja Emit-korjauksesta; kentät päivittyvät toistensa mukana. Puuttuvan aseman
   kohdalla lisätään leima. **Kohdistettava leima** voi siirtää olemassa olevan
   ylimääräisen leiman puuttuvalle asemalle: valitse leima ja tarkista koodi ja aika.
4. **Poista leima laskennasta** jättää kyseisen leiman pois laskennan syötteestä.
   Alkuperäinen leima säilyy lukuhistoriassa.
5. Anna perustelu ja valitse **Tallenna korjaus**. Virheellinen syöte jää
   valintaikkunaan korjattavaksi. Peruuta tai Esc ei tallenna luonnosta.

Käsikello korjataan entiseen tapaan asematuloksen kellokentästä. Jaetun ULA:n
aikaviite korjataan sitä edeltävän saman leiman aseman kautta. Leimojen on
oltava aikajärjestyksessä; mahdotonta tai toistuvan koodin väärään asemakohtaan
kohdistuvaa muutosta ei tallenneta.

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
  uutena perusteltuna tapahtumana. Aikaisempia korjauksia ei poisteta.
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
