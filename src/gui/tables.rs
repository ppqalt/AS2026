//! Taulukoiden sisältö johdetaan avatusta kilpailusta. Rivin avain pysyy
//! alkuperäisen mallin indeksinä myös haussa ja tulosten järjestämisessä.

use super::{ResultOverview, TableRow};
use as2026::{
    CoreError, Punch, application,
    legacy::{Archive, Competitor},
    ranking::{GroupKind, Status, TiePolicy},
    workspace::Workspace,
};
use slint::{ModelRc, SharedString, VecModel};
use std::{collections::HashMap, rc::Rc};

#[derive(Default)]
pub struct ViewData {
    pub rows: Vec<TableRow>,
    pub headers: Vec<SharedString>,
    pub weights: Vec<f32>,
    pub detail_rows: Vec<TableRow>,
    pub detail_headers: Vec<SharedString>,
    pub detail_weights: Vec<f32>,
    pub detail_title: String,
    pub notes: String,
    pub note_details: String,
    pub result_overview: ResultOverview,
}

fn row(index: usize, cells: Vec<String>) -> TableRow {
    TableRow {
        key: index as i32,
        cells: ModelRc::from(Rc::new(VecModel::from(
            cells
                .into_iter()
                .map(|cell| {
                    if cell.trim().is_empty() {
                        "-".to_owned()
                    } else {
                        cell
                    }
                })
                .map(SharedString::from)
                .collect::<Vec<_>>(),
        ))),
    }
}

fn headers(values: &[&str]) -> Vec<SharedString> {
    values.iter().map(|value| (*value).into()).collect()
}

fn field(values: &[String], index: usize) -> String {
    values.get(index).map_or(String::new(), |s| s.trim().into())
}

fn start(driver: &Competitor) -> String {
    let components = [10, 11, 12].map(|index| {
        driver
            .header_fields
            .get(index)
            .and_then(|value| value.trim().parse::<u32>().ok())
    });
    match components {
        [Some(hour), Some(minute), Some(second)] => format!("{hour:02}:{minute:02}:{second:02}"),
        _ => "-".into(),
    }
}

fn status(value: &str) -> String {
    match Status::from_legacy(value) {
        Status::Classified => "Tarkastettu",
        Status::Interrupted => "Keskeyttänyt",
        Status::Cancelled => "Peruuttanut",
        Status::Disqualified => "Hylätty",
        Status::Excluded => "Suljettu",
        Status::Registered => "Ilmoittautunut",
        Status::DidNotStart => "Ei lähtenyt",
        Status::Unknown(_) => return format!("Tuntematon: {value}"),
    }
    .into()
}

fn rule(value: i64) -> String {
    match value {
        0 => "Normaali",
        1 => "Ei ennakkoa",
        2 => "Ei ajanottoa",
        3 => "Ei myöhästymistä",
        4 => "Mitätöity",
        5 => "ULA: lähin minuutti",
        6 => "ULA: alaspäin",
        7 => "ULA: ylöspäin",
        _ => return format!("Tuntematon ({value})"),
    }
    .into()
}

/// Sivut: 1 kilpailijat, 2 lukuhistoria, 3 tulokset, 4 luokat, 5 radat.
pub fn data(
    ws: &Workspace,
    page: i32,
    selected_key: i32,
    filter: &str,
    result_group: &str,
) -> Result<ViewData, CoreError> {
    let archive = if page == 3 {
        ws.results()?
    } else {
        ws.archive()?
    };
    let mut view = ViewData::default();
    let mut notes = archive.warnings.clone();
    match page {
        1 => competitors(ws, &archive, &mut view),
        2 => history(ws, &archive, selected_key, &mut view, &mut notes),
        3 => results(
            ws,
            &archive,
            selected_key,
            &mut view,
            &mut notes,
            result_group,
        )?,
        4 => classes(&archive, &mut view),
        5 => tracks(&archive, selected_key, &mut view),
        _ => {}
    }
    let search = filter.trim().to_lowercase();
    if !search.is_empty() {
        use slint::Model;
        view.rows.retain(|row| {
            row.cells
                .iter()
                .any(|cell| cell.to_lowercase().contains(&search))
        });
    }
    notes.sort();
    notes.dedup();
    view.note_details = notes.join("\n");
    view.notes = summarize_notices(
        &notes,
        ws.project().is_none_or(|p| {
            p.luokat
                .iter()
                .any(|c| c.nousupisteet != 0 || c.sarjapistetapa != 0)
        }),
    );
    Ok(view)
}

/// Toistuvat viestit ryhmitellään; alkuperäinen diagnostiikka säilyy lisätiedoissa.
fn summarize_notices(notes: &[String], publish_points: bool) -> String {
    use std::collections::{BTreeMap, BTreeSet};
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut provisional = BTreeSet::new();
    for raw in notes {
        if raw.contains("Tuntematon tarkastustulos \"HYVÄKSYMÄTÖN\"")
            || raw.contains(
                "koodin etuliite tai päinvastoin; luokat erotetaan koko koodin perusteella",
            )
        {
            continue;
        }
        if raw.contains("Käsin kirjattavan aseman puuttuvasta leimasta ei määrätä puuttumissakkoa")
        {
            continue;
        }
        if raw.contains("Tulokset ovat alustavat: ILM-tiloja") {
            if !publish_points {
                continue;
            }
            if let Some(group) = raw
                .strip_prefix("Ryhmä ")
                .and_then(|s| s.split_once(": "))
                .map(|(group, _)| group)
            {
                provisional.insert(group.to_owned());
            }
            continue;
        }
        let Some(note) = as2026::report::user_notice(raw) else {
            continue;
        };
        if note.contains("Käsin kirjattavan aseman puuttuvasta leimasta ei määrätä puuttumissakkoa")
        {
            continue;
        }
        if let Some((driver, message)) = note
            .split_once(": ")
            .filter(|(prefix, _)| prefix.starts_with("Kilpailija "))
        {
            groups
                .entry(message.to_owned())
                .or_default()
                .insert(driver.trim_start_matches("Kilpailija ").to_owned());
        } else {
            groups.entry(note).or_default();
        }
    }
    let mut output = Vec::new();
    for (message, drivers) in groups {
        if drivers.is_empty() {
            output.push(message);
        } else {
            let mut drivers = drivers.into_iter().collect::<Vec<_>>();
            drivers.sort_by_key(|number| number.parse::<u32>().unwrap_or(u32::MAX));
            let mut numbers = drivers
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            if drivers.len() > 8 {
                numbers.push_str(&format!(" ja {} muuta", drivers.len() - 8));
            }
            output.push(format!("{message} — kilpailijat {numbers}"));
        }
    }
    if !provisional.is_empty() {
        output.push(format!(
            "Nousu- ja sarjapisteet odottavat luokkien valmistumista: {}.",
            provisional.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    output.join("\n\n")
}

fn competitors(ws: &Workspace, archive: &Archive, view: &mut ViewData) {
    let columns = ws
        .project()
        .and_then(|p| as2026::starts::columns(p).ok())
        .unwrap_or_else(|| as2026::starts::START_COLUMNS.to_vec());
    view.headers = headers(&columns.iter().map(|(_, label)| *label).collect::<Vec<_>>());
    view.weights = columns
        .iter()
        .map(|(id, _)| match *id {
            "numero" | "luokka" => 0.06,
            "emit" | "lahtoaika" => 0.10,
            _ => 0.17,
        })
        .collect();
    let total: f32 = view.weights.iter().sum();
    for weight in &mut view.weights {
        *weight /= total;
    }
    view.rows = archive
        .competitors
        .iter()
        .enumerate()
        .map(|(index, driver)| {
            row(
                index,
                columns
                    .iter()
                    .map(|(id, _)| match *id {
                        "numero" => driver.number.to_string(),
                        "luokka" => driver.class_code.clone(),
                        "emit" => driver.emit_card.clone(),
                        "lahtoaika" => start(driver),
                        "kuljettaja" => driver.driver.clone(),
                        "kuljettajan_seura" => field(&driver.header_fields, 7),
                        "kartturi" => driver.navigator.clone(),
                        "kartturin_seura" => field(&driver.header_fields, 8),
                        "auto" => field(&driver.header_fields, 9),
                        _ => String::new(),
                    })
                    .collect(),
            )
        })
        .collect();
}

fn classes(archive: &Archive, view: &mut ViewData) {
    view.headers = headers(&[
        "Tunnus",
        "Nimi",
        "Rata",
        "1. numero",
        "Lähtö",
        "Lähtöväli, min",
        "Myöh. raja, min",
    ]);
    view.weights = vec![0.08, 0.27, 0.09, 0.10, 0.12, 0.17, 0.17];
    view.rows = archive
        .basic
        .classes
        .iter()
        .enumerate()
        .map(|(index, class)| {
            row(
                index,
                vec![
                    class.code.clone(),
                    class.name.clone(),
                    if class.route == "0" {
                        "-".into()
                    } else {
                        archive
                            .routes
                            .iter()
                            .find(|r| r.index.to_string() == class.route)
                            .map_or_else(|| class.route.clone(), |r| r.name.clone())
                    },
                    field(&class.fields, 3),
                    field(&class.fields, 4).replace('.', ":"),
                    field(&class.fields, 5),
                    field(&class.fields, 7),
                ],
            )
        })
        .collect();
    for (index, group) in archive.basic.combined_classes.iter().enumerate() {
        let members = archive
            .basic
            .classes
            .iter()
            .enumerate()
            .filter(|(member, _)| field(group, member + 4) == "1")
            .map(|(_, class)| class.code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        view.rows.push(row(
            archive.basic.classes.len() + index,
            vec![
                format!("Y{}", index + 1),
                format!("{} · yhdistetty: {members}", field(group, 1)),
                "-".into(),
                "-".into(),
                "-".into(),
                "-".into(),
                "-".into(),
            ],
        ));
    }
}

fn tracks(archive: &Archive, selected_key: i32, view: &mut ViewData) {
    view.headers = headers(&["Radan nimi", "Asemia"]);
    view.weights = vec![0.82, 0.18];
    view.rows = archive
        .routes
        .iter()
        .enumerate()
        .map(|(index, track)| {
            row(
                index,
                vec![track.name.clone(), track.stations.len().to_string()],
            )
        })
        .collect();
    let Some(track) = usize::try_from(selected_key)
        .ok()
        .and_then(|key| archive.routes.get(key))
    else {
        return;
    };
    view.detail_title = track.name.clone();
    view.detail_headers = headers(&[
        "Järj.",
        "Tyyppi",
        "Aseman nimi",
        "Emit",
        "Tavoite, s",
        "Sääntö",
        "Jakso, min",
    ]);
    view.detail_weights = vec![0.06, 0.07, 0.26, 0.09, 0.15, 0.20, 0.17];
    view.detail_rows = track
        .stations
        .iter()
        .enumerate()
        .map(|(index, station)| {
            row(
                index,
                vec![
                    station.index.to_string(),
                    station.station_type.clone(),
                    station.name.clone(),
                    station.expected_code.to_string(),
                    station.seconds.to_string(),
                    rule(station.rule),
                    station.section_text.clone(),
                ],
            )
        })
        .collect();
}

fn result_row(
    project: Option<&as2026::native::Project>,
    archive: &Archive,
    index: usize,
    driver: &Competitor,
    position: String,
) -> TableRow {
    let native = project.and_then(|project| {
        project
            .kilpailijat
            .iter()
            .find(|d| d.numero == driver.number)
    });
    let pending = project
        .and_then(|project| {
            project
                .kilpailijat
                .iter()
                .find(|d| d.numero == driver.number)
        })
        .and_then(|driver| driver.tallennettu_tulos.as_ref())
        .is_some_and(|result| !result.can_approve());
    row(
        index,
        vec![
            position,
            driver.number.to_string(),
            driver.class_code.clone(),
            driver.driver.clone(),
            driver.navigator.clone(),
            if native.is_some_and(|d| d.emit_number().is_none() || d.lahtoaika.is_empty())
                && !archive.has_confirmed_interruption(driver.number)
                && driver.status == "ILM"
            {
                "Täydennettävä".into()
            } else if native.is_some_and(|d| {
                d.tila.as_ref().is_some_and(|s| s.tunnus == "KESK")
                    && !d.has_confirmed_interruption()
            }) {
                "Keskeytys vahvistamatta".into()
            } else if archive.result_publication.is_some()
                && !archive.has_approved_result(driver.number)
                && matches!(driver.status.as_str(), "TRK" | "KESK")
                && !archive.has_confirmed_interruption(driver.number)
            {
                "Täydennettävä tulos".into()
            } else {
                status(&driver.status)
            },
            if archive.has_confirmed_interruption(driver.number)
                || pending
                || driver.results.is_empty()
                || !matches!(
                    Status::from_legacy(&driver.status),
                    Status::Classified | Status::Interrupted
                )
            {
                "-".into()
            } else if !archive.has_approved_result(driver.number) {
                format!("{} p (esikatselu)", driver.total_penalties)
            } else {
                format!("{} p", driver.total_penalties)
            },
            driver.emit_card.clone(),
        ],
    )
}

fn results(
    ws: &Workspace,
    archive: &Archive,
    selected_key: i32,
    view: &mut ViewData,
    notes: &mut Vec<String>,
    result_group: &str,
) -> Result<(), CoreError> {
    let result_project = ws.result_project()?;
    view.headers = headers(&[
        "Sija",
        "Nro",
        "Luokka",
        "Kuljettaja",
        "Kartanlukija",
        "Tila",
        "Tulos",
        "Emit",
    ]);
    view.weights = vec![0.05, 0.05, 0.07, 0.20, 0.20, 0.14, 0.19, 0.10];
    let ranking = application::rank_stored(archive, TiePolicy::KeepSharedAndFlag)?;
    let combined = ranking
        .groups
        .iter()
        .any(|g| g.kind == GroupKind::Combined && g.code == result_group);
    for group in ranking.groups.iter().filter(|group| {
        if combined {
            group.code == result_group
        } else {
            group.kind == GroupKind::Ordinary
        }
    }) {
        for entry in &group.entries {
            let Some(index) = entry.source_index.checked_sub(1) else {
                continue;
            };
            if let Some(driver) = archive.competitors.get(index) {
                let position = if entry.status == Status::Classified {
                    entry.primary_position.to_string()
                } else {
                    "-".into()
                };
                view.rows.push(result_row(
                    result_project.as_ref(),
                    archive,
                    index,
                    driver,
                    position,
                ));
            }
        }
    }
    for source_index in &ranking.omitted_source_indices {
        if let Some(index) = source_index.checked_sub(1)
            && let Some(driver) = archive.competitors.get(index)
        {
            view.rows.push(result_row(
                result_project.as_ref(),
                archive,
                index,
                driver,
                "-".into(),
            ));
        }
    }
    notes.extend(ranking.diagnostics.iter().map(ToString::to_string));
    if !ranking.omitted_source_indices.is_empty() {
        notes.push(format!("{} kilpailijan tulos odottaa hyväksyntää tai täydentämistä. Valitse kilpailija tulosluettelosta.", ranking.omitted_source_indices.len()));
    }

    let Some(driver) = usize::try_from(selected_key)
        .ok()
        .and_then(|index| archive.competitors.get(index))
    else {
        return Ok(());
    };
    let native = result_project.as_ref().and_then(|project| {
        project
            .kilpailijat
            .iter()
            .find(|d| d.numero == driver.number)
    });
    let interrupted = native
        .and_then(|d| d.tila.as_ref())
        .is_some_and(|s| s.tunnus == "KESK");
    let interruption_confirmed = native.is_some_and(|d| d.has_confirmed_interruption());
    let (snapshot, snapshot_error) = match ws.result_snapshot(driver.number) {
        Ok(snapshot) => (snapshot, None),
        Err(error) => (None, Some(error.to_string())),
    };
    let pending_clocks = snapshot
        .as_ref()
        .map_or_else(Vec::new, |result| result.pending_manual_clocks());
    let calculated = !driver.results.is_empty()
        && snapshot_error.is_none()
        && snapshot.as_ref().is_none_or(|result| result.can_approve());
    let points = |index: usize| -> SharedString {
        // Korttiluvusta laskettuja aikapisteitä ei näytetä ilman laskentatulosta,
        // mutta kilpailijalle käsin tallennetut lisävirheet ovat silti tiedossa.
        if interrupted || (!calculated && matches!(index, 19 | 20)) {
            return "-".into();
        }
        driver
            .header_fields
            .get(index)
            .and_then(|field| field.trim().parse::<i64>().ok())
            .map_or_else(|| "-".into(), |value| format!("{value} p").into())
    };
    let clock = |offset: usize| -> SharedString {
        if !calculated || interruption_confirmed {
            return "-".into();
        }
        let fields = (0..3)
            .map(|i| {
                driver
                    .header_fields
                    .get(offset + i)
                    .and_then(|field| field.trim().parse::<u32>().ok())
            })
            .collect::<Vec<_>>();
        match fields.as_slice() {
            [Some(h), Some(m), Some(s)] if *m < 60 && *s < 60 => {
                format!("{h:02}:{m:02}:{s:02}").into()
            }
            _ => "-".into(),
        }
    };
    let quantity = |index: usize| -> SharedString {
        // Lisävirheiden lukumäärät ovat tallennettuja tietoja, eivät
        // korttiluvusta keksittyjä pisteitä. KESK ei kuitenkaan näytä niitä.
        if interrupted {
            return "-".into();
        }
        driver
            .header_fields
            .get(index)
            .and_then(|value| value.trim().parse::<u32>().ok())
            .map_or_else(|| "-".into(), |value| value.to_string().into())
    };
    let read_state = if interrupted {
        if interruption_confirmed {
            "Keskeytys vahvistettu".to_owned()
        } else {
            "Keskeytys odottaa vahvistusta".to_owned()
        }
    } else {
        result_project.as_ref().map_or_else(
            || "AS2015A-aineisto · vain katselu".to_string(),
            |project| {
                if driver
                    .emit_card
                    .parse::<u32>()
                    .ok()
                    .is_some_and(|card| project.active_readout(card).is_some())
                {
                    if calculated && snapshot.as_ref().is_some_and(|result| result.hyvaksytty) {
                        "Tulos valmis".to_owned()
                    } else if calculated {
                        "Emit-kortti luettu · täydennä ajat ja tulosta tulos".to_owned()
                    } else {
                        "Emit-kortti luettu · tulos vaatii tarkastuksen (katso huomautukset)"
                            .to_owned()
                    }
                } else {
                    "Emit-korttia ei ole vielä luettu".to_owned()
                }
            },
        )
    };
    // A calculation failure must be visible next to the selected competitor,
    // not buried among other competitors' diagnostics at the bottom of the page.
    let missing_result_prefix = format!("Kilpailijan {} tulos puuttuu: ", driver.number);
    let mut issues: Vec<String> = if interrupted {
        Vec::new()
    } else {
        archive
            .warnings
            .iter()
            .find_map(|warning| warning.strip_prefix(&missing_result_prefix))
            .map(str::to_owned)
            .into_iter()
            .collect()
    };
    if interrupted && !interruption_confirmed {
        issues.push(
            "Tarkista keskeytyksen syy kilpailijan tiedoista ja valitse Tulosta tulos.".into(),
        );
    }
    if !interrupted && let Some(result) = &snapshot {
        if !pending_clocks.is_empty() {
            issues.push(format!(
                "Täydennä käsikellot: {}. Valitse Tulosta tulos.",
                pending_clocks
                    .iter()
                    .map(|index| driver
                        .results
                        .iter()
                        .find(|row| row.output_index == *index)
                        .map_or_else(
                            || format!("rivi {index}"),
                            |row| format!("asema {} {}", row.sequence, row.name)
                        ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        issues.extend(result.virheelliset_ajat.iter().cloned());
    }
    if !interrupted
        && let Some(error) = snapshot_error
        && !issues.contains(&error)
    {
        issues.push(error);
    }
    view.result_overview = ResultOverview {
        selected: true,
        identity: format!("Nro {} · {}", driver.number, driver.driver).into(),
        details: format!(
            "Luokka {} · Kartanlukija {} · Emit {}",
            driver.class_code, driver.navigator, driver.emit_card
        )
        .into(),
        status: if interrupted {
            "Keskeyttänyt".into()
        } else if archive.result_publication.is_some()
            && !archive.has_approved_result(driver.number)
            && matches!(driver.status.as_str(), "TRK" | "KESK")
        {
            "Täydennettävä tulos".into()
        } else {
            status(&driver.status).into()
        },
        reason: if interrupted {
            native
                .and_then(|d| d.tila.as_ref())
                .map_or("", |s| s.syy.as_str())
                .into()
        } else {
            field(&driver.header_fields, 29).into()
        },
        read_state: read_state.into(),
        issue: issues.join("\n").into(),
        start: start(driver).into(),
        elapsed: clock(13),
        finish: clock(16),
        time_points: points(19),
        station_points: points(20),
        missing_rt: points(22),
        missing_rt_count: quantity(21),
        extra_rt: points(24),
        extra_rt_count: quantity(23),
        other_points: points(25),
        total: if calculated && !interruption_confirmed {
            if archive.has_approved_result(driver.number) {
                format!("{} p", driver.total_penalties).into()
            } else {
                format!("{} p (esikatselu)", driver.total_penalties).into()
            }
        } else {
            "-".into()
        },
    };
    view.detail_title = format!(
        "Nro {} · {} / {} · {}",
        driver.number,
        driver.driver,
        driver.navigator,
        if ws.is_editable() {
            if ws
                .project()
                .and_then(|p| p.kilpailijat.iter().find(|d| d.numero == driver.number))
                .is_some_and(|d| d.tallennettu_tulos.is_some())
            {
                "tallennetut asematulokset"
            } else {
                "lasketut asematulokset"
            }
        } else {
            "tallennetut asematulokset"
        }
    );
    view.detail_headers = headers(&[
        "Järj.",
        "Asema",
        "Emit od./luettu",
        "Tavoite, s",
        "Kello",
        "Aika, s",
        "Aikavirhe, p",
        "Muut, p",
        "Yhteensä, p",
        "Huomautus",
    ]);
    view.detail_weights = vec![0.04, 0.16, 0.08, 0.07, 0.10, 0.07, 0.09, 0.07, 0.09, 0.23];
    view.detail_rows = driver
        .results
        .iter()
        .enumerate()
        .map(|(index, station)| {
            let pending = pending_clocks.contains(&station.output_index);
            row(
                index,
                vec![
                    station.output_index.to_string(),
                    format!("{} {}", station.station_type, station.name),
                    format!(
                        "{} / {}",
                        station.expected_code,
                        if station.received_code == 0 {
                            "-".into()
                        } else {
                            station.received_code.to_string()
                        }
                    ),
                    station.target_seconds.to_string(),
                    if pending {
                        "-".into()
                    } else {
                        format!(
                            "{:02}:{:02}:{:02}",
                            station.clock[0], station.clock[1], station.clock[2]
                        )
                    },
                    if pending {
                        "-".into()
                    } else {
                        station.actual_seconds.to_string()
                    },
                    if pending {
                        "-".into()
                    } else {
                        station.signed_time_penalty.to_string()
                    },
                    station.other_penalty.to_string(),
                    if pending {
                        "-".into()
                    } else {
                        station.total_penalty.to_string()
                    },
                    if pending {
                        "Käsikello puuttuu".into()
                    } else {
                        station.note.clone()
                    },
                ],
            )
        })
        .collect();
    Ok(())
}

fn card_owner(archive: &Archive, card: u32) -> String {
    let owners: Vec<_> = archive
        .competitors
        .iter()
        .filter(|driver| driver.emit_card.trim().parse::<u32>().ok() == Some(card))
        .collect();
    match owners.as_slice() {
        [driver] => format!("{} · {}", driver.number, driver.driver),
        [] => "Ei vielä kilpailijaa".into(),
        _ => "Kortti usealla kilpailijalla".into(),
    }
}

fn punches_detail(view: &mut ViewData, card: u32, clock: &str, punches: &[Punch]) {
    view.detail_title = format!("Emit {card} · {clock} · luettu sisältö");
    view.detail_headers = headers(&["Järj.", "Koodi", "Aika, s", "Tulkinta"]);
    view.detail_weights = vec![0.12, 0.18, 0.20, 0.50];
    view.detail_rows = punches
        .iter()
        .enumerate()
        .map(|(index, punch)| {
            row(
                index,
                vec![
                    (index + 1).to_string(),
                    punch.code.to_string(),
                    punch.elapsed_seconds.to_string(),
                    if punch.code == 250 {
                        "Lukumerkki"
                    } else if index == 0 {
                        "Ankkurileima"
                    } else {
                        "Leimaus"
                    }
                    .into(),
                ],
            )
        })
        .collect();
}

fn history(
    ws: &Workspace,
    archive: &Archive,
    selected_key: i32,
    view: &mut ViewData,
    notes: &mut Vec<String>,
) {
    view.headers = headers(&[
        "Luettu",
        "Emit",
        "Kilpailija",
        "Tapahtuma",
        "Leimoja",
        "Lukuja",
        "Käyttö",
    ]);
    view.weights = vec![0.18, 0.10, 0.25, 0.18, 0.07, 0.07, 0.15];
    if let Some(project) = ws.project() {
        struct Group {
            // Ensimmäinen alkuperäinen indeksi pitää rivivalinnan vakaana uusintaluvuissa.
            first: usize,
            latest: usize,
            content: Option<Vec<Punch>>,
            event: &'static str,
        }
        let mut previous: HashMap<u32, Vec<Punch>> = HashMap::new();
        let mut active = HashMap::new();
        let mut groups: Vec<Group> = Vec::new();
        for (index, read) in project.leimaluvut.iter().enumerate() {
            let (content, event) = match read.content() {
                Ok(content) => {
                    let event = if previous.get(&read.emit).is_some_and(|old| *old != content) {
                        "Leimat päivitetty"
                    } else {
                        "Kortti luettu"
                    };
                    previous.insert(read.emit, content.clone());
                    active.insert(read.emit, index);
                    (Some(content), event)
                }
                Err(error) => {
                    notes.push(format!(
                        "Kortin {} lukukerta {} ohitetaan laskennassa: {error}",
                        read.emit,
                        index + 1
                    ));
                    (None, "Virheellinen lukema")
                }
            };
            if let Some(group) = groups.last_mut()
                && content.is_some()
                && project.leimaluvut[group.latest].emit == read.emit
                && group.content == content
            {
                group.latest = index;
            } else {
                groups.push(Group {
                    first: index,
                    latest: index,
                    content,
                    event,
                });
            }
        }
        for group in groups.iter().rev().take(50) {
            let read = &project.leimaluvut[group.latest];
            let clock = read
                .luettu_unix_ms
                .and_then(|value| i64::try_from(value).ok())
                .and_then(chrono::DateTime::from_timestamp_millis)
                .map(|time| {
                    time.with_timezone(&chrono::Local)
                        .format("%d.%m.%Y %H:%M:%S")
                        .to_string()
                })
                .unwrap_or_else(|| read.lukukello.clone());
            view.rows.push(row(
                group.first,
                vec![
                    clock,
                    read.emit.to_string(),
                    card_owner(archive, read.emit),
                    group.event.into(),
                    group
                        .content
                        .as_ref()
                        .map_or_else(|| "-".into(), |p| p.len().to_string()),
                    (group.latest - group.first + 1).to_string(),
                    if active.get(&read.emit) == Some(&group.latest) {
                        "Aktiivinen"
                    } else {
                        "Historia"
                    }
                    .into(),
                ],
            ));
        }
        // Ryhmän tarkastelu näyttää viimeisimmän fyysisen luvun, myös sen lukumerkin.
        if let Some(read) = usize::try_from(selected_key)
            .ok()
            .and_then(|key| groups.iter().find(|g| (g.first..=g.latest).contains(&key)))
            .and_then(|group| project.leimaluvut.get(group.latest))
        {
            let punches: Vec<_> = read
                .leimat
                .iter()
                .map(|punch| Punch {
                    code: punch.koodi,
                    elapsed_seconds: punch.aika_sekuntia,
                })
                .collect();
            punches_detail(view, read.emit, &read.lukukello, &punches);
            if let Some(driver) = project.kilpailijat.iter().find(|d| d.emit == read.emit) {
                // Only the active card content can be edited. Older distinct reads stay raw.
                let active = project
                    .active_readout(read.emit)
                    .is_some_and(|r| r.content().ok() == read.content().ok());
                if active && let Ok(rows) = as2026::emit_corrections::rows(project, driver.numero) {
                    use as2026::emit_corrections::{clock_text, punch_clock};
                    view.detail_title = format!(
                        "Emit {} · {} · luettu sisältö",
                        read.emit, driver.kuljettaja
                    );
                    view.detail_headers =
                        headers(&["Asema", "Koodi", "Aika (s)", "Aika (hh:mm:ss)"]);
                    view.detail_weights = vec![0.38, 0.14, 0.20, 0.28];
                    view.detail_rows = vec![row(
                        usize::MAX,
                        vec![
                            "0 · Lähtö".into(),
                            "0".into(),
                            "0".into(),
                            driver.lahtoaika.clone(),
                        ],
                    )];
                    view.detail_rows
                        .extend(rows.into_iter().filter(|r| r.expected_code != 0).map(|r| {
                            let clock = if r.code == 0 {
                                String::new()
                            } else {
                                punch_clock(project, driver.numero, i64::from(r.elapsed_seconds))
                                    .map(clock_text)
                                    .unwrap_or_default()
                            };
                            row(
                                r.index as usize,
                                vec![
                                    r.name,
                                    r.code.to_string(),
                                    r.elapsed_seconds.to_string(),
                                    clock,
                                ],
                            )
                        }));
                    if let Some(marker) = read.leimat.iter().find(|p| p.koodi == 250) {
                        view.detail_rows.push(row(
                            usize::MAX - 1,
                            vec![
                                "Lukumerkki".into(),
                                "250".into(),
                                marker.aika_sekuntia.to_string(),
                                String::new(),
                            ],
                        ));
                    }
                }
            }
        }
        if groups.len() > 50 {
            notes.push(format!(
                "Näytetään 50 uusinta lukutapahtumaa; kilpailutiedosto säilyttää kaikki {} lukukertaa.",
                project.leimaluvut.len()
            ));
        }
    } else if let Some(document) = archive.documents.get("emit.txt") {
        match as2026::legacy::parse_saved_emit(document) {
            Ok(reads) => {
                for (index, read) in reads.iter().enumerate().rev().take(50) {
                    let accepted = read.accepted();
                    let content = accepted
                        .as_ref()
                        .map_or("Virheellinen lukema", |_| "Tuotu lukutapahtuma");
                    if let Err(error) = accepted {
                        notes.push(format!("emit.txt rivi {}: {error}", read.line));
                    }
                    let clock = format!(
                        "{:02}:{:02}:{:02}",
                        read.readout_clock.hour,
                        read.readout_clock.minute,
                        read.readout_clock.second
                    );
                    view.rows.push(row(
                        index,
                        vec![
                            clock,
                            read.card_number.to_string(),
                            card_owner(archive, read.card_number),
                            content.into(),
                            read.records.len().to_string(),
                            "1".into(),
                            "Tuontiaineisto".into(),
                        ],
                    ));
                }
                if let Some(read) = usize::try_from(selected_key)
                    .ok()
                    .and_then(|key| reads.get(key))
                {
                    let clock = format!(
                        "{:02}:{:02}:{:02}",
                        read.readout_clock.hour,
                        read.readout_clock.minute,
                        read.readout_clock.second
                    );
                    punches_detail(view, read.card_number, &clock, &read.records);
                }
            }
            Err(error) => notes.push(format!("Lukuhistorian avaaminen epäonnistui: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use as2026::{native::Project, storage::ProjectStore};
    use slint::Model;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn notices_group_repetition_without_losing_full_diagnostics() {
        let mut notes = (1..=70)
            .map(|n| format!("Kilpailija {n}: Maalileima puuttuu."))
            .collect::<Vec<_>>();
        notes.push("Tuntematon tarkastustulos \"HYVÄKSYMÄTÖN\"; tietue jätettiin tulosluettelon ulkopuolelle".into());
        let summary = summarize_notices(&notes, false);
        assert_eq!(summary.lines().count(), 1);
        assert!(summary.contains("Maalileima puuttuu."));
        assert!(summary.contains("62 muuta"));
        assert!(summary.contains("1, 2, 3, 4, 5, 6, 7, 8"));
        assert!(!summary.contains("Tuntematon tarkastustulos"));
        assert_eq!(notes.len(), 71);
    }

    #[test]
    fn all_export_formats_respect_approval_and_can_be_read_back() {
        let mut fixture = Fixture::new(|_| {});
        let csv =
            String::from_utf8(super::super::export_bytes(&fixture.workspace, 1).unwrap()).unwrap();
        assert!(!csv.contains("Esimerkkikuljettaja"));
        fixture.workspace.review_results(7).unwrap();
        let finish = fixture
            .workspace
            .result_snapshot(7)
            .unwrap()
            .unwrap()
            .rivit
            .last()
            .unwrap()
            .output_index;
        fixture
            .workspace
            .complete_result_entry(7, &[(finish, [12, 11, 0])], 0, 0)
            .unwrap();
        for kind in 0..=2 {
            let bytes = super::super::export_bytes(&fixture.workspace, kind).unwrap();
            if kind == 0 {
                use std::io::Read;
                let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
                let mut xml = String::new();
                zip.by_name("xl/worksheets/sheet1.xml")
                    .unwrap()
                    .read_to_string(&mut xml)
                    .unwrap();
                assert!(xml.contains("Esimerkkikuljettaja"));
            } else {
                assert!(
                    String::from_utf8(bytes)
                        .unwrap()
                        .contains("Esimerkkikuljettaja")
                );
            }
        }
    }

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        directory: PathBuf,
        workspace: Workspace,
    }

    impl Fixture {
        fn new(edit: impl FnOnce(&mut Project)) -> Self {
            let mut project =
                Project::from_toml(include_str!("../../fixtures/native-demo/kilpailu.toml"))
                    .unwrap();
            edit(&mut project);
            let directory = std::env::temp_dir().join(format!(
                "as2026-gui-tables-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&directory).unwrap();
            let path = directory.join("kilpailu.toml");
            ProjectStore::create(&path, project).unwrap();
            let workspace = Workspace::open(&path).unwrap();
            Self {
                directory,
                workspace,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.directory).unwrap();
        }
    }

    fn text(row: &TableRow, column: usize) -> String {
        row.cells.row_data(column).unwrap().to_string()
    }

    #[test]
    fn combined_class_rows_keep_their_keys_when_filtered() {
        let fixture = Fixture::new(|project| {
            project
                .yhdistetyt_luokat
                .push(as2026::native::CombinedClass {
                    nimi: "Yhteistulos".into(),
                    luokat: vec!["A".into()],
                    nousupisteet: 0,
                    sarjapistetapa: 0,
                });
        });
        let view = data(&fixture.workspace, 4, -1, "Yhteistulos").unwrap();
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].key, 1);
        assert!(text(&view.rows[0], 1).contains("yhdistetty: A"));
    }

    #[test]
    fn filtering_and_result_order_keep_original_model_keys() {
        let mut fixture = Fixture::new(|project| {
            let mut second = project.kilpailijat[0].clone();
            second.numero = 2;
            second.kuljettaja = "Toinen koe".into();
            second.emit = 654321;
            project.kilpailijat.push(second);
        });
        fixture.workspace.review_results(7).unwrap();
        let finish = fixture
            .workspace
            .result_snapshot(7)
            .unwrap()
            .unwrap()
            .rivit
            .last()
            .unwrap()
            .output_index;
        fixture
            .workspace
            .edit_station_result(7, finish, as2026::workspace::ResultField::Clock, "12:10:00")
            .unwrap();
        fixture.workspace.recalculate_stored(7).unwrap();
        fixture.workspace.approve_result(7).unwrap();
        let filtered = data(&fixture.workspace, 1, -1, "TOINEN").unwrap();
        assert_eq!(filtered.rows.len(), 1);
        assert_eq!(filtered.rows[0].key, 1);
        assert_eq!(text(&filtered.rows[0], 0), "2");
        let by_card = data(&fixture.workspace, 1, -1, "654321").unwrap();
        assert_eq!(by_card.rows[0].key, 1);
        let results = data(&fixture.workspace, 3, 0, "").unwrap();
        assert_eq!(results.rows.len(), 2);
        assert_eq!(results.rows[0].key, 0);
        assert_eq!(results.rows[1].key, 1);
        assert_eq!(text(&results.rows[1], 6), "-");
        assert!(!results.detail_rows.is_empty());
        assert!(!results.notes.contains("AS2015A"));
        assert_eq!(results.result_overview.finish, "12:10:00");
    }

    #[test]
    fn result_overview_tracks_selection_without_inventing_points() {
        let fixture = Fixture::new(|project| {
            let mut second = project.kilpailijat[0].clone();
            second.numero = 2;
            second.kuljettaja = "Ei leimoja".into();
            second.emit = 77777;
            project.kilpailijat.push(second);
        });
        let no_selection = data(&fixture.workspace, 3, -1, "").unwrap();
        assert!(!no_selection.result_overview.selected);
        let initial = data(&fixture.workspace, 3, 0, "").unwrap();
        assert!(initial.result_overview.selected);
        assert!(initial.result_overview.identity.contains("Nro 7"));
        let empty = data(&fixture.workspace, 3, 1, "").unwrap();
        assert!(empty.result_overview.identity.contains("Ei leimoja"));
        assert_eq!(empty.result_overview.total.to_string(), "-");
        assert_eq!(empty.result_overview.elapsed.to_string(), "-");
        assert_eq!(empty.result_overview.finish.to_string(), "-");
        assert_eq!(empty.result_overview.time_points.to_string(), "-");
        // Ensimmäisen kilpailijan puuttuva käsimaali ei kuulu lukemattoman
        // toisen kilpailijan huomautukseksi.
        assert!(!empty.result_overview.issue.contains("Syötä käsikellot"));
        assert!(
            empty
                .result_overview
                .read_state
                .contains("ei ole vielä luettu")
        );
    }

    #[test]
    fn result_overview_shows_known_additions_and_reason_without_a_readout() {
        let fixture = Fixture::new(|project| {
            project.leimaluvut.clear();
            let driver = &mut project.kilpailijat[0];
            driver.lisavirheet.puuttuvat_rt_kpl = 3;
            driver.lisavirheet.puuttuvat_rt_pisteet = 1800;
            driver.lisavirheet.ylimaaraiset_rt_kpl = 2;
            driver.lisavirheet.ylimaaraiset_rt_pisteet = 100;
            driver.lisavirheet.muut_virhepisteet = 75;
            driver.tila = Some(as2026::native::ManualStatus::new("HYL", "Väärä kalusto").unwrap());
        });
        let data = data(&fixture.workspace, 3, 0, "").unwrap();
        let summary = data.result_overview;
        assert_eq!(summary.missing_rt_count, "3");
        assert_eq!(summary.missing_rt, "1800 p");
        assert_eq!(summary.extra_rt_count, "2");
        assert_eq!(summary.extra_rt, "100 p");
        assert_eq!(summary.other_points, "75 p");
        assert_eq!(summary.status, "Hylätty");
        assert_eq!(summary.reason, "Väärä kalusto");
        assert_eq!(summary.total, "-");
        assert_eq!(summary.finish, "-");
        assert_eq!(summary.start, "12:00:00");
    }

    #[test]
    fn pending_manual_clock_is_blank_but_entered_midnight_is_visible() {
        let mut fixture = Fixture::new(|_| {});
        let view = data(&fixture.workspace, 3, 0, "").unwrap();
        assert_eq!(view.result_overview.total, "-");
        assert_eq!(view.result_overview.finish, "-");
        assert_eq!(text(view.detail_rows.last().unwrap(), 4), "-");
        assert_eq!(
            text(view.detail_rows.last().unwrap(), 9),
            "Käsikello puuttuu"
        );
        fixture.workspace.review_results(7).unwrap();
        let finish = fixture
            .workspace
            .result_snapshot(7)
            .unwrap()
            .unwrap()
            .rivit
            .last()
            .unwrap()
            .output_index;
        fixture
            .workspace
            .edit_station_result(7, finish, as2026::workspace::ResultField::Clock, "00:00:00")
            .unwrap();
        let entered = data(&fixture.workspace, 3, 0, "").unwrap();
        assert_eq!(text(entered.detail_rows.last().unwrap(), 4), "00:00:00");
        assert_eq!(entered.result_overview.total, "-");
    }

    #[test]
    fn manual_interruption_has_its_own_confirmation_state_without_readout() {
        let mut fixture = Fixture::new(|project| {
            project.leimaluvut.clear();
            project.kilpailijat[0].tila =
                Some(as2026::native::ManualStatus::new("KESK", "Moottorivika").unwrap());
        });
        let pending = data(&fixture.workspace, 3, 0, "").unwrap();
        assert_eq!(text(&pending.rows[0], 0), "-");
        assert_eq!(text(&pending.rows[0], 5), "Keskeytys vahvistamatta");
        assert_eq!(
            pending.result_overview.read_state,
            "Keskeytys odottaa vahvistusta"
        );
        assert_eq!(pending.result_overview.reason, "Moottorivika");
        assert_eq!(pending.result_overview.total, "-");
        assert_eq!(pending.result_overview.finish, "-");
        assert!(!pending.result_overview.issue.contains("kortti"));

        fixture
            .workspace
            .confirm_interruption(7, "Moottorivika")
            .unwrap();
        let confirmed = data(&fixture.workspace, 3, 0, "").unwrap();
        assert_eq!(text(&confirmed.rows[0], 0), "-");
        assert_eq!(text(&confirmed.rows[0], 5), "Keskeyttänyt");
        assert_eq!(
            confirmed.result_overview.read_state,
            "Keskeytys vahvistettu"
        );
        assert_eq!(confirmed.result_overview.reason, "Moottorivika");
        assert_eq!(confirmed.result_overview.finish, "-");
        assert_eq!(confirmed.result_overview.total, "-");
        assert!(confirmed.result_overview.issue.is_empty());
    }

    #[test]
    fn consecutive_repeats_group_without_hiding_changes_or_bad_reads() {
        let fixture = Fixture::new(|project| {
            let original = project.leimaluvut[0].clone();
            let mut repeated = original.clone();
            repeated.leimat.last_mut().unwrap().aika_sekuntia += 123;
            project.leimaluvut.push(repeated);
            let mut changed = original.clone();
            changed.leimat[1].aika_sekuntia += 1;
            project.leimaluvut.push(changed);
            let mut unknown = original.clone();
            unknown.emit = 555;
            project.leimaluvut.push(unknown);
            let mut invalid = original;
            invalid.leimat.clear();
            project.leimaluvut.push(invalid);
        });
        let view = data(&fixture.workspace, 2, 1, "").unwrap();
        assert_eq!(
            view.rows.iter().map(|row| row.key).collect::<Vec<_>>(),
            vec![4, 3, 2, 0]
        );
        assert_eq!(text(&view.rows[0], 3), "Virheellinen lukema");
        assert_eq!(text(&view.rows[1], 2), "Ei vielä kilpailijaa");
        assert_eq!(text(&view.rows[2], 3), "Leimat päivitetty");
        assert_eq!(text(&view.rows[2], 6), "Aktiivinen");
        assert_eq!(text(&view.rows[3], 3), "Kortti luettu");
        assert_eq!(text(&view.rows[3], 5), "2");
        assert_eq!(fixture.workspace.project().unwrap().leimaluvut.len(), 5);
        assert_eq!(view.detail_rows.len(), 5);
        assert_eq!(text(view.detail_rows.last().unwrap(), 3), "Lukumerkki");
        assert_eq!(text(view.detail_rows.last().unwrap(), 2), "903");
        assert!(view.notes.contains("ohitetaan laskennassa"));
    }

    #[test]
    fn different_cards_break_groups_and_unregistered_cards_can_be_linked_later() {
        let mut fixture = Fixture::new(|project| {
            let original = project.leimaluvut[0].clone();
            let mut other = original.clone();
            other.emit = 555;
            project.leimaluvut.push(other);
            project.leimaluvut.push(original);
        });
        let view = data(&fixture.workspace, 2, 1, "").unwrap();
        assert_eq!(
            view.rows.iter().map(|row| row.key).collect::<Vec<_>>(),
            vec![2, 1, 0]
        );
        assert_eq!(text(&view.rows[1], 2), "Ei vielä kilpailijaa");
        fixture
            .workspace
            .edit_driver(7, as2026::workspace::DriverField::Card, "555")
            .unwrap();
        let linked = data(&fixture.workspace, 2, 1, "").unwrap();
        assert_eq!(text(&linked.rows[1], 2), "7 · Esimerkkikuljettaja");
        assert_eq!(linked.detail_rows.len(), 5);
        assert_eq!(text(&linked.rows[2], 6), "Historia");
        assert_eq!(text(&linked.rows[0], 6), "Aktiivinen");
    }

    #[test]
    fn many_repeated_frames_keep_one_stable_row_and_latest_details() {
        let fixture = Fixture::new(|project| {
            let original = project.leimaluvut[0].clone();
            for marker in 781..=880 {
                let mut repeated = original.clone();
                repeated.leimat.last_mut().unwrap().aika_sekuntia = marker;
                repeated.lukukello = "12:12:00".into();
                project.leimaluvut.push(repeated);
            }
        });
        let view = data(&fixture.workspace, 2, 0, "").unwrap();
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].key, 0);
        assert_eq!(text(&view.rows[0], 0), "12:12:00");
        assert_eq!(text(&view.rows[0], 5), "101");
        assert_eq!(text(&view.rows[0], 6), "Aktiivinen");
        assert_eq!(text(view.detail_rows.last().unwrap(), 2), "880");
        assert_eq!(fixture.workspace.project().unwrap().leimaluvut.len(), 101);
    }

    #[test]
    fn column_counts_and_weights_match_each_page_and_detail() {
        let fixture = Fixture::new(|_| {});
        for page in 1..=5 {
            let view = data(&fixture.workspace, page, 0, "").unwrap();
            assert_eq!(view.headers.len(), view.weights.len());
            assert!((view.weights.iter().sum::<f32>() - 1.0).abs() < 0.0001);
            for row in &view.rows {
                assert_eq!(row.cells.row_count(), view.headers.len());
            }
            assert_eq!(view.detail_headers.len(), view.detail_weights.len());
            if !view.detail_weights.is_empty() {
                assert!((view.detail_weights.iter().sum::<f32>() - 1.0).abs() < 0.0001);
            }
            for row in &view.detail_rows {
                assert_eq!(row.cells.row_count(), view.detail_headers.len());
            }
        }
    }
}
