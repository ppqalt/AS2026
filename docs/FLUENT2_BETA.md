# AS2026 Fluent 2 -käyttöliittymäbeta

> **Kehityshaara:** `beta/fluent2-v071` · **lähtökohta:** julkaistu `v0.7.1`, commit `d071eb9d9d661162bb5e580b41cf46f472d72c3b`.
>
> **Tila:** kokeellinen ja keskeneräinen. Tämä tiedosto on tavoitemäärittely ja tarkastuslista, ei väite siitä, että kaikki kohdat olisi jo toteutettu. Älä julkaise vakaana versiona tai yhdistä `main`-haaraan ilman arviointia.

## Komponenttistandardin ensisijainen lähde

**[FLUENT2_KOMPONENTTISTANDARDI.md](FLUENT2_KOMPONENTTISTANDARDI.md)** on tämän beta-haaran kaikkia `ui/**/*.slint`-muutoksia ohjaava mittojen, varianttien, saavutettavuuden, tilojen, suunnittelulähteiden ja tiedostokohtaisten komponenttien normatiivinen preset-katalogi. Älä tee uutta rinnakkaista mittataulukkoa tähän tiedostoon. Osa katalogin nimistä on tulevia toteutuksia, ei vielä importoitavia Slint-komponentteja.

## Slint 1.18.1 -päivityksen tila

Runtime, kääntäjä ja paikalliset widget-lähteet käyttävät **Slint 1.18.1**:tä. Vanha 1.16.0-pohja on verrattu tiedosto kerrallaan ja tarpeelliset projektisovitukset siirretty uudempaan lähteeseen. Alkuperäislähteiden hashit on vahvistettu virallisen Git-tagin puusta. Muutokset ja säilytettävät sovitukset: [widgettien ylläpito-ohje](../ui/fluent/README.md).

Tietoja-ikkunassa on tallentuva järjestelmä/vaalea/tumma/Dark Spruce -ulkoasu. Perusohjainten tuonnit on siirretty yhteisiin presetteihin. Galleria on oma ikkuna ja ohittaa tavallisen sovelluksen asetukset, kilpailut, tulostimen, lukijan ja päivityspalvelut. WinUI-pikselivertailu, täydellinen suuren kontrastin tuki ja Windowsin Mica/Acrylic eivät ole valmiita.

## Tavoite

AS2026:n tulee näyttää ja käyttäytyä mahdollisimman uskollisesti **Windows 11:n WinUI 3 / Fluent 2** -sovellukselta. Pelkkä vihreän väripaletin tai kulmasäteen vaihtaminen ei täytä tavoitetta. Kokonaisuus kattaa myös komponenttien mitat, tekstihierarkian, navigaation, ohjauselementtien tilat, painikkeiden semantiikan, valintaikkunat, vierityksen, näppäimistö- ja hiirikäytön, saavutettavuuden ja ikkunan elinkaaren.

Windows 11:n design-ohjeet ovat visuaalinen vertailukohta. **Slint ei ole WinUI 3**, joten tarvittaessa tehdään sovelluskohtaiset vastineet ja dokumentoidaan erot. Säilytä sovelluksen Rust + Slint -arkkitehtuuri, suomenkielinen käyttöliittymä ja Windows/Linux-tuki. Älä korvaa sovellusta web- tai Electron-sovelluksella.

## Ensisijaiset lähteet

- [Windowsin typografia: Segoe UI Variable ja tekstihierarkia](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography)
- [Windows 11:n geometria: 4 px -säätimet, 8 px -valintaikkunat ja ponnahdusikkunat](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/geometry)
- [Windowsin materiaalit: Mica, Acrylic ja Smoke](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/materials)
- [Fluent 2 -design-tokenit: globaalit ja semanttiset tokenit](https://fluent2.microsoft.design/design-tokens/)
- [Windowsin suunnitteluresurssit ja WinUI 3 Gallery](https://learn.microsoft.com/en-us/windows/apps/design/downloads/)

Älä päättele visuaalisia arvoja satunnaisista kuvakaappauksista, jos virallinen design-resurssi on saatavilla. Materiaaliefektejä ei tarvitse lisätä kaikkialle: käytä niitä Windowsin suosittelemiin käyttötarkoituksiin ja huomioi suorituskyky, kontrasti sekä Linuxin peittävä vaihtoehto.

## v0.7.1:n tarkastuksessa havaittu

- `ui/theme.slint`: aikaisempi `Google Sans`, tumma Dark Spruce -paletti sekä 5/6 px -kulmasäteet. Betan ensimmäisessä pienessä muutoksessa Windowsin ensisijaiseksi fontiksi valittiin `Segoe UI Variable` sekä peruskulmasäteiksi 4/8 px. **Paletti ja suurin osa mitoista odottavat tarkastusta.**
- `ui/fluent/style`: Slint 1.16.0:n paikallinen Fluent-sovitus. `ui/fluent/README.md` kertoo, että animaatiot, varjot, läpikuultavuus, järjestelmäkorostus ja reunojen liukuvärit on tarkoituksella poistettu. Käy jokainen muutos läpi verrattuna viralliseen Windows-viitteeseen; älä palauta tehosteita mekaanisesti.
- `ui/components.slint`: sovelluskohtaisia komponentteja (mm. `NavItem`, `Workspace`, `DataTable`, `Editor`, `DialogSurface`) on arvioitava erikseen vakiosäätimistä.
- `ui/app.slint`: seitsemän pääsivun runko, sivupalkki, tilarivi, Toiminnot-valikko ja useita valintaikkunoita. Myös pienet näytöt ja skaalaukset kuuluvat arviointiin.
- Sivupalkin **Päivitykset ↻**- ja **Tietoja**-painikkeet ovat käyttäjän nimeämä nykyinen visuaalinen vertailukohta, **eivät hyväksytty lopullinen komponenttimalli**.
- `docs/KAYTTOLIITTYMAOPAS.md` ja `docs/README.md` kuvaavat edelleen 0.7.0:aa. Nykyinen käyttöliittymäopas sekoittaa käyttäjälle tarkoitetut toimintaohjeet, komponenttien lähdekoodikartan ja visuaaliset parametrit. Jaa nämä selkeisiin, eri tarkoituksia palveleviin ohjeisiin.

## Animaatiot, sumennus ja muut visuaaliset tehosteet — käyttäjän päätös

**Käyttäjän vaatimus (25.10.2026): kaikki valinnaiset, suorituskykyyn vaikuttavat animaatiot ja visuaaliset tehosteet ovat OLETUKSENA POIS käytöstä.** Niille on yksi yhteinen, pysyvä valinta nimenomaan **Tietoja AS2026:sta** -ikkunassa: **Ota animaatiot ja tehosteet käyttöön**. Älä tee monia erillisiä asetuksia tai käänteistä `Poista tehosteet käytöstä` -valintaa.

- Valinta tallennetaan käyttäjän sovellusasetuksiin eikä kilpailukohtaiseen aineistoon. Puuttuva/vanha asetus tarkoittaa `false`; tallennusvirhe palauttaa edellisen arvon ja näyttää virheen.
- Yksi keskitetty `Theme.effects-enabled`-portti ohjaa Slintin valinnaisia hover-/painallus-/navigaatioanimaatioita, popup- ja sivusiirtymiä, läpikuultavuutta, varjoja, blur-efektejä, mahdollisia Mica/Acrylic-taustoja ja muita tulevia valinnaisia GPU/CPU-kustannuksia. Jokaisen uuden tehosteen on kunnioitettava porttia.
- Pois päältä: animaatiot eivät käynnisty, ajastimia ei pyöritetä niille tarpeettomasti, valinnaista blurria tai taustamateriaaleja ei alusteta/ylläpidetä. Ei näytön jatkuvaa ylimääräistä piirtämistä. Peruskäytettävyys, kontrasti, fokusmerkinnät ja virhetilat pysyvät näkyvinä ilman animaatioita.
- Päällä: vain todella toteutetut, testatut ja alustalla tuetut Fluent/Windows 11 -tehosteet aktivoidaan; niidenkin tulee noudattaa tarkoituksenmukaisia rajoja.
- **Ensimmäinen beta-vaihe:** lisätty Tietoja-ikkunan valinta ja käyttäjäasetuksen tallennus sekä keskitetty tehosteportti. Vain ensimmäiset Fluent-painike- ja sivupalkkianimaatiot on sidottu porttiin. **Windowsin oikeaa Mica/Acrylic-sumennusta eikä kaikkia tehosteita ole vielä toteutettu.** Älä kuvaa niitä valmiiksi käyttäjäoppaissa tai PR:ssä.

Testaa erityisesti: uusi asennus (pois), vanhojen asetusten migraatio (pois), päälle/pois ilman uudelleenkäynnistystä, asetuksen pysyvyys uudelleenkäynnistyksessä, epäonnistunut tallennus, Linux ilman Windows-efektejä, Windowsin laitekiihdytys ja ohjelmistorenderöinti sekä ettei mikään uusi animaatio toimi valinnan ollessa pois.

## Toteutusvaihe: preset-kirjasto ja galleria

`ui/presets.slint` sisältää betan ensimmäiset oikeat preset-kontrollit: `UiButton` (kolme kokoa, neljä varianttia), `UiTextField`, `UiComboBox`, `UiCheckBox`, `UiScrollView`, `UiCard`, `UiSeparator` sekä `UiSlider` sekä valinta-, numero-, monirivisyöte-, välilehti-, luettelo-, etenemis- ja viestikomponentit. `ui/fluent-gallery.slint` esittää ne interaktiivisesti kehitystilassa. Suora käynnistys ja QA-vaatimukset: **[FLUENT2_GALLERIA.md](FLUENT2_GALLERIA.md)**. Nämä ovat ensimmäisen vaiheen toteutuksia: täydellinen teema, WinUI-pikselivertailu ja sivujen migraatio ovat yhä keskeneräisiä.

## Koko käyttöliittymän tarkastus

Käy jokainen kohta läpi sekä vaaleassa että tummassa tilassa ja kaikissa olennaisissa ohjain-/virhetiloissa.

| Osa-alue | Tarkastettava |
| --- | --- |
| Sovellusikkuna | Otsikkorivi, ikkunaikonit, aktiivisuus, reunat, minimointi, maksimointi, koon muuttuminen |
| Navigointi | Sivupalkki, valintapalkki, yhteystila, ikonit, alareunan päivitys- ja tietopainikkeet, näppäimistöfokus |
| Yleiset säätimet | Tavalliset, ensisijaiset ja vaaralliset painikkeet; hover/pressed/focus/disabled; tekstikentät, aikakentät, valintaruudut, yhdistelmäruudut |
| Taulukot | Rivivalinta, sarakeotsikot, katkaistut tekstit, vieritys, tyhjä taulukko, näppäimistökäyttö |
| Lomakkeet | Otsikot, selitteet, virheviestit, kenttäväli, pitkät arvot, automaattitallennus, vieritys |
| Ponnahdusikkunat | Toiminnot, kontekstivalikot, työkaluvihjeet, fokus, ESC, popupin sulkemisen elinkaari |
| Modaalit | Otsikko, sisältö, sisäinen vieritys, peruuta/vahvista, fokus, sulkeminen, pienet näytöt |
| Tilat | Odotus, lukitus, keskeneräisyys, virhe, onnistuminen, offline, yhdistetty, käytöstä poistettu |
| Ulkoasu | Typografia, semanttiset pintavärit, Mica/Acrylic-käyttömahdollisuudet, kontrasti, valinnat, ikonografia, reunat, välistys |
| Erityistapaukset | Pitkät kilpailunimet ja kenttäarvot, Finnin ääkköset, 100 % / 150 % skaalaus, 900 × 600 sekä suuret näytöt |

**Sivukohtainen kattavuus:** Yleiskatsaus, Kilpailijat, Emit, Tulokset, Luokat, Radat ja asemat, Asetukset; lisäksi kaikki lataus-, julkaisu-, korjaus-, tulostus-, LAN- ja tietovalintaikkunat.

## Toteutusperiaatteet

1. **Semanttiset tokenit:** peruspaletti, pintatasot, tekstit, viivat, fokus, korostus, palautetilat, mitat ja typografia määritellään keskitetysti. Älä kovakoodaa yksittäisiin näkymiin uusia omia värejä.
2. **Yhteiset komponentit:** sama painike tai syötekenttä tarkoittaa samaa vuorovaikutusta kaikkialla. Käy paikallisen `ui/fluent`-sovituksen kaikki muokatut säätimet läpi.
3. **Teemat:** suunnittele vaalea, tumma ja järjestelmän mukaan valittava tila sekä myöhemmin käyttäjän korostusväri. Älä kutsu niitä valmiiksi ennen kuin myös kaikki omat komponentit noudattavat tilaa. Tarkista myös suuren kontrastin tarpeet.
4. **Vierityksen oikeellisuus:** sulkeutuvat ja laajenevat osiot eivät saa jättää vierityskohtaa sallitun alueen ulkopuolelle. Komponenttien toimintaa ei muuteta ilman testejä.
5. **Alustariippuvuus:** Windows-efektit ja fontit toteutetaan vain tuetulla alustalla, Linuxille säilytetään luettava, selkeä vaihtoehto. Älä jaa Microsoftin fonttitiedostoja osana pakettia.
6. **Turvalliset rajat:** älä muuta tuloslaskennan sääntöjä, tallennusmallia, Emitin tulkintaa, LAN-synkronointia, tiedostovientiä tai toimintosopimuksia käyttöliittymäbetan mukana.
7. **Oikea verifiointi:** suorita käännös, testit, tarkista suomenkieliset tekstit, työnkulut, fokus ja kuvakaappaukset; vertaa oikeassa Windows-ympäristössä eikä vain Linuxin renderöintiin.

## Dokumentaation uudistaminen

- **`docs/KAYTTOLIITTYMAOPAS.md`:** kirjoita uudelleen käyttäjän tehtävien mukaiseksi, ei Slint-komponenttiluetteloksi. Esittele jokaisen sivun yleiset toimet ja etenevät käyttötapaukset. Käytä betan uusista ruuduista otettuja ajantasaisia kuvia. Älä dokumentoi keskeneräisiä ominaisuuksia toteutettuina.
- **`docs/UI_PERIAATTEET.md`:** kirjoita uudelleen kehittäjän viiteohjeeksi. Taulukoi Fluent-komponentit, tokenit, käyttäytymistilat, saavutettavuus, alustojen erot sekä sovelluskohtaiset poikkeukset, perusteluineen.
- **`docs/README.md`:** korjaa versiot ja ohjaa käyttäjät sekä kehittäjät oikeisiin ohjeisiin.
- **`ui/fluent/README.md`:** ylläpidä täsmällistä muutosluetteloa verrattuna Slintin jakamaan Fluent-tyyliin, sekä lisenssi- ja päivitysvelvoitteet.
- Käy läpi myös `docs/KILPAILUTOIMINNOT.md`, `docs/KILPAILUPAIVA.md`, `docs/LAHIVERKKO.md`, `docs/EMIT_RIVIN_KORJAUS.md`, `docs/TULOSTUS.md`, `docs/LIVE_JULKAISU.md` sekä muut UI-vaiheita kuvaavat ohjeet, ja päivitä muuttuneet käyttövaiheet vasta kun toteutus on todistettavasti toimiva.

## Hyväksymisehdot ennen mahdollista yhdistämistä

- [ ] Jokainen pääsivu ja jokainen valintaikkuna käy läpi komponentti- ja vuorovaikutustarkastuksen.
- [ ] Ei ristiriitaisia painike-, kenttä-, fokus-, popup- tai scroll-tyylejä.
- [ ] Oikeat Windows 11 -vertailukuvat, 100 % ja 150 % skaalaus; myös 900 × 600 -koon käyttökelpoisuus.
- [ ] Windowsin ja Linuxin GUI- sekä Rust-regressiotestit läpäisevät.
- [ ] Kaikki käyttöliittymäoppaat vastaavat toteutettua betaa ja ovat sisällöltään luotettavia.
- [ ] Kumpikin kehittäjä voi arvioida erillistä betaa ilman, että vakaata `main`-haaraa muutetaan.
