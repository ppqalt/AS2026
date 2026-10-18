# AS2026 0.6.2

## Korjaukset ja parannukset

- Lähtöarvonnan luokat voidaan valita erillisessä **Arvonnan luokat…** -ikkunassa. Peruuttaminen säilyttää valinnat, ja arvonta näyttää erillisen hyväksyttävän esikatselun.
- Jo luettujen korttien ja tallennettujen tulosten kilpailijat ohitetaan arvonnassa. Muiden valittujen luokkien lähtöajat voidaan silti arpoa; pois jätettyjen luokkien ajat eivät muutu.
- Tulokset-sivun pitkä kilpailijaeditori ja yhteenveto vierivät yhdessä myös 150 % skaalauksella. Painikkeiden ja kenttien sijoittelua on korjattu.
- Hyväksymättömät laskentatulokset pysyvät toimitsijan esikatselussa eivätkä saa virallista sijoitusta tai päädy virallisiin HTML-, CSV- ja XLSX-vienteihin hyväksyttyinä.
- Toimitsija voi vahvistaa perustellun keskeytyksen (KESK) myös ilman maalileimausta tai luettavaa Emit-korttia. Tästä ei muodosteta keksittyjä aikoja eikä pisteitä.
- HYL/PER-tilat, vanhat tuodut tulokset, käsikellot ja turvallinen uudelleenavaaminen säilyvät.
- Excel-tuonti on Rust-toteutus; vanha tarpeeton Python-testigeneraattori poistettiin lähdekoodista.

## Varmennuksen rajat

Rust-testit ja Linux/Windows CI tarkastavat muutoksia automaattisesti ennen julkaisemista. Synteettiset kilpailukokeet eivät korvaa oikean Windows-koneen ja fyysisen Emit-lukijan käyttökoetta tai vertailua historiallisiin kilpailutuloksiin. Suosittelemme harjoituskilpailua ennen virallista käyttöönottoa.
