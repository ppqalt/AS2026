use as2026::interpretation::{EntryKind, preview_rows};

#[test]
fn missing_rt_summary_separates_routes_sections_occurrences_and_pairs() {
    use as2026::{
        interpretation::missing_rt_summary,
        native::{MissingRtSection, Project, TulkkiPenaltySource},
    };
    let mut p = Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    p.kilpailijat[0].emit = 0;
    p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl = 3;
    p.kilpailijat[0].lisavirheet.tulkki_siirto = Some(TulkkiPenaltySource {
        tapahtumatunniste: None,
        tulkatut_jaksot: Vec::new(),
        jaksoja: 0,
        tunniste: "a".repeat(64),
        muut_virhepisteet: 0,
        puuttuvat_rt: vec![
            MissingRtSection {
                jakso: 1,
                kilvet: vec!["15".into(), "15".into()],
            },
            MissingRtSection {
                jakso: 2,
                kilvet: vec!["08".into()],
            },
        ],
    });
    let mut second = p.kilpailijat[0].clone();
    second.numero = 8;
    second.lisavirheet.puuttuvat_rt_kpl = 1;
    second
        .lisavirheet
        .tulkki_siirto
        .as_mut()
        .unwrap()
        .puuttuvat_rt = vec![MissingRtSection {
        jakso: 1,
        kilvet: vec!["15".into()],
    }];
    p.kilpailijat.push(second.clone());
    let mut route = p.radat[0].clone();
    route.numero = 2;
    p.radat.push(route);
    let mut class = p.luokat[0].clone();
    class.tunnus = "B".into();
    class.rata = 2;
    p.luokat.push(class);
    second.numero = 9;
    second.luokka = "B".into();
    p.kilpailijat.push(second);
    let before = p.clone();
    let s = missing_rt_summary(&p);
    assert_eq!(p, before);
    assert_eq!(s.rows.len(), 3);
    assert_eq!(s.received_pairs, 3);
    assert_eq!(
        (
            s.rows[0].route,
            s.rows[0].section,
            s.rows[0].station.as_str(),
            s.rows[0].occurrences
        ),
        (1, Some(1), "15", 3)
    );
    assert_eq!(s.rows[0].pairs, vec![7, 8]);
    assert_eq!(s.rows[1].section, Some(2));
    assert_eq!(s.rows[2].route, 2);
    let reopened = Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(missing_rt_summary(&reopened), s);
}

#[test]
fn missing_rt_summary_does_not_guess_labels_for_manual_counts() {
    let mut p =
        as2026::native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
            .unwrap();
    p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl = u32::MAX;
    let mut second = p.kilpailijat[0].clone();
    second.numero = 8;
    p.kilpailijat.push(second);
    let s = as2026::interpretation::missing_rt_summary(&p);
    assert_eq!(s.received_pairs, 0);
    assert_eq!(s.unidentified_pairs, 2);
    assert_eq!(s.rows[0].section, None);
    assert_eq!(s.rows[0].occurrences, 2 * u64::from(u32::MAX));
    assert_eq!(s.rows[0].pairs, vec![7, 8]);
}

#[test]
fn time_input_is_configured_per_occurrence_and_survives_reimport() {
    use as2026::interpretation::{import_route, set_time_input};
    let mut project = as2026::native::Project::new("Ajan syöttötavat").unwrap();
    let preview = preview_rows("Syotto_B", &rows(&["08", "AT", "JAT", "ULA", "M"]));
    import_route(&mut project, 1, &preview).unwrap();
    let at = project.oikeat_rivit[0].merkinnat[1].tunniste.clone();
    let jat = project.oikeat_rivit[0].merkinnat[2].tunniste.clone();
    set_time_input(&mut project, 1, &at, Some(true)).unwrap();
    set_time_input(&mut project, 1, &jat, Some(false)).unwrap();
    assert!(project.oikeat_rivit[0].merkinnat[1].requires_manual_time(true));
    assert!(!project.oikeat_rivit[0].merkinnat[2].requires_manual_time(false));
    let saved = project.to_toml().unwrap();
    let restored = as2026::native::Project::from_toml(&saved).unwrap();
    assert_eq!(project, restored);
    import_route(&mut project, 1, &preview).unwrap();
    assert_eq!(project.to_toml().unwrap(), saved);
    let sign = project.oikeat_rivit[0].merkinnat[0].tunniste.clone();
    assert!(set_time_input(&mut project, 1, &sign, Some(true)).is_err());
    assert_eq!(project.to_toml().unwrap(), saved);
}

#[test]
#[ignore = "Vaatii tilaajan paikallisen Excel-pohjan ympäristömuuttujassa"]
fn actual_macro_workbook_is_read_only_and_wrong_import_has_guidance() {
    let path =
        std::env::var_os("AS2026_OIKEA_RIVI_MALLI").expect("AS2026_OIKEA_RIVI_MALLI puuttuu");
    let path = std::path::Path::new(&path);
    let before = std::fs::read(path).unwrap();
    let previews = as2026::interpretation::preview_workbook(path).unwrap();
    assert_eq!(previews.len(), 3);
    let b = previews.iter().find(|p| p.sheet == "Syotto_B").unwrap();
    assert!(b.can_publish());
    assert_eq!(b.entries.len(), 42);
    let c = previews.iter().find(|p| p.sheet == "Syotto_C").unwrap();
    assert_eq!(c.issues.len(), 5);
    assert!(!c.can_publish());
    let mut project = as2026::native::Project::new("Excel-koe").unwrap();
    as2026::interpretation::import_route(&mut project, 1, b).unwrap();
    let restored = as2026::native::Project::from_toml(&project.to_toml().unwrap()).unwrap();
    assert_eq!(restored, project);
    let error = as2026::excel_import::read(path).unwrap_err().to_string();
    assert!(error.contains("Tuo oikea rivi"));
    assert_eq!(before, std::fs::read(path).unwrap());
}

#[test]
fn stored_rows_roundtrip_without_affecting_timing_and_reimport_keeps_ids() {
    let mut project = as2026::native::Project::new("Tulkkauskoe").unwrap();
    let original_stations = project.radat[0].asemat.clone();
    let preview = preview_rows("Syotto_B", &rows(&["25", "25", "JAT", "ULA", "M"]));
    as2026::interpretation::import_route(&mut project, 1, &preview).unwrap();
    assert_ne!(
        project.oikeat_rivit[0].merkinnat[0].tunniste,
        project.oikeat_rivit[0].merkinnat[1].tunniste
    );
    let saved = project.to_toml().unwrap();
    assert_eq!(as2026::native::Project::from_toml(&saved).unwrap(), project);
    as2026::interpretation::import_route(&mut project, 1, &preview).unwrap();
    assert_eq!(project.to_toml().unwrap(), saved);
    assert_eq!(project.radat[0].asemat, original_stations);
    let changed = preview_rows("Syotto_B", &rows(&["08", "M"]));
    assert!(as2026::interpretation::import_route(&mut project, 1, &changed).is_err());
    assert_eq!(project.to_toml().unwrap(), saved);
}

#[test]
fn invalid_or_unmapped_import_does_not_modify_project() {
    let mut project = as2026::native::Project::new("Tulkkauskoe").unwrap();
    let before = project.clone();
    let invalid = preview_rows("Syotto_C", &rows(&["??", "M"]));
    assert!(as2026::interpretation::import_route(&mut project, 1, &invalid).is_err());
    let valid = preview_rows("Syotto_C", &rows(&["08", "M"]));
    assert!(as2026::interpretation::import_route(&mut project, 99, &valid).is_err());
    assert_eq!(before, project);
    let mut malformed = valid.clone();
    malformed.entries[0].section = 99;
    assert!(as2026::interpretation::import_route(&mut project, 1, &malformed).is_err());
    assert_eq!(before, project);
}

fn rows(values: &[&str]) -> Vec<(u32, String)> {
    values
        .iter()
        .enumerate()
        .map(|(i, text)| (i as u32 + 1, (*text).into()))
        .collect()
}

#[test]
fn numbered_time_stations_keep_labels_and_section_boundaries() {
    use as2026::interpretation::EntryKind;
    let preview = preview_rows(
        "Syotto_B",
        &rows(&["08", "at1", "AT2", "JAT3", "ULA3", "15", "JAT4", "M"]),
    );
    assert!(preview.can_publish(), "{:?}", preview.issues);
    assert_eq!(
        preview
            .entries
            .iter()
            .map(|e| e.section)
            .collect::<Vec<_>>(),
        vec![1, 1, 1, 1, 1, 2, 2, 3]
    );
    assert_eq!(preview.entries[1].label, "AT1");
    assert_eq!(preview.entries[2].kind, EntryKind::At);
    assert_eq!(preview.entries[3].kind, EntryKind::Jat);
    assert_eq!(preview.entries[4].kind, EntryKind::Ula);
    let mut project = as2026::native::Project::new("Numeroidut asemat").unwrap();
    as2026::interpretation::import_route(&mut project, 1, &preview).unwrap();
    assert_eq!(
        as2026::native::Project::from_toml(&project.to_toml().unwrap()).unwrap(),
        project
    );
    for invalid in ["ATx", "JAT3x", "AT-1"] {
        assert!(!preview_rows("Syotto_B", &rows(&[invalid, "M"])).can_publish());
    }
}

#[test]
fn repeats_and_jat_ula_remain_distinct_and_ordered() {
    let preview = preview_rows(
        "Syotto_B",
        &rows(&["25", "25", "JAT", "ULA", "00", "JAT", "08", "M"]),
    );
    assert!(preview.can_publish());
    assert_eq!(
        preview
            .entries
            .iter()
            .map(|e| e.section)
            .collect::<Vec<_>>(),
        vec![1, 1, 1, 1, 2, 2, 3, 3]
    );
    assert_eq!(preview.entries[0].label, preview.entries[1].label);
    assert_ne!(preview.entries[0].source_row, preview.entries[1].source_row);
    assert_eq!(preview.entries[4].label, "00");
}

#[test]
fn stamp_is_not_a_start_and_numeric_codes_keep_zero() {
    let preview = preview_rows("Syotto_C", &rows(&["MRT", "L", "JRT", "5", "M"]));
    assert!(preview.can_publish());
    assert_eq!(preview.entries[1].kind, EntryKind::Stamp);
    assert_eq!(preview.entries[3].label, "05");
}

#[test]
fn incomplete_unknown_and_blank_rows_block_publication() {
    let preview = preview_rows("Syotto_C", &rows(&["08", "??", "", "RT", "100", "M"]));
    assert!(!preview.can_publish());
    assert_eq!(preview.issues.len(), 4);
    assert_eq!(preview.issues[0].source_row, 2);
    assert!(!preview_rows("Syotto_C", &[]).can_publish());
}

#[test]
fn invalid_boundary_and_rows_after_finish_are_reported() {
    assert!(!preview_rows("Syotto_C", &rows(&["ULA", "M"])).can_publish());
    assert!(!preview_rows("Syotto_C", &rows(&["M", "08"])).can_publish());
}

#[test]
fn emit_only_changes_manual_entry_for_ula_and_finish() {
    for kind in [EntryKind::Ula, EntryKind::Finish] {
        assert!(kind.requires_manual_time(false));
        assert!(!kind.requires_manual_time(true));
    }
    for kind in [EntryKind::At, EntryKind::Jat] {
        assert!(kind.requires_manual_time(true));
    }
    assert!(!EntryKind::Stamp.requires_manual_time(false));
}
