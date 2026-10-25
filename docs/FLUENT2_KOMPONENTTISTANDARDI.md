# AS2026 — Fluent 2 / Windows 11 -komponenttistandardi ja esiasetuskatalogi

> **NORMATIIVINEN BETA-OHJE KEHITTÄJILLE JA KOODAAVILLE KIELIMALLEILLE.**
> Lähtöversio: AS2026 `v0.7.1` · haara `beta/fluent2-v071`.
> **ÄLÄ MUOKKAA `main`-HAARAA, ÄLÄ YHDISTÄ PR:ÄÄ, ÄLÄ JULKAISE UUTTA VAKAATA VERSIOTA ILMAN PROJEKTIN OMISTAJAN NIMENOMAISTA LUPAA.**
> Päivitetty 25.10.2026. Tämä on **suunnittelun ja toteutuksen sopimus**, EI väite siitä, että kaikki luetellut komponentit olisivat jo käytössä tai testattuina.
>
> **Tavoite:** vastaavuus Windows 11:n WinUI 3 -sovelluksen rakenteeseen, ohjaimiin, tiloihin ja käyttötapaan. Fluent 2:n yleiset periaatteet täydentävät Windows-spesifistä ohjausta. Toteutus: **Rust + Slint**, suomeksi, Windows + Linux, ei Electronia/web-UI:ta.

## 0. Lue tämä ensin (myös tekoälyagentit)

1. Lue **tämä kokonainen tiedosto** ennen minkään `ui/**/*.slint`-tiedoston muokkaamista. Lue myös `docs/FLUENT2_BETA.md`, `ui/theme.slint`, `ui/components.slint` ja `ui/fluent/README.md` sekä asianomaisen näkymän lähdekoodi.
2. **Käytä ensisijaisesti jo olemassa olevaa, toimivaa yhteistä komponenttia.** Jos preset puuttuu, lisää se yhteen yhteiseen komponenttikirjastoon ja tokenit teemaan; älä tee uutta ad hoc -`Rectangle + TouchArea` -painiketta joka sivulle. Älä kopioi WinUI/XAML/React-koodia suoraan Slintiksi.
3. Jokaisella komponentilla on oltava **semanttinen rooli, variantti, koko ja interaktiotilat**. Esiasetusten nimien tulee olla koodissa yksiselitteiset; tämän dokumentin preset-nimet ovat sopimus tulevaa toteutusta varten, eivät vielä automaattisesti olemassa olevia Slint-tyyppejä.
4. Virallisen Windows-arvon korvaaminen oletusarvauksella on kielletty. Alla `[MS]` tarkoittaa **Microsoftin dokumentoimaa Windows-vaatimusta/suositusta**, `[AS]` **AS2026:n ehdotettua valintaa**, `[NYK]` **v0.7.1:ssä havaittua toteutusta** ja `[TOTEUTTAMATTA]` vielä rakentamatonta komponenttia tai tilaa. `[AS]`-arvot eivät ole Microsoftin virallisia mittoja.
5. **Kaikki valinnaiset animaatiot ja tehosteet oletuksena POIS.** Vain `Tietoja → Ota animaatiot ja tehosteet käyttöön` saa muuttaa niiden tilan. Käytä `Theme.effects-enabled`-porttia; yksikään animaatio, blur, varjo tai grafiikka-ajastin ei saa ohittaa sitä. Sisältö, valinta, kohdistus ja virheilmoitukset pysyvät toimivina ilman tehosteita.
6. UI-muutokset eivät saa muuttaa kilpailusääntöjä, tulosten laskentaa, hyväksyntöjä, Emit-protokollaa, synkronointia, tietomalleja, luetteloiden tallennusta, tulostetta tai tiedostovientiä. Julkisiin kuviin ja testiin vain synteettistä kilpailudataa.
7. **Älä oleta että komponentti on toteutettu siksi, että sille on preset tässä tiedostossa.** Tarkista `ui/fluent/style/std-widgets.slint`-viennit. Perusvalikoimaan on lisätty `Slider`, `RadioGroup`, `Switch`, `SpinBox`, `TextEdit`, `TabWidget` ja `ProgressIndicator`; erikoisvalitsimia ei ole toteutettu.

### Lähteiden tärkeysjärjestys

1. **Windows 11 / WinUI 3 design:** Windowsin sovellusohjeet ja käyttöjärjestelmän aidot kontrollimallit; näistä otetaan natiivin Windows-sovelluksen ensisijainen visuaalinen ja käyttäytymisreferenssi.
2. **Fluent 2 Design System:** komponentin tarkoitus, semanttiset tokenit, sisältö, saavutettavuus ja tilat. Web/React-ohje ei ole automaattisesti WinUI 3:n pikselimitta.
3. **WinUI 3 Gallery + Windows UI Kit:** mittojen, kuvakkeiden, täytteiden ja tilojen visuaalinen vertailu aidossa Windows 11 -ympäristössä; dokumentoi käyttämäsi lähteen tarkka versio/ruutukaappaus.
4. **Slint 1.18.1 + AS2026:n paikallinen Fluent-sovitus:** lopullisen toteutuksen tekninen totuus. Tarkista API, paikallinen komponenttien export ja lisenssirajat.

**Jos lähteet ovat ristiriidassa:** Windows 11 -natiivi käyttäytyminen voittaa Fluent Webin yksityiskohdan; nykyisen kilpailutoiminnon turvallisuus voittaa kosmeettisen yhdenmukaistamisen. Epävarma mitta merkitään `[AS]`, ei `[MS]`.

### Päivityshuomautus: Slint 1.18.1 (beta)

Beta käyttää **Slint 1.18.1**:tä sekä runtimessa, kääntäjässä että paikallisten widgettien lähdepohjana. Alkuperäiset tiedostot on verrattu viralliseen tagiin `372cf0ee5577c3dfec309a45e7b778ba4e81b734`; hashit ovat `ui/fluent/UPSTREAM.sha256`:ssä ja sovitukset `ui/fluent/README.md`:ssä. `LineEditInterface`, valikot ja `content-*`-vieritysrajapinta on siirretty uuteen lähteeseen. Tämä ei ole pelkkä riippuvuuden versionumeron vaihto.

### Toteutuksen ajantasainen rajaus

`ui/presets.slint` tarjoaa UiButton-, UiIconButton-, UiMenuButton-, UiTextField-, UiTextArea-, UiComboBox-, UiCheckBox-, UiSwitch-, UiSpinBox-, UiSlider-, UiRadioGroup-, UiTabs-, UiListView-, UiProgressBar-, UiScrollView-, UiCard-, UiSeparator-, UiInfoBar-, UiEmptyState- ja UiValidationMessage-esiasetukset. Alla olevat `Fluent...`-nimet ovat semanttinen suunnittelukatalogi; ne eivät ole rinnakkaisia Slint-tyyppejä. Vastaava `Ui...`-tyyppi on käytettävissä, vaikka vanhassa katalogirivissä lukee vielä `[TOTEUTTAMATTA]`; merkintä koskee koko rivillä kuvattua tavoitekäytöstä, ei pelkkää perusohjainta.

Todellisten sivujen Button/LineEdit/ComboBox/CheckBox/ScrollView-tuonnit kulkevat nyt yhteisten presettien kautta. Kilpailutietojen callbackit ja laskenta säilyvät. Ulkoasu valitaan Tietoja-ikkunasta: järjestelmä, vaalea, tumma tai Dark Spruce. Järjestelmätila seuraa Slintin alustasiltaa; tuntematon järjestelmätila käyttää vaaleaa. Tehosteasetus on erillinen ja oletuksena pois. Galleria käyttää vain muistissa olevaa esitystilaa eikä avaa käyttäjän asetuksia tai kilpailupalveluja.

Mica/Acrylic, järjestelmän korostusvärin seuranta, suuren kontrastin täydellinen alustaintegraatio ja pikselitason vertailu aitoon WinUI 3 Galleryyn ovat edelleen tekemättä. Näitä ei saa kuvata toteutetuiksi.

## 1. Viralliset viitelähteet (ensisijaiset linkit)

| Lähde | Mikä tulee tarkistaa |
| --- | --- |
| [Microsoft Fluent 2 – design tokens](https://fluent2.microsoft.design/design-tokens/) | Globaalit + semanttiset alias-tokenit, light/dark/high-contrast |
| [Windows 11 – geometry](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/geometry) | 0/4/8 epx corner radius, kontrollit / popupit |
| [Windows 11 – typography](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography) | Segoe UI Variable, type ramp, painot, rivikorkeus |
| [Windows 11 – materials](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/materials) | Mica, Acrylic ja modal Smoke; niiden oikeat käyttötapaukset |
| [Windows app materials](https://learn.microsoft.com/en-us/windows/apps/develop/ui/materials) | Materiaalien alusta- ja kontrastifallbackit |
| [Windows 11 – navigation](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/navigationview) | Pane/nav, valinta, mukautuminen ikkunakokoon |
| [Windows 11 – scroll controls](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/scroll-controls) | ScrollView/ScrollViewer, automaattinen näkyvyys, kahden akselin säännöt |
| [Windows – forms](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/forms) | Oikean input-kontrollin valinta |
| [Windows – Segoe Fluent Icons](https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-fluent-icons-font) | Kuvakkeen merkitys, optiset koot, alustasaatavuus |
| [Fluent 2 – shapes](https://fluent2.microsoft.design/shapes) | Komponenttimuodot ja visuaalinen sanasto |
| [Fluent 2 – button](https://fluent2.microsoft.design/components/web/react/core/button/usage/) | Variantit, primary, sisältö, fokus, saavutettavuus |
| [Fluent 2 – slider](https://fluent2.microsoft.design/components/web/react/core/slider/usage/) | Epätarkan arvon valinta, step, track, keyboard |
| [Fluent 2 – field](https://fluent2.microsoft.design/components/web/react/core/field/usage/) | Label, helper, validation, error |
| [Fluent 2 – input](https://fluent2.microsoft.design/components/web/react/core/input/usage/) | Lyhyt tekstisyöte ja otsikointi |
| [Fluent 2 – combobox](https://fluent2.microsoft.design/components/web/react/core/combobox/usage/) | Pitkät valintalistat, haku, yksittäis-/monivalinta |
| [Fluent 2 – checkbox](https://fluent2.microsoft.design/components/web/react/core/checkbox/usage/) | Valinnaiset valinnat ja indeterminate-tila |
| [Fluent 2 – switch](https://fluent2.microsoft.design/components/web/react/core/switch/usage/) | Välittömästi vaikuttava kyllä/ei-valinta |
| [Fluent 2 – spin button](https://fluent2.microsoft.design/components/web/react/core/spin/usage/) | Tarkka numeroarvo, step ja rajat |
| [Fluent 2 – dialog](https://fluent2.microsoft.design/components/web/react/core/dialog/usage/) | Modaalit, kolme osaa, focus trap ja vieritys |
| [Fluent 2 – menu](https://fluent2.microsoft.design/components/web/react/core/menu/usage/) | Popup-menun toiminta ja ryhmittely |
| [Fluent 2 – tooltip](https://fluent2.microsoft.design/components/web/react/core/tooltip/usage/) | Aputeksti, sijoittelu ja fokus |
| [Fluent 2 – tabs](https://fluent2.microsoft.design/components/web/react/core/tablist/usage/) | Sivun sisäiset luokitellut näkymät |
| [Fluent 2 – progress bar](https://fluent2.microsoft.design/components/web/react/core/progressbar/usage/) | Determinaattinen/indeterminaattinen työn tila |
| [Slint – Widget Styles](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/style/) | Fluent-widgettyyli, build-time variantit |
| [Slint – Slider API](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/basic-widgets/slider/) | Slintin oikeat `minimum`, `maximum`, `value`, `step`, `changed`, `released` |
| [WinUI 3 Gallery / design resources](https://learn.microsoft.com/en-us/windows/apps/design/downloads/) | Referenssin silmämääräinen ja käyttäytymisvertailu |

**Huom.** Microsoftin Fluent Web -tuotteella ja WinUI 3:lla voi olla erilaisia kokoasteikkoja ja rakennemalleja. Älä esitä alla olevaa AS2026-katalogia Microsoftin sertifioimana pikselitaulukkona. Tarkista käyttäytyminen uusien lähdeversioiden mukaan; kaikki ulkoiset linkit eivät takaa pysyvää sisältöä.


## 2. Viralliset geometria-, typografia- ja saavutettavuusrajat

### Kulmat [MS] — pikselinauha

| Kulma | Käyttö |
| --- | --- |
| **0 epx** | Toisiaan vasten liittyvät suorat reunat, jaetut kontrollit, snapped/maximized-window-konteksti |
| **4 epx** | Painikkeet, lomakekontrollit, listan taustalevyt, scrollbar, progress bar, slider, tooltip |
| **8 epx** | Ikkunoiden ulkoreunat (Windows-hallittu), dialogit, popupit, flyout ja overlay-kortit |

`epx` on Windowsin **effective pixel**; Slintin loogiset pikselit ovat vertailukelpoinen lähtökohta mutta renderöinnin skaalaus on testattava. Älä piirrä Windowsin aidon ikkuna-kehyksen pyöristystä sovelluksen sisälle toisena keinotekoisena pyöristettynä kehyksenä.

### Tekstin Windows type ramp [MS]

| Token ehdotettu | Windows 11 tyyli | Koko / rivikorkeus | Paino |
| --- | --- | --- | --- |
| `text-caption` | Caption | **12 / 16 epx** | Regular |
| `text-body` | Body | **14 / 20** | Regular |
| `text-body-strong` | Body strong | **14 / 20** | Semibold (600) |
| `text-body-large` | Body large | **18 / 24** | Regular |
| `text-subtitle` | Subtitle | **20 / 28** | Semibold |
| `text-title` | Title | **28 / 36** | Semibold |
| `text-title-large` | Title large | **40 / 52** | Semibold |
| `text-display` | Display | **68 / 92** | Semibold |

Käytä **Segoe UI Variable** Windowsissa, paikallisesti saatavilla olevaa metrisesti soveltuvaa fonttia Linuxissa. Älä paketoi Microsoftin fonttitiedostoja. Käytä näitä kokoja harkitusti: jokaisen taulukon otsikon ei tarvitse olla `20px`; tiheässä tuloslaskentatyössä 12/16 caption ja 14/20 body ovat riittäviä. Vältä alle 12 px tekstiä ja raskasta boldia; yllä oleva on virallista Windowsin tyypografiaa, mutta ohjelman nykyinen 22 px `Theme.heading-font-size` on **[NYK]**, joka vaatii siirtymäsuunnitelman (ei hiljaista maailmanlaajuista fonttikoon kaksinkertaistusta).

### Kontrasti ja status

- **Teksti vähintään 4.5:1**, merkitykselliset ikonit, ohjainrajat ja tilavihjeet **vähintään 3:1** soveltuvissa tiloissa; vertaile hover/focus/pressed/light/dark. Tarkista työkalulla, ei silmällä.
- Fokus ei saa näkyä **vain** värinmuutoksena; sillä tulee olla tunnistettava kaksitasoinen tai selkeä reunus. Näppäimistöfokus näkyy myös tehosteiden ollessa pois.
- Tulos, varoitus, yhteyskatko, hyväksyntä, virhe ja käytöstäpoisto tarvitsevat tekstin/ikonin lisäksi, eivät yksin punaista/vihreää väriä.
- Luokat, tulokset ja Emit-koodit ovat ensisijaisesti tietoa, eivät koristeita; älä häivytä olennaista dataa 11 px tai matalakontrastiseksi.
- Käyttöliittymän jokaisella muokkauskentällä on pysyvä näkyvä nimi, valinnalla tilan semantiikka ja ikonipainikkeella saavutettava nimi.
- `Tab` / `Shift+Tab` etenevät luonnollisessa käyttöjärjestyksessä; Enter/Space aktivoivat soveltuvan ohjaimen; Escape sulkee oikean popupin/dialogin palauttaen kohdistuksen. Ei focus trap -vuotoja.


## 3. AS2026 preset -tokeneiden katalogi [AS]

**Tärkeä ero:** Seuraavat ovat **ehdotettuja projektin esiasetuksia**, eivät kaikkien osien Windowsin virallisia kokoja. Ne muodostavat yhden yhteisen toteutussopimuksen, ja muutokset tehdään **keskitetysti** `ui/theme.slint` / tulevan preset-kirjaston kautta.

### Etäisyydet, korotukset, korkeudet

| Preset | Ehdotettu arvo | Käyttö / rajoitus |
| --- | ---: | --- |
| `spacing-0` | 0px | Liittyvät osat |
| `spacing-1` … `spacing-8` | **4, 8, 12, 16, 20, 24, 32, 40px** | Ainoa perusväliasteikko |
| `radius-control` | **4px [MS]** | Painike, input, valinta |
| `radius-overlay` | **8px [MS]** | Popup, modaali |
| `radius-none` | **0px [MS]** | Suora yhteisreuna |
| `stroke-subtle` / `stroke-focus` | 1px / 2px | Sovelluskohtainen; tarkista WinUI/Gallery |
| `control-compact` | **28px** | Vain hyvin tiivis taulukkorivi/työkalurivi; käyttöalue vähintään ~36px |
| `control-default` | **36px [NYK]** | Kaikki tavalliset lomakepainikkeet ja syötteet |
| `control-large` | **44px** | Harvemmin käytettävä suuri CTA, kosketuskäyttö |
| `control-min-text-width` | **76px [NYK]** | Tavallisen nimetyn painikkeen nykyinen minimi |
| `icon-small` / `icon-default` / `icon-large` | **16 / 20 / 24px** | Oikea Segoe Fluent Icons -kokoperhe [MS]; Slint SVG fallback |
| `icon-button-compact/default/large` | **28 / 36 / 44px** | Näkyvä runko, erillinen saavutettava nimi |
| `nav-row` / `nav-row-compact` | **40px [NYK] / 36px** | Sivupalkki ja mukautuva tiivis tila |
| `table-row-compact/default/comfortable` | **32 / 38 [NYK] / 44px** | Tietotiheys valittava asiayhteydessä, ei per-sivu arvailua |
| `form-label-gap` | **4px** | Label → kenttä |
| `form-row-gap` | **12–16px** | Kenttäryhmien väli |
| `page-gutter` / `card-padding` | **20 [NYK] / 16px [NYK]** | Päivitetään tarvittaessa yhdenmukaisesti |
| `dialog-padding` / `dialog-edge-gap` | **24 [NYK] / 12px [NYK]** | Otsikko, sisältö ja alapainikkeet |
| `dialog-narrow/default/wide/preview` | **420 / 560 [NYK] / 840 / 1100 [NYK] px** | Suurimmat leveydet; rajoita ikkunaan, vieritä sisältö |
| `sidebar-expanded` | **200px [NYK]** | Ei pakota 200 px leveyttä kapeaan ikkunaaan |
| `scrollbar-reserve` | **14px [NYK]** | Nykyisen kopioidun Slint-tyylin varaama vierityskaista |
| `scrollbar-indicator` / `scrollbar-thumb` | **2 / 6px [MS-esimerkkikuva]** | Windowsin panning-indicator vs käsiteltävä thumb; luo toiminnallisesti, älä valehtele puuttuvasta toteutuksesta |
| `slider-track` / `slider-thumb-default` | **4 / 20px [AS]** | Ehdotettu rakentamattoman slider-komponentin lähtömitta |
| `slider-thumb-large` | **24px [AS]** | Harvemmin tarvittava suurennettu kohde |
| `tooltip-offset` | **4px [Fluent Web]** | Popup ei peitä käyttäjän toimintaa |
| `menu-maximum-width` | **300px [Fluent Web]** | Yläraja Fluent Webin guideline; WinUI-vertailu ja pitkä suomi huomioitava |

**Ei piilotettuja paikallisia kopioita:** esimerkiksi `ui/results.slint` ei saa asettaa omia `height: 34px` tai `border-radius: 7px` ilman virallista poikkeusta dokumentissa. Vältä muuttamasta olemassa olevia kokoarvoja ilman kuvallista ja toiminnallista vertailua.

### Värit – nimeä roolin mukaan, älä hexin mukaan

Luonnostele kaksi tasoa:

- **Globaalit arvot:** väriportaiden perusarvot, neutraalit, aksentit ja kontrastivastineet sekä geometria.
- **Semanttiset alias-tokenit:** `canvas`, `layer`, `surface`, `surface-raised`, `control-rest`, `control-hover`, `control-pressed`, `control-disabled`, `text-primary`, `text-secondary`, `text-disabled`, `stroke-default`, `stroke-subtle`, `focus-outer`, `focus-inner`, `accent-rest`, `accent-hover`, `accent-pressed`, `selection-rest`, `danger`, `warning`, `success`, `modal-smoke`.

**[NYK]** `ui/theme.slint` sisältää tummat `canvas/surface/surface-raised/line/ink/muted/accent` jne. Niitä ei saa esitellä Microsoftin virallisina tokeneina. Nykyisen Dark Spruce -vihreän säilytys on yhteensopivuusvalinta; myöhemmin myös Windows-tyylin omat neutraalit light/dark-palettikartat. `Theme.color-scheme` on nykyisin hardcoded dark; light ja system follow **eivät ole valmis ominaisuus**.

### Kaikki käyttötilat samalla sopimuksella

**Rest, hover, pressed, keyboard-focus, selected/checked, disabled, error, validation success, loading/busy, read-only**. Ne eivät tarkoita samaa kaikissa komponenteissa: älä tee esimerkiksi `disabled = readonly`, `hover = focus` tai `selected = pressed` -sekoituksia. Jokainen muuttaa vain sille kuuluvia semanttisia tokeneita; hitbox ja lukutila säilyvät selkeinä.

### Typografian muutospelisääntö

Käytä `text-caption/body/body-strong/subtitle/title` -rooleja. **Älä käytä satunnaisesti 11px/13px/15px pelkän tilansäästön vuoksi.** Jos testissä 900 × 600 -ikkuna ei mahdu, muuta asettelua, ryhmittelyä ja vieritystä ennen kuin kutistat tekstiä.


## 4. Komponenttipresettien kattava toimintokatalogi

Sarakkeen `Komponentti` nimet kuvaavat **suunniteltua yhteistä AS2026 API:a**, eivät kaikkien osalta vielä olemassa olevaa Slint-vientiä. `[TOTEUTTAMATTA]` vaatii toteutuksen; älä käytä suoraan, kunnes rakennettu ja testattu.

### 4.1 Painikkeet ja komennot

| Preset | Koko / variantti | Käytä näin |
| --- | --- | --- |
| `FluentButton.Compact` | 28px, neutral | Taulukon välittömässä rivitoiminnossa, kun hallintapinta on pieni; 36+ px hitbox |
| `FluentButton.Default` | 36px, neutral **[NYK]** | Kaikki tavalliset painikkeet; päivitykset ja tietoja nykyinen vertailukohta |
| `FluentButton.Large` | 44px, neutral | Ohjattu aloitustoiminto tai suurempi kosketusalue |
| `FluentButton.Primary` | sama kuin koko, aksenttitäyttö | Vain **yksi** päätoiminto samassa toimintaryhmässä |
| `FluentButton.Subtle` | 28/36/44, huomaamaton | Taulukon hover-rivityökalut ja ylimääräiset toiminnot |
| `FluentButton.Outline` | 28/36/44, rajaus | Toissijaiset toiminnot ilman väripainetta |
| `FluentButton.Danger` | 36/44, vaarallinen toiminto | Poista/korvaa; vahvistus tarvittaessa eikä väärää primary-väriä |
| `FluentButton.IconOnly` | 28/36/44 + 16/20/24px icon | Näytä tooltip ja saavutettava suomenkielinen nimi |
| `FluentButton.Menu` | 36px + chevron | Avaa Toiminnot-listan; ei muuta kilpailua ennen valintaa |
| `FluentButton.Split` **[TOTEUTTAMATTA]** | 36px, primary + nuoli | Päätoiminto sekä sen vaihtoehdot; keskinäinen reunus 0px |
| `FluentButton.Toggle` | 36px, neutral/selected | Vain toolbar-tyyppiset päälle/pois-näppäimet; ei korvaa asetusswitchiä |
| `FluentButton.Compound` **[TOTEUTTAMATTA]** | 44+px, title + description | Harvinainen kaksirivinen iso valinta; ei navigoinnin oletus |

Ohje: aktiivinen verbi, lausekielinen teksti (esim. `Tallenna`, `Yhdistä kilpailuun`), ei pistettä. Painikkeen hover/pressed/focus pitää tuottaa oikeat tilat myös `Theme.effects-enabled = false`. Älä käytä `Rectangle`-elementtiä kuvitteellisena nappina vain ulkonäön vuoksi, sillä se ohittaa Slintin saavutettavuuden. `AccentButton` käyttää [NYK] samaa varsinaista Buttonia `primary: true` -tilassa.

### 4.2 Teksti- ja syötekomponentit

| Preset | Koko / variantti | Käytä näin |
| --- | --- | --- |
| `FluentField.Text` | 36px input + pysyvä label | Kilpailijan nimi, auto, Emit-numero |
| `FluentField.Compact` | 28px + label | Lyhyt työkalurivin filtterisyöte (ei kaikkeen tietoon) |
| `FluentField.Large` | 44px + label | Korkean näkyvyyden yksittäinen syöte |
| `FluentField.Search` | 36px, search icon | Hae nimellä, numerolla tai Emit-numerolla; selvä label myös kun placeholder katoaa |
| `FluentField.ReadOnly` | 36px, selkeä lukutila | Ei saa näyttää disabled-tietoa, jota ei voi lukea/kopioida |
| `FluentField.Error` | rajaus + viesti | Kerro tarkka virhe sekä korjaustapa; arvot eivät katoa |
| `FluentField.Warning` | varoitus + teksti | Epävarma data, kelvollinen mutta tarkistettava |
| `FluentField.Multiline` **[TOTEUTTAMATTA]** | 3+ rivin korkeus | Vapaat muistiinpanot; pysyvä label ja sisäinen scroll |
| `FluentField.Masked` | 36px + näytä/piilota | Salasana vain tarvittaessa; ei koskaan lokiteta |
| `FluentField.Numeric` | 36px + yksikkö | Tarkka lukuarvo, esim. virhepisteet/sekunnit |
| `FluentField.Time` | 3 synkronoitua syötettä | `h : min : s`; validointi, etunollat ja yön yli jatkuva ajanotto |
| `FluentField.PIN4` **[TOTEUTTAMATTA]** | 36/44px, `0000`–`9999` | LAN-paritus issue #41; **säilytä nollat ja varmista palvelinrajoitukset** |
| `FluentField.Combobox` | 36px + list popup | Luokka tai muu rajallinen sallittu arvo |
| `FluentField.SearchableCombo` **[TOTEUTTAMATTA]** | 36px + suodatus | Pitkä, hakua tarvitseva valintalista |
| `FluentField.Spin` **[TOTEUTTAMATTA]** | 36px + stepper | Tarkat rajatut numerot, `min/max/step`; nuolinäppäimet |
| `FluentField.Date` **[TOTEUTTAMATTA]** | 36px | Vain kilpailun päivämäärän kaltaiseen päivätietoon |

**Field-sääntö:** Label aina näkyvissä kentän yläpuolella, lisäohje ja validointi sen yhteydessä. Placeholder on esimerkki/hintti, ei labelin korvike. Validoi sekä käyttöliittymässä että Rust-ytimessä; teema ei saa luoda rinnakkaista laskentalogiikkaa. Tallennusvirhe ja kenttävirhe ovat eri tiloja.

### 4.3 Valinnat ja ohjaus

| Preset | Koko / variantti | Semantiikka |
| --- | --- | --- |
| `FluentCheckBox.Default` | 18px [NYK] box + label | Valinnainen toiminto, monivalinta |
| `FluentCheckBox.Indeterminate` **[TOTEUTTAMATTA]** | sama | Valitse kaikki, kun vain osa on valittu |
| `FluentRadioGroup` **[TOTEUTTAMATTA]** | valintaryhmä | Täsmälleen yksi sallituista vaihtoehdoista |
| `FluentSwitch` **[TOTEUTTAMATTA]** | noin 40×20px [AS] | **Heti vaikuttava** päälle/pois-asetus; älä tee pääkäyttöön valintapainiketta |
| `FluentSegmented` **[TOTEUTTAMATTA]** | 36px ryhmä | 2–3 läheistä toisensa poissulkevaa näkymää |
| `FluentSlider.Default` **[TOTEUTTAMATTA]** | 4px track, 20px thumb [AS] | Epätarkka arvo, esim. kirkkaus; vakiona vaakasuora |
| `FluentSlider.Stepped` **[TOTEUTTAMATTA]** | samat mitat + `step` | Harvat portaat; merkit vain jos ymmärrettävät |
| `FluentSlider.Large` **[TOTEUTTAMATTA]** | 4px track, 24px thumb [AS] | Harvempi tilanne, suurempi kosketusalue |
| `FluentSlider.Vertical` **[TOTEUTTAMATTA]** | suunta pystysuora | Vain kun tieto on luonnostaan pystysuuntainen |
| `FluentSlider.Range` **[TOTEUTTAMATTA]** | kaksikahvainen | Ei rakentamista ilman selvää tarvetta ja täyttä keyboard-tukea |

**Tärkeä:** Älä käytä slideria tarkkaan kilpailupisteeseen, lähtöaikaan, RT/AT-lukuun, emit-korjaukseen, lähtöväliin tai IP/porttiarvoon. Niihin kuuluu **tarkka numeerinen tekstikenttä/Spin**. Sliderin arvolla tulee olla saavutettava nimi, näkyvät min/max, numeerinen arvo (tooltip tai kenttä) ja nuolinäppäintuet. Vaikka Slintin täysi vakio-`Slider` tukee `minimum, maximum, value, step, orientation, changed, released`, **nykyinen `ui/fluent/style/std-widgets.slint` ei exportoi Slideriä**, joten suunnittele ja testaa sen palauttaminen ennen käyttöä. `released` soveltuu sitoutuvaan muutokseen, `changed` esikatseluun.

### 4.4 Vieritys, luettelot, taulukot ja navigaatio

| Preset | Rakenne / mitat | Käytä näin |
| --- | --- | --- |
| `FluentScrollView.Vertical` | 14px reserved [NYK], scrollbar auto | Lomakkeet, tekstin pitkä näkymä |
| `FluentScrollView.Horizontal` | auto + header mukana | Vain leveä tietotaulukko |
| `FluentScrollView.Both` | 2 akselia | Vain kun sisältö tosiasiassa vaatii molemmat |
| `FluentScrollBar.Panning` | ohut 2px indicator [MS] | Kosketus/panning/rauhoittunut tila — vain jos toteutettu |
| `FluentScrollBar.Interactive` | 6px thumb [MS], 14px varaus [NYK] | Hiiren raahaus; handle ei saa kadota toiminnallisesti |
| `FluentDataTable.Compact` | rivit 32px [AS] | Tiivis tekninen raporttitaulukko, ei joka käyttöön |
| `FluentDataTable.Default` | rivit 38px [NYK] | Kilpailijat, Emit ja tulokset |
| `FluentDataTable.Comfortable` | rivit 44px [AS] | Kosketuskäyttö tai leveä näyttö |
| `FluentList.Details` | listan vasen puoli, editori oikealla | Kilpailijat ja vastaavat list/details-työt |
| `FluentNavItem.Default` | 40px [NYK], 3px active-rail [NYK] | Pääsivujen sivupalkki |
| `FluentNavItem.Compact` | 36px [AS], tooltip | Kapeaan tilaan vasta saavutettavuuden tarkistuksen jälkeen |
| `FluentTabList` **[TOTEUTTAMATTA]** | 36px, sisäinen sivuvaihto | Läheiset sisällöt, ei koko sovelluksen navigointiin |
| `FluentTree` **[TOTEUTTAMATTA]** | 36/40px rivi | Vain oikea hierarkkinen data, ei käytä listan sijasta |
| `FluentPagination` **[TOTEUTTAMATTA]** | 36px komennot | Tarvittaessa erittäin suurelle datalle; älä korvaa tehokasta listaa turhaan |

**Vierityksen muuttumaton invariantti:**

`maxScroll = max(0, contentExtent - viewportExtent)`  
`scrollOffset = clamp(scrollOffset, 0, maxScroll)`.

Kun sisältö **kutistuu** tai osio suljetaan, korjaa scroll-offset uuden asettelun **jälkeen**. Jos sisältö mahtuu, offset on 0 ja koko sisältö on saavutettavissa; jos sisältö ylivuotaa edelleen, pidä sallittu sijainti. **Älä pakota automaattista vieritystä avattaessa `Seurat ja lisenssit`** (käyttäjä erikseen kielsi sen). Pitkässä lomakkeessa yksi scroll-owner; otsikko ja alapainikkeet pysyvät tarvittaessa näkyvissä. Tab-fokus vierittää kohteen näkyviin. Varmista ettei scrollbar jää piiloon kun vanhaa sijaintia ei voi palauttaa.

**Taulukkosopimus:** oikeat headerit, tasaiset sarakkeet, näkyvä valinta + fokus, mouse/keyboard, wheel, tyhjä/virhe/lataus, pitkän nimen ellipsis + koko arvon tarkistus. Pidä tulosarvot ja aikaleimat luettavina. Älä riko alkuperäisen Emit-rivin korjausta eikä raakadataa. Tietotaulukon vaakavieritys siirtää otsikkoriviä ja dataa yhdessä.

### 4.5 Pinnat, dialogit, popupit

| Preset | Koko / kulma | Käyttö |
| --- | --- | --- |
| `FluentCard.Default` | padding16 [AS], radius4 [MS] | Kevyesti korotettu paneeli, ei jokaisen kentän oma laatikko |
| `FluentCard.Section` | padding16–24 [AS], radius4 | Selkeä lomakkeen osa |
| `FluentDialog.Small` | max420px [AS], radius8 [MS] | Lyhyt vahvistus |
| `FluentDialog.Default` | max560px [NYK], radius8 | Tavallinen asetus/muokkaus |
| `FluentDialog.Wide` | max840px [AS], radius8 | Monimutkainen lista/editori |
| `FluentDialog.Preview` | max1100px [NYK], radius8 | Excel-esikatselu / laaja luokkataulukko |
| `FluentFlyout.Default` | radius8 [MS] | Kevyt ei-modaalinen valinta |
| `FluentMenu.Default` | popup max300 [Fluent Web] | Toiminnot, kontekstivalikko; tarkista natiivireferenssi |
| `FluentTooltip` | radius4 [MS], 4px offset [Fluent Web] | Vain lisätieto, ei virheen ainoa ilmoitus |
| `FluentTeachingTip` **[TOTEUTTAMATTA]** | overlay8 [MS] | Harvinainen ohjaava huomautus, ei jatkuva banneri |
| `FluentModalSmoke` | musta dimmer [MS] | Modal-only, EI lisäblur oletuksena |
| `FluentMicaBackdrop` **[TOTEUTTAMATTA]** | Windows desktop background | Vain appin peruspintaan, jos efekti sallittu ja alusta tukee |
| `FluentAcrylicFlyout` **[TOTEUTTAMATTA]** | Windows Acrylic | Transient surface; vain valinnaisen efektin sallimalla |
| `FluentDivider` | 1px [AS] | Lohkojen rajaus, ei raskasta varjoseinää |

**Dialogin rakenne:** otsikko (**pysyvä**), body (**vieritettävä**), action-footer (**pysyvä**) ja turvallinen sulkeminen. Focus trap ja paluu laukaisijalle; avain- ja näppäintoiminnot suoritettava **ennen popupin tuhoamista**. Ei haamu-`FocusScope`-kerroksia, jotka sieppaavat pointer-eventit. Vaarallista tallentamatonta muutosta ei saa sulkea pelkällä ulkonapsautuksella, vaikka esimerkkisivuston yleinen modal-ohje sallii light dismissin.

### 4.6 Informaatio, eteneminen ja tilat

| Preset | Esitystapa | Käytä näin |
| --- | --- | --- |
| `FluentStatusText` | 12–14px [MS-tyyli] | Valittu kone, tallennus, Emit-lukija, virhe |
| `FluentInfoBar.Info` **[TOTEUTTAMATTA]** | neutraali status | Vain asianmukainen paikallinen tieto |
| `FluentInfoBar.Warning` **[TOTEUTTAMATTA]** | varoitus + viesti | Vain silloin kun käyttäjän tulee huomata riski |
| `FluentInfoBar.Error` **[TOTEUTTAMATTA]** | virhe + toiminta | Tiedon tallennus/laitevika, ei pelkkä punainen laatikko |
| `FluentBadge.Status` **[TOTEUTTAMATTA]** | label + tarvittaessa icon | Ei rankingin virheellistä värittämistä |
| `FluentProgressBar.Determinate` **[TOTEUTTAMATTA]** | tunnettu prosentti | Pitkä tiedonsiirto, päivitys tai vienti |
| `FluentProgressBar.Indeterminate` **[TOTEUTTAMATTA]** | työ käynnissä | Muuttumaton selkeä teksti, kun tehosteet pois |
| `FluentProgressRing` **[TOTEUTTAMATTA]** | kompakti | Harvinainen komennon vieressä; ei pakollista animaatiota |
| `FluentToast` **[TOTEUTTAMATTA]** | ei-modaalinen ilmoitus | Lyhyt onnistuminen ilman sisällön peittoa |
| `FluentEmptyState` **[TOTEUTTAMATTA]** | teksti + toiminto | Ei kilpailua/ei kilpailijoita/ei hakutuloksia |
| `FluentSkeleton` **[TOTEUTTAMATTA]** | staattinen offline-first | Jos tulevaisuudessa asynkroninen lataus tarvitsee sen |
| `FluentMetric` | suuri luku + label | Yleiskatsauksen lukumäärät; oikea tietosisältö |
| `FluentSeparator` | 1px [AS] | Selkeä visuaalinen ryhmittely |

**Edistymisen merkitys > animaatio:** Kun efektit on kytketty pois, indeterminaattinen työ ei saa näyttää jäätyneeltä. Näytä `Ladataan…`, työn nimi ja tarvittaessa tilastot tai laskeva määrä staattisesti päivittyen tapahtumien mukaan; vain koristeellisen viivan loputon liike on pois. Etenemispalkki ei saa koskaan valehdella prosenteista.

### 4.7 Ikonit, oikeat sanoitukset, yhteydet

- Käytä ensisijaisesti **Segoe Fluent Icons** -metaforia/virallisesti lisensoituja korvaavia SVG-tiedostoja; kaikkia fontteja **ei saa jakaa Linux-paketissa**. Nykyisten `ui/fluent/style/_*.svg`-resurssien lisenssit säilyvät.
- Kuvakkeen koko 16/20/24 [MS]. Valintanäkymät eivät saa perustua pelkkiin emoji-merkkeihin, erityisesti ilmoituksissa ja virhetiloissa.
- Rivin tapahtuma = verbi (`Tallenna`, `Tarkista`, `Päivitä tulostinluettelo`); määritys = substantiivi (`Tulostin`, `Luokka`). Lausekielinen isotuskäytäntö, ei turhia huutomerkkejä.
- Yhteystila näyttää **Paikallinen / Isäntä / Liitetty / Ei yhteyttä / Synkronointi kesken** todellisen tiedon mukaan. Värin lisäksi teksti, yhteyden virhe ja jonossa olevat muutokset. Tämä dokumentti ei muuta LAN-protokollaa eikä paritussääntöä.


## 5. Slint-toteutusmalli: valmiit ja puuttuvat esiasetukset

### Presettikirjaston ensimmäinen toteutus [BETA, VAATII GUI-VERIFIOINNIN]

`ui/presets.slint` tarjoaa yhteiset kontrollit ja semanttiset viestit. `UiButton` käyttää `preset-size`-arvoja `compact/default/large` ja `preset-variant`-arvoja `standard/primary/subtle/danger`. `UiSlider` perii Slintin Sliderin. `UiSwitch`, `UiSpinBox`, `UiRadioGroup`, `UiTextArea`, `UiTabs` ja `UiProgressBar` käyttävät samaa 1.18.1-lähdepohjaa. `UiInfoBar`, `UiEmptyState` ja `UiValidationMessage` ovat pieniä yhteisiä esityskomponentteja. `NavItem`, `DataTable`, `Workspace`, `DialogSurface` ja `DialogActions` säilyvät yhteisinä AS2026-komponentteina; niistä ei tehdä tarpeettomia rinnakkaisia kopioita. Tooltip on Slintin oma `Tooltip`-elementti, jota ei voi periä.

Gallerian ja todellisten sivujen testit täydentävät lähdekooditarkistuksia. Windowsin materiaaleja tai kaikkien ohjainten WinUI-pikselivastaavuutta ei ole vahvistettu. [Gallerian käyttö- ja testausohje](FLUENT2_GALLERIA.md).

### Toteutettujen presettien API

Kaikki koot ovat loogisia pikseleitä. Nimet ovat `ui/presets.slint`-vientejä. Perityn Slint-ohjaimen tavalliset ominaisuudet ja callbackit säilyvät; tämän taulukon lisäykset ovat AS2026:n omia.

| Komponentti | Ominaisuudet ja käyttö |
| --- | --- |
| `UiButton` | `preset-size`: compact/default/large (28/36/44); `preset-variant`: standard/primary/subtle/danger; `text`, `icon`, `enabled`, `clicked` säilyvät |
| `UiIconButton` | Sama painike, `label` on pakollinen saavutettava nimi; aseta `icon`; teksti ei korvaa kuvaketta |
| `UiMenuButton` | Painike avausnuolella; `expanded` välittää saavutettavan tilan; omistaja avaa ja sulkee oikean popupin |
| `UiTextField` | Slint LineEditInterface + `preset-size`; `text`, `placeholder-text`, `read-only`, `edited`, `accepted`, valinta ja kumoa/tee uudelleen |
| `UiTextArea` | Monirivinen TextEdit, sanarivitys; vähintään 100 korkea, oletusehdotus 120 |
| `UiSpinBox` | `minimum`, `maximum`, `value`, `step-size`, `read-only`, `preset-size`; ei muutoksia kilpailun numeromuotoihin |
| `UiComboBox` | `model`, `current-index`, `current-value`, `selected`, `preset-size`; ei väitetä hakukelpoiseksi comboboxiksi |
| `UiCheckBox` | `text`, `checked`, `toggled`, `enabled`, `preset-size`; Space ja fokus Slintiltä |
| `UiSwitch` | `text`, `checked`, `toggled`, `enabled`; vähimmäiskorkeus 36 |
| `UiRadioGroup` | Vain Slintin `RadioButton`-lapset; `title`, `current-value`, `selected`; ei julkista current-index-ominaisuutta |
| `UiSlider` | `label`, `preset-size`, `minimum`, `maximum`, `value`, `step`, `changed`, `released`; pieni/default 20:n ja suuri 24:n nuppi. Tarkkoihin kilpailuarvoihin käytetään tekstisyötettä |
| `UiScrollView` | Slintin `content-*`, `visible-*` ja scrollbar-policy; 14:n oma vierityskaista, supistumisen rajaus |
| `UiTabs` | Slintin `Tab`-lapset ja välilehtien näppäimistölogiikka; sisäisen sisällön luokitteluun |
| `UiListView` | Sisältölapset (`@children`), ei omaa model-ominaisuutta; mallipohjainen tavallinen luettelo on Slintin StandardListView |
| `UiProgressBar` | `progress` 0–1, `indeterminate`; jatkuva liike vain tehosteiden ollessa päällä, muuten staattinen palkki ja saavutettava työn tilan nimi |
| `UiCard` | `raised`, lapsisisältö; teemapinta, viiva ja 4:n kulma |
| `UiSeparator` | Yhden pikselin teemaviiva |
| `UiInfoBar` | `severity`: info/warning/error/success; `text`, vapaaehtoinen `action-text`, `action`; näkyvä tilasana värin lisäksi |
| `UiEmptyState` | `title`, `description`, vapaaehtoinen `action-text` ja `action` |
| `UiValidationMessage` | `text`, `severity`: error/warning/success; rivittyvä kenttäviesti |

Yhteiset sovelluskomponentit `NavItem`, `DataTable`, `Workspace`, `DialogSurface` ja `DialogActions` tarjoavat navigaation, taulukon, lista/tiedot-asettelun sekä modaalin rungon. Älä luo niille toista rinnakkaista Ui-versiota. Slintin oma `PopupWindow`, `ContextMenuArea` ja `Tooltip` täydentävät valikoimaa. Tooltip-elementtiä ei voi periä, joten sille ei tehdä näennäistä UiTooltip-wrapperia.

### Nykyinen komponenttipolku [NYK]

`build.rs` valitsee `.with_style("fluent")` ja hakee `ui/fluent/style`-hakemistosta paikallisesti rajatut `std-widgets.slint`-viennit. Nykyiset vietyjen widgetien nimet löytyvät `ui/fluent/style/std-widgets.slint`-tiedostosta. Sovelluksen lisäkomponentit löytyvät `ui/components.slint`-tiedostosta. Varsinaiset värit/mitat löytyvät `ui/theme.slint`-tiedostosta.

**Nykyisin suoraan käytettävät** (vahvista aina tuoreesta repositoriosta): `Button`, `LineEdit`, `ComboBox`, `CheckBox`, `ScrollView`, `ListView`, `Palette` ja muut kyseisen tiedoston viennit; yhteisinä `AccentButton`, `NavItem`, `FormField`, `DataTable`, `Workspace`, `DialogSurface`, `DialogActions`, `Editor`, `ChoiceRow`, `Metric`, `Heading`, `Note`.

### Nykyiseen Slintiin sopiva painikkeen käyttö

```slint
import { UiButton as Button } from "presets.slint";
import { Theme } from "theme.slint";
import { AccentButton } from "components.slint";

// Esimerkki komponentin sisään:
HorizontalLayout {
    spacing: Theme.space-2;
    Button {
        height: Theme.control-height;
        text: "Peruuta";
        clicked => { /* kutsu olemassa olevaa sulkemistoimintoa */ }
    }
    AccentButton {
        height: Theme.control-height;
        text: "Tallenna";
        clicked => { /* kutsu olemassa olevaa tallennustoimintoa */ }
    }
}
```

Koodi on ohjeellinen komponentin sisälle liitettävä katkelma; se ei ole kokonainen, sellaisenaan suoritettava ikkuna. Nämä koko- ja väriarvot tulevat keskitetystä `Theme`-globaalista, **eivät** eri sivun uusista hex-arvoista.

### Presetien jatkokehitys – suunnitelma, ei vielä kokonaan importoitava toteutus

Nyt toteutettu `ui/presets.slint` antaa ensimmäiset nimet ja komponentit. Seuraavien komponenttien tulee tarjota **yhteiset variantit** kuten `UiButton`-roolit `neutral/primary/subtle/danger` ja `compact/default/large`, `UiSlider`, `UiSpinBox`, `UiScrollView`, `UiDialog` jne. Koska Slintin komponenttien perintä, property-binding ja käytettävät widget-viennit vaihtelevat, **älä kirjoita fiktiivistä Slint-koodia dokumentin perusteella**. Valitse toteutus niin, että:

1. Komponentti käyttää todellista Slint-widgetiä tai täsmällisesti toteutettua accessible-role/callback/focus -vastinetta.
2. Yhtenäiset tilat ja geometria tulevat `Theme` / yhdestä preset-julkaisusta.
3. Slintin olemassa olevat callbackit ja Rust-sidonnat eivät rikkoudu.
4. Jos uuden elementin tulee saada `Slider`, tuo se paikalliseen kopioituun Fluent-widgettyyliin oikein, mukaan lukien lähdekoodit, lisenssit ja regressiotestit, tai toteuta oma aidosti esteetön vastine. Älä sekoita mainline-Slintin widget-exportteja rajattuun AS2026-kopioon.
5. Lisää komponentille kuvalliset tilat (rest/hover/pressed/focus/disabled), functional keyboard-painallustestit ja demo-esimerkki.
6. Muut sivut eivät omista vanhaa ja uutta versiota samasta komponentista rinnakkain.

### Globaalin efektiportin vaatimukset [NYK + suunnitelma]

`Theme.effects-enabled: false` on keskitetyssä `Theme`-globaalissa. Valinta on `Tietoja`-ikkunassa, tallennus Rustin `PreferencesStore`-asetuksiin; valinnan lähtötila off. Nykyisen betan vain ensimmäiset taustanvaihtoanimaatiot on liitetty porttiin. **Älä väitä blurin tai Mican olevan jo olemassa.**

Kaikki tulevat muutokset: animoidut hoverit, siirtymät, easing, fade, shadow, Mica/Acrylic, taustan blur, jatkuva ajan kanssa päivittyvä koristeellinen efekti, hover-glow => portin alle. Eri tehostetta ei saa toteuttaa pelkällä `if windows`-ehdolla, koska käyttäjän valinta voittaa. Windowsin saavutettavuus-/kontrastiasetukset ja systeemin tehostevalinnat voivat estää efektin **vaikka sovelluksessa on päällä**. Tehosteiden poistaminen ei saa estää prosessin todellisen etenemisen ilmoittamista tai poistaa semanttista palautetta.

## 6. Näkymä- ja tiedostokohtaiset velvoitteet — kattaa jokaisen nykyisen `.slint`-tiedoston

| Lähdetiedosto | Mitä tarkastetaan ja mitä preset-komponentteja sen on käytettävä |
| --- | --- |
| `ui/theme.slint` | **Kaikkien** korkeuksien, rajausten, spacing-, teksti-, paletti- ja efektitokenien omistaja. Ei eri sivujen omia värejä. |
| `ui/types.slint` | Vain tyypit ja semanttinen data; ulkoasu ei vuoda mallin kenttiä muuttavaksi logiikaksi. |
| `ui/components.slint` | `Heading, Note, AccentButton, Workspace, DialogSurface, DialogActions, NavItem, FormField, DataTable, EditorEntry, EditorFields, Editor, Metric, ChoiceRow`; jokaiselle tokenit, variantit ja tilat. Tarkista list/details ja poikkeavat scrollit. |
| `ui/app.slint` | Seitsemän pääsivua, sivupalkki, otsikko/tilarivi, Päivitykset ja Tietoja, Toiminnot, pääkomentopinnat, LAN-dialogit, popup-elinkaari, valinta, virheet, pieni ikkuna. |
| `ui/results.slint` | `ResultPoint`, `ResultSummary`, tila-, piste-, aika-, hyväksyntä-, tulostus- ja avauspainikkeiden johdonmukaisuus. |
| `ui/result-entry.slint` | Tulosmuokkaus: label, tarkka numerosyöte, virhevalidointi, fokus, pitkä sisäinen scroll, pysyvä action-footer. |
| `ui/emit-correction.slint` | Raakaleimausten säilytys ja näyttö, rivin korjaus, taulukko, toiminnon tila, jokaisen virheen tarkka kuvaus. Ei muutoksia laitetulkintaan. |
| `ui/beginners-cup.slint` | BC-välilehdet, eri ryhmien listat, valinnat/indeterminate, scroll/focus, tilannekohtainen validointi, action-footer. |
| `ui/combined-class.slint` | Yhdistetyn luokan lomake, pakollinen nimi, numerot, luokkien valinta ja virheet omissa paikoissaan. |
| `ui/draw-classes.slint` | Lähtöarvonnan luokkavalinnat, valitse kaikki, luokkien listan vieritys, action-footer. |
| `ui/start-columns.slint` | Sarakejärjestys, painikkeet siirtoihin, valintalistat, fokus ja ahtaat näytöt. |
| `ui/print-classes.slint` | Tulostuksen luokkavalinnat, paperiasettelu, valitse kaikki, virhetilat, action-footer. |
| `ui/live-publish.slint` | SFTP/FTP-kentät, salasana, osoitteet, status, toiminnon busy/disabled, luokkavalinnat, vieritys. |
| `ui/fluent/style/*.slint` | Paikallinen standardi-widgetkopio (`button, checkbox, combobox, lineedit, menu, scrollview, styling, style-base` jne.): käy läpi `UPSTREAM.sha256`, lisenssit, paletit, FocusBorder, input callbacks. |
| `ui/fluent/common/*.slint` | AboutSlint, lineedit/combobox/listview/menu -pohjat, focus ja accessibility, lokalisaatio, popup-mekanismit ja ylläpidettävä upstream-diff. |
| `ui/fluent/style/_*.svg` | Kuvakkeen näkyvä metafora, skaalautuvuus, colorize, lisenssit ja käytettävyys vaalealla sekä tummalla taustalla. |

**Ei vain rivitasoinen haku:** käy läpi kaikki toiminnot, avaavat painikkeet ja ehdolliset elementit sekä GUI:ssa saavutettavat polut. Jos komponentti on piilossa harvoin esiintyvän kilpailutilanteen takana, tee synteettinen testi sen avaamiseksi. Lähdekoodin katselmointi ilman käyttökoetta ei riitä.

## 7. Uuden komponentin tekemisen pakollinen työjärjestys AI:lle ja kehittäjälle

1. Nimeä toiminto ja valitse oikea komponenttityyppi käyttötehtävän mukaan (tehtävä, ei vain ulkonäkö): tarkka luku → Number/Spin, epätarkka luku → Slider, välitön kyllä/ei → Switch, lomakevalinta → CheckBox, yksi monesta → Radio/Combobox, toiminto → Button.
2. Tarkista taulukosta, löytyykö preset ja onko se **[NYK]** vai **[TOTEUTTAMATTA]**.
3. Avaa Microsoftin virallinen WinUI-ohje sekä Fluent 2 -ohje juuri kyseiselle komponentille. Ota mitat WinUI Gallery -referenssistä silloin kun ne eivät ole dokumentoituja. Erottele lähteet/päätökset.
4. Tarkista `std-widgets.slint`-vienti ja AS2026:n käytössä oleva Slint **1.18.1** (ei uusimman dokumentaation API:sta tehtyä oletusta).
5. Lisää token `ui/theme.slint`-tiedostoon **vain, jos vastaavaa ei ole**. Valitse roolinimi, älä heksakoodia. Lisää yhteinen uusi komponentti yhden kerran `ui/components.slint`- tai hyväksyttyyn `ui/presets.slint`-tiedostoon; käytä sitä muissa näkymissä.
6. Toteuta keyboard, hiiri, fokus, saavutettavat nimet, validation, disabled, overlay-dismiss ja mahdolliset popup-elinkaaren safe-call -ehdot. Älä muuta liiketoiminnan Rust-rajapintaa kuin teknisesti välttämätön esitystilan sidonta.
7. Käy läpi *kaikki* interaktiotilat: off/on (jos relevantti), rest/hover/pressed/focus/disabled, invalid, loading, read-only. Testaa myös hiiren ja näppäimistön yhdistelmäkäyttö.
8. Testaa `Theme.effects-enabled=false`: staattinen oikea lopputila ilman animaatiota; enabled saa lisätä vain hyödyllisen Fluent-tehosteen. Testaa erilaiset DPI-skaalat, Linux+Windows ja valon/tumman tilan kontrasti **vain kun toteutettu**.
9. Päivitä **tämä** dokumentti ja komponentin käyttöesimerkki, jos lisäsit uuden variantin; lisää `ui/fluent/README.md`-tiedostoon upstream-kopioiden muutokset.
10. Aja projektin `docs/BUILDING.md`-tarkistukset (fmt, test, clippy, GUI smoke 100 % / 150 %, Windows+Linux). Nimeä selvästi testit, joita ei voitu suorittaa. **Älä julkaise, älä muuta mainia, älä merkitse valmiiksi ilman näyttöä.**

## 8. Kattavuus- ja regressiomatriisi

**Jokaisen uuden presetiin kuuluvan ohjaimen testit**: Windows 11 WinUI-referenssikuva; Linux fallback; 100/150 % DPI; 900×600 ja leveä ikkuna; tumma/vaalea kun olemassa; kirkas aksentti sekä normaalit kontrastisuhteet; off/on effects; mouse/keyboard; focus/selection; long Finnish label; pitkä kilpailun nimi; empty/loading/disabled/error; uudelleenavaus ja tallennuksen persistenssi.

**Tärkeät todellisen kilpailun tilanteet**: Emit-kortin lisäys, virheellinen puuttuva leima, käsinkorjaus, kilpailijan tietojen muokkaus, BC-yhdistelmä, tulosten julkaisu, Windows-tulostimen vaihto, väliaikainen LAN-yhteyskatko ja konfliktin käsittely. Näillä on oltava vakaa GUI myös tilanteissa, joissa verkko tai laite epäonnistuu.

**Käyttäjän nimenomainen scroll-regressio (#42):** avaa kilpailijan `Seurat ja lisenssit`, vieritä alas, sulje osio, varmista että paneelin alkuun pääsee takaisin *ilman uudelleen avaamista*. **Älä automaattisesti vieritä lisäkenttiin avatessa.**

**Tehosteregressio:** uudella asennuksella ja vanhoilla prefs-asetuksilla tehosteet pois; `Tietoja`-asetus päälle/pois; pysyvyys; epäonnistuneen tallennuksen rollback; ei turhia ajastimia/blur-renderöintiä off-tilassa.

## 9. Dokumentaation vastuut ja versionhallinta

- **Tämä tiedosto on komponenttipresettien normatiivinen lähde betassa**: mitoille, varianteille, valinnoille, tiloille ja kaikkien Slint-tiedostojen yhteisille säännöille. Älä kirjoita ristiriitaista kopioitua ohjeistusta 12 eri .md-tiedostoon.
- `docs/FLUENT2_BETA.md`: projektin kokonaistavoite, valmistumisvaatimukset ja eteneminen; linkitä tähän katalogiin.
- `docs/UI_PERIAATTEET.md`: nykyinen historiallinen/siirtymävaiheen kehitysohje; uudistettaessa siitä tulee **yleisten käyttöliittymäperiaatteiden ja projektirajoitteiden** johdanto, joka linkittää tähän, ei vaihtoehtoinen väritaulukko.
- `docs/KAYTTOLIITTYMAOPAS.md`: kilpailun toimihenkilön **käyttöopas**, ei Slintin lähdekoodin rakennekuvaus. Kun beta on valmis, kirjoita käyttäjän työnkulut ja oikeat kuvakaappaukset tänne.
- `ui/fluent/README.md`: jokaisen vendoroidun Slint-widgetin tekninen muutosloki, alkuperäinen versio, tiedostovertailu ja oikeat lisenssit.
- `docs/README.md`: dokumenttien päävalikko ja linkitys tähän katalogiin; vakaassa `main`-haarassa ei vielä ole tätä tiedostoa ennen omistajan hyväksyntää.

**Käyttäjän julkaisuraja:** kaikki muutokset pysyvät `beta/fluent2-v071`-haarassa (mahdollisesti olemassa oleva luonnos-PR #43); PR ei merkitse yhdistämislupaa. Kenenkään kielimallin tai kehittäjän ei tule tulkita CI-vihreää tai draft-PR:ää julkaisuluvaksi.

---

*Lähteet tarkastettu 25.10.2026; tarkista lähteiden versiot uudestaan varsinaisessa komponenttitoteutuksessa. Projektille ehdotetut preset-koot eivät ole valmistuneiden kontrollien luettelo eivätkä Windows 11 -sertifioitu mittataulukko.*
