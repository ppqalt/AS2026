# AS2026 0.6.1

AS2026:n toimitsijatyönkulku on laajentunut. Koko käyttöliittymän suunnittelukieli on edelleen Fluent 2 / Dark Spruce.

## Uutta

- Emit-kortin tallentunut luku siirtyy Tulokset-näkymään oikealle kilpailijaparille, ellei toimitsijalla ole keskeneräistä muokkausta.
- Tulokset-näkymässä ovat kilpailijan ajat, asemarivit, viisi virhepisteosaa ja käsin syötettävät asemakellot. Tuloksen voi tutkia ja hyväksyä.
- Hyväksyntä huomioi puuttuvat käsikellot, virheelliset ajat, JAT-puutteet ja myöhästymisrajat. TRK/KESK ja keskeytyssyyt määräytyvät laskennasta; käsin asetetut HYL/PER-tilat säilyvät.
- Excelin vanhat RATA- ja KILP-välilehdet voidaan tuoda .xls- ja .xlsx-tiedostoista uuteen kilpailuun esikatselun kautta. Makroja ei suoriteta.
- Luokkien osallistumista lähtöaikojen arvontaan voidaan säätää, ja arvonnassa on esikatselu.
- Uudet testit suojaavat käsikelloja, hyväksyntää, uusintalukuja, Excel-tuontia ja laskentapolkuja.

## Ennen ensimmäistä kilpailukäyttöä

Windowsin automaattinen GitHub-käännös ei korvaa fyysisen Emit-lukijan testausta todellisessa Windows-ympäristössä. Varmista lukijan yhteys, kellot, käsiasemien syöttö, tulosten hyväksyminen ja tulosteet harjoituskilpailulla. Tee kilpailutiedoista erillinen varmuuskopio. AS2015A:n kaikkia laskentatapauksia ei ole varmennettu historiallisilla kilpailutuloksilla.
