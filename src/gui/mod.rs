//! Slint-sovitin. Kilpailutieto, laskenta ja tallennus kuuluvat yhteiseen ytimeen.
slint::include_modules!();
mod desktop;
mod forms;
mod smoke;
mod tables;

use as2026::{
    ClockTime, CoreError, application, emit, installation, io_message,
    live_reader::{ReaderEvent, ReaderWorker},
    native::{self, Project},
    preferences::PreferencesStore,
    report,
    storage::ProjectStore,
    updates,
    workspace::Workspace,
};
use chrono::Timelike;
use forms::Selection;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc::Receiver,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

type GuiResult<T> = Result<T, CoreError>;
type SharedState = Rc<RefCell<State>>;

enum UpdateEvent {
    Checked(Result<updates::UpdateCheck, String>),
    Downloaded(Result<PathBuf, String>),
    Handoff(Result<(), String>),
}

enum PendingAction {
    Review(u32),
    ConfirmInterruption {
        number: u32,
        reason: String,
    },
    Draw(Box<as2026::starts::StartDraw>),
    Import {
        preview: Box<as2026::excel_import::ImportPreview>,
        directory: Option<PathBuf>,
    },
    Notice,
}

struct PendingResult {
    number: u32,
    card: u32,
    notice: String,
}

#[derive(Default)]
struct State {
    pending: Option<PendingAction>,
    pending_result: Option<PendingResult>,
    workspace: Option<Workspace>,
    preferences: Option<PreferencesStore>,
    update: Option<updates::Update>,
    update_receiver: Option<Receiver<UpdateEvent>>,
    update_bundle: Option<PathBuf>,
    install_bundle: Option<PathBuf>,
    page: i32,
    selected: i32,
    detail: i32,
    selection: Selection,
    draft: BTreeMap<i32, String>,
    invalid_field: bool,
    invalid_fields: BTreeSet<i32>,
    edit_draft: BTreeMap<i32, String>,
    route_draft: BTreeMap<i32, String>,
    invalid_route_fields: BTreeSet<i32>,
    filter: String,
    worker: Option<ReaderWorker>,
    receiver: Option<Receiver<ReaderEvent>>,
    unsaved: Vec<(Box<emit::EmitFrame>, ClockTime)>,
    read_notice_timer: Timer,
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn model<T: Clone + 'static>(values: Vec<T>) -> ModelRc<T> {
    Rc::new(VecModel::from(values)).into()
}
fn status(app: &AppWindow, text: impl Into<SharedString>, error: bool) {
    app.set_status_text(text.into());
    app.set_status_error(error);
}
fn run_action(app: &AppWindow, operation: impl FnOnce() -> GuiResult<()>) {
    if let Err(error) = operation() {
        let message = error.to_string();
        if app.get_import_open() {
            app.set_import_error(
                message
                    .strip_prefix("Virheellinen syöte: ")
                    .unwrap_or(&message)
                    .into(),
            );
        }
        status(
            app,
            message
                .strip_prefix("Virheellinen syöte: ")
                .unwrap_or(&message),
            true,
        );
    }
}

fn is_new(selection: &Selection) -> bool {
    matches!(
        selection,
        Selection::NewDriver
            | Selection::NewClass
            | Selection::NewTrack
            | Selection::NewStation { .. }
    )
}
fn can_leave(state: &mut State, app: &AppWindow) -> bool {
    for (key, value) in state.route_draft.clone() {
        if !commit_route_field(state, app, key, &value) {
            return false;
        }
    }
    if !is_new(&state.selection) {
        let pending = state.edit_draft.clone();
        for (key, value) in pending {
            if !commit_field(state, app, key, &value) {
                return false;
            }
        }
    }
    if state.invalid_field || is_new(&state.selection) {
        status(
            app,
            "Hyväksy tai peruuta keskeneräinen muutos. Virheellisen kentän voi korjata tai palauttaa tallennetut tiedot.",
            true,
        );
        return false;
    }
    true
}
fn can_switch(state: &mut State, app: &AppWindow) -> bool {
    if !can_leave(state, app) {
        return false;
    }
    if state.worker.is_some() || !state.unsaved.is_empty() {
        status(
            app,
            "Pysäytä lukija ja tallenna odottavat lukemat ennen kilpailun vaihtamista.",
            true,
        );
        return false;
    }
    true
}

fn current_workspace(state: &mut State) -> GuiResult<&mut Workspace> {
    state
        .workspace
        .as_mut()
        .ok_or_else(|| invalid("Avaa ensin kilpailu"))
}

fn update_header(state: &State, app: &AppWindow) -> GuiResult<()> {
    let Some(ws) = &state.workspace else {
        return Ok(());
    };
    app.set_opened(true);
    app.set_editable(ws.is_editable());
    app.set_competition_name(ws.name().into());
    app.set_competition_path(ws.path().display().to_string().into());
    let archive = ws.archive()?;
    app.set_driver_count(archive.competitors.len() as i32);
    app.set_class_count(archive.basic.classes.len() as i32);
    app.set_track_count(archive.routes.len() as i32);
    app.set_read_count(ws.project().map_or(0, |p| p.leimaluvut.len()) as i32);
    app.set_unsaved_count(state.unsaved.len() as i32);
    Ok(())
}

fn update_tables(state: &State, app: &AppWindow) -> GuiResult<()> {
    let Some(ws) = &state.workspace else {
        return Ok(());
    };
    let view = tables::data(ws, state.page, state.selected, &state.filter)?;
    app.set_rows(model(view.rows));
    app.set_headers(model(view.headers));
    app.set_weights(model(view.weights));
    app.set_detail_rows(model(view.detail_rows));
    app.set_detail_headers(model(view.detail_headers));
    app.set_detail_weights(model(view.detail_weights));
    app.set_detail_title(view.detail_title.into());
    app.set_calculation_note_count(
        view.notes
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count() as i32,
    );
    app.set_calculation_notes(view.notes.into());
    app.set_result_overview(view.result_overview);
    app.set_selected_key(state.selected);
    app.set_detail_key(state.detail);
    let stored = ws
        .project()
        .and_then(|p| {
            usize::try_from(state.selected)
                .ok()
                .and_then(|i| p.kilpailijat.get(i))
        })
        .and_then(|d| d.tallennettu_tulos.as_ref());
    app.set_examine_supported(stored.is_some());
    let selected_number = ws
        .project()
        .and_then(|p| {
            usize::try_from(state.selected)
                .ok()
                .and_then(|i| p.kilpailijat.get(i))
        })
        .map(|d| d.numero);
    let snapshot = selected_number.and_then(|number| ws.result_snapshot(number).ok().flatten());
    let interrupted = ws.project().is_some_and(|project| {
        selected_number.is_some_and(|number| {
            project
                .kilpailijat
                .iter()
                .find(|driver| driver.numero == number)
                .and_then(|driver| driver.tila.as_ref())
                .is_some_and(|status| status.tunnus == "KESK")
        })
    });
    let approvable = if interrupted {
        selected_number.is_some_and(|number| ws.can_confirm_interruption(number))
    } else {
        snapshot.as_ref().is_some_and(|r| r.can_approve())
            && ws.project().is_some_and(|p| {
                selected_number.is_some_and(|number| {
                    application::validate_project_result(p, number, snapshot.as_ref().unwrap())
                        .is_ok()
                })
            })
    };
    app.set_result_interrupted(interrupted);
    app.set_result_approvable(approvable);
    app.set_result_stored(stored.is_some());
    app.set_clock_pending(stored.is_some_and(|r| r.kasikellot_muuttuneet));
    update_header(state, app)
}

fn update_form(state: &State, app: &AppWindow) {
    let new = is_new(&state.selection);
    let visible = !matches!(state.selection, Selection::None) && state.page != 6;
    app.set_editor_visible(visible);
    app.set_new_record(new);
    app.set_form_blocked(state.invalid_field);
    let title = match &state.selection {
        Selection::Driver(_) => "Kilpailijan tiedot",
        Selection::Class(_) => "Luokan tiedot",
        Selection::Track(_) => "Radan tiedot",
        Selection::Station { .. } => "Aseman tiedot",
        Selection::ResultStation { .. } => "Asematuloksen korjaus",
        Selection::Settings => "Kilpailun asetukset",
        Selection::NewDriver => "Uusi kilpailija",
        Selection::NewClass => "Uusi luokka",
        Selection::NewTrack => "Uusi rata",
        Selection::NewStation { .. } => "Uusi asema",
        Selection::None => "",
    };
    app.set_editor_title(title.into());
    app.set_editor_description(
        if matches!(state.selection, Selection::ResultStation { .. }) {
            "Kellokorjaus säilyttää pisteet. Tutki laskee tallennetuista kelloista. Tarkista palauttaa kortinlukutuloksen."
        } else if new {
            "Täytä tiedot ja valitse Lisää. Kilpailu tallentuu automaattisesti."
        } else {
            "Automaattinen tallennus"
        }
        .into(),
    );
    let fields = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .map_or_else(Vec::new, |p| forms::fields(p, &state.selection));
    app.set_fields(model(fields));
    // Route details are independent of station selection, so the panel
    // remains visible at a stable position whenever a route is selected.
    let selected_route = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .and_then(|project| {
            usize::try_from(state.selected)
                .ok()
                .and_then(|index| project.radat.get(index))
                .map(|track| (project, track.numero))
        });
    let route_visible = state.page == 5
        && !matches!(state.selection, Selection::NewTrack)
        && selected_route.is_some();
    app.set_route_editor_visible(route_visible);
    app.set_route_fields(model(
        selected_route.map_or_else(Vec::new, |(project, number)| {
            forms::fields(project, &Selection::Track(number))
        }),
    ));
}

fn choose_row(state: &mut State, key: i32) -> GuiResult<()> {
    state.selected = key;
    state.detail = -1;
    state.selection = Selection::None;
    let Some(project) = state.workspace.as_ref().and_then(Workspace::project) else {
        return Ok(());
    };
    let index = usize::try_from(key).ok();
    state.selection = match state.page {
        1 => index
            .and_then(|i| project.kilpailijat.get(i))
            .map_or(Selection::None, |d| Selection::Driver(d.numero)),
        4 => index
            .and_then(|i| project.luokat.get(i))
            .map_or(Selection::None, |c| Selection::Class(c.tunnus.clone())),
        5 => index
            .and_then(|i| project.radat.get(i))
            .map_or(Selection::None, |t| Selection::Track(t.numero)),
        6 => Selection::Settings,
        _ => Selection::None,
    };
    Ok(())
}

fn navigate(state: &mut State, app: &AppWindow, page: i32) -> GuiResult<()> {
    if !(0..=6).contains(&page) || !can_leave(state, app) {
        return Ok(());
    }
    state.page = page;
    state.filter.clear();
    state.draft.clear();
    app.set_page(page);
    app.set_filter("".into());
    choose_row(state, -1)?;
    if let Some(ws) = &state.workspace {
        let first = tables::data(ws, page, -1, "")?
            .rows
            .first()
            .map(|row| row.key);
        if let Some(key) = first {
            choose_row(state, key)?;
        }
    }
    update_tables(state, app)?;
    update_form(state, app);
    app.window().request_redraw();
    Ok(())
}

fn open_workspace(state: &mut State, app: &AppWindow, path: &Path, demo: bool) -> GuiResult<()> {
    let ws = Workspace::open(path)?;
    let message = if demo {
        "Esimerkkikilpailu avattu."
    } else if ws.is_editable() {
        "Kilpailu avattu. Muutokset tallentuvat automaattisesti."
    } else {
        "AS2015A-kilpailu avattu vain katseltavaksi. Alkuperäistä aineistoa ei muokata."
    };
    state.workspace = Some(ws);
    state.selection = Selection::None;
    state.pending_result = None;
    state.draft.clear();
    state.invalid_field = false;
    state.invalid_fields.clear();
    state.edit_draft.clear();
    state.route_draft.clear();
    state.invalid_route_fields.clear();
    app.set_demo(demo);
    state.read_notice_timer.stop();
    app.set_read_notice_visible(false);
    navigate(state, app, 0)?;
    status(app, message, false);
    if !demo {
        let path = state.workspace.as_ref().unwrap().path().to_owned();
        if let Some(preferences) = state.preferences.as_mut()
            && let Err(error) = preferences.remember_competition(&path)
        {
            status(
                app,
                format!("Kilpailu avattu. Viimeisimmän kilpailun muistaminen epäonnistui: {error}"),
                true,
            );
        }
    }
    Ok(())
}

fn demo_path() -> GuiResult<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| invalid("Järjestelmän kello on virheellinen"))?
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("as2026-kokeilu-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&directory).map_err(|e| invalid(&io_message(&e)))?;
    let project = Project::from_toml(include_str!("../../fixtures/native-demo/kilpailu.toml"))?;
    let path = directory.join(native::FILE_NAME);
    ProjectStore::create(&path, project)?;
    Ok(path)
}

fn add_record(state: &mut State, app: &AppWindow, station: bool) -> GuiResult<()> {
    if !can_leave(state, app) {
        return Ok(());
    }
    let project = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .ok_or_else(|| invalid("Avaa muokattava AS2026-kilpailu"))?;
    let selection = if station {
        let track = project
            .radat
            .get(state.selected as usize)
            .ok_or_else(|| invalid("Valitse ensin rata"))?;
        Selection::NewStation {
            track: track.numero,
        }
    } else {
        match state.page {
            1 if project.luokat.is_empty() => {
                return Err(invalid("Lisää ensin kilpailijan luokka Luokat-näkymässä"));
            }
            1 => Selection::NewDriver,
            4 if project.radat.is_empty() => {
                return Err(invalid("Lisää ensin rata Radat ja asemat -näkymässä"));
            }
            4 => Selection::NewClass,
            5 => Selection::NewTrack,
            _ => return Ok(()),
        }
    };
    state.draft = forms::fields(project, &selection)
        .iter()
        .map(|f| (f.key, f.value.to_string()))
        .collect();
    state.selection = selection;
    update_form(state, app);
    status(
        app,
        "Uusi tietue on luonnos. Täytä tiedot ja valitse Lisää tai Peruuta.",
        false,
    );
    Ok(())
}

fn sync_selected_key(state: &mut State) {
    let Some(project) = state.workspace.as_ref().and_then(Workspace::project) else {
        return;
    };
    let index = match &state.selection {
        Selection::Driver(number) => project.kilpailijat.iter().position(|d| d.numero == *number),
        Selection::Class(code) => project.luokat.iter().position(|c| c.tunnus == *code),
        Selection::Track(number) => project.radat.iter().position(|t| t.numero == *number),
        Selection::Station { track, index } => {
            state.detail = *index as i32;
            project.radat.iter().position(|t| t.numero == *track)
        }
        _ => None,
    };
    if let Some(index) = index {
        state.selected = index as i32;
    }
}

fn update_station_rules(state: &State, app: &AppWindow) {
    if !matches!(
        state.selection,
        Selection::Station { .. } | Selection::NewStation { .. }
    ) {
        return;
    }
    let Some(project) = state.workspace.as_ref().and_then(Workspace::project) else {
        return;
    };
    let fields = forms::fields(project, &state.selection);
    let value = |key| {
        state.draft.get(&key).cloned().unwrap_or_else(|| {
            fields
                .iter()
                .find(|f| f.key == key)
                .map_or_else(String::new, |f| f.value.to_string())
        })
    };
    let model = app.get_fields();
    if let Some(index) = model.iter().position(|field| field.key == 4) {
        model.set_row_data(index, forms::station_rule_field(&value(0), &value(4)));
    }
}

fn commit_field(state: &mut State, app: &AppWindow, key: i32, value: &str) -> bool {
    if is_new(&state.selection) {
        state.draft.insert(key, value.into());
        if key == 0 {
            update_station_rules(state, app);
        }
        return true;
    }
    let selection = state.selection.clone();
    let result =
        current_workspace(state).and_then(|ws| forms::apply_field(ws, &selection, key, value));
    match result {
        Ok(selection) => {
            state.selection = selection;
            state.invalid_fields.remove(&key);
            state.edit_draft.remove(&key);
            state.invalid_field =
                !state.invalid_fields.is_empty() || !state.invalid_route_fields.is_empty();
            app.set_form_blocked(state.invalid_field);
            sync_selected_key(state);
            if key == 0 {
                update_station_rules(state, app);
            }
            if state.page == 6 && (30..=33).contains(&key) {
                update_form(state, app);
            }
            if let Err(error) = update_tables(state, app) {
                status(app, error.to_string(), true);
            } else {
                status(
                    app,
                    if state.invalid_field {
                        "Kenttä tallennettu. Korjaa myös aiempi virheellinen kenttä."
                    } else {
                        "Muutos tallennettu."
                    },
                    state.invalid_field,
                );
            }
            // Lomaketta ei rakenneta uudelleen joka kentällä: kohdistus ja luonnokset säilyvät.
            true
        }
        Err(error) => {
            state.invalid_field = true;
            state.invalid_fields.insert(key);
            app.set_form_blocked(true);
            status(app, format!("Muutosta ei tallennettu: {error}"), true);
            false
        }
    }
}

// The parent route remains editable even while an individual station is open.
// Do not replace state.selection with Selection::Track, as that would hide
// the station editor and cause the old jumping layout.
fn commit_route_field(state: &mut State, app: &AppWindow, key: i32, value: &str) -> bool {
    let route_number = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .and_then(|project| {
            usize::try_from(state.selected)
                .ok()
                .and_then(|index| project.radat.get(index))
                .map(|route| route.numero)
        });
    let Some(route_number) = route_number else {
        status(app, "Valitse ensin rata.", true);
        return false;
    };
    match current_workspace(state).and_then(|workspace| {
        forms::apply_field(workspace, &Selection::Track(route_number), key, value)
    }) {
        Ok(Selection::Track(updated_number)) => {
            state.route_draft.remove(&key);
            state.invalid_route_fields.remove(&key);
            state.invalid_field =
                !state.invalid_fields.is_empty() || !state.invalid_route_fields.is_empty();
            app.set_form_blocked(state.invalid_field);
            if key == 0 {
                match &mut state.selection {
                    Selection::Track(number) => *number = updated_number,
                    Selection::Station { track, .. } | Selection::NewStation { track } => {
                        *track = updated_number;
                    }
                    _ => {}
                }
            }
            if let Err(error) = update_tables(state, app) {
                status(app, error.to_string(), true);
            } else {
                status(app, "Radan tiedot tallennettu.", false);
            }
            // Updating route number changes the editor's referent.
            if key == 0 {
                update_form(state, app);
            }
            true
        }
        Ok(_) => {
            status(app, "Radan muokkaus ei palauttanut rataa.", true);
            false
        }
        Err(error) => {
            state.route_draft.insert(key, value.into());
            state.invalid_route_fields.insert(key);
            state.invalid_field = true;
            app.set_form_blocked(true);
            status(app, format!("Radan tietoja ei tallennettu: {error}"), true);
            false
        }
    }
}

fn can_follow_readout(state: &State, app: &AppWindow) -> bool {
    !state.invalid_field
        && state.edit_draft.is_empty()
        && state.route_draft.is_empty()
        && !is_new(&state.selection)
        && !app.get_dialog_open()
        && !app.get_about_open()
        && !app.get_confirm_open()
        && !app.get_draw_classes_open()
        && !app.get_setup_open()
        && !app.get_install_busy()
}

fn follow_pending_result(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    if state.pending_result.is_none() || !can_follow_readout(state, app) {
        return Ok(());
    }
    let pending = state.pending_result.take().unwrap();
    let index = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .and_then(|p| {
            p.kilpailijat
                .iter()
                .position(|d| d.numero == pending.number && d.emit == pending.card)
        })
        .ok_or_else(|| {
            invalid("Luetun kortin kilpailijatiedot ovat muuttuneet. Valitse kilpailija uudelleen.")
        })?;
    state.page = 3;
    state.filter.clear();
    app.set_page(3);
    app.set_filter("".into());
    choose_row(state, index as i32)?;
    update_tables(state, app)?;
    update_form(state, app);
    status(app, pending.notice, false);
    Ok(())
}

fn defer_pending_result(state: &SharedState, app: &AppWindow) {
    if state.borrow().pending_result.is_some() {
        let state = state.clone();
        let weak = app.as_weak();
        Timer::single_shot(Duration::ZERO, move || {
            if let Some(app) = weak.upgrade() {
                run_action(&app, || {
                    follow_pending_result(&mut state.borrow_mut(), &app)
                });
            }
        });
    }
}

fn update_draw_class_summary(app: &AppWindow) {
    let classes = app.get_draw_classes();
    let selected = classes.iter().filter(|entry| entry.selected).count();
    app.set_draw_class_summary(
        format!("Valittuna {selected} / {} luokkaa", classes.row_count()).into(),
    );
}

fn show_read_notice(state: &mut State, app: &AppWindow, text: String) {
    app.set_read_notice(text.into());
    app.set_read_notice_visible(true);
    let weak = app.as_weak();
    // Sama ajastin käynnistyy uudestaan: vanha kuittaus ei piilota uudempaa lukua.
    state
        .read_notice_timer
        .start(TimerMode::SingleShot, Duration::from_secs(3), move || {
            if let Some(app) = weak.upgrade() {
                app.set_read_notice_visible(false);
            }
        });
}

fn handle_event(state: &mut State, app: &AppWindow, event: ReaderEvent) -> bool {
    match event {
        ReaderEvent::Connected { port } => {
            app.set_reader_status(format!("Yhdistetty: {port} · 9600 8N2").into());
            false
        }
        ReaderEvent::Frame { frame, read_clock } => {
            match current_workspace(state).and_then(|ws| ws.record_frame(&frame, read_clock)) {
                Ok(outcome) => {
                    let competitor = state
                        .workspace
                        .as_ref()
                        .and_then(Workspace::project)
                        .and_then(|p| {
                            p.kilpailijat
                                .iter()
                                .find(|d| Some(d.numero) == outcome.competitor_number)
                        });
                    let identity = competitor.map_or_else(
                        || "Tuntematon kortti".into(),
                        |d| format!("#{} {} / {}", d.numero, d.kuljettaja, d.kartanlukija),
                    );
                    let preserved = competitor.is_some_and(|d| d.tallennettu_tulos.is_some());
                    show_read_notice(
                        state,
                        app,
                        format!("✓ Emit {} luettu · {identity}", outcome.card_number),
                    );
                    if let Some(number) = outcome.competitor_number {
                        let note = if preserved {
                            "Aiempi tarkastustulos säilyi. Tarkista muodostaa asemarivit uudesta korttiluvusta."
                        } else {
                            "Täydennä käsikellot ja tarkasta tulos."
                        };
                        let mut notice = format!(
                            "Emit {} tallennettu · {identity}. {note}",
                            outcome.card_number
                        );
                        if let Err(error) =
                            current_workspace(state).and_then(|ws| ws.ensure_readout_result(number))
                        {
                            notice = format!(
                                "Lukema tallennettiin. Asematuloksen muodostaminen ei onnistunut: {error}"
                            );
                        }
                        state.pending_result = Some(PendingResult {
                            number,
                            card: outcome.card_number,
                            notice,
                        });
                        if !can_follow_readout(state, app) {
                            status(
                                app,
                                "Uusi korttiluku tallennettu. Tulokset avataan, kun keskeneräinen muutos on hyväksytty tai peruutettu.",
                                false,
                            );
                        }
                        if let Err(error) = follow_pending_result(state, app) {
                            status(app, error.to_string(), true);
                        }
                    } else {
                        status(
                            app,
                            format!(
                                "Emit {} tallennettu. Kortille ei löydy kilpailijaa; lukemaa ei kohdistettu.",
                                outcome.card_number
                            ),
                            false,
                        );
                    }
                    if let Err(error) = update_tables(state, app) {
                        status(app, error.to_string(), true);
                    }
                    false
                }
                Err(error) => {
                    state.read_notice_timer.stop();
                    app.set_read_notice_visible(false);
                    state.unsaved.push((frame, read_clock));
                    app.set_unsaved_count(state.unsaved.len() as i32);
                    status(
                        app,
                        format!(
                            "Lukemaa ei tallennettu: {error}. Lukija pysäytetään; lukema säilyy muistissa uudelleenyritystä varten."
                        ),
                        true,
                    );
                    true
                }
            }
        }
        ReaderEvent::Error(error) => {
            app.set_reader_status(error.clone().into());
            status(app, error, true);
            true
        }
        ReaderEvent::Stopped => false,
    }
}

fn stop_reader(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    let pending = if let Some(mut worker) = state.worker.take() {
        worker.stop().map_err(|e| invalid(&e))?
    } else {
        Vec::new()
    };
    let queued: Vec<_> = state
        .receiver
        .take()
        .map_or_else(Vec::new, |r| r.try_iter().collect());
    for event in queued.into_iter().chain(pending) {
        handle_event(state, app, event);
    }
    app.set_reader_running(false);
    if state.unsaved.is_empty() {
        app.set_reader_status("Lukija pysäytetty.".into());
    }
    Ok(())
}

fn poll_reader(state: &mut State, app: &AppWindow, timer: &Timer) {
    let events: Vec<_> = state
        .receiver
        .as_ref()
        .map_or_else(Vec::new, |r| r.try_iter().collect());
    let mut stop = false;
    for event in events {
        if matches!(event, ReaderEvent::Stopped) {
            stop = true;
        }
        stop |= handle_event(state, app, event);
    }
    if let Err(error) = follow_pending_result(state, app) {
        status(app, error.to_string(), true);
    }
    if stop {
        if let Err(error) = stop_reader(state, app) {
            status(app, error.to_string(), true);
        }
        timer.stop();
    }
}

fn selected_number(state: &State) -> GuiResult<u32> {
    state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .and_then(|p| {
            usize::try_from(state.selected)
                .ok()
                .and_then(|i| p.kilpailijat.get(i))
        })
        .map(|d| d.numero)
        .ok_or_else(|| invalid("Valitse kilpailija"))
}

fn confirmation(
    state: &mut State,
    app: &AppWindow,
    action: PendingAction,
    title: &str,
    text: String,
) {
    state.pending = Some(action);
    app.set_confirm_acceptable(true);
    app.set_confirm_title(title.into());
    app.set_confirm_text(text.into());
    app.set_confirm_open(true);
}

fn offer_excel_import(
    state: &mut State,
    app: &AppWindow,
    source: &Path,
    preview: &as2026::excel_import::ImportPreview,
    directory: Option<PathBuf>,
) -> GuiResult<()> {
    let summary = preview.summary();
    if !preview.has_importable_tracks() {
        confirmation(
            state,
            app,
            PendingAction::Notice,
            "Excel-tuonnin tarkistus",
            format!(
                "{summary}\n\n{}\n\nKorjaa tuonnin estävät kohdat työkirjassa.",
                preview.issue_details()
            ),
        );
        app.set_confirm_acceptable(false);
    } else {
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Tuotu kilpailu");
        app.set_import_name(name.into());
        app.set_import_details(preview.issue_details().into());
        app.set_import_only_tracks(false);
        app.set_import_tab(0);
        app.set_import_details_open(false);
        app.set_import_error("".into());
        app.set_import_location_open(false);
        app.set_import_root(app.get_default_root());
        app.set_import_classes(model(
            preview
                .kilpailijat
                .iter()
                .map(|d| d.tiedot.luokka.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(SharedString::from)
                .collect(),
        ));
        app.set_import_routes(model(
            preview
                .radat
                .iter()
                .map(|r| SharedString::from(r.luokka.as_str()))
                .collect(),
        ));
        app.set_import_route_choices(model(
            preview
                .kilpailijat
                .iter()
                .map(|d| d.tiedot.luokka.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|code| {
                    preview
                        .route_for_class(&code)
                        .and_then(|route| preview.radat.iter().position(|r| r.luokka == route))
                        .map_or(-1, |index| index as i32)
                })
                .collect(),
        ));
        app.set_import_rows(model(
            preview
                .kilpailijat
                .iter()
                .map(|d| TableRow {
                    key: d.tiedot.numero as i32,
                    cells: model(vec![
                        d.tiedot.numero.to_string().into(),
                        d.tiedot.luokka.clone().into(),
                        d.tiedot.kuljettaja.clone().into(),
                        d.tiedot.kartanlukija.clone().into(),
                        d.tiedot
                            .emit_number()
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "Puuttuu".into())
                            .into(),
                        if d.tiedot.lahtoaika.is_empty() {
                            "Puuttuu".into()
                        } else {
                            d.tiedot.lahtoaika.clone().into()
                        },
                    ]),
                })
                .collect(),
        ));
        app.set_import_station_rows(model(
            preview
                .radat
                .iter()
                .flat_map(|r| {
                    r.asemat
                        .iter()
                        .enumerate()
                        .map(move |(i, station)| TableRow {
                            key: i as i32,
                            cells: model(vec![
                                r.luokka.clone().into(),
                                (i + 1).to_string().into(),
                                station.nimi.clone().into(),
                                station.tyyppi.clone().into(),
                                station.emit_koodi.to_string().into(),
                                station.tavoiteaika_sekuntia.to_string().into(),
                            ]),
                        })
                })
                .collect(),
        ));
        confirmation(
            state,
            app,
            PendingAction::Import {
                preview: Box::new(preview.clone()),
                directory,
            },
            "Tuo Excel-kilpailu",
            summary,
        );
        app.set_import_open(true);
        app.set_confirm_acceptable(preview.can_import());
    }
    Ok(())
}

fn bind(app: &AppWindow, state: &SharedState, reader_timer: &Rc<Timer>) {
    macro_rules! action {
        ($method:ident, |$s:ident, $a:ident $(, $arg:ident)*| $body:expr) => {{
            let state = state.clone(); let weak = app.as_weak();
            app.$method(move |$($arg),*| {
                if let Some($a) = weak.upgrade() {
                    let mut borrowed = state.borrow_mut(); let $s = &mut *borrowed;
                    run_action(&$a, || $body);
                }
            });
        }}
    }
    action!(on_navigate, |s, a, page| navigate(s, &a, page));
    action!(on_open_path, |s, a, path| {
        if can_switch(s, &a) {
            let path = as2026::terminal::clean_path(&path);
            open_workspace(s, &a, &path, false)?;
        }
        Ok(())
    });
    action!(on_open_file, |s, a| {
        if can_switch(s, &a)
            && let Some(path) = rfd::FileDialog::new()
                .set_title("Avaa AS2026-kilpailutiedosto")
                .add_filter("Kilpailutiedosto", &["toml"])
                .pick_file()
        {
            open_workspace(s, &a, &path, false)?;
        }
        Ok(())
    });
    action!(on_open_demo, |s, a| {
        if can_switch(s, &a) {
            open_workspace(s, &a, &demo_path()?, true)?;
        }
        Ok(())
    });
    action!(on_create_project, |s, a, path, name| {
        if can_switch(s, &a) {
            let preferences = s
                .preferences
                .as_ref()
                .ok_or_else(|| invalid("Sovellusasetuksia ei voitu avata"))?;
            let workspace = if path.trim().is_empty() {
                preferences.create_competition(&name)
            } else {
                preferences.create_competition_in(&name, &as2026::terminal::clean_path(&path))
            }
            .map_err(|e| invalid(&e))?;
            let path = workspace.path().to_owned();
            open_workspace(s, &a, &path, false)?;
            a.set_dialog_open(false);
        }
        Ok(())
    });
    action!(on_change_default_root, |s, a| {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Valitse uusien kilpailujen tallennuspaikka")
            .pick_folder()
        {
            let preferences = s
                .preferences
                .as_mut()
                .ok_or_else(|| invalid("Sovellusasetuksia ei voitu avata"))?;
            preferences.save_root(Some(path)).map_err(|e| invalid(&e))?;
            a.set_default_root(
                preferences
                    .competition_root()
                    .map_err(|e| invalid(&e))?
                    .display()
                    .to_string()
                    .into(),
            );
            status(&a, "Uusien kilpailujen tallennuspaikka muutettu.", false);
        }
        Ok(())
    });
    action!(on_reset_default_root, |s, a| {
        let preferences = s
            .preferences
            .as_mut()
            .ok_or_else(|| invalid("Sovellusasetuksia ei voitu avata"))?;
        preferences.save_root(None).map_err(|e| invalid(&e))?;
        a.set_default_root(
            preferences
                .competition_root()
                .map_err(|e| invalid(&e))?
                .display()
                .to_string()
                .into(),
        );
        status(&a, "Uusien kilpailujen oletussijainti palautettu.", false);
        Ok(())
    });
    action!(on_browse_new_path, |_s, a| {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Valitse uuden kilpailun kansio")
            .pick_folder()
        {
            a.set_new_path(path.display().to_string().into());
        }
        Ok(())
    });
    action!(on_reload, |s, a| {
        if can_switch(s, &a) {
            current_workspace(s)?.reload()?;
            choose_row(s, s.selected)?;
            if s.page == 6 {
                s.selection = Selection::Settings;
            }
            update_tables(s, &a)?;
            update_form(s, &a);
            status(&a, "Kilpailun tiedot päivitetty levyltä.", false);
        }
        Ok(())
    });
    action!(on_filter_changed, |s, a, text| {
        s.filter = text.to_string();
        update_tables(s, &a)
    });
    action!(on_select_row, |s, a, key| {
        if can_leave(s, &a) {
            choose_row(s, key)?;
            update_tables(s, &a)?;
            update_form(s, &a);
        }
        Ok(())
    });
    action!(on_select_detail, |s, a, key| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        if s.page == 3 && current_workspace(s)?.is_editable() {
            let number = selected_number(s)?;
            let archive = current_workspace(s)?.results()?;
            if let Some(row) = archive
                .competitors
                .iter()
                .find(|d| d.number == number)
                .and_then(|d| usize::try_from(key).ok().and_then(|i| d.results.get(i)))
            {
                s.detail = key;
                s.selection = Selection::ResultStation {
                    number,
                    row: row.output_index,
                };
                a.set_detail_key(key);
                update_form(s, &a);
            }
        } else if s.page == 5 {
            let project = s
                .workspace
                .as_ref()
                .and_then(Workspace::project)
                .ok_or_else(|| invalid("Aineisto on vain katseltavissa"))?;
            if let Some(track) = project.radat.get(s.selected as usize)
                && key >= 0
                && (key as usize) < track.asemat.len()
            {
                s.detail = key;
                s.selection = Selection::Station {
                    track: track.numero,
                    index: key as usize,
                };
                a.set_detail_key(key);
                update_form(s, &a);
            }
        }
        Ok(())
    });
    action!(on_add_record, |s, a| add_record(s, &a, false));
    action!(on_add_station, |s, a| add_record(s, &a, true));
    action!(on_move_station, |s, a, delta| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        let project = s
            .workspace
            .as_ref()
            .and_then(Workspace::project)
            .ok_or_else(|| invalid("Valitse rata"))?;
        let track = project
            .radat
            .get(s.selected as usize)
            .ok_or_else(|| invalid("Valitse rata"))?;
        let track_number = track.numero;
        let target = s.detail + delta;
        if s.detail >= 0 && target >= 0 && (target as usize) < track.asemat.len() {
            let from = s.detail as usize;
            current_workspace(s)?.move_station(track_number, from, target as usize)?;
            s.detail = target;
            s.selection = Selection::Station {
                track: track_number,
                index: target as usize,
            };
            update_tables(s, &a)?;
            update_form(s, &a);
            status(&a, "Asemajärjestys tallennettu.", false);
        }
        Ok(())
    });
    {
        let state = state.clone();
        let weak = app.as_weak();
        app.on_field_commit(move |key, value| {
            weak.upgrade().is_some_and(|app| {
                let accepted = commit_field(&mut state.borrow_mut(), &app, key, &value);
                if accepted {
                    defer_pending_result(&state, &app);
                }
                accepted
            })
        });
    }
    {
        let state = state.clone();
        let weak = app.as_weak();
        app.on_route_field_commit(move |key, value| {
            weak.upgrade().is_some_and(|app| {
                let accepted = commit_route_field(&mut state.borrow_mut(), &app, key, &value);
                if accepted {
                    defer_pending_result(&state, &app);
                }
                accepted
            })
        });
    }
    // Radan ja aseman luonnokset ovat erillisiä, vaikka kenttätunnukset ovat samoja.
    action!(on_route_field_draft, |s, _a, key, value| {
        s.route_draft.insert(key, value.to_string());
        Ok(())
    });
    action!(on_field_draft, |s, a, key, value| {
        if is_new(&s.selection) {
            s.draft.insert(key, value.to_string());
            if key == 0 {
                update_station_rules(s, &a);
            }
        } else {
            s.edit_draft.insert(key, value.to_string());
        }
        Ok(())
    });
    action!(on_add_commit, |s, a| {
        let selection = s.selection.clone();
        let draft = s.draft.clone();
        let added = forms::add(current_workspace(s)?, &selection, &draft)?;
        s.selection = added;
        s.draft.clear();
        s.invalid_field = false;
        sync_selected_key(s);
        update_tables(s, &a)?;
        update_form(s, &a);
        status(&a, "Tietue lisätty ja tallennettu.", false);
        follow_pending_result(s, &a)
    });
    action!(on_cancel_edit, |s, a| {
        s.invalid_field = false;
        s.invalid_fields.clear();
        s.invalid_route_fields.clear();
        s.edit_draft.clear();
        s.route_draft.clear();
        s.draft.clear();
        choose_row(s, s.selected)?;
        if s.page == 6 {
            s.selection = Selection::Settings;
        }
        update_form(s, &a);
        status(&a, "Tallennetut tiedot palautettu näkymään.", false);
        follow_pending_result(s, &a)
    });
    action!(on_review_results, |s, a| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        let number = selected_number(s)?;
        confirmation(
            s,
            &a,
            PendingAction::Review(number),
            "Tarkista leimaukset",
            format!(
                "Kilpailijan {number} asemarivit muodostetaan uudelleen tallennetusta korttiluvusta. Asemien käsikellot ja pistekorjaukset korvataan. Erilliset lisävirheet ja käsin valittu tila säilyvät."
            ),
        );
        Ok(())
    });
    action!(on_edit_result_competitor, |s, a| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        let number = selected_number(s)?;
        s.selection = Selection::Driver(number);
        s.detail = -1;
        a.set_detail_key(-1);
        update_form(s, &a);
        Ok(())
    });
    action!(on_approve_results, |s, a| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        let number = selected_number(s)?;
        let interrupted = current_workspace(s)?
            .project()
            .and_then(|project| {
                project
                    .kilpailijat
                    .iter()
                    .find(|driver| driver.numero == number)
            })
            .and_then(|driver| driver.tila.as_ref())
            .filter(|status| status.tunnus == "KESK")
            .map(|status| status.syy.clone());
        if let Some(reason) = interrupted {
            if !current_workspace(s)?.can_confirm_interruption(number) {
                return Err(invalid(
                    "Tarkista keskeytyksen syy kilpailijan tiedoista. Jo vahvistettua keskeytystä ei tarvitse vahvistaa uudelleen.",
                ));
            }
            confirmation(
                s,
                &a,
                PendingAction::ConfirmInterruption {
                    number,
                    reason: reason.clone(),
                },
                "Vahvista keskeytys",
                format!(
                    "Kilpailija {number} merkitään vahvistetusti keskeyttäneeksi.\n\nSyy: {reason}\n\nPäätös ei edellytä korttilukua tai maaliaikaa eikä muodosta kilpailijalle sijoitusta."
                ),
            );
            return Ok(());
        }
        current_workspace(s)?.approve_result(number)?;
        update_tables(s, &a)?;
        update_form(s, &a);
        status(
            &a,
            "Tulos hyväksytty ja tallennettu. Asemien pistekorjaukset säilytettiin.",
            false,
        );
        Ok(())
    });
    action!(on_import_excel, |s, a| {
        if !can_switch(s, &a) {
            return Ok(());
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Tuo RATA- ja KILP-välilehdet")
            .add_filter("Excel-kilpailupohja", &["xls", "xlsx"])
            .pick_file()
        {
            let preview = as2026::excel_import::read(&path)?;
            offer_excel_import(s, &a, &path, &preview, None)?;
        }
        Ok(())
    });
    action!(on_import_class_route, |s, a, code, route| {
        if let Some(PendingAction::Import { preview, .. }) = s.pending.as_mut() {
            preview.set_class_route(&code, &route)?;
            a.set_confirm_text(preview.summary().into());
            a.set_confirm_acceptable(preview.can_import() || a.get_import_only_tracks());
        }
        Ok(())
    });
    action!(on_import_tracks_only, |s, a, only| {
        a.set_import_only_tracks(only);
        if let Some(PendingAction::Import { preview, .. }) = s.pending.as_ref() {
            a.set_confirm_acceptable(if only {
                preview.has_importable_tracks()
            } else {
                preview.can_import()
            });
            a.set_confirm_text(
                if only {
                    preview.tracks_only().summary()
                } else {
                    preview.summary()
                }
                .into(),
            );
        }
        Ok(())
    });
    action!(on_browse_import_root, |s, a| {
        if matches!(s.pending, Some(PendingAction::Import { .. }))
            && let Some(root) = rfd::FileDialog::new()
                .set_title("Valitse kilpailukansioiden sijainti")
                .pick_folder()
        {
            a.set_import_root(root.display().to_string().into());
        }
        Ok(())
    });
    action!(on_examine_results, |s, a| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        let number = selected_number(s)?;
        current_workspace(s)?.recalculate_stored(number)?;
        update_tables(s, &a)?;
        update_form(s, &a);
        status(
            &a,
            "Tutki: tallennetut asemarivit ja käsikellot laskettu. Asemarivien pistekorjaukset korvattiin laskennalla.",
            false,
        );
        Ok(())
    });
    action!(on_configure_draw_classes, |s, a| {
        if !can_switch(s, &a) {
            return Ok(());
        }
        let project = current_workspace(s)?
            .project()
            .ok_or_else(|| invalid("Luokkavalinta edellyttää muokattavaa kilpailua"))?;
        let options = project
            .luokat
            .iter()
            .map(|class| {
                let count = project
                    .kilpailijat
                    .iter()
                    .filter(|d| d.luokka == class.tunnus)
                    .count();
                let label = if class.nimi == class.tunnus {
                    class.tunnus.clone()
                } else {
                    format!("{} — {}", class.tunnus, class.nimi)
                };
                let count_label = if count == 1 {
                    "1 kilpailija".into()
                } else {
                    format!("{count} kilpailijaa")
                };
                DrawClassOption {
                    code: class.tunnus.clone().into(),
                    label: format!(
                        "{label} ({count_label}) · {} · {} min",
                        class.lahtoaika, class.lahtovali_minuutteina
                    )
                    .into(),
                    selected: class.osallistuu_lahtoarvontaan,
                }
            })
            .collect();
        a.set_draw_classes(model(options));
        update_draw_class_summary(&a);
        a.set_draw_classes_open(true);
        Ok(())
    });
    action!(on_draw_class_toggled, |_s, a, index, selected| {
        if let Ok(index) = usize::try_from(index) {
            let classes = a.get_draw_classes();
            if let Some(mut entry) = classes.row_data(index) {
                entry.selected = selected;
                classes.set_row_data(index, entry);
                update_draw_class_summary(&a);
            }
        }
        Ok(())
    });
    action!(on_draw_classes_all, |_s, a, selected| {
        let classes = a.get_draw_classes();
        for index in 0..classes.row_count() {
            if let Some(mut entry) = classes.row_data(index) {
                entry.selected = selected;
                classes.set_row_data(index, entry);
            }
        }
        update_draw_class_summary(&a);
        Ok(())
    });
    action!(on_draw_classes_close, |s, a, accepted| {
        if accepted {
            let selected = a
                .get_draw_classes()
                .iter()
                .filter(|entry| entry.selected)
                .map(|entry| entry.code.to_string())
                .collect::<Vec<_>>();
            current_workspace(s)?.set_draw_classes(&selected)?;
            update_tables(s, &a)?;
            update_form(s, &a);
            status(
                &a,
                "Arvonnan luokkavalinnat tallennettu. Valitse Arvo lähtöajat nähdäksesi esikatselun.",
                false,
            );
        }
        a.set_draw_classes_open(false);
        follow_pending_result(s, &a)
    });
    action!(on_draw_starts, |s, a| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        let project = current_workspace(s)?
            .project()
            .ok_or_else(|| invalid("Aineisto on vain katseltavissa"))?;
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let draw = as2026::starts::draw(project, seed)?;
        let selected_classes = project
            .luokat
            .iter()
            .filter(|class| class.osallistuu_lahtoarvontaan)
            .map(|class| class.tunnus.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let preview = draw
            .assignments
            .iter()
            .map(|entry| {
                let class = project
                    .kilpailijat
                    .iter()
                    .find(|d| d.numero == entry.number)
                    .map_or("", |d| d.luokka.as_str());
                format!(
                    "{class}   {}   {:02}:{:02}:{:02}",
                    entry.number, entry.time.hour, entry.time.minute, entry.time.second
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        confirmation(
            s,
            &a,
            PendingAction::Draw(Box::new(draw)),
            "Lähtöaikojen arvonta",
            format!(
                "Valitut luokat: {selected_classes}\n\nLähtöajat arvotaan luokkien lähtöajoista ja lähtöväleistä. Jo luetut kortit ja tallennetut tulokset ohitetaan. Hyväksy korvaa vain alla olevien kilpailijoiden lähtöajat.\n\nLuokka   Nro   Lähtö\n{preview}"
            ),
        );
        Ok(())
    });
    action!(on_confirm_action, |s, a, accepted| {
        if accepted && a.get_import_open() && a.get_import_name().trim().is_empty() {
            a.set_import_error("Anna kilpailun nimi.".into());
            status(&a, "Anna kilpailun nimi.", true);
            return Ok(());
        }
        // Esikatselu ja ratavalinnat säilyvät myös tallennusvirheen jälkeen.
        if accepted && let Some(PendingAction::Import { preview, directory }) = s.pending.as_ref() {
            let mut preview = preview.clone();
            let directory = directory.clone();
            if !can_switch(s, &a) {
                return Ok(());
            }
            if a.get_import_only_tracks() {
                *preview = preview.tracks_only();
            }
            let project = preview.to_project(&a.get_import_name())?;
            let imported = if let Some(directory) = directory {
                Workspace::create_imported(&directory, project)?
            } else {
                let preferences = s
                    .preferences
                    .as_ref()
                    .ok_or_else(|| invalid("Tallennusasetukset puuttuvat"))?;
                let root = if a.get_import_root().trim().is_empty() {
                    preferences.competition_root().map_err(|e| invalid(&e))?
                } else {
                    PathBuf::from(a.get_import_root().as_str())
                };
                preferences
                    .create_imported_in(project, &root)
                    .map_err(|e| invalid(&e))?
            };
            let path = imported.path().to_owned();
            open_workspace(s, &a, &path, false)?;
            s.pending = None;
            a.set_confirm_open(false);
            a.set_import_open(false);
            status(&a, "Excel-kilpailu tuotu ja tallennettu.", false);
            return Ok(());
        }
        a.set_confirm_open(false);
        a.set_import_open(false);
        let Some(pending) = s.pending.take() else {
            return Ok(());
        };
        if accepted {
            let message = match pending {
                PendingAction::Review(number) => {
                    current_workspace(s)?.review_results(number)?;
                    "Tarkista: asematulokset muodostettu korttiluvusta ja tallennettu."
                }
                PendingAction::ConfirmInterruption { number, reason } => {
                    current_workspace(s)?.confirm_interruption(number, &reason)?;
                    "Keskeytys ja sen syy vahvistettu ja tallennettu."
                }
                PendingAction::Import { .. } => {
                    return Err(invalid("Avaa Excel-tuonnin esikatselu uudelleen"));
                }
                PendingAction::Notice => "Esikatselu suljettu.",
                PendingAction::Draw(draw) => {
                    current_workspace(s)?.apply_start_draw(&draw)?;
                    "Arvotut lähtöajat tallennettu."
                }
            };
            update_tables(s, &a)?;
            update_form(s, &a);
            status(&a, message, false);
        }
        follow_pending_result(s, &a)
    });
    action!(on_export_starts, |s, a| {
        if !can_leave(s, &a) {
            return Ok(());
        }
        let project = current_workspace(s)?
            .project()
            .ok_or_else(|| invalid("Lähtöluettelo edellyttää AS2026-kilpailua"))?;
        let html = as2026::starts::start_list_html(project)?;
        let csv = as2026::starts::start_list_csv(project)?;
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Tallenna lähtöluettelo")
            .add_filter("HTML-lähtöluettelo", &["html"])
            .add_filter("CSV-lähtöluettelo", &["csv"])
            .set_file_name("lahtoluettelo.html")
            .save_file()
        {
            let output = if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("csv"))
            {
                &csv
            } else {
                &html
            };
            report::save_new(&path, output)?;
            status(
                &a,
                "Lähtöluettelo tallennettu. Sen voi avata ja tulostaa selaimella.",
                false,
            );
        }
        Ok(())
    });
    action!(on_export_report, |s, a| {
        let ws = s
            .workspace
            .as_ref()
            .ok_or_else(|| invalid("Avaa ensin kilpailu"))?;
        let archive = ws.results()?;
        let ranking =
            application::rank_stored(&archive, as2026::ranking::TiePolicy::KeepSharedAndFlag)?;
        let html = if ws.is_editable() {
            report::project_ranking_html(&archive, &ranking)
        } else {
            report::ranking_html(&archive, &ranking)
        };
        let weak = a.as_weak();
        std::thread::Builder::new()
            .name("as2026-raportti".into())
            .spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Vie tulokset uutena HTML-tiedostona")
                    .set_file_name("tulokset.html")
                    .add_filter("HTML-raportti", &["html"])
                    .save_file()
                {
                    let result = report::save_new(&path, &html);
                    let message = match &result {
                        Ok(()) => format!("Raportti tallennettu: {}", path.display()),
                        Err(error) => error.to_string(),
                    };
                    let failed = result.is_err();
                    let _ = weak.upgrade_in_event_loop(move |app| status(&app, message, failed));
                }
            })
            .map_err(|e| invalid(&io_message(&e)))?;
        Ok(())
    });
    action!(on_export_csv, |s, a| {
        let ws = s
            .workspace
            .as_ref()
            .ok_or_else(|| invalid("Avaa ensin kilpailu"))?;
        let archive = ws.results()?;
        let ranking =
            application::rank_stored(&archive, as2026::ranking::TiePolicy::KeepSharedAndFlag)?;
        let csv = report::ranking_csv(&archive, &ranking);
        let weak = a.as_weak();
        std::thread::Builder::new()
            .name("as2026-csv".into())
            .spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Vie tulokset uutena CSV-tiedostona")
                    .set_file_name("tulokset.csv")
                    .add_filter("CSV-taulukko", &["csv"])
                    .save_file()
                {
                    let result = report::save_new(&path, &csv);
                    let message = match &result {
                        Ok(()) => format!("CSV tallennettu: {}", path.display()),
                        Err(error) => error.to_string(),
                    };
                    let failed = result.is_err();
                    let _ = weak.upgrade_in_event_loop(move |app| status(&app, message, failed));
                }
            })
            .map_err(|e| invalid(&io_message(&e)))?;
        Ok(())
    });
    action!(on_export_xlsx, |s, a| {
        let ws = s
            .workspace
            .as_ref()
            .ok_or_else(|| invalid("Avaa ensin kilpailu"))?;
        let archive = ws.results()?;
        let ranked =
            application::rank_stored(&archive, as2026::ranking::TiePolicy::KeepSharedAndFlag)?;
        let bytes = report::ranking_xlsx(&archive, &ranked)?;
        let weak = a.as_weak();
        std::thread::Builder::new()
            .name("as2026-xlsx".into())
            .spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Vie tulokset uutena XLSX-tiedostona")
                    .set_file_name("tulokset.xlsx")
                    .add_filter("Excel-työkirja", &["xlsx"])
                    .save_file()
                {
                    let result = report::save_new_bytes(&path, &bytes);
                    let message = match &result {
                        Ok(()) => format!("XLSX tallennettu: {}", path.display()),
                        Err(error) => error.to_string(),
                    };
                    let failed = result.is_err();
                    let _ = weak.upgrade_in_event_loop(move |app| status(&app, message, failed));
                }
            })
            .map_err(|error| invalid(&io_message(&error)))?;
        Ok(())
    });
    action!(on_import_frame, |s, a| {
        if s.worker.is_some() || !can_leave(s, &a) {
            return Ok(());
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Lue Emit-kehystiedosto")
            .add_filter("217 tavun Emit-kehys", &["raw", "bin"])
            .pick_file()
        {
            let bytes = std::fs::read(path).map_err(|e| invalid(&io_message(&e)))?;
            let frame = emit::parse_frame(&bytes)?;
            let now = chrono::Local::now();
            handle_event(
                s,
                &a,
                ReaderEvent::Frame {
                    frame: Box::new(frame),
                    read_clock: ClockTime::new(now.hour(), now.minute(), now.second())?,
                },
            );
        }
        Ok(())
    });
    {
        let state = state.clone();
        let weak = app.as_weak();
        let timer = reader_timer.clone();
        app.on_reader_start(move |port| {
            let Some(app) = weak.upgrade() else {
                return;
            };
            let mut s = state.borrow_mut();
            if app.get_update_busy()
                || app.get_install_busy()
                || !can_switch(&mut s, &app)
                || !s.workspace.as_ref().is_some_and(Workspace::is_editable)
            {
                return;
            }
            match ReaderWorker::start((!port.trim().is_empty()).then(|| port.to_string())) {
                Ok((worker, receiver)) => {
                    s.worker = Some(worker);
                    s.receiver = Some(receiver);
                    app.set_reader_running(true);
                    app.set_reader_status("Etsitään ja avataan lukijaa…".into());
                    let tick_state = state.clone();
                    let tick_weak = app.as_weak();
                    let tick_timer = Rc::downgrade(&timer);
                    timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
                        if let (Some(app), Some(timer)) =
                            (tick_weak.upgrade(), tick_timer.upgrade())
                        {
                            poll_reader(&mut tick_state.borrow_mut(), &app, &timer);
                        }
                    });
                }
                Err(error) => status(&app, error, true),
            }
        });
    }
    {
        let state = state.clone();
        let weak = app.as_weak();
        let timer = reader_timer.clone();
        app.on_reader_stop(move || {
            timer.stop();
            if let Some(app) = weak.upgrade()
                && let Err(error) = stop_reader(&mut state.borrow_mut(), &app)
            {
                status(&app, error.to_string(), true);
            }
        });
    }
    action!(on_retry_readouts, |s, a| {
        if s.worker.is_some() {
            return Err(invalid("Pysäytä lukija ennen uutta tallennusyritystä"));
        }
        current_workspace(s)?.reload()?;
        let pending = std::mem::take(&mut s.unsaved);
        for (frame, read_clock) in pending {
            handle_event(s, &a, ReaderEvent::Frame { frame, read_clock });
        }
        update_tables(s, &a)?;
        if s.unsaved.is_empty() {
            status(&a, "Odottavat lukemat tallennettu.", false);
        }
        Ok(())
    });
    {
        let state = state.clone();
        let weak = app.as_weak();
        let timer = reader_timer.clone();
        app.window().on_close_requested(move || {
            let Some(app) = weak.upgrade() else { return slint::CloseRequestResponse::HideWindow; };
            let mut s = state.borrow_mut();
            if app.get_install_busy() { return slint::CloseRequestResponse::KeepWindowShown; }
            if !can_leave(&mut s, &app) { return slint::CloseRequestResponse::KeepWindowShown; }
            timer.stop();
            if let Err(error) = stop_reader(&mut s, &app) { status(&app, error.to_string(), true); return slint::CloseRequestResponse::KeepWindowShown; }
            if !s.unsaved.is_empty() {
                status(&app, "Lukemia odottaa tallennusta. Käytä Emit-näkymän uudelleenyritystä ennen sulkemista.", true);
                return slint::CloseRequestResponse::KeepWindowShown;
            }
            slint::CloseRequestResponse::HideWindow
        });
    }
}

fn save_view(app: &AppWindow, path: &Path) -> Result<(), String> {
    let buffer = app.window().take_snapshot().map_err(|e| e.to_string())?;
    image::save_buffer(
        path,
        buffer.as_bytes(),
        buffer.width(),
        buffer.height(),
        image::ColorType::Rgba8,
    )
    .map_err(|e| e.to_string())
}

fn capture_views(
    app: &AppWindow,
    state: SharedState,
    directory: PathBuf,
    keyboard_test: bool,
    lifecycle_path: Option<PathBuf>,
) -> GuiResult<()> {
    std::fs::create_dir_all(&directory).map_err(|e| invalid(&io_message(&e)))?;
    let weak = app.as_weak();
    let timer = Rc::new(Timer::default());
    let tick_timer = Rc::downgrade(&timer);
    // Leaked owner is unnecessary: callback keeps the timer alive until the last snapshot.
    let owner = Rc::new(RefCell::new(Some(timer.clone())));
    let phase = Rc::new(RefCell::new(0usize));
    timer.start(TimerMode::Repeated, Duration::from_millis(900), move || {
        let Some(app) = weak.upgrade() else {
            return;
        };
        let current = *phase.borrow();
        if current == 6
            && !keyboard_test
            && let Err(error) = save_view(&app, &directory.join("1b-kilpailijat-vieritetty.png"))
        {
            eprintln!("Kuvakaappaus epäonnistui: {error}");
        }
        if current >= if keyboard_test { 45 } else { 24 } {
            if keyboard_test && app.get_about_open() {
                smoke::key(&app, slint::platform::Key::Escape);
                assert!(!app.get_about_open(), "Esc sulkee Tietoja-ikkunan");
            }
            if let Some(timer) = tick_timer.upgrade() {
                timer.stop();
            }
            owner.borrow_mut().take();
            let _ = slint::quit_event_loop();
            return;
        }
        if current >= 31 {
            match current {
                31 => {
                    if app.get_about_open() {
                        smoke::key(&app, slint::platform::Key::Escape);
                    }
                    if let Some(path) = &lifecycle_path {
                        open_workspace(&mut state.borrow_mut(), &app, path, true).unwrap();
                        app.invoke_navigate(3);
                        app.invoke_select_row(0);
                        app.invoke_edit_result_competitor();
                    }
                }
                32 => save_view(&app, &directory.join("10-kilpailu-valmis.png")).unwrap(),
                33 => {
                    app.set_competitor_details_open(true);
                    let scale = app.window().scale_factor();
                    let size = app.window().size();
                    app.window()
                        .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                            position: slint::LogicalPosition::new(
                                size.width as f32 / scale - 60.,
                                size.height as f32 / scale * 0.6,
                            ),
                            delta_x: 0.,
                            delta_y: -3000.,
                        });
                }
                34 => save_view(&app, &directory.join("11-kilpailijan-tiedot-alas.png")).unwrap(),
                35 => {
                    app.invoke_navigate(4);
                    app.invoke_configure_draw_classes();
                }
                36 => save_view(&app, &directory.join("12-arvonnan-luokat.png")).unwrap(),
                37 => {
                    let before = app.get_draw_classes().row_data(0).unwrap().selected;
                    for _ in 0..4 {
                        smoke::key(&app, slint::platform::Key::Tab);
                    }
                    smoke::key(&app, slint::platform::Key::Space);
                    assert_ne!(
                        app.get_draw_classes().row_data(0).unwrap().selected,
                        before,
                        "Luokan valintaruutu toimii Tabilla ja välilyönnillä"
                    );
                    smoke::key(&app, slint::platform::Key::Escape);
                    assert!(!app.get_draw_classes_open());
                    println!(
                        "GUI-luokkavalinnan näppäimistökoe hyväksytty: Tab, valintaruutu ja Esc."
                    );
                }
                38 => {
                    app.invoke_navigate(0);
                }
                39 => {
                    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("fixtures/excel-import/synteettinen.xlsx");
                    let mut preview = as2026::excel_import::read(&source).unwrap();
                    for driver in &mut preview.kilpailijat {
                        driver.tiedot.emit = 0;
                        driver.tiedot.lahtoaika.clear();
                    }
                    preview.kilpailijat[0].tiedot.luokka = "SM-A".into();
                    offer_excel_import(&mut state.borrow_mut(), &app, &source, &preview, None)
                        .unwrap();
                }
                40 => save_view(&app, &directory.join("13-excel-kilpailijat.png")).unwrap(),
                41 => {
                    app.set_import_tab(1);
                }
                42 => save_view(&app, &directory.join("14-excel-radat.png")).unwrap(),
                43 => {
                    app.invoke_confirm_action(false);
                }
                _ => {}
            }
            *phase.borrow_mut() += 1;
            return;
        }
        if current == 24 {
            smoke::open_dialog(&app);
        } else if current == 25 {
            smoke::fill_dialog(&app);
        } else if current == 27 {
            smoke::key(&app, slint::platform::Key::Escape);
            assert!(!app.get_dialog_open(), "Esc sulkee valintaikkunan");
            println!("GUI-näppäimistökoe hyväksytty: Ctrl+N, tekstisyöte, Tab ja Esc.");
        } else if current == 28 {
            app.set_about_open(true);
        } else if current == 29 {
            app.window().request_redraw();
        }
        if current >= 24 && current != 26 && current != 30 {
            *phase.borrow_mut() += 1;
            return;
        }
        if current < 24 && current.is_multiple_of(3) {
            if let Err(error) = navigate(&mut state.borrow_mut(), &app, (current / 3) as i32) {
                eprintln!("{error}");
            }
            if keyboard_test && current == 6 {
                show_read_notice(
                    &mut state.borrow_mut(),
                    &app,
                    "✓ Emit 123456 luettu · #7 Esimerkkikuljettaja".into(),
                );
            }
        } else if current < 24 && current % 3 == 1 {
            if current == 10 || current == 16 {
                app.invoke_select_detail(1);
            }
            if current == 16 && !keyboard_test {
                app.window()
                    .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                        position: slint::LogicalPosition::new(225., 400.),
                        delta_x: 0.,
                        delta_y: -600.,
                    });
            }
            app.window().request_redraw();
        } else {
            let index = if current == 30 { 9 } else { current / 3 };
            let names = [
                "yleiskatsaus",
                "kilpailijat",
                "emit",
                "tulokset",
                "luokat",
                "radat",
                "asetukset",
                "paivitykset",
                "uusi-kilpailu",
                "tietoja",
            ];
            let result = save_view(
                &app,
                &directory.join(format!("{index}-{}.png", names[index])),
            );
            if let Err(error) = result {
                eprintln!("Kuvakaappaus epäonnistui: {error}");
            }
            if current == 5 && !keyboard_test {
                app.window()
                    .dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                        position: slint::LogicalPosition::new(250., 275.),
                        delta_x: -250.,
                        delta_y: 0.,
                    });
                app.window().request_redraw();
            }
        }
        *phase.borrow_mut() += 1;
    });
    Ok(())
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut initial_path = None;
    let mut demo = false;
    let mut smoke_test = false;
    let mut captures = None;
    let mut health_check = None;
    let mut install_requested = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--demo") => demo = true,
            Some("--install") => install_requested = true,
            Some("--health-check") => {
                health_check = Some(
                    args.next()
                        .ok_or("Käynnistyskuittauksen tunnus puuttuu")?
                        .into_string()
                        .map_err(|_| "Käynnistyskuittaus on virheellinen")?,
                )
            }
            Some("--smoke-test") => smoke_test = true,
            Some("--capture-dir") => {
                captures = Some(PathBuf::from(
                    args.next().ok_or("Kuvakansion polku puuttuu")?,
                ))
            }
            Some("--help" | "ohje") => {
                println!(
                    "AS2026-käyttöliittymä\n  as2026-gui [kilpailu.toml tai kilpailukansio]\n  as2026-gui --demo\n  as2026-gui --demo --capture-dir KANSIO"
                );
                return Ok(());
            }
            _ if initial_path.is_none() => initial_path = Some(PathBuf::from(arg)),
            _ => return Err("Tuntematon lisävalinta".into()),
        }
    }
    if demo && initial_path.is_some() {
        return Err("Valitse joko esimerkkikilpailu tai oma kilpailu".into());
    }
    if smoke_test && !demo {
        return Err("Työnkulkukoe tarvitsee --demo-valinnan".into());
    }
    if captures.is_some() {
        // Kuvataan nykyinen näkymä suoraan ohjelmistorenderöinnistä, ei edellistä GL-puskuria.
        slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .select()?;
    }
    let app = AppWindow::new()?;
    slint::select_bundled_translation("fi").map_err(|_| "Suomennoksen avaaminen epäonnistui")?;
    app.set_app_version(env!("CARGO_PKG_VERSION").into());
    app.set_managed(installation::is_managed());
    let preferences = if demo {
        let root = demo_path()?.parent().unwrap().to_owned();
        PreferencesStore::open_at(
            &root.join("sovellusasetukset.toml"),
            &root.join("Kilpailut"),
        )
    } else {
        PreferencesStore::open()
    };
    let preferences = match preferences {
        Ok(preferences) => {
            if let Ok(root) = preferences.competition_root() {
                app.set_default_root(root.display().to_string().into());
            }
            Some(preferences)
        }
        Err(error) => {
            status(&app, error, true);
            None
        }
    };
    let package = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_owned))
        .filter(|p| p.join("release.json").is_file());
    app.set_installable(package.is_some());
    app.set_setup_open((package.is_some() && !app.get_managed() && !demo) || install_requested);
    let previous = preferences
        .as_ref()
        .and_then(|p| p.get().last_competition.clone());
    let state = Rc::new(RefCell::new(State {
        selected: -1,
        detail: -1,
        preferences,
        install_bundle: package,
        ..Default::default()
    }));
    let reader_timer = Rc::new(Timer::default());
    bind(&app, &state, &reader_timer);
    let update_timer = Rc::new(Timer::default());
    desktop::bind(&state, &app, &update_timer, &reader_timer);
    let path = if demo {
        Some(demo_path()?)
    } else {
        initial_path.or(previous)
    };
    if let Some(path) = path
        && let Err(error) = open_workspace(&mut state.borrow_mut(), &app, &path, demo)
    {
        status(&app, error.to_string(), true);
    }
    if let Some(error) = installation::take_startup_error()? {
        status(&app, error, true);
    }
    app.show()?;
    if let Some(token) = health_check {
        let weak = app.as_weak();
        Timer::single_shot(Duration::from_millis(150), move || {
            if let Err(error) = installation::acknowledge_startup(&token) {
                if let Some(app) = weak.upgrade() {
                    status(&app, error, true);
                }
                let _ = slint::quit_event_loop();
            }
        });
    }
    let lifecycle_path = if smoke_test {
        Some(smoke::run(&app, &state)?)
    } else {
        None
    };
    if let Some(directory) = captures {
        capture_views(&app, state.clone(), directory, smoke_test, lifecycle_path)?;
    }
    slint::run_event_loop()?;
    update_timer.stop();
    reader_timer.stop();
    stop_reader(&mut state.borrow_mut(), &app)?;
    Ok(())
}
