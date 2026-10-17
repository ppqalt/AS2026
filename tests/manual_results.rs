//! Käsikorjausten synteettiset työnkulut; alkuperäistä ohjelmaa ei suoriteta.
use as2026::{application, native};

fn project() -> native::Project {
    native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}
fn manual_project() -> native::Project {
    let mut p = project();
    p.kilpailu.ajanottotapa = 0;
    p.radat[0].asemat[1].emit_koodi = 0;
    p
}
#[test]
fn clock_edit_is_saved_without_rescoring_then_tutki_uses_stored_clock() {
    let mut p = manual_project();
    let original_history = p.leimaluvut.clone();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    let before = result.clone();
    result.set_clock(2, [12, 4, 35]).unwrap();
    assert_eq!(
        result.rivit[1].signed_time_penalty,
        before.rivit[1].signed_time_penalty
    );
    assert_eq!(
        result.rivit[1].actual_seconds,
        before.rivit[1].actual_seconds
    );
    assert_eq!(result.aikavirheet, before.aikavirheet);
    assert!(result.kasikellot_muuttuneet);
    p.kilpailijat[0].tallennettu_tulos = Some(result.clone());
    let visible = application::calculated_project(&p).unwrap();
    assert_eq!(visible.competitors[0].results[1].clock, [12, 4, 35]);
    assert_eq!(
        visible.competitors[0].results[1].signed_time_penalty,
        before.rivit[1].signed_time_penalty
    );
    let recalculated = application::recalculate_stored_result(&p, 7, &result).unwrap();
    assert_eq!(recalculated.rivit[1].elapsed_seconds, 275);
    assert_eq!(recalculated.rivit[1].actual_seconds, 275);
    assert_eq!(recalculated.rivit[1].signed_time_penalty, 45);
    assert!(!recalculated.kasikellot_muuttuneet);
    assert_eq!(p.leimaluvut, original_history);
    let rebuilt = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(rebuilt.rivit[1].clock, [0; 3]);
    assert_eq!(rebuilt.rivit[1].actual_seconds, 0);
}
#[test]
fn row_penalties_update_five_part_sum_and_survive_newer_readout() {
    let mut p = project();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    result.set_row_penalties(2, -100, 7).unwrap();
    p.kilpailijat[0].lisavirheet = native::AdditionalPenalties {
        puuttuvat_rt_kpl: 1,
        puuttuvat_rt_pisteet: 600,
        ylimaaraiset_rt_kpl: 2,
        ylimaaraiset_rt_pisteet: 9,
        muut_virhepisteet: 3,
    };
    p.kilpailijat[0].tila = Some(native::ManualStatus {
        tunnus: "HYL".into(),
        syy: "Muu syy".into(),
    });
    p.kilpailijat[0].tallennettu_tulos = Some(result.clone());
    let before_history = p.leimaluvut.clone();
    let mut newer = p.leimaluvut[0].clone();
    newer.leimat[1].aika_sekuntia = 420;
    p.record_readout(newer).unwrap();
    assert_eq!(
        &p.leimaluvut[..before_history.len()],
        before_history.as_slice()
    );
    let a = application::calculated_project(&p).unwrap();
    assert_eq!(a.competitors[0].results[1].signed_time_penalty, -100);
    assert_eq!(a.competitors[0].results[1].other_penalty, 7);
    assert_eq!(a.competitors[0].status, "HYL");
    assert_eq!(a.competitors[0].header_fields[29], "Muu syy");
    assert_eq!(
        a.competitors[0].total_penalties,
        result.aikavirheet + result.asemien_muut_virheet + 600 + 9 + 3
    );
    let rebuilt = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(rebuilt.rivit[1].signed_time_penalty, 0);
    assert_eq!(rebuilt.rivit[1].other_penalty, 0);
}
#[test]
fn snapshot_round_trip_keeps_finnish_fields_and_all_manual_values() {
    let mut p = manual_project();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    result.set_clock(2, [12, 4, 35]).unwrap();
    result.set_row_penalties(2, -55, 12).unwrap();
    p.kilpailijat[0].tallennettu_tulos = Some(result);
    let text = p.to_toml().unwrap();
    assert!(text.contains("tallennettu_tulos"));
    assert!(text.contains("aikavirhe = -55"));
    assert!(!text.contains("signed_time_penalty"));
    let loaded = native::Project::from_toml(&text).unwrap();
    assert_eq!(loaded, p);
    let a = application::calculated_project(&loaded).unwrap();
    assert_eq!(a.competitors[0].results[1].clock, [12, 4, 35]);
    assert_eq!(a.competitors[0].results[1].signed_time_penalty, -55);
}
#[test]
fn route_change_blocks_stale_snapshot_but_explicit_raw_rebuild_is_available() {
    let mut p = project();
    p.kilpailijat[0].tallennettu_tulos = Some(application::capture_project_result(&p, 7).unwrap());
    p.radat[0].asemat[1].tavoiteaika_sekuntia += 1;
    let view = application::calculated_project(&p).unwrap();
    assert_eq!(view.competitors[0].status, "ILM");
    assert!(view.competitors[0].results.is_empty());
    assert!(
        view.warnings
            .iter()
            .any(|w| w.contains("rata on muuttunut"))
    );
    let stored = p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
    assert!(application::recalculate_stored_result(&p, 7, stored).is_err());
    assert!(application::capture_project_result(&p, 7).is_ok());
}
#[test]
fn operational_manual_finish_does_not_use_undefined_static_path() {
    let p = project();
    let result = application::capture_project_result(&p, 7).unwrap();
    let checked = application::recalculate_stored_result(&p, 7, &result).unwrap();
    assert_eq!(checked.tila, "ILM");
    assert_eq!(checked.maalikello_sekuntia, 0);
    assert!(!checked.can_approve());
    // Tutkimusrajapinta säilyttää alkuperäisen alustamattoman tapauksen torjunnan.
    let mut archive = p.to_archive().unwrap();
    archive.competitors[0].results = result.as_station_results();
    assert!(application::prepare_stored_competitor_timing(&archive, 7).is_err());
}
#[test]
fn invalid_penalty_edit_is_atomic() {
    let p = project();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    let original = result.clone();
    assert!(result.set_row_penalties(2, i64::MIN, 5).is_err());
    assert_eq!(result, original);
    assert!(result.set_clock(2, [25, 0, 0]).is_err());
    assert_eq!(result, original);
}
#[test]
fn recheck_does_not_keep_old_automatic_reason_as_current_outcome() {
    let p = manual_project();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    result.tila = "KESK".into();
    result.syy = "Vanha syy".into();
    let checked = application::recalculate_stored_result(&p, 7, &result).unwrap();
    assert_eq!(checked.tila, "ILM");
    assert!(!checked.syy.contains("Vanha syy"));
    assert!(checked.syy.contains("käsikello puuttuu"));
}
#[test]
fn operational_emit_and_mode_two_use_the_live_coordinate_reference() {
    let mut p = project();
    p.radat[0].asemat.remove(2);
    p.radat[0].asemat.last_mut().unwrap().emit_koodi = 32;
    let stored = application::capture_project_result(&p, 7).unwrap();
    let emit = application::recalculate_stored_result(&p, 7, &stored).unwrap();
    assert_eq!(emit.rivit[1].actual_seconds, 275);
    assert_eq!(emit.rivit.last().unwrap().actual_seconds, 600);
    assert_eq!(emit.aikavirheet + emit.asemien_muut_virheet, 645);
    p.kilpailu.ajanottotapa = 2;
    let mode_two = application::recalculate_stored_result(&p, 7, &stored).unwrap();
    assert_eq!(mode_two.korjaus_sekuntia, 120);
    assert_eq!(mode_two.rivit[1].actual_seconds, 275);
    assert_eq!(mode_two.rivit.last().unwrap().actual_seconds, 600);
    assert_eq!(mode_two.aikavirheet + mode_two.asemien_muut_virheet, 645);
}
#[test]
fn missing_manual_station_waits_for_operator_clock_without_missing_punch_penalty() {
    let p = manual_project();
    let stored = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(stored.rivit[1].other_penalty, 0);
    let checked = application::recalculate_stored_result(&p, 7, &stored).unwrap();
    assert_eq!(checked.rivit[1].other_penalty, 0);
    assert_eq!(checked.rivit[1].note, "Käsikello puuttuu");
    assert_eq!(checked.tila, "ILM");
}
struct Directory(std::path::PathBuf);
impl Directory {
    fn new(project: native::Project) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let path = std::env::temp_dir().join(format!(
            "as2026-manual-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        as2026::storage::ProjectStore::create(&path.join(native::FILE_NAME), project).unwrap();
        Self(path)
    }
    fn workspace(&self) -> as2026::workspace::Workspace {
        as2026::workspace::Workspace::open(&self.0).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn workspace_clock_and_penalty_edits_reopen_and_require_explicit_recalculation() {
    use as2026::workspace::{DriverField, ResultField};
    let project = manual_project();
    let history = project.leimaluvut.clone();
    let dir = Directory::new(project);
    let mut workspace = dir.workspace();
    workspace
        .edit_station_result(7, 2, ResultField::Clock, "12:04:35")
        .unwrap();
    workspace
        .edit_station_result(7, 2, ResultField::TimePenalty, "-13")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::OtherPenalties, "17")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::Status, "HYL")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::StatusReason, "Tuomariston päätös")
        .unwrap();
    let mut reopened = dir.workspace();
    assert_eq!(reopened.project().unwrap().leimaluvut, history);
    let saved = reopened.results().unwrap();
    assert_eq!(saved.competitors[0].results[1].clock, [12, 4, 35]);
    assert_eq!(saved.competitors[0].results[1].signed_time_penalty, -13);
    assert_eq!(saved.competitors[0].status, "HYL");
    reopened.recalculate_stored(7).unwrap();
    let after = dir.workspace().results().unwrap();
    assert_eq!(after.competitors[0].results[1].signed_time_penalty, 45);
    assert_eq!(after.competitors[0].status, "HYL");
    assert_eq!(after.competitors[0].header_fields[29], "Tuomariston päätös");
    assert_eq!(reopened.project().unwrap().leimaluvut, history);
    reopened.review_results(7).unwrap();
    let rebuilt = dir.workspace().results().unwrap();
    assert_eq!(rebuilt.competitors[0].results[1].clock, [0; 3]);
    assert_eq!(rebuilt.competitors[0].results[1].signed_time_penalty, 0);
    assert_eq!(rebuilt.competitors[0].status, "HYL");
    assert_eq!(reopened.project().unwrap().leimaluvut, history);
}
#[test]
fn workspace_failed_clock_edit_preserves_file_and_pending_manual_edits() {
    use as2026::workspace::ResultField;
    let dir = Directory::new(project());
    let mut workspace = dir.workspace();
    workspace.review_results(7).unwrap();
    workspace
        .edit_station_result(7, 2, ResultField::TimePenalty, "-13")
        .unwrap();
    let before = std::fs::read(dir.0.join(native::FILE_NAME)).unwrap();
    let in_memory = workspace.project().unwrap().clone();
    assert!(
        workspace
            .edit_station_result(7, 5, ResultField::Clock, "25:00:00")
            .is_err()
    );
    assert_eq!(workspace.project().unwrap(), &in_memory);
    assert_eq!(
        std::fs::read(dir.0.join(native::FILE_NAME)).unwrap(),
        before
    );
}
#[test]
fn workspace_stale_route_stays_accessible_for_explicit_tarkista() {
    use as2026::workspace::StationField;
    let dir = Directory::new(project());
    let mut workspace = dir.workspace();
    workspace.review_results(7).unwrap();
    workspace
        .edit_station(1, 1, StationField::Target, "301")
        .unwrap();
    let unavailable = workspace.results().unwrap();
    assert!(unavailable.competitors[0].results.is_empty());
    assert_eq!(unavailable.competitors[0].status, "ILM");
    assert!(
        workspace.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .is_some()
    );
    workspace.review_results(7).unwrap();
    assert!(
        !dir.workspace().results().unwrap().competitors[0]
            .results
            .is_empty()
    );
}
#[test]
fn tampered_snapshot_sums_are_not_ranked_or_used_for_recalculation() {
    let mut p = project();
    let mut snapshot = application::capture_project_result(&p, 7).unwrap();
    snapshot.aikavirheet = 0;
    p.kilpailijat[0].tallennettu_tulos = Some(snapshot.clone());
    p.kilpailijat[0].tila = Some(native::ManualStatus {
        tunnus: "TRK".into(),
        syy: String::new(),
    });
    let view = application::calculated_project(&p).unwrap();
    assert_eq!(view.competitors[0].status, "ILM");
    assert!(view.competitors[0].results.is_empty());
    assert!(
        view.warnings
            .iter()
            .any(|w| w.contains("pistesummat eivät vastaa"))
    );
    assert!(application::recalculate_stored_result(&p, 7, &snapshot).is_err());
}
#[test]
fn manual_classified_status_cannot_publish_zero_without_calculated_rows() {
    for requested in ["TRK", "KESK"] {
        let mut p = project();
        p.leimaluvut.clear();
        p.kilpailijat[0].tila = Some(native::ManualStatus {
            tunnus: requested.into(),
            syy: "Käsin valittu".into(),
        });
        let view = application::calculated_project(&p).unwrap();
        assert_eq!(view.competitors[0].status, "ILM");
        assert!(view.competitors[0].results.is_empty());
        assert!(view.competitors[0].header_fields[29].contains("puuttuu"));
        assert!(
            view.warnings
                .iter()
                .any(|w| w.contains("ilman laskettua asematulosta"))
        );
        assert_eq!(p.kilpailijat[0].tila.as_ref().unwrap().tunnus, requested);
        let ranking =
            application::rank_stored(&view, as2026::ranking::TiePolicy::KeepSharedAndFlag).unwrap();
        assert!(
            ranking
                .groups
                .iter()
                .flat_map(|g| &g.entries)
                .all(|entry| entry.status != as2026::ranking::Status::Classified)
        );
    }
}
