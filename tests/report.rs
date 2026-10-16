use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use as2026::{
    CoreError, MatchKind, MatchedRow, RouteStation, Rule, StationKind,
    application::{CalculationReport, CompetitorPreview},
    legacy::{Archive, BasicSettings, Competitor},
    ranking::{
        self, ClassDefinition, CompetitorInput, Diagnostic, DiagnosticKind, RankingInput, Status,
    },
    report::{calculation_html, ranking_html, save_new, user_notice},
    scoring::PenaltyTotals,
    timing::{
        CalculatedRow, CalculationDiagnostic, CalculationPolicy, CalculationStatus, LiveCalculation,
    },
};

const IMPORTED: &str = "<script>alert(\"x\")</script> & 'Ää'";

fn archive() -> Archive {
    Archive {
        documents: BTreeMap::new(),
        parameters: Vec::new(),
        basic: BasicSettings {
            identity: vec!["".into(), IMPORTED.into()],
            ..Default::default()
        },
        routes: Vec::new(),
        competitors: vec![Competitor {
            number: 7,
            class_code: IMPORTED.into(),
            emit_card: "123".into(),
            driver: IMPORTED.into(),
            navigator: IMPORTED.into(),
            header_fields: Vec::new(),
            total_penalties: 23,
            declared_row_count: 0,
            status: "TRK".into(),
            correction_seconds: 0,
            results: Vec::new(),
        }],
        warnings: vec![IMPORTED.into()],
    }
}

fn ranked(archive: &Archive) -> ranking::RankingReport {
    let mut report = ranking::rank(&RankingInput {
        classes: vec![ClassDefinition {
            code: IMPORTED.into(),
            np_enabled: true,
            series_method: 1,
        }],
        competitors: vec![CompetitorInput {
            source_index: 1,
            number: 7,
            class_code: IMPORTED.into(),
            route_code: IMPORTED.into(),
            status: Status::Classified,
            total_penalty: archive.competitors[0].total_penalties,
            start_hour: 10,
            driver_licence: "123".into(),
            navigator_licence: "456".into(),
            tie_rows: None,
            route_errors: None,
        }],
        ..Default::default()
    })
    .unwrap();
    report.diagnostics.push(Diagnostic {
        group: Some(IMPORTED.into()),
        source_indices: vec![1],
        kind: DiagnosticKind::UnknownStatus {
            status: IMPORTED.into(),
        },
    });
    report
}

fn safe_html(html: &str) {
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<html lang=\"fi\">"));
    assert!(html.contains("<meta charset=\"utf-8\">"));
    assert!(html.contains("<p class=\"brand\">AS2026</p>"));
    assert!(html.contains("@media print"));
    assert!(html.contains("@media(max-width:600px)"));
    assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; &#39;Ää&#39;"));
    assert!(!html.contains("<script"));
    assert!(!html.contains("src=\"http"));
    assert!(!html.contains("href=\"http"));
    assert!(html.ends_with("</main></body></html>\n"));
}

#[test]
fn ranking_html_escapes_imported_identity_names_route_and_diagnostics() {
    let archive = archive();
    let report = ranked(&archive);
    let html = ranking_html(&archive, &report);
    safe_html(&html);
    for heading in [
        "Luokkakohtaiset tulokset",
        "Ratakohtaiset tulokset",
        "Sij.",
        "Nro",
        "Kuljettaja",
        "Kartanlukija",
        "Luokka",
        "Tulos",
        "Np",
        "Sp",
    ] {
        assert!(html.contains(heading), "{heading}");
    }
    assert!(html.contains("tallennettuihin tuloksiin"));
    assert!(html.contains("23 p"));
    assert!(html.contains("Tuntematon tarkastustulos"));
    assert!(!html.contains("UnknownStatus"));
    assert!(!html.contains("Classified"));
}

#[test]
fn calculation_html_escapes_rows_errors_reasons_and_notes_and_keeps_totals_separate() {
    let station = RouteStation {
        sequence: 1,
        kind: StationKind::At,
        label: IMPORTED.into(),
        expected_code: 31,
        target_seconds: 60,
        rule: Rule::Normal,
        section_time_text: String::new(),
    };
    let row = CalculatedRow {
        source: MatchedRow {
            output_index: 1,
            station: Some(station),
            punch: None,
            kind: MatchKind::Matched,
            clock: None,
        },
        received_code: 31,
        effective_rule: Rule::Normal,
        derived_elapsed_seconds: 70,
        clock_seconds: None,
        target_seconds: 60,
        actual_seconds: 70,
        signed_time_points: -7,
        other_points: 2,
        total_points: 9,
        note: IMPORTED.into(),
    };
    let calculation = LiveCalculation {
        policy: CalculationPolicy::RecoveredLiveV1,
        rows: vec![row],
        totals: PenaltyTotals {
            time_points: 7,
            other_points: 2,
            total_points: 9,
        },
        status: CalculationStatus::Interrupted,
        reason: IMPORTED.into(),
        finish_clock_seconds: 0,
        total_duration_seconds: 0,
        cumulative_schedule_difference_seconds: 0,
        diagnostics: vec![CalculationDiagnostic {
            code: "internal-code-not-for-display",
            row: Some(1),
            message: IMPORTED.into(),
        }],
        trace: Vec::new(),
    };
    let report = CalculationReport {
        competition_name: IMPORTED.into(),
        warnings: vec![IMPORTED.into()],
        competitors: vec![
            CompetitorPreview {
                number: 7,
                driver: IMPORTED.into(),
                class_code: IMPORTED.into(),
                imported_total: 23,
                calculation: Ok(calculation),
                notes: vec![IMPORTED.into()],
            },
            CompetitorPreview {
                number: 8,
                driver: IMPORTED.into(),
                class_code: IMPORTED.into(),
                imported_total: 12,
                calculation: Err(CoreError::InvalidInput(IMPORTED.into())),
                notes: Vec::new(),
            },
        ],
    };
    let html = calculation_html(&report);
    safe_html(&html);
    assert!(html.contains("Tuloslaskennan esikatselu"));
    assert!(html.contains("Tallennettu tulos: <strong>23 p</strong>"));
    assert!(html.contains("Virhepisteet yhteensä (asemarivit): 9 p"));
    assert!(!html.contains("kehitysversio"));
    assert!(!html.contains("varmistamatta"));
    assert!(html.contains("Aikavirhepisteet: 7 p"));
    assert!(html.contains("Tarkastustulos: KESK"));
    assert!(html.contains("Laskentaa ei voitu tehdä: Virheellinen syöte:"));
    assert!(html.contains("Tavoite (s)"));
    assert!(html.contains("Toteutunut (s)"));
    assert!(!html.contains("internal-code-not-for-display"));
    assert!(!html.contains("RecoveredLiveV1"));
    assert!(!html.contains("Interrupted"));
}

#[test]
fn provisional_ranking_does_not_publish_suppressed_points() {
    let archive = archive();
    let mut report = ranked(&archive);
    let group = &mut report.groups[0];
    group.provisional = true;
    group.np_enabled = false;
    group.series_method = 0;
    group.entries[0].table_points = 4;
    group.entries[0].series_points = 9999;
    let html = ranking_html(&archive, &report);
    assert!(html.contains("Alustavat tulokset."));
    assert!(!html.contains(">9999<"));
    assert!(html.contains("<td class=\"numeric\">—</td><td class=\"numeric\">—</td>"));
}

static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

fn temp_directory() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "as2015-raporttitesti-{}-{}",
        std::process::id(),
        NEXT_PATH.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn save_new_writes_utf8_and_refuses_to_overwrite_existing_file() {
    let directory = temp_directory();
    let path = directory.join("tulokset ä.html");
    let html = "<!doctype html><html lang=\"fi\"><p>Ääkköset säilyvät.</p></html>";
    save_new(&path, html).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), html);
    let error = save_new(&path, "korvattu").unwrap_err().to_string();
    assert!(error.contains("sitä ei korvata"));
    assert_eq!(fs::read_to_string(&path).unwrap(), html);
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn save_new_refuses_existing_symlink_without_touching_target() {
    let directory = temp_directory();
    let target = directory.join("lähde.html");
    let link = directory.join("linkki.html");
    fs::write(&target, "säilytettävä").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(save_new(&link, "korvattu").is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "säilytettävä");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn user_notices_keep_actionable_input_errors_and_hide_research_notes() {
    let note = "Kilpailija 7: Palautettu korttilaskenta johtaa leimattoman maalin ajan kortin viimeisestä leimasta. Vanhan ohjelman tämän kohdan käsittely on ristiriitainen. Valitun laskentatavan vastaavuus vanhoihin kilpailutuloksiin on vielä tarkistettava.";
    assert_eq!(
        user_notice(note).unwrap(),
        "Kilpailija 7: Maaliaika on johdettu kortin viimeisestä leimasta."
    );
    assert_eq!(
        user_notice("Virheellinen syöte: Kortin 123 raakaleimauksia ei löydy emit.txt-tiedostosta")
            .unwrap(),
        "Virheellinen syöte: Kortin 123 raakaleimauksia ei löydy emit.txt-tiedostosta"
    );
    assert!(user_notice("Esikatselu sisältää asemarivien virhepisteet. Erillisiä RT-lisäyksiä ja muita kilpailijakohtaisia virhepisteitä ei vielä yhdistetä siihen.").is_none());
}

#[test]
fn native_reports_present_results_without_development_banners() {
    let project =
        as2026::native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
            .unwrap();
    let archive = as2026::application::calculated_project(&project).unwrap();
    let calculation = as2026::application::calculate(&project.to_archive().unwrap()).unwrap();
    let ranking =
        as2026::application::rank_stored(&archive, ranking::TiePolicy::KeepSharedAndFlag).unwrap();
    for html in [
        as2026::report::project_calculation_html(&calculation),
        as2026::report::project_ranking_html(&archive, &ranking),
    ] {
        assert!(!html.contains("kehitysversio"));
        assert!(!html.contains("varmistamatta"));
        assert!(!html.contains("Vanhan ohjelman"));
        assert!(!html.contains("vastaavuus"));
        assert!(html.contains("Maaliaika on johdettu kortin viimeisestä leimasta."));
        assert!(html.contains("Virhepisteet") || html.contains("Tulos"));
    }
}
