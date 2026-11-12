//! Oikean rivin Excel-tuonti olemassa olevaan kilpailuun.
use super::*;
use as2026::interpretation::{self, Preview};

pub(super) fn update_page(state: &State, app: &AppWindow) {
    refresh_summary(state, app);
    let Some(project) = state.workspace.as_ref().and_then(Workspace::project) else {
        app.set_signs_routes(model(Vec::new()));
        app.set_signs_rows(model(Vec::new()));
        app.set_signs_summary("Avaa kilpailu nähdäksesi kilvet.".into());
        return;
    };
    let selected =
        (app.get_signs_route().max(0) as usize).min(project.radat.len().saturating_sub(1));
    app.set_signs_routes(model(
        project
            .radat
            .iter()
            .map(|r| format!("{} – {}", r.numero, r.nimi).into())
            .collect(),
    ));
    app.set_signs_route(selected as i32);
    let stored = project
        .radat
        .get(selected)
        .and_then(|route| project.oikeat_rivit.iter().find(|r| r.rata == route.numero));
    let Some(stored) = stored else {
        app.set_signs_rows(model(Vec::new()));
        app.set_signs_summary(
            if project.radat.is_empty() {
                "Lisää ensin rata Radat ja asemat -välilehdellä."
            } else {
                "Oikeaa riviä ei ole vielä tuotu valitulle radalle."
            }
            .into(),
        );
        return;
    };
    let query = state.filter.trim().to_lowercase();
    app.set_signs_rows(model(
        stored
            .merkinnat
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.merkinta.label.to_lowercase().contains(&query))
            .map(|(index, entry)| TableRow {
                key: index as i32,
                cells: model(vec![
                    (index + 1).to_string().into(),
                    entry.merkinta.section.to_string().into(),
                    entry.merkinta.label.clone().into(),
                    if !entry.merkinta.kind.is_clock() {
                        "–".into()
                    } else {
                        if project.kilpailu.ajanottotapa == 0 || entry.requires_manual_time(true) {
                            "1"
                        } else {
                            "0"
                        }
                        .into()
                    },
                    if entry.merkinta.kind.is_clock() {
                        "–"
                    } else if entry.mitatoity {
                        "1"
                    } else {
                        "0"
                    }
                    .into(),
                ]),
            })
            .collect(),
    ));
    let sections = stored
        .merkinnat
        .iter()
        .map(|e| e.merkinta.section)
        .collect::<BTreeSet<_>>()
        .len();
    app.set_signs_summary(
        format!(
            "{} merkintää, {} jaksoa. Lähde: {}.",
            stored.merkinnat.len(),
            sections,
            stored.lahde
        )
        .into(),
    );
}

pub(super) fn refresh_summary(state: &State, app: &AppWindow) {
    if !app.get_rt_summary_open() {
        return;
    }
    let Some(project) = state.workspace.as_ref().and_then(Workspace::project) else {
        return;
    };
    let summary = interpretation::missing_rt_summary(project);
    let rows: Vec<_> = summary
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| TableRow {
            key: i as i32,
            cells: model(vec![
                format!("{} – {}", r.route, r.route_name).into(),
                r.section.map_or("–".into(), |j| j.to_string()).into(),
                r.station.clone().into(),
                r.occurrences.to_string().into(),
                r.pairs.len().to_string().into(),
                r.pairs
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
                    .into(),
            ]),
        })
        .collect();
    let current = app.get_rt_summary_rows();
    if current.row_count() != rows.len()
        || current
            .iter()
            .zip(&rows)
            .any(|(a, b)| a.key != b.key || a.cells.iter().ne(b.cells.iter()))
    {
        app.set_rt_summary_rows(model(rows));
    }
    app.set_rt_summary_text(format!("Koko kilpailu · vastaanotettuja kirjauksia {} / {} parilta. Puutteita = puuttuvat merkinnät, pareja = eri kilpailuparit.{}",summary.received_pairs,summary.total_pairs,if summary.unidentified_pairs>0{format!(" {} parilla RT-määrälle ei ole vastaavaa tunnusluetteloa; määrä näkyy yksilöimättömänä.",summary.unidentified_pairs)}else{String::new()}).into());
}

pub(super) fn set_manual_time(
    state: &mut State,
    app: &AppWindow,
    key: i32,
    checked: bool,
) -> GuiResult<()> {
    let project = current_workspace(state)?
        .project()
        .ok_or_else(|| invalid("Avaa muokattava kilpailu"))?;
    if project.kilpailu.ajanottotapa == 0 && !checked {
        return Err(invalid(
            "Manuaalisessa ajanotossa kaikkien aika-asemien ajat syötetään käsin",
        ));
    }
    let route = project
        .radat
        .get(app.get_signs_route() as usize)
        .ok_or_else(|| invalid("Valitse rata"))?
        .numero;
    let entry = project
        .oikeat_rivit
        .iter()
        .find(|r| r.rata == route)
        .and_then(|r| usize::try_from(key).ok().and_then(|i| r.merkinnat.get(i)))
        .ok_or_else(|| invalid("Valitse aikarivi"))?
        .tunniste
        .clone();
    current_workspace(state)?.set_right_row_time_input(route, &entry, Some(checked))?;
    app.set_signs_selected_key(key);
    update_page(state, app);
    status(app, "Aikamerkinnän syöttötapa tallennettu.", false);
    Ok(())
}

pub(super) fn open(state: &mut State, app: &AppWindow, saved: bool) -> GuiResult<()> {
    if !can_leave(state, app) {
        return Ok(());
    }
    let project = current_workspace(state)?
        .project()
        .ok_or_else(|| invalid("Avaa muokattava AS2026-kilpailu"))?;
    let routes = project.radat.iter().map(|r| r.numero).collect::<Vec<_>>();
    let labels = project
        .radat
        .iter()
        .map(|r| format!("{} – {}", r.numero, r.nimi).into())
        .collect();
    if routes.is_empty() {
        return Err(invalid(
            "Lisää kilpailuun kohderata ennen oikean rivin tuontia",
        ));
    }
    let previews = if saved {
        let previews: Vec<_> = project
            .oikeat_rivit
            .iter()
            .map(|r| Preview {
                sheet: format!("{} (rata {})", r.lahde, r.rata),
                entries: r.merkinnat.iter().map(|e| e.merkinta.clone()).collect(),
                issues: Vec::new(),
            })
            .collect();
        if previews.is_empty() {
            return Err(invalid("Kilpailuun ei ole vielä tuotu oikeita rivejä"));
        }
        previews
    } else {
        let Some(path) = choose_file(state, || {
            rfd::FileDialog::new()
                .set_title("Tuo oikea rivi Syotto-välilehdeltä")
                .add_filter("Oikean rivin Excel", &["xlsm", "xlsx", "xls"])
                .pick_file()
        }) else {
            return Ok(());
        };
        interpretation::preview_workbook(&path)?
    };
    state.right_row_previews = previews;
    state.right_row_routes = routes;
    app.set_right_row_sheets(model(
        state
            .right_row_previews
            .iter()
            .map(|p| p.sheet.clone().into())
            .collect(),
    ));
    app.set_right_row_routes(model(labels));
    app.set_right_row_sheet(0);
    app.set_right_row_view_only(saved);
    app.set_right_row_route(if state.page == 7 {
        app.get_signs_route()
    } else {
        0
    });
    confirmation(
        state,
        app,
        PendingAction::Notice,
        if saved {
            "Tallennetut oikeat rivit"
        } else {
            "Tuo oikea rivi"
        },
        String::new(),
    );
    app.set_confirm_action_label(
        if saved {
            "Sulje"
        } else {
            "Tallenna oikea rivi"
        }
        .into(),
    );
    app.set_import_open(false);
    app.set_right_row_open(true);
    refresh(state, app)
}

pub(super) fn refresh(state: &mut State, app: &AppWindow) -> GuiResult<()> {
    state.right_row_replacement_basis = None;
    if !app.get_right_row_view_only() {
        app.set_confirm_action_label("Tallenna oikea rivi".into());
    }
    let preview = state
        .right_row_previews
        .get(app.get_right_row_sheet() as usize)
        .ok_or_else(|| invalid("Valitse Syotto-välilehti"))?;
    let viewing = app.get_right_row_view_only();
    app.set_confirm_text(if viewing {
        "Tallennettu oikea rivi. Jokainen toistuva merkintä on oma esiintymänsä."
    } else {
        "Valitse välilehti ja kohderata. Tuonti tallentaa oikean rivin avoimeen kilpailuun; se ei korvaa ajanottorataa tai kilpailijoita. Tuo muut välilehdet samalla tavalla erikseen."
    }.into());
    app.set_right_row_rows(model(
        preview
            .entries
            .iter()
            .map(|e| TableRow {
                key: e.source_row as i32,
                cells: model(vec![
                    e.source_row.to_string().into(),
                    e.section.to_string().into(),
                    e.label.clone().into(),
                ]),
            })
            .collect(),
    ));
    app.set_right_row_error(
        preview
            .issues
            .iter()
            .map(|i| format!("A{}: {}", i.source_row, i.message))
            .collect::<Vec<_>>()
            .join("\n")
            .into(),
    );
    app.set_confirm_acceptable(
        viewing
            || (preview.can_publish()
                && state
                    .right_row_routes
                    .get(app.get_right_row_route() as usize)
                    .is_some()),
    );
    Ok(())
}

pub(super) fn finish(state: &mut State, app: &AppWindow, accepted: bool) -> GuiResult<()> {
    if accepted && !app.get_right_row_view_only() {
        let preview = state
            .right_row_previews
            .get(app.get_right_row_sheet() as usize)
            .ok_or_else(|| invalid("Valitse Syotto-välilehti"))?
            .clone();
        let route = *state
            .right_row_routes
            .get(app.get_right_row_route() as usize)
            .ok_or_else(|| invalid("Valitse kohderata"))?;
        let config = void_config(state);
        let project = current_workspace(state)?.project().unwrap().clone();
        let replacing = project
            .oikeat_rivit
            .iter()
            .find(|r| r.rata == route)
            .is_some_and(|r| {
                r.merkinnat
                    .iter()
                    .map(|e| &e.merkinta)
                    .ne(preview.entries.iter())
            });
        if replacing {
            if current_workspace(state)?.lan_client().is_some() {
                app.set_right_row_error("Oikea rivi korvataan kilpailun pääkoneella.".into());
                return Ok(());
            }
            let mut basis_config = config.clone();
            basis_config.last_received_at.clear();
            let basis = serde_json::to_string(&(&project, &basis_config, route, &preview.entries))
                .map_err(|_| invalid("Esikatselun muodostus epäonnistui"))?;
            if state.right_row_replacement_basis.as_deref() != Some(&basis) {
                let mut candidate = project.clone();
                if let Err(error) =
                    as2026::tulkki_yhteys::correct_route(&mut candidate, &config, route, &preview)
                {
                    app.set_right_row_error(error.to_string().into());
                    return Ok(());
                }
                let old = project
                    .oikeat_rivit
                    .iter()
                    .find(|r| r.rata == route)
                    .unwrap();
                let new = candidate
                    .oikeat_rivit
                    .iter()
                    .find(|r| r.rata == route)
                    .unwrap();
                let added = new
                    .merkinnat
                    .iter()
                    .filter(|e| !old.merkinnat.iter().any(|o| o.tunniste == e.tunniste))
                    .count();
                let removed = old
                    .merkinnat
                    .iter()
                    .filter(|e| !new.merkinnat.iter().any(|n| n.tunniste == e.tunniste))
                    .count();
                let changes = project
                    .kilpailijat
                    .iter()
                    .zip(&candidate.kilpailijat)
                    .filter(|(a, b)| {
                        a.lisavirheet.puuttuvat_rt_kpl != b.lisavirheet.puuttuvat_rt_kpl
                            || a.lisavirheet.puuttuvat_rt_pisteet
                                != b.lisavirheet.puuttuvat_rt_pisteet
                    })
                    .map(|(a, b)| {
                        format!(
                            "Nro {}: puuttuvia {} → {}, RT-pisteet {} → {}",
                            a.numero,
                            a.lisavirheet.puuttuvat_rt_kpl,
                            b.lisavirheet.puuttuvat_rt_kpl,
                            a.lisavirheet.puuttuvat_rt_pisteet,
                            b.lisavirheet.puuttuvat_rt_pisteet
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                app.set_confirm_text(format!("Korvataan radan {route} oikea rivi. Lisättyjä merkintöjä {added}, poistuvia {removed}.\n\nSamat kilvet kohdistetaan jakson, tunnuksen ja esiintymäjärjestyksen mukaan. Uusia kilpiä ei merkitä automaattisesti puuttuviksi. Poistuvat kilvet poistuvat nykyisistä puutteista, mutta alkuperäiset kirjaukset säilyvät historiassa. Tulkattu-tila säilyy ja kirjauksia voi korjata. Lisätyn ULA:n tuntematon käsin syötettävä aika jää täydennettäväksi; sitä ei merkitä puuttuvaksi asemaksi. Muuttuneiden tulosten hyväksyntä poistuu.\n\n{changes}\n\nVahvista korvaaminen tai valitse Poistu.").into());
                app.set_right_row_error("".into());
                app.set_confirm_action_label("Korvaa oikea rivi".into());
                state.right_row_replacement_basis = Some(basis);
                return Ok(());
            }
        }
        let result = if replacing {
            current_workspace(state)?.correct_right_row(route, &preview, &config)
        } else {
            current_workspace(state)?.import_right_row(route, &preview)
        };
        if let Err(error) = result {
            app.set_right_row_error(error.to_string().into());
            return Ok(());
        }
        status(app, "Oikea rivi tallennettu kilpailuun.", false);
        state.right_row_replacement_basis = None;
        state.tulkki_apply_pending = false;
        app.set_confirm_action_label("Tallenna oikea rivi".into());
        update_tables(state, app)?;
        update_form(state, app);
        app.set_signs_route(app.get_right_row_route());
        update_page(state, app);
        app.set_right_row_error("".into());
        app.set_confirm_text(format!("Oikea rivi tallennettu radalle {route}. Valitse seuraava välilehti ja kohderata tai lopeta Poistu-painikkeesta.").into());
        return Ok(());
    }
    app.set_right_row_open(false);
    app.set_confirm_open(false);
    state.pending = None;
    state.right_row_previews.clear();
    state.right_row_routes.clear();
    follow_pending_result(state, app)
}

fn void_config(state: &State) -> as2026::tulkki_yhteys::Config {
    state
        .workspace
        .as_ref()
        .and_then(|w| {
            state
                .preferences
                .as_ref()
                .and_then(|p| p.get().tulkki_yhteydet.get(w.path()))
        })
        .cloned()
        .unwrap_or_default()
}
fn void_basis(project: &Project, mut config: as2026::tulkki_yhteys::Config) -> GuiResult<String> {
    // Onnistuneen taustahaun kellonaika ei muuta esikatselun tuloksia.
    config.last_received_at.clear();
    serde_json::to_string(&(project, config)).map_err(|_| invalid("Esikatselua ei voitu muodostaa"))
}
pub(super) fn offer_void(
    state: &mut State,
    app: &AppWindow,
    key: i32,
    checked: bool,
) -> GuiResult<()> {
    if !checked {
        return Err(invalid("Mitätöintiä ei voi perua"));
    }
    if !can_leave(state, app) {
        return Ok(());
    }
    let project = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .ok_or_else(|| invalid("Avaa kilpailu"))?;
    let route = project
        .radat
        .get(app.get_signs_route().max(0) as usize)
        .ok_or_else(|| invalid("Valitse rata"))?
        .numero;
    let entry = project
        .oikeat_rivit
        .iter()
        .find(|r| r.rata == route)
        .and_then(|r| usize::try_from(key).ok().and_then(|i| r.merkinnat.get(i)))
        .ok_or_else(|| invalid("Valitse kilpi"))?;
    let id = entry.tunniste.clone();
    let label = entry.merkinta.label.clone();
    let section = entry.merkinta.section;
    let basis = void_basis(project, void_config(state))?;
    let mut preview = project.clone();
    let affected = as2026::tulkki_yhteys::void_sign(&mut preview, route, &id, &void_config(state))?;
    let lines = affected
        .iter()
        .map(|n| {
            let old = project.kilpailijat.iter().find(|d| d.numero == *n).unwrap();
            let new = preview.kilpailijat.iter().find(|d| d.numero == *n).unwrap();
            format!(
                "Nro {n}: puuttuvia {} → {}, RT-pisteet {} → {}",
                old.lisavirheet.puuttuvat_rt_kpl,
                new.lisavirheet.puuttuvat_rt_kpl,
                old.lisavirheet.puuttuvat_rt_pisteet,
                new.lisavirheet.puuttuvat_rt_pisteet
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    confirmation(
        state,
        app,
        PendingAction::VoidSign { route, id, basis },
        "Mitätöi kilpi",
        format!(
            "Rata {route}, jakso {section}, kilpi {label}. Mitätöinti koskee vain tätä esiintymää eikä sitä voi perua.\n\n{}\n\nKorjataanko nämä tulokset? Hyväksytyt ja edelleen laskentakelpoiset tulokset säilyvät hyväksyttyinä. Muut pysyvät tarkistettavina. Muutos lähetetään myös sivustolle.",
            if affected.is_empty() {
                "Ei muutettavia puhelintulkintaan perustuvia tuloksia. Käsin kirjattuja, yksilöimättömiä RT-määriä ei voida korjata automaattisesti.".into()
            } else {
                lines
            }
        ),
    );
    update_page(state, app);
    Ok(())
}
pub(super) fn finish_void(
    state: &mut State,
    app: &AppWindow,
    route: u32,
    id: &str,
    basis: &str,
) -> GuiResult<()> {
    let current = state
        .workspace
        .as_ref()
        .and_then(Workspace::project)
        .ok_or_else(|| invalid("Avaa kilpailu"))?;
    if void_basis(current, void_config(state))? != basis {
        return Err(invalid(
            "Kilpailutiedot muuttuivat. Avaa mitätöinnin esikatselu uudelleen.",
        ));
    }
    let config = void_config(state);
    let affected = current_workspace(state)?.void_right_row(route, id, &config)?;
    update_tables(state, app)?;
    update_form(state, app);
    update_page(state, app);
    let project = current_workspace(state)?
        .result_project()?
        .ok_or_else(|| invalid("Avaa kilpailu"))?;
    let printable = affected
        .iter()
        .copied()
        .filter(|n| as2026::printing::competitor(&project, *n, None).is_ok())
        .collect::<Vec<_>>();
    if !printable.is_empty() {
        confirmation(
            state,
            app,
            PendingAction::PrintVoided(printable.clone()),
            "Tulosta muuttuneet tulokset",
            format!(
                "Kilpi mitätöity ja tulokset korjattu. Tulostetaanko kilpailuparien {} uudet tuloslaput?",
                printable
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    status(
        app,
        "Kilpi mitätöity. Sivustopäivitys tehdään taustalla. Tarkista AS tulkin yhteystila.",
        false,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn void_confirmation_ignores_contact_time_but_detects_changed_competition_data() {
        let project =
            Project::from_toml(include_str!("../../fixtures/native-demo/kilpailu.toml")).unwrap();
        let mut config = as2026::tulkki_yhteys::Config {
            last_received_at: "12.11.2026 12:00:00".into(),
            ..Default::default()
        };
        let basis = void_basis(&project, config.clone()).unwrap();
        config.last_received_at = "12.11.2026 12:00:05".into();
        assert_eq!(void_basis(&project, config.clone()).unwrap(), basis);
        config.revision += 1;
        assert_ne!(void_basis(&project, config).unwrap(), basis);
        let mut changed = project.clone();
        changed.kilpailijat[0].lisavirheet.muut_virhepisteet += 600;
        assert_ne!(
            void_basis(&changed, as2026::tulkki_yhteys::Config::default()).unwrap(),
            basis
        );
    }
}
