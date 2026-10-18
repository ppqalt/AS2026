//! Valmistelutietoja ei muuteta ajanotoksi. Aineisto on synteettistä.
use as2026::{application, excel_import, native::Project, ranking::TiePolicy, report, starts};
use calamine::{Data, ExcelDateTime, ExcelDateTimeType, Range, Reader, open_workbook_auto};
use std::path::Path;

fn sheets() -> (Range<Data>, Range<Data>) {
    let mut workbook = open_workbook_auto(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/excel-import/synteettinen.xlsx"),
    )
    .unwrap();
    (
        workbook.worksheet_range("RATA").unwrap(),
        workbook.worksheet_range("KILP").unwrap(),
    )
}

#[test]
fn drafts_roundtrip_without_card_or_clock_and_do_not_publish_results() {
    let (track, mut drivers) = sheets();
    for row in 1..=2 {
        for col in [2, 3, 4] {
            drivers.set_value((row, col), Data::Empty);
        }
    }
    let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert!(preview.can_import(), "{}", preview.issue_details());
    let mut project = preview.to_project("Valmisteltava kilpailu").unwrap();
    assert_eq!(project.kilpailijat.len(), 2);
    assert!(
        project
            .kilpailijat
            .iter()
            .all(|d| d.emit_number().is_none() && d.start_clock().unwrap().is_none())
    );
    let toml = project.to_toml().unwrap();
    // Julkaistun 0.6.2:n lukija edellyttää nämä kentät myös keskeneräiselle riville.
    assert!(toml.contains("emit = 0"));
    assert!(toml.contains("lahtoaika = \"\""));
    assert_eq!(Project::from_toml(&toml).unwrap(), project);
    assert!(application::capture_project_result(&project, 1).is_err());
    let archive = application::calculated_project(&project).unwrap();
    assert!(
        archive
            .warnings
            .iter()
            .all(|warning| !warning.contains("tulos puuttuu"))
    );
    assert!(
        archive
            .competitors
            .iter()
            .all(|c| c.header_fields[10].is_empty() && c.emit_card.is_empty())
    );
    let ranking = application::rank_stored(&archive, TiePolicy::KeepSharedAndFlag).unwrap();
    assert!(ranking.groups.iter().all(|g| g.entries.is_empty()));
    let html = report::ranking_html(&archive, &ranking);
    assert!(!html.contains(&project.kilpailijat[0].kuljettaja));
    let list = starts::start_list_csv(&project).unwrap();
    assert!(list.contains(&project.kilpailijat[0].kuljettaja));
    assert!(!list.contains("00:00:00"));
    project.luokat[0].osallistuu_lahtoarvontaan = true;
    project.luokat[0].lahtoaika = "10:00:00".into();
    let draw = starts::draw(&project, 42).unwrap();
    assert_eq!(draw.assignments.len(), 1);
}

#[test]
fn unused_route_block_is_ignored_but_unlabelled_data_blocks_import() {
    let (mut tracks, drivers) = sheets();
    tracks.set_value((0, 10), Data::String("Luokka".into()));
    let preview = excel_import::preview_sheets(&tracks, &drivers).unwrap();
    assert!(preview.can_import(), "{}", preview.issue_details());
    assert_eq!(preview.radat.len(), 2);
    tracks.set_value((3, 11), Data::String("Lähtö".into()));
    let preview = excel_import::preview_sheets(&tracks, &drivers).unwrap();
    assert!(!preview.can_import());
    assert!(preview.huomautukset.iter().any(|i| i.estaa_tuonnin));
}

#[test]
fn tracks_only_import_ignores_competitor_conflicts_but_keeps_route_validation() {
    let (mut tracks, mut drivers) = sheets();
    drivers.set_value((2, 0), drivers.get_value((1, 0)).unwrap().clone());
    let preview = excel_import::preview_sheets(&tracks, &drivers).unwrap();
    assert!(!preview.can_import());
    assert!(preview.has_importable_tracks());
    let project = preview.tracks_only().to_project("Vain radat").unwrap();
    assert!(project.kilpailijat.is_empty());
    assert_eq!(project.radat.len(), 2);
    tracks.set_value((0, 6), Data::String("C".into()));
    let preview = excel_import::preview_sheets(&tracks, &drivers).unwrap();
    assert!(!preview.has_importable_tracks());
    assert!(preview.tracks_only().to_project("Ei kaksoisrataa").is_err());
}

#[test]
fn excel_clock_is_not_misread_as_separate_hour_and_conflicts_are_reported() {
    let (tracks, mut drivers) = sheets();
    drivers.set_value(
        (1, 3),
        Data::DateTime(ExcelDateTime::new(0.5, ExcelDateTimeType::DateTime, false)),
    );
    drivers.set_value((1, 4), Data::Int(0));
    let preview = excel_import::preview_sheets(&tracks, &drivers).unwrap();
    assert_eq!(preview.kilpailijat[0].tiedot.lahtoaika, "12:00:00");
    drivers.set_value((1, 4), Data::Int(15));
    let preview = excel_import::preview_sheets(&tracks, &drivers).unwrap();
    assert_eq!(preview.kilpailijat.len(), 2);
    assert!(preview.kilpailijat[0].tiedot.lahtoaika.is_empty());
    assert!(preview.issue_details().contains("ristiriidassa"));
}

#[test]
fn missing_legacy_start_hour_does_not_break_other_results_or_become_midnight() {
    let project =
        Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    let mut archive = project.to_archive().unwrap();
    archive.competitors[0].header_fields[10].clear();
    application::rank_stored(&archive, TiePolicy::KeepSharedAndFlag).unwrap();
    assert!(
        application::calculate(&archive).unwrap().competitors[0]
            .calculation
            .is_err()
    );
}

#[test]
#[ignore = "Vain paikallinen käyttäjän Excel; henkilötietoja ei kopioida testiaineistoon"]
fn local_preparation_workbook_retains_all_named_rows_without_guessing_routes() {
    let path = std::env::var_os("AS2026_EXCEL_VALMISTELU").expect("Anna paikallisen Excelin polku");
    let before = std::fs::read(&path).unwrap();
    let preview = excel_import::read(Path::new(&path)).unwrap();
    assert!(preview.has_importable_data(), "{}", preview.issue_details());
    assert_eq!(preview.radat.len(), 3);
    assert_eq!(preview.kilpailijat.len(), 70);
    assert_eq!(
        preview
            .kilpailijat
            .iter()
            .filter(|d| d.tiedot.emit_number().is_none())
            .count(),
        43
    );
    assert!(
        preview
            .kilpailijat
            .iter()
            .all(|d| d.tiedot.start_clock().unwrap().is_none())
    );
    assert!(preview.huomautukset.is_empty());
    assert!(!preview.can_confirm_import());
    assert!(!preview.unresolved_classes().is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    println!(
        "Paikallinen Excel: 70 kilpailijaparia, 3 rataa, 89 asemaa; 43 Emit-numeroa ja 70 lähtöaikaa täydennettävä. Ratavalintoja ei arvattu, alkuperäinen säilyi tavulleen."
    );
}
