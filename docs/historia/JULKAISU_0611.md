# Version 0.6.11 julkaisutiedot

Tämä kuvaa aiempaa julkaisua; ohjeiden linkit on suhteutettu arkistokansioon. [Nykyiset julkaisutiedot](../../RELEASE_NOTES.md).

# AS2026 0.6.11

- Beginners Cupin puuttuvat RT:t ja muut virhepisteet kysytään erillisessä syöttövaiheessa NAC-tuloksen tallentamisen jälkeen. NAC- ja BC-pisteet säilyvät itsenäisinä.
- Livetuloksiin valitaan julkaistavat luokat. Jokaisella luokalla on oma sivu: pääsivu `tulokset.html`, sen jälkeen `tulokset1.html` jne. Sivut vaihtuvat 60 sekunnin välein ja viimeinen palaa pääsivulle. Perusnimi on muokattavissa.
- Livetulokset esitetään pystysuunnassa tiiviimmin. Julkisilla sivuilla ei näytetä virheilmoituksia, nousupisteitä eikä sarjapisteitä.
- Paikallinen live tallentaa ja päivittää samat sivut omalle koneelle ilman palvelinyhteyttä. FTP/SFTP-julkaisu käyttää edelleen 0.6.10:n sisäistä tiedostosiirtoa, salasanan tai avaintiedoston tunnistautumista ja varmennettua SFTP-sormenjälkeä.
- Tulosnäkymä näyttää automaattisen KESK-tilan heti puuttuvan JAT:n jälkeen. Kilpailijavalinta ja luokkarajaus kohdistetaan uuden kortin kilpailijaan.
- Asemataulukossa ovat Käytetty, klo, etumerkillinen AT, ATp/ATy ja jakson Myöhä. Kokonaismyöhä näkyy yhteenvedossa. Version 0.6.10 tulostuskorjaukset säilyvät.
- Sivupalkin yhteysilmaisimen tasaus, hakukentän ja luokkavalinnan väli sekä asetuspaneelien samankorkuinen asettelu on korjattu.
- Windowsin tilapäisiä tiedostolukituksia käsitellään rajatuilla uudelleenyrityksillä atomisessa tallennuksessa. `--portable` pitää sovellusasetukset ohjelmakansiossa.

Pohjana on julkaistu 0.6.10. Sen valikko-, Emit-, LAN-, tulostus- ja SFTP-korjaukset säilyvät mukana.

[Nykyinen käyttöliittymäopas](../KAYTTOLIITTYMAOPAS.md) · [Livejulkaisun ohje](../LIVE_JULKAISU.md) · [Beginners Cup](../LUOKAT_JA_BEGINNERS_CUP.md)
