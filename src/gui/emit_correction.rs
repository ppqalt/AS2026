use super::*;
use as2026::emit_corrections;

pub(super) struct Basis {
    number: u32,
    path: PathBuf,
    content: Vec<as2026::Punch>,
}
fn current_content(project: &Project, number: u32) -> GuiResult<Vec<as2026::Punch>> {
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailija puuttuu"))?;
    let read = project
        .active_readout(driver.emit)
        .ok_or_else(|| invalid("Korttiluku puuttuu"))?;
    emit_corrections::effective_readout(project, read)?.content()
}
pub(super) fn open(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    if !can_leave(state, app) {
        return Ok(());
    }
    let project = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .ok_or_else(|| invalid("Avaa kilpailu"))?;
    let number = if state.page == 2 {
        let read = usize::try_from(state.selected)
            .ok()
            .and_then(|i| project.leimaluvut.get(i))
            .ok_or_else(|| invalid("Valitse lukutapahtuma"))?;
        project
            .kilpailijat
            .iter()
            .find(|d| d.emit == read.emit)
            .ok_or_else(|| invalid("Kortin kilpailija puuttuu"))?
            .numero
    } else {
        selected_number(state)?
    };
    let ws = current_workspace(state)?;
    ws.pin_lan_basis();
    let project = ws.project().unwrap();
    let content = current_content(project, number)?;
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .unwrap();
    app.set_emit_edit_identity(
        format!(
            "Nro {number} · {} · Emit {}",
            driver.kuljettaja, driver.emit
        )
        .into(),
    );
    app.set_emit_edit_fields(model(forms::emit_card_fields(project, number, &content)?));
    app.set_emit_edit_history("".into());
    app.set_emit_edit_error("".into());
    app.set_emit_edit_open(true);
    state.emit_edit_basis = Some(Basis {
        number,
        path: ws.path().into(),
        content,
    });
    Ok(())
}
pub(super) fn edit(_state: &mut State, app: &AppWindow, key: i32, value: &str) -> GuiResult<()> {
    let fields = app.get_emit_edit_fields();
    if let Some(i) = fields.iter().position(|f| f.key == key) {
        let mut f = fields.row_data(i).unwrap();
        f.value = value.into();
        if f.duration {
            let parts = value.split(':').collect::<Vec<_>>();
            if let [h, m, s] = parts.as_slice() {
                f.hours = (*h).into();
                f.minutes = (*m).into();
                f.seconds = (*s).into();
            }
        }
        f.choice = f
            .values
            .iter()
            .position(|v| v == value)
            .map_or(-1, |i| i as i32);
        fields.set_row_data(i, f);
    }
    app.set_emit_edit_error("".into());
    Ok(())
}
pub(super) fn add(state: &mut State, app: &AppWindow, restore: bool) -> GuiResult<()> {
    let basis = state
        .emit_edit_basis
        .as_ref()
        .ok_or_else(|| invalid("Avaa leimat uudelleen"))?;
    let project = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .ok_or_else(|| invalid("Kilpailu puuttuu"))?;
    if restore {
        let driver = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == basis.number)
            .unwrap();
        let content = project.active_readout(driver.emit).unwrap().content()?;
        app.set_emit_edit_fields(model(forms::emit_card_fields(
            project,
            basis.number,
            &content,
        )?));
    } else {
        let mut fields = app.get_emit_edit_fields().iter().collect::<Vec<_>>();
        let key = fields.len() as i32;
        let mut added = forms::emit_card_fields(
            project,
            basis.number,
            &[
                as2026::Punch {
                    code: 0,
                    elapsed_seconds: 0,
                },
                as2026::Punch {
                    code: 1,
                    elapsed_seconds: 0,
                },
            ],
        )?;
        added[0].label = format!("Uusi leima {} · Emit-koodi", key / 3 + 1).into();
        added[0].value = "".into();
        added[1].value = "".into();
        added[1].hours = "".into();
        added[1].minutes = "".into();
        added[1].seconds = "".into();
        for field in &mut added {
            field.key += key;
        }
        fields.extend(added);
        app.set_emit_edit_fields(model(fields));
    }
    app.set_emit_edit_error("".into());
    Ok(())
}
pub(super) fn close(state: &mut State, app: &AppWindow, save: bool) -> GuiResult<()> {
    if !save {
        state.emit_edit_basis = None;
        app.set_emit_edit_open(false);
        return follow_pending_result(state, app);
    }
    let basis = state
        .emit_edit_basis
        .as_ref()
        .ok_or_else(|| invalid("Avaa leimat uudelleen"))?;
    let ws = state
        .workspace
        .as_ref()
        .ok_or_else(|| invalid("Kilpailu puuttuu"))?;
    let project = ws.project().unwrap();
    if basis.path != ws.path() || current_content(project, basis.number)? != basis.content {
        return Err(invalid(
            "Korttiluku on muuttunut muokkauksen aikana. Avaa leimat uudelleen",
        ));
    }
    let origin = emit_corrections::punch_clock(project, basis.number, 0)?;
    let fields = app.get_emit_edit_fields().iter().collect::<Vec<_>>();
    let mut punches = Vec::new();
    for (i, fields) in fields.as_chunks::<3>().0.iter().enumerate() {
        if fields[2].value == "1" {
            continue;
        }
        let code = fields[0]
            .value
            .trim()
            .parse::<u32>()
            .map_err(|_| invalid(&format!("Leima {}: anna Emit-koodi", i + 1)))?;
        let clock = as2026::terminal::parse_clock(&fields[1].value)?.seconds_since_midnight()?;
        let elapsed = (clock - origin).rem_euclid(86_400) as u32;
        punches.push(native::ReadPunch {
            koodi: code,
            aika_sekuntia: elapsed,
        });
    }
    // Card order follows the supplied clocks, so a missing punch can be added anywhere in the editor.
    punches.sort_by_key(|p| p.aika_sekuntia);
    let number = basis.number;
    current_workspace(state)?.replace_emit_content(number, punches)?;
    app.set_emit_edit_open(false);
    state.emit_edit_basis = None;
    state.detail = -1;
    app.set_detail_key(-1);
    update_tables(state, app)?;
    update_form(state, app);
    status(
        app,
        "Leimat tallennettu ja tulokset laskettu uudelleen.",
        false,
    );
    follow_pending_result(state, app)
}

/// Todellinen ikkunallinen koe vain --demo-ajon omaan synteettiseen kilpailuun.
pub(super) fn capture(app: &AppWindow, state: SharedState, directory: PathBuf) -> GuiResult<()> {
    std::fs::create_dir_all(&directory).map_err(|e| invalid(&io_message(&e)))?;
    let raw = {
        let mut state = state.borrow_mut();
        let ws = current_workspace(&mut state)?;
        let p = ws.project().unwrap();
        let config = as2026::beginners::BeginnersCup {
            nimi: "Synteettinen BC".into(),
            tunnus: "BC".into(),
            lahderata: 1,
            lahdeasemat: p.radat[0].asemat.clone(),
            mukana_asemat: vec![1, 2, 3, 4],
        };
        let raw = p.leimaluvut.clone();
        ws.save_beginners_cup(config, &[7])?;
        ws.ensure_readout_result(7)?;
        for bc in [false, true] {
            ws.set_bc_view(bc)?;
            let finish = ws
                .result_snapshot(7)?
                .unwrap()
                .rivit
                .iter()
                .find(|r| r.station_type == "M")
                .unwrap()
                .output_index;
            ws.edit_station_result(7, finish, as2026::workspace::ResultField::Clock, "12:10:00")?;
            ws.recalculate_stored(7)?;
            ws.approve_result(7)?;
        }
        ws.set_bc_view(false)?;
        raw
    };
    let weak = app.as_weak();
    let timer = Rc::new(Timer::default());
    let tick_timer = Rc::downgrade(&timer);
    let owner = Rc::new(RefCell::new(Some(timer.clone())));
    let phase = Rc::new(RefCell::new(0_u32));
    timer.start(TimerMode::Repeated, Duration::from_millis(450), move || {
        let Some(app) = weak.upgrade() else { return; };
        match *phase.borrow() {
            0 => { app.invoke_navigate(2); app.invoke_select_row(0); app.invoke_open_emit_edit(); assert!(app.get_emit_edit_open()); }
            1 => {
                save_view(&app, &directory.join("01-kaikki-leimat.png")).unwrap();
                assert_eq!(app.get_emit_edit_fields().row_count(),9);
                app.invoke_emit_edit_edited(0,"250".into());
                app.invoke_emit_edit_close(true); assert!(app.get_emit_edit_open());
            }
            2 => {
                app.invoke_emit_edit_edited(0,"31".into());
                app.invoke_emit_edit_edited(1,"12:05:00".into());
                app.invoke_emit_edit_edited(5,"1".into()); // remove arbitrary extra punch
                app.invoke_emit_edit_add(false);
                app.invoke_emit_edit_edited(9,"99".into());
                app.invoke_emit_edit_edited(10,"12:05:01".into());
                save_view(&app,&directory.join("02-korjattu-kortti.png")).unwrap();
                app.invoke_emit_edit_close(true);
                assert!(!app.get_emit_edit_open(),"{}",app.get_emit_edit_error());
                let state=state.borrow(); let p=state.workspace.as_ref().unwrap().project().unwrap();
                assert_eq!(p.leimaluvut,raw);
                for result in [p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap(),p.bc_osallistujat[0].tallennettu_tulos.as_ref().unwrap()] {
                    assert_eq!((result.aikavirheet,result.asemien_muut_virheet),(0,0));
                    assert!(!result.hyvaksytty);
                }
            }
            3 => {
                app.invoke_open_emit_edit(); assert!(app.get_emit_edit_open());
                app.invoke_emit_edit_add(true); assert_eq!(app.get_emit_edit_fields().row_data(1).unwrap().value,"12:04:35");
                app.invoke_emit_edit_close(false);
                app.invoke_open_emit_edit();
                app.window().set_size(slint::LogicalSize::new(900.,600.));
            }
            4 => {
                save_view(&app,&directory.join("03-pieni-ikkuna.png")).unwrap();
                app.invoke_emit_edit_close(false);
                let state=state.borrow();let ws=state.workspace.as_ref().unwrap();
                let reopened=Workspace::open(ws.path()).unwrap();assert_eq!(reopened.project().unwrap().leimaluvut,raw);
                println!("0.6.9 Emit-editori: kaikki leimat, kellosta laskettu korttiaika, lisäys, poisto, 99, virheellinen koodi, NAC/BC, peruutus ja uudelleenavaus hyväksytty.");
            }
            5 => { app.invoke_open_emit_edit(); assert!(app.get_emit_edit_open()); }
            6 | 7 => { /* Odota mallin ja ensimmäisen kentän kohdistuksen asettumista. */ }
            8 => {
                app.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Control.into() });
                smoke::key(&app, "a");
                app.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: slint::platform::Key::Control.into() });
                smoke::key(&app, "42");
                assert_eq!(app.get_emit_edit_fields().row_data(0).unwrap().value, "42", "Koe kirjoittaa ensin Emit-koodikenttään: {:?}", app.get_emit_edit_fields().iter().map(|f| f.value.to_string()).collect::<Vec<_>>());
            }
            9 | 12 => { smoke::key(&app, slint::platform::Key::Return); }
            10 => {
                save_view(&app, &directory.join("04-enterin-jalkeen.png")).unwrap();
                smoke::key(&app, slint::platform::Key::Tab);
            }
            11 => {
                app.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Control.into() });
                smoke::key(&app, "a");
                app.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: slint::platform::Key::Control.into() });
                smoke::key(&app, "13");
                assert!(app.get_emit_edit_fields().row_data(1).unwrap().value.starts_with("13:"), "Enter ja Tab siirtyvät seuraavalle kelloriville: {:?}", app.get_emit_edit_fields().iter().map(|f| f.value.to_string()).collect::<Vec<_>>());
            }
            13 => {
                app.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Shift.into() });
                smoke::key(&app, slint::platform::Key::Tab);
                app.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: slint::platform::Key::Shift.into() });
            }
            14 => {
                app.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Control.into() });
                smoke::key(&app, "a");
                app.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: slint::platform::Key::Control.into() });
                smoke::key(&app, "43");
                assert_eq!(app.get_emit_edit_fields().row_data(0).unwrap().value, "43", "Enter ja Shift+Tab palaavat edelliselle kentälle");
                app.invoke_emit_edit_close(false);
                println!("Enterin jälkeinen Tab/Shift+Tab hyväksytty oikealla näppäinsyötteellä.");
                if let Some(timer)=tick_timer.upgrade(){timer.stop();} owner.borrow_mut().take();let _=slint::quit_event_loop();
            }
            _ => unreachable!(),
        }
        *phase.borrow_mut()+=1;
    });
    Ok(())
}
