use as2026::{
    ClockTime, emit, native,
    storage::ProjectStore,
    workspace::{
        ClassEdit, ClassField, CompetitionField, DriverEdit, DriverField, OverrideField,
        StationEdit, StationField, TrackEdit, TrackField, Workspace,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(1);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "as2026-workspace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join(native::FILE_NAME)
    }
    fn example(&self) -> Workspace {
        let project =
            native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
                .unwrap();
        ProjectStore::create(&self.file(), project).unwrap();
        Workspace::open(&self.0).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn driver(number: &str, card: &str) -> DriverEdit {
    DriverEdit {
        number: number.into(),
        class_code: "A".into(),
        driver: "Koe".into(),
        navigator: "Testi".into(),
        card_number: card.into(),
        car: "Harjoitusauto".into(),
        start_time: "12:00:00".into(),
        correction_seconds: "0".into(),
    }
}

#[test]
fn new_competition_reopens_without_replacing_existing_file() {
    let directory = Directory::new();
    let workspace = Workspace::create(&directory.0, "  Harjoituskilpailu  ").unwrap();
    assert!(workspace.is_editable());
    assert_eq!(workspace.name(), "Harjoituskilpailu");
    assert_eq!(
        workspace.path(),
        fs::canonicalize(directory.file()).unwrap()
    );
    assert_eq!(
        Workspace::open(workspace.path()).unwrap().name(),
        workspace.name()
    );
    let original = fs::read(directory.file()).unwrap();
    assert!(Workspace::create(&directory.0, "Korvaava").is_err());
    assert_eq!(fs::read(directory.file()).unwrap(), original);
}

#[test]
fn individual_driver_edits_autosave_and_preserve_nonvisible_fields() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    let before = workspace.project().unwrap().clone();
    workspace
        .edit_driver(7, DriverField::Driver, "Korjattu nimi")
        .unwrap();
    let mut expected = before;
    expected.kilpailijat[0].kuljettaja = "Korjattu nimi".into();
    assert_eq!(workspace.project().unwrap(), &expected);
    assert_eq!(
        ProjectStore::open(&directory.file()).unwrap().project(),
        &expected
    );
    assert_eq!(
        workspace.archive().unwrap().competitors[0].driver,
        "Korjattu nimi"
    );
    assert_eq!(
        workspace.calculation().unwrap().competitors[0].driver,
        "Korjattu nimi"
    );
    assert_eq!(
        workspace.results().unwrap().competitors[0].driver,
        "Korjattu nimi"
    );
    assert!(!workspace.ranking().unwrap().groups.is_empty());
}

#[test]
fn invalid_or_incomplete_edit_cannot_replace_the_saved_row() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    let original = fs::read(directory.file()).unwrap();
    let before = workspace.project().unwrap().clone();
    assert!(workspace.edit_driver(7, DriverField::Card, "").is_err());
    assert!(workspace.edit_driver(7, DriverField::Start, "12:").is_err());
    assert!(
        workspace
            .edit_driver(7, DriverField::Class, "PUUTTUU")
            .is_err()
    );
    assert!(workspace.edit_driver(99, DriverField::Car, "Auto").is_err());
    assert!(workspace.add_driver(driver("7", "42")).is_err());
    assert_eq!(workspace.project().unwrap(), &before);
    assert_eq!(fs::read(directory.file()).unwrap(), original);
}

#[test]
fn external_conflict_requires_explicit_reload_and_never_loses_other_changes() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    let mut other = Workspace::open(&directory.0).unwrap();
    other
        .edit_driver(7, DriverField::Car, "Toisen ikkunan auto")
        .unwrap();
    let error = workspace
        .edit_driver(7, DriverField::Driver, "Oma muutos")
        .unwrap_err();
    assert!(error.to_string().contains("Avaa kilpailu uudelleen"));
    assert_eq!(workspace.project().unwrap().kilpailijat[0].auto, "");
    workspace.reload().unwrap();
    workspace
        .edit_driver(7, DriverField::Driver, "Oma muutos")
        .unwrap();
    let saved = ProjectStore::open(&directory.file()).unwrap();
    assert_eq!(saved.project().kilpailijat[0].auto, "Toisen ikkunan auto");
    assert_eq!(saved.project().kilpailijat[0].kuljettaja, "Oma muutos");
}

#[test]
fn failed_save_and_failed_reload_keep_last_successful_state() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    let original = fs::read(directory.file()).unwrap();
    fs::create_dir(directory.0.join("kilpailu.toml.bak")).unwrap();
    assert!(
        workspace
            .edit_competition(CompetitionField::Name, "Ei tallentunut")
            .is_err()
    );
    assert_eq!(workspace.name(), "Esimerkkikilpailu");
    assert_eq!(fs::read(directory.file()).unwrap(), original);
    fs::write(directory.file(), "keskeneräinen ulkoinen tiedosto").unwrap();
    assert!(workspace.reload().is_err());
    assert_eq!(workspace.name(), "Esimerkkikilpailu");
}

#[test]
fn renamed_class_and_track_keep_references_and_calculation_in_sync() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    workspace.edit_class("A", ClassField::Code, "H").unwrap();
    workspace.edit_track(1, TrackField::Number, "3").unwrap();
    let project = workspace.project().unwrap();
    assert_eq!(project.kilpailijat[0].luokka, "H");
    assert_eq!(project.luokat[0].rata, 3);
    assert_eq!(project.radat[0].numero, 3);
    assert!(
        workspace.calculation().unwrap().competitors[0]
            .calculation
            .is_ok()
    );
    let old_result = workspace.results().unwrap().competitors[0].total_penalties;
    workspace
        .edit_station(3, 1, StationField::Target, "400")
        .unwrap();
    assert_ne!(
        workspace.results().unwrap().competitors[0].total_penalties,
        old_result
    );
    let reopened = Workspace::open(&directory.0).unwrap();
    assert_eq!(workspace.project(), reopened.project());
}

#[test]
fn class_route_change_uses_current_route_and_preserves_class_settings() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    let class = workspace.project().unwrap().luokat[0].clone();
    assert!(
        workspace.calculation().unwrap().competitors[0]
            .calculation
            .is_ok()
    );
    workspace
        .add_track(TrackEdit {
            number: "2".into(),
            name: "Toinen rata".into(),
        })
        .unwrap();
    workspace.edit_class("A", ClassField::Route, "2").unwrap();
    let mut expected = class;
    expected.rata = 2;
    assert_eq!(workspace.project().unwrap().luokat[0], expected);
    // Kortin leimaukset eivät kuulu uudelle lähtö–maali-radalle. Vanhaa,
    // onnistunutta laskentaa ei saa näyttää uuden radan tuloksena.
    assert!(
        workspace.calculation().unwrap().competitors[0]
            .calculation
            .is_err()
    );
    assert_eq!(workspace.results().unwrap().competitors[0].status, "ILM");
    assert_eq!(
        workspace.results().unwrap().competitors[0].total_penalties,
        0
    );
    assert_eq!(workspace.results().unwrap().routes.len(), 2);
}

#[test]
fn complete_new_rows_and_station_order_are_saved_together() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    workspace
        .add_track(TrackEdit {
            number: "2".into(),
            name: "Uusi rata".into(),
        })
        .unwrap();
    workspace
        .add_class(ClassEdit {
            code: "B".into(),
            name: "Toinen luokka".into(),
            route: "2".into(),
            first_number: "10".into(),
            start_time: "13:00:00".into(),
            interval_minutes: "2".into(),
            lateness_minutes: "60".into(),
        })
        .unwrap();
    let mut entry = driver("10", "42");
    entry.class_code = "B".into();
    workspace.add_driver(entry).unwrap();
    let added_index = workspace
        .add_station(
            2,
            StationEdit {
                kind: "AT".into(),
                name: "AT 47".into(),
                code: "47".into(),
                target_seconds: "120".into(),
                rule: "0".into(),
                section_minutes: "2".into(),
            },
        )
        .unwrap();
    assert_eq!(added_index, 1);
    workspace
        .add_station(
            2,
            StationEdit {
                kind: "AT".into(),
                name: "AT 48".into(),
                code: "48".into(),
                target_seconds: "180".into(),
                rule: "0".into(),
                section_minutes: "3".into(),
            },
        )
        .unwrap();
    workspace.move_station(2, 2, 1).unwrap();
    let project = workspace.project().unwrap();
    assert_eq!(project.radat[1].asemat[1].nimi, "AT 48");
    assert_eq!(project.radat[1].asemat[2].nimi, "AT 47");
    assert_eq!(project.radat[1].asemat[3].tyyppi, "M");
    assert!(workspace.move_station(2, 99, 0).is_err());
    assert_eq!(
        workspace.project(),
        Workspace::open(&directory.0).unwrap().project()
    );
}

#[test]
fn new_intermediate_station_is_inserted_before_finish_and_returns_its_index() {
    let directory = Directory::new();
    let mut workspace = Workspace::create(&directory.0, "Asemakoe").unwrap();
    let index = workspace
        .add_station(
            1,
            StationEdit {
                kind: "AT".into(),
                name: "AT 47".into(),
                code: "47".into(),
                target_seconds: "120".into(),
                rule: "0".into(),
                section_minutes: "2".into(),
            },
        )
        .unwrap();
    assert_eq!(index, 1);
    let stations = &workspace.project().unwrap().radat[0].asemat;
    assert_eq!(
        stations
            .iter()
            .map(|station| station.tyyppi.as_str())
            .collect::<Vec<_>>(),
        ["L", "AT", "M"]
    );
    assert_eq!(stations[index].nimi, "AT 47");
    assert_eq!(
        workspace.project(),
        Workspace::open(&directory.0).unwrap().project()
    );
}

#[test]
fn invalid_ula_rule_is_rejected_without_writing_and_type_can_be_changed_safely() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    let original = fs::read(directory.file()).unwrap();
    let error = workspace
        .edit_station(1, 1, StationField::Rule, "5")
        .unwrap_err();
    assert!(error.to_string().contains("vain ULA-asemalle"));
    assert!(
        workspace
            .add_station(
                1,
                StationEdit {
                    kind: "AT".into(),
                    name: "Virheellinen".into(),
                    code: "47".into(),
                    target_seconds: "120".into(),
                    rule: "5".into(),
                    section_minutes: "2".into(),
                }
            )
            .is_err()
    );
    assert_eq!(fs::read(directory.file()).unwrap(), original);
    workspace
        .edit_station(1, 1, StationField::Kind, "ULA")
        .unwrap();
    workspace
        .edit_station(1, 1, StationField::Rule, "5")
        .unwrap();
    let valid_ula = fs::read(directory.file()).unwrap();
    let error = workspace
        .edit_station(1, 1, StationField::Kind, "AT")
        .unwrap_err();
    assert!(error.to_string().contains("ensin normaaliksi"));
    assert_eq!(fs::read(directory.file()).unwrap(), valid_ula);
    assert_eq!(
        workspace.project().unwrap().radat[0].asemat[1].tyyppi,
        "ULA"
    );
    workspace
        .edit_station(1, 1, StationField::Rule, "0")
        .unwrap();
    workspace
        .edit_station(1, 1, StationField::Kind, "AT")
        .unwrap();
    assert_eq!(workspace.project().unwrap().radat[0].asemat[1].saanto, 0);
    assert_eq!(workspace.project().unwrap().radat[0].asemat[1].tyyppi, "AT");
    assert_eq!(
        workspace.project(),
        Workspace::open(&directory.0).unwrap().project()
    );
}

#[test]
fn existing_station_rule_combination_still_opens_and_can_be_repaired() {
    let directory = Directory::new();
    let mut project =
        native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    project.radat[0].asemat[1].saanto = 5;
    ProjectStore::create(&directory.file(), project).unwrap();
    let mut workspace = Workspace::open(&directory.0).unwrap();
    workspace
        .edit_driver(7, DriverField::Driver, "Nimen korjaus")
        .unwrap();
    workspace
        .edit_station(1, 1, StationField::Name, "Korjattava AT")
        .unwrap();
    assert_eq!(workspace.project().unwrap().radat[0].asemat[1].saanto, 5);
    workspace
        .edit_station(1, 1, StationField::Rule, "0")
        .unwrap();
    assert!(
        workspace.calculation().unwrap().competitors[0]
            .calculation
            .is_ok()
    );
}

#[test]
fn settings_patch_does_not_clear_unrelated_overrides() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    workspace
        .edit_override(OverrideField::MissingRt, "123")
        .unwrap();
    workspace
        .edit_override(OverrideField::FreeTime, "20")
        .unwrap();
    workspace
        .edit_competition(CompetitionField::Number, "23")
        .unwrap();
    workspace
        .edit_competition(CompetitionField::Timing, "0")
        .unwrap();
    workspace
        .edit_override(OverrideField::FreeTime, "")
        .unwrap();
    assert_eq!(
        workspace.project().unwrap().virhepisteet.puuttuva_rt,
        Some(123)
    );
    assert_eq!(
        workspace
            .project()
            .unwrap()
            .virhepisteet
            .vapaa_aika_sekunteina,
        None
    );
    assert_eq!(workspace.project().unwrap().kilpailu.numero, 23);
    assert_eq!(workspace.project().unwrap().kilpailu.ajanottotapa, 0);
    assert!(
        workspace
            .edit_competition(CompetitionField::Timing, "2")
            .is_err()
    );
    assert_eq!(
        workspace.project(),
        Workspace::open(&directory.0).unwrap().project()
    );
}

#[test]
fn frames_autosave_without_duplicate_results_and_reject_damaged_read() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    let frame = emit::parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw")).unwrap();
    let clock = ClockTime::new(12, 11, 0).unwrap();
    let first = workspace.record_frame(&frame, clock).unwrap();
    assert_eq!(first.competitor_number, Some(7));
    let mut reread = frame.clone();
    reread.records[4].elapsed_seconds += 1;
    let repeated = workspace.record_frame(&reread, clock).unwrap();
    assert!(!repeated.content_changed);
    assert_eq!(workspace.results().unwrap().competitors.len(), 1);
    assert_eq!(workspace.project().unwrap().leimaluvut.len(), 3);
    let original = fs::read(directory.file()).unwrap();
    let mut damaged = frame.clone();
    damaged.records[0].code = 250;
    assert!(workspace.record_frame(&damaged, clock).is_err());
    assert_eq!(fs::read(directory.file()).unwrap(), original);
    let mut unknown = frame;
    unknown.card_number = 42;
    assert_eq!(
        workspace
            .record_frame(&unknown, clock)
            .unwrap()
            .competitor_number,
        None
    );
    workspace.add_driver(driver("10", "42")).unwrap();
    assert!(
        workspace.calculation().unwrap().competitors[1]
            .calculation
            .is_ok()
    );
    assert_eq!(
        workspace.project(),
        Workspace::open(&directory.0).unwrap().project()
    );
}

#[test]
fn legacy_is_read_only_and_keeps_imported_results_separate() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/synthetic");
    let directory = Directory::new();
    for name in [
        "Aspara.txt",
        "Asperus.txt",
        "Kilp.txt",
        "Radat.txt",
        "emit.txt",
    ] {
        fs::copy(source.join(name), directory.0.join(name)).unwrap();
    }
    let before = fs::read(directory.0.join("Kilp.txt")).unwrap();
    let mut workspace = Workspace::open(&directory.0).unwrap();
    assert!(!workspace.is_editable());
    assert!(workspace.project().is_none());
    assert_eq!(
        workspace.archive().unwrap().competitors[0].total_penalties,
        workspace.results().unwrap().competitors[0].total_penalties
    );
    assert!(
        workspace
            .edit_driver(7, DriverField::Driver, "Muutos")
            .is_err()
    );
    assert!(
        workspace
            .edit_competition(CompetitionField::Name, "Muutos")
            .is_err()
    );
    assert!(workspace.add_driver(driver("10", "42")).is_err());
    assert_eq!(fs::read(directory.0.join("Kilp.txt")).unwrap(), before);
    assert!(!directory.file().exists());
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 5);
}

#[test]
fn rt_counts_and_other_penalties_autosave_and_survive_normal_driver_edits() {
    let directory = Directory::new();
    let mut workspace = directory.example();
    workspace
        .edit_driver(7, DriverField::MissingRtCount, "2")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::OtherPenalties, "100")
        .unwrap();
    workspace
        .edit_driver(7, DriverField::Driver, "Uusi kuljettaja")
        .unwrap();
    let driver = &workspace.project().unwrap().kilpailijat[0];
    assert_eq!(driver.lisavirheet.puuttuvat_rt_kpl, 2);
    assert_eq!(driver.lisavirheet.puuttuvat_rt_pisteet, 1200);
    assert_eq!(driver.lisavirheet.muut_virhepisteet, 100);
    assert_eq!(
        workspace.results().unwrap().competitors[0].total_penalties,
        1945
    );
    let reopened = Workspace::open(&directory.file()).unwrap();
    assert_eq!(
        reopened.project().unwrap().kilpailijat[0].lisavirheet,
        driver.lisavirheet
    );
    assert_eq!(
        reopened.results().unwrap().competitors[0].total_penalties,
        1945
    );
}
