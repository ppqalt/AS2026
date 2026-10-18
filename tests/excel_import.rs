use as2026::{excel_import, native::Project};
use calamine::{Data, ExcelDateTime, ExcelDateTimeType, Range, Reader, open_workbook_auto};
use std::path::{Path, PathBuf};

fn fixture(extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/excel-import/synteettinen.{extension}"))
}
fn sheets() -> (Range<Data>, Range<Data>) {
    let mut workbook = open_workbook_auto(fixture("xlsx")).unwrap();
    (
        workbook.worksheet_range("RATA").unwrap(),
        workbook.worksheet_range("KILP").unwrap(),
    )
}

#[test]
fn both_excel_formats_import_only_the_two_supported_sheets_without_modifying_the_source() {
    let mut previews = Vec::new();
    for extension in ["xls", "xlsx"] {
        let path = fixture(extension);
        let original = std::fs::read(&path).unwrap();
        let preview = excel_import::read(&path).unwrap();
        assert!(preview.can_import(), "{}", preview.summary());
        assert!(preview.huomautukset.is_empty(), "{}", preview.summary());
        assert_eq!(preview.radat.len(), 2);
        assert_eq!(preview.kilpailijat.len(), 2);
        assert_eq!(preview.radat[0].luokka, "C");
        assert_eq!(preview.radat[1].luokka, "B");
        assert_eq!(preview.radat[0].asemat[1].tavoiteaika_sekuntia, 448);
        assert_eq!(preview.radat[0].asemat[1].saanto, 1);
        assert_eq!(preview.radat[0].asemat[1].jakso_minuutteina, Some(15));
        assert_eq!(preview.radat[0].asemat[2].emit_koodi, 0);
        assert_eq!(preview.radat[0].asemat[2].tavoiteaika_sekuntia, 754);
        assert_eq!(preview.radat[0].asemat[3].tavoiteaika_sekuntia, 780);
        assert_eq!(preview.kilpailijat[0].tiedot.lahtoaika, "17:02:00");
        assert_eq!(preview.kilpailijat[1].tiedot.lahtoaika, "18:30:05");
        let project = preview.to_project("Tuotu esimerkkikilpailu").unwrap();
        assert_eq!(project.kilpailijat[0].kuljettajan_seura, "Seura A");
        assert_eq!(project.kilpailijat[0].kartanlukijan_seura, "Seura B");
        assert_eq!(project.kilpailijat[1].kuljettajan_seura, "Seura C");
        assert_eq!(project.kilpailijat[1].auto, "Toinen auto");
        assert_eq!(project.luokat[0].rata, 1);
        assert_eq!(project.luokat[1].rata, 2);
        assert!(
            project
                .luokat
                .iter()
                .all(|class| !class.osallistuu_lahtoarvontaan)
        );
        assert!(project.leimaluvut.is_empty());
        assert!(
            project
                .kilpailijat
                .iter()
                .all(|driver| driver.numero != 999)
        );
        let archive = project.to_archive().unwrap();
        assert_eq!(archive.competitors[0].header_fields[7], "Seura A");
        assert_eq!(archive.competitors[0].header_fields[8], "Seura B");
        assert_eq!(
            Project::from_toml(&project.to_toml().unwrap()).unwrap(),
            project
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        previews.push(preview);
    }
    assert_eq!(previews[0], previews[1]);
}

#[test]
fn incomplete_competitors_are_retained_and_unknown_classes_require_explicit_mapping() {
    let (track, mut drivers) = sheets();
    drivers.set_value((1, 2), Data::Empty);
    let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert!(preview.can_import());
    assert_eq!(preview.kilpailijat.len(), 2);
    assert_eq!(preview.kilpailijat[0].tiedot.emit_number(), None);
    assert!(preview.summary().contains("1 Emit-numeroa"));
    assert_eq!(
        preview
            .to_project("Vain kelvolliset")
            .unwrap()
            .kilpailijat
            .len(),
        2
    );

    drivers.set_value((2, 1), Data::String("Tuntematon".into()));
    let mut preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert_eq!(preview.kilpailijat.len(), 2);
    assert!(!preview.can_import());
    assert_eq!(preview.unresolved_classes(), ["Tuntematon"]);
    assert!(preview.to_project("Ei arvata").is_err());
    preview.set_class_route("Tuntematon", "B").unwrap();
    let project = preview.to_project("Valittu rata").unwrap();
    assert_eq!(
        project
            .luokat
            .iter()
            .find(|c| c.tunnus == "Tuntematon")
            .unwrap()
            .rata,
        2
    );
}

#[test]
fn unknown_station_does_not_create_an_incomplete_or_guessed_route() {
    let (mut track, drivers) = sheets();
    track.set_value((4, 1), Data::String("Tuntematon piste".into()));
    let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert!(!preview.can_import());
    assert_eq!(preview.radat.len(), 1);
    assert_eq!(preview.radat[0].luokka, "B");
    assert_eq!(preview.kilpailijat.len(), 2);
    assert_eq!(preview.unresolved_classes(), ["C"]);
    assert!(
        preview
            .issue_details()
            .contains("Asematyyppiä ei tunnisteta")
    );
    assert!(preview.to_project("Ei arvata rataa").is_err());
}

#[test]
fn duplicate_competitor_or_emit_identifiers_block_confirmation() {
    for column in [0, 2] {
        let (track, mut drivers) = sheets();
        drivers.set_value((2, column), drivers.get_value((1, column)).unwrap().clone());
        let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
        assert!(!preview.can_import());
        assert!(preview.huomautukset.iter().any(|issue| issue.estaa_tuonnin));
        assert!(preview.to_project("Ei tallenneta").is_err());
        assert!(preview.issue_details().contains("esiintyy useasti"));
    }
}

#[test]
fn duplicate_class_codes_and_wrong_headers_are_never_guessed() {
    let (mut track, drivers) = sheets();
    track.set_value((0, 6), Data::String("C".into()));
    let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert!(!preview.can_import());
    let (track, mut drivers) = sheets();
    drivers.set_value((0, 2), Data::String("Muu numero".into()));
    let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert!(!preview.can_import());
    assert!(preview.issue_details().contains("otsikot"));
}

#[test]
fn numeric_minutes_seconds_are_distinct_from_formatted_excel_time_values() {
    let (mut track, mut drivers) = sheets();
    track.set_value((4, 2), Data::Float(0.5));
    drivers.set_value(
        (1, 3),
        Data::DateTime(ExcelDateTime::new(0.5, ExcelDateTimeType::DateTime, false)),
    );
    drivers.set_value((1, 4), Data::Empty);
    let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert_eq!(preview.radat[0].asemat[1].tavoiteaika_sekuntia, 50);
    assert_eq!(preview.kilpailijat[0].tiedot.lahtoaika, "12:00:00");
    track.set_value(
        (4, 2),
        Data::DateTime(ExcelDateTime::new(0.5, ExcelDateTimeType::TimeDelta, false)),
    );
    let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
    assert_eq!(preview.radat[0].asemat[1].tavoiteaika_sekuntia, 43_200);
    for invalid in [7.68, 7.281, -1.0, f64::NAN] {
        track.set_value((4, 2), Data::Float(invalid));
        let preview = excel_import::preview_sheets(&track, &drivers).unwrap();
        assert!(preview.radat.iter().all(|track| track.luokka != "C"));
    }
}

#[test]
fn import_limits_and_file_types_fail_with_finnish_messages() {
    let (_, drivers) = sheets();
    let too_far = Range::<Data>::new((10_000, 0), (10_000, 0));
    assert!(
        excel_import::preview_sheets(&too_far, &drivers)
            .unwrap_err()
            .to_string()
            .contains("tuontirajan")
    );
    assert!(
        excel_import::read(Path::new("ei-tueta.xlsm"))
            .unwrap_err()
            .to_string()
            .contains(".xls ja .xlsx")
    );
    assert!(
        excel_import::read(Path::new("puuttuva.xlsx"))
            .unwrap_err()
            .to_string()
            .contains("ei löydy")
    );
}

#[test]
#[ignore = "Paikallinen alkuperäisen tyhjän mallipohjan koe; tiedostoa ei jaeta projektin mukana"]
fn actual_empty_as2014a_template_is_diagnosed_without_importing_example_or_macro_sheets() {
    let path = std::env::var_os("AS2026_EXCEL_MALLIPOHJA")
        .expect("Anna mallipohjan polku AS2026_EXCEL_MALLIPOHJA-muuttujassa");
    let path = Path::new(&path);
    let before = std::fs::read(path).unwrap();
    let preview = excel_import::read(path).unwrap();
    assert!(preview.radat.is_empty());
    assert!(preview.kilpailijat.is_empty());
    assert!(!preview.can_import());
    assert!(preview.issue_details().contains("rata on tyhjä"));
    assert_eq!(std::fs::read(path).unwrap(), before);
}
