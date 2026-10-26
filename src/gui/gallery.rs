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
    app.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    app.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}

fn pointer(app: &GalleryWindow, pressed: bool, x: f32, y: f32) {
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
                    12..=35 => {
                        if current == 12 { assert_eq!(app.get_internal_scroll_offset(), -40., "Laajentuminen säilyttää myös nollasta poikkeavan vierityskohdan"); }
                        let index = (current - 12) / 2;
                        if current % 2 == 0 {
                            app.set_appearance_index(index / 2);
                            app.set_effects_enabled(index % 2 != 0);
                        } else {
                            capture(&app, &directory.join(format!("teema-{}-tehosteet-{}.png", index / 2, index % 2)))?;
                        }
                    }
                    36 => {
                        app.invoke_show_slider();
                        let p = app.get_slider_position();
                        let x = p.x + app.get_slider_width() * 0.35;
                        let y = p.y + app.get_slider_height() / 2.;
                        pointer(&app, true, x, y); pointer(&app, false, x, y);
                        assert!((app.get_slider_value() - 35.).abs() <= 5.);
                        assert_eq!(app.get_slider_value() % 5., 0.);
                        app.invoke_show_additional(); app.invoke_focus_switch();
                    }
                    37 => { key(&app, " "); assert!(app.get_sample_switch()); app.invoke_focus_number(); }
                    38 => { key(&app, Key::UpArrow); assert_eq!(app.get_sample_number(), 10); }
                    39 => { key(&app, Key::DownArrow); assert_eq!(app.get_sample_number(), 5); capture(&app, &directory.join("lisakomponentit.png"))?; app.invoke_focus_menu(); key(&app, Key::Return); }
                    40 => { assert!(app.get_popup_open(), "Valikkopainikkeen Enter avaa popupin"); capture(&app, &directory.join("valikko.png"))?; key(&app, Key::Escape); }
                    41 => { assert!(!app.get_popup_open(), "Esc sulkee popupin ja päivittää laajentumistilan"); let scale = app.window().scale_factor(); app.window().set_size(slint::PhysicalSize::new((900. * scale) as u32, (600. * scale) as u32)); }
                    42 => { app.invoke_reset_scroll(); }
                    43 => {
                        let size = app.window().size();
                        let scale = app.window().scale_factor();
                        assert!((size.width as f32 / scale - 900.).abs() <= 2. && (size.height as f32 / scale - 600.).abs() <= 2., "Gallerian todellinen koko: {size:?}, skaala {scale}");
                        capture(&app, &directory.join("pieni-900x600.png"))?;
                    }
                    _ => {
                        app.set_effects_enabled(false);
                        capture(&app, &directory.join("fluent-gallery.png"))?;
                        println!("Gallerian GUI-koe hyväksytty: liukusäätimen näppäimet ja hiiren liike, valintaruutu, tekstisyöttö, kuusi ulkoasua ja tehosteportti.");
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
