//! Kortinluvun jälkeinen syöttö ja tulostus. Kilpailijatieto ja tulostusjono
//! pysyvät erillään: tulostinvirhe ei hävitä tallennettuja aikoja.
use super::*;
use as2026::printing::{self, Document, Kind};

pub(super) fn open_entry(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    let number = selected_number(state)?;
    let ws = current_workspace(state)?;
    let project = ws
        .result_project()?
        .ok_or_else(|| invalid("Aikojen syöttö tarvitsee AS2026-kilpailun"))?;
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    let result = ws.result_snapshot(number)?;
    if result.is_none() && !driver.tila.as_ref().is_some_and(|s| s.tunnus == "KESK") {
        return Err(invalid("Lue ensin kilpailijan Emit-kortti"));
    }
    let pending = result
        .as_ref()
        .map_or_else(Vec::new, |r| r.pending_manual_clocks());
    let mut fields = result
        .as_ref()
        .into_iter()
        .flat_map(|r| r.rivit.iter())
        .filter(|r| r.expected_code == 0 && r.station_type != "L")
        .map(|r| ResultEntryInput {
            key: r.output_index as i32,
            label: format!("{} · {} · kellonaika", r.sequence, r.name).into(),
            value: if pending.contains(&r.output_index) {
                String::new()
            } else {
                format!("{:02}:{:02}:{:02}", r.clock[0], r.clock[1], r.clock[2])
            }
            .into(),
        })
        .collect::<Vec<_>>();
    fields.push(ResultEntryInput {
        key: -1,
        label: "Puuttuvat RT-asemat (kpl)".into(),
        value: driver.lisavirheet.puuttuvat_rt_kpl.to_string().into(),
    });
    fields.push(ResultEntryInput {
        key: -2,
        label: "Muut virhepisteet (p)".into(),
        value: driver.lisavirheet.muut_virhepisteet.to_string().into(),
    });
    app.set_result_entry_identity(
        format!(
            "#{} · {} / {}{}",
            number,
            driver.kuljettaja,
            driver.kartanlukija,
            if ws.bc_view() {
                " · Beginners Cup"
            } else {
                ""
            }
        )
        .into(),
    );
    state.entry_number = Some(number);
    app.set_result_entry_fields(model(fields));
    app.set_result_entry_error("".into());
    app.set_result_entry_focus(0);
    app.set_result_entry_open(true);
    Ok(())
}
fn parse_clock(value: &str) -> GuiResult<[i64; 3]> {
    let compact = value.trim();
    let normalized = if compact.len() == 6 && compact.bytes().all(|b| b.is_ascii_digit()) {
        format!("{}:{}:{}", &compact[..2], &compact[2..4], &compact[4..])
    } else {
        compact.replace('.', ":")
    };
    let values = normalized
        .split(':')
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid("Anna kellonaika muodossa hh:mm:ss, esimerkiksi 18:03:25"))?;
    let clock: [i64; 3] = values
        .try_into()
        .map_err(|_| invalid("Anna tunnit, minuutit ja sekunnit"))?;
    if !(0..=23).contains(&clock[0])
        || !(0..=59).contains(&clock[1])
        || !(0..=59).contains(&clock[2])
    {
        return Err(invalid("Kellonajan pitää olla välillä 00:00:00–23:59:59"));
    }
    Ok(clock)
}
fn selected_printer(state: &State) -> String {
    state
        .preferences
        .as_ref()
        .map_or(String::new(), |p| p.get().tulostin.clone())
}

fn queue(state: &mut State, app: &AppWindow, document: Document) -> GuiResult<()> {
    if state.print_test {
        state.last_print = Some(document);
        return Ok(());
    }
    if app.get_print_busy() {
        return Err(invalid("Odota edellisen tulostustyön lähettämistä"));
    }
    let printer = selected_printer(state);
    let weak = app.as_weak();
    app.set_print_busy(true);
    let result = std::thread::Builder::new().name("as2026-tulostus".into()).spawn(move || {
        let result = printing::submit(&document,&printer);
        let title = document.title;
        let _ = weak.upgrade_in_event_loop(move |app| {
            app.set_print_busy(false);
            match result {
                Ok(()) => status(&app,format!("{title}: tulostustyö lähetetty tulostusjonoon."),false),
                Err(error) => status(&app,format!("{title}: {error}\nAjat ja virhepisteet ovat tallessa. Tarkista tulostin asetuksista ja tulosta uudelleen."),true),
            }
        });
    });
    if let Err(error) = result {
        app.set_print_busy(false);
        return Err(invalid(&io_message(&error)));
    }
    Ok(())
}
fn finish_entry(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    let number = state
        .entry_number
        .ok_or_else(|| invalid("Valitse kilpailija uudelleen"))?;
    let mut clocks = Vec::new();
    let mut missing = 0;
    let mut other = 0;
    for (index, field) in app.get_result_entry_fields().iter().enumerate() {
        let result = if field.key > 0 {
            parse_clock(&field.value).map(|clock| clocks.push((field.key as u32, clock)))
        } else if field.key == -1 {
            field
                .value
                .trim()
                .parse::<u32>()
                .map(|v| missing = v)
                .map_err(|_| {
                    invalid("Puuttuvien RT-asemien määrän pitää olla ei-negatiivinen kokonaisluku")
                })
        } else {
            field
                .value
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|v| *v >= 0)
                .map(|v| other = v)
                .ok_or_else(|| {
                    invalid("Muiden virhepisteiden pitää olla ei-negatiivinen kokonaisluku")
                })
        };
        if let Err(error) = result {
            app.set_result_entry_focus(index as i32);
            return Err(error);
        }
    }
    let ws = current_workspace(state)?;
    ws.complete_result_entry(number, &clocks, missing, other)?;
    let project = ws.result_project()?.unwrap();
    let label = if ws.bc_view() {
        ws.project()
            .and_then(|p| p.beginners_cup.as_ref())
            .map(|c| c.tunnus.as_str())
    } else {
        None
    };
    let document = printing::competitor(&project, number, label)?;
    queue(state, app, document)?;
    app.set_result_entry_open(false);
    state.entry_number = None;
    update_tables(state, app)?;
    update_form(state, app);
    if state.print_test {
        status(
            app,
            "Tulostuskoe: tulos tallennettu ja tuloste muodostettu.",
            false,
        );
    }
    Ok(())
}
fn sources(
    ws: &Workspace,
    include_bc: bool,
) -> GuiResult<Vec<(&'static str, as2026::legacy::Archive)>> {
    if let Some(project) = ws.project() {
        let mut result = vec![("N", application::calculated_project(project)?)];
        if include_bc && project.beginners_cup.is_some() {
            result.push(("B", as2026::beginners::results(project)?));
        }
        Ok(result)
    } else {
        Ok(vec![("N", ws.results()?)])
    }
}
fn open_classes(state: &mut State, app: &AppWindow, stations: bool) -> GuiResult<()> {
    let number = selected_number(state).ok();
    let ws = current_workspace(state)?;
    let selected = ws
        .results()?
        .competitors
        .iter()
        .find(|d| Some(d.number) == number)
        .map(|d| d.class_code.clone());
    let mut choices = Vec::new();
    for (scope, archive) in sources(ws, false)? {
        for code in printing::class_choices(&archive)? {
            choices.push(DrawClassOption {
                code: format!("{scope}:{code}").into(),
                label: code.clone().into(),
                selected: selected.as_ref() == Some(&code),
            });
        }
    }
    if let Some(config) = ws.project().and_then(|p| p.beginners_cup.as_ref()) {
        choices.push(DrawClassOption {
            code: format!("B:{}", config.tunnus).into(),
            label: format!("{} · {}", config.tunnus, config.nimi).into(),
            selected: ws.bc_view(),
        });
    }
    app.set_print_classes(model(choices));
    app.set_print_stations(stations);
    app.set_print_landscape(stations);
    app.set_print_fit(true);
    app.set_print_error("".into());
    app.set_print_classes_open(true);
    Ok(())
}
fn print_classes(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    let selections = app
        .get_print_classes()
        .iter()
        .filter(|c| c.selected)
        .map(|c| c.code.to_string())
        .collect::<Vec<_>>();
    if selections.is_empty() {
        return Err(invalid("Valitse vähintään yksi tulostettava luokka"));
    }
    let mut document = Document {
        title: "AS2026 tulokset".into(),
        landscape: app.get_print_landscape(),
        pages: Vec::new(),
    };
    let include_bc = selections.iter().any(|s| s.starts_with("B:"));
    for (scope, archive) in sources(current_workspace(state)?, include_bc)? {
        let codes = selections
            .iter()
            .filter_map(|s| s.strip_prefix(&format!("{scope}:")))
            .map(String::from)
            .collect::<Vec<_>>();
        if !codes.is_empty() {
            document.pages.extend(
                printing::classes(
                    &archive,
                    &codes,
                    if app.get_print_stations() {
                        Kind::Stations
                    } else {
                        Kind::Classes
                    },
                    document.landscape,
                    app.get_print_fit(),
                )?
                .pages,
            );
        }
    }
    queue(state, app, document)?;
    app.set_print_classes_open(false);
    Ok(())
}
pub(super) fn initialize_printer(state: &State, app: &AppWindow) {
    let name = selected_printer(state);
    let mut names = vec!["Käyttöjärjestelmän oletus".to_owned()];
    if !name.is_empty() {
        names.push(name.clone());
    }
    app.set_printer_index(if name.is_empty() { 0 } else { 1 });
    app.set_printers(model(names.into_iter().map(SharedString::from).collect()));
    app.set_printer_name(if name.is_empty() {
        "Käyttöjärjestelmän oletus".into()
    } else {
        name.into()
    });
}
pub(super) fn bind(state: &SharedState, app: &AppWindow) {
    let state2 = state.clone();
    let weak = app.as_weak();
    app.on_result_entry_edited(move |key, value| {
        if let Some(app) = weak.upgrade() {
            let fields = app.get_result_entry_fields();
            if let Some(index) = fields.iter().position(|f| f.key == key) {
                let mut field = fields.row_data(index).unwrap();
                field.value = value;
                fields.set_row_data(index, field);
            }
            app.set_result_entry_error("".into());
        }
    });
    let weak = app.as_weak();
    app.on_result_entry_close(move |accepted| {
        if let Some(app) = weak.upgrade() {
            let mut state = state2.borrow_mut();
            if !accepted {
                app.set_result_entry_open(false);
                state.entry_number = None;
                drop(state);
                defer_pending_result(&state2, &app);
            } else if let Err(error) = finish_entry(&mut state, &app) {
                app.set_result_entry_error(error_text(&error).into());
            } else {
                drop(state);
                defer_pending_result(&state2, &app);
            }
        }
    });
    let state2 = state.clone();
    let weak = app.as_weak();
    app.on_open_print_classes(move |stations| {
        if let Some(app) = weak.upgrade() {
            run_action(&app, || {
                let mut s = state2.borrow_mut();
                if can_leave(&mut s, &app) {
                    open_classes(&mut s, &app, stations)
                } else {
                    Ok(())
                }
            });
        }
    });
    let weak = app.as_weak();
    app.on_print_class_toggled(move |index, selected| {
        if let Some(app) = weak.upgrade() {
            let items = app.get_print_classes();
            if let Ok(i) = usize::try_from(index)
                && let Some(mut item) = items.row_data(i)
            {
                item.selected = selected;
                items.set_row_data(i, item);
            }
        }
    });
    let state2 = state.clone();
    let weak = app.as_weak();
    app.on_print_classes_close(move |accepted| {
        if let Some(app) = weak.upgrade() {
            if !accepted {
                app.set_print_classes_open(false);
            } else if let Err(error) = print_classes(&mut state2.borrow_mut(), &app) {
                app.set_print_error(error_text(&error).into());
            }
        }
    });
    let state2 = state.clone();
    let weak = app.as_weak();
    app.on_refresh_printers(move || {
        if let Some(app) = weak.upgrade() {
            run_action(&app, || {
                let state = state2.borrow();
                let current = selected_printer(&state);
                let mut names = vec!["Käyttöjärjestelmän oletus".to_owned()];
                names.extend(printing::printers().map_err(|e| invalid(&e))?);
                if !current.is_empty() && !names.contains(&current) {
                    names.push(current.clone());
                }
                app.set_printer_index(names.iter().position(|n| n == &current).unwrap_or(0) as i32);
                app.set_printers(model(names.into_iter().map(SharedString::from).collect()));
                Ok(())
            });
        }
    });
    let state2 = state.clone();
    let weak = app.as_weak();
    app.on_select_printer(move |index| {
        if let Some(app) = weak.upgrade() {
            run_action(&app, || {
                let name = if index == 0 {
                    String::new()
                } else {
                    app.get_printers()
                        .row_data(index as usize)
                        .ok_or_else(|| invalid("Valitse tulostin luettelosta"))?
                        .to_string()
                };
                state2
                    .borrow_mut()
                    .preferences
                    .as_mut()
                    .ok_or_else(|| invalid("Asetustiedosto ei ole käytettävissä"))?
                    .save_printer(&name)
                    .map_err(|e| invalid(&e))?;
                app.set_printer_name(if name.is_empty() {
                    "Käyttöjärjestelmän oletus".into()
                } else {
                    name.into()
                });
                Ok(())
            });
        }
    });
}
