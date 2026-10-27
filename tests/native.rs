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
    assert_eq!(total, 2415);
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

/// Aito toimitsijapolku: käsimaalille annetaan kello ja tallennetut asemat tutkitaan.
fn complete_manual_finish(project: &mut native::Project) {
    let mut result = application::capture_project_result(project, 7).unwrap();
    let finish = result
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    result.set_clock(finish, [12, 10, 0]).unwrap();
    let result = application::recalculate_stored_result(project, 7, &result).unwrap();
    assert_eq!(result.tila, "TRK");
    assert!(result.can_approve());
    project.kilpailijat[0].tallennettu_tulos = Some(result);
}

/// Sijoitus- ja sarjapistetesteissä käytetään nimenomaisesti hyväksyttyä tulosta.
fn approve_completed_result(project: &mut native::Project) {
    let result = project.kilpailijat[0]
        .tallennettu_tulos
        .as_mut()
        .expect("Tarkastettu asematulos puuttuu");
    assert!(result.can_approve());
    result.hyvaksytty = true;
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
        lahde: None,
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
    complete_manual_finish(&mut project);
    let baseline = application::calculated_project(&project)
        .unwrap()
        .competitors
        .remove(0);
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
    assert_eq!(competitor.status, "TRK");
    assert_eq!(competitor.header_fields[19], baseline.header_fields[19]);
    assert_eq!(competitor.header_fields[20], baseline.header_fields[20]);
    assert_eq!(
        competitor.total_penalties,
        baseline.total_penalties + 1200 + 75 + 100
    );
    assert_eq!(
        competitor.header_fields[26],
        competitor.total_penalties.to_string()
    );
    assert_eq!(
        competitor
            .results
            .iter()
            .map(|row| row.total_penalty)
            .sum::<i64>(),
        baseline.total_penalties
    );
    assert_eq!(
        native::Project::from_toml(&project.to_toml().unwrap()).unwrap(),
        project
    );
}

#[test]
fn native_keeps_stored_rt_sum_without_repricing_and_rejects_invalid_totals() {
    let mut project = project();
    complete_manual_finish(&mut project);
    let baseline = application::calculated_project(&project)
        .unwrap()
        .competitors[0]
        .total_penalties;
    project.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl = 2;
    project.kilpailijat[0].lisavirheet.puuttuvat_rt_pisteet = 700;
    assert_eq!(
        application::calculated_project(&project)
            .unwrap()
            .competitors[0]
            .total_penalties,
        baseline + 700
    );
    project.virhepisteet.puuttuva_rt = Some(999);
    let updated = application::calculated_project(&project).unwrap();
    assert_eq!(updated.competitors[0].total_penalties, baseline + 700);
    assert_eq!(updated.competitors[0].header_fields[22], "700");
    project.kilpailijat[0].lisavirheet.muut_virhepisteet = -1;
    assert!(project.validate().is_err());
    project.kilpailijat[0].lisavirheet.muut_virhepisteet = i64::MAX;
    assert!(application::calculated_project(&project).is_err());
}

#[test]
fn calculated_native_results_write_consistent_clocks_status_and_header_fields() {
    let mut project = project();
    let history = project.leimaluvut.clone();
    let pending = application::calculated_project(&project).unwrap();
    let driver = &pending.competitors[0];
    assert_eq!(driver.status, "ILM");
    assert_eq!(driver.header_fields[28], "ILM");
    assert!(driver.header_fields[29].contains("manuaalinen aika puuttuu"));
    assert_eq!(
        &driver.header_fields[13..=18],
        ["0", "0", "0", "0", "0", "0"]
    );
    assert_eq!(driver.results.last().unwrap().clock, [0; 3]);
    assert_eq!(driver.results.last().unwrap().signed_time_penalty, 0);
    let initial_at = driver
        .results
        .iter()
        .find(|r| r.expected_code == 31)
        .unwrap()
        .clone();
    assert_eq!(initial_at.clock, [12, 4, 35]);
    assert_eq!(initial_at.actual_seconds, 275);
    complete_manual_finish(&mut project);
    let archive = application::calculated_project(&project).unwrap();
    let driver = &archive.competitors[0];
    assert_eq!(driver.status, "TRK");
    assert_eq!(driver.header_fields[28], "TRK");
    assert_eq!(driver.header_fields[29], "");
    assert_eq!(&driver.header_fields[13..=15], ["0", "10", "0"]);
    assert_eq!(&driver.header_fields[16..=18], ["12", "10", "0"]);
    assert_eq!(
        driver.header_fields[27],
        driver.declared_row_count.to_string()
    );
    assert_eq!(driver.results.last().unwrap().clock, [12, 10, 0]);
    assert_eq!(driver.results.last().unwrap().actual_seconds, 600);
    assert_eq!(driver.results.last().unwrap().signed_time_penalty, 0);
    let checked_at = driver
        .results
        .iter()
        .find(|r| r.expected_code == 31)
        .unwrap();
    assert_eq!(checked_at.clock, initial_at.clock);
    assert_eq!(checked_at.actual_seconds, initial_at.actual_seconds);
    assert_eq!(project.leimaluvut, history);
}

#[test]
fn native_class_np_sp_options_are_persisted_and_mapped_to_ranking_input() {
    let mut project = project();
    complete_manual_finish(&mut project);
    project.luokat[0].nousupisteet = 1;
    project.luokat[0].sarjapistetapa = 1;
    let archive = project.to_archive().unwrap();
    assert_eq!(archive.basic.classes[0].fields[8], "1");
    assert_eq!(archive.basic.classes[0].fields[9], "1");
    approve_completed_result(&mut project);
    let results = application::calculated_project(&project).unwrap();
    let ranking =
        application::rank_stored(&results, as2026::ranking::TiePolicy::KeepSharedAndFlag).unwrap();
    let ordinary = ranking
        .groups
        .iter()
        .find(|group| group.kind == as2026::ranking::GroupKind::Ordinary)
        .unwrap();
    assert_eq!(ordinary.entries[0].series_points, 100);
    assert_eq!(
        native::Project::from_toml(&project.to_toml().unwrap()).unwrap(),
        project
    );
    project.luokat[0].sarjapistetapa = 4;
    assert!(project.validate().is_err());
    project.luokat[0].sarjapistetapa = 0;
    project.luokat[0].nousupisteet = 2;
    assert!(project.validate().is_err());
}

#[test]
fn native_custom_series_schedule_is_persisted_and_used_for_sp_method_four() {
    let mut project = project();
    complete_manual_finish(&mut project);
    project.sarjapisteet = vec![42, 30, 17];
    project.luokat[0].sarjapistetapa = 4;
    let text = project.to_toml().unwrap();
    assert_eq!(native::Project::from_toml(&text).unwrap(), project);
    let archive = project.to_archive().unwrap();
    let table = &archive.documents["sarjap.txt"].records[0].fields;
    assert_eq!(&table[..3], ["42", "30", "17"]);
    approve_completed_result(&mut project);
    let calculated = application::calculated_project(&project).unwrap();
    let ranked =
        application::rank_stored(&calculated, as2026::ranking::TiePolicy::KeepSharedAndFlag)
            .unwrap();
    assert_eq!(ranked.groups[0].entries[0].series_points, 42);
}

#[test]
fn native_rejects_missing_invalid_or_oversized_custom_series_schedule() {
    let mut project = project();
    project.luokat[0].sarjapistetapa = 4;
    assert!(project.validate().is_err());
    project.sarjapisteet = vec![42];
    assert!(project.validate().is_ok());
    project.sarjapisteet = vec![-1];
    assert!(project.validate().is_err());
    project.sarjapisteet = vec![1; 1001];
    assert!(project.validate().is_err());
}

#[test]
fn combined_classes_are_persisted_and_ranked_from_native_toml() {
    let mut project = project();
    complete_manual_finish(&mut project);
    project.luokat[0].sarjapistetapa = 1;
    project.yhdistetyt_luokat.push(native::CombinedClass {
        nimi: "Yhteinen".into(),
        luokat: vec!["A".into()],
        nousupisteet: 1,
        sarjapistetapa: 5,
    });
    let toml = project.to_toml().unwrap();
    assert_eq!(native::Project::from_toml(&toml).unwrap(), project);
    approve_completed_result(&mut project);
    let archive = application::calculated_project(&project).unwrap();
    assert_eq!(
        archive.basic.combined_classes[0],
        ["1", "Yhteinen", "1", "5", "1"]
    );
    let ranking =
        application::rank_stored(&archive, as2026::ranking::TiePolicy::KeepSharedAndFlag).unwrap();
    let group = ranking
        .groups
        .iter()
        .find(|g| g.kind == as2026::ranking::GroupKind::Combined)
        .unwrap();
    assert_eq!(group.entries.len(), 1);
    assert_eq!(group.entries[0].series_points, 100);
}

#[test]
fn combined_class_member_codes_are_exact_and_invalid_configs_are_rejected() {
    let mut project = project();
    let group = native::CombinedClass {
        nimi: "Kooste".into(),
        luokat: vec!["A".into()],
        nousupisteet: 1,
        sarjapistetapa: 0,
    };
    project.yhdistetyt_luokat.push(group);
    assert!(project.validate().is_ok());
    project.yhdistetyt_luokat[0].luokat = vec!["AX".into()];
    assert!(project.validate().is_err());
    project.yhdistetyt_luokat[0].luokat = vec!["A".into(), "A".into()];
    assert!(project.validate().is_err());
    project.yhdistetyt_luokat[0].luokat = vec!["A".into()];
    project.yhdistetyt_luokat[0].sarjapistetapa = 4;
    assert!(project.validate().is_err());
    project.sarjapisteet = vec![100, 90];
    assert!(project.validate().is_ok());
    let first = project.yhdistetyt_luokat[0].clone();
    project.yhdistetyt_luokat = vec![first; 5];
    assert!(project.validate().is_err());
}
