use super::*;
use as2026::emit_corrections::{self, Action, Request};

pub(super) struct Basis {
    number: u32,
    row: u32,
    path: PathBuf,
    content: Vec<as2026::Punch>,
}
fn current_content(project: &Project, number: u32) -> GuiResult<Vec<as2026::Punch>> {
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    let original = project
        .active_readout(driver.emit)
        .ok_or_else(|| invalid("Kilpailijan korttiluku puuttuu"))?;
    emit_corrections::effective_readout(project, original)?.content()
}

pub(super) fn open(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    if !can_leave(state, app) {
        return Ok(());
    }
    let Selection::ResultStation { number, row } = state.selection else {
        return Err(invalid("Valitse ensin korjattava asemarivi"));
    };
    let ws = current_workspace(state)?;
    if ws.bc_view() {
        return Err(invalid(
            "Korjaa yhteinen leima NAC / perusluokat -näkymässä",
        ));
    }
    let project = ws
        .project()
        .ok_or_else(|| invalid("Avaa muokattava kilpailu"))?;
    let fields = forms::emit_correction_fields(project, number, row)?;
    let selected = emit_corrections::rows(project, number)?
        .into_iter()
        .find(|r| r.index == row)
        .ok_or_else(|| invalid("Asemariviä ei löydy"))?;
    let snapshot = ws
        .result_snapshot(number)?
        .ok_or_else(|| invalid("Lue ensin kilpailijan kortti"))?;
    if snapshot
        .rivit
        .iter()
        .find(|r| r.output_index == row)
        .is_none_or(|r| r.sequence != selected.sequence || r.received_code != selected.code)
    {
        return Err(invalid(
            "Kortin kohdistus on muuttunut. Valitse ensin Tarkista ja sen jälkeen korjattava asemarivi",
        ));
    }
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .unwrap();
    let history = project
        .emit_korjaukset
        .iter()
        .filter(|e| {
            project
                .leimaluvut
                .get(e.lahdelukema.saturating_sub(1))
                .is_some_and(|r| r.emit == driver.emit)
        })
        .rev()
        .take(5)
        .map(|e| {
            let describe = |p: &[native::ReadPunch]| {
                p.iter()
                    .skip(1)
                    .map(|p| format!("{}@{}s", p.koodi, p.aika_sekuntia))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            format!(
                "{}: {}\n{} → {}",
                e.asema,
                e.syy,
                describe(&e.ennen),
                describe(&e.jalkeen)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let basis = Basis {
        number,
        row,
        path: ws.path().into(),
        content: current_content(project, number)?,
    };
    app.set_emit_edit_identity(
        format!("Nro {number} · {} · {}", driver.kuljettaja, selected.name).into(),
    );
    app.set_emit_edit_fields(model(fields));
    app.set_emit_edit_history(history.into());
    app.set_emit_edit_error("".into());
    app.set_emit_edit_open(true);
    state.emit_edit_basis = Some(basis);
    Ok(())
}

fn set_field(app: &AppWindow, key: i32, value: &str) {
    let fields = app.get_emit_edit_fields();
    if let Some(index) = fields.iter().position(|f| f.key == key) {
        let mut field = fields.row_data(index).unwrap();
        field.value = value.into();
        field.choice = field
            .values
            .iter()
            .position(|v| v == value)
            .map_or(-1, |i| i as i32);
        if field.duration {
            let parts: Vec<_> = value.split(':').collect();
            if let [h, m, s] = parts.as_slice() {
                field.hours = (*h).into();
                field.minutes = (*m).into();
                field.seconds = (*s).into();
            }
        }
        fields.set_row_data(index, field);
    }
}
pub(super) fn edit(state: &mut State, app: &AppWindow, key: i32, value: &str) -> GuiResult<()> {
    let previous_mode = app
        .get_emit_edit_fields()
        .iter()
        .find(|f| f.key == 0)
        .map(|f| f.value.to_string());
    set_field(app, key, value);
    app.set_emit_edit_error("".into());
    if key == 0 && previous_mode.as_deref() != Some(value) {
        let basis = state
            .emit_edit_basis
            .as_ref()
            .ok_or_else(|| invalid("Avaa korjaus uudelleen"))?;
        let project = state
            .workspace
            .as_ref()
            .and_then(Workspace::project)
            .ok_or_else(|| invalid("Kilpailu puuttuu"))?;
        let reason = app
            .get_emit_edit_fields()
            .iter()
            .find(|f| f.key == 4)
            .map(|f| f.value.to_string())
            .unwrap_or_default();
        let mut fields = forms::emit_correction_fields(project, basis.number, basis.row)?;
        if value != "0" {
            fields.retain(|f| matches!(f.key, 0 | 4));
        }
        app.set_emit_edit_fields(model(fields));
        set_field(app, 0, value);
        set_field(app, 4, &reason);
    }
    if key == 1
        && let Ok(source) = value.parse::<u32>()
        && source != 0
    {
        let basis = state
            .emit_edit_basis
            .as_ref()
            .ok_or_else(|| invalid("Avaa korjaus uudelleen"))?;
        let project = state
            .workspace
            .as_ref()
            .and_then(Workspace::project)
            .ok_or_else(|| invalid("Kilpailu puuttuu"))?;
        if let Some(row) = emit_corrections::rows(project, basis.number)?
            .iter()
            .find(|r| r.index == source)
        {
            // Kohdistus käyttää valitun lisäleiman aikaa; koodin toimitsija vahvistaa.
            set_field(
                app,
                3,
                &format!(
                    "{:02}:{:02}:{:02}",
                    row.elapsed_seconds / 3600,
                    row.elapsed_seconds % 3600 / 60,
                    row.elapsed_seconds % 60
                ),
            );
        }
    }
    Ok(())
}
pub(super) fn close(state: &mut State, app: &AppWindow, save: bool) -> GuiResult<()> {
    if !save {
        state.emit_edit_basis = None;
        app.set_emit_edit_open(false);
        return Ok(());
    }
    let basis = state
        .emit_edit_basis
        .as_ref()
        .ok_or_else(|| invalid("Avaa korjaus uudelleen"))?;
    let ws = state
        .workspace
        .as_ref()
        .ok_or_else(|| invalid("Kilpailu puuttuu"))?;
    if basis.path != ws.path()
        || current_content(
            ws.project().ok_or_else(|| invalid("Kilpailu puuttuu"))?,
            basis.number,
        )? != basis.content
    {
        return Err(invalid(
            "Korttiluku on muuttunut korjauksen aikana. Sulje ikkuna ja tarkista uusi lukema",
        ));
    }
    let values = app
        .get_emit_edit_fields()
        .iter()
        .map(|f| (f.key, f.value.to_string()))
        .collect::<BTreeMap<_, _>>();
    let value = |key| values.get(&key).map_or("", String::as_str);
    let action = match value(0) {
        "0" => Action::Edit,
        "1" => Action::Remove,
        "2" => Action::Restore,
        _ => return Err(invalid("Valitse korjauksen tapa")),
    };
    let parse = |value: &str| {
        value
            .trim()
            .parse::<u32>()
            .map_err(|_| invalid("Anna kelvollinen kokonaisluku"))
    };
    let (code, elapsed_seconds, source_row) = if action == Action::Edit {
        let parts = value(3)
            .split(':')
            .map(parse)
            .collect::<Result<Vec<_>, _>>()?;
        let [h, m, s] = parts.as_slice() else {
            return Err(invalid("Anna korttiaika muodossa hh:mm:ss"));
        };
        if *m >= 60 || *s >= 60 {
            return Err(invalid("Minuuttien ja sekuntien on oltava 0–59"));
        }
        let elapsed = h
            .checked_mul(3600)
            .and_then(|n| n.checked_add(m * 60))
            .and_then(|n| n.checked_add(*s))
            .ok_or_else(|| invalid("Korttiaika on liian suuri"))?;
        (
            parse(value(2))?,
            elapsed,
            Some(parse(value(1))?).filter(|i| *i != 0),
        )
    } else {
        (0, 0, None)
    };
    let number = basis.number;
    let request = Request {
        row: basis.row,
        source_row,
        action,
        code,
        elapsed_seconds,
        reason: value(4).into(),
    };
    current_workspace(state)?.correct_emit_row(number, &request)?;
    app.set_emit_edit_open(false);
    state.emit_edit_basis = None;
    state.detail = -1;
    state.selection = Selection::Driver(number);
    app.set_detail_key(-1);
    update_tables(state, app)?;
    update_form(state, app);
    status(
        app,
        "Emit-korjaus tallennettu. NAC- ja BC-tulokset on tarkistettava ennen julkaisua.",
        false,
    );
    Ok(())
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
    let attempts = Rc::new(RefCell::new(0_u32));
    timer.start(TimerMode::Repeated, Duration::from_millis(650), move || {
        let Some(app) = weak.upgrade() else { return; };
        let current = *phase.borrow();
        match current {
            0 => { app.invoke_navigate(3); app.invoke_select_row(0); app.invoke_select_detail(1); }
            1 => { app.invoke_open_emit_edit(); assert!(app.get_emit_edit_open(), "{}", app.get_status_text()); }
            2 => {
                save_view(&app, &directory.join("01-emit-korjaus.png")).unwrap();
                app.invoke_emit_edit_close(true);
                assert!(app.get_emit_edit_open());
                assert!(app.get_emit_edit_error().contains("perustelu"));
            }
            3 => {
                save_view(&app, &directory.join("02-perustelu-pakollinen.png")).unwrap();
                app.invoke_emit_edit_edited(3, "00:07:00".into());
                app.invoke_emit_edit_edited(4, "Synteettinen varakello vahvistaa AT31-leiman".into());
                app.invoke_emit_edit_close(true);
                assert!(!app.get_emit_edit_open(), "{}", app.get_emit_edit_error());
                let state = state.borrow(); let p=state.workspace.as_ref().unwrap().project().unwrap();
                assert_eq!(p.leimaluvut,raw); assert!(!p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap().hyvaksytty);
                assert!(!p.bc_osallistujat[0].tallennettu_tulos.as_ref().unwrap().hyvaksytty);
                assert_eq!(p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap().aikavirheet,0);
            }
            4 => {
                save_view(&app, &directory.join("03-aika-korjattu.png")).unwrap();
                app.invoke_select_detail(2); app.invoke_open_emit_edit();
                assert!(app.get_emit_edit_open());
                app.invoke_emit_edit_edited(0, "1".into());
                app.invoke_emit_edit_edited(4, "Synteettinen ylimääräinen leima ei kuulu kilpailurataan".into());
                app.invoke_emit_edit_close(true);
                assert!(!app.get_emit_edit_open(), "{}",app.get_emit_edit_error());
            }
            5 => {
                save_view(&app, &directory.join("04-ylimaarainen-poistettu.png")).unwrap();
                let mut state=state.borrow_mut(); let ws=current_workspace(&mut state).unwrap();
                for bc in [false,true] {
                    ws.set_bc_view(bc).unwrap(); let result=ws.result_snapshot(7).unwrap().unwrap();
                    assert_eq!(result.aikavirheet,0); assert_eq!(result.asemien_muut_virheet,0);
                    assert_eq!(result.rivit.len(),4); ws.approve_result(7).unwrap();
                }
                ws.set_bc_view(false).unwrap();
            }
            6 => {
                app.invoke_reload(); app.invoke_select_detail(1); app.invoke_open_emit_edit(); assert!(app.get_emit_edit_open());
                let size = app.window().size().to_logical(app.window().scale_factor());
                app.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                    position: slint::LogicalPosition::new(size.width/2.,size.height/2.), delta_x: 0., delta_y: -3000.,
                });
            }
            7 => {
                save_view(&app, &directory.join("05-korjaushistoria.png")).unwrap();
                smoke::key(&app, slint::platform::Key::Escape); assert!(!app.get_emit_edit_open());
                app.invoke_open_emit_edit(); app.window().set_fullscreen(true);
            }
            8 => {
                assert!(app.window().is_fullscreen());
                save_view(&app,&directory.join("06-kokonaytto.png")).unwrap();
                println!("Emit-korjauskoe: koko näyttö {}x{}",app.window().size().width,app.window().size().height);
                app.window().set_fullscreen(false);
                println!("Emit-korjauskoe: 900x600");
            }
            9 => {
                let size=app.window().size().to_logical(app.window().scale_factor());
                if (size.width-900.).abs()>2. || (size.height-600.).abs()>2. {
                    *attempts.borrow_mut()+=1; assert!(*attempts.borrow()<45,"Koetta ei muutettu 900 × 600 kokoon"); return;
                }
                save_view(&app,&directory.join("07-pieni-ikkuna.png")).unwrap();
                app.invoke_emit_edit_edited(2,"250".into());
                app.invoke_emit_edit_edited(4,"Tarkoituksella virheellinen koodi".into());
                app.invoke_emit_edit_close(true); assert!(app.get_emit_edit_open());
            }
            10 => {
                save_view(&app,&directory.join("08-virheellinen-koodi.png")).unwrap();
                smoke::key(&app,slint::platform::Key::Escape); assert!(!app.get_emit_edit_open());
                let state=state.borrow(); let ws=state.workspace.as_ref().unwrap();
                let reopened=Workspace::open(ws.path()).unwrap(); let p=reopened.project().unwrap();
                assert_eq!(p.leimaluvut,raw); assert_eq!(p.emit_korjaukset.len(),2);
                assert!(p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap().hyvaksytty);
                assert!(p.bc_osallistujat[0].tallennettu_tulos.as_ref().unwrap().hyvaksytty);
                std::fs::write(directory.join("valmis-synteettinen-kilpailu.toml"),p.to_toml().unwrap()).unwrap();
                println!("Emit-korjauksen GUI-koe hyväksytty: kello, lisäleima, perustelu, virheellinen koodi, NAC/BC-hyväksynnät, tallennus, uudelleenavaus, Esc, pieni ikkuna ja koko näyttö.");
                if let Some(timer)=tick_timer.upgrade(){timer.stop();} owner.borrow_mut().take(); let _=slint::quit_event_loop();
            }
            _ => unreachable!(),
        }
        *phase.borrow_mut()+=1;
    });
    Ok(())
}
