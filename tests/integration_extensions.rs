//! Synthetic regression vectors. AS2015A has not been executed.
use as2026::{
    ClockTime, MatchKind, Punch, RouteStation, Rule, StationKind, application, live_report,
    matching,
    native::Project,
    ranking::TiePolicy,
    report,
    scoring::ScoringParameters,
    timing::{self, CalculationPolicy, LiveCalculationOptions, TimingMethod},
};
use std::io::{Cursor, Read};

fn station(sequence: u32, kind: StationKind, code: u32) -> RouteStation {
    RouteStation {
        sequence,
        kind,
        label: format!("Asema {sequence}"),
        expected_code: code,
        target_seconds: 480,
        rule: Rule::Normal,
        section_time_text: String::new(),
    }
}

#[test]
fn stamped_finish_ignores_later_extras_for_finish_clock_and_keeps_extras() {
    let start = ClockTime::new(12, 0, 0).unwrap();
    let route = [
        station(1, StationKind::Start, 0),
        station(2, StationKind::Finish, 31),
    ];
    let punches = [
        Punch {
            code: 31,
            elapsed_seconds: 600,
        },
        Punch {
            code: 47,
            elapsed_seconds: 650,
        },
    ];
    let rows = matching::match_route(&route, &punches, start).unwrap();
    assert_eq!(rows.last().unwrap().kind, MatchKind::Extra);
    let options = LiveCalculationOptions {
        official_start: start,
        reset_correction_seconds: 120,
        timing_method: TimingMethod::Emit,
        last_punch_elapsed_seconds: 650,
        cumulative_limit_minutes: None,
        parameters: ScoringParameters::default(),
        policy: CalculationPolicy::RecoveredAs2015aStaticV2,
    };
    let before = rows.clone();
    let result = timing::calculate_live_with_trailing_extras(&rows, &options).unwrap();
    assert_eq!(rows, before);
    assert_eq!(result.rows.len(), 3);
    assert_eq!(result.rows[2].received_code, 47);
    assert_eq!(result.rows[2].other_points, 600);
    assert_eq!(result.total_duration_seconds, 480);
    assert_eq!(result.finish_clock_seconds, 12 * 3600 + 480);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "post-finish-extras-as2026-policy")
    );
}

#[test]
fn combined_class_editor_is_atomic_and_renaming_preserves_membership() {
    let directory = std::env::temp_dir().join(format!(
        "as2026-combined-editor-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let mut workspace = as2026::workspace::Workspace::create(&directory, "Kooste").unwrap();
    assert!(workspace.edit_combined_class(0, "Kesken").is_err());
    assert!(workspace.project().unwrap().yhdistetyt_luokat.is_empty());
    workspace
        .edit_combined_class(0, "Yhteinen | H | 1 | 5")
        .unwrap();
    assert_eq!(
        workspace.project().unwrap().yhdistetyt_luokat[0].luokat,
        ["H"]
    );
    workspace
        .edit_class("H", as2026::workspace::ClassField::Code, "Z")
        .unwrap();
    assert_eq!(
        workspace.project().unwrap().yhdistetyt_luokat[0].luokat,
        ["Z"]
    );
    assert_eq!(
        workspace.project(),
        as2026::workspace::Workspace::open(&directory)
            .unwrap()
            .project()
    );
    workspace.edit_combined_class(0, "").unwrap();
    assert!(workspace.project().unwrap().yhdistetyt_luokat.is_empty());
    drop(workspace);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn xlsx_contains_real_zip_parts_and_xml_escaped_inline_text() {
    let mut project =
        Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    project.kilpailijat[0].kuljettaja = "=X&<Y>".into();
    let mut result = application::capture_project_result(&project, 7).unwrap();
    let finish = result
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    result.set_clock(finish, [12, 11, 0]).unwrap();
    let mut result = application::recalculate_stored_result(&project, 7, &result).unwrap();
    assert!(result.can_approve());
    result.hyvaksytty = true;
    project.kilpailijat[0].tallennettu_tulos = Some(result);
    let archive = application::calculated_project(&project).unwrap();
    let ranked = application::rank_stored(&archive, TiePolicy::KeepSharedAndFlag).unwrap();
    let bytes = report::ranking_xlsx(&archive, &ranked).unwrap();
    assert!(bytes.starts_with(b"PK"));
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut sheet = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet)
        .unwrap();
    assert!(sheet.contains("inlineStr"));
    assert!(sheet.contains("&amp;&lt;Y&gt;"));
    assert!(!sheet.contains("<f>"));
    let mut workbook = String::new();
    zip.by_name("xl/workbook.xml")
        .unwrap()
        .read_to_string(&mut workbook)
        .unwrap();
    assert!(workbook.contains("Tulokset"));
}

#[test]
fn live_html_refreshes_without_embedded_scripts() {
    let html = "<!doctype html><html><head><title>Tulokset</title></head><body>x</body></html>";
    let refreshed = live_report::refreshed_html(html);
    assert!(refreshed.contains("http-equiv=\"refresh\" content=\"5\""));
    assert!(!refreshed.contains("<script"));
}
