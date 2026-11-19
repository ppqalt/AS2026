//! Kilpailun valmistelut ja lyhyt laskentajärjestys. Pikapainikkeet vain navigoivat.
use super::*;

pub(super) fn smoke(app: &AppWindow, state: &SharedState, directory: PathBuf) -> GuiResult<Timer> {
    use std::{cell::Cell, rc::Rc};
    std::fs::create_dir_all(&directory).map_err(|e| invalid(&io_message(&e)))?;
    let before = state
        .borrow()
        .workspace
        .as_ref()
        .unwrap()
        .project()
        .unwrap()
        .to_toml()?;
    let step = Rc::new(Cell::new(0));
    let weak = app.as_weak();
    let shared = state.clone();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, std::time::Duration::from_millis(350), move || {
        let app = weak.upgrade().unwrap();
        match step.get() {
            0 => { app.window().set_size(slint::LogicalSize::new(900.,600.)); app.invoke_navigate(0); app.invoke_open_guide(); }
            1 => {
                assert!(app.get_guide_open());
                save_view(&app, &directory.join("01-manuaalinen.png")).unwrap();
                app.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                    position: slint::LogicalPosition::new(
                        app.window().size().width as f32 / app.window().scale_factor() / 2.,
                        app.window().size().height as f32 / app.window().scale_factor() / 2.), delta_x:0.,delta_y:-1200.,
                });
            }
            2 => {
                assert!(app.get_guide_scroll_offset() < 0., "Pitkä ohje vierii pienessä ikkunassa");
                save_view(&app, &directory.join("02-manuaalinen-loppu.png")).unwrap();
                app.invoke_guide_topic_changed(1);
                assert_eq!(app.get_guide_scroll_offset(),0.);
            }
            3 => {
                save_view(&app, &directory.join("03-emit.png")).unwrap();
                app.invoke_guide_go(2); assert_eq!(app.get_page(),2); assert!(!app.get_guide_open());
                app.invoke_open_guide(); app.invoke_guide_topic_changed(2);
            }
            4 => {
                save_view(&app, &directory.join("04-tulkki.png")).unwrap();
                smoke::key(&app, slint::platform::Key::Tab);
                smoke::key(&app, slint::platform::Key::Escape);
                assert!(!app.get_guide_open());
                for topic in 0..3 {
                    app.invoke_guide_topic_changed(topic);
                    let pages = app.get_guide_steps().iter().map(|s|s.page).collect::<Vec<_>>();
                    assert!(pages.len() >= 6);
                    for page in pages {
                        app.invoke_open_guide(); app.invoke_guide_go(page);
                        assert_eq!(app.get_page(),page); assert!(!app.get_guide_open());
                    }
                }
                assert_eq!(shared.borrow().workspace.as_ref().unwrap().project().unwrap().to_toml().unwrap(),before,
                    "Ohjeiden lukeminen ja pikapainikkeet eivät muuta kilpailua");
                app.window().set_size(slint::LogicalSize::new(1480.,960.));
                app.invoke_navigate(0); app.invoke_open_guide(); app.invoke_guide_topic_changed(2);
            }
            5 => {
                save_view(&app, &directory.join("05-tulkki-suuri.png")).unwrap();
                smoke::key(&app,slint::platform::Key::Escape);
            }
            6 => {
                save_view(&app, &directory.join("06-yleisnakyma.png")).unwrap();
                let mut state = shared.borrow_mut();
                state.print_test = true;
                let path = state.workspace.as_ref().unwrap().path().to_owned();
                let mut config = as2026::tulkki_yhteys::Config::prepare("https://example.invalid/", "koe", "QR-koekilpailu").unwrap();
                config.competition_id = 1; config.revision = 1;
                state.preferences.as_mut().unwrap().save_tulkki(path,config).unwrap();
                drop(state);
                app.invoke_open_tulkki(); app.invoke_open_tulkki_qr(); assert!(app.get_tulkki_qr_open());
                app.invoke_print_tulkki_qr();
                let state = shared.borrow();
                let document = state.last_print.as_ref().expect("QR-tuloste siirtyy tulostusjonoon");
                assert_eq!(document.pages.len(),1); assert!(!document.pages[0].rectangles.is_empty());
                #[cfg(windows)]
                as2026::printing::preview_png(document,&directory.join("qr-tuloste")).unwrap();
                #[cfg(not(windows))]
                std::fs::write(directory.join("qr-tuloste.ps"),as2026::printing::postscript(document)).unwrap();
            }
            7 => {
                save_view(&app,&directory.join("07-qr-tulostus.png")).unwrap();
                smoke::key(&app,slint::platform::Key::Tab);
                smoke::key(&app,slint::platform::Key::Escape); assert!(!app.get_tulkki_qr_open());
                std::fs::write(directory.join("result.txt"), "PASS: kolme ohjetta, kaikki pikapainikkeet, tietojen säilyminen, 900x600/1480x960, vieritys, Tab ja Esc\n").unwrap();
                slint::quit_event_loop().unwrap();
            }
            _ => {}
        }
        step.set(step.get()+1);
    });
    Ok(timer)
}

pub(super) fn select(app: &AppWindow, topic: i32) {
    app.set_guide_scroll_offset(0.);
    let steps = match topic {
        1 => vec![
            (
                "Valitse Emit-ajanotto",
                "Tarkista kilpailun ajanottotapa ja virhepisteasetukset. Emit-kortista otetaan lopulliset ajat.",
                "Avaa asetukset",
                6,
            ),
            (
                "Tarkista laskentaradat",
                "Määritä AT-, JAT-, ULA- ja maaliasemat oikeaan järjestykseen. Tarkista asemien Emit-koodit, tavoiteajat ja laskentasäännöt. Maalin koodin tulee vastata kortin leimaa.",
                "Avaa radat ja asemat",
                5,
            ),
            (
                "Valmistele luokat ja lähdöt",
                "Kohdista luokat radoille. Tarkista luokkien lähtöajat, lähtövälit ja koko kilpailun myöhästymisraja.",
                "Avaa luokat",
                4,
            ),
            (
                "Tarkista kilpailijaparit",
                "Tarkista kilpailunumerot, luokat, Emit-korttien numerot ja parien lähtöajat ennen ensimmäistä laskentaa.",
                "Avaa kilpailijat",
                1,
            ),
            (
                "Käynnistä kortinlukija",
                "Valitse oikea lukijaportti ja käynnistä lukija. Tarkista, että kortti kohdistuu oikealle parille ja leimakoodit näkyvät lukuhistoriassa.",
                "Avaa Emit",
                2,
            ),
            (
                "Laske ja hyväksy tulos",
                "Lue kortti. Tarkista muodostaa asematuloksen uusimmasta Emit-luvusta; käytä sitä myös korjatun korttiluvun jälkeen. Täydennä mahdolliset puuttuvat käsiajat. Tutki laskee muokatut ajat uudelleen. Tarkista RT-puutteet ja muut virheet ennen tallennusta ja tulostusta.",
                "Avaa tulokset",
                3,
            ),
        ],
        2 => vec![
            (
                "Valitse aikojen käyttötapa",
                "Ilman Emitiä puhelimen ajat ovat laskennan ajat. Emit-kilpailussa puhelimen käsiajat tuottavat väliaikatuloksen; Emit-kortin luku korvaa ne lopullisessa laskennassa.",
                "Avaa asetukset",
                6,
            ),
            (
                "Tarkista radat ja kilpailijaparit",
                "Laskentaratojen aika-asemien pitää vastata oikean rivin aika-asemia tyypin ja järjestyksen mukaan. Luokkien pitää osoittaa oikeille radoille ja parien oikeisiin luokkiin.",
                "Avaa radat ja asemat",
                5,
            ),
            (
                "Tuo oikeat rivit",
                "Tuo tai kirjaa jokaisen käytettävän radan oikea rivi Kilvet-näkymään. Tarkista jaksot ja aika-asemien järjestys. Valitse Ajan syöttö käsin jokaiselle tarvittavalle aika-asemalle ja radalle erikseen. Ilman Emitiä aika-asemien ajat syötetään käsin.",
                "Avaa Kilvet",
                7,
            ),
            (
                "Yhdistä ja julkaise kilpailu",
                "Avaa Kilvet-näkymän AS tulkki -yhteys. Anna sivuston osoite; ensimmäinen kilpailun luonti tarvitsee WordPressin ylläpitäjän tunnuksen ja sovellussalasanan. Lähetä kilpailutiedot ja varmista yhteyden tila. Lisäosan tulee olla asennettu sivustolle.",
                "Avaa Kilvet",
                7,
            ),
            (
                "Liity puhelimella ja tulkkaa",
                "Näytä liittymisen QR-koodi AS2026:ssa. Skannaa se puhelimella ja anna tarkastajan nimi. Valitse pari sekä jakso tai Koko kilpailu. Merkitse puuttuvat asemat, syötä vaaditut ajat ja muut virhepisteet. Valmis lähettää kirjaukset. Tarkista, että lähetysjono tyhjenee.",
                "Avaa Kilvet",
                7,
            ),
            (
                "Varmista vastaanotto ja laske",
                "Tarkista Kilvet-näkymän tulkinnoista, että kirjaukset saapuivat oikealle parille. Tulokset-sivulla näkyvät RT-puutteet, muut virheet ja aikojen laskenta. Emit-kilpailun väliaikatulosta ei hyväksytä lopulliseksi ennen kortin lukua.",
                "Avaa tulokset",
                3,
            ),
            (
                "Käynnistä tarvittaessa livetulokset",
                "Avaa Tulokset-sivun Toiminnot-valikosta Live-tulosten julkaisu. Valitse luokat ja käynnistä paikallinen live tai testaa verkkoyhteys ja aloita julkaisu. AS2026:n uudelleenkäynnistyksen jälkeen livejulkaisu käynnistetään uudelleen.",
                "Avaa tulokset",
                3,
            ),
        ],
        _ => vec![
            (
                "Valitse manuaalinen ajanotto",
                "Valitse kilpailun ajanottotavaksi Manuaalinen. Tarkista virhepisteasetukset ja koko kilpailun myöhästymisraja ennen laskentaa.",
                "Avaa asetukset",
                6,
            ),
            (
                "Määritä laskentaradat",
                "Lisää tarvittavat AT-, JAT-, ULA- ja maaliasemat oikeaan järjestykseen. Tarkista tavoiteajat ja laskentasäännöt.",
                "Avaa radat ja asemat",
                5,
            ),
            (
                "Valmistele luokat ja lähdöt",
                "Kohdista luokat radoille ja tarkista ensimmäisen parin lähtöaika sekä luokan lähtöväli.",
                "Avaa luokat",
                4,
            ),
            (
                "Tarkista kilpailijaparit",
                "Tarkista kilpailunumerot, luokat ja jokaisen parin lähtöaika. Lähtöaika tarvitaan käsiaikojen laskentaan. Emit-korttia ei tarvita.",
                "Avaa kilpailijat",
                1,
            ),
            (
                "Valmistele puhelintulkkaus tarvittaessa",
                "Jos ajat ja RT-puutteet kirjataan AS tulkilla, tuo oikeat rivit jokaiselle radalle ja julkaise kilpailutiedot. Tarkempi järjestys löytyy tämän ikkunan AS tulkin käyttö -ohjeesta.",
                "Avaa Kilvet",
                7,
            ),
            (
                "Syötä ajat ja laske tulos",
                "Valitse pari Tulokset-sivulta. Syötä kortin ajat itse tai vastaanota ne AS tulkista. Ajan voi kirjoittaa muodossa 180334. Merkitse todelliset puuttuvat aikamerkinnät erikseen. Tutki laskee muokatut ajat uudelleen. Tarkista RT-puutteet ja muut virheet, hyväksy ja tulosta tulos.",
                "Avaa tulokset",
                3,
            ),
        ],
    };
    app.set_guide_topic(topic.clamp(0, 2));
    app.set_guide_steps(model(
        steps
            .into_iter()
            .map(|(title, body, button, page)| GuideStep {
                title: title.into(),
                body: body.into(),
                button: button.into(),
                page,
            })
            .collect(),
    ));
}
