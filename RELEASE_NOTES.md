# AS2026 0.6.7

Tämä on 0.6.6-julkaisuehdokkaan korjattu seuraaja. 0.6.6:n tarkistukset epäonnistuivat,
eikä sen julkaisutagia tai paketteja korvata. Alla ovat myös edellisen ehdokkaan uudet ominaisuudet.

- Korjattu Windowsin LAN-yhteyden muodostus: kättely ei enää peri kuuntelijan ei-estävää yhteystilaa. Salattu kättely, viestien aikarajat ja virheellisen parituskoodin torjunta säilyvät.
- Korjattu nopea Tab/Space-käyttö luokkavalinnoissa ja Tab-siirtymät tuloksen syöttöikkunassa. Vanhentunut ajastin ei saa siirtää kohdistusta takaisin aiempaan kenttään.
- Lisätty alustariippumaton ei-estävän yhteyden regressiotesti sekä Windows-GUI:n työnkulkukokeet 100 % ja 150 % skaalauksella. Aiemmat testit ja niiden tarkistukset säilyvät.
- Kilpailua voi käyttää yhtä aikaa usealta koneelta lähiverkossa. Yksi kone jakaa kilpailun, muut liittyvät yläpalkin **Lähiverkko**-painikkeesta parituskoodilla. Automaattisen haun rinnalla on IP-osoiteyhteys.
- Kaikilla koneilla on samat kilpailun muokkausoikeudet ja oma lukija sekä tulostin. Muutokset päivittyvät taustalla avaamatta toisen toimitsijan lomaketta uudelleen. Ristiriitainen muutos ei korvaa uudempia tietoja huomaamatta.
- Katkon aikana Emit-luvut tallentuvat paikalliseen jonoon ja välittyvät turvallisesti yhteyden palattua. Alkuperäiset luvut ja korjaushistoria säilyvät. Muut muokkaukset, vienti ja tulostus edellyttävät toimivaa yhteyttä isäntään.
- Lisätty yksittäisten Emit-asemarivien manuaalinen korjaus ja alkuperäisen lukuaineiston jäljitettävyys.
- Korjattu tulosvientien tallennustoimintoja ja tiedostopäätteiden käsittelyä.
- Selkeytetty kilpailun huomautuksia ja niiden yksityiskohtaista tarkastelua.
- Parannettu käsikellojen syöttöä, Tab-navigointia ja vieritystä.
- Parannettu lähtöarvonnan, luokkatulostuksen, yhdistettyjen luokkien ja Beginners Cupin käyttöliittymiä.
- Siirretty Windows-tulostuksen toteutusta PowerShellistä natiiviin Rust-koodiin.
- Laajennettu tulostuksen ja kilpailutyönkulkujen testejä.

## Käyttöohjeet

[Lähiverkon perustaminen, ristiriidat ja palautuminen](https://github.com/ppqalt/AS2026/blob/v0.6.7/docs/LAHIVERKKO.md) · [Kilpailutoiminnot](https://github.com/ppqalt/AS2026/blob/v0.6.7/KILPAILUTOIMINNOT.md) · [Tulostus](https://github.com/ppqalt/AS2026/blob/v0.6.7/docs/TULOSTUS.md)

Isännän palomuurissa tarvitaan TCP 42165 ja automaattisessa haussa UDP 42166. Internetiä tai jaettua kansiota ei tarvita. Isäntä on pidettävä käynnissä; automaattista varaisäntää ei ole. Säilytä katkon aikana myös LAN-sivutiedostot ja asiakkaiden lähetysjonot.

## Varmennus

Linuxissa läpäistiin 389 Rust-testiä ja 16 GUI-sovitintestiä. Kahdella oikealla sovellusprosessilla virtuaalinäytössä kokeiltiin samanaikaisia Emit-lukuja, käsiaikoja, virhepisteitä, hyväksyntää, tulosteiden muodostusta ja katkon jälkeistä jonon purkamista. Tulokset vastasivat toisiaan. Erilliset kokeet kattoivat ristiriidat, uusintalähetykset, tallennuksen palautumisen ja NAC/BC-korjaukset. Käyttöliittymää tarkastettiin myös 150 % skaalauksella ja pienessä ikkunassa.

Nämä kokeet eivät korvaa kahden fyysisen koneen WLAN-, Windows-palomuuri-, Emit-lukija- tai paperitulostuskoetta. GitHubin julkaisutarkistukset varmentavat erikseen Linux- ja Windows-käännökset ennen pakettien julkaisemista.
