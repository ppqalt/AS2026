//! Toimitsijan KESK-päätös ei vaadi eikä keksi korttia, maaliaikaa tai asematulosta.
use as2026::{
    ClockTime,
    changes::Changes,
    emit,
    native::{self, ManualStatus, Project},
    workspace::{DriverField, Workspace},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "as2026-keskeytysvahvistus-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join(native::FILE_NAME)
    }
    fn workspace(&self, read: bool) -> Workspace {
        let mut project = example();
        if !read {
            project.leimaluvut.clear();
        }
        Workspace::create_imported(&self.0, project).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn example() -> Project {
    Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}
#[test]
fn reasoned_interruption_is_confirmed_and_reopened_without_creating_scoring_data() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(false);
    assert!(!workspace.can_confirm_interruption(7));
    assert!(!workspace.interruption_confirmed(7));
    workspace
        .edit_driver(7, DriverField::Status, "KESK")
        .unwrap();
    assert!(!workspace.can_confirm_interruption(7));
    workspace
        .edit_driver(7, DriverField::StatusReason, "Moottorivaurio reitillä")
        .unwrap();
    assert!(workspace.can_confirm_interruption(7));
    workspace.approve_result(7).unwrap();
    let project = workspace.project().unwrap();
    let driver = &project.kilpailijat[0];
    assert!(driver.has_confirmed_interruption());
    assert!(workspace.interruption_confirmed(7));
    assert!(!workspace.can_confirm_interruption(7));
    assert_eq!(driver.tila.as_ref().unwrap().tunnus, "KESK");
    assert_eq!(driver.tila.as_ref().unwrap().syy, "Moottorivaurio reitillä");
    assert!(driver.tallennettu_tulos.is_none());
    assert!(project.leimaluvut.is_empty());
    assert_eq!(driver.lisavirheet, native::AdditionalPenalties::default());
    let reopened = Workspace::open(&directory.0).unwrap();
    assert_eq!(reopened.project(), Some(project));
    assert!(reopened.interruption_confirmed(7));
    assert!(
        fs::read_to_string(directory.file())
            .unwrap()
            .contains("keskeytys_vahvistettu = true")
    );
}
#[test]
fn invalid_reason_and_missing_competitor_fail_atomically() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(false);
    let before = workspace.project().unwrap().clone();
    let bytes = fs::read(directory.file()).unwrap();
    for reason in ["", "   ", "\n", "Syy\nToinen rivi", "Syy\0"] {
        assert!(
            workspace.confirm_interruption(7, reason).is_err(),
            "{reason:?}"
        );
        assert_eq!(workspace.project(), Some(&before));
        assert_eq!(fs::read(directory.file()).unwrap(), bytes);
    }
    assert!(
        workspace
            .confirm_interruption(999, "Kelvollinen syy")
            .is_err()
    );
    assert_eq!(workspace.project(), Some(&before));
    assert_eq!(fs::read(directory.file()).unwrap(), bytes);
    workspace
        .confirm_interruption(7, "  Kortti ei palaudu keskeytyksen vuoksi  ")
        .unwrap();
    assert_eq!(
        workspace.project().unwrap().kilpailijat[0]
            .tila
            .as_ref()
            .unwrap()
            .syy,
        "Kortti ei palaudu keskeytyksen vuoksi"
    );
}
#[test]
fn editing_reason_or_status_revokes_confirmation_but_other_statuses_are_preserved() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(false);
    workspace
        .confirm_interruption(7, "Alkuperäinen syy")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::StatusReason, "Tarkennettu syy")
        .unwrap();
    assert!(!workspace.interruption_confirmed(7));
    assert!(workspace.can_confirm_interruption(7));
    workspace.approve_result(7).unwrap();
    assert!(workspace.interruption_confirmed(7));
    for status in ["HYL", "PER"] {
        workspace
            .edit_driver(7, DriverField::Status, status)
            .unwrap();
        assert!(!workspace.interruption_confirmed(7));
        assert_eq!(
            workspace.project().unwrap().kilpailijat[0]
                .tila
                .as_ref()
                .unwrap()
                .tunnus,
            status
        );
        workspace
            .confirm_interruption(7, "Keskeytyspäätös")
            .unwrap();
    }
    workspace.edit_driver(7, DriverField::Status, "").unwrap();
    assert!(workspace.project().unwrap().kilpailijat[0].tila.is_none());
    assert!(!workspace.interruption_confirmed(7));
}
#[test]
fn partial_card_snapshot_and_later_reread_do_not_destroy_confirmed_decision() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    workspace.ensure_readout_result(7).unwrap();
    let pending = workspace.result_snapshot(7).unwrap().unwrap();
    assert!(!pending.can_approve());
    workspace
        .confirm_interruption(7, "Ajo päättyi ennen maalia")
        .unwrap();
    assert_eq!(workspace.result_snapshot(7).unwrap().unwrap(), pending);
    let before = workspace.project().unwrap().leimaluvut.clone();
    let mut frame =
        emit::parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw")).unwrap();
    frame.records[1].elapsed_seconds += 1;
    workspace
        .record_frame(&frame, ClockTime::new(12, 12, 0).unwrap())
        .unwrap();
    workspace.ensure_readout_result(7).unwrap();
    assert!(workspace.interruption_confirmed(7));
    assert_eq!(workspace.result_snapshot(7).unwrap().unwrap(), pending);
    assert_eq!(
        &workspace.project().unwrap().leimaluvut[..before.len()],
        before.as_slice()
    );
    assert_eq!(
        workspace.project().unwrap().kilpailijat[0]
            .tila
            .as_ref()
            .unwrap()
            .syy,
        "Ajo päättyi ennen maalia"
    );
}
#[test]
fn old_files_default_to_unconfirmed_and_invalid_confirmation_flags_are_rejected() {
    let mut project = example();
    assert!(!project.kilpailijat[0].keskeytys_vahvistettu);
    assert!(!project.to_toml().unwrap().contains("keskeytys_vahvistettu"));
    project.kilpailijat[0].keskeytys_vahvistettu = true;
    assert!(project.validate().is_err());
    project.kilpailijat[0].tila = Some(ManualStatus::new("KESK", "").unwrap());
    assert!(project.validate().is_err());
    project.kilpailijat[0].tila = Some(ManualStatus::new("HYL", "Syy").unwrap());
    assert!(project.validate().is_err());
    project.kilpailijat[0].tila = Some(ManualStatus::new("KESK", "Perusteltu syy").unwrap());
    assert!(project.validate().is_ok());
}
fn change_driver(project: &mut Project, driver: &native::Driver, status: bool, confirmation: bool) {
    let mut fields = toml::Value::try_from(driver)
        .unwrap()
        .as_table()
        .unwrap()
        .clone();
    if !status {
        fields.remove("tila");
    }
    if !confirmation {
        fields.remove("keskeytys_vahvistettu");
    }
    let mut root = toml::map::Map::new();
    root.insert(
        "kilpailijat".into(),
        toml::Value::Array(vec![toml::Value::Table(fields)]),
    );
    let changes: Changes =
        toml::from_str(&toml::to_string(&toml::Value::Table(root)).unwrap()).unwrap();
    changes.apply(project).unwrap();
}
#[test]
fn older_change_files_preserve_confirmation_only_when_status_is_unchanged() {
    let mut project = example();
    project.kilpailijat[0].tila = Some(ManualStatus::new("KESK", "Keskeytyksen syy").unwrap());
    project.kilpailijat[0].keskeytys_vahvistettu = true;
    let mut driver = project.kilpailijat[0].clone();
    driver.kuljettaja = "Korjattu nimi".into();
    change_driver(&mut project, &driver, false, false);
    assert!(project.kilpailijat[0].has_confirmed_interruption());
    assert_eq!(project.kilpailijat[0].kuljettaja, "Korjattu nimi");
    driver.tila = Some(ManualStatus::new("KESK", "Muutettu syy").unwrap());
    change_driver(&mut project, &driver, true, true);
    assert!(!project.kilpailijat[0].keskeytys_vahvistettu);
    assert_eq!(
        project.kilpailijat[0].tila.as_ref().unwrap().syy,
        "Muutettu syy"
    );
}

fn complete_result(workspace: &mut Workspace) {
    workspace.ensure_readout_result(7).unwrap();
    let finish = workspace
        .result_snapshot(7)
        .unwrap()
        .unwrap()
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    workspace
        .edit_station_result(7, finish, as2026::workspace::ResultField::Clock, "12:10:00")
        .unwrap();
    workspace.recalculate_stored(7).unwrap();
    workspace.approve_result(7).unwrap();
}
#[test]
fn calculation_setting_edits_revoke_result_approval_without_revoking_interruption_decision() {
    use as2026::workspace::{ClassField, CompetitionField, OverrideField};
    for variant in 0..3 {
        let directory = Directory::new();
        let mut workspace = directory.workspace(true);
        complete_result(&mut workspace);
        workspace
            .confirm_interruption(7, "Toimitsijan vahvistettu päätös")
            .unwrap();
        match variant {
            0 => workspace
                .edit_competition(CompetitionField::Timing, "0")
                .unwrap(),
            1 => workspace
                .edit_override(OverrideField::FreeTime, "11")
                .unwrap(),
            _ => workspace
                .edit_class("A", ClassField::Lateness, "61")
                .unwrap(),
        }
        let snapshot = workspace.result_snapshot(7).unwrap().unwrap();
        assert!(!snapshot.hyvaksytty);
        assert!(snapshot.kasikellot_muuttuneet);
        assert!(!snapshot.can_approve());
        assert!(
            workspace.interruption_confirmed(7),
            "päätös ei riipu asetusten vaihtumisesta"
        );
    }
}
#[test]
fn ranking_only_edits_do_not_require_station_recalculation() {
    use as2026::workspace::ClassField;
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    complete_result(&mut workspace);
    workspace.edit_class("A", ClassField::Np, "1").unwrap();
    workspace.edit_class("A", ClassField::Sp, "1").unwrap();
    workspace
        .edit_driver(7, DriverField::DriverLicence, "123X")
        .unwrap();
    let snapshot = workspace.result_snapshot(7).unwrap().unwrap();
    assert!(snapshot.hyvaksytty);
    assert!(!snapshot.kasikellot_muuttuneet);
    assert!(snapshot.can_approve());
}
#[test]
fn card_replacement_requires_new_card_data_and_does_not_move_old_snapshot() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    complete_result(&mut workspace);
    let history = workspace.project().unwrap().leimaluvut.clone();
    workspace
        .edit_driver(7, DriverField::Card, "654321")
        .unwrap();
    assert!(
        workspace.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .is_none()
    );
    assert!(workspace.result_snapshot(7).unwrap().is_none());
    assert!(workspace.approve_result(7).is_err());
    assert!(workspace.recalculate_stored(7).is_err());
    assert!(workspace.review_results(7).is_err());
    assert_eq!(workspace.project().unwrap().leimaluvut, history);
}
#[test]
fn changes_files_revoke_approval_when_timing_inputs_change() {
    for variant in 0..4 {
        let directory = Directory::new();
        let mut workspace = directory.workspace(true);
        complete_result(&mut workspace);
        let mut project = workspace.project().unwrap().clone();
        match variant {
            0 => {
                let changes: Changes = toml::from_str(
                    "[kilpailu]\nnumero=1\nnimi='Esimerkkikilpailu'\najanottotapa=0",
                )
                .unwrap();
                changes.apply(&mut project).unwrap();
            }
            1 => {
                let changes: Changes =
                    toml::from_str("[virhepisteet]\nvapaa_aika_sekunteina=11").unwrap();
                changes.apply(&mut project).unwrap();
            }
            2 => {
                let mut driver = project.kilpailijat[0].clone();
                driver.lahtoaika = "11:59:00".into();
                change_driver(&mut project, &driver, false, false);
            }
            _ => {
                let mut driver = project.kilpailijat[0].clone();
                driver.emit = 654321;
                change_driver(&mut project, &driver, false, false);
            }
        }
        if variant == 3 {
            assert!(project.kilpailijat[0].tallennettu_tulos.is_none());
        } else {
            let snapshot = project.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
            assert!(!snapshot.hyvaksytty);
            assert!(snapshot.kasikellot_muuttuneet);
        }
    }
}
