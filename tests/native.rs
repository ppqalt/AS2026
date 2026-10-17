use as2026::{application, native};
use std::path::Path;

#[test]
fn synthetic_toml_uses_the_existing_scoring_core() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/native-demo/kilpailu.toml");
    let archive = native::load(&p).unwrap();
    assert_eq!(archive.basic.identity[1], "Esimerkkikilpailu");
    assert_eq!(archive.routes[0].index, 1);
    let report = application::calculate(&archive).unwrap();
    let total = report.competitors[0]
        .calculation
        .as_ref()
        .unwrap()
        .totals
        .total_points;
    assert_eq!(total, 645);
}

#[test]
fn unknown_native_format_version_is_rejected() {
    let original = include_str!("../fixtures/native-demo/kilpailu.toml");
    let replacement = original.replacen("versio = 1", "versio = 99", 1);
    let project: native::Project = toml::from_str(&replacement).unwrap();
    assert!(project.into_archive().is_err());
}

#[test]
fn typo_in_field_is_not_silently_ignored() {
    let original = include_str!("../fixtures/native-demo/kilpailu.toml");
    let replacement = original.replacen("ajanottotapa = 1", "ajanottotapp = 1", 1);
    assert!(toml::from_str::<native::Project>(&replacement).is_err());
}

fn project() -> native::Project {
    native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}

fn read(card: u32, marker_time: u32, finish_time: u32) -> native::Readout {
    native::Readout {
        emit: card,
        lukukello: "15:10:00".into(),
        leimat: vec![
            native::ReadPunch {
                koodi: 0,
                aika_sekuntia: 0,
            },
            native::ReadPunch {
                koodi: 47,
                aika_sekuntia: finish_time,
            },
            native::ReadPunch {
                koodi: 250,
                aika_sekuntia: marker_time,
            },
        ],
        luettu_unix_ms: None,
    }
}

#[test]
fn version_one_round_trip_retains_every_known_field() {
    let mut project = project();
    project.kilpailijat[0].auto = "Harjoitusauto".into();
    project.virhepisteet.puuttuva_rt = Some(123);
    let encoded = project.to_toml().unwrap();
    assert_eq!(native::Project::from_toml(&encoded).unwrap(), project);
    assert_eq!(
        project.to_archive().unwrap().competitors[0].header_fields[9],
        "Harjoitusauto"
    );
    assert!(!encoded.contains("luettu_unix_ms"));
}

#[test]
fn reread_ignores_reader_marker_time_but_preserves_physical_events() {
    let mut project = project();
    assert!(
        project
            .record_readout(read(538747, 5122, 145))
            .unwrap()
            .content_changed
    );
    let reread = project.record_readout(read(538747, 5239, 145)).unwrap();
    assert!(!reread.content_changed);
    assert_eq!(project.leimaluvut.len(), 3);
    assert_eq!(
        project.active_readout(538747).unwrap().leimat[2].aika_sekuntia,
        5239
    );
    let archive = project.to_archive().unwrap();
    let reads = as2026::legacy::parse_saved_emit(&archive.documents["emit.txt"]).unwrap();
    assert_eq!(reads.len(), 2); // Kortit 123456 ja 538747, yksi aktiivinen lukema kummallekin.
}

#[test]
fn changed_punches_and_interleaved_cards_keep_independent_current_content() {
    let mut project = project();
    project.record_readout(read(538747, 5122, 145)).unwrap();
    project.record_readout(read(42, 600, 200)).unwrap();
    assert!(
        !project
            .record_readout(read(538747, 5239, 145))
            .unwrap()
            .content_changed
    );
    assert!(
        project
            .record_readout(read(538747, 5240, 146))
            .unwrap()
            .content_changed
    );
    assert_eq!(
        project.active_readout(42).unwrap().leimat[1].aika_sekuntia,
        200
    );
    assert_eq!(
        project.active_readout(538747).unwrap().leimat[1].aika_sekuntia,
        146
    );
}

#[test]
fn incomplete_read_does_not_replace_valid_content_and_old_bad_history_is_skipped() {
    let mut project = project();
    let before = project.clone();
    let mut bad = read(123456, 5122, 145);
    bad.leimat.pop();
    assert!(project.record_readout(bad.clone()).is_err());
    assert_eq!(project, before);
    // Vanha käsin muokattu versio 1 voi jo sisältää huonon lukeman.
    project.leimaluvut.push(bad);
    let archive = project.to_archive().unwrap();
    assert_eq!(archive.warnings.len(), 1);
    let reads = as2026::legacy::parse_saved_emit(&archive.documents["emit.txt"]).unwrap();
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].records[1].code, 31);
}

#[test]
fn unknown_card_can_be_registered_after_reading() {
    let mut project = project();
    let mut read = project.leimaluvut[0].clone();
    read.emit = 538747;
    let result = project.record_readout(read.clone()).unwrap();
    assert_eq!(result.competitor_number, None);
    let mut driver = project.kilpailijat[0].clone();
    driver.emit = 538747;
    driver.numero = 8;
    project.kilpailijat.push(driver);
    project.validate().unwrap();
    read.leimat.last_mut().unwrap().aika_sekuntia += 1;
    let result = project.record_readout(read).unwrap();
    assert_eq!(result.competitor_number, Some(8));
    assert!(!result.content_changed);
    let report = application::calculate(&project.to_archive().unwrap()).unwrap();
    let calculation = &report
        .competitors
        .iter()
        .find(|driver| driver.number == 8)
        .unwrap()
        .calculation;
    assert!(calculation.is_ok(), "{calculation:?}");
}

#[test]
fn renamed_competitor_and_changed_class_track_use_the_same_current_model() {
    let mut project = project();
    project.kilpailijat[0].kuljettaja = "Korjattu nimi".into();
    let mut track = project.radat[0].clone();
    track.numero = 2;
    track.nimi = "Toinen rata".into();
    project.radat.push(track);
    project.luokat[0].rata = 2;
    let archive = project.to_archive().unwrap();
    assert_eq!(archive.competitors[0].driver, "Korjattu nimi");
    assert_eq!(archive.basic.classes[0].route, "2");
    assert!(
        as2026::application::calculate(&archive)
            .unwrap()
            .competitors[0]
            .calculation
            .is_ok()
    );
}

#[test]
fn station_change_immediately_changes_fresh_calculation() {
    let mut project = project();
    let before = application::calculate(&project.to_archive().unwrap())
        .unwrap()
        .competitors[0]
        .calculation
        .as_ref()
        .unwrap()
        .totals
        .total_points;
    project.radat[0].asemat[1].tavoiteaika_sekuntia += 100;
    let after = application::calculate(&project.to_archive().unwrap())
        .unwrap()
        .competitors[0]
        .calculation
        .as_ref()
        .unwrap()
        .totals
        .total_points;
    assert_ne!(before, after);
}

#[test]
fn new_project_uses_shared_defaults_and_serializes_special_names() {
    let project = native::Project::new("Syysajo: ä \"2026\"").unwrap();
    assert_eq!(project.radat[0].asemat[1].tavoiteaika_sekuntia, 300);
    assert_eq!(
        native::Project::from_toml(&project.to_toml().unwrap()).unwrap(),
        project
    );
    assert!(native::Project::new("  ").is_err());
}

#[test]
fn independent_rt_and_other_penalties_contribute_to_final_native_result() {
    let mut project = project();
    project.kilpailijat[0].lisavirheet = native::AdditionalPenalties {
        puuttuvat_rt_kpl: 2,
        puuttuvat_rt_pisteet: 1200,
        ylimaaraiset_rt_kpl: 3,
        ylimaaraiset_rt_pisteet: 75,
        muut_virhepisteet: 100,
    };
    let archive = project.to_archive().unwrap();
    assert_eq!(
        &archive.competitors[0].header_fields[21..=25],
        ["2", "1200", "3", "75", "100"]
    );

    let results = application::calculated_project(&project).unwrap();
    let competitor = &results.competitors[0];
    assert_eq!(competitor.header_fields[19], "45");
    assert_eq!(competitor.header_fields[20], "600");
    assert_eq!(competitor.header_fields[26], "2020");
    assert_eq!(competitor.total_penalties, 2020);
    assert_eq!(
        competitor
            .results
            .iter()
            .map(|row| row.total_penalty)
            .sum::<i64>(),
        645
    );
    assert_eq!(
        native::Project::from_toml(&project.to_toml().unwrap()).unwrap(),
        project
    );
}

#[test]
fn native_keeps_stored_rt_sum_without_repricing_and_rejects_invalid_totals() {
    let mut project = project();
    project.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl = 2;
    project.kilpailijat[0].lisavirheet.puuttuvat_rt_pisteet = 700;
    assert_eq!(
        application::calculated_project(&project)
            .unwrap()
            .competitors[0]
            .total_penalties,
        1345
    );
    project.kilpailijat[0].lisavirheet.muut_virhepisteet = -1;
    assert!(project.validate().is_err());
    project.kilpailijat[0].lisavirheet.muut_virhepisteet = i64::MAX;
    assert!(application::calculated_project(&project).is_err());
}
