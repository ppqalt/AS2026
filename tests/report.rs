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
    report::{calculation_html, ranking_csv, ranking_html, save_new, user_notice},
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
        result_publication: None,
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
            status: Status::from_legacy(&archive.competitors[0].status),
            total_penalty: archive.competitors[0].total_penalties,
            start_hour: Some(10),
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
fn ranking_html_escapes_imported_identity_names_and_route_without_publishing_diagnostics() {
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
    assert!(!html.contains("tallennettuihin tuloksiin"));
    assert!(html.contains("23 p"));
    assert!(!html.contains("Tuntematon tarkastustulos"));
    assert!(!html.contains("UnknownStatus"));
    assert!(!html.contains("Classified"));
}

fn public_results_only(html: &str) {
    for forbidden in [
        "class=\"notice",
        "class=\"warning",
        "Huomautukset",
        "Tulosten huomautukset",
        "Vain hyväksytyt asematulokset sijoitetaan",
        "Vahvistettu keskeytys julkaistaan",
        "Sijoitukset ja pisteet perustuvat tuodun kilpailun",
        "Tasapisteissä kilpailijoilla on jaettu sija",
        "Osallistumistilastoa:",
        "Tuloslaskennan lähtijämäärä:",
        "Tulosluettelon ulkopuolelle jäi",
        "käsikelloa on muutettu",
        "HYVÄKSYMÄTÖN",
        "Tuntematon tarkastustulos",
        "Nousu- ja sarjapisteitä ei julkaista",
        "lähdetietue",
    ] {
        assert!(
            !html.contains(forbidden),
            "Julkisessa tuloksessa: {forbidden}"
        );
    }
}

#[test]
fn public_class_combined_route_and_print_results_omit_all_official_diagnostics() {
    let mut archive = archive();
    let template = archive.competitors[0].clone();
    archive.competitors = (1..=57)
        .map(|number| {
            let mut c = template.clone();
            c.number = number;
            c.class_code = if number <= 28 { "A" } else { "B" }.into();
            c.driver = format!("Pari {number}");
            c.status = if number <= 13 {
                "TRK"
            } else {
                "HYVÄKSYMÄTÖN"
            }
            .into();
            c
        })
        .collect();
    archive.warnings = (29..=41).map(|n| format!("Kilpailija {n}: käsikelloa on muutettu. Pisteet säilyvät ennallaan, kunnes valitaan Tutki.")).collect();
    let report = ranking::rank(&RankingInput {
        classes: ["A", "B"]
            .into_iter()
            .map(|code| ClassDefinition {
                code: code.into(),
                np_enabled: true,
                series_method: 1,
            })
            .collect(),
        combined_classes: vec![ranking::CombinedClassDefinition {
            code: "A+B".into(),
            class_codes: vec!["A".into(), "B".into()],
            np_enabled: true,
            series_method: 1,
        }],
        competitors: archive
            .competitors
            .iter()
            .enumerate()
            .map(|(index, c)| CompetitorInput {
                source_index: index + 1,
                number: i64::from(c.number),
                class_code: c.class_code.clone(),
                route_code: "R1".into(),
                status: Status::from_legacy(&c.status),
                total_penalty: c.total_penalties,
                start_hour: Some(12),
                driver_licence: String::new(),
                navigator_licence: String::new(),
                tie_rows: None,
                route_errors: None,
            })
            .collect(),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(report.omitted_source_indices.len(), 44);
    assert!(!report.diagnostics.is_empty());
    let original_report = report.clone();
    let original_archive = archive.clone();
    for html in [
        ranking_html(&archive, &report),
        as2026::report::project_ranking_html(&archive, &report),
    ] {
        public_results_only(&html);
        assert!(html.contains("@media print"));
        assert!(html.contains("@media(max-width:600px)"));
        assert!(html.contains("<main>\n<section><h2>Luokka: A</h2>"));
        assert!(html.contains("Yhdistetty luokka: A+B"));
        assert!(html.contains("Luokka: B</h2><p class=\"muted\">Alustavat tulokset.</p>"));
        assert!(html.contains("Rata: R1"));
        assert!(html.contains("Pari 13</td>"));
        assert!(!html.contains("Pari 14</td>"));
        assert!(html.contains("<td class=\"numeric\">—</td><td class=\"numeric\">—</td>"));
        if let Ok(folder) = std::env::var("AS2026_PUBLIC_REPORT_CAPTURE") {
            fs::create_dir_all(&folder).unwrap();
            fs::write(PathBuf::from(folder).join("julkiset-tulokset.html"), &html).unwrap();
        }
    }
    assert_eq!(report, original_report);
    assert_eq!(archive.warnings, original_archive.warnings);
    assert_eq!(archive.competitors.len(), 57);
    let internal = calculation_html(&CalculationReport {
        competition_name: "Sisäinen tarkastus".into(),
        competitors: Vec::new(),
        warnings: archive.warnings.clone(),
    });
    assert!(internal.contains("Huomautukset"));
    assert!(internal.contains("käsikelloa on muutettu"));
}

#[test]
fn public_bc_results_and_empty_pending_classes_have_no_explanatory_boxes() {
    let mut project =
        as2026::native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
            .unwrap();
    project.beginners_cup = Some(Box::new(as2026::beginners::BeginnersCup {
        nimi: "Beginners Cup".into(),
        tunnus: "BC".into(),
        lahderata: 1,
        lahdeasemat: project.radat[0].asemat.clone(),
        mukana_asemat: vec![1, 2, 3, 4],
    }));
    project.bc_osallistujat = vec![as2026::beginners::Entry {
        numero: 7,
        osallistuu: true,
        ..Default::default()
    }];
    let html =
        as2026::report::competition_ranking_html(&project, ranking::TiePolicy::default()).unwrap();
    public_results_only(&html);
    for class in ["A", "BC"] {
        assert!(html.contains(&format!(
            "Luokka: {class}</h2><p class=\"muted\">Alustavat tulokset.</p>"
        )));
    }
    assert!(!html.contains("Esimerkkikuljettaja</td>"));
    let internal = as2026::report::project_results_html(&project).unwrap();
    assert!(internal.contains("Tulos odottaa tarkastusta"));
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
    let ranking =
        as2026::application::rank_stored(&archive, ranking::TiePolicy::KeepSharedAndFlag).unwrap();
    assert!(
        as2026::report::project_results_html(&project)
            .unwrap()
            .contains("käsikello puuttuu")
    );
    let public = as2026::report::project_ranking_html(&archive, &ranking);
    public_results_only(&public);
    assert!(!public.contains("käsikello puuttuu"));
    for html in [
        as2026::report::project_results_html(&project).unwrap(),
        as2026::report::project_ranking_html(&archive, &ranking),
    ] {
        assert!(!html.contains("kehitysversio"));
        assert!(!html.contains("varmistamatta"));
        assert!(!html.contains("Vanhan ohjelman"));
        assert!(!html.contains("vastaavuus"));
        assert!(!html.contains("Maalikello perustuu viimeiseen leimaan"));
        assert!(html.contains("Virhepisteet") || html.contains("Tulos"));
    }
}

#[test]
fn ranking_csv_escapes_delimiters_quotes_and_spreadsheet_formulas() {
    let mut source = archive();
    source.competitors[0].driver = "=1+2".into();
    source.competitors[0].navigator = "A\";B".into();
    let csv = ranking_csv(&source, &ranked(&source));
    assert!(csv.starts_with("\"Ryhmän tyyppi\";\"Ryhmä\";"));
    assert!(csv.contains("\"'=1+2\""));
    assert!(csv.contains("\"A\"\";B\""));
    assert!(csv.contains("\"23\""));
    assert!(csv.contains("\r\n"));
    // CSV is plain text, not HTML. Preserve angle brackets as data.
    assert!(csv.contains("<script>"));
}

#[test]
fn provisional_ranking_csv_leaves_np_and_sp_cells_blank() {
    let source = archive();
    let mut report = ranked(&source);
    report.groups[0].provisional = true;
    report.groups[0].np_enabled = false;
    report.groups[0].series_method = 0;
    let csv = ranking_csv(&source, &report);
    let class_row = csv.lines().nth(1).unwrap();
    assert!(class_row.ends_with("\"\";\"\""));
}

#[test]
fn status_reasons_are_appended_to_html_csv_and_xlsx_without_changing_previous_columns() {
    use std::io::{Cursor, Read};
    let reason = "=Päätös \"<script>\" & 'Ää'; perustelu";
    for status in ["HYL", "PER", "KESK"] {
        let mut source = archive();
        source.competitors[0].status = status.into();
        source.competitors[0].header_fields = vec![String::new(); 31];
        source.competitors[0].header_fields[28] = status.into();
        source.competitors[0].header_fields[29] = reason.into();
        let original = source.competitors[0].header_fields.clone();
        let report = ranked(&source);
        let html = ranking_html(&source, &report);
        assert!(html.contains("<th scope=\"col\">Syy</th>"));
        assert_eq!(
            html.matches("=Päätös &quot;&lt;script&gt;&quot; &amp; &#39;Ää&#39;; perustelu")
                .count(),
            2
        );
        assert!(!html.contains("<script>"));
        assert!(html.contains(&format!("<td>{status}</td>")));

        let csv = ranking_csv(&source, &report);
        let header = csv.lines().next().unwrap();
        assert!(header.ends_with("\"Np\";\"Sp\";\"Syy\""));
        for row in csv.lines().skip(1) {
            assert!(row.contains(&format!(";\"{status}\";\"23\";")));
            assert!(row.ends_with("\"'=Päätös \"\"<script>\"\" & 'Ää'; perustelu\""));
        }

        let bytes = as2026::report::ranking_xlsx(&source, &report).unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut sheet = String::new();
        zip.by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet)
            .unwrap();
        assert!(
            sheet.contains("<c r=\"L1\" t=\"inlineStr\"><is><t xml:space=\"preserve\">Syy</t>")
        );
        assert!(sheet.contains("<c r=\"L2\" t=\"inlineStr\">"));
        assert!(
            sheet.contains("Päätös &quot;&lt;script&gt;&quot; &amp; &apos;Ää&apos;; perustelu")
        );
        assert!(!sheet.contains("<f>"));
        assert_eq!(source.competitors[0].header_fields, original);
    }
}

#[test]
fn operational_station_report_hides_pending_finish_duration_and_final_total() {
    let project =
        as2026::native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
            .unwrap();
    let before = project.clone();
    let html = as2026::report::project_results_html(&project).unwrap();
    assert!(html.contains("Tila: <strong>ILM</strong>"));
    assert!(html.contains("Käsikelloja puuttuu: 1."));
    assert!(html.contains("Käytetty aika: — · Maaliaika: —"));
    assert!(html.contains("Kokonaistulos: —"));
    assert!(!html.contains("Kokonaistulos: 645 p"));
    assert!(html.contains("<th scope=\"col\">Emit</th>"));
    assert!(html.contains("<th scope=\"col\">Luettu</th>"));
    assert!(html.contains("<th scope=\"col\">Kello</th>"));
    assert!(html.contains("12:04:35"));
    assert!(!html.contains("Maalikello perustuu viimeiseen leimaan"));
    assert_eq!(
        project, before,
        "raportin muodostaminen ei tallenna eikä muuta kilpailua"
    );
}

#[test]
fn operational_station_report_escapes_names_and_preserves_accepted_five_part_corrections() {
    let mut project =
        as2026::native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
            .unwrap();
    project.kilpailu.nimi = IMPORTED.into();
    project.kilpailijat[0].kuljettaja = IMPORTED.into();
    project.radat[0].asemat[1].nimi = IMPORTED.into();
    let mut stored = as2026::application::capture_project_result(&project, 7).unwrap();
    let finish = stored
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    stored.set_clock(finish, [12, 10, 0]).unwrap();
    let mut stored = as2026::application::recalculate_stored_result(&project, 7, &stored).unwrap();
    stored.set_row_penalties(2, -77, 11).unwrap();
    stored.hyvaksytty = true;
    project.kilpailijat[0].lisavirheet = as2026::native::AdditionalPenalties {
        puuttuvat_rt_kpl: 2,
        puuttuvat_rt_pisteet: 1200,
        ylimaaraiset_rt_kpl: 3,
        ylimaaraiset_rt_pisteet: 75,
        muut_virhepisteet: 100,
    };
    project.kilpailijat[0].tila = Some(as2026::native::ManualStatus {
        tunnus: "HYL".into(),
        syy: IMPORTED.into(),
    });
    project.kilpailijat[0].tallennettu_tulos = Some(stored);
    let before = project.clone();
    let html = as2026::report::project_results_html(&project).unwrap();
    safe_html(&html);
    assert!(html.contains("Hyväksytty tulos."));
    assert!(html.contains("Käytetty aika: 00:10:00 · Maaliaika: 12:10:00"));
    assert!(html.contains("Kokonaistulos: 2063 p"));
    assert!(html.contains("Aikavirheet: 77 p"));
    assert!(html.contains("Asemien muut virheet: 611 p"));
    assert!(html.contains("Puuttuvat RT: 2 kpl / 1200 p"));
    assert!(html.contains("Muut virheet: 100 p"));
    assert!(html.contains("Ylimääräiset RT: 3 kpl / 75 p"));
    assert!(html.contains("Tila: <strong>HYL</strong>"));
    assert!(html.contains("class=\"numeric\">-77</td>"));
    assert_eq!(project, before);
}
