//! Oikean GUI-sovittimen laitteeton työnkulkukoe erillisessä esimerkkikopiossa.
use super::*;

pub fn key(app: &AppWindow, text: impl Into<SharedString>) {
    let text = text.into();
    app.window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
    app.window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
}

pub fn open_dialog(app: &AppWindow) {
    use slint::platform::{Key, WindowEvent};
    app.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    key(app, "n");
    app.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    assert!(app.get_dialog_open(), "Ctrl+N avaa uuden kilpailun");
}

pub fn fill_dialog(app: &AppWindow) {
    app.set_new_location_visible(true);
    key(app, "Syysajo 2026");
    key(app, slint::platform::Key::Tab);
    key(app, "C:/Kilpailut/Syysajo");
    assert_eq!(app.get_new_name(), "Syysajo 2026");
    assert_eq!(app.get_new_path(), "C:/Kilpailut/Syysajo");
}

pub fn run(app: &AppWindow, state: &SharedState) -> Result<(), Box<dyn std::error::Error>> {
    let (path, original) = {
        let state = state.borrow();
        let ws = state
            .workspace
            .as_ref()
            .ok_or("Esimerkkikilpailu puuttuu")?;
        (
            ws.path().to_owned(),
            ws.project().ok_or("Muokattava esimerkki puuttuu")?.clone(),
        )
    };
    // Nimi riittää. Tämä koe käyttää vain --demo-ajon omaa asetuskansiota.
    app.invoke_create_project("".into(), "Nimellä luotu kilpailu".into());
    {
        let state = state.borrow();
        let created = state.workspace.as_ref().unwrap();
        let preferences = state.preferences.as_ref().unwrap();
        assert_eq!(created.name(), "Nimellä luotu kilpailu");
        assert!(created.path().starts_with(preferences.competition_root()?));
        let reopened =
            PreferencesStore::open_at(preferences.path(), &preferences.competition_root()?)?;
        assert_eq!(
            reopened.get().last_competition.as_deref(),
            Some(created.path())
        );
        assert_eq!(
            Workspace::open(reopened.get().last_competition.as_ref().unwrap())?.name(),
            "Nimellä luotu kilpailu"
        );
    }
    open_workspace(&mut state.borrow_mut(), app, &path, true)?;
    // Päivitykset ovat sivupalkin painike, eivät oma kilpailusivunsa.
    app.invoke_navigate(7);
    assert_eq!(
        app.get_page(),
        0,
        "Päivityspainike ei muuta kilpailunäkymää"
    );
    app.invoke_navigate(6);
    assert_eq!(app.get_page(), 6, "Asetukset kuuluvat kilpailunhallintaan");
    app.invoke_navigate(1);
    app.invoke_select_row(0);
    // Näkymän vaihto hyväksyy myös vielä kohdistetussa kentässä olevan luonnoksen.
    app.invoke_field_draft(1, "Käytettävyyskoe".into());
    app.invoke_navigate(0);
    assert_eq!(
        state
            .borrow()
            .workspace
            .as_ref()
            .unwrap()
            .project()
            .unwrap()
            .kilpailijat[0]
            .kuljettaja,
        "Käytettävyyskoe"
    );
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0].kuljettaja,
        "Käytettävyyskoe"
    );
    app.invoke_navigate(1);
    app.invoke_select_row(0);
    assert!(!app.invoke_field_commit(4, "virheellinen".into()));
    assert!(app.get_form_blocked());
    assert!(app.invoke_field_commit(5, "Testiauto".into()));
    assert!(
        app.get_form_blocked(),
        "Toisen kentän onnistuminen ei saa poistaa aiempaa virhettä"
    );
    app.invoke_navigate(3);
    assert_eq!(app.get_page(), 1);
    assert!(app.invoke_field_commit(4, "123456".into()));
    assert!(!app.get_form_blocked());
    app.invoke_add_record();
    app.invoke_field_draft(0, "8".into());
    app.invoke_field_draft(1, "Uusi kilpailija".into());
    app.invoke_field_draft(4, "654321".into());
    app.invoke_add_commit();
    assert!(!app.get_new_record());
    assert_eq!(ProjectStore::open(&path)?.project().kilpailijat.len(), 2);
    app.invoke_navigate(4);
    app.invoke_select_row(0);
    assert!(app.invoke_field_commit(0, "H".into()));
    assert!(
        ProjectStore::open(&path)?
            .project()
            .kilpailijat
            .iter()
            .all(|d| d.luokka == "H")
    );
    app.invoke_navigate(5);
    app.invoke_select_row(0);
    app.invoke_select_detail(1);
    assert!(app.invoke_field_commit(3, "275".into()));
    app.invoke_navigate(3);
    assert!(!app.get_calculation_notes().contains("varmistamatta"));
    let results = state.borrow().workspace.as_ref().unwrap().results()?;
    assert_eq!(results.competitors[0].total_penalties, 600);
    // Tallennusvirhe ei hävitä jo luettua kehystä; uudelleenyritys käyttää samaa kilpailua.
    let mut backup_name = path.as_os_str().to_owned();
    backup_name.push(".bak");
    let backup = PathBuf::from(backup_name);
    std::fs::remove_file(&backup)?;
    std::fs::create_dir(&backup)?;
    let frame = emit::parse_frame(include_bytes!("../../fixtures/synthetic/emit-frame.raw"))?;
    assert!(handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(frame),
            read_clock: ClockTime::new(12, 11, 0)?,
        }
    ));
    assert_eq!(app.get_unsaved_count(), 1);
    std::fs::remove_dir(&backup)?;
    app.invoke_retry_readouts();
    assert_eq!(app.get_unsaved_count(), 0);
    assert_eq!(ProjectStore::open(&path)?.project().leimaluvut.len(), 2);
    assert!(app.get_read_notice_visible());
    assert!(app.get_read_notice().contains("Emit 123456 luettu"));
    app.invoke_navigate(2);
    let history_key = app.get_rows().row_data(0).unwrap().key;
    app.invoke_select_row(history_key);
    assert_eq!(app.get_detail_rows().row_count(), 5);
    let repeated = emit::parse_frame(include_bytes!("../../fixtures/synthetic/emit-frame.raw"))?;
    assert!(!handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(repeated),
            read_clock: ClockTime::new(12, 11, 1)?,
        },
    ));
    assert_eq!(ProjectStore::open(&path)?.project().leimaluvut.len(), 3);
    assert_eq!(app.get_rows().row_count(), 1);
    assert_eq!(app.get_rows().row_data(0).unwrap().key, history_key);
    assert_eq!(app.get_selected_key(), history_key);
    assert_eq!(
        app.get_rows()
            .row_data(0)
            .unwrap()
            .cells
            .row_data(5)
            .unwrap(),
        "3"
    );
    app.invoke_navigate(5);
    app.invoke_add_station();
    let rule_options = || {
        app.get_fields()
            .iter()
            .find(|f| f.key == 4)
            .unwrap()
            .options
            .row_count()
    };
    assert_eq!(rule_options(), 5);
    app.invoke_field_draft(0, "ULA".into());
    assert!(app.invoke_field_commit(0, "ULA".into()));
    assert_eq!(rule_options(), 8);
    app.invoke_field_draft(1, "Testiasema".into());
    app.invoke_field_draft(2, "99".into());
    app.invoke_field_draft(4, "5".into());
    app.invoke_add_commit();
    assert!(!app.get_new_record());
    assert_eq!(app.get_detail_key(), 3);
    assert_eq!(
        ProjectStore::open(&path)?.project().radat[0]
            .asemat
            .last()
            .unwrap()
            .tyyppi,
        "M"
    );
    assert!(!app.invoke_field_commit(0, "AT".into()));
    assert!(app.invoke_field_commit(4, "0".into()));
    assert!(app.invoke_field_commit(0, "AT".into()));
    assert_eq!(rule_options(), 5);
    // Palautetaan vain tämän kokeen itse luoma synteettinen kopio kuvausta varten.
    ProjectStore::open(&path)?.update(|project| {
        *project = original.clone();
        Ok(())
    })?;
    {
        let mut state = state.borrow_mut();
        state.selection = Selection::None;
        state.draft.clear();
        state.edit_draft.clear();
        state.invalid_fields.clear();
        state.invalid_field = false;
        open_workspace(&mut state, app, &path, true)?;
    }
    assert!(
        !app.get_read_notice_visible(),
        "Kilpailun vaihto tyhjentää kuittauksen"
    );
    show_read_notice(&mut state.borrow_mut(), app, "✓ Kortti luettu".into());
    assert!(app.get_read_notice_visible());
    let weak = app.as_weak();
    Timer::single_shot(Duration::from_millis(3200), move || {
        if let Some(app) = weak.upgrade() {
            assert!(
                !app.get_read_notice_visible(),
                "Lukukuittaus poistuu itsestään"
            );
            println!("GUI-lukukuittauksen ajastinkoe hyväksytty.");
        }
    });
    println!(
        "GUI-työnkulkukoe hyväksytty: valinnat, luonnokset, automaattitallennus, virheestä palautuminen ja yhteinen laskenta."
    );
    Ok(())
}
