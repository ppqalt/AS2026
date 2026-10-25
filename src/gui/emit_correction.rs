use super::*;
use as2026::emit_corrections;

pub(super) struct Basis {
    number: u32,
    path: PathBuf,
    content: Vec<as2026::Punch>,
    new: bool,
    source_card: u32,
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
    let selected_read = if state.page == 2 {
        usize::try_from(state.selected)
            .ok()
            .and_then(|i| project.leimaluvut.get(i))
    } else {
        None
    };
    let assigned = selected_read
        .and_then(|read| project.kilpailijat.iter().find(|d| d.emit == read.emit))
        .map(|d| d.numero);
    let number = if state.page == 2 {
        assigned
            .or_else(|| project.kilpailijat.first().map(|d| d.numero))
            .ok_or_else(|| invalid("Lisää ensin kilpailija"))?
    } else {
        selected_number(state)?
    };
    let source_card = selected_read.map(|r| r.emit);
    let page = state.page;
    let ws = current_workspace(state)?;
    ws.pin_lan_basis();
    let project = ws.project().unwrap();
    let driver_card = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .unwrap()
        .emit;
    let source_card = source_card.unwrap_or(driver_card);
    let existing = project
        .active_readout(source_card)
        .map(|r| emit_corrections::effective_readout(project, r).and_then(|r| r.content()))
        .transpose()?;
    let new = existing.is_none();
    let content = existing.unwrap_or_else(|| {
        vec![as2026::Punch {
            code: 0,
            elapsed_seconds: 0,
        }]
    });
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
    app.set_emit_edit_card(driver.emit.to_string().into());
    app.set_emit_edit_competitors(model(
        project
            .kilpailijat
            .iter()
            .map(|d| SharedString::from(format!("{} · {}", d.numero, d.kuljettaja)))
            .collect(),
    ));
    app.set_emit_edit_driver(if page == 2 && assigned.is_none() {
        -1
    } else {
        project
            .kilpailijat
            .iter()
            .position(|d| d.numero == number)
            .unwrap() as i32
    });
    app.set_emit_edit_new(new);
    app.set_emit_edit_routes(model(
        project
            .radat
            .iter()
            .map(|r| SharedString::from(format!("{} · {}", r.numero, r.nimi)))
            .collect(),
    ));
    app.set_emit_edit_route(-1);
    app.set_emit_edit_history("".into());
    app.set_emit_edit_error("".into());
    app.set_emit_edit_open(true);
    state.emit_edit_basis = Some(Basis {
        number,
        path: ws.path().into(),
        content,
        new,
        source_card,
    });
    Ok(())
}
pub(super) fn edit(_state: &mut State, app: &AppWindow, key: i32, value: &str) -> GuiResult<()> {
    if key >= 0 && key % 3 == 2 && value == "1" {
        let mut fields = app.get_emit_edit_fields().iter().collect::<Vec<_>>();
        let start = (key / 3 * 3) as usize;
        if start + 3 <= fields.len() {
            fields.drain(start..start + 3);
        }
        for (i, f) in fields.iter_mut().enumerate() {
            f.key = i as i32;
        }
        app.set_emit_edit_fields(model(fields));
        return Ok(());
    }
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
    if restore && basis.new {
        return template(state, app, app.get_emit_edit_route());
    }
    if restore {
        let content = project
            .active_readout(basis.source_card)
            .unwrap()
            .content()?;
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
pub(super) fn select_driver(state: &mut State, app: &AppWindow, index: i32) -> GuiResult<()> {
    let basis = state
        .emit_edit_basis
        .as_mut()
        .ok_or_else(|| invalid("Avaa editori uudelleen"))?;
    let project = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .ok_or_else(|| invalid("Kilpailu puuttuu"))?;
    let driver = project
        .kilpailijat
        .get(index as usize)
        .ok_or_else(|| invalid("Valitse kilpailija"))?;
    basis.number = driver.numero;
    app.set_emit_edit_driver(index);
    app.set_emit_edit_card(driver.emit.to_string().into());
    app.set_emit_edit_fields(model(forms::emit_card_fields(
        project,
        driver.numero,
        &basis.content,
    )?));
    Ok(())
}
pub(super) fn insert(state: &mut State, app: &AppWindow, after: i32) -> GuiResult<()> {
    add(state, app, false)?;
    let mut fields = app.get_emit_edit_fields().iter().collect::<Vec<_>>();
    let added = fields.split_off(fields.len() - 3);
    let position = ((after + 1).max(0) as usize * 3).min(fields.len());
    fields.splice(position..position, added);
    for (i, field) in fields.iter_mut().enumerate() {
        field.key = i as i32;
    }
    app.set_emit_edit_fields(model(fields));
    Ok(())
}
pub(super) fn template(state: &mut State, app: &AppWindow, index: i32) -> GuiResult<()> {
    let basis = state
        .emit_edit_basis
        .as_ref()
        .ok_or_else(|| invalid("Avaa editori uudelleen"))?;
    if !basis.new {
        return Ok(());
    }
    let project = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .ok_or_else(|| invalid("Kilpailu puuttuu"))?;
    let route = project
        .radat
        .get(index as usize)
        .ok_or_else(|| invalid("Valitse rata"))?;
    let mut content = vec![as2026::Punch {
        code: 0,
        elapsed_seconds: 0,
    }];
    content.extend(
        route
            .asemat
            .iter()
            .filter(|s| s.emit_koodi != 0)
            .map(|s| as2026::Punch {
                code: s.emit_koodi,
                elapsed_seconds: 0,
            }),
    );
    let mut fields = forms::emit_card_fields(project, basis.number, &content)?;
    for field in fields.iter_mut().filter(|f| f.key % 3 == 1) {
        field.value = "".into();
        field.hours = "".into();
        field.minutes = "".into();
        field.seconds = "".into();
    }
    app.set_emit_edit_route(index);
    app.set_emit_edit_fields(model(fields));
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
    if basis.path != ws.path()
        || (!basis.new
            && project
                .active_readout(basis.source_card)
                .map(|r| emit_corrections::effective_readout(project, r).and_then(|r| r.content()))
                .transpose()?
                .as_ref()
                != Some(&basis.content))
    {
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
    // Card order is explicit in the table; validation reports out-of-order times.
    let number = basis.number;
    if app.get_emit_edit_driver() < 0 {
        return Err(invalid("Valitse kilpailija, jolle korttiluku kuuluu"));
    }
    let source_card = basis.source_card;
    let card = app
        .get_emit_edit_card()
        .trim()
        .parse()
        .map_err(|_| invalid("Anna Emit-numero kokonaislukuna"))?;
    let new = basis.new;
    if new && app.get_emit_edit_route() < 0 {
        return Err(invalid("Valitse rata uudelle leimariville"));
    }
    current_workspace(state)?.save_emit_card_from(number, source_card, card, punches, new)?;
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
                app.invoke_emit_edit_insert(0);
                app.invoke_emit_edit_edited(3,"99".into());
                app.invoke_emit_edit_edited(4,"12:05:01".into());
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
            5 => {
                app.invoke_navigate(1); app.invoke_select_row(0); app.invoke_open_emit_edit();
                assert!(app.get_emit_edit_open());
                app.set_emit_edit_card("654321".into());
                app.invoke_emit_edit_close(true);
                assert!(!app.get_emit_edit_open(),"{}",app.get_emit_edit_error());
                let state=state.borrow();let p=state.workspace.as_ref().unwrap().project().unwrap();
                assert_eq!(p.kilpailijat[0].emit,654321);
                assert_eq!(&p.leimaluvut[..raw.len()],raw.as_slice());
                assert!(p.active_readout(654321).is_some());
                println!("0.7.2 Emit-taulukko: lisäys väliin, välitön poisto ja korttinumeron korjaus hyväksytty.");
            }
            6 => {app.invoke_navigate(1);app.invoke_select_row(0);app.window().set_size(slint::LogicalSize::new(1366.,900.));}
            7 => {menu_smoke::click(&app,345.,140.);app.set_competitor_details_open(true);}
            8 => {assert!(app.get_editor_scroll_min()<0.,"Näkyvän laajennetun editorin pitää ylivuotaa");app.set_editor_scroll_offset(app.get_editor_scroll_min());}
            9 => {app.set_competitor_details_open(false);}
            10 => {assert!(app.get_editor_scroll_offset()>=app.get_editor_scroll_min()-0.5);assert!(app.get_editor_scroll_offset()<=0.5);save_view(&app,&directory.join("05-vieritys-suljettuna.png")).unwrap();app.window().set_size(slint::LogicalSize::new(1366.,600.));app.set_competitor_details_open(true);}
            11 => {menu_smoke::click(&app,345.,140.);app.set_competitor_details_open(true);}
            12 => {app.set_editor_scroll_offset(app.get_editor_scroll_min());app.set_competitor_details_open(false);}
            13 => {
                assert!(app.get_editor_scroll_min()<0.,"Myös matalan ikkunan editorin pitää näkyä ja ylivuotaa");
                assert!(app.get_editor_scroll_offset()>=app.get_editor_scroll_min()-0.5);assert!(app.get_editor_scroll_offset()<=0.5);save_view(&app,&directory.join("06-vieritys-pieni.png")).unwrap();
                let mut state=state.borrow_mut();let mut p=state.workspace.as_ref().unwrap().project().unwrap().clone();p.leimaluvut.clear();p.emit_korjaukset.clear();for d in &mut p.kilpailijat{d.tallennettu_tulos=None;}for d in &mut p.bc_osallistujat{d.tallennettu_tulos=None;}
                state.workspace=Some(Workspace::create_imported(&directory.join("tyhja-kortti"),p).unwrap());drop(state);
                app.invoke_navigate(1);app.invoke_select_row(0);app.invoke_open_emit_edit();assert!(app.get_emit_edit_new());
            }
            14 => {app.invoke_emit_edit_template(0);assert_eq!(app.get_emit_edit_fields().row_count(),6);assert_eq!(app.get_emit_edit_fields().row_data(0).unwrap().value,"31");app.invoke_emit_edit_close(true);assert!(app.get_emit_edit_open(),"Tyhjiä kellonaikoja ei arvata");}
            15 => {app.invoke_emit_edit_edited(1,"120500".into());app.invoke_emit_edit_edited(4,"121000".into());save_view(&app,&directory.join("07-tyhjalta-pohjalta.png")).unwrap();app.invoke_emit_edit_close(true);assert!(!app.get_emit_edit_open(),"{}",app.get_emit_edit_error());let state=state.borrow();let p=state.workspace.as_ref().unwrap().project().unwrap();assert!(p.leimaluvut[0].lahde.as_deref().unwrap().contains("Käsin"));}
            16 => {let mut state=state.borrow_mut();let mut p=state.workspace.as_ref().unwrap().project().unwrap().clone();p.leimaluvut[0].emit=999001;p.kilpailijat[0].emit=555555;p.kilpailijat[0].tallennettu_tulos=None;for d in &mut p.bc_osallistujat{d.tallennettu_tulos=None;}state.workspace=Some(Workspace::create_imported(&directory.join("vaara-numero"),p).unwrap());drop(state);app.invoke_navigate(2);app.invoke_select_row(0);app.invoke_open_emit_edit();assert_eq!(app.get_emit_edit_driver(),-1);app.invoke_emit_edit_close(true);assert!(app.get_emit_edit_open());app.invoke_emit_edit_select_driver(0);}
            17 => {app.invoke_emit_edit_close(true);assert!(!app.get_emit_edit_open(),"{}",app.get_emit_edit_error());let state=state.borrow();let p=state.workspace.as_ref().unwrap().project().unwrap();assert_eq!(p.leimaluvut[0].emit,999001);assert!(p.active_readout(555555).is_some());println!("0.7.2: vieritys, ratapohjasta luonti ja vääränumeroisen kortin kohdistus varmennettu natiivisti.");if let Some(timer)=tick_timer.upgrade(){timer.stop();}owner.borrow_mut().take();let _=slint::quit_event_loop();}
            _ => unreachable!(),
        }
        *phase.borrow_mut()+=1;
    });
    Ok(())
}
