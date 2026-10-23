# AS2026 0.6.10

Julkaisu täydentää Mikky100:n 0.6.9-toteutusta. Kilpailusäännöt, AT/JAT-laskenta,
Emit-korjauksen oletus ja kilpailijakohtainen poikkeus, hyväksyntöjen säilyttäminen,
lähtöluettelon sarakkeet sekä LAN-tilaviestit säilyvät.

## Korjaukset

- Koko Toiminnot-valikko toimii: kilpailun avaaminen, luonti, Excel-tuonti,
  luokkien ja yhdistettyjen luokkien lisääminen, kilpailijat, radat, lähtöarvonta,
  kehystiedoston luku, tulostus ja tulosvienti. Korjaa Mikky100:n ilmoituksen #10.
- Vie tulokset avaa alavalikon ja XLSX-, CSV- sekä HTML-tallennuksen oikein.
- Muokkaa kilpailijaa avaa keskitetyn ikkunan. Valmis ja Esc sulkevat sen
  nykyisten tallennus- ja validointisääntöjen mukaisesti.
- Modaalien tausta on neutraali läpinäkyvä musta ilman snapshot-kuvaa tai sumennusta.
- Beginners Cup -painikkeen uusi painallus palauttaa perusnäkymään.
- Asetusten kentät jakautuvat leveässä ikkunassa tasaisiin palstoihin; pieni ikkuna vierii.
- Radan tietokortti ja Emit-sivun rinnakkaistaulukot on tasattu.
- Fluent-valikot avautuvat kentän alapuolelle ja lyhyet listat ilman turhaa vieritystä.
- LAN-ikkunan parituskoodi ja osoite on erotettu selkeästi. Liittyminen onnistuu Enterillä.

## Ohjeet ja varmennus

README ja käännös-/UI-ohjeet on ajantasaistettu. Mukana ovat
[kuvallinen Slint-opas](https://github.com/ppqalt/AS2026/blob/v0.6.10/docs/KAYTTOLIITTYMAOPAS.md),
[LAN-ohje](https://github.com/ppqalt/AS2026/blob/v0.6.10/docs/LAHIVERKKO.md) ja [Windows/UFW-palomuuri](https://github.com/ppqalt/AS2026/blob/v0.6.10/docs/PALOMUURI.md).

Linux- ja Windows-tarkistukset kattavat Rustin, Slintin, Clippyn, GUI-työnkulut
100/150 %:n skaalauksilla, kaikki päävalikkovalinnat hiirellä ja näppäimistöllä,
luokkien tallennuksen/uudelleenavauksen, kolme vientimuotoa ja Emit-editorin.
Kahden synteettisen GUI-prosessin LAN-kokeessa tarkistettiin korttiluvut,
käsikellot, hyväksynnät ja katkon jälkeinen lähetysjonon palautuminen.
Fyysistä Emit-lukijaa tai paperitulostusta ei tämän julkaisun varmennuksessa käytetty.

## Lataaminen ja yhteensopivuus

Linux x86-64: `AS2026-0.6.10-linux-x86_64.tar.gz`.
Windows x86-64: `AS2026-0.6.10-windows-x86_64.zip`.
Molemmat toimitetaan allekirjoitetun `release.json`-manifestin ja `release.sig`-allekirjoituksen kanssa.
Pura koko paketti ja avaa asennusta varten as2026-gui tai as2026-gui.exe.

Kilpailumalliin ei lisätä uutta formaattia tässä versiossa. Varmuuskopioi
kilpailukansio ennen päivitystä; uusia asetuksia sisältävien tiedostojen
avaamista vanhemmalla ohjelmaversiolla ei luvata.

[Aikaisempien versioiden julkaisutiedot](https://github.com/ppqalt/AS2026/blob/v0.6.10/docs/historia/JULKAISUT_068_069.md).
