//! AS tulkin kilpailujulkaisu taustalla. Ei käytä käyttöliittymäsäiettä verkkoon.
use super::*;
use as2026::tulkki_yhteys::{self, Api, Config};

const INTERPRETATION_POLL_INTERVAL: Duration = Duration::from_secs(15);

pub(super) struct Job {
    path: PathBuf,
    publishing: bool,
    management: bool,
    receiver: Receiver<(Config, Result<(), String>)>,
}

fn config(state: &State) -> Option<&Config> {
    let workspace = state.workspace.as_ref()?;
    state
        .preferences
        .as_ref()?
        .get()
        .tulkki_yhteydet
        .get(workspace.path())
}

fn pair_status(
    config: Option<&Config>,
    manifest: Option<&serde_json::Value>,
    number: u32,
) -> (String, String, bool) {
    let Some(config) = config.filter(|c| c.competition_id > 0) else {
        return (
            "–".into(),
            "Kilpailua ei ole yhdistetty AS tulkkiin.".into(),
            false,
        );
    };
    let total = manifest
        .and_then(|m| tulkki_yhteys::route_for_pair(m, number))
        .and_then(|r| r["sections"].as_array())
        .map_or(0, Vec::len);
    if total == 0 {
        return (
            "–".into(),
            "Parin julkaistua oikeaa riviä ei ole vielä vastaanotettu.".into(),
            false,
        );
    }
    let receipts = config
        .received
        .iter()
        .filter(|r| r.pair_number == number)
        .collect::<Vec<_>>();
    let label = if receipts.len() == total {
        "Tulkattu".into()
    } else if receipts.is_empty() {
        "Ei tulkattu".into()
    } else {
        format!("{}/{} jaksoa", receipts.len(), total)
    };
    let counts = format!(
        "Kilpipuutteet: {} · Aikapuutteet: {} · Muut virheet: {}",
        receipts
            .iter()
            .map(|r| r.section.missing_signs)
            .sum::<u32>(),
        receipts
            .iter()
            .map(|r| r.section.missing_times)
            .sum::<u32>(),
        receipts.iter().map(|r| r.section.other_errors).sum::<u32>()
    );
    (label, counts, true)
}

/// Pelkkä esityskerros: lähdeindeksit, tulospisteet ja tulosten järjestys säilyvät.
pub(super) fn decorate_results(state: &State, view: &mut tables::ViewData) {
    decorate_result_view(config(state), view);
}

fn decorate_result_view(config: Option<&Config>, view: &mut tables::ViewData) {
    let manifest =
        config.and_then(|c| serde_json::from_str::<serde_json::Value>(&c.published_manifest).ok());
    view.headers.push("Tulkkaus".into());
    view.weights.push(0.16);
    for row in &mut view.rows {
        let number = row.cells.row_data(1).and_then(|n| n.parse::<u32>().ok());
        let label = number
            .map(|n| pair_status(config, manifest.as_ref(), n).0)
            .unwrap_or_else(|| "–".into());
        let mut cells = row.cells.iter().collect::<Vec<_>>();
        cells.push(label.into());
        row.cells = model(cells);
    }
}

fn refresh_results(
    state: &State,
    app: &AppWindow,
    config: Option<&Config>,
    manifest: Option<&serde_json::Value>,
) {
    if state.page != 3 {
        return;
    }
    if let Some(column) = app.get_headers().iter().position(|h| h == "Tulkkaus") {
        let rows = app.get_rows();
        for index in 0..rows.row_count() {
            let Some(mut row) = rows.row_data(index) else {
                continue;
            };
            let label = row
                .cells
                .row_data(1)
                .and_then(|n| n.parse::<u32>().ok())
                .map(|n| pair_status(config, manifest, n).0)
                .unwrap_or_else(|| "–".into());
            if row.cells.row_data(column).is_some_and(|old| old == label) {
                continue;
            }
            let mut cells = row.cells.iter().collect::<Vec<_>>();
            if let Some(cell) = cells.get_mut(column) {
                *cell = label.into();
                row.cells = model(cells);
                rows.set_row_data(index, row);
            }
        }
    }
    let number = selected_number(state).ok();
    let receipts = config
        .map(|c| {
            c.received
                .iter()
                .filter(|r| Some(r.pair_number) == number)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    app.set_result_tulkki_has_data(!receipts.is_empty());
    app.set_result_tulkki_missing_signs(
        receipts
            .iter()
            .map(|r| r.section.missing_signs)
            .sum::<u32>()
            .to_string()
            .into(),
    );
    app.set_result_tulkki_missing_times(
        receipts
            .iter()
            .map(|r| r.section.missing_times)
            .sum::<u32>()
            .to_string()
            .into(),
    );
    app.set_result_tulkki_other_errors(
        receipts
            .iter()
            .map(|r| r.section.other_errors)
            .sum::<u32>()
            .to_string()
            .into(),
    );
    let info = number.map(|n| pair_status(config, manifest, n));
    let (label, counts, available) = info.unwrap_or_else(|| ("–".into(), String::new(), false));
    app.set_result_tulkki_status(label.into());
    app.set_result_tulkki_counts(counts.into());
    app.set_result_tulkki_available(available);
}

fn detail_text(config: &Config, number: u32) -> String {
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&config.published_manifest) else {
        return "Oikeaa riviä ei ole vielä noudettu.".into();
    };
    let Some(route) = tulkki_yhteys::route_for_pair(&manifest, number) else {
        return "Parin julkaistu rata puuttuu.".into();
    };
    let Some(sections) = route["sections"].as_array() else {
        return String::new();
    };
    let pair = manifest["pairs"].as_array().and_then(|pairs| {
        pairs
            .iter()
            .find(|p| p["number"].as_u64() == Some(u64::from(number)))
    });
    let mut lines = vec![
        format!(
            "{} / {}",
            pair.and_then(|p| p["driver"].as_str()).unwrap_or(""),
            pair.and_then(|p| p["navigator"].as_str()).unwrap_or("")
        ),
        if config.automatic_results {
            "Automaattinen hyväksyntä ja tulostus on käytössä.".into()
        } else {
            "Lopputulos hyväksytään käsin.".into()
        },
    ];
    for section in sections {
        let id = section["id"].as_str().unwrap_or("");
        let name = section["name"].as_str().unwrap_or(id);
        lines.push(format!("\n{name}"));
        let Some(receipt) = config
            .received
            .iter()
            .find(|r| r.pair_number == number && r.section.id == id)
        else {
            lines.push("Ei vielä tulkattu.".into());
            continue;
        };
        let time = chrono::DateTime::parse_from_rfc3339(&receipt.saved_at)
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%d.%m.%Y %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|_| receipt.saved_at.clone());
        lines.push(format!("Tulkattu · {} · {time}", receipt.reviewer));
        if !receipt.section.pending_time_ids.is_empty() {
            lines.push(format!(
                "Täydennettävät ULA-ajat: {}. Aikaa ei ole merkitty puuttuvaksi korttimerkinnäksi.",
                receipt
                    .section
                    .pending_time_ids
                    .iter()
                    .map(|id| section["entries"]
                        .as_array()
                        .and_then(|ee| ee.iter().find(|e| e["id"].as_str() == Some(id)))
                        .and_then(|e| e["label"].as_str())
                        .unwrap_or(id)
                        .to_owned())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        lines.push(format!(
            "Kilpipuutteet: {} · Aikapuutteet: {} · Muut virheet: {}",
            receipt.section.missing_signs,
            receipt.section.missing_times,
            receipt.section.other_errors
        ));
        if let Some(entries) = section["entries"].as_array() {
            let mut missing = Vec::new();
            let mut times = Vec::new();
            for (index, entry) in entries.iter().enumerate() {
                let eid = entry["id"].as_str().unwrap_or("");
                let label = entry["label"].as_str().unwrap_or(eid);
                if receipt.section.missing_entry_ids.iter().any(|e| e == eid) {
                    missing.push(format!("{label} (jakson rivi {})", index + 1));
                }
                if let Some(time) = receipt.section.times.get(eid) {
                    times.push(format!("{label} (jakson rivi {}): {time}", index + 1));
                }
            }
            lines.push(if missing.is_empty() {
                "Puuttuvat: ei puuttuvia asemia".into()
            } else {
                format!("Puuttuvat: {}", missing.join(", "))
            });
            if !times.is_empty() {
                lines.push(format!("Ajat:\n{}", times.join("\n")));
            }
        }
        if !receipt.section.note.is_empty() {
            lines.push(format!("Huomautus: {}", receipt.section.note));
        }
    }
    lines.push("\nMUUTOSHISTORIA – uusin ensin\nHistoria kertyy tämän version käyttöönotosta. Jokaisesta tallennuksesta näytetään sen aikaiset kirjaukset.".into());
    for event in config
        .history
        .iter()
        .rev()
        .filter(|e| e.pair_number == number)
    {
        lines.push(format!("\n{} · {}", event.saved_at, event.reviewer));
        let original = if event.original_sections.is_empty() {
            event
                .sections
                .iter()
                .filter_map(|s| serde_json::to_value(s).ok())
                .collect()
        } else {
            event.original_sections.clone()
        };
        for saved in &original {
            let id = saved["id"].as_str().unwrap_or("");
            let section = sections.iter().find(|s| s["id"].as_str() == Some(id));
            let label = |id: &str| {
                event
                    .original_labels
                    .get(id)
                    .cloned()
                    .or_else(|| {
                        section
                            .and_then(|s| s["entries"].as_array())
                            .and_then(|ee| {
                                ee.iter()
                                    .enumerate()
                                    .find(|(_, e)| e["id"].as_str() == Some(id))
                            })
                            .map(|(i, e)| {
                                format!("{} (rivi {})", e["label"].as_str().unwrap_or(id), i + 1)
                            })
                    })
                    .unwrap_or_else(|| id.into())
            };
            let missing = saved["missingEntryIds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .map(label)
                .collect::<Vec<_>>();
            lines.push(format!(
                "{} · Puuttuvat: {} · Muut virheet: {}",
                section.and_then(|s| s["name"].as_str()).unwrap_or(id),
                if missing.is_empty() {
                    "ei puuttuvia".into()
                } else {
                    missing.join(", ")
                },
                saved["otherErrors"]
            ));
            for (id, time) in saved["times"].as_object().into_iter().flatten() {
                lines.push(format!("{}: {}", label(id), time.as_str().unwrap_or("")));
            }
        }
    }
    lines.join("\n")
}

pub(super) fn refresh(state: &State, app: &AppWindow) {
    let last = config(state)
        .map(|c| c.last_received_at.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("ei vielä onnistunutta hakua");
    let waiting = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .and_then(|p| config(state).map(|c| tulkki_yhteys::penalty_changes(p, c)))
        .and_then(Result::ok)
        .map(|changes| changes.len());
    let waiting = waiting.map_or_else(
        || "Laskentajono ei tiedossa".to_string(),
        |n| format!("Laskentaan odottaa: {n} paria"),
    );
    app.set_tulkki_sync_status(
        format!(
            "{} · Viimeisin onnistunut haku: {last} · {waiting}",
            state.tulkki_sync_notice
        )
        .into(),
    );
    let Some(project) = state.workspace.as_ref().and_then(Workspace::project) else {
        app.set_tulkki_rows(model(Vec::new()));
        app.set_result_tulkki_status("–".into());
        app.set_result_tulkki_counts(String::new().into());
        app.set_result_tulkki_available(false);
        app.set_result_tulkki_has_data(false);
        return;
    };
    let config = config(state);
    app.set_tulkki_configured(config.is_some_and(|c| c.competition_id > 0));
    let manifest =
        config.and_then(|c| serde_json::from_str::<serde_json::Value>(&c.published_manifest).ok());
    refresh_results(state, app, config, manifest.as_ref());
    let query = state.filter.trim().to_lowercase();
    let mut completed = 0;
    let rows = project
        .kilpailijat
        .iter()
        .filter_map(|pair| {
            let receipts = config
                .map(|c| {
                    c.received
                        .iter()
                        .filter(|r| r.pair_number == pair.numero)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let total = manifest
                .as_ref()
                .and_then(|m| tulkki_yhteys::route_for_pair(m, pair.numero))
                .and_then(|r| r["sections"].as_array())
                .map_or(0, Vec::len);
            let ready = total > 0 && receipts.len() == total;
            if ready {
                completed += 1;
            }
            let names = format!("{} / {}", pair.kuljettaja, pair.kartanlukija);
            if !format!("{} {names}", pair.numero)
                .to_lowercase()
                .contains(&query)
            {
                return None;
            }
            Some(TableRow {
                key: pair.numero as i32,
                cells: model(vec![
                    pair.numero.to_string().into(),
                    names.into(),
                    if ready {
                        "Tulkattu".into()
                    } else if receipts.is_empty() {
                        "Ei tulkattu".into()
                    } else {
                        format!("{}/{} jaksoa", receipts.len(), total).into()
                    },
                    receipts
                        .iter()
                        .map(|r| r.section.missing_signs)
                        .sum::<u32>()
                        .to_string()
                        .into(),
                    receipts
                        .iter()
                        .map(|r| r.section.missing_times)
                        .sum::<u32>()
                        .to_string()
                        .into(),
                    receipts
                        .iter()
                        .map(|r| r.section.other_errors)
                        .sum::<u32>()
                        .to_string()
                        .into(),
                ]),
            })
        })
        .collect();
    app.set_tulkki_rows(model(rows));
    app.set_tulkki_summary(if config.is_some_and(|c|c.competition_id>0){format!("{completed}/{} paria tulkattu kokonaan. Avaa pari nähdäksesi puuttuvat kilvet, ajat ja huomautukset.",project.kilpailijat.len())}else{"Yhdistä kilpailu AS tulkki -painikkeesta vastaanottaaksesi tulkinnat.".into()}.into());
    if app.get_tulkki_details_open()
        && let Some(config) = config
    {
        app.set_tulkki_detail_text(detail_text(config, app.get_tulkki_pair().max(0) as u32).into());
    }
}

fn save(state: &mut State, path: PathBuf, config: Config) -> Result<(), String> {
    state
        .preferences
        .as_mut()
        .ok_or("Asetusten tallennus ei ole käytettävissä.")?
        .save_tulkki(path, config)
}

fn feedback(app: &AppWindow, message: impl Into<SharedString>, error: bool) {
    app.set_tulkki_status(message.into());
    app.set_tulkki_error(error);
}

pub(super) fn process_automatic(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    let Some(mut config) = config(state).cloned().filter(|c| c.automatic_results) else {
        return Ok(());
    };
    let workspace = current_workspace(state)?;
    if workspace.bc_view() || workspace.lan_client().is_some() {
        return Ok(());
    }
    let Some(project) = workspace.project() else {
        return Ok(());
    };
    let candidates = project
        .kilpailijat
        .iter()
        .filter_map(|d| {
            tulkki_yhteys::automatic_result_key(project, &config, d.numero)
                .map(|key| (d.numero, key))
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Ok(());
    }
    let path = workspace.path().to_owned();
    let before = config.automatic_prints.clone();
    let mut numbers = Vec::new();
    for (number, key) in candidates {
        workspace.approve_result(number)?;
        config.automatic_prints.insert(number.to_string(), key);
        numbers.push(number);
    }
    // Tallennus ensin: uudelleenkäynnistys tai saman verkkosivun uusintahaku ei tuplaa tulosteita.
    save(state, path.clone(), config.clone()).map_err(|e| invalid(&e))?;
    if let Err(error) = printing::print_numbers(state, app, &numbers) {
        config.automatic_prints = before;
        save(state, path, config).map_err(|e| invalid(&e))?;
        return Err(error);
    }
    update_tables(state, app)?;
    update_form(state, app);
    status(
        app,
        format!(
            "Automaattisesti hyväksytty ja lähetetty tulostukseen: {}",
            numbers
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        false,
    );
    Ok(())
}

pub(super) fn bind(state: &SharedState, app: &AppWindow, timer: &Timer) {
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_tulkki_automatic_changed(move |enabled| {
        let Some(app) = weak.upgrade() else { return };
        let mut state = shared.borrow_mut();
        let result = (|| -> Result<(), String> {
            let path = state
                .workspace
                .as_ref()
                .filter(|w| w.lan_client().is_none())
                .ok_or("Valinta tehdään pääkoneella")?
                .path()
                .to_owned();
            let mut config = config(&state).cloned().ok_or("Luo ensin verkkokilpailu")?;
            config.automatic_results = enabled;
            save(&mut state, path, config)?;
            state.tulkki_apply_pending = true;
            Ok(())
        })();
        app.set_tulkki_automatic(config(&state).is_some_and(|c| c.automatic_results));
        if let Err(error) = result {
            feedback(&app, error, true);
        } else {
            feedback(
                &app,
                if enabled {
                    "Automaattinen hyväksyntä ja tulostus käytössä."
                } else {
                    "Tulokset hyväksytään käsin."
                },
                false,
            );
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_tulkki_manage(move |kind| {
        let Some(app) = weak.upgrade() else { return };
        let mut state = shared.borrow_mut();
        let result = (|| -> Result<(), String> {
            if state.tulkki_job.is_some() {
                return Err("Odota nykyisen tiedonsiirron valmistumista.".into());
            }
            let path = state
                .workspace
                .as_ref()
                .filter(|w| w.lan_client().is_none())
                .ok_or("Avaa kilpailu pääkoneella")?
                .path()
                .to_owned();
            let mut config = config(&state)
                .cloned()
                .filter(|c| c.competition_id > 0)
                .ok_or("Luo ensin verkkokilpailu")?;
            let password = app.get_tulkki_password().to_string();
            if password.trim().is_empty() {
                return Err("Anna WordPressin sovellussalasana hallintaa varten.".into());
            }
            config.user = app.get_tulkki_user().trim().into();
            let rotate = kind == 1;
            if rotate {
                config.stage_join_code()?;
            }
            save(&mut state, path.clone(), config.clone())?;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("as2026-tulkki-hallinta".into())
                .spawn(move || {
                    let result = Api::new(&config.site)
                        .and_then(|api| api.manage(&mut config, &password, rotate));
                    let _ = tx.send((config, result));
                })
                .map_err(|_| "Hallintapyynnön käynnistäminen epäonnistui")?;
            state.tulkki_job = Some(Job {
                path,
                publishing: true,
                management: true,
                receiver: rx,
            });
            app.set_tulkki_busy(true);
            feedback(&app, "Tallennetaan hallintatoimintoa…", false);
            Ok(())
        })();
        if let Err(error) = result {
            feedback(&app, error, true);
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_open_tulkki_qr(move || {
        if let Some(app) = weak.upgrade() {
            let state = shared.borrow();
            let result = (|| -> Result<(), String> {
                let link = config(&state)
                    .ok_or("Avaa kilpailun AS tulkki -yhteys")?
                    .join_url()?;
                let bitmap = as2026::join_qr::encode(&link)?;
                let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                    &bitmap.rgba,
                    bitmap.side,
                    bitmap.side,
                );
                app.set_tulkki_qr_image(slint::Image::from_rgba8(buffer));
                app.set_tulkki_qr_link(link.into());
                app.set_tulkki_qr_open(true);
                Ok(())
            })();
            if let Err(error) = result {
                feedback(&app, error, true);
            }
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_open_result_tulkki(move || {
        if let Some(app) = weak.upgrade() {
            let number = selected_number(&shared.borrow()).ok();
            if let Some(number) = number {
                app.invoke_select_tulkki_pair(number as i32);
            }
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_toggle_tulkki_view(move || {
        if let Some(app) = weak.upgrade() {
            let mut state = shared.borrow_mut();
            if !can_leave(&mut state, &app) {
                return;
            }
            state.filter.clear();
            app.set_filter("".into());
            app.set_tulkki_view(!app.get_tulkki_view());
            refresh(&state, &app);
            right_row::update_page(&state, &app);
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_select_tulkki_pair(move |number| {
        if let Some(app) = weak.upgrade() {
            let state = shared.borrow();
            let Some(project) = state.workspace.as_ref().and_then(Workspace::project) else {
                return;
            };
            let Some(pair) = project
                .kilpailijat
                .iter()
                .find(|p| p.numero == number as u32)
            else {
                return;
            };
            app.set_tulkki_pair(number);
            app.set_tulkki_detail_title(format!("Pari {} – tulkinnat", pair.numero).into());
            app.set_tulkki_detail_text(
                config(&state)
                    .map(|c| detail_text(c, pair.numero))
                    .unwrap_or_else(|| "Ei vielä vastaanotettuja tulkintoja.".into())
                    .into(),
            );
            app.set_tulkki_details_open(true);
        }
    });
    let shared = state.clone();
    app.on_receive_tulkki(move || {
        let mut state = shared.borrow_mut();
        state.tulkki_poll_due = None;
        if config(&state).is_none_or(|c| c.revision == 0) {
            state.tulkki_sync_notice =
                "Lähetä kilpailutiedot AS tulkki -painikkeesta ennen vastaanottoa.".into();
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_open_tulkki(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };
        let mut state = shared.borrow_mut();
        if !can_leave(&mut state, &app) {
            return;
        }
        let Some(workspace) = state
            .workspace
            .as_ref()
            .filter(|w| w.project().is_some() && w.lan_client().is_none())
        else {
            status(&app, "Avaa muokattava kilpailu pääkoneella.", true);
            return;
        };
        let config = state
            .preferences
            .as_ref()
            .and_then(|p| p.get().tulkki_yhteydet.get(workspace.path()))
            .cloned()
            .unwrap_or_default();
        app.set_tulkki_qr_ready(config.join_url().is_ok());
        app.set_tulkki_closed(config.closed);
        app.set_tulkki_automatic(config.automatic_results);
        app.set_tulkki_site(if config.site.is_empty() {
            "https://autosuunnistus.net/".into()
        } else {
            config.site.into()
        });
        app.set_tulkki_user(config.user.into());
        app.set_tulkki_password("".into());
        app.set_tulkki_configured(config.competition_id > 0);
        app.set_tulkki_code(config.join_code.into());
        feedback(
            &app,
            if config.revision > 0 {
                format!(
                    "Verkkokilpailu {}. Kilpailutietojen versio {} on lähetetty.",
                    config.competition_id, config.revision
                )
            } else {
                "Kilpailutietoja ei ole vielä lähetetty. Luo verkkokilpailu AS2026:n tiedoista."
                    .into()
            },
            false,
        );
        app.set_tulkki_open(true);
    });
    let weak = app.as_weak();
    app.on_close_tulkki(move || {
        if let Some(app) = weak.upgrade()
            && !app.get_tulkki_busy()
        {
            app.set_tulkki_password("".into());
            app.set_tulkki_open(false);
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_tulkki_publish(move || {
        let Some(app) = weak.upgrade() else {
            return;
        };
        if app.get_tulkki_busy() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let mut state = shared.borrow_mut();
            if state.tulkki_job.is_some() {
                return Err("Vastaanotto on kesken. Odota hetki ennen lähettämistä.".into());
            }
            let workspace = state
                .workspace
                .as_ref()
                .filter(|w| w.lan_client().is_none())
                .ok_or("Avaa kilpailu pääkoneella.")?;
            let project = workspace.project().ok_or("Avaa AS2026-kilpailu.")?;
            let manifest = tulkki_yhteys::manifest(project)?;
            let path = workspace.path().to_owned();
            let mut config = state
                .preferences
                .as_ref()
                .and_then(|p| p.get().tulkki_yhteydet.get(&path))
                .cloned()
                .unwrap_or_default();
            if config.request_id.is_empty() {
                config = Config::prepare(
                    &app.get_tulkki_site(),
                    &app.get_tulkki_user(),
                    &project.kilpailu.nimi,
                )?;
            } else if config.competition_id == 0 {
                if tulkki_yhteys::api_url(&app.get_tulkki_site())?.as_str() != config.site {
                    return Err(
                        "Kesken jääneen verkkokilpailun luontia jatketaan alkuperäiseen sivustoon."
                            .into(),
                    );
                }
                config.user = app.get_tulkki_user().trim().into();
            }
            let password = app.get_tulkki_password().to_string();
            if config.competition_id == 0 && (config.user.is_empty() || password.trim().is_empty())
            {
                return Err(
                    "Anna WordPressin ylläpitäjän käyttäjätunnus ja sovellussalasana.".into(),
                );
            }
            // Jos tallennus epäonnistuu, palvelimelle ei tehdä luontipyyntöä.
            save(&mut state, path.clone(), config.clone())?;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("as2026-tulkki-lahetys".into())
                .spawn(move || {
                    let result = (|| {
                        let api = Api::new(&config.site)?;
                        if config.competition_id == 0 {
                            api.create(&mut config, &password)?;
                        }
                        api.publish(&mut config, &manifest)
                    })();
                    let _ = tx.send((config, result));
                })
                .map_err(|_| "Lähetyksen käynnistäminen epäonnistui.")?;
            state.tulkki_job = Some(Job {
                path,
                receiver: rx,
                publishing: true,
                management: false,
            });
            app.set_tulkki_busy(true);
            feedback(&app, "Lähetetään kilpailutietoja sivustolle…", false);
            Ok(())
        })();
        if let Err(error) = result {
            feedback(&app, error, true);
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(200), move || {
        let Some(app) = weak.upgrade() else { return; };
        let mut state = shared.borrow_mut();
        let current=state.workspace.as_ref().filter(|w|w.lan_client().is_none()).map(|w|w.path().to_owned());
        if current!=state.tulkki_poll_path {
            state.tulkki_poll_path=current.clone(); state.tulkki_poll_due=None; state.tulkki_apply_pending=true;
            state.tulkki_sync_notice.clear(); app.set_tulkki_sync_error(false); refresh(&state,&app);
        }
        if state.tulkki_job.is_none() {
            if !app.get_demo() && state.tulkki_apply_pending
                && !state.invalid_field && state.edit_draft.is_empty() && state.route_draft.is_empty()
                && state.draft.is_empty() && !state.entry_dirty && !is_new(&state.selection)
                && !app.get_competitor_edit_open() && !app.get_result_entry_open() && !app.get_confirm_open()
                && !app.get_emit_edit_open() && state.pending_result.is_none()
                && let Some(config)=config(&state).cloned() {
                state.tulkki_apply_pending=false;
                let result=state.workspace.as_mut().unwrap().apply_tulkki_penalties(&config);
                match result {
                    Ok(true)=>{if let Err(error)=update_tables(&state,&app){status(&app,error.to_string(),true);}update_form(&state,&app);}
                    Ok(false)=>{right_row::refresh_summary(&state,&app);}
                    Err(error)=>{state.tulkki_sync_notice=format!("Tulkinnan siirto laskentaan epäonnistui: {error}");app.set_tulkki_sync_error(true);refresh(&state,&app);}
                }
                if !app.get_tulkki_sync_error() && !app.get_print_busy()
                    && let Err(error)=process_automatic(&mut state,&app){status(&app,format!("Automaattinen tulos: {error}"),true);}
            }
            if !app.get_demo() && state.tulkki_poll_due.is_none_or(|due|std::time::Instant::now()>=due)
                && let Some(path)=current && let Some(mut config)=config(&state).cloned().filter(|c|c.competition_id>0 && c.revision>0 && !c.closed) {
                let outgoing=state.workspace.as_ref().and_then(Workspace::project).and_then(|p|tulkki_yhteys::manifest(p).ok()).filter(|m|serde_json::from_str::<serde_json::Value>(&config.published_manifest).is_ok_and(|old|old!=*m && (config.received.is_empty() || tulkki_yhteys::compatible_row_change(&old,m))));
                let publishing=outgoing.is_some();
                let (tx,rx)=std::sync::mpsc::channel();
                let started=std::thread::Builder::new().name("as2026-tulkki-vastaanotto".into()).spawn(move ||{
                    let result=Api::new(&config.site).and_then(|api|if let Some(manifest)=outgoing {api.publish(&mut config,&manifest)}else{api.receive(&mut config).map(|_|())});
                    let _=tx.send((config,result));
                });
                if started.is_ok(){state.tulkki_job=Some(Job{path,receiver:rx,publishing,management:false});if publishing{app.set_tulkki_busy(true);}else{app.set_tulkki_receiving(true);}}
                else {state.tulkki_poll_due=Some(std::time::Instant::now()+INTERPRETATION_POLL_INTERVAL);state.tulkki_sync_notice="Vastaanoton käynnistäminen epäonnistui.".into();app.set_tulkki_sync_error(true);refresh(&state,&app);}
            }
            return;
        }
        let job=state.tulkki_job.as_ref().unwrap();
        let Some(outcome) = background::poll(&job.receiver) else { return; };
        let job = state.tulkki_job.take().unwrap();
        state.tulkki_poll_due=Some(std::time::Instant::now()+INTERPRETATION_POLL_INTERVAL);
        if job.publishing {app.set_tulkki_busy(false);} else {app.set_tulkki_receiving(false);}
        let active=state.workspace.as_ref().is_some_and(|w|w.path()==job.path);
        match outcome {
            Ok((mut config, result)) => {
                if result.is_ok(){config.last_received_at=chrono::Local::now().format("%d.%m.%Y %H:%M:%S").to_string();}
                if let Err(error) = save(&mut state, job.path, config.clone()) {
                    if active {state.tulkki_sync_notice=format!("Vastaanotettuja tietoja ei tallennettu: {error}. Nouto uusitaan.");app.set_tulkki_sync_error(true);refresh(&state,&app);if job.publishing{feedback(&app,state.tulkki_sync_notice.clone(),true);}}
                    return;
                }
                if !active { return; }
                app.set_tulkki_closed(config.closed);
                if job.publishing {
                    state.tulkki_sync_notice=match &result {Ok(())=>"Kilvet ja yhteystila tallennettu sivustolle.".into(),Err(error)=>format!("Sivustopäivitys ei onnistunut: {error}. Paikalliset muutokset ovat tallessa; tarkista AS tulkin yhteys.")};
                    app.set_tulkki_sync_error(result.is_err());refresh(&state,&app);
                }
                state.tulkki_apply_pending=true;
                if !job.publishing {
                    match result {
                        Ok(())=>{state.tulkki_sync_notice=format!("Vastaanotto ajan tasalla · {}",chrono::Local::now().format("%H:%M:%S"));app.set_tulkki_sync_error(false);}
                        Err(error)=>{state.tulkki_sync_notice=format!("Vastaanotto ei ole ajan tasalla: {error}");app.set_tulkki_sync_error(true);}
                    }
                    refresh(&state,&app);return;
                }
                app.set_tulkki_configured(config.competition_id > 0);
                app.set_tulkki_qr_ready(config.join_url().is_ok());
                app.set_tulkki_code(config.join_code.into());
                app.set_tulkki_site(config.site.into());
                if config.competition_id > 0 { app.set_tulkki_password("".into()); }
                match result {
                    Ok(()) => feedback(&app, if job.management {if config.closed{"Puhelinyhteys suljettu. Puhelimet liittyvät uudelleen avaamisen jälkeen.".into()}else{"Hallinta tallennettu. Puhelinyhteys on avoinna; käytä nykyistä liittymiskoodia tai QR-koodia.".into()}}else{format!("Kilpailutiedot lähetetty. Tietoversio {}. Tulkinnat vastaanotetaan taustalla Kilvet → Tulkinnat -näkymään.",config.revision)}, false),
                    Err(error) => feedback(&app, error, true),
                }
            }
            Err(_) => {if active{state.tulkki_sync_notice="Tiedonsiirto keskeytyi. Vastaanottoa yritetään uudelleen.".into();app.set_tulkki_sync_error(true);refresh(&state,&app);if job.publishing{feedback(&app,state.tulkki_sync_notice.clone(),true);}}}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use as2026::tulkki_yhteys::{ReceivedSection, SectionInterpretation};
    fn fixture() -> Config {
        let mut config = Config {
            competition_id: 7,
            revision: 1,
            ..Config::default()
        };
        config.published_manifest =
            serde_json::json!({"routes":[{"id":"r1","sections":[{"id":"j1"},{"id":"j2"}]}],
            "pairs":[{"number":101,"routeId":"r1"},{"number":202,"routeId":"r1"}]})
            .to_string();
        config.received.push(ReceivedSection {
            event_id: 1,
            pair_number: 101,
            reviewer: "Koe".into(),
            saved_at: "2026-11-10T18:00:00Z".into(),
            section: SectionInterpretation {
                id: "j1".into(),
                missing_entry_ids: vec!["e2".into()],
                times: BTreeMap::new(),
                note: "Koe".into(),
                other_errors: 0,
                missing_signs: 1,
                missing_times: 0,
                interpreted: true,
                pending_time_ids: Vec::new(),
            },
        });
        config
    }
    #[test]
    fn results_are_matched_by_competition_number_without_changing_order_or_points() {
        let mut view = tables::ViewData {
            headers: vec!["Sija".into(), "Nro".into(), "Luokka".into(), "Tulos".into()],
            weights: vec![1.; 4],
            rows: vec![
                TableRow {
                    key: 42,
                    cells: model(vec!["1".into(), "202".into(), "B".into(), "50 p".into()]),
                },
                TableRow {
                    key: 7,
                    cells: model(vec!["2".into(), "101".into(), "A".into(), "100 p".into()]),
                },
            ],
            ..Default::default()
        };
        let original = view
            .rows
            .iter()
            .map(|r| (r.key, r.cells.iter().collect::<Vec<_>>()))
            .collect::<Vec<_>>();
        let config = fixture();
        decorate_result_view(Some(&config), &mut view);
        assert_eq!(view.headers.last().unwrap(), "Tulkkaus");
        assert_eq!(view.headers.len(), view.weights.len());
        assert_eq!(view.rows[0].cells.row_data(4).unwrap(), "Ei tulkattu");
        assert_eq!(view.rows[1].cells.row_data(4).unwrap(), "1/2 jaksoa");
        for (row, (key, cells)) in view.rows.iter().zip(original) {
            assert_eq!(row.key, key);
            assert_eq!(
                row.cells.iter().take(cells.len()).collect::<Vec<_>>(),
                cells
            );
        }
    }
    #[test]
    fn unconnected_competitions_and_completed_pairs_have_distinct_states() {
        let mut config = fixture();
        let manifest = serde_json::from_str(&config.published_manifest).unwrap();
        assert_eq!(pair_status(None, None, 101).0, "–");
        assert!(!pair_status(None, None, 101).2);
        assert_eq!(
            pair_status(Some(&config), Some(&manifest), 101).0,
            "1/2 jaksoa"
        );
        let mut second = config.received[0].clone();
        second.section.id = "j2".into();
        second.section.missing_signs = 0;
        config.received.push(second);
        let (status, counts, available) = pair_status(Some(&config), Some(&manifest), 101);
        assert_eq!(status, "Tulkattu");
        assert!(available);
        assert!(counts.contains("Kilpipuutteet: 1"));
        assert!(!pair_status(Some(&config), Some(&manifest), 999).2);
    }
}
