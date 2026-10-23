# Emit-kortin leimausten korjaus

1. Valitse Emit-sivulta aktiivinen korttiluku ja **Muokkaa leimauksia**.
2. Muokkaa leiman Emit-koodia tai hh:mm:ss-kellonaikaa. Korttiaika lasketaan
   automaattisesti kilpailijan lähtöajan ja Emit-korjauksen perusteella.
   Alkuperäinen nollaleima säilyy eikä sille ole muokkauskenttää.
3. Lisää puuttuva leima painikkeesta **+ Lisää leima**. Poista leima valitsemalla
   sen **Leiman käyttö** -kentästä **Poista leima**.
4. Paina **Tallenna korjaus**. Virheellinen syöte jää korjattavaksi.
   **Peruuta** tai Esc hylkää luonnoksen.

Editori ei kysy erillistä perustelua. Alkuperäisiä lukemia ei korvata.
**Palauta alkuperäiset leimat** palauttaa korttisisällön muokkausluonnokseen;
palautus tallentuu uutena korjauksena vasta painikkeesta **Tallenna korjaus**.
Aikaisempia korjauksia ei poisteta. Käsikellot ja muut virhepisteet täydennetään
tuloksen syöttöikkunassa; ne eivät ole Emit-kortin alkuperäisiä leimoja.

## Seuraukset ja jäljitettävyys

- NAC ja Beginners Cup lasketaan samasta korjatusta Emit-syötteestä. Molempien
  hyväksyntä poistuu myös silloin, kun BC:n pisteet eivät muutu. Yhdistetyt
  luokat käyttävät päivitettyjä NAC-tuloksia ja samaa hyväksyntäporttia.
- Käsin annetut kellot säilytetään rata-aseman perusteella muuttuvien
  rivinumeroiden sijaan. Erilliset RT- ja muut lisävirheet sekä HYL/PER/KESK-tila
  säilyvät. Asemariveille käsin kirjoitetut pisteet korvataan uudella laskennalla.
- `leimaluvut` ei muutu. `emit_korjaukset` sisältää alkuperäisen lukeman
  viittauksen, kilpailijan, aseman, perustelun, ajan sekä ennen/jälkeen-leimat.
  Koko korjaushistoria säilyy kilpailutiedostossa.
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

Koe säätää oman ikkunansa myös 900 × 600 loogiseen kokoon. CI ajaa sen
natiivisti Linuxissa ja Windowsissa; fyysistä lukijaa ei avata. Käytä uutta
kuvakansiota. Yleiset GUI-työnkulut tarkistetaan lisäksi 100/150 %:n
skaalauksilla. Kuvat syntyvät oikeasta Slint-ikkunasta, eivät mallikuvista.
