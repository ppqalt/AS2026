use as2026::{
    application,
    changes::Changes,
    native::{ManualStatus, Project},
    ranking::{Status, TiePolicy},
};

fn project() -> Project {
    Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}

fn old_driver_change() -> &'static str {
    "[[kilpailijat]]\nnumero = 7\nluokka = \"A\"\nkuljettaja = \"Korjattu nimi\"\nkartanlukija = \"Esimerkkikartanlukija\"\nemit = 123456\nlahtoaika = \"12:00:00\"\nemit_korjaus_sekuntia = 120\n"
}

#[test]
fn older_competition_files_keep_automatic_status_and_omit_new_empty_fields() {
    let project = project();
    let driver = &project.kilpailijat[0];
    assert!(driver.kuljettajan_lisenssi.is_empty());
    assert!(driver.kartanlukijan_lisenssi.is_empty());
    assert!(driver.tila.is_none());
    assert!(driver.tallennettu_tulos.is_none());
    let encoded = project.to_toml().unwrap();
    assert!(!encoded.contains("lisenssi"));
    assert!(!encoded.contains("kilpailijat.tila"));
    assert!(!encoded.contains("tallennettu_tulos"));
    assert_eq!(Project::from_toml(&encoded).unwrap(), project);
}

#[test]
fn explicit_status_and_both_licences_round_trip_into_the_existing_legacy_fields() {
    let mut project = project();
    let driver = &mut project.kilpailijat[0];
    driver.kuljettajan_lisenssi = "12345X".into();
    driver.kartanlukijan_lisenssi = "X67890".into();
    driver.tila = Some(ManualStatus::new("HYL", "Kilpailun johdon päätös").unwrap());
    let reopened = Project::from_toml(&project.to_toml().unwrap()).unwrap();
    assert_eq!(reopened, project);
    let archive = reopened.to_archive().unwrap();
    let competitor = &archive.competitors[0];
    assert_eq!(competitor.header_fields[5], "12345X");
    assert_eq!(competitor.header_fields[6], "X67890");
    assert_eq!(competitor.header_fields[28], "HYL");
    assert_eq!(competitor.header_fields[29], "Kilpailun johdon päätös");
    assert_eq!(competitor.status, "HYL");
}

#[test]
fn only_confirmed_legacy_status_codes_are_accepted() {
    let mut project = project();
    for code in ["TRK", "KESK", "PER", "HYL", "SULJ", "ILM", "EILÄ"] {
        project.kilpailijat[0].tila = Some(ManualStatus::new(code, "").unwrap());
        project.validate().unwrap();
    }
    for code in ["", "trk", "TRK ", "DNS", "HYLÄTTY"] {
        assert!(ManualStatus::new(code, "").is_err());
        project.kilpailijat[0].tila = Some(ManualStatus {
            tunnus: code.into(),
            syy: String::new(),
        });
        assert!(project.validate().is_err());
    }
}

#[test]
fn legacy_changes_do_not_discard_new_fields_or_additional_penalties() {
    let mut project = project();
    let stored = application::capture_project_result(&project, 7).unwrap();
    let driver = &mut project.kilpailijat[0];
    driver.kuljettajan_lisenssi = "12X".into();
    driver.kartanlukijan_lisenssi = "345".into();
    driver.tila = Some(ManualStatus::new("PER", "Peruttu").unwrap());
    driver.lisavirheet.muut_virhepisteet = 12;
    driver.tallennettu_tulos = Some(stored);
    let before = driver.clone();
    toml::from_str::<Changes>(old_driver_change())
        .unwrap()
        .apply(&mut project)
        .unwrap();
    let after = &project.kilpailijat[0];
    assert_eq!(after.kuljettaja, "Korjattu nimi");
    assert_eq!(after.kuljettajan_lisenssi, before.kuljettajan_lisenssi);
    assert_eq!(after.kartanlukijan_lisenssi, before.kartanlukijan_lisenssi);
    assert_eq!(after.tila, before.tila);
    assert_eq!(after.lisavirheet, before.lisavirheet);
    assert_eq!(after.tallennettu_tulos, before.tallennettu_tulos);
    assert_eq!(project.leimaluvut.len(), 1);
}

#[test]
fn explicit_change_can_clear_licences_and_return_status_to_automatic() {
    let mut project = project();
    project.kilpailijat[0].kuljettajan_lisenssi = "X".into();
    project.kilpailijat[0].kartanlukijan_lisenssi = "X".into();
    project.kilpailijat[0].tila = Some(ManualStatus::new("HYL", "Päätös").unwrap());
    let change = format!(
        "{}kuljettajan_lisenssi = \"\"\nkartanlukijan_lisenssi = \"\"\ntila = {{ tunnus = \"\" }}\n",
        old_driver_change()
    );
    toml::from_str::<Changes>(&change)
        .unwrap()
        .apply(&mut project)
        .unwrap();
    let driver = &project.kilpailijat[0];
    assert!(driver.kuljettajan_lisenssi.is_empty());
    assert!(driver.kartanlukijan_lisenssi.is_empty());
    assert!(driver.tila.is_none());
}

#[test]
fn bad_status_change_is_atomic() {
    let mut project = project();
    let before = project.clone();
    let change = format!("{}tila = {{ tunnus = \"DNS\" }}\n", old_driver_change());
    let error = toml::from_str::<Changes>(&change)
        .unwrap()
        .apply(&mut project)
        .unwrap_err();
    assert!(error.to_string().contains("Kilpailijan tilan"));
    assert_eq!(project, before);
}

#[test]
fn either_licence_uses_the_recovered_left_to_right_series_eligibility() {
    let mut project = project();
    project.luokat[0].nousupisteet = 1;
    project.luokat[0].sarjapistetapa = 1;
    project.kilpailijat[0].tila = Some(ManualStatus::new("TRK", "").unwrap());
    for (driver_licence, navigator_licence, excluded) in [
        ("123X", "123", true),
        ("X123", "123", false),
        ("123", "123x", true),
        ("123", "X456", false),
        ("", "", false),
    ] {
        project.kilpailijat[0].kuljettajan_lisenssi = driver_licence.into();
        project.kilpailijat[0].kartanlukijan_lisenssi = navigator_licence.into();
        let ranked =
            application::rank_stored(&project.to_archive().unwrap(), TiePolicy::KeepSharedAndFlag)
                .unwrap();
        let entry = &ranked.groups[0].entries[0];
        assert_eq!(entry.series_excluded, excluded);
        assert_eq!(entry.series_points, if excluded { 0 } else { 100 });
    }
}

#[test]
fn manual_status_is_applied_after_score_calculation_and_reaches_export() {
    let mut project = project();
    project.kilpailijat[0].tila = Some(ManualStatus::new("HYL", "Johdon päätös").unwrap());
    let archive = application::calculated_project(&project).unwrap();
    let competitor = &archive.competitors[0];
    assert_eq!(competitor.status, "HYL");
    assert_eq!(competitor.header_fields[29], "Johdon päätös");
    assert_eq!(competitor.total_penalties, 2415);
    assert!(!competitor.results.is_empty());
    let ranked = application::rank_stored(&archive, TiePolicy::KeepSharedAndFlag).unwrap();
    assert_eq!(ranked.groups[0].entries[0].status, Status::Disqualified);
    assert!(as2026::report::ranking_csv(&archive, &ranked).contains("\"HYL\""));
}
