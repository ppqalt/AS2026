#[cfg(feature = "gui")]
use as2026::printing::{self, Kind};
use as2026::{
    application,
    native::{Project, ReadPunch},
    ranking::*,
    terminal,
};
fn project() -> Project {
    Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}

#[test]
fn manual_station_time_is_between_neighbors_across_midnight() {
    let mut p = project();
    p.kilpailijat[0].lahtoaika = "23:55:00".into();
    p.radat[0].asemat[1].emit_koodi = 0;
    let mut r = application::capture_project_result(&p, 7).unwrap();
    let index = r
        .rivit
        .iter()
        .find(|r| r.sequence == 2)
        .unwrap()
        .output_index;
    let before = r.clone();
    let error = r.set_clock(index, [23, 54, 0]).unwrap_err().to_string();
    assert!(error.contains("edellisen ja seuraavan"));
    assert_eq!(r, before);
    r.set_clock(index, [0, 0, 0]).unwrap();
    assert!(r.set_clock(index, [0, 6, 0]).is_err());
    r.set_clock(index, [23, 55, 0]).unwrap();
}

#[cfg(feature = "desktop")]
#[test]
fn unchecked_interruption_is_not_dropped_from_live_results() {
    let mut p = project();
    p.radat[0].asemat[1].tyyppi = "JAT".into();
    p.radat[0].asemat[1].emit_koodi = 77;
    let settings = as2026::live_pages::Settings {
        base: "tulokset.html".into(),
        classes: vec!["A".into()],
    };
    let pages = as2026::live_pages::pages(&p, &settings).unwrap();
    assert!(pages[0].1.contains(">AJAA<"));
    let mut result = application::capture_project_result(&p, 7).unwrap();
    let finish = result
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    result.set_clock(finish, [12, 10, 0]).unwrap();
    let mut result = application::recalculate_stored_result(&p, 7, &result).unwrap();
    assert_eq!(result.tila, "KESK");
    result.hyvaksytty = true;
    p.kilpailijat[0].tallennettu_tulos = Some(result);
    let pages = as2026::live_pages::pages(&p, &settings).unwrap();
    assert!(!pages[0].1.contains(">AJAA<"));
    assert!(pages[0].1.contains(">KESK<"));
}

#[test]
fn compact_clock_validates_all_six_digits_and_leading_zeroes() {
    for (value, h, m, s) in [
        ("123456", 12, 34, 56),
        ("090507", 9, 5, 7),
        ("000000", 0, 0, 0),
    ] {
        assert_eq!(
            terminal::parse_clock(value).unwrap(),
            as2026::ClockTime::new(h, m, s).unwrap()
        );
    }
    for value in ["12345", "1234567", "240000", "126000", "120060", "12a456"] {
        assert!(terminal::parse_clock(value).is_err(), "{value}");
    }
}

#[test]
fn kiti_template_keeps_identity_cells_and_writes_numeric_results() {
    use calamine::{DataType, Reader};
    use std::io::{Cursor, Read, Write};
    let mut p = project();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    let finish = result
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    result.set_clock(finish, [12, 10, 0]).unwrap();
    result = application::recalculate_stored_result(&p, 7, &result).unwrap();
    result.hyvaksytty = true;
    p.kilpailijat[0].tallennettu_tulos = Some(result);
    p.kilpailijat[0].lisavirheet.muut_virhepisteet = 1000;
    let mut class = p.luokat[0].clone();
    class.tunnus = "B".into();
    class.nimi = "Toinen luokka".into();
    p.luokat.push(class);
    let mut driver = p.kilpailijat[0].clone();
    driver.numero = 8;
    driver.emit = 888;
    driver.luokka = "B".into();
    driver.lisavirheet.muut_virhepisteet = 0;
    driver.tallennettu_tulos = None;
    p.kilpailijat.push(driver);
    let mut read = p.leimaluvut[0].clone();
    read.emit = 888;
    p.leimaluvut.push(read);
    let mut result = application::capture_project_result(&p, 8).unwrap();
    result.set_clock(finish, [12, 10, 0]).unwrap();
    let mut result = application::recalculate_stored_result(&p, 8, &result).unwrap();
    result.hyvaksytty = true;
    p.kilpailijat[1].tallennettu_tulos = Some(result);
    p.yhdistetyt_luokat.push(as2026::native::CombinedClass {
        nimi: "Yhteistulos".into(),
        luokat: vec!["A".into(), "B".into()],
        nousupisteet: 0,
        sarjapistetapa: 0,
    });
    let labels = [
        "EnrollID",
        "Kilpailunumero / Competition number",
        "1-Ohjaaja / Driver",
        "Kansallisuus / Nationality",
        "Seura / Club",
        "Lisenssinumero / License number",
        "Lisenssiluokka / Licence level",
        "Date of birth",
        "2-Ohjaaja / Driver",
        "Nationality",
        "Club",
        "License number",
        "Licence level",
        "Date of birth",
        "Ilmoittaja / Entrant",
        "Auton merkki / Car make",
        "Auton malli / Car model",
        "Auton nimi / Car name",
        "Tulosselite / Result definition",
        "Sij. / Position",
        "Lkasij. / Class position",
        "AT-pisteet / Time Control points",
        "RT-pisteet / Route Control points",
        "Kilpailun loppupisteet / Total points",
        "Nousup. / Rating points",
    ];
    let mut identity = vec![""; 25];
    identity[0] = "synthetic-enroll-0001";
    identity[1] = "7";
    identity[2] = "Kuvitteellinen Ää";
    identity[5] = "001234";
    let csv = format!(
        "Synteettinen kilpailu\n\nLuokka / Class;A\n{}\n{}\n",
        labels.join(";"),
        identity.join(";")
    );
    let bytes = as2026::report::xlsx_from_csv(&csv).unwrap();
    let mut input = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut file = input.by_index(i).unwrap();
        let name = file.name().to_owned();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        if name == "xl/workbook.xml" {
            bytes = String::from_utf8(bytes)
                .unwrap()
                .replace("name=\"Tulokset\"", "name=\"Results\"")
                .into_bytes();
        }
        output
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        output.write_all(&bytes).unwrap();
    }
    let bytes = output.finish().unwrap().into_inner();
    let directory = std::env::temp_dir().join(format!("as2026-kiti-072-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("synthetic-template.xlsx");
    std::fs::write(&path, &bytes).unwrap();
    assert_eq!(as2026::kiti::template_classes(&path).unwrap(), ["A"]);
    let codes = as2026::kiti::parse_codes("TRK=1,KESK=2").unwrap();
    let filled = as2026::kiti::fill(&p, &path, &codes).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut original = calamine::Xlsx::new(Cursor::new(bytes.clone())).unwrap();
    let original = original.worksheet_range("Results").unwrap();
    let mut workbook = calamine::Xlsx::new(Cursor::new(filled.clone())).unwrap();
    let range = workbook.worksheet_range("Results").unwrap();
    for r in 0..5 {
        for c in 0..18 {
            assert_eq!(range.get_value((r, c)), original.get_value((r, c)));
        }
    }
    assert_eq!(
        range.get_value((4, 18)).and_then(DataType::as_f64),
        Some(1.0)
    );
    assert_eq!(
        range.get_value((4, 19)).and_then(DataType::as_f64),
        Some(1.0)
    );
    assert_eq!(
        range.get_value((4, 20)).and_then(DataType::as_f64),
        Some(1.0)
    );
    assert!(matches!(
        range.get_value((4, 23)),
        Some(calamine::Data::Int(_) | calamine::Data::Float(_))
    ));
    let mut mappings = std::collections::BTreeMap::from([(
        "A".into(),
        as2026::kiti::ClassMapping {
            base: "A".into(),
            combined: "Y1: Yhteistulos".into(),
        },
    )]);
    let mapped = as2026::kiti::fill_with_class_mappings(&p, &path, &codes, &mappings).unwrap();
    let mut workbook = calamine::Xlsx::new(Cursor::new(mapped)).unwrap();
    let mapped = workbook.worksheet_range("Results").unwrap();
    assert_eq!(
        mapped.get_value((4, 19)).and_then(DataType::as_f64),
        Some(2.0)
    );
    assert_eq!(
        mapped.get_value((4, 20)).and_then(DataType::as_f64),
        Some(1.0)
    );
    // Both selectors accept every result class, including a combined base.
    mappings.get_mut("A").unwrap().base = "Y1: Yhteistulos".into();
    mappings.get_mut("A").unwrap().combined = "A".into();
    let mapped = as2026::kiti::fill_with_class_mappings(&p, &path, &codes, &mappings).unwrap();
    let mut workbook = calamine::Xlsx::new(Cursor::new(mapped)).unwrap();
    let mapped = workbook.worksheet_range("Results").unwrap();
    assert_eq!(
        mapped.get_value((4, 19)).and_then(DataType::as_f64),
        Some(1.0)
    );
    assert_eq!(
        mapped.get_value((4, 20)).and_then(DataType::as_f64),
        Some(2.0)
    );
    mappings.get_mut("A").unwrap().combined = "B".into();
    assert!(as2026::kiti::fill_with_class_mappings(&p, &path, &codes, &mappings).is_err());
    // The same competitor has a different class code in the independent BC results.
    p.beginners_cup = Some(Box::new(as2026::beginners::BeginnersCup {
        nimi: "Beginners Cup".into(),
        tunnus: "BC".into(),
        lahderata: p.radat[0].numero,
        lahdeasemat: p.radat[0].asemat.clone(),
        mukana_asemat: vec![1, 2, 3, 4],
    }));
    p.bc_osallistujat.push(as2026::beginners::Entry {
        numero: 7,
        osallistuu: true,
        ..Default::default()
    });
    let projected = as2026::beginners::projection(&p).unwrap();
    let mut bc_result = application::capture_project_result(&projected, 7).unwrap();
    bc_result.set_clock(finish, [12, 10, 0]).unwrap();
    let mut bc_result = application::recalculate_stored_result(&projected, 7, &bc_result).unwrap();
    bc_result.hyvaksytty = true;
    p.bc_osallistujat[0].tallennettu_tulos = Some(bc_result);
    mappings.get_mut("A").unwrap().base = "A".into();
    mappings.get_mut("A").unwrap().combined = "BC".into();
    let mapped = as2026::kiti::fill_with_class_mappings(&p, &path, &codes, &mappings).unwrap();
    let mut workbook = calamine::Xlsx::new(Cursor::new(mapped)).unwrap();
    let mapped = workbook.worksheet_range("Results").unwrap();
    assert_eq!(
        mapped.get_value((4, 19)).and_then(DataType::as_f64),
        Some(1.0)
    );
    assert_eq!(
        mapped.get_value((4, 20)).and_then(DataType::as_f64),
        Some(1.0)
    );
    let mut before = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut after = zip::ZipArchive::new(Cursor::new(filled.clone())).unwrap();
    for i in 0..before.len() {
        let mut file = before.by_index(i).unwrap();
        if file.name() == "xl/worksheets/sheet1.xml" {
            continue;
        }
        let mut old = Vec::new();
        file.read_to_end(&mut old).unwrap();
        let mut new = Vec::new();
        after
            .by_name(file.name())
            .unwrap()
            .read_to_end(&mut new)
            .unwrap();
        assert_eq!(old, new);
    }
    if let Some(folder) = std::env::var_os("AS2026_RELEASE_QA_DIR") {
        std::fs::write(
            std::path::PathBuf::from(folder).join("kiti-synthetic.xlsx"),
            filled,
        )
        .unwrap();
    }
    assert!(as2026::kiti::fill(&p, &path, &std::collections::BTreeMap::new()).is_err());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&directory).unwrap();
}
#[test]
fn second_interval_keeps_old_files_and_draws_ninety_seconds() {
    let mut p = project();
    assert_eq!(p.luokat[0].start_interval_seconds(), 60);
    p.luokat[0].lahtovali_sekuntia = Some(90);
    p.leimaluvut.clear();
    let mut second = p.kilpailijat[0].clone();
    second.numero = 8;
    second.emit = 888;
    second.tallennettu_tulos = None;
    p.kilpailijat.push(second);
    let draw = as2026::starts::draw(&p, 42).unwrap();
    let mut times = draw
        .assignments
        .iter()
        .map(|a| a.time.seconds_since_midnight().unwrap())
        .collect::<Vec<_>>();
    times.sort();
    assert_eq!(times[1] - times[0], 90);
    let reopened = Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(reopened.luokat[0].start_interval_seconds(), 90);
}
#[test]
fn alternating_battery_artifacts_keep_first_real_time_and_raw_history() {
    for marker in [99, 999] {
        let mut p = project();
        p.radat[0].asemat[1].emit_koodi = 36;
        p.radat[0].asemat[2].emit_koodi = 37;
        p.leimaluvut[0].leimat = vec![
            ReadPunch {
                koodi: 0,
                aika_sekuntia: 120,
            },
            ReadPunch {
                koodi: 36,
                aika_sekuntia: 300,
            },
            ReadPunch {
                koodi: marker,
                aika_sekuntia: 301,
            },
            ReadPunch {
                koodi: 36,
                aika_sekuntia: 302,
            },
            ReadPunch {
                koodi: marker,
                aika_sekuntia: 303,
            },
            ReadPunch {
                koodi: 36,
                aika_sekuntia: 304,
            },
            ReadPunch {
                koodi: marker,
                aika_sekuntia: 305,
            },
            ReadPunch {
                koodi: 37,
                aika_sekuntia: 600,
            },
            ReadPunch {
                koodi: 250,
                aika_sekuntia: 700,
            },
        ];
        let raw = p.leimaluvut.clone();
        let result = application::capture_project_result(&p, 7).unwrap();
        assert_eq!(
            result
                .rivit
                .iter()
                .filter(|r| r.received_code == 36)
                .count(),
            1
        );
        assert_eq!(
            result
                .rivit
                .iter()
                .find(|r| r.received_code == 36)
                .unwrap()
                .elapsed_seconds,
            300
        );
        assert!(
            !result
                .rivit
                .iter()
                .any(|r| matches!(r.received_code, 99 | 999))
        );
        assert_eq!(p.leimaluvut, raw);
    }
}
#[test]
fn blank_card_and_card_number_correction_are_traceable_and_persistent() {
    let mut p = project();
    p.leimaluvut.clear();
    as2026::emit_corrections::save_card(
        &mut p,
        7,
        7001,
        vec![
            ReadPunch {
                koodi: 31,
                aika_sekuntia: 395,
            },
            ReadPunch {
                koodi: 32,
                aika_sekuntia: 720,
            },
        ],
        true,
    )
    .unwrap();
    assert!(p.leimaluvut[0].lahde.as_deref().unwrap().contains("Käsin"));
    let original = p.leimaluvut[0].clone();
    as2026::emit_corrections::save_card(
        &mut p,
        7,
        7002,
        vec![
            ReadPunch {
                koodi: 31,
                aika_sekuntia: 395,
            },
            ReadPunch {
                koodi: 32,
                aika_sekuntia: 720,
            },
        ],
        false,
    )
    .unwrap();
    assert_eq!(p.leimaluvut[0], original);
    assert_eq!(p.kilpailijat[0].emit, 7002);
    let reopened = Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(reopened.kilpailijat[0].emit, 7002);
    assert!(reopened.active_readout(7002).is_some());
}
#[test]
fn missing_finish_and_jat_clear_only_after_real_emit_correction() {
    for kind in ["JAT", "M"] {
        let mut p = project();
        p.radat[0].asemat.last_mut().unwrap().emit_koodi = 33;
        if kind == "JAT" {
            p.radat[0].asemat[1].tyyppi = "JAT".into();
            p.radat[0].asemat[1].emit_koodi = 77;
        } else {
            p.radat[0].asemat.last_mut().unwrap().emit_koodi = 77;
        }
        let r = application::capture_project_result(&p, 7).unwrap();
        assert_eq!(r.tila, "KESK");
        assert!(r.syy.contains(if kind == "JAT" {
            "JAT puuttuu"
        } else {
            "maali puuttuu"
        }));
        p.kilpailijat[0].tallennettu_tulos = Some(r);
        let punches = if kind == "JAT" {
            vec![
                ReadPunch {
                    koodi: 77,
                    aika_sekuntia: 395,
                },
                ReadPunch {
                    koodi: 32,
                    aika_sekuntia: 720,
                },
                ReadPunch {
                    koodi: 33,
                    aika_sekuntia: 730,
                },
            ]
        } else {
            vec![
                ReadPunch {
                    koodi: 31,
                    aika_sekuntia: 395,
                },
                ReadPunch {
                    koodi: 32,
                    aika_sekuntia: 720,
                },
                ReadPunch {
                    koodi: 77,
                    aika_sekuntia: 730,
                },
            ]
        };
        as2026::emit_corrections::replace_content(&mut p, 7, punches).unwrap();
        let r = p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
        assert_ne!(r.tila, "KESK");
        assert!(!r.hyvaksytty);
    }
}
#[test]
fn pairwise_example_and_reversed_input_have_same_winner() {
    let entry = |id, missing| CompetitorInput {
        source_index: id,
        number: id as i64,
        class_code: "A".into(),
        route_code: "1".into(),
        status: Status::Classified,
        total_penalty: 10000,
        start_hour: Some(12),
        driver_licence: String::new(),
        navigator_licence: String::new(),
        route_errors: Some(RouteErrors {
            missing_count: missing,
            missing_sum: i64::from(missing) * 600,
            extra_count: 0,
            extra_sum: 0,
        }),
        tie_rows: Some(
            (0..13)
                .map(|i| TieRow {
                    position: i + 1,
                    station_key: format!("AT{i}"),
                    kind: as2026::StationKind::At,
                    signed_time_penalty: if (id == 1 && i < 5) || (id == 2 && i >= 5) {
                        0
                    } else {
                        60
                    },
                })
                .collect(),
        ),
    };
    for entries in [
        vec![entry(1, 4), entry(2, 6)],
        vec![entry(2, 6), entry(1, 4)],
    ] {
        let ranked = rank(&RankingInput {
            competitors: entries,
            classes: vec![ClassDefinition {
                code: "A".into(),
                np_enabled: false,
                series_method: 0,
            }],
            tie_policy: TiePolicy::PairwiseControlComparison,
            ..Default::default()
        })
        .unwrap();
        assert_eq!(ranked.groups[0].entries[0].number, 2);
        assert_eq!(ranked.groups[0].entries[0].primary_position, 1);
    }
}
#[cfg(feature = "desktop")]
#[test]
fn live_pending_is_between_approved_and_interrupted_and_interval_cycles() {
    let mut p = project();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    let finish = result
        .rivit
        .iter()
        .find(|r| r.station_type == "M")
        .unwrap()
        .output_index;
    result.set_clock(finish, [12, 10, 0]).unwrap();
    result = application::recalculate_stored_result(&p, 7, &result).unwrap();
    result.hyvaksytty = true;
    p.kilpailijat[0].tallennettu_tulos = Some(result);
    p.kilpailijat[0].kuljettaja = "Valmis pari".into();
    let mut pending = p.kilpailijat[0].clone();
    pending.numero = 8;
    pending.emit = 808;
    pending.kuljettaja = "Ajava pari".into();
    pending.tallennettu_tulos = None;
    p.kilpailijat.push(pending);
    let mut stopped = p.kilpailijat[0].clone();
    stopped.numero = 9;
    stopped.emit = 909;
    stopped.kuljettaja = "Keskeyttänyt pari".into();
    stopped.tila = Some(as2026::native::ManualStatus::new("KESK", "maali puuttuu").unwrap());
    stopped.keskeytys_vahvistettu = true;
    stopped.tallennettu_tulos = None;
    p.kilpailijat.push(stopped);
    let settings = as2026::live_pages::Settings {
        base: "tulokset.html".into(),
        classes: vec!["A".into()],
    };
    let pages = as2026::live_pages::pages_with_interval(&p, &settings, 7).unwrap();
    let html = &pages[0].1;
    assert!(html.find("Valmis pari").unwrap() < html.find("Ajava pari").unwrap());
    assert!(html.find("Ajava pari").unwrap() < html.find("Keskeyttänyt pari").unwrap());
    assert!(html.contains(">AJAA<"));
    assert!(html.contains("7;url=tulokset.html"));
    assert!(as2026::live_pages::pages_with_interval(&p, &settings, 0).is_err());
}
#[cfg(feature = "gui")]
#[test]
fn selected_pdf_embeds_unicode_font_logo_and_keeps_other_class_out() {
    let p = project();
    let archive = application::calculated_project(&p).unwrap();
    let mut document =
        printing::classes(&archive, &["A".into()], Kind::Classes, false, false).unwrap();
    document.pages[0].texts[0].value = "Synteettinen kilpailu Ää Öö".into();
    let image = image::RgbaImage::from_pixel(80, 40, image::Rgba([20, 80, 160, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let bytes = as2026::export::pdf(&document, &png.into_inner()).unwrap();
    assert!(bytes.starts_with(b"%PDF-1.7"));
    let source = String::from_utf8_lossy(&bytes);
    assert!(source.contains("/FontFile2"));
    assert!(source.contains("/ToUnicode"));
    assert!(source.contains("/Logo"));
    if let Some(folder) = std::env::var_os("AS2026_RELEASE_QA_DIR") {
        let folder = std::path::PathBuf::from(folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("classes.pdf"), bytes).unwrap();
        for kind in [Kind::Classes, Kind::Stations] {
            let doc =
                printing::classes(&archive, &["A".into()], kind, kind == Kind::Stations, false)
                    .unwrap();
            std::fs::write(
                folder.join(if kind == Kind::Stations {
                    "stations.pdf"
                } else {
                    "no-logo.pdf"
                }),
                as2026::export::pdf(&doc, &[]).unwrap(),
            )
            .unwrap();
        }
    }
    let mut many = p.clone();
    for number in 8..108 {
        let mut driver = many.kilpailijat[0].clone();
        driver.numero = number;
        driver.emit = 900000 + number;
        driver.kuljettaja = format!("Synteettinen kuljettaja {number}");
        driver.tallennettu_tulos = None;
        many.kilpailijat.push(driver);
    }
    let archive = application::calculated_project(&many).unwrap();
    let multi = printing::classes(&archive, &["A".into()], Kind::Classes, false, false).unwrap();
    assert!(multi.pages.len() >= 3);
    if let Some(folder) = std::env::var_os("AS2026_RELEASE_QA_DIR") {
        std::fs::write(
            std::path::PathBuf::from(folder).join("multipage.pdf"),
            as2026::export::pdf(&multi, &[]).unwrap(),
        )
        .unwrap();
    }
}
#[cfg(feature = "desktop")]
#[test]
fn pin_leading_zeroes_and_invalid_lengths() {
    for pin in ["0000", "0001", "0427", "9999"] {
        as2026::lan::validate_pairing_code(pin).unwrap();
    }
    for pin in ["", "123", "12345", "a427", " 0427"] {
        assert!(as2026::lan::validate_pairing_code(pin).is_err());
    }
}

#[test]
fn kiti_defaults_and_visible_combined_class_suggestion() {
    let codes = as2026::kiti::parse_codes("").unwrap();
    assert_eq!(codes["TRK"], 1);
    assert_eq!(codes["KESK"], 2);
    assert_eq!(codes["HYL"], 3);
    assert_eq!(codes["SULJ"], 4);
    assert_eq!(codes["PER"], 0);
    assert_eq!(codes["EILÄ"], 0);
    let mut p = project();
    p.luokat[0].nimi = "SM-A+NAC-K".into();
    let code = p.luokat[0].tunnus.clone();
    p.yhdistetyt_luokat.push(as2026::native::CombinedClass {
        nimi: "SM-A".into(),
        luokat: vec![code.clone()],
        nousupisteet: 0,
        sarjapistetapa: 0,
    });
    assert_eq!(as2026::kiti::suggested_group(&p, "SM-A+NAC-K"), "Y1: SM-A");
    assert!(
        as2026::kiti::group_choices(&p).contains(&("SM-A (yhdistetty)".into(), "Y1: SM-A".into()))
    );
    p.yhdistetyt_luokat[0].luokat.clear();
    assert_eq!(as2026::kiti::suggested_group(&p, "SM-A+NAC-K"), code);
}
