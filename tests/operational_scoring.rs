//! AS2026:n määritelty toimitsijalaskenta on erillään AS2015A:n staattisista vertailuvektoreista.
use as2026::{application, corrections::StoredResult, native};
fn example() -> native::Project {
    native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}
fn finish_clock(result: &mut StoredResult, clock: [i64; 3]) {
    let row = result
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    result.set_clock(row, clock).unwrap();
}
fn completed() -> (native::Project, StoredResult) {
    let p = example();
    let mut r = application::capture_project_result(&p, 7).unwrap();
    finish_clock(&mut r, [12, 10, 0]);
    let r = application::recalculate_stored_result(&p, 7, &r).unwrap();
    (p, r)
}
#[test]
fn manual_finish_is_pending_then_explicit_clock_works_for_each_method() {
    for method in 0..=2 {
        let mut p = example();
        p.kilpailu.ajanottotapa = method;
        let mut r = application::capture_project_result(&p, 7).unwrap();
        assert_eq!(r.tila, "ILM");
        assert!(!r.can_approve());
        assert_eq!(r.pending_manual_clocks().len(), 1);
        assert_eq!(r.maalikello_sekuntia, 0);
        assert_eq!(r.kesto_sekuntia, 0);
        assert_eq!(
            r.rivit
                .iter()
                .find(|r| r.station_type == "M")
                .unwrap()
                .clock,
            [0; 3]
        );
        finish_clock(
            &mut r,
            if method == 0 {
                [12, 12, 0]
            } else {
                [12, 10, 0]
            },
        );
        assert!(!r.can_approve());
        let checked = application::recalculate_stored_result(&p, 7, &r).unwrap();
        assert_eq!(checked.tila, "TRK");
        assert!(checked.can_approve());
        assert_eq!(checked.kesto_sekuntia, if method == 0 { 720 } else { 600 });
        assert_eq!(
            checked.maalikello_sekuntia,
            if method == 0 { 43920 } else { 43800 }
        );
        assert_eq!(
            checked
                .rivit
                .iter()
                .find(|r| r.station_type == "M")
                .unwrap()
                .actual_seconds,
            if method == 0 { 720 } else { 600 }
        );
        assert!(!checked.hyvaksytty);
    }
}
#[test]
fn several_manual_stations_and_explicit_midnight_keep_provenance_and_persist() {
    let mut p = example();
    p.kilpailu.ajanottotapa = 1;
    p.kilpailijat[0].lahtoaika = "23:55:00".into();
    p.radat[0].asemat[1].emit_koodi = 0;
    p.radat[0].asemat[2].emit_koodi = 0;
    let mut r = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(r.pending_manual_clocks().len(), 3);
    let a = r
        .rivit
        .iter()
        .find(|r| r.sequence == 2)
        .unwrap()
        .output_index;
    let b = r
        .rivit
        .iter()
        .find(|r| r.sequence == 3)
        .unwrap()
        .output_index;
    r.set_clock(a, [0, 0, 0]).unwrap();
    r.set_clock(b, [0, 5, 0]).unwrap();
    finish_clock(&mut r, [0, 5, 0]);
    let r = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert!(r.pending_manual_clocks().is_empty());
    assert!(r.can_approve());
    assert_eq!(r.tila, "TRK");
    assert_eq!(
        r.rivit
            .iter()
            .find(|r| r.sequence == 2)
            .unwrap()
            .actual_seconds,
        300
    );
    assert_eq!(
        r.rivit
            .iter()
            .find(|r| r.sequence == 3)
            .unwrap()
            .actual_seconds,
        600
    );
    assert_eq!(r.kesto_sekuntia, 600);
    p.kilpailijat[0].tallennettu_tulos = Some(r);
    let loaded = native::Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(loaded, p);
}
#[test]
fn manual_finish_and_postfinish_card_punches_do_not_invent_finish_time() {
    let mut p = example();
    let marker = p.leimaluvut[0].leimat.pop().unwrap();
    p.leimaluvut[0].leimat.push(native::ReadPunch {
        koodi: 88,
        aika_sekuntia: 750,
    });
    p.leimaluvut[0].leimat.push(marker);
    let mut r = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(r.tila, "ILM");
    assert_eq!(r.maalikello_sekuntia, 0);
    let extra = r.rivit.iter().find(|r| r.received_code == 88).unwrap();
    assert_eq!(extra.other_penalty, 600);
    finish_clock(&mut r, [12, 10, 0]);
    let checked = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert_eq!(checked.tila, "TRK");
    assert_eq!(checked.kesto_sekuntia, 600);
    assert_eq!(
        checked
            .rivit
            .iter()
            .find(|r| r.received_code == 88)
            .unwrap()
            .elapsed_seconds,
        750
    );
    assert_eq!(checked.asemien_muut_virheet, 1200);
}
#[test]
fn manual_hyl_and_per_survive_reads_but_manual_trk_cannot_classify_pending_clock() {
    for status in ["HYL", "PER", "TRK"] {
        let mut p = example();
        p.kilpailijat[0].tila = Some(native::ManualStatus {
            tunnus: status.into(),
            syy: "Toimitsijan syy".into(),
        });
        let view = application::calculated_project(&p).unwrap();
        assert_eq!(
            view.competitors[0].status,
            if status == "TRK" { "ILM" } else { status }
        );
        if status != "TRK" {
            assert_eq!(view.competitors[0].header_fields[29], "Toimitsijan syy");
        }
    }
}
#[test]
fn missing_jat_confirms_interruption_even_while_finish_clock_is_pending() {
    let mut p = example();
    p.radat[0].asemat[1].tyyppi = "JAT".into();
    p.radat[0].asemat[1].emit_koodi = 77;
    let r = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(r.tila, "KESK");
    assert!(r.syy.contains("JAT puuttuu"));
    assert!(!r.can_approve());
    assert_eq!(r.pending_manual_clocks().len(), 1);
    p.kilpailijat[0].tallennettu_tulos = Some(r);
    let view = application::calculated_project(&p).unwrap();
    assert_eq!(view.competitors[0].status, "KESK");
    let reopened = native::Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(
        application::calculated_project(&reopened)
            .unwrap()
            .competitors[0]
            .status,
        "KESK"
    );
}
#[test]
fn section_and_cumulative_limits_preserve_all_reasons_at_exact_seconds() {
    let mut p = example();
    p.radat[0].asemat[1].tyyppi = "JAT".into();
    p.radat[0].asemat[1].emit_koodi = 77;
    p.radat[0].asemat[2].tyyppi = "JAT".into();
    p.radat[0].asemat[2].jakso_minuutteina = Some(0);
    p.radat[0].asemat[3].jakso_minuutteina = Some(0);
    p.leimaluvut[0].leimat[3].aika_sekuntia = 3721; // 3601 s after the anchor: default60min+1sec.
    p.leimaluvut[0].leimat[4].aika_sekuntia = 3800;
    let mut r = application::capture_project_result(&p, 7).unwrap();
    finish_clock(&mut r, [13, 0, 1]);
    let checked = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert_eq!(checked.tila, "KESK");
    assert!(checked.syy.contains("JAT puuttuu"));
    assert!(checked.syy.contains("JAT-aseman"));
    assert!(checked.syy.contains("kokonaismyöhästymisraja"));
    let repeated = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert_eq!(repeated.syy, checked.syy);
    p.virhepisteet.jakson_raja_minuutteina = Some(61);
    p.virhepisteet.kokonaismyohastymisraja_minuutteina = Some(61);
    p.luokat[0].myohastymisraja_minuutteina = 61;
    let relaxed = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert!(!relaxed.syy.contains("jakson myöhästymisraja"));
    assert!(!relaxed.syy.contains("kokonaismyöhästymisraja"));
}
#[test]
fn accepted_row_penalty_edit_is_not_erased_by_approval_or_reread() {
    let (mut p, mut r) = completed();
    r.hyvaksytty = true;
    r.set_row_penalties(2, -77, 12).unwrap();
    assert!(!r.hyvaksytty);
    assert!(r.can_approve());
    r.hyvaksytty = true;
    p.kilpailijat[0].tallennettu_tulos = Some(r.clone());
    let mut new = p.leimaluvut[0].clone();
    new.leimat[1].aika_sekuntia += 1;
    p.record_readout(new).unwrap();
    let view = application::calculated_project(&p).unwrap();
    assert_eq!(view.competitors[0].results[1].signed_time_penalty, -77);
    assert!(
        p.kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    let mut edited = r;
    finish_clock(&mut edited, [12, 11, 0]);
    assert!(!edited.hyvaksytty);
    assert_eq!(edited.tila, "ILM");
    assert!(!edited.can_approve());
}
#[test]
fn invalid_manual_clock_order_blocks_approval_without_dropping_raw_data() {
    let (p, mut r) = completed();
    let before = r.clone();
    let finish = r
        .rivit
        .iter()
        .find(|row| row.station_type == "M")
        .unwrap()
        .output_index;
    assert!(r.set_clock(finish, [12, 1, 0]).is_err());
    assert_eq!(r, before);
    // Still reject an invalid snapshot imported from an older version.
    r.rivit
        .iter_mut()
        .find(|row| row.output_index == finish)
        .unwrap()
        .clock = [12, 1, 0];
    r.kasikellot_muuttuneet = true;
    let history = p.leimaluvut.clone();
    let checked = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert!(!checked.can_approve());
    assert!(!checked.virheelliset_ajat.is_empty());
    assert_ne!(checked.tila, "TRK");
    assert_eq!(p.leimaluvut, history);
}
#[test]
fn finish_punch_precedes_trailing_extras_and_controls_clock_and_duration() {
    let mut p = example();
    p.radat[0].asemat.remove(2);
    p.radat[0].asemat.last_mut().unwrap().emit_koodi = 32;
    let marker = p.leimaluvut[0].leimat.pop().unwrap();
    p.leimaluvut[0].leimat.push(native::ReadPunch {
        koodi: 88,
        aika_sekuntia: 750,
    });
    p.leimaluvut[0].leimat.push(marker);
    let result = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(result.tila, "TRK");
    assert!(result.can_approve());
    assert_eq!(result.maalikello_sekuntia, 43800);
    assert_eq!(result.kesto_sekuntia, 600);
    assert_eq!(result.asemien_muut_virheet, 1200);
    assert_eq!(result.rivit.last().unwrap().received_code, 88);
}
#[test]
fn all_confirmed_interruption_conditions_are_retained_in_route_order() {
    let mut p = example();
    p.radat[0].asemat[1].tyyppi = "JAT".into();
    p.radat[0].asemat[1].emit_koodi = 77;
    p.radat[0].asemat[2].tyyppi = "JAT".into();
    p.radat[0].asemat[2].jakso_minuutteina = Some(0);
    p.radat[0].asemat[3].jakso_minuutteina = Some(0);
    p.leimaluvut[0].leimat[3].aika_sekuntia = 3721;
    p.leimaluvut[0].leimat[4].aika_sekuntia = 3800;
    let mut result = application::capture_project_result(&p, 7).unwrap();
    finish_clock(&mut result, [14, 0, 2]);
    let result = application::recalculate_stored_result(&p, 7, &result).unwrap();
    let reasons = result.syy.split("; ").collect::<Vec<_>>();
    assert_eq!(reasons.len(), 4);
    assert!(reasons[0].contains("JAT puuttuu"));
    assert!(reasons[1].contains("JAT-aseman"));
    assert!(reasons[2].contains("Maalijakson"));
    assert!(reasons[3].contains("kokonaismyöhästymisraja"));
    assert_eq!(result.tila, "KESK");
    assert!(result.can_approve());
}
#[test]
fn missing_vs_explicit_zero_clock_remains_distinct_at_midnight_start() {
    let mut p = example();
    p.kilpailijat[0].lahtoaika = "00:00:00".into();
    for station in &mut p.radat[0].asemat {
        station.emit_koodi = 0;
        station.tavoiteaika_sekuntia = 0;
        station.jakso_minuutteina = None;
    }
    let mut result = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(result.tila, "ILM");
    for index in result.pending_manual_clocks() {
        result.set_clock(index, [0; 3]).unwrap();
    }
    let result = application::recalculate_stored_result(&p, 7, &result).unwrap();
    assert!(result.can_approve());
    assert_eq!(result.tila, "TRK");
    assert_eq!(result.kesto_sekuntia, 0);
    assert!(
        result
            .rivit
            .iter()
            .filter(|r| r.sequence > 1)
            .all(|r| r.clock == [0; 3])
    );
}
#[test]
fn legacy_derived_manual_finish_requires_a_new_explicit_clock_on_native_review() {
    let p = example();
    let archive = p.to_archive().unwrap();
    let legacy = application::calculate(&archive)
        .unwrap()
        .competitors
        .remove(0)
        .calculation
        .unwrap();
    assert_eq!(legacy.totals.total_points, 2415);
    let mut old = application::capture_project_result(&p, 7).unwrap();
    old.laskentatapa = as2026::corrections::StoredCalculationPolicy::As2015aStatic;
    old.rivit.last_mut().unwrap().clock = [12, 10, 0];
    old.maalikello_sekuntia = 43800;
    old.kesto_sekuntia = 600;
    old.tila = "TRK".into();
    assert!(!old.can_approve());
    let reviewed = application::recalculate_stored_result(&p, 7, &old).unwrap();
    assert_eq!(reviewed.tila, "ILM");
    assert_eq!(reviewed.maalikello_sekuntia, 0);
    assert_eq!(reviewed.pending_manual_clocks().len(), 1);
}

#[test]
fn changed_start_and_correction_cannot_publish_stale_checked_result() {
    use as2026::{
        storage::ProjectStore,
        workspace::{DriverField, Workspace},
    };
    for (field, value) in [
        (DriverField::Start, "11:59:00"),
        (DriverField::Correction, "121"),
    ] {
        let (mut project, mut snapshot) = completed();
        snapshot.hyvaksytty = true;
        project.kilpailijat[0].tallennettu_tulos = Some(snapshot.clone());
        let dir = std::env::temp_dir().join(format!(
            "as2026-stale-time-{}-{field:?}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        ProjectStore::create(&dir.join(as2026::native::FILE_NAME), project).unwrap();
        let mut workspace = Workspace::open(&dir).unwrap();
        workspace.edit_driver(7, field, value).unwrap();
        let view = workspace.results().unwrap();
        assert_eq!(view.competitors[0].status, "ILM");
        assert!(view.competitors[0].header_fields[29].contains("Valitse Tutki"));
        assert_eq!(
            view.competitors[0]
                .results
                .iter()
                .map(as2026::corrections::StoredRow::from)
                .collect::<Vec<_>>(),
            snapshot.rivit,
            "muutos ei saa hävittää tallennettuja korjauksia tai laskea pisteitä salaa"
        );
        assert!(
            !workspace.project().unwrap().kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .hyvaksytty
        );
        workspace
            .edit_driver(7, DriverField::Status, "TRK")
            .unwrap();
        assert_eq!(workspace.results().unwrap().competitors[0].status, "ILM");
        for manual in ["HYL", "PER"] {
            workspace
                .edit_driver(7, DriverField::Status, manual)
                .unwrap();
            assert_eq!(workspace.results().unwrap().competitors[0].status, manual);
        }
        workspace.edit_driver(7, DriverField::Status, "").unwrap();
        workspace.recalculate_stored(7).unwrap();
        let reviewed = workspace.results().unwrap();
        assert_eq!(reviewed.competitors[0].status, "TRK");
        assert!(
            workspace.project().unwrap().kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .can_approve()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn jat_lateness_reason_uses_route_sequence_and_name_despite_preceding_extra_punch() {
    let mut project = example();
    project.radat[0].asemat[2].tyyppi = "JAT".into();
    project.radat[0].asemat[2].nimi = "Metsän JAT".into();
    project.radat[0].asemat[2].jakso_minuutteina = Some(0);
    project.radat[0].asemat[3].jakso_minuutteina = Some(0);
    project.leimaluvut[0].leimat[3].aika_sekuntia = 3721;
    project.leimaluvut[0].leimat[4].aika_sekuntia = 3800;
    let mut result = application::capture_project_result(&project, 7).unwrap();
    let jat = result
        .rivit
        .iter()
        .find(|r| r.station_type == "JAT")
        .unwrap();
    assert_eq!(jat.sequence, 3);
    assert_eq!(jat.output_index, 4);
    finish_clock(&mut result, [13, 0, 1]);
    let result = application::recalculate_stored_result(&project, 7, &result).unwrap();
    assert_eq!(result.tila, "KESK");
    assert!(
        result
            .syy
            .contains("JAT-aseman 3 Metsän JAT jakson myöhästymisraja ylitetty")
    );
    assert!(!result.syy.contains("JAT-aseman 4"));
}
