//! Julkaisuraja on erillinen laskentakelpoisuudesta. Kaikki aineisto on synteettistä.
use as2026::{
    application, legacy, native,
    ranking::{self, RankingReport, Status},
    report,
    workspace::{
        ClassField, CompetitionField, DriverField, OverrideField, ResultField, StationField,
        Workspace,
    },
};
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

const DRIVER: &str = "Julkaisuportin mallikuljettaja";
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "as2026-julkaisuraja-{}-{}",
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
        let mut project =
            native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
                .unwrap();
        project.kilpailijat[0].kuljettaja = DRIVER.into();
        project.luokat[0].nousupisteet = 1;
        project.luokat[0].sarjapistetapa = 1;
        project.yhdistetyt_luokat.push(native::CombinedClass {
            nimi: "Yhdistetty koe".into(),
            luokat: vec!["A".into()],
            nousupisteet: 1,
            sarjapistetapa: 1,
        });
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

fn ready(workspace: &mut Workspace) {
    workspace.ensure_readout_result(7).unwrap();
    let finish = workspace
        .result_snapshot(7)
        .unwrap()
        .unwrap()
        .rivit
        .iter()
        .find(|row| row.station_type == "M")
        .unwrap()
        .output_index;
    workspace
        .edit_station_result(7, finish, ResultField::Clock, "12:11:00")
        .unwrap();
    workspace.recalculate_stored(7).unwrap();
    let snapshot = workspace.result_snapshot(7).unwrap().unwrap();
    assert!(snapshot.can_approve());
    assert!(!snapshot.hyvaksytty);
}
fn xml_from_xlsx(bytes: Vec<u8>) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut sheet = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet)
        .unwrap();
    sheet
}
fn assert_unranked(report: &RankingReport) {
    assert!(
        report
            .groups
            .iter()
            .all(|group| group.entries.iter().all(|entry| entry.number != 7))
    );
    assert!(
        report
            .routes
            .iter()
            .all(|route| route.entries.iter().all(|entry| entry.number != 7))
    );
}
fn assert_export_omits_draft(archive: &legacy::Archive, ranking: &RankingReport) {
    let html = report::project_ranking_html(archive, ranking);
    let generic_html = report::ranking_html(archive, ranking);
    let csv = report::ranking_csv(archive, ranking);
    let sheet = xml_from_xlsx(report::ranking_xlsx(archive, ranking).unwrap());
    assert!(!html.contains(&format!("<td>{DRIVER}</td>")), "{html}");
    assert!(
        !generic_html.contains(&format!("<td>{DRIVER}</td>")),
        "{generic_html}"
    );
    assert!(!csv.contains(&format!("\"{DRIVER}\"")), "{csv}");
    assert!(!sheet.contains(DRIVER), "{sheet}");
}

#[test]
fn calculation_ready_draft_has_no_official_position_points_or_exported_result() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    ready(&mut workspace);
    let preview = workspace.results().unwrap();
    assert_eq!(preview.competitors[0].status, "TRK");
    assert!(preview.competitors[0].total_penalties > 0);
    assert!(preview.result_publication.is_some());
    let ranking = workspace.ranking().unwrap();
    assert_unranked(&ranking);
    assert!(ranking.groups.iter().all(|group| group.provisional));
    assert_export_omits_draft(&preview, &ranking);
    let detailed = report::project_results_html(workspace.project().unwrap()).unwrap();
    assert!(detailed.contains("Hyväksymätön esikatselu"));
    assert!(!detailed.contains("Hyväksytty tulos."));
    assert!(!detailed.contains(&format!(
        "Kokonaistulos: {} p",
        preview.competitors[0].total_penalties
    )));

    workspace.approve_result(7).unwrap();
    let approved = workspace.results().unwrap();
    let ranking = workspace.ranking().unwrap();
    for group in &ranking.groups {
        let winner = group
            .entries
            .iter()
            .find(|entry| entry.number == 7)
            .unwrap();
        assert_eq!(winner.status, Status::Classified);
        assert_eq!(winner.primary_position, 1);
        assert_eq!(winner.series_points, 100);
        assert!(!group.provisional);
    }
    assert!(report::ranking_csv(&approved, &ranking).contains(DRIVER));
    assert!(xml_from_xlsx(report::ranking_xlsx(&approved, &ranking).unwrap()).contains(DRIVER));
    assert!(
        report::project_results_html(workspace.project().unwrap())
            .unwrap()
            .contains("Hyväksytty tulos.")
    );
}

#[test]
fn stale_approved_ranking_report_cannot_bypass_archive_publication_gate() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    ready(&mut workspace);
    workspace.approve_result(7).unwrap();
    let previously_approved_ranking = workspace.ranking().unwrap();
    workspace
        .edit_driver(7, DriverField::OtherPenalties, "41")
        .unwrap();
    let draft_archive = workspace.results().unwrap();
    assert!(!workspace.result_snapshot(7).unwrap().unwrap().hyvaksytty);
    assert_unranked(&workspace.ranking().unwrap());
    assert_export_omits_draft(&draft_archive, &previously_approved_ranking);

    let mut missing_metadata_entry = draft_archive;
    missing_metadata_entry
        .result_publication
        .as_mut()
        .unwrap()
        .clear();
    assert_unranked(
        &application::rank_stored(
            &missing_metadata_entry,
            ranking::TiePolicy::BestRowsWithExplicitCounts,
        )
        .unwrap(),
    );
    assert_export_omits_draft(&missing_metadata_entry, &previously_approved_ranking);
}

#[test]
fn tampered_ranking_identity_cannot_replace_an_approved_competitor() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    ready(&mut workspace);
    workspace.approve_result(7).unwrap();
    let archive = workspace.results().unwrap();
    let mut wrong_identity = workspace.ranking().unwrap();
    for group in &mut wrong_identity.groups {
        for entry in &mut group.entries {
            entry.number = 999;
        }
    }
    for route in &mut wrong_identity.routes {
        for entry in &mut route.entries {
            entry.number = 999;
        }
    }
    // The exporter deliberately recalculates the ranking from the approved
    // archive instead of trusting the caller's stale/tampered entry numbers.
    // A valid approved result must remain visible, never silently disappear.
    let csv = report::ranking_csv(&archive, &wrong_identity);
    let html = report::project_ranking_html(&archive, &wrong_identity);
    let xlsx = xml_from_xlsx(report::ranking_xlsx(&archive, &wrong_identity).unwrap());
    assert!(csv.contains(&format!("\"{DRIVER}\"")));
    assert!(html.contains(&format!("<td>{DRIVER}</td>")));
    assert!(xlsx.contains(DRIVER));
    assert!(!csv.contains(";\"999\";"));
    assert!(!html.contains("<td>999</td>"));
}

#[test]
fn cli_html_csv_and_xlsx_apply_the_same_gate_with_explicit_tie_resolution() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    ready(&mut workspace);
    for (phase, approved) in [("luonnos", false), ("hyvaksytty", true)] {
        if approved {
            workspace.approve_result(7).unwrap();
        }
        let html = directory.0.join(format!("{phase}.html"));
        let csv = directory.0.join(format!("{phase}.csv"));
        let xlsx = directory.0.join(format!("{phase}.xlsx"));
        let output = Command::new(env!("CARGO_BIN_EXE_as2026"))
            .arg("tulokset")
            .arg(&directory.0)
            .arg("--raportti")
            .arg(&html)
            .arg("--csv")
            .arg(&csv)
            .arg("--xlsx")
            .arg(&xlsx)
            .arg("--ratkaise-tasatulokset")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read_to_string(&csv).unwrap().contains(DRIVER), approved);
        assert_eq!(
            fs::read_to_string(&html)
                .unwrap()
                .contains(&format!("<td>{DRIVER}</td>")),
            approved
        );
        assert_eq!(
            xml_from_xlsx(fs::read(&xlsx).unwrap()).contains(DRIVER),
            approved
        );
        if !approved {
            let preview = directory.0.join("laskennan-esikatselu.html");
            let output = Command::new(env!("CARGO_BIN_EXE_as2026"))
                .arg("laske")
                .arg(&directory.0)
                .arg("--raportti")
                .arg(&preview)
                .output()
                .unwrap();
            assert!(output.status.success());
            let html = fs::read_to_string(preview).unwrap();
            assert!(html.contains("Hyväksymätön esikatselu"));
            assert!(!html.contains("Hyväksytty tulos."));
        }
    }
}

#[test]
fn confirmed_interruption_requires_reason_but_no_card_or_invented_finish_and_survives_reopen() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(false);
    let before = fs::read(directory.file()).unwrap();
    for reason in ["", "   ", "Syy\nuusi rivi", "\t"] {
        assert!(workspace.confirm_interruption(7, reason).is_err());
        assert_eq!(fs::read(directory.file()).unwrap(), before);
    }
    workspace
        .confirm_interruption(7, "  Moottoririkko Ää  ")
        .unwrap();
    assert!(workspace.interruption_confirmed(7));
    assert!(workspace.project().unwrap().leimaluvut.is_empty());
    assert!(
        workspace.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .is_none()
    );
    let archive = workspace.results().unwrap();
    assert_eq!(archive.competitors[0].status, "KESK");
    assert_eq!(archive.competitors[0].header_fields[29], "Moottoririkko Ää");
    assert!(archive.competitors[0].results.is_empty());
    let ranking = workspace.ranking().unwrap();
    assert!(ranking.groups.iter().all(|group| {
        group
            .entries
            .iter()
            .any(|entry| entry.number == 7 && entry.status == Status::Interrupted)
    }));
    let csv = report::ranking_csv(&archive, &ranking);
    for row in csv.lines().skip(1) {
        let fields = row
            .split(';')
            .map(|value| value.trim_matches('"'))
            .collect::<Vec<_>>();
        assert_eq!(fields[7], "KESK");
        assert!(
            fields[8..=10]
                .iter()
                .all(|value| value.is_empty() || *value == "—")
        );
        assert_eq!(fields.last().unwrap(), &"Moottoririkko Ää");
    }
    let detailed = report::project_results_html(workspace.project().unwrap()).unwrap();
    assert!(detailed.contains("Moottoririkko Ää"));
    assert!(!detailed.contains("Maaliaika: 00:00:00"));
    assert!(!detailed.contains("Kokonaistulos: 0 p"));
    let reopened = Workspace::open(&directory.0).unwrap();
    assert!(reopened.interruption_confirmed(7));
    assert_eq!(
        report::ranking_csv(&reopened.results().unwrap(), &reopened.ranking().unwrap()),
        csv
    );
    workspace
        .edit_driver(7, DriverField::StatusReason, "Muu syy")
        .unwrap();
    assert!(!workspace.interruption_confirmed(7));
    assert_unranked(&workspace.ranking().unwrap());
}

#[test]
fn route_and_scoring_configuration_changes_remove_prior_publication_permission() {
    for change in 0..4 {
        let directory = Directory::new();
        let mut workspace = directory.workspace(true);
        ready(&mut workspace);
        workspace.approve_result(7).unwrap();
        match change {
            0 => workspace
                .edit_station(1, 1, StationField::Target, "301")
                .unwrap(),
            1 => workspace
                .edit_competition(CompetitionField::Timing, "2")
                .unwrap(),
            2 => workspace
                .edit_override(OverrideField::FreeTime, "11")
                .unwrap(),
            _ => workspace
                .edit_class("A", ClassField::Lateness, "59")
                .unwrap(),
        }
        let archive = workspace.results().unwrap();
        let ranking = workspace.ranking().unwrap();
        assert_unranked(&ranking);
        assert_export_omits_draft(&archive, &ranking);
        assert!(workspace.approve_result(7).is_err());
    }
}

#[test]
fn changing_emit_card_cannot_reapprove_the_previous_cards_snapshot() {
    let directory = Directory::new();
    let mut workspace = directory.workspace(true);
    ready(&mut workspace);
    workspace.approve_result(7).unwrap();
    workspace
        .edit_driver(7, DriverField::Card, "987654")
        .unwrap();
    assert_unranked(&workspace.ranking().unwrap());
    assert!(workspace.approve_result(7).is_err());
}

#[test]
fn imported_legacy_results_keep_their_explicit_stored_result_contract() {
    let archive = legacy::Archive::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/synthetic"),
        legacy::Encoding::Windows1252,
    )
    .unwrap();
    assert!(archive.result_publication.is_none());
    let ranking =
        application::rank_stored(&archive, ranking::TiePolicy::KeepSharedAndFlag).unwrap();
    assert!(ranking.groups.iter().any(|group| {
        group
            .entries
            .iter()
            .any(|entry| entry.status == Status::Classified)
    }));
    assert!(report::ranking_csv(&archive, &ranking).contains(&archive.competitors[0].driver));
}
