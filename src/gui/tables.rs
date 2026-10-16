//! Taulukoiden sisältö johdetaan avatusta kilpailusta. Rivin avain pysyy
//! alkuperäisen mallin indeksinä myös haussa ja tulosten järjestämisessä.

use super::TableRow;
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
}

fn row(index: usize, cells: Vec<String>) -> TableRow {
    TableRow {
        key: index as i32,
        cells: ModelRc::from(Rc::new(VecModel::from(
            cells
                .into_iter()
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
        _ => "—".into(),
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
        3 => results(ws, &archive, selected_key, &mut view, &mut notes)?,
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
    view.notes = notes
        .iter()
        .filter_map(|note| as2026::report::user_notice(note))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(view)
}

fn competitors(ws: &Workspace, archive: &Archive, view: &mut ViewData) {
    view.headers = headers(&[
        "Nro",
        "Luokka",
        "Kuljettaja",
        "Kartanlukija",
        "Emit",
        "Auto",
        "Lähtö",
        if ws.is_editable() {
            "Korttiluku"
        } else {
            "Tila"
        },
    ]);
    view.weights = vec![0.06, 0.07, 0.20, 0.18, 0.10, 0.13, 0.10, 0.16];
    view.rows = archive
        .competitors
        .iter()
        .enumerate()
        .map(|(index, driver)| {
            let state = match ws.project() {
                Some(project) => {
                    let card = driver.emit_card.parse::<u32>().ok();
                    if card.is_some_and(|card| project.active_readout(card).is_some()) {
                        "Luettu".into()
                    } else {
                        "Ei luettu".into()
                    }
                }
                None => status(&driver.status),
            };
            row(
                index,
                vec![
                    driver.number.to_string(),
                    driver.class_code.clone(),
                    driver.driver.clone(),
                    driver.navigator.clone(),
                    driver.emit_card.clone(),
                    field(&driver.header_fields, 9),
                    start(driver),
                    state,
                ],
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
                    class.route.clone(),
                    field(&class.fields, 3),
                    field(&class.fields, 4).replace('.', ":"),
                    field(&class.fields, 5),
                    field(&class.fields, 7),
                ],
            )
        })
        .collect();
}

fn tracks(archive: &Archive, selected_key: i32, view: &mut ViewData) {
    view.headers = headers(&["Numero", "Radan nimi", "Asemia"]);
    view.weights = vec![0.14, 0.68, 0.18];
    view.rows = archive
        .routes
        .iter()
        .enumerate()
        .map(|(index, track)| {
            row(
                index,
                vec![
                    track.index.to_string(),
                    track.name.clone(),
                    track.stations.len().to_string(),
                ],
            )
        })
        .collect();
    let Some(track) = usize::try_from(selected_key)
        .ok()
        .and_then(|key| archive.routes.get(key))
    else {
        return;
    };
    view.detail_title = format!("Rata {} · {}", track.index, track.name);
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

fn result_row(index: usize, driver: &Competitor, position: String) -> TableRow {
    row(
        index,
        vec![
            position,
            driver.number.to_string(),
            driver.class_code.clone(),
            driver.driver.clone(),
            driver.navigator.clone(),
            status(&driver.status),
            if !matches!(
                Status::from_legacy(&driver.status),
                Status::Classified | Status::Interrupted
            ) {
                "—".into()
            } else {
                format!("{} p", driver.total_penalties)
            },
        ],
    )
}

fn results(
    ws: &Workspace,
    archive: &Archive,
    selected_key: i32,
    view: &mut ViewData,
    notes: &mut Vec<String>,
) -> Result<(), CoreError> {
    view.headers = headers(&[
        "Sija",
        "Nro",
        "Luokka",
        "Kuljettaja",
        "Kartanlukija",
        "Tila",
        "Tulos",
    ]);
    view.weights = vec![0.06, 0.06, 0.08, 0.23, 0.23, 0.14, 0.20];
    let ranking = application::rank_stored(archive, TiePolicy::KeepSharedAndFlag)?;
    for group in ranking
        .groups
        .iter()
        .filter(|group| group.kind == GroupKind::Ordinary)
    {
        for entry in &group.entries {
            let Some(index) = entry.source_index.checked_sub(1) else {
                continue;
            };
            if let Some(driver) = archive.competitors.get(index) {
                let position = if entry.status == Status::Classified {
                    entry.primary_position.to_string()
                } else {
                    "—".into()
                };
                view.rows.push(result_row(index, driver, position));
            }
        }
    }
    for source_index in &ranking.omitted_source_indices {
        if let Some(index) = source_index.checked_sub(1)
            && let Some(driver) = archive.competitors.get(index)
        {
            view.rows.push(result_row(index, driver, "—".into()));
        }
    }
    notes.extend(ranking.diagnostics.iter().map(ToString::to_string));
    if !ranking.omitted_source_indices.is_empty() {
        notes.push("Keskeneräiset ja tuntemattomat tarkastustilat näkyvät luettelon lopussa ilman sijoitusta.".into());
    }
    notes.push(
        "Tasapisteissä jaettu sija säilyy; rivien esitysjärjestys ei ratkaise tasapisteitä.".into(),
    );

    let Some(driver) = usize::try_from(selected_key)
        .ok()
        .and_then(|index| archive.competitors.get(index))
    else {
        return Ok(());
    };
    view.detail_title = format!(
        "Nro {} · {} / {} · {}",
        driver.number,
        driver.driver,
        driver.navigator,
        if ws.is_editable() {
            "lasketut asematulokset"
        } else {
            "tallennetut asematulokset"
        }
    );
    view.detail_headers = headers(&[
        "Järj.",
        "Asema",
        "Emit od./luettu",
        "Tavoite, s",
        "Aika, s",
        "Aikavirhe, p",
        "Muut, p",
        "Yhteensä, p",
        "Huomautus",
    ]);
    view.detail_weights = vec![0.04, 0.21, 0.07, 0.09, 0.09, 0.10, 0.07, 0.10, 0.23];
    view.detail_rows = driver
        .results
        .iter()
        .enumerate()
        .map(|(index, station)| {
            row(
                index,
                vec![
                    station.output_index.to_string(),
                    format!("{} {}", station.station_type, station.name),
                    format!("{} / {}", station.expected_code, station.received_code),
                    station.target_seconds.to_string(),
                    station.actual_seconds.to_string(),
                    station.signed_time_penalty.to_string(),
                    station.other_penalty.to_string(),
                    station.total_penalty.to_string(),
                    station.note.clone(),
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
                        .map_or_else(|| "—".into(), |p| p.len().to_string()),
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
    fn filtering_and_result_order_keep_original_model_keys() {
        let fixture = Fixture::new(|project| {
            let mut second = project.kilpailijat[0].clone();
            second.numero = 2;
            second.kuljettaja = "Toinen koe".into();
            second.emit = 654321;
            project.kilpailijat.push(second);
        });
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
        assert_eq!(text(&results.rows[1], 6), "—");
        assert!(!results.detail_rows.is_empty());
        assert!(!results.notes.contains("AS2015A"));
        assert!(
            results
                .notes
                .contains("Maaliaika on johdettu kortin viimeisestä leimasta.")
        );
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
