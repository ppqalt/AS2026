//! Työpöytäasetukset ja päivitysten taustatyö. Kilpailun käyttö ei tarvitse verkkoa.
use super::*;

fn begin_task(
    state: &SharedState,
    app: &AppWindow,
    timer: &Rc<Timer>,
    reader_timer: &Rc<Timer>,
    operation: impl FnOnce() -> UpdateEvent + Send + 'static,
) -> GuiResult<()> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("as2026-paivitys".into())
        .spawn(move || {
            let _ = sender.send(operation());
        })
        .map_err(|e| invalid(&io_message(&e)))?;
    state.borrow_mut().update_receiver = Some(receiver);
    app.set_update_busy(true);
    let state = state.clone();
    let weak = app.as_weak();
    let weak_timer = Rc::downgrade(timer);
    let reader_timer = reader_timer.clone();
    timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
        if let (Some(app), Some(timer)) = (weak.upgrade(), weak_timer.upgrade()) {
            poll(&state, &app, &timer, &reader_timer);
        }
    });
    Ok(())
}
fn handoff(
    state: &SharedState,
    app: &AppWindow,
    timer: &Rc<Timer>,
    reader_timer: &Rc<Timer>,
    bundle: PathBuf,
) -> GuiResult<()> {
    {
        let mut s = state.borrow_mut();
        if !can_leave(&mut s, app) {
            return Err(invalid(
                "Hyväksy tai peruuta keskeneräinen muutos ennen päivitystä",
            ));
        }
        reader_timer.stop();
        stop_reader(&mut s, app)?;
        if !s.unsaved.is_empty() {
            return Err(invalid("Tallenna odottavat Emit-lukemat ennen päivitystä"));
        }
    }
    app.set_setup_open(false);
    app.set_install_busy(true);
    app.set_editable(false);
    let result = begin_task(state, app, timer, reader_timer, move || {
        UpdateEvent::Handoff(installation::spawn_installer(&bundle))
    });
    if result.is_err() {
        app.set_install_busy(false);
        let _ = update_header(&state.borrow(), app);
    }
    result
}
fn failed(app: &AppWindow, error: String) {
    app.set_update_status(error.clone().into());
    status(app, error, true);
}
fn poll(state: &SharedState, app: &AppWindow, timer: &Rc<Timer>, reader_timer: &Rc<Timer>) {
    let event = state
        .borrow()
        .update_receiver
        .as_ref()
        .and_then(|receiver| receiver.try_recv().ok());
    let Some(event) = event else {
        return;
    };
    timer.stop();
    state.borrow_mut().update_receiver = None;
    app.set_update_busy(false);
    match event {
        UpdateEvent::Checked(Ok(updates::UpdateCheck::NoRelease)) => {
            state.borrow_mut().update = None;
            state.borrow_mut().update_bundle = None;
            app.set_available_version("".into());
            let message = "GitHubissa ei vielä ole vakaata julkaisua.";
            app.set_update_status(message.into());
            status(app, message, false);
        }
        UpdateEvent::Checked(Ok(updates::UpdateCheck::Current)) => {
            state.borrow_mut().update = None;
            app.set_available_version("".into());
            app.set_update_status("Ohjelma on ajan tasalla.".into());
            status(app, "Ohjelma on ajan tasalla.", false);
        }
        UpdateEvent::Checked(Ok(updates::UpdateCheck::Available(update))) => {
            app.set_available_version(update.version.clone().into());
            app.set_update_status(format!("Uusi versio {} on saatavilla.", update.version).into());
            status(
                app,
                format!(
                    "Versio {} on saatavilla. Asenna ja käynnistä uudelleen sivupalkista.",
                    update.version
                ),
                false,
            );
            state.borrow_mut().update = Some(update);
        }
        UpdateEvent::Downloaded(Ok(bundle)) => {
            state.borrow_mut().update_bundle = Some(bundle.clone());
            if let Err(error) = handoff(state, app, timer, reader_timer, bundle) {
                failed(app, error.to_string());
            }
        }
        UpdateEvent::Handoff(Ok(())) => {
            let _ = slint::quit_event_loop();
        }
        UpdateEvent::Handoff(Err(error)) => {
            app.set_install_busy(false);
            let _ = update_header(&state.borrow(), app);
            failed(app, error);
        }
        UpdateEvent::Checked(Err(error)) | UpdateEvent::Downloaded(Err(error)) => {
            failed(app, error)
        }
    }
}
pub(super) fn bind(
    state: &SharedState,
    app: &AppWindow,
    timer: &Rc<Timer>,
    reader_timer: &Rc<Timer>,
) {
    {
        let (state, weak, timer, reader_timer) = (
            state.clone(),
            app.as_weak(),
            timer.clone(),
            reader_timer.clone(),
        );
        app.on_check_updates(move || {
            let Some(app) = weak.upgrade() else {
                return;
            };
            if app.get_update_busy() || app.get_install_busy() {
                return;
            }
            state.borrow_mut().update_bundle = None;
            app.set_update_status("Tarkistetaan päivityksiä".into());
            if let Err(error) = begin_task(&state, &app, &timer, &reader_timer, || {
                UpdateEvent::Checked(updates::check())
            }) {
                failed(&app, error.to_string());
            }
        });
    }
    {
        let (state, weak, timer, reader_timer) = (
            state.clone(),
            app.as_weak(),
            timer.clone(),
            reader_timer.clone(),
        );
        app.on_apply_update(move || {
            let Some(app) = weak.upgrade() else {
                return;
            };
            if app.get_update_busy() || app.get_install_busy() {
                return;
            }
            let prepared = state.borrow().update_bundle.clone();
            if let Some(bundle) = prepared {
                if let Err(error) = handoff(&state, &app, &timer, &reader_timer, bundle) {
                    failed(&app, error.to_string());
                }
                return;
            }
            let Some(update) = state.borrow().update.clone() else {
                return;
            };
            app.set_update_status(format!("Ladataan versiota {}", update.version).into());
            if let Err(error) = begin_task(&state, &app, &timer, &reader_timer, move || {
                UpdateEvent::Downloaded(updates::download(&update))
            }) {
                failed(&app, error.to_string());
            }
        });
    }
    {
        let (state, weak, timer, reader_timer) = (
            state.clone(),
            app.as_weak(),
            timer.clone(),
            reader_timer.clone(),
        );
        app.on_install_app(move || {
            let Some(app) = weak.upgrade() else {
                return;
            };
            if app.get_update_busy() || app.get_install_busy() {
                return;
            }
            let bundle = state.borrow().install_bundle.clone();
            if let Some(bundle) = bundle
                && let Err(error) = handoff(&state, &app, &timer, &reader_timer, bundle)
            {
                failed(&app, error.to_string());
            }
        });
    }
}
