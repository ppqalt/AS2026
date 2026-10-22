//! Oikean GUI-sovittimen laitteeton työnkulkukoe erillisessä esimerkkikopiossa.
use super::*;

// Aiemmat työnkulkukokeet jatkavat asemakorjausten testaamista sulkemalla
// automaattisen syöttöikkunan. Uusi tulostuskoe käyttää oikeaa tapahtumaa.
fn handle_event(state: &mut State, app: &AppWindow, event: ReaderEvent) -> bool {
    app.set_result_entry_open(false);
    state.entry_number = None;
    let result = super::handle_event(state, app, event);
    app.set_result_entry_open(false);
    state.entry_number = None;
    result
}
fn print_current(app: &AppWindow) {
    app.invoke_approve_results();
    assert!(app.get_result_entry_open(), "{}", app.get_status_text());
    app.invoke_result_entry_close(true);
    assert!(
        !app.get_result_entry_open(),
        "{}",
        app.get_result_entry_error()
    );
}

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

pub fn run(app: &AppWindow, state: &SharedState) -> Result<PathBuf, Box<dyn std::error::Error>> {
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
    combined_classes(app, state, &path, &original)?;
    beginners_cup(app, state, &path, &original)?;
    automatic_preparation(app, state, &path, &original)?;
    result_entry_and_printing(app, state, &path, &original)?;
    // Nimi riittää. Tämä koe käyttää vain --demo-ajon omaa asetuskansiota.
    app.invoke_create_project("".into(), "Nimellä luotu kilpailu".into());
    {
        let state = state.borrow();
        let created = state.workspace.as_ref().unwrap();
        let preferences = state.preferences.as_ref().unwrap();
        assert_eq!(created.name(), "Nimellä luotu kilpailu");
        // Workspace käyttää kanonista polkua; Windowsissa se voi sisältää
        // \\?\-etuliitteen, jota asetusten vielä luomattomalla juurella ei ollut.
        let root = std::fs::canonicalize(preferences.competition_root()?)?;
        assert!(created.path().starts_with(&root));
        assert_eq!(
            created.path().parent().and_then(Path::parent),
            Some(root.as_path()),
            "Kilpailu syntyy omaan kansioonsa valitun tallennuspaikan alle"
        );
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
    let invalid_card = app
        .get_fields()
        .iter()
        .find(|field| field.key == 4)
        .unwrap();
    assert!(
        !invalid_card.error.is_empty(),
        "Tarkka virhe näkyy kentän yhteydessä"
    );
    assert_eq!(
        invalid_card.value, "virheellinen",
        "Virheellinen syöte säilyy korjattavana"
    );
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0].emit,
        original.kilpailijat[0].emit
    );
    assert!(app.invoke_field_commit(5, "Testiauto".into()));
    assert!(
        app.get_form_blocked(),
        "Toisen kentän onnistuminen ei saa poistaa aiempaa virhettä"
    );
    app.invoke_navigate(3);
    assert_eq!(app.get_page(), 1);
    assert!(app.invoke_field_commit(4, "123456".into()));
    assert!(
        app.get_fields()
            .iter()
            .find(|field| field.key == 4)
            .unwrap()
            .error
            .is_empty()
    );
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
    assert!(app.invoke_field_commit(3, "00:04:35".into()));
    app.invoke_navigate(3);
    assert!(!app.get_calculation_notes().contains("varmistamatta"));
    let results = state.borrow().workspace.as_ref().unwrap().results()?;
    // Käsimaali odottaa toimitsijaa; tunnetun AT:n muutos poistaa 45 pisteen ennakon.
    assert_eq!(results.competitors[0].total_penalties, 600);
    assert_eq!(results.competitors[0].status, "ILM");
    assert!(!app.get_result_approvable());
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
    assert_eq!(
        app.get_page(),
        3,
        "tunnettu Emit avaa Tulokset myös Emit-sivulta"
    );
    assert_eq!(selected_number(&state.borrow())?, 7);
    app.invoke_navigate(2);
    app.invoke_select_row(history_key);
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
    // Käsikorjaus ja tilan/lisenssien tallennus kulkevat oikeiden callbackien kautta.
    app.invoke_navigate(5);
    app.invoke_select_row(0);
    app.invoke_select_detail(1);
    assert!(app.invoke_field_commit(2, "0".into()));
    app.invoke_navigate(6);
    assert!(app.invoke_field_commit(2, "0".into()));
    app.invoke_navigate(3);
    app.invoke_select_row(0);
    let before_review = ProjectStore::open(&path)?.project().kilpailijat[0]
        .tallennettu_tulos
        .clone();
    app.invoke_review_results();
    assert!(app.get_confirm_open());
    app.invoke_confirm_action(false);
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0].tallennettu_tulos,
        before_review
    );
    app.invoke_review_results();
    app.invoke_confirm_action(true);
    assert!(app.get_result_stored());
    app.invoke_select_detail(1);
    let before_points = state
        .borrow()
        .workspace
        .as_ref()
        .unwrap()
        .results()?
        .competitors[0]
        .total_penalties;
    assert!(app.invoke_field_commit(100, "12:04:35".into()));
    assert!(app.get_clock_pending());
    assert_eq!(
        state
            .borrow()
            .workspace
            .as_ref()
            .unwrap()
            .results()?
            .competitors[0]
            .total_penalties,
        before_points
    );
    assert!(app.invoke_field_commit(101, "-75".into()));
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .rivit[1]
            .signed_time_penalty,
        -75
    );
    app.invoke_examine_results();
    assert!(!app.get_clock_pending());
    assert_eq!(
        state
            .borrow()
            .workspace
            .as_ref()
            .unwrap()
            .results()?
            .competitors[0]
            .results[1]
            .clock,
        [12, 4, 35]
    );
    assert!(
        !app.get_result_approvable(),
        "käsimaalin puuttuva aika estää hyväksymisen"
    );
    let finish_index = state
        .borrow()
        .workspace
        .as_ref()
        .unwrap()
        .results()?
        .competitors[0]
        .results
        .iter()
        .position(|r| r.station_type == "M")
        .unwrap();
    app.invoke_select_detail(finish_index as i32);
    assert!(app.invoke_field_commit(100, "12:15:00".into()));
    app.invoke_examine_results();
    assert!(
        app.get_result_approvable(),
        "tulos ei hyväksyttävissä: {:?}",
        state
            .borrow()
            .workspace
            .as_ref()
            .unwrap()
            .result_snapshot(7)?
    );
    app.invoke_select_detail(1);
    assert!(app.invoke_field_commit(101, "-75".into()));
    print_current(app);
    assert!(
        ProjectStore::open(&path)?.project().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .rivit[1]
            .signed_time_penalty,
        -75
    );

    // Uusi lukema tallentuu kesken muokkauksen mutta ei katkaise sitä.
    app.invoke_navigate(1);
    app.invoke_select_row(0);
    app.invoke_field_draft(1, "Keskeneräinen nimi".into());
    let history_count = ProjectStore::open(&path)?.project().leimaluvut.len();
    let repeated = emit::parse_frame(include_bytes!("../../fixtures/synthetic/emit-frame.raw"))?;
    assert!(!handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(repeated),
            read_clock: ClockTime::new(12, 12, 0)?
        }
    ));
    assert_eq!(app.get_page(), 1);
    assert_eq!(
        ProjectStore::open(&path)?.project().leimaluvut.len(),
        history_count + 1
    );
    assert!(state.borrow().pending_result.is_some());
    assert!(app.invoke_field_commit(1, "Keskeneräinen nimi".into()));
    follow_pending_result(&mut state.borrow_mut(), app)?;
    app.invoke_result_entry_close(false);
    assert_eq!(app.get_page(), 3);
    assert_eq!(selected_number(&state.borrow())?, 7);
    assert!(
        ProjectStore::open(&path)?.project().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .rivit[1]
            .signed_time_penalty,
        -75
    );
    // Tuntematon kortti ei siirry eikä kohdistu valittuun kilpailijaan.
    app.invoke_navigate(1);
    let unknown = synthetic_frame(999999, &[(0, 120), (31, 395), (250, 450)]);
    assert!(!handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(unknown),
            read_clock: ClockTime::new(12, 13, 0)?
        }
    ));
    assert_eq!(app.get_page(), 1);
    assert!(state.borrow().pending_result.is_none());
    assert!(app.get_status_text().contains("ei löydy kilpailijaa"));
    app.invoke_select_row(0);
    assert!(app.invoke_field_commit(11, "12345X".into()));
    assert!(app.invoke_field_commit(12, "67890".into()));
    assert!(app.invoke_field_commit(13, "HYL".into()));
    assert!(app.invoke_field_commit(14, "Toimitsijan päätös".into()));
    app.invoke_navigate(3);
    assert_eq!(
        state
            .borrow()
            .workspace
            .as_ref()
            .unwrap()
            .results()?
            .competitors[0]
            .status,
        "HYL"
    );
    let saved = ProjectStore::open(&path)?;
    assert_eq!(
        saved.project().kilpailijat[0].kuljettajan_lisenssi,
        "12345X"
    );
    assert_eq!(
        saved.project().kilpailijat[0].tila.as_ref().unwrap().syy,
        "Toimitsijan päätös"
    );

    // Arvonnan esikatselu, peruutus ja hyväksyntä erillisessä lukemattomassa kopiossa.
    let draw_dir = path.parent().unwrap().join("arvontakoe");
    std::fs::create_dir_all(&draw_dir)?;
    let draw_path = draw_dir.join(native::FILE_NAME);
    let mut draw_project = original.clone();
    draw_project.leimaluvut.clear();
    let mut second = draw_project.kilpailijat[0].clone();
    second.numero += 1;
    second.emit += 1;
    draw_project.kilpailijat.push(second);
    ProjectStore::create(&draw_path, draw_project.clone())?;
    open_workspace(&mut state.borrow_mut(), app, &draw_path, true)?;
    app.invoke_navigate(1);
    app.invoke_draw_starts();
    assert!(app.get_confirm_open());
    assert!(app.get_confirm_text().contains("Nro"));
    app.invoke_confirm_action(false);
    assert_eq!(ProjectStore::open(&draw_path)?.project(), &draw_project);
    app.invoke_draw_starts();
    app.invoke_confirm_action(true);
    assert!(!app.get_confirm_open());
    let drawn = ProjectStore::open(&draw_path)?;
    assert_ne!(
        drawn.project().kilpailijat[0].lahtoaika,
        drawn.project().kilpailijat[1].lahtoaika
    );
    assert!(drawn.project().leimaluvut.is_empty());
    app.invoke_navigate(4);
    app.invoke_select_row(0);
    assert!(app.invoke_field_commit(9, "0".into()));
    let not_drawn = ProjectStore::open(&draw_path)?.project().clone();
    app.invoke_draw_starts();
    assert!(!app.get_confirm_open());
    assert_eq!(ProjectStore::open(&draw_path)?.project(), &not_drawn);

    // Excel-esikatselun peruutus ei kirjoita; hyväksyntä luo uuden kilpailun.
    let excel =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/excel-import/synteettinen.xlsx");
    let preview = as2026::excel_import::read(&excel)?;
    let import_directory = path.parent().unwrap().join("tuontikoe");
    offer_excel_import(
        &mut state.borrow_mut(),
        app,
        &excel,
        &preview,
        Some(import_directory.clone()),
    )?;
    assert!(app.get_confirm_open());
    assert!(app.get_confirm_acceptable());
    assert!(!import_directory.join(native::FILE_NAME).exists());
    app.invoke_confirm_action(false);
    assert!(!import_directory.join(native::FILE_NAME).exists());
    offer_excel_import(
        &mut state.borrow_mut(),
        app,
        &excel,
        &preview,
        Some(import_directory.clone()),
    )?;
    app.invoke_confirm_action(true);
    let imported = ProjectStore::open(&import_directory.join(native::FILE_NAME))?;
    assert_eq!(imported.project().kilpailijat.len(), 2);
    assert_eq!(imported.project().radat.len(), 2);
    assert!(
        imported
            .project()
            .luokat
            .iter()
            .all(|c| !c.osallistuu_lahtoarvontaan)
    );
    assert!(
        !imported.project().kilpailijat[0]
            .kuljettajan_seura
            .is_empty()
    );
    finish_imported_race(app, state, &import_directory)?;
    let imported_before_retry = std::fs::read(import_directory.join(native::FILE_NAME))?;
    offer_excel_import(
        &mut state.borrow_mut(),
        app,
        &excel,
        &preview,
        Some(import_directory.clone()),
    )?;
    app.invoke_confirm_action(true);
    assert!(
        !app.get_import_error().is_empty(),
        "Tuontivirhe näkyy tuonti-ikkunassa"
    );
    assert!(
        app.get_confirm_open() && app.get_import_open(),
        "Tallennusvirhe säilyttää esikatselun"
    );
    assert_eq!(
        std::fs::read(import_directory.join(native::FILE_NAME))?,
        imported_before_retry
    );
    app.invoke_confirm_action(false);
    // Valmistelutuonti ja keskeytyspäätös ilman korttia, lähtöä tai maalia.
    let mut draft_preview = preview.clone();
    for driver in &mut draft_preview.kilpailijat {
        driver.tiedot.emit = 0;
        driver.tiedot.lahtoaika.clear();
    }
    draft_preview.kilpailijat[0].tiedot.luokka = "SM-A".into();
    let tracks_directory = path.parent().unwrap().join("pelkat-radat");
    offer_excel_import(
        &mut state.borrow_mut(),
        app,
        &excel,
        &draft_preview,
        Some(tracks_directory.clone()),
    )?;
    app.invoke_import_selection(1);
    assert!(app.get_confirm_acceptable());
    app.invoke_confirm_action(true);
    let tracks = ProjectStore::open(&tracks_directory.join(native::FILE_NAME))?;
    assert!(tracks.project().kilpailijat.is_empty());
    assert_eq!(tracks.project().radat.len(), 2);
    drop(tracks);
    let drivers_directory = path.parent().unwrap().join("pelkat-kilpailijat");
    offer_excel_import(
        &mut state.borrow_mut(),
        app,
        &excel,
        &draft_preview,
        Some(drivers_directory.clone()),
    )?;
    app.invoke_import_selection(2);
    assert!(app.get_confirm_acceptable());
    assert_eq!(app.get_import_tab(), 0);
    app.invoke_import_selection(0);
    assert!(app.get_confirm_text().contains("2 rataa"));
    app.invoke_import_selection(2);
    app.invoke_confirm_action(true);
    let drivers = ProjectStore::open(&drivers_directory.join(native::FILE_NAME))?;
    assert_eq!(drivers.project().kilpailijat.len(), 2);
    assert!(drivers.project().radat.is_empty());
    assert!(drivers.project().luokat.iter().all(|class| class.rata == 0));
    drop(drivers);
    let draft_directory = path.parent().unwrap().join("valmistelukoe");
    offer_excel_import(
        &mut state.borrow_mut(),
        app,
        &excel,
        &draft_preview,
        Some(draft_directory.clone()),
    )?;
    assert!(
        app.get_confirm_acceptable(),
        "Ratavalinnat voi täydentää tuonnin jälkeen"
    );
    assert_eq!(app.get_import_rows().row_count(), 2);
    app.invoke_import_class_route("SM-A".into(), "C".into());
    assert!(app.get_confirm_acceptable());
    app.set_import_name("Valmistelukoe".into());
    app.invoke_confirm_action(true);
    assert!(!app.get_status_error(), "{}", app.get_status_text());
    app.invoke_navigate(1);
    app.invoke_select_row(0);
    assert!(
        app.get_fields()
            .iter()
            .find(|field| field.key == 4)
            .unwrap()
            .value
            .is_empty()
    );
    assert!(app.invoke_field_commit(13, "KESK".into()));
    assert!(app.invoke_field_commit(14, "Auto rikkoutui ennen lähtöä".into()));
    app.invoke_navigate(3);
    app.invoke_select_row(0);
    assert!(app.get_result_approvable());
    print_current(app);
    let interrupted = ProjectStore::open(&draft_directory.join(native::FILE_NAME))?;
    assert!(interrupted.project().kilpailijat[0].has_confirmed_interruption());
    assert!(
        interrupted.project().kilpailijat[0]
            .tallennettu_tulos
            .is_none()
    );
    assert!(interrupted.project().leimaluvut.is_empty());
    drop(interrupted);
    app.invoke_reload();
    assert_eq!(app.get_result_overview().total.as_str(), "-");
    println!(
        "GUI-valmistelukoe hyväksytty: ratavalinta → tuonti ilman Emitiä ja lähtöä → perusteltu KESK → uudelleenavaus ilman aikoja tai pisteitä."
    );
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
        "GUI-työnkulkukoe hyväksytty: Emit → Tulokset, keskeneräisen työn suojaus, käsikellot, pistekorjaus, hyväksyminen, tilat, Excel-tuonti ja lähtöarvonta."
    );
    Ok(import_directory.join(native::FILE_NAME))
}

fn beginners_cup(
    app: &AppWindow,
    state: &SharedState,
    original_path: &Path,
    original: &Project,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = original_path.parent().unwrap().join("beginners-cup");
    let mut fixture = original.clone();
    let mut second = fixture.kilpailijat[0].clone();
    second.numero = 9;
    second.emit = 654321;
    second.kuljettaja = "BC-koekilpailija".into();
    fixture.kilpailijat.push(second);
    let mut read = fixture.leimaluvut[0].clone();
    read.emit = 654321;
    fixture.leimaluvut.push(read);
    let ws = Workspace::create_imported(&directory, fixture.clone())?;
    let path = ws.path().to_owned();
    open_workspace(&mut state.borrow_mut(), app, &path, true)?;
    app.invoke_navigate(4);
    app.invoke_configure_bc();
    assert!(app.get_bc_open());
    app.invoke_bc_station_toggled(1, false);
    app.invoke_bc_participant_toggled(1, true);
    app.invoke_bc_close(true);
    assert!(!app.get_bc_open(), "{}", app.get_bc_error());
    assert!(app.get_bc_configured());
    let stored = ProjectStore::open(&path)?;
    assert_eq!(
        stored
            .project()
            .beginners_cup
            .as_ref()
            .unwrap()
            .mukana_asemat,
        [1, 3, 4]
    );
    assert_eq!(stored.project().kilpailijat, fixture.kilpailijat);
    assert_eq!(stored.project().leimaluvut, fixture.leimaluvut);
    app.invoke_navigate(1);
    app.invoke_select_row(1);
    assert!(app.get_fields().iter().any(|field| field.key == 17));
    assert!(app.invoke_field_commit(17, "0".into()));
    assert!(!ProjectStore::open(&path)?.project().bc_osallistujat[0].osallistuu);
    assert!(app.invoke_field_commit(17, "1".into()));
    assert!(ProjectStore::open(&path)?.project().bc_osallistujat[0].osallistuu);
    app.invoke_navigate(3);
    app.invoke_result_scope(true);
    assert!(app.get_bc_results());
    assert_eq!(selected_number(&state.borrow())?, 9);
    app.invoke_edit_result_competitor();
    assert!(
        app.get_fields()
            .iter()
            .all(|f| matches!(f.key, 8 | 9 | 10 | 13 | 14))
    );
    assert!(app.invoke_field_commit(10, "77".into()));
    let saved = ProjectStore::open(&path)?;
    assert_eq!(
        saved.project().bc_osallistujat[0]
            .lisavirheet
            .muut_virhepisteet,
        77
    );
    assert_eq!(
        saved.project().kilpailijat[0].lisavirheet,
        original.kilpailijat[0].lisavirheet
    );
    assert_eq!(
        saved.project().kilpailijat[1].lisavirheet,
        fixture.kilpailijat[1].lisavirheet
    );
    let archive = state.borrow().workspace.as_ref().unwrap().results()?;
    let finish = archive.competitors[0]
        .results
        .iter()
        .position(|r| r.station_type == "M")
        .unwrap();
    app.invoke_select_detail(finish as i32);
    assert!(app.invoke_field_commit(100, "12:11:00".into()));
    app.invoke_examine_results();
    assert!(app.get_result_approvable(), "{}", app.get_status_text());
    print_current(app);
    let saved = ProjectStore::open(&path)?;
    assert!(
        saved.project().bc_osallistujat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    assert_eq!(
        saved.project().kilpailijat[0].tallennettu_tulos,
        original.kilpailijat[0].tallennettu_tulos
    );
    app.invoke_result_scope(false);
    assert!(!app.get_bc_results());
    assert_eq!(selected_number(&state.borrow())?, 9);
    assert_eq!(
        ProjectStore::open(&path)?.project().leimaluvut,
        fixture.leimaluvut
    );
    open_workspace(&mut state.borrow_mut(), app, original_path, true)?;
    println!("GUI-BC-koe: asemat, osallistuja, erilliset virheet, käsimaali ja hyväksyntä.");
    Ok(())
}

fn combined_classes(
    app: &AppWindow,
    state: &SharedState,
    original_path: &Path,
    original: &Project,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = original_path.parent().unwrap().join("yhdistetty-luokka");
    let mut project = original.clone();
    let mut second = project.luokat[0].clone();
    second.tunnus = "B".into();
    second.nimi = "Toinen luokka".into();
    project.luokat.push(second);
    let ws = Workspace::create_imported(&directory, project)?;
    let path = ws.path().to_owned();
    open_workspace(&mut state.borrow_mut(), app, &path, true)?;
    app.invoke_navigate(4);
    app.invoke_add_combined_class();
    assert!(app.get_combined_open());
    app.invoke_combined_class_toggled(0, true);
    app.invoke_combined_close(true);
    assert!(
        app.get_combined_open(),
        "Puuttuva nimi jättää lomakkeen auki"
    );
    assert!(!app.get_combined_error().is_empty());
    assert!(
        ProjectStore::open(&path)?
            .project()
            .yhdistetyt_luokat
            .is_empty()
    );
    app.set_combined_name("Yhteistulos".into());
    app.invoke_combined_class_toggled(1, true);
    app.invoke_combined_close(true);
    assert!(!app.get_combined_open(), "{}", app.get_combined_error());
    let saved = ProjectStore::open(&path)?;
    assert_eq!(saved.project().yhdistetyt_luokat[0].luokat, ["A", "B"]);
    assert_eq!(saved.project().kilpailijat, original.kilpailijat);
    assert_eq!(saved.project().leimaluvut, original.leimaluvut);
    app.invoke_filter_changed("Yhteistulos".into());
    assert_eq!(app.get_rows().row_count(), 1);
    let key = app.get_rows().row_data(0).unwrap().key;
    app.invoke_select_row(key);
    assert!(app.get_combined_open());
    assert!(!app.get_combined_creating());
    app.set_combined_name("Peruttu nimi".into());
    app.invoke_combined_class_toggled(0, false);
    app.invoke_combined_close(false);
    assert_eq!(ProjectStore::open(&path)?.project(), saved.project());
    app.invoke_select_row(key);
    assert_eq!(app.get_combined_name(), "Yhteistulos");
    assert!(app.get_combined_classes().row_data(0).unwrap().selected);
    app.set_combined_name("Uusi yhteistulos".into());
    app.invoke_combined_class_toggled(0, false);
    app.invoke_combined_close(true);
    let changed = ProjectStore::open(&path)?;
    assert_eq!(
        changed.project().yhdistetyt_luokat[0].nimi,
        "Uusi yhteistulos"
    );
    assert_eq!(changed.project().yhdistetyt_luokat[0].luokat, ["B"]);
    open_workspace(&mut state.borrow_mut(), app, original_path, true)?;
    println!("GUI-yhdistetyn luokan koe: luonti, validointi, haku, peruutus ja muokkaus.");
    Ok(())
}

fn automatic_preparation(
    app: &AppWindow,
    state: &SharedState,
    original_path: &Path,
    original: &Project,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = original_path
        .parent()
        .unwrap()
        .join("automaattinen-valmistelu");
    let mut project = original.clone();
    project.kilpailijat.clear();
    project.leimaluvut.clear();
    let mut second = project.radat[0].clone();
    second.numero = 2;
    second.nimi = "Toinen rata".into();
    let original_stations = second.asemat.len();
    project.radat.push(second);
    let workspace = Workspace::create_imported(&directory, project)?;
    let path = workspace.path().to_owned();
    open_workspace(&mut state.borrow_mut(), app, &path, true)?;
    app.invoke_navigate(5);
    app.invoke_select_row(1);
    app.invoke_add_station();
    app.invoke_field_draft(1, "Luonnoksen asema".into());
    app.invoke_navigate(4);
    assert_eq!(
        app.get_page(),
        4,
        "Vajavainen uusi asema ei estä sivun vaihtamista"
    );
    assert!(!app.get_status_error());
    app.invoke_navigate(5);
    assert!(app.get_new_record());
    assert_eq!(app.get_selected_key(), 1, "Luonnos palaa oikealle radalle");
    assert_eq!(
        app.get_fields()
            .iter()
            .find(|field| field.key == 1)
            .unwrap()
            .value
            .as_str(),
        "Luonnoksen asema"
    );
    app.invoke_select_detail(1);
    assert!(!app.get_new_record());
    assert_eq!(
        app.get_fields()
            .iter()
            .find(|field| field.key == 1)
            .unwrap()
            .value
            .as_str(),
        original.radat[0].asemat[1].nimi,
        "Uuden aseman luonnos ei saa peittää olemassa olevan aseman nimeä"
    );
    app.invoke_navigate(4);
    app.invoke_navigate(5);
    assert!(
        app.get_new_record(),
        "Keskeneräinen asema säilyy luonnoksena"
    );
    assert!(app.invoke_field_commit(2, "77".into()));
    assert!(
        !app.get_new_record(),
        "Valmis asema tallentuu ilman Lisää-painiketta"
    );
    let saved = ProjectStore::open(&path)?;
    assert_eq!(saved.project().radat[0].asemat.len(), original_stations);
    assert_eq!(saved.project().radat[1].asemat.len(), original_stations + 1);
    assert_eq!(
        saved.project().radat[1]
            .asemat
            .iter()
            .filter(|s| s.emit_koodi == 77)
            .count(),
        1
    );
    assert!(app.invoke_field_commit(1, "Tallennettu asema".into()));
    app.invoke_reload();
    assert_eq!(
        ProjectStore::open(&path)?.project().radat[1]
            .asemat
            .iter()
            .filter(|s| s.emit_koodi == 77)
            .count(),
        1,
        "Uudelleenavaus ei monista asemaa"
    );
    app.invoke_navigate(4);
    app.invoke_select_row(0);
    assert!(
        app.get_fields()
            .iter()
            .find(|field| field.key == 4)
            .unwrap()
            .duration
    );
    assert!(app.invoke_field_commit(4, "15:23:17".into()));
    assert_eq!(
        ProjectStore::open(&path)?.project().luokat[0].lahtoaika,
        "15:23:17"
    );
    assert!(!app.invoke_field_commit(4, "24:00:00".into()));
    assert_eq!(
        ProjectStore::open(&path)?.project().luokat[0].lahtoaika,
        "15:23:17"
    );
    assert!(app.invoke_field_commit(4, "15:23:17".into()));
    app.invoke_navigate(1);
    app.invoke_add_record();
    assert!(app.invoke_field_commit(1, "Automaattikuljettaja".into()));
    assert!(!app.get_new_record());
    assert_eq!(ProjectStore::open(&path)?.project().kilpailijat.len(), 1);
    // Pelkkä GUI-tila: koetta varten ei avata sarjaporttia tai käynnistetä lukijaa.
    app.set_reader_running(true);
    app.invoke_configure_draw_classes();
    assert!(app.get_draw_classes_open());
    app.invoke_draw_classes_close(false);
    app.invoke_draw_starts();
    assert!(
        app.get_confirm_open(),
        "Lähtöarvontaa voi valmistella lukijan ollessa käynnissä"
    );
    assert!(app.get_reader_running());
    app.invoke_confirm_action(false);
    app.set_reader_running(false);
    let excel =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/excel-import/synteettinen.xlsx");
    let mut preview = as2026::excel_import::read(&excel)?;
    preview.kilpailijat[0].tiedot.luokka = "SM-A".into();
    let imported_directory = directory.join("rata-taydennetaan");
    offer_excel_import(
        &mut state.borrow_mut(),
        app,
        &excel,
        &preview,
        Some(imported_directory.clone()),
    )?;
    assert!(
        app.get_confirm_acceptable(),
        "Tuonti onnistuu ilman ratavalintoja"
    );
    assert_eq!(app.get_import_routes().row_data(0).unwrap().as_str(), "-");
    app.invoke_confirm_action(true);
    app.invoke_navigate(4);
    let imported = ProjectStore::open(&imported_directory.join(native::FILE_NAME))?;
    let index = imported
        .project()
        .luokat
        .iter()
        .position(|class| class.tunnus == "SM-A")
        .unwrap();
    assert_eq!(imported.project().luokat[index].rata, 0);
    app.invoke_select_row(index as i32);
    assert_eq!(
        app.get_fields()
            .iter()
            .find(|field| field.key == 2)
            .unwrap()
            .options
            .row_data(0)
            .unwrap()
            .as_str(),
        "-"
    );
    assert!(app.invoke_field_commit(2, "1".into()));
    assert_eq!(
        ProjectStore::open(&imported_directory.join(native::FILE_NAME))?
            .project()
            .luokat[index]
            .rata,
        1
    );
    open_workspace(&mut state.borrow_mut(), app, original_path, true)?;
    println!(
        "GUI-automaattitallennus hyväksytty: asemaluonnoksen palautuminen oikealle radalle, automaattinen lisäys, kellokentän rajat, kilpailijan lisäys, arvonta aktiivisessa lukutilassa sekä tuonti ja ratavalinnan täydentäminen."
    );
    Ok(())
}

fn synthetic_frame(card: u32, punches: &[(u8, u16)]) -> emit::EmitFrame {
    let mut decoded = [0; emit::FRAME_LENGTH];
    decoded[..2].copy_from_slice(&[255, 255]);
    decoded[2..5].copy_from_slice(&card.to_le_bytes()[..3]);
    for (index, (code, elapsed)) in punches.iter().enumerate() {
        let offset = 10 + index * 3;
        decoded[offset] = *code;
        decoded[offset + 1..offset + 3].copy_from_slice(&elapsed.to_le_bytes());
    }
    emit::parse_frame(&decoded.map(|byte| byte ^ emit::XOR_MASK)).unwrap()
}

/// Yksi kilpailu tuonnista valmiisiin tuloksiin oikeiden GUI-callbackien kautta.
fn finish_imported_race(
    app: &AppWindow,
    state: &SharedState,
    directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = directory.join(native::FILE_NAME);
    app.invoke_navigate(4);
    for (code, start) in [("C", "17:02:00"), ("B", "18:30:05")] {
        let index = state
            .borrow()
            .workspace
            .as_ref()
            .unwrap()
            .project()
            .unwrap()
            .luokat
            .iter()
            .position(|c| c.tunnus == code)
            .unwrap();
        app.invoke_select_row(index as i32);
        assert!(app.invoke_field_commit(4, start.into()));
    }
    let before_choice = std::fs::read(&path)?;
    app.invoke_configure_draw_classes();
    assert!(app.get_draw_classes_open());
    app.invoke_draw_classes_all(true);
    app.invoke_draw_classes_close(false);
    assert_eq!(
        std::fs::read(&path)?,
        before_choice,
        "Peruutus ei muuta luokkavalintoja"
    );
    app.invoke_configure_draw_classes();
    app.invoke_draw_classes_all(true);
    let c = app
        .get_draw_classes()
        .iter()
        .position(|c| c.code == "C")
        .unwrap();
    app.invoke_draw_class_toggled(c as i32, false);
    assert_eq!(
        app.get_draw_classes().iter().filter(|c| c.selected).count(),
        1
    );
    app.invoke_draw_class_toggled(c as i32, true);
    app.invoke_draw_classes_close(true);
    app.invoke_draw_starts();
    assert!(app.get_confirm_open());
    assert!(app.get_confirm_text().contains("Valitut luokat:"));
    app.invoke_confirm_action(true);
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0].lahtoaika,
        "17:02:00"
    );

    let frame = synthetic_frame(12345, &[(0, 120), (140, 568), (100, 1380), (250, 1440)]);
    assert!(!handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(frame.clone()),
            read_clock: ClockTime::new(17, 24, 0)?
        }
    ));
    assert_eq!(app.get_page(), 3);
    assert_eq!(selected_number(&state.borrow())?, 1);
    assert!(!app.get_result_approvable());
    complete_manual_station(app, state, 1, "17:22:02")?;
    app.invoke_edit_result_competitor();
    for (key, value) in [(8, "3"), (9, "1"), (10, "25")] {
        assert!(app.invoke_field_commit(key, value.into()));
    }
    let row_index = state
        .borrow()
        .workspace
        .as_ref()
        .unwrap()
        .results()?
        .competitors[0]
        .results
        .iter()
        .position(|r| r.sequence == 3)
        .unwrap();
    app.invoke_select_detail(row_index as i32);
    assert!(app.invoke_field_commit(101, "-17".into()));
    assert!(app.invoke_field_commit(102, "9".into()));
    let pending_archive = state.borrow().workspace.as_ref().unwrap().results()?;
    let pending_ranking = application::rank_stored(
        &pending_archive,
        as2026::ranking::TiePolicy::KeepSharedAndFlag,
    )?;
    assert!(!report::ranking_html(&pending_archive, &pending_ranking).contains("Mallikuljettaja"));
    assert!(!report::ranking_csv(&pending_archive, &pending_ranking).contains("Mallikuljettaja"));
    print_current(app);
    let approved = ProjectStore::open(&path)?.project().kilpailijat[0].clone();
    assert!(approved.tallennettu_tulos.as_ref().unwrap().hyvaksytty);
    assert_eq!(approved.lisavirheet.puuttuvat_rt_pisteet, 1800);
    assert_eq!(approved.lisavirheet.muut_virhepisteet, 25);

    // Radan otsikkokentän keskeneräinen muutos suojataan kuten kilpailijakenttä.
    app.invoke_navigate(5);
    app.invoke_select_row(0);
    app.invoke_route_field_draft(1, "Elinkaarikokeen rata".into());
    handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(frame),
            read_clock: ClockTime::new(17, 25, 0)?,
        },
    );
    assert_eq!(app.get_page(), 5);
    assert!(state.borrow().pending_result.is_some());
    assert!(app.invoke_route_field_commit(1, "Elinkaarikokeen rata".into()));
    follow_pending_result(&mut state.borrow_mut(), app)?;
    app.invoke_result_entry_close(false);
    assert_eq!(app.get_page(), 3);
    assert_eq!(
        ProjectStore::open(&path)?.project().kilpailijat[0].tallennettu_tulos,
        approved.tallennettu_tulos
    );

    let frame = synthetic_frame(23456, &[(0, 120), (100, 720), (250, 780)]);
    handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(frame),
            read_clock: ClockTime::new(18, 41, 5)?,
        },
    );
    assert_eq!(selected_number(&state.borrow())?, 2);
    complete_manual_station(app, state, 2, "18:35:05")?;
    print_current(app);
    let store = ProjectStore::open(&path)?;
    assert!(
        store.project().kilpailijat.iter().all(|d| d
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty)
    );
    assert_eq!(store.project().leimaluvut.len(), 3);
    let archive = state.borrow().workspace.as_ref().unwrap().results()?;
    assert!(archive.competitors.iter().all(|d| d.status == "TRK"));
    let ranked = application::rank_stored(&archive, as2026::ranking::TiePolicy::KeepSharedAndFlag)?;
    for (name, text) in [
        (
            "lahtoluettelo.html",
            as2026::starts::start_list_html(store.project())?,
        ),
        (
            "lahtoluettelo.csv",
            as2026::starts::start_list_csv(store.project())?,
        ),
        (
            "asemaraportti.html",
            report::project_results_html(store.project())?,
        ),
        (
            "tulokset.html",
            report::project_ranking_html(&archive, &ranked),
        ),
        ("tulokset.csv", report::ranking_csv(&archive, &ranked)),
    ] {
        let output = directory.join(name);
        report::save_new(&output, &text)?;
        assert!(
            report::save_new(&output, &text).is_err(),
            "Raporttia ei ylikirjoiteta"
        );
    }
    report::save_new_bytes(
        &directory.join("tulokset.xlsx"),
        &report::ranking_xlsx(&archive, &ranked)?,
    )?;
    let before_reopen = std::fs::read(&path)?;
    open_workspace(&mut state.borrow_mut(), app, &path, true)?;
    assert_eq!(
        state.borrow().workspace.as_ref().unwrap().project(),
        Some(store.project())
    );
    let reopened = state.borrow().workspace.as_ref().unwrap().results()?;
    assert_eq!(
        report::ranking_csv(
            &reopened,
            &application::rank_stored(&reopened, as2026::ranking::TiePolicy::KeepSharedAndFlag)?
        ),
        report::ranking_csv(&archive, &ranked)
    );
    assert_eq!(std::fs::read(&path)?, before_reopen);
    println!(
        "GUI-kilpailun elinkaarikoe hyväksytty: Excel → luokat ja radat → lähtöarvonta → Emit → käsikellot → viisi pisteosaa → hyväksytyt tulokset → raportit → uudelleenavaus. Aineisto: {}",
        directory.display()
    );
    Ok(())
}

fn complete_manual_station(
    app: &AppWindow,
    state: &SharedState,
    number: u32,
    clock: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let archive = state.borrow().workspace.as_ref().unwrap().results()?;
    let competitor = archive
        .competitors
        .iter()
        .find(|d| d.number == number)
        .unwrap();
    let index = competitor
        .results
        .iter()
        .position(|r| r.expected_code == 0 && r.station_type != "L")
        .unwrap();
    app.invoke_select_detail(index as i32);
    assert!(app.invoke_field_commit(100, clock.into()));
    app.invoke_examine_results();
    assert!(app.get_result_approvable(), "{}", app.get_status_text());
    Ok(())
}

pub fn prepare_print_entry(
    app: &AppWindow,
    state: &SharedState,
    parent: &Path,
    original: &Project,
    suffix: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut p = original.clone();
    p.leimaluvut.clear();
    p.beginners_cup = None;
    p.bc_osallistujat.clear();
    p.yhdistetyt_luokat.clear();
    p.kilpailijat.truncate(1);
    let driver = &mut p.kilpailijat[0];
    driver.numero = 7;
    driver.emit = 123456;
    driver.lahtoaika = "12:00:00".into();
    driver.emit_korjaus_sekuntia = 0;
    driver.tallennettu_tulos = None;
    driver.lisavirheet = Default::default();
    driver.tila = None;
    let route = p
        .luokat
        .iter()
        .find(|c| c.tunnus == driver.luokka)
        .unwrap()
        .rata;
    let stations = &mut p
        .radat
        .iter_mut()
        .find(|r| r.numero == route)
        .unwrap()
        .asemat;
    let mut at = stations[0].clone();
    at.tyyppi = "AT".into();
    at.nimi = "AT käsikello".into();
    at.emit_koodi = 0;
    at.tavoiteaika_sekuntia = 300;
    let mut jat = at.clone();
    jat.tyyppi = "JAT".into();
    jat.nimi = "JAT käsikello".into();
    jat.tavoiteaika_sekuntia = 480;
    jat.jakso_minuutteina = Some(8);
    let mut finish = at.clone();
    finish.tyyppi = "M".into();
    finish.nimi = "Maali".into();
    finish.emit_koodi = 100;
    finish.tavoiteaika_sekuntia = 180;
    finish.jakso_minuutteina = Some(3);
    *stations = vec![stations[0].clone(), at, jat, finish];
    let directory = parent.join(suffix);
    let workspace = Workspace::create_imported(&directory, p)?;
    let path = workspace.path().to_owned();
    open_workspace(&mut state.borrow_mut(), app, &path, true)?;
    super::handle_event(
        &mut state.borrow_mut(),
        app,
        ReaderEvent::Frame {
            frame: Box::new(synthetic_frame(123456, &[(0, 120), (100, 780), (250, 840)])),
            read_clock: ClockTime::new(12, 12, 0)?,
        },
    );
    assert!(app.get_result_entry_open(), "{}", app.get_status_text());
    assert_eq!(app.get_result_entry_fields().row_count(), 4);
    Ok(path)
}
fn result_entry_and_printing(
    app: &AppWindow,
    state: &SharedState,
    original_path: &Path,
    original: &Project,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = prepare_print_entry(
        app,
        state,
        original_path.parent().unwrap(),
        original,
        "tulostuksen-syotto",
    )?;
    let fields = app.get_result_entry_fields();
    let first = fields.row_data(0).unwrap().key;
    app.invoke_result_entry_edited(first, "24:00:00".into());
    app.invoke_result_entry_close(true);
    assert!(app.get_result_entry_open());
    assert!(!app.get_result_entry_error().is_empty());
    for (i, value) in ["12:05:00", "12:08:00", "3", "25"].iter().enumerate() {
        app.invoke_result_entry_edited(fields.row_data(i).unwrap().key, (*value).into());
    }
    app.invoke_result_entry_close(true);
    assert!(
        !app.get_result_entry_open(),
        "{}",
        app.get_result_entry_error()
    );
    let stored = ProjectStore::open(&path)?;
    let driver = &stored.project().kilpailijat[0];
    assert!(driver.tallennettu_tulos.as_ref().unwrap().hyvaksytty);
    assert_eq!(driver.lisavirheet.puuttuvat_rt_pisteet, 1800);
    assert_eq!(driver.lisavirheet.muut_virhepisteet, 25);
    assert_eq!(state.borrow().last_print.as_ref().unwrap().pages.len(), 1);
    app.invoke_open_print_classes(true);
    assert!(app.get_print_classes_open());
    assert!(app.get_print_landscape());
    assert!(app.get_print_fit());
    app.invoke_print_classes_all(false);
    assert!(app.get_print_classes().iter().all(|item| !item.selected));
    app.invoke_print_classes_close(true);
    assert!(!app.get_print_error().is_empty());
    app.invoke_print_classes_all(true);
    assert!(app.get_print_classes().iter().all(|item| item.selected));
    assert!(app.get_print_error().is_empty());
    app.invoke_print_classes_all(false);
    app.invoke_print_class_toggled(0, true);
    app.invoke_print_classes_close(true);
    assert!(!app.get_print_classes_open(), "{}", app.get_print_error());
    assert!(state.borrow().last_print.as_ref().unwrap().landscape);
    open_workspace(&mut state.borrow_mut(), app, original_path, true)?;
    println!(
        "GUI-tulostuskoe: kortinluvun syöttöikkuna, aikavalidointi, automaattilaskenta, RT/muut pisteet, luokkavalinta ja asematuloste ilman fyysistä tulostusta."
    );
    Ok(())
}
