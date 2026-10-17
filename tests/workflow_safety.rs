//! Toimitsijan hyväksyntäpolku yhteisen työtilan kautta; ei rinnakkaista laskentaa.
use as2026::{
    ClockTime, emit, excel_import, native,
    storage::ProjectStore,
    workspace::{DriverField, ResultField, StationField, Workspace},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "as2026-tyonkulun-turvallisuus-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join(native::FILE_NAME)
    }
    fn workspace(&self) -> Workspace {
        let project =
            native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
                .unwrap();
        Workspace::create_imported(&self.0, project).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Korttiluvun käsimaali täydennetään ennen Tutki-toimintoa ja hyväksyntää.
fn complete_manual_finish(workspace: &mut Workspace) -> u32 {
    workspace.ensure_readout_result(7).unwrap();
    let before = workspace.result_snapshot(7).unwrap().unwrap();
    let finish = before
        .rivit
        .iter()
        .find(|row| row.station_type == "M")
        .unwrap()
        .output_index;
    assert!(before.pending_manual_clocks().contains(&finish));
    workspace
        .edit_station_result(7, finish, ResultField::Clock, "12:11:00")
        .unwrap();
    let edited = workspace.result_snapshot(7).unwrap().unwrap();
    assert!(edited.pending_manual_clocks().is_empty());
    assert!(edited.kasikellot_muuttuneet);
    assert!(!edited.can_approve());
    workspace.recalculate_stored(7).unwrap();
    let checked = workspace.result_snapshot(7).unwrap().unwrap();
    assert!(checked.can_approve(), "{checked:#?}");
    assert!(!checked.kasikellot_muuttuneet);
    assert_eq!(
        checked
            .rivit
            .iter()
            .find(|row| row.output_index == finish)
            .unwrap()
            .clock,
        [12, 11, 0]
    );
    finish
}

#[test]
fn unread_competitor_has_no_snapshot_and_approval_does_not_create_a_result() {
    let directory = Directory::new();
    let mut project =
        native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    project.leimaluvut.clear();
    let mut workspace = Workspace::create_imported(&directory.0, project).unwrap();
    let bytes = fs::read(directory.file()).unwrap();
    assert!(workspace.result_snapshot(7).unwrap().is_none());
    assert!(workspace.approve_result(7).is_err());
    assert!(
        workspace.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .is_none()
    );
    assert_eq!(fs::read(directory.file()).unwrap(), bytes);
}

#[test]
fn missing_manual_clock_and_unexamined_clock_change_cannot_be_approved() {
    let directory = Directory::new();
    let mut workspace = directory.workspace();
    workspace.ensure_readout_result(7).unwrap();
    let snapshot = workspace.result_snapshot(7).unwrap().unwrap();
    assert!(!snapshot.pending_manual_clocks().is_empty());
    let bytes = fs::read(directory.file()).unwrap();
    assert!(workspace.approve_result(7).is_err());
    assert_eq!(fs::read(directory.file()).unwrap(), bytes);
    let finish = snapshot
        .rivit
        .iter()
        .find(|row| row.station_type == "M")
        .unwrap()
        .output_index;
    workspace
        .edit_station_result(7, finish, ResultField::Clock, "12:11:00")
        .unwrap();
    let bytes = fs::read(directory.file()).unwrap();
    assert!(workspace.approve_result(7).is_err());
    assert_eq!(fs::read(directory.file()).unwrap(), bytes);
    assert!(!workspace.result_snapshot(7).unwrap().unwrap().hyvaksytty);
}

#[test]
fn approval_after_clock_and_examination_preserves_explicit_station_point_corrections() {
    let directory = Directory::new();
    let mut workspace = directory.workspace();
    let original_reads = workspace.project().unwrap().leimaluvut.clone();
    complete_manual_finish(&mut workspace);
    let at = workspace
        .result_snapshot(7)
        .unwrap()
        .unwrap()
        .rivit
        .iter()
        .find(|row| row.expected_code == 31)
        .unwrap()
        .output_index;
    workspace
        .edit_station_result(7, at, ResultField::TimePenalty, "77")
        .unwrap();
    workspace
        .edit_station_result(7, at, ResultField::OtherPenalty, "9")
        .unwrap();
    let mut expected = workspace.result_snapshot(7).unwrap().unwrap();
    assert!(!expected.hyvaksytty);
    workspace.approve_result(7).unwrap();
    expected.hyvaksytty = true;
    assert_eq!(workspace.result_snapshot(7).unwrap().unwrap(), expected);
    let result = workspace.results().unwrap();
    let row = result.competitors[0]
        .results
        .iter()
        .find(|row| row.output_index == at)
        .unwrap();
    assert_eq!(row.signed_time_penalty, 77);
    assert_eq!(row.other_penalty, 9);
    assert_eq!(row.total_penalty, 86);
    assert_eq!(result.competitors[0].status, "TRK");
    assert_eq!(workspace.project().unwrap().leimaluvut, original_reads);
    assert_eq!(
        Workspace::open(&directory.0)
            .unwrap()
            .result_snapshot(7)
            .unwrap()
            .unwrap(),
        expected
    );
}

#[test]
fn changed_and_repeated_card_reads_preserve_approved_clocks_points_and_manual_status() {
    let directory = Directory::new();
    let mut workspace = directory.workspace();
    complete_manual_finish(&mut workspace);
    let at = workspace
        .result_snapshot(7)
        .unwrap()
        .unwrap()
        .rivit
        .iter()
        .find(|row| row.expected_code == 31)
        .unwrap()
        .output_index;
    workspace
        .edit_station_result(7, at, ResultField::TimePenalty, "77")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::Status, "HYL")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::StatusReason, "Toimitsijan päätös")
        .unwrap();
    workspace.approve_result(7).unwrap();
    let approved = workspace.result_snapshot(7).unwrap().unwrap();
    let mut frame =
        emit::parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw")).unwrap();
    assert_eq!(frame.card_number, 123456);
    frame.records[1].elapsed_seconds += 7;
    let before_reads = workspace.project().unwrap().leimaluvut.len();
    let outcome = workspace
        .record_frame(&frame, ClockTime::new(12, 12, 0).unwrap())
        .unwrap();
    assert!(outcome.content_changed);
    assert_eq!(outcome.competitor_number, Some(7));
    workspace.ensure_readout_result(7).unwrap();
    assert_eq!(workspace.result_snapshot(7).unwrap().unwrap(), approved);
    frame.records[4].elapsed_seconds += 1;
    let outcome = workspace
        .record_frame(&frame, ClockTime::new(12, 12, 1).unwrap())
        .unwrap();
    assert!(!outcome.content_changed);
    workspace.ensure_readout_result(7).unwrap();
    assert_eq!(workspace.result_snapshot(7).unwrap().unwrap(), approved);
    assert_eq!(
        workspace.project().unwrap().leimaluvut.len(),
        before_reads + 2
    );
    let result = workspace.results().unwrap();
    assert_eq!(result.competitors.len(), 1);
    assert_eq!(result.competitors[0].status, "HYL");
    assert_eq!(
        result.competitors[0].header_fields[29],
        "Toimitsijan päätös"
    );
    assert_eq!(
        Workspace::open(&directory.0)
            .unwrap()
            .result_snapshot(7)
            .unwrap()
            .unwrap(),
        approved
    );
}

#[test]
fn stale_route_or_inconsistent_stored_points_cannot_gain_approval() {
    let directory = Directory::new();
    let mut workspace = directory.workspace();
    complete_manual_finish(&mut workspace);
    workspace
        .edit_station(1, 1, StationField::Target, "301")
        .unwrap();
    let bytes = fs::read(directory.file()).unwrap();
    assert!(workspace.approve_result(7).is_err());
    assert_eq!(fs::read(directory.file()).unwrap(), bytes);

    // Tarkista muodostaa uuden radan mukaiset rivit; käsimaali täydennetään uudestaan.
    workspace.review_results(7).unwrap();
    complete_manual_finish(&mut workspace);
    ProjectStore::open(&directory.file())
        .unwrap()
        .update(|project| {
            project.kilpailijat[0]
                .tallennettu_tulos
                .as_mut()
                .unwrap()
                .rivit[0]
                .total_penalty += 1;
            Ok(())
        })
        .unwrap();
    let mut reopened = Workspace::open(&directory.0).unwrap();
    let bytes = fs::read(directory.file()).unwrap();
    assert!(reopened.approve_result(7).is_err());
    assert_eq!(fs::read(directory.file()).unwrap(), bytes);
}

#[test]
fn competitor_additional_penalty_edits_invalidate_approval_and_keep_station_corrections() {
    let directory = Directory::new();
    let mut workspace = directory.workspace();
    complete_manual_finish(&mut workspace);
    let at = workspace
        .result_snapshot(7)
        .unwrap()
        .unwrap()
        .rivit
        .iter()
        .find(|row| row.expected_code == 31)
        .unwrap()
        .output_index;
    workspace
        .edit_station_result(7, at, ResultField::TimePenalty, "77")
        .unwrap();
    workspace.approve_result(7).unwrap();
    for (field, value) in [
        (DriverField::MissingRtCount, "3"),
        (DriverField::ExtraRtCount, "1"),
        (DriverField::OtherPenalties, "25"),
    ] {
        workspace.edit_driver(7, field, value).unwrap();
        let snapshot = workspace.result_snapshot(7).unwrap().unwrap();
        assert!(!snapshot.hyvaksytty);
        assert_eq!(
            snapshot
                .rivit
                .iter()
                .find(|row| row.output_index == at)
                .unwrap()
                .signed_time_penalty,
            77
        );
        workspace.approve_result(7).unwrap();
    }
    let project = workspace.project().unwrap();
    assert_eq!(
        project.kilpailijat[0].lisavirheet.puuttuvat_rt_pisteet,
        1800
    );
    assert_eq!(project.kilpailijat[0].lisavirheet.muut_virhepisteet, 25);
    assert_eq!(
        Workspace::open(&directory.0).unwrap().project(),
        Some(project)
    );
}

#[test]
fn excel_import_create_is_exclusive_and_reopen_preserves_clubs_and_draw_choices() {
    let directory = Directory::new();
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/excel-import/synteettinen.xlsx");
    let project = excel_import::read(&path)
        .unwrap()
        .to_project("Tuontikoe Ää")
        .unwrap();
    let workspace = Workspace::create_imported(&directory.0, project.clone()).unwrap();
    assert_eq!(workspace.project(), Some(&project));
    let reopened = Workspace::open(&directory.0).unwrap();
    assert_eq!(reopened.project(), Some(&project));
    assert_eq!(
        reopened.project().unwrap().kilpailijat[0].kuljettajan_seura,
        "Seura A"
    );
    assert_eq!(
        reopened.project().unwrap().kilpailijat[0].kartanlukijan_seura,
        "Seura B"
    );
    assert!(
        reopened
            .project()
            .unwrap()
            .luokat
            .iter()
            .all(|class| !class.osallistuu_lahtoarvontaan)
    );
    let before = fs::read(directory.file()).unwrap();
    assert!(
        Workspace::create_imported(&directory.0, native::Project::new("Korvaava").unwrap())
            .is_err()
    );
    assert_eq!(fs::read(directory.file()).unwrap(), before);
}
