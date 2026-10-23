# Kilpailupäivän tarkistuslista — AS2026 0.7.0

Tee valmistelut ennen kilpailijoiden saapumista. Kokeile laitteet erillisellä
esimerkkikilpailulla; älä tee koelukuja tai koemuokkauksia oikeaan kilpailuun.

## Ennen ensimmäistä lähtöä

1. Asenna sama julkaistu versio kaikille toimitsijakoneille. Tarkista versio
   **Tietoja**-ikkunasta. Avaa oikea kilpailu ja varmista sen nimi ja tallennuspaikka.
2. Ota **koko kilpailukansiosta** erillinen varmuuskopio. LAN-käytössä mukaan kuuluvat
   isännän sivutiedostot ja asiakkaiden paikalliset lähetysjonot. Edellinen tallennus
   säilyy myös `kilpailu.toml.bak`-tiedostossa; se ei korvaa erillistä varmuuskopiota.
   Älä jaa kilpailutiedostoa
   verkkolevyn tai pilvisynkronoinnin kautta.
3. Tarkista Excel-tuonnin huomautukset, kilpailun nimi, luokkien radat ja kilpailijat.
   Puuttuva Emit tai lähtöaika on täydennettävä; tuonti ei arvaa niitä.
4. Tarkista asematyypit, Emit-koodit, ihanneajat, jaksot ja käsin kellotettavat asemat.
   Varmista kilpailun piste- ja myöhästymisasetukset sekä oletus Emit-korjaus.
5. Tarkista luokkien ensimmäiset kilpailunumerot, lähtöajat, lähtövälit ja arvontavalinnat.
   Tulosta tai vie lähtöluettelo halutuilla sarakkeilla ja tarkista se ennen jakamista.
6. Kokeile käytettävää **fyysistä Emit-lukijaa ja korttia** esimerkkikilpailussa.
   Tarkista kortin numero ja leimat; valitse oikea sarjaportti ja käyttöoikeudet.
7. Valitse tulostin Asetuksissa ja tulosta koetulos paperille. Tarkista jonon toiminta,
   A4-suunta, nimet, ajat ja pistetaulukko. Ohjelman lähetyskuittaus ei takaa paperitulostusta.

## Lähiverkko

Valitse yksi isäntä ja pidä se käynnissä koko yhteiskäytön ajan. Yhdistä asiakkaat
sivupalkin yhteyspainikkeesta, parituskoodilla ja tarvittaessa IP-osoitteella.
Tarkista molempien koneiden tilasta, että alkusynkronointi on valmis. Tee erillisessä
koekilpailussa yksi muutos kummaltakin koneelta ja tarkista sen näkyminen toisella.

Isäntä tarvitsee luotetussa verkossa TCP-portin **42165** ja automaattihaku
UDP-portin **42166**. Vieras-WLAN tai asiakaseristys voi estää yhteyden.
[Palomuuriohje](PALOMUURI.md) · [LAN-käyttö ja palautuminen](LAHIVERKKO.md).

Katkon aikana Emit-luvut säilyvät paikallisessa jonossa. Muut muokkaukset tarvitsevat
isännän kuittauksen. Odota jonon tyhjeneminen ennen koneen käytöstä poistamista;
älä tee samaa epäselvästi toimitettua muutosta uudelleen. Yhteys palautetaan samaan
kilpailuun. Isäntäkoneen automaattista vaihtoa ei ole.

## Kortinluvusta hyväksyttyyn tulokseen

- Tarkista kilpailijan tiedot, kortti ja lähtöaika. Täydennä pyydetyt käsikellot
  sekä RT- ja muut virheet. Aloitettu lomake säilyy uuden kortin saapuessa.
- Tarkasta asemarivit ja automaattinen tila. Puuttuva JAT tai myöhästymisrajan
  ylitys voi aiheuttaa KESK-tilan; puuttuva käsikello jätetään tyhjäksi.
- Korjaa tarvittaessa aktiivisen Emit-luvun leimaukset Emit-sivulla ja anna perustelu.
  Alkuperäinen lukema säilyy. Korjauksen jälkeen tarkista laskenta ja hyväksy tulos uudelleen.
- BC-osallistujan NAC-tallennusta seuraa BC:n oma syöttö. Syötä sen RT:t ja muut
  virhepisteet erikseen; NAC:n pisteet eivät korvaa BC:n arvoja.
- **Tutki** laskee asemarivit uudelleen. **Tarkista** muodostaa ne aktiivisesta
  kortista uudelleen. Lue vahvistus ennen käsin korjattujen tietojen korvaamista.

Tarkat työvaiheet: [tuloslaskenta](../KILPAILUTOIMINNOT.md),
[Emit-korjaus](EMIT_RIVIN_KORJAUS.md), [Beginners Cup](LUOKAT_JA_BEGINNERS_CUP.md).

## Live ja kilpailun päättäminen

Valitse **yksi julkaiseva kone**. Tarkista julkaistavat luokat, tiedostonimi,
palvelin ja julkinen osoite. SFTP:n palvelinavain vahvistetaan ylläpitäjältä saadulla
sormenjäljellä; salasanaa ei tallenneta kilpailuun. Paikallinen live toimii ilman
palvelinta. Tarkista sivu myös selaimella, älä vain ohjelman onnistumisviestiä.
[Livejulkaisun ohje](LIVE_JULKAISU.md).

Ennen lopullista julkaisua tarkista keskeneräiset tulokset, luokkasijoitukset,
yhdistetyt luokat ja BC erikseen. Tulosta lopulliset luettelot ja vie HTML-, CSV-
tai XLSX-kopiot. Vertaa tiedostojen sisältöä sovelluksen näkyviin tuloksiin.
**Julkaise lopullisina** pysäyttää palvelinjulkaisijan vasta onnistuneen siirron jälkeen.

Varmista lopuksi LAN-jonojen tyhjeneminen. Sulje kilpailu hallitusti ja ota koko
kilpailukansiosta uusi varmuuskopio. Säilytä alkuperäinen Excel, lopulliset viennit,
Emit-historia ja korjausten jäljitettävyys.

## Tallennusvirhe kilpailun aikana

Älä sulje ohjelmaa tai katkaise virtaa, jos se ilmoittaa tallentamattomasta
Emit-luvusta. Tarkista levytila, kilpailukansion käyttöoikeus ja tallennuspaikan
saavutettavuus. Tavallinen sulkeminen estetään, kun lukema on tallentamatta,
mutta epäonnistuneen levytallennuksen muistissa olevaa varalukemaa ei voi
palauttaa prosessin pakotetun lopetuksen tai sähkökatkon jälkeen. Korjaa
tallennusvirheen syy ja varmista tallentuminen ennen työn jatkamista.
