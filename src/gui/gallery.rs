//! Eristetty komponenttigalleria: vain muistissa oleva esitystila.
use super::GalleryWindow;
use slint::{
    ComponentHandle, Timer,
    platform::{Key, WindowEvent},
};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

fn key(app: &GalleryWindow, text: impl Into<slint::SharedString>) {
    let text = text.into();
    let tab: slint::SharedString = Key::Tab.into();
    if text == tab {
        app.global::<super::Theme>()
            .set_keyboard_focus_visible(true);
    }
    app.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    app.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}

fn pointer(app: &GalleryWindow, pressed: bool, x: f32, y: f32) {
    app.global::<super::Theme>()
        .set_keyboard_focus_visible(false);
    let position = slint::LogicalPosition::new(x, y);
    let button = slint::platform::PointerEventButton::Left;
    app.window().dispatch_event(if pressed {
        WindowEvent::PointerPressed { position, button }
    } else {
        WindowEvent::PointerReleased { position, button }
    });
}

fn capture(app: &GalleryWindow, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = app.window().take_snapshot()?;
    image::save_buffer(
        path,
        buffer.as_bytes(),
        buffer.width(),
        buffer.height(),
        image::ColorType::Rgba8,
    )?;
    Ok(())
}

pub(super) fn run(
    captures: Option<PathBuf>,
    smoke: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let gallery = GalleryWindow::new()?;
    let focus_weak = gallery.as_weak();
    super::input_mode::bind(gallery.window(), move |visible| {
        if let Some(gallery) = focus_weak.upgrade() {
            gallery
                .global::<super::Theme>()
                .set_keyboard_focus_visible(visible);
        }
    });
    super::class_selection::register(gallery.global::<super::ClassSelectionLogic>());
    slint::select_bundled_translation("fi").map_err(|_| "Suomennoksen avaaminen epäonnistui")?;
    gallery.show()?;
    let timer = Rc::new(Timer::default());
    let weak_timer = Rc::downgrade(&timer);
    if let Some(directory) = captures {
        std::fs::create_dir_all(&directory)?;
        let weak = gallery.as_weak();
        let phase = Rc::new(Cell::new(0));
        timer.start(slint::TimerMode::Repeated, Duration::from_millis(350), move || {
            let Some(app) = weak.upgrade() else { return; };
            let current = phase.get();
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                if !smoke {
                    if current == 0 { capture(&app, &directory.join("fluent-gallery.png"))?; }
                    if let Some(timer) = weak_timer.upgrade() { timer.stop(); }
                    return Ok(());
                }
                match current {
                    0 => { assert!(!app.get_effects_enabled()); app.invoke_show_slider(); app.invoke_focus_slider(); }
                    1 => { key(&app, Key::RightArrow); assert_eq!(app.get_slider_value(), 40.); }
                    2 => { key(&app, Key::Home); assert_eq!(app.get_slider_value(), 0.); }
                    3 => { key(&app, Key::End); assert_eq!(app.get_slider_value(), 100.); }
                    4 => { key(&app, Key::LeftArrow); assert_eq!(app.get_slider_value(), 95.); }
                    5 => {
                        let p = app.get_slider_position();
                        app.window().dispatch_event(WindowEvent::PointerMoved { position: slint::LogicalPosition::new(p.x + app.get_slider_width() / 2., p.y + app.get_slider_height() / 2.) });
                        assert_eq!(app.get_slider_value(), 95., "Hiiren pelkkä liike ei saa muuttaa arvoa");
                        app.invoke_reset_scroll(); app.invoke_focus_check();
                    }
                    6 => { key(&app, " "); assert!(!app.get_sample_checkbox(), "Välilyönnin pitää vaihtaa valinta"); }
                    7 => { key(&app, " "); assert!(app.get_sample_checkbox()); app.invoke_focus_text(); }
                    8 => { key(&app, Key::End); key(&app, "!"); assert!(app.get_sample_text().ends_with('!')); }
                    9 => { app.set_internal_scroll_offset(app.get_internal_scroll_minimum()); app.set_scroll_rows(2); }
                    10 => { assert_eq!(app.get_internal_scroll_offset(), 0., "Supistunut sisältö ei saa jäädä alueen ulkopuolelle"); app.set_scroll_rows(15); }
                    11 => {
                        assert_eq!(app.get_internal_scroll_offset(), 0., "Laajentuminen ei saa vierittää itsestään");
                        app.set_internal_scroll_offset(-40.); app.set_scroll_rows(20);
                    }
                    12..=51 => {
                        if current == 12 { assert_eq!(app.get_internal_scroll_offset(), -40., "Laajentuminen säilyttää myös nollasta poikkeavan vierityskohdan"); }
                        let index = (current - 12) / 2;
                        if current % 2 == 0 {
                            app.set_appearance_index(index / 2);
                            app.set_effects_enabled(index % 2 != 0);
                        } else {
                            capture(&app, &directory.join(format!("teema-{}-tehosteet-{}.png", index / 2, index % 2)))?;
                        }
                    }
                    52 => { app.invoke_focus_appearance(); key(&app, Key::Return); }
                    53 => {
                        assert_eq!(app.get_appearance_index(), 9);
                        key(&app, Key::UpArrow); assert_eq!(app.get_appearance_index(), 8);
                        key(&app, Key::DownArrow); key(&app, Key::DownArrow);
                        assert_eq!(app.get_appearance_index(), 9, "Pitkän teemavalikon viimeinen rivi toimii ja valinta pysyy rajoissa");
                        capture(&app, &directory.join("teemavalikon-loppu.png"))?;
                    }
                    54 => { key(&app, Key::Escape); }
                    55 => {
                        app.invoke_show_slider();
                        let p = app.get_slider_position();
                        let x = p.x + app.get_slider_width() * 0.35;
                        let y = p.y + app.get_slider_height() / 2.;
                        pointer(&app, true, x, y); pointer(&app, false, x, y);
                        assert!((app.get_slider_value() - 35.).abs() <= 5.);
                        assert_eq!(app.get_slider_value() % 5., 0.);
                        app.invoke_show_additional(); app.invoke_focus_switch();
                    }
                    56 => { key(&app, " "); assert!(app.get_sample_switch()); app.invoke_focus_number(); }
                    57 => { key(&app, Key::UpArrow); assert_eq!(app.get_sample_number(), 10); }
                    58 => { key(&app, Key::DownArrow); assert_eq!(app.get_sample_number(), 5); capture(&app, &directory.join("lisakomponentit.png"))?; app.invoke_focus_menu(); key(&app, Key::Return); }
                    59 => { assert!(app.get_popup_open(), "Valikkopainikkeen Enter avaa popupin"); capture(&app, &directory.join("valikko.png"))?; key(&app, Key::Escape); }
                    60 => { assert!(!app.get_popup_open(), "Esc sulkee popupin ja päivittää laajentumistilan"); let scale = app.window().scale_factor(); app.window().set_size(slint::PhysicalSize::new((900. * scale) as u32, (600. * scale) as u32)); }
                    61 => { app.invoke_reset_scroll(); }
                    62 => {
                        let size = app.window().size();
                        let scale = app.window().scale_factor();
                        assert!((size.width as f32 / scale - 900.).abs() <= 2. && (size.height as f32 / scale - 600.).abs() <= 2., "Gallerian todellinen koko: {size:?}, skaala {scale}");
                        capture(&app, &directory.join("pieni-900x600.png"))?;
                    }
                    63 => { app.invoke_show_class_picker(); }
                    64 => {
                        assert_eq!(app.get_preview_count(), 6);
                        assert_eq!(app.get_preview_visible_count(), 12);
                        let p = app.get_preview_card_position();
                        let x = p.x + app.get_preview_card_width() - 8.;
                        let y = p.y + 34.;
                        pointer(&app, true, x, y); pointer(&app, false, x, y);
                        assert_eq!(app.get_preview_count(), 5, "Kortin alakulma vaihtaa valinnan tasan kerran");
                        assert!(!app.global::<super::Theme>().get_keyboard_focus_visible());
                        capture(&app, &directory.join("luokkavalitsin-hiiri.png"))?;
                    }
                    65 => {
                        key(&app, " ");
                        assert_eq!(app.get_preview_count(), 6, "Kortin klikkaus säilyttää valintaruudun näppäimistökohdistuksen");
                        key(&app, Key::Tab);
                        assert!(app.global::<super::Theme>().get_keyboard_focus_visible());
                        capture(&app, &directory.join("luokkavalitsin-tab.png"))?;
                        app.set_preview_query(" cup ".into());
                    }
                    66 => {
                        assert_eq!(app.get_preview_visible_count(), 1);
                        let p = app.get_preview_card_position();
                        pointer(&app, true, p.x + 16., p.y + 36.); pointer(&app, false, p.x + 16., p.y + 36.);
                        assert_eq!(app.get_preview_count(), 7, "Suodatettu kortti valitsee alkuperäisen mallin oikean luokan");
                        app.set_preview_selected_only(true);
                    }
                    67 => {
                        assert_eq!(app.get_preview_visible_count(), 1);
                        let p = app.get_preview_card_position();
                        pointer(&app, true, p.x + 16., p.y + 36.); pointer(&app, false, p.x + 16., p.y + 36.);
                        assert_eq!(app.get_preview_count(), 6);
                    }
                    68 => {
                        assert_eq!(app.get_preview_visible_count(), 0, "Poistettu valinta katoaa Vain valitut -rajauksesta");
                        app.set_preview_selected_only(false); app.set_preview_query("".into());
                    }
                    69 => { capture(&app, &directory.join("luokkavalitsin-900x600.png"))?; key(&app, Key::Escape); }
                    70 => { app.set_sizing_content(100.); }
                    71 => { assert_eq!(app.get_sizing_height(), 300.); app.set_sizing_content(800.); }
                    72 => { assert_eq!(app.get_sizing_height(), 800.); app.set_sizing_content(2000.); }
                    73 => { assert_eq!(app.get_sizing_height(), 1000.); app.set_sizing_available(600.); }
                    74 => {
                        assert_eq!(app.get_sizing_height(), 576.);
                        assert_eq!(app.get_sizing_y(), 12.);
                        app.set_sizing_content(-1.);
                    }
                    75 => { assert_eq!(app.get_sizing_height(), 520.); assert_eq!(app.get_sizing_y(), 40.); }
                    _ => {
                        app.set_effects_enabled(false);
                        capture(&app, &directory.join("fluent-gallery.png"))?;
                        println!("Gallerian GUI-koe hyväksytty: liukusäätimen näppäimet ja hiiren liike, valintaruutu, tekstisyöttö, kymmenen ulkoasua ja tehosteportti.");
                        if let Some(timer) = weak_timer.upgrade() { timer.stop(); }
                        slint::quit_event_loop()?;
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                eprintln!("Gallerian GUI-koe epäonnistui: {error}");
                if smoke { std::process::exit(1); }
            }
            phase.set(current + 1);
        });
    }
    slint::run_event_loop()?;
    Ok(())
}
