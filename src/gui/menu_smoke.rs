//! Oikeiden Slint-valikkopainikkeiden koe synteettisellä kilpailulla.
use super::*;
use slint::platform::{PointerEventButton, WindowEvent};

fn click(app: &AppWindow, x: f32, y: f32) {
    let position = slint::LogicalPosition::new(x, y);
    app.window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    app.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    app.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

pub(super) fn start(app: &AppWindow, state: &SharedState, directory: PathBuf) -> GuiResult<Timer> {
    std::fs::create_dir_all(&directory).map_err(|e| invalid(&io_message(&e)))?;
    let directory = std::fs::canonicalize(directory).map_err(|e| invalid(&io_message(&e)))?;
    state.borrow_mut().print_test = true;
    let path = {
        let mut s = state.borrow_mut();
        let ws = current_workspace(&mut s)?;
        ws.review_results(7)?;
        let route = ws.project().unwrap().radat[0].clone();
        ws.save_beginners_cup(
            as2026::beginners::BeginnersCup {
                nimi: "Valikkokokeen Beginners Cup".into(),
                tunnus: "BC".into(),
                lahderata: route.numero,
                lahdeasemat: route.asemat,
                mukana_asemat: vec![1, 2, 3, 4],
            },
            &[7],
        )?;
        ws.path().to_owned()
    };
    let original = ProjectStore::open(&path)?.project().clone();
    // (Sivu, valikkorivi, odotettu toiminto). Mikky100:n sijoittelu säilyy.
    let cases = [
        (4, 1, "luokka"),
        (4, 2, "yhdistetty"),
        (1, 3, "kilpailija"),
        (1, 4, "arvonta"),
        (4, 3, "bc"),
        (5, 2, "rata"),
        (3, 2, "luokkatuloste"),
        (3, 3, "asematuloste"),
        (0, 2, "uusi"),
        (0, 1, "avaa"),
        (0, 3, "excel"),
        (1, 1, "excel"),
        (1, 2, "lahtoluettelo"),
        (5, 1, "excel"),
        (2, 1, "kehys"),
        (3, 1, "vienti"),
        (3, 1, "vienti-csv"),
        (3, 1, "vienti-html"),
        (0, 0, "paivitys"),
        (1, 0, "paivitys"),
        (2, 0, "paivitys"),
        (3, 0, "paivitys"),
        (4, 0, "paivitys"),
        (5, 0, "paivitys"),
        (6, 0, "paivitys"),
    ];
    let weak = app.as_weak();
    let shared = state.clone();
    let timer = Timer::default();
    let mut phase = 0usize;
    let mut read_count = 0usize;
    let mut unmodal_pixel = [0u8; 3];
    timer.start(TimerMode::Repeated, Duration::from_millis(180), move || {
        let Some(app) = weak.upgrade() else { return; };
        let test_count = cases.len() * 2;
        if phase < test_count * 4 {
            let case = phase / 4;
            let mouse = case >= cases.len();
            let (page, row, expected) = cases[case % cases.len()];
            let method = if mouse { "hiiri" } else { "nappaimisto" };
            match phase % 4 {
                0 => {
                    shared.borrow_mut().test_file_choice = match expected {
                        "avaa" => Some(Some(path.clone())),
                        "excel" => Some(Some(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/excel-import/synteettinen.xlsx")))),
                        "lahtoluettelo" => Some(Some(directory.join(format!("lahtoluettelo-{method}.html")))),
                        "kehys" => Some(None),
                        _ => None,
                    };
                    app.invoke_navigate(page);
                    assert_eq!(app.get_page(), page, "Sivun vaihto: {expected}");
                }
                1 => app.invoke_show_extras_menu(),
                2 => {
                    if mouse {
                        let width = app.window().size().width as f32 / app.window().scale_factor();
                        click(&app, width - 160., 90. + row as f32 * 44.);
                    } else {
                        for _ in 0..row { smoke::key(&app, slint::platform::Key::Tab); }
                        smoke::key(&app, slint::platform::Key::Return);
                    }
                }
                3 => {
                    let stem = format!("{method}-{expected}");
                    match expected {
                        "luokka" => {
                            assert!(app.get_new_record(), "Lisää luokka ({method}): {}", app.get_status_text());
                            save_view(&app, &directory.join(format!("{stem}.png"))).unwrap();
                            let code = if mouse { "HIIRI" } else { "NAPPIS" };
                            assert!(app.invoke_field_commit(0, code.into()));
                            assert!(app.invoke_field_commit(1, format!("Valikkokoe {method}").into()));
                            assert!(!app.get_new_record(), "Luokka tallentui automaattisesti");
                            assert!(ProjectStore::open(&path).unwrap().project().luokat.iter().any(|c| c.tunnus == code));
                        }
                        "yhdistetty" => {
                            assert!(app.get_combined_open(), "Lisää yhdistetty luokka ({method}): {}", app.get_status_text());
                            save_view(&app, &directory.join(format!("{stem}.png"))).unwrap();
                            app.set_combined_name(format!("Valikkoyhdistelma {method}").into());
                            app.invoke_combined_class_toggled(0, true);
                            app.invoke_combined_class_toggled(1, true);
                            app.invoke_combined_close(true);
                            assert!(!app.get_combined_open(), "{}", app.get_combined_error());
                            assert!(ProjectStore::open(&path).unwrap().project().yhdistetyt_luokat.iter().any(|c| c.nimi == format!("Valikkoyhdistelma {method}")));
                        }
                        "kilpailija" | "rata" => { assert!(app.get_new_record(), "{expected} ({method})"); app.invoke_cancel_edit(); }
                        "arvonta" => { assert!(app.get_draw_classes_open()); app.invoke_draw_classes_close(false); }
                        "bc" => { assert!(app.get_bc_open(), "{}", app.get_status_text()); app.invoke_bc_close(false); }
                        "luokkatuloste" | "asematuloste" => { assert!(app.get_print_classes_open(), "{expected}: {}", app.get_status_text()); app.invoke_print_classes_all(true); app.invoke_print_classes_close(true);
                            assert!(shared.borrow().last_print.as_ref().is_some_and(|d| !d.pages.is_empty()), "Tuloste muodostui: {expected}"); }
                        "uusi" => { assert!(app.get_dialog_open()); smoke::key(&app, slint::platform::Key::Escape); }
                        "vienti" | "vienti-csv" | "vienti-html" => {
                            let kind = match expected { "vienti-csv" => 1, "vienti-html" => 2, _ => 0 };
                            for _ in 0..kind { smoke::key(&app, slint::platform::Key::Tab); }
                            smoke::key(&app, slint::platform::Key::Return);
                            assert!(app.get_export_open(), "Vienti valikon kautta");
                            assert_eq!(app.get_export_kind(), kind);
                            let extension = ["xlsx", "csv", "html"][kind as usize];
                            let destination = directory.join(format!("tulokset-{method}.{extension}"));
                            app.set_export_path(destination.display().to_string().into());
                            app.invoke_save_export();
                            assert!(!app.get_export_open(), "{}", app.get_export_error());
                            let bytes = std::fs::read(&destination).unwrap();
                            assert!(!bytes.is_empty());
                            if kind == 0 { assert!(bytes.starts_with(b"PK")); }
                            else { let text = String::from_utf8(bytes).unwrap(); assert!(text.contains(if kind == 2 { "<table" } else { ";" })); }
                        }
                        "paivitys" => assert!(!app.get_status_error(), "{}", app.get_status_text()),
                        "avaa" => assert!(app.get_opened(), "Kilpailu avautui oikeasta tiedostosta"),
                        "excel" => { assert!(app.get_import_open(), "Excel-esikatselu: {}", app.get_status_text()); app.invoke_confirm_action(false); }
                        "lahtoluettelo" => { let html = std::fs::read_to_string(directory.join(format!("lahtoluettelo-{method}.html"))).unwrap(); assert!(html.contains("<table")); }
                        "kehys" => assert!(shared.borrow().test_file_choice.is_none(), "Kehysvalitsin avautui ja peruutettiin"),
                        _ => unreachable!(),
                    }
                    println!("Valikkokoe hyväksytty: {method} / sivu {page} / {expected}");
                }
                _ => unreachable!(),
            }
        } else {
            let step = phase - test_count * 4;
            if step == 0 {
                let reopened = ProjectStore::open(&path).unwrap();
                assert_eq!(reopened.project().leimaluvut, original.leimaluvut, "Alkuperäiset korttiluvut säilyvät");
                assert_eq!(reopened.project().kilpailijat[0].tallennettu_tulos, original.kilpailijat[0].tallennettu_tulos, "Luokkien lisääminen säilyttää lasketun tuloksen");
            }
            if step < 56 {
                let size = [(900, 600), (1366, 768), (1920, 1080), (2560, 1440)][step / 14];
                let page = (step % 14) / 2;
                if step.is_multiple_of(2) {
                    let scale = app.window().scale_factor();
                    app.window().set_size(slint::PhysicalSize::new((size.0 as f32 * scale) as u32, (size.1 as f32 * scale) as u32));
                    app.invoke_navigate(page as i32);
                } else {
                    save_view(&app, &directory.join(format!("{}x{}-sivu-{page}.png", size.0, size.1))).unwrap();
                }
            } else if step == 56 {
                app.invoke_navigate(3);
                assert!(app.get_bc_configured());
                click(&app, 275., 90.);
                assert!(app.get_bc_results(), "Beginners Cup avautuu painikkeesta");
            } else if step == 57 {
                click(&app, 275., 90.);
                assert!(!app.get_bc_results(), "Beginners Cup sulkeutuu samasta painikkeesta");
                app.invoke_select_row(0);
                let pixels = app.window().take_snapshot().unwrap();
                let offset = (8 * pixels.width() as usize + 8) * 4;
                unmodal_pixel.copy_from_slice(&pixels.as_bytes()[offset..offset + 3]);
                app.invoke_edit_result_competitor();
                assert!(app.get_competitor_edit_open());
            } else if step == 58 {
                let pixels = app.window().take_snapshot().unwrap();
                let offset = (8 * pixels.width() as usize + 8) * 4;
                for (before, after) in unmodal_pixel.iter().zip(&pixels.as_bytes()[offset..offset + 3]) {
                    assert!((i16::from(*after) * 2 - i16::from(*before)).abs() <= 2, "Modaalitausta tummentaa neutraalisti noin 50 %, ilman värivirhettä");
                }
                save_view(&app, &directory.join("kilpailijan-muokkaus.png")).unwrap();
                app.invoke_edit_result_competitor();
                assert!(!app.get_competitor_edit_open(), "Muokkaus sulkeutuu uudelleen painamalla");
                app.invoke_edit_result_competitor();
            } else if step == 59 {
                smoke::key(&app, slint::platform::Key::Escape);
                assert!(!app.get_competitor_edit_open(), "Esc sulkee kilpailijan muokkauksen");
                let mut s = shared.borrow_mut();
                s.workspace = None;
                app.set_opened(false); app.set_editable(false); app.set_competition_name("Ei avointa kilpailua".into());
                s.selection = Selection::None;
                update_header(&s, &app).unwrap();
                drop(s);
                app.invoke_navigate(0);
            } else if step == 60 {
                assert!(!app.get_opened());
                save_view(&app, &directory.join("tyhja-yleiskatsaus.png")).unwrap();
                app.invoke_navigate(6);
            } else if step == 61 {
                save_view(&app, &directory.join("asetukset-ilman-kilpailua.png")).unwrap();
            } else if step < 86 {
                // Aloituspolut myös ilman avointa kilpailua, kummallakin syöttötavalla.
                let case = (step - 62) / 4;
                let row = (case % 3) + 1;
                match (step - 62) % 4 {
                    0 => {
                        let mut s = shared.borrow_mut();
                        s.workspace = None;
                app.set_opened(false); app.set_editable(false); app.set_competition_name("Ei avointa kilpailua".into()); s.selection = Selection::None;
                        update_header(&s, &app).unwrap(); drop(s);
                        app.invoke_navigate(0);
                        shared.borrow_mut().test_file_choice = match row {
                            1 => Some(Some(path.clone())),
                            3 => Some(Some(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/excel-import/synteettinen.xlsx")))),
                            _ => None,
                        };
                    }
                    1 => app.invoke_show_extras_menu(),
                    2 => {
                        if case >= 3 { let width = app.window().size().width as f32 / app.window().scale_factor(); click(&app, width - 160., 90. + row as f32 * 44.); }
                        else { for _ in 0..row { smoke::key(&app, slint::platform::Key::Tab); } smoke::key(&app, slint::platform::Key::Return); }
                    }
                    3 => match row {
                        1 => assert!(app.get_opened()),
                        2 => { assert!(app.get_dialog_open()); smoke::key(&app, slint::platform::Key::Escape); }
                        3 => { assert!(app.get_import_open()); app.invoke_confirm_action(false); }
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            } else if step == 86 {
                open_workspace(&mut shared.borrow_mut(), &app, &path, true).unwrap();
                app.invoke_navigate(4);
                app.invoke_add_record();
                assert!(app.get_new_record());
                app.invoke_show_extras_menu();
            } else if step == 87 {
                let width = app.window().size().width as f32 / app.window().scale_factor();
                click(&app, width - 160., 134.);
                assert!(app.get_new_record(), "Keskeneräinen uusi luokka säilyy");
                smoke::key(&app, slint::platform::Key::Escape);
                app.invoke_cancel_edit();
                app.set_editable(false);
                app.invoke_show_extras_menu();
            } else if step == 88 {
                let width = app.window().size().width as f32 / app.window().scale_factor();
                click(&app, width - 160., 134.);
                assert!(!app.get_new_record(), "Vain luku: luokan lisäys estetty");
                click(&app, width - 160., 178.);
                assert!(!app.get_combined_open(), "Vain luku: yhdistetyn luokan lisäys estetty");
                smoke::key(&app, slint::platform::Key::Escape);
                app.set_editable(true); app.invoke_navigate(3);
                app.set_print_busy(true); app.invoke_show_extras_menu();
            } else if step == 89 {
                let width = app.window().size().width as f32 / app.window().scale_factor();
                click(&app, width - 160., 178.);
                assert!(!app.get_print_classes_open());
                click(&app, width - 160., 222.);
                assert!(!app.get_print_classes_open(), "Käynnissä oleva tulostus estää uuden tulostusikkunan");
                smoke::key(&app, slint::platform::Key::Escape); app.set_print_busy(false);
            } else if step == 90 {
                read_count = shared.borrow().workspace.as_ref().unwrap().project().unwrap().leimaluvut.len();
                shared.borrow_mut().test_file_choice = Some(Some(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/synthetic/emit-frame.raw"))));
                app.invoke_navigate(2); app.invoke_show_extras_menu();
            } else if step == 91 {
                smoke::key(&app, slint::platform::Key::Tab); smoke::key(&app, slint::platform::Key::Return);
                assert_eq!(shared.borrow().workspace.as_ref().unwrap().project().unwrap().leimaluvut.len(), read_count + 1, "Oikea Emit-kehystiedosto tallentui valikon kautta");
            } else {
                println!("Valikkokoe valmis: 50 oikeaa valikkovalintaa, luokat tallennettu ja avattu uudelleen; seitsemän sivua neljässä koossa.");
                slint::quit_event_loop().unwrap();
            }
        }
        phase += 1;
    });
    Ok(timer)
}
