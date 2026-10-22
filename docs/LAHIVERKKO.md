# Kilpailun yhteiskäyttö lähiverkossa

Saatavilla versiosta 0.6.7 alkaen. Käyttöliittymä ja yhteysvirheet ovat suomeksi.

## Käyttö

1. Pysäytä Emit-lukija yhteyden perustamisen ajaksi. Avaa kilpailu isäntäkoneessa. Paina yläpalkin keskellä **Lähiverkko** ja **Jaa tämä kilpailu**.
2. Muilla koneilla avaa **Lähiverkko**. Haku täydentää löytyneen isännän osoitteen.
   Anna isäntäkoneessa näkyvä 12 merkin parituskoodi ja valitse **Yhdistä kilpailuun**.
3. Jos haku ei löydä isäntää, kirjoita sen IP-osoite käsin. Isännän yhteysikkuna näyttää
   osoitteen. Tarvittaessa osoitteeseen voi liittää portin, esimerkiksi `192.168.1.20:42165`.
4. Kaikki muokkaavat samaa kilpailua samoilla oikeuksilla. Jokaisella on oma lukija ja
   paikallinen tulostinvalinta. Tulostaminen käyttää aina kyseisen koneen tulostinta. Yhteyden valmistuttua
   jokainen voi käynnistää oman Emit-lukijansa.

Vienti ja tulostus pyytävät isännältä tuoreen tilannekuvan. Niitä jatketaan, kun
lähetysjono on kuitattu ja isäntäkoneeseen saadaan yhteys.

Isännän palomuurin pitää sallia TCP 42165 lähiverkosta. Automaattinen haku käyttää UDP 42166:ta.
Vieraat WLAN-verkot voivat estää laitteiden välisen liikenteen. Sovellus ei muuta palomuuria.
Internet-yhteyttä, verkkolevyä tai yhteistä kansiota ei käytetä.

Yläpalkki näyttää ajan tasalla olevan yhteyden, jonottavat tapahtumat, katkon tai
huomautuksen. Yhteysikkunasta näkee tarkemman virheen. Isäntäkone pitää jättää käyntiin.
Kilpailun vaihtaminen edellyttää yhteyden sulkemista. Palvelinroolin siirtämistä toiselle
koneelle kesken katkon ei tehdä automaattisesti.

## Muokkausten toiminta

Isäntä järjestää muutokset ja validoi kilpailun ennen hyväksymistä. Myös isäntäkoneen
oma GUI käyttää samaa verkkorajapintaa. Tietoja ei kopioida asiakkaalta isännän tiedoston päälle.

Kilpailija, luokka, rata ja BC-osallistuja ovat erikseen vertailtavia tietueita.
Eri kilpailijoiden muutokset voidaan yhdistää. Saman tietueen samanaikainen muutos
hylätään mieluummin kuin korvataan huomaamatta. Radan asemat muodostavat yhden
muokkauskokonaisuuden. Kilpailun muut yhteiset asetukset vertaillaan tietoryhmittäin.
Hyväksyntää ja korjausta tehtäessä tarkistetaan myös laskennan syötteet, kuten korttiluku,
rata ja BC:n asetukset. Joissakin tilanteissa myös eri toimitsijoiden erilliset muutokset
voivat siksi edellyttää tietojen avaamista uudelleen.

Etäpäivitys päivittää luettelot ja kilpailun mallin. Se ei vaihda avoimen lomakkeen
tekstiä tai kohdistusta. Tallennuksessa tarkistetaan lomakkeen alkuperäinen muokkauspohja.
Ristiriidan oma syöte jää näkyviin. **Palauta tallennetut tiedot** hakee nykyiset arvot;
sen jälkeen muutoksen voi tehdä uudelleen. Verkossa hylätty tapahtuma säilyy myös
paikallisena `.hylatty-<tunnus>.json`-tiedostona perusteluineen.

## Emit ja yhteyskatkot

Lukutapahtuma kirjoitetaan paikalliseen lähetysjonoon ennen lähettämistä. Alkuperäistä
leimahistoriaa ei korvata. Isäntä lisää hyväksytyt lukemat peräkkäin, joten asemakorjausten
lähdelukemien indeksit säilyvät. Uusintaluvut ovat aitoja erillisiä lukutapahtumia;
saman verkkotapahtuman uudelleenlähetys ei lisää toista lukemaa.

Yhteyskatkon aikana Emit-lukuja voi jatkaa. Muut muokkaukset tarvitsevat isännän
kuittauksen. Epäselvästi toimitettu muutos säilyy jonossa ja näkyy odottavana; älä tee
samaa muutosta uudelleen kuittausta odottaessa. Yhteyden palautuessa jono toistetaan
samoin tapahtumatunnuksin. Myös sovelluksen sulkeminen säilyttää jonon. Ohjelman
uudelleenkäynnistyksen jälkeen liity samaan kilpailuun uudelleen samalla koodilla.

Saman kortin eri sisältöiset lukemat säilyvät kaikki historiassa. Aktiivinen lukema
määräytyy isännän vahvistusjärjestyksessä, myös katkon jälkeisessä toistossa. Eri sisältöinen
verkkolukema poistaa kilpailijan ja BC:n tuloksen hyväksynnän ja merkitsee ajat tarkistettaviksi.
Toimitsijan on tarkistettava oikea korttisisältö ja tulos ennen uudelleenhyväksyntää.
Isäntä vertaa hyväksyttävien asemarivien leimakoodeja ja aikoja myös nykyiseen
Emit-syötteeseen. Vanhaa pohjaa ei voi hyväksyä uudelleen vain kuittaamalla lomaketta.
Tietokoneiden seinäkelloja ei käytetä ristiriitojen automaattiseen ratkaisemiseen.

Emit-rivin korjaus käyttää samaa olemassa olevaa perusteltua korjaushistoriaa myös
verkossa. Alkuperäiset leimat säilyvät. Korjaus, pisteet ja hyväksynnän poistuminen
välittyvät kaikille koneille.

## Tallennus ja palautuminen

`kilpailu.toml` säilyy varsinaisena kilpailumuotona. LAN-tiedostot ovat sivutiedostoja:

- Isännällä `kilpailu.toml.lan-isanta.json` sisältää kilpailun tunnuksen, parituskoodin,
  vahvistetun tilan, versionumeron ja jo käsitellyt tapahtumatunnukset.
- Isännän `.lan-lukko` estää kahden isännän avaamisen samalle tiedostolle. Myös
  tämän version paikalliset muokkaajat ohjataan liittymään jaettuun kilpailuun.
- Lähetysjonon paikallinen lukko estää kahta ikkunaa kirjoittamasta samaan jonoon.
- Asiakkaan kopio ja `lahetysjono.json` ovat sovelluksen kilpailukansion
  `LAN-kopiot/<kilpailun tunnus>/`-kansiossa. Isännän omalla GUI:lla on myös paikallinen jono.
- Keskeytynyt isäntätallennus voidaan palauttaa ennen/jälkeen-tilojen avulla.
  Epäselvästi ulkoisesti muokattua keskeytynyttä tallennusta ei korvata automaattisesti.

Pidä LAN-sivutiedostot ja asiakkaiden jonot mukana varmuuskopiossa, etenkin katkon aikana.
Linuxissa lähetysjonot ja isännän LAN-palautumistiedosto kirjoitetaan vain omistajalle luettavina. Windows käyttää kansiosta
periytyviä käyttöoikeuksia. Parituskoodin haltija saa täydet kilpailun muokkausoikeudet.

Yhteys käyttää Rustin `snow`-kirjaston Noise NNpsk0 / Curve25519 / ChaChaPoly / SHA256
-toteutusta. Satunnainen 12 merkin koodi toimii yhteisenä avaimena. Liikenne todennetaan
ja salataan. Löytöviestissä välitetään kilpailun nimi ja osoite, ei parituskoodia.
Katso [kirjaston dokumentaatio](https://docs.rs/snow/0.10.0/snow/).

Asiakas tarkistaa muutokset taustalla noin 180 ms välein ja GUI noin 250 ms välein.
Muutoksista lähetetään tietuekohtaiset erot; uusi kokonaiskuva lähetetään vain revision
vaihtuessa. Tavallinen muokkaus odottaa kuittausta enintään 750 ms GUI-kutsussa.
Verkkoluku ja uudelleenyhdistäminen toimivat taustasäikeissä. Sanomakoko on rajattu
32 MiB:iin ja samanaikaisia palvelinyhteyksiä on enintään 16.

## Varmennus ja rajat

Automaattiset Rust-kokeet käyttävät oikeita TCP-yhteyksiä ja erillisiä paikallisia
kopioita. Ne kattavat erilliset ja ristiriitaiset muokkaukset, kaksi lukijaa,
tapahtuman uudelleenlähetyksen, asiakkaan/isännän uudelleenkäynnistyksen,
levyltä palautuvan jonon, keskeytyneen tallennuksen, NAC/BC-hyväksynnät ja Emit-korjauksen.

Kahden oikean GUI-prosessin laitteeton koe on ajettavissa vain `--demo`-tilassa:
`AS2026_LAN_GUI_ROLE=isanta` tai `asiakas`, yhteinen `AS2026_LAN_GUI_DIR`-testikansio.
`AS2026_LAN_GUI_RECOVERY=1` lisää isännän pysäytyksen, katkon Emit-luvun ja automaattisen
palautumisen. Kansio on vain testin ohjausta varten; tuotannon LAN-yhteys ei käytä sitä.
Koe ohjaa oikeita GUI-callbackeja ja tuottaa kuvakaappaukset sekä kilpailukopiot.
Tulostuskoe muodostaa tulosteen lähettämättä sitä fyysiselle tulostimelle.

Tämän Linux-isännän virtuaalikokeet eivät todista kahden fyysisen koneen WLAN-yhteyttä,
Windowsin palomuuria, fyysisten Emit-lukijoiden samanaikaista käyttöä tai paperitulostusta.
Isäntäkoneen automaattinen varaisäntä, WAN-yhteys ja koneiden IP-osoitteen vaihtuminen
kesken istunnon eivät kuulu tähän toteutukseen. Vaihtuneeseen osoitteeseen voi liittyä
käsin; kilpailun pysyvä tunnus yhdistää saman paikallisen jonon uudelleen.
