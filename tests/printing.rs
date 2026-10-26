use as2026::{
    application,
    native::Project,
    printing::{self, Kind},
    workspace::Workspace,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "desktop")]
fn save_preview(root: &std::path::Path, name: &str, document: &printing::Document) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join(format!("{name}.json")),
        serde_json::to_vec(document).unwrap(),
    )
    .unwrap();
    #[cfg(all(windows, feature = "gui"))]
    {
        let paths = printing::preview_png(document, &root.join(name)).unwrap();
        assert_eq!(paths.len(), document.pages.len());
        let expected = if document.landscape {
            (1754, 1240)
        } else {
            (1240, 1754)
        };
        for path in paths {
            let image = image::open(path).unwrap().to_rgb8();
            assert_eq!(image.dimensions(), expected);
            // Kokonaan valkoinen sivu ei ole onnistunut piirto.
            assert!(
                image
                    .pixels()
                    .filter(|p| p.0.iter().any(|&c| c < 200))
                    .count()
                    > 100
            );
        }
    }
}

#[test]
fn pair_layout_and_raw_time_differences() {
    let mut f = Fixture::new();
    let mut clocks = f.clocks();
    clocks[0].1 = [12, 5, 7];
    f.ws.complete_result_entry(7, &clocks, 1, 25).unwrap();
    let doc = printing::competitor(f.ws.project().unwrap(), 7, None).unwrap();
    let texts = &doc.pages[0].texts;
    assert!(
        !texts
            .iter()
            .any(|t| matches!(t.value.as_str(), "Kilpailijaparin tulos" | "Asema" | "Klo"))
    );
    assert!(
        texts
            .iter()
            .any(|t| t.size == 33. && t.x >= 145. && t.y < 20.)
    );
    for label in ["AT", "ATp/ATy", "RT", "Muut"] {
        assert!(texts.iter().any(|t| t.value == label && t.size == 18.));
    }
    let result = f.ws.result_snapshot(7).unwrap().unwrap();
    let penalty = result
        .rivit
        .iter()
        .find(|r| r.sequence == 2)
        .unwrap()
        .signed_time_penalty;
    assert!(texts.iter().any(|t| t.value == format!("{penalty:+}")));
    assert!(texts.iter().any(|t| t.value == "12:05:07"));
    for label in ["klo", "ATp", "ATy"] {
        assert!(texts.iter().any(|t| t.value == label));
    }
    assert!(
        texts
            .iter()
            .filter(|t| t.value == "Nimi")
            .all(|t| (80. ..85.).contains(&t.y))
    );
}

#[test]
fn total_delay_sums_sections_without_an_explicit_section_value() {
    let mut f = Fixture::new();
    f.ws.complete_result_entry(7, &f.clocks(), 0, 0).unwrap();
    for (actual, expected) in [(vec![607, 63], "-00:10"), (vec![600, 60], "+00:00")] {
        let mut project = f.ws.project().unwrap().clone();
        let result = project.kilpailijat[0].tallennettu_tulos.as_mut().unwrap();
        for (row, seconds) in result
            .rivit
            .iter_mut()
            .filter(|r| matches!(r.station_type.as_str(), "JAT" | "M"))
            .zip(actual)
        {
            row.section_text.clear();
            row.actual_seconds = seconds;
        }
        let doc = printing::competitor(&project, 7, None).unwrap();
        let texts = &doc.pages[0].texts;
        let index = texts
            .iter()
            .position(|t| t.value == "Kokonaismyöhä")
            .unwrap();
        assert_eq!(texts[index + 1].value, expected);
    }
}

#[test]
fn terminal_statuses_print_without_emit_and_class_rows_have_sporting_order() {
    let mut f = Fixture::new();
    f.ws.complete_result_entry(7, &f.clocks(), 0, 0).unwrap();
    let mut p = f.ws.project().unwrap().clone();
    p.yhdistetyt_luokat.push(as2026::native::CombinedClass {
        nimi: "Yhteistulos".into(),
        luokat: vec!["A".into()],
        nousupisteet: 0,
        sarjapistetapa: 0,
    });
    for (number, status) in [(8, "PER"), (9, "HYL"), (10, "KESK"), (11, "ILM")] {
        let mut d = p.kilpailijat[0].clone();
        d.numero = number;
        d.emit = 0;
        d.tallennettu_tulos = None;
        d.kuljettaja = format!("Kuljettaja{number}");
        d.tila = Some(as2026::native::ManualStatus::new(status, "Tekninen vika").unwrap());
        d.keskeytys_vahvistettu = false;
        p.kilpailijat.push(d);
    }
    let folder = f.directory.join("tilat");
    let mut ws = Workspace::create_imported(&folder, p).unwrap();
    for number in [8, 9, 10] {
        ws.complete_result_entry(number, &[], 0, 0).unwrap();
        assert!(printing::competitor(ws.project().unwrap(), number, None).is_ok());
    }
    let archive = ws.results().unwrap();
    let combined = printing::class_choices(&archive)
        .unwrap()
        .into_iter()
        .find(|c| c.starts_with('Y'))
        .unwrap();
    for kind in [Kind::Classes, Kind::Stations] {
        let doc = printing::classes(&archive, std::slice::from_ref(&combined), kind, false, true)
            .unwrap();
        let texts = &doc.pages[0].texts;
        assert!(
            texts
                .iter()
                .any(|t| t.value.ends_with("luokka: Yhteistulos"))
        );
        assert!(!texts.iter().any(|t| t.value.contains("(alustava)")
            || t.value.contains("Y0:")
            || t.value.contains("Y1:")));
        let names = texts
            .iter()
            .filter(|t| t.value.starts_with("Kuljettaja") && t.value != "Kuljettaja")
            .map(|t| t.value.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            ["Kuljettaja11", "Kuljettaja10", "Kuljettaja9", "Kuljettaja8"]
        );
        assert!(texts.iter().any(|t| t.value == "AJAA"));
    }
}
struct Fixture {
    directory: PathBuf,
    ws: Workspace,
}
impl Fixture {
    fn new() -> Self {
        let mut p =
            Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
        p.radat[0].asemat[1].emit_koodi = 0;
        p.radat[0].asemat[2].emit_koodi = 0;
        p.radat[0].asemat[2].tyyppi = "JAT".into();
        p.radat[0].asemat[3].tavoiteaika_sekuntia = 60;
        p.kilpailijat[0].kuljettajan_seura = "HyUA".into();
        p.kilpailijat[0].kartanlukijan_seura = "I-HelUA".into();
        p.kilpailijat[0].auto = "SX4-TP".into();
        let directory = std::env::temp_dir().join(format!(
            "as2026-print-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut ws = Workspace::create_imported(&directory, p).unwrap();
        ws.ensure_readout_result(7).unwrap();
        Self { directory, ws }
    }
    fn clocks(&self) -> Vec<(u32, [i64; 3])> {
        self.ws
            .result_snapshot(7)
            .unwrap()
            .unwrap()
            .rivit
            .iter()
            .filter(|r| r.expected_code == 0 && r.station_type != "L")
            .map(|r| {
                (
                    r.output_index,
                    match r.sequence {
                        2 => [12, 5, 0],
                        3 => [12, 10, 0],
                        _ => [12, 11, 0],
                    },
                )
            })
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}
#[test]
fn entry_calculates_manual_clocks_and_publishes_without_a_separate_approval() {
    let mut f = Fixture::new();
    let raw = f.ws.project().unwrap().leimaluvut.clone();
    f.ws.complete_result_entry(7, &f.clocks(), 3, 25).unwrap();
    let p = f.ws.project().unwrap();
    let d = &p.kilpailijat[0];
    assert!(d.tallennettu_tulos.as_ref().unwrap().hyvaksytty);
    assert_eq!(d.lisavirheet.puuttuvat_rt_pisteet, 1800);
    assert_eq!(d.lisavirheet.muut_virhepisteet, 25);
    assert_eq!(p.leimaluvut, raw);
    let doc = printing::competitor(p, 7, None).unwrap();
    assert_eq!(doc.pages.len(), 1);
    assert!(!doc.landscape);
    assert!(
        doc.pages[0]
            .texts
            .iter()
            .any(|t| t.value == "HyUA / I-HelUA")
    );
    assert!(doc.pages[0].texts.iter().any(|t| t.value == "SX4-TP"));
    assert!(doc.pages[0].texts.iter().any(|t| t.value == "12:00:00"));
    #[cfg(feature = "desktop")]
    if let Some(root) = std::env::var_os("AS2026_PRINT_PREVIEW_DIR") {
        let root = PathBuf::from(root);
        save_preview(&root, "kilpailija", &doc);
        for (name, kind, landscape) in [
            ("luokka", Kind::Classes, false),
            ("asemat", Kind::Stations, true),
        ] {
            let doc = printing::classes(
                &application::calculated_project(p).unwrap(),
                &["A".into()],
                kind,
                landscape,
                true,
            )
            .unwrap();
            save_preview(&root, name, &doc);
        }
    }
}
#[test]
fn incomplete_or_invalid_entry_preserves_the_previous_file() {
    let mut f = Fixture::new();
    let before = std::fs::read(f.ws.path()).unwrap();
    let mut clocks = f.clocks();
    assert!(f.ws.complete_result_entry(7, &clocks[..1], 2, 0).is_err());
    clocks[0].1 = [12, 60, 0];
    assert!(f.ws.complete_result_entry(7, &clocks, 2, 0).is_err());
    assert_eq!(std::fs::read(f.ws.path()).unwrap(), before);
    assert!(f.ws.complete_result_entry(7, &f.clocks(), 0, -1).is_err());
}
#[test]
fn unfinished_results_are_not_printed_as_classified_and_extras_are_not_route_columns() {
    let mut f = Fixture::new();
    assert!(printing::competitor(f.ws.project().unwrap(), 7, None).is_err());
    let doc = printing::classes(
        &f.ws.results().unwrap(),
        &["A".into()],
        Kind::Classes,
        false,
        true,
    )
    .unwrap();
    assert!(
        doc.pages[0]
            .texts
            .iter()
            .any(|t| t.value == "Esimerkkikuljettaja")
    );
    assert!(doc.pages[0].texts.iter().any(|t| t.value == "AJAA"));
    assert!(
        doc.pages[0]
            .texts
            .iter()
            .any(|t| t.value.contains("avoin 1"))
    );
    let clocks = f.clocks();
    f.ws.complete_result_entry(7, &clocks, 0, 0).unwrap();
    let doc = printing::classes(
        &f.ws.results().unwrap(),
        &["A".into()],
        Kind::Stations,
        true,
        true,
    )
    .unwrap();
    assert!(
        !doc.pages[0]
            .texts
            .iter()
            .any(|t| t.value == "Ylimääräinen asema")
    );
    let doc = printing::competitor(f.ws.project().unwrap(), 7, None).unwrap();
    assert!(doc.pages[0].texts.iter().any(|t| t.value == "90"));
}

#[test]
fn class_selection_orientation_and_dense_page_fitting() {
    let mut f = Fixture::new();
    f.ws.complete_result_entry(7, &f.clocks(), 1, 12).unwrap();
    let mut p = f.ws.project().unwrap().clone();
    let mut class = p.luokat[0].clone();
    class.tunnus = "B".into();
    p.luokat.push(class);
    for number in 8..=90 {
        let mut d = p.kilpailijat[0].clone();
        d.numero = number;
        d.emit = 0;
        d.kuljettaja = format!("Ääkkönen {number}");
        if number > 70 {
            d.luokka = "B".into();
        }
        p.kilpailijat.push(d);
    }
    let archive = application::calculated_project(&p).unwrap();
    for kind in [Kind::Classes, Kind::Stations] {
        let doc = printing::classes(&archive, &["A".into(), "B".into()], kind, true, true).unwrap();
        assert_eq!(doc.pages.len(), 2);
        assert!(doc.landscape);
        #[cfg(feature = "desktop")]
        if let Some(root) = std::env::var_os("AS2026_PRINT_PREVIEW_DIR") {
            let root = PathBuf::from(root);
            save_preview(
                &root,
                if kind == Kind::Stations {
                    "tihea-asemat"
                } else {
                    "tihea-luokat"
                },
                &doc,
            );
        }
        for page in doc.pages {
            for text in page.texts {
                assert!(text.x >= 10. && text.x + text.width <= 287.01);
                assert!(text.y >= 10. && text.y + text.height <= 200.01);
                assert!(text.size > 0.);
            }
        }
    }
    let one = printing::classes(&archive, &["B".into()], Kind::Classes, false, true).unwrap();
    assert_eq!(one.pages.len(), 1);
    assert!(!one.landscape);
    assert!(
        !one.pages[0]
            .texts
            .iter()
            .any(|t| t.value == "Esimerkkikuljettaja")
    );
    let paginated =
        printing::classes(&archive, &["A".into()], Kind::Classes, false, false).unwrap();
    assert!(paginated.pages.len() > 1);
    assert!(printing::classes(&archive, &[], Kind::Classes, false, true).is_err());
    assert!(printing::classes(&archive, &["puuttuu".into()], Kind::Classes, false, true).is_err());
}
#[test]
fn postscript_escapes_competitor_text_and_keeps_finnish_letters() {
    let mut f = Fixture::new();
    f.ws.complete_result_entry(7, &f.clocks(), 0, 0).unwrap();
    let mut p = f.ws.project().unwrap().clone();
    p.kilpailijat[0].kuljettaja = "Ää (testi) \\ showpage".into();
    let doc = printing::competitor(&p, 7, None).unwrap();
    let ps = printing::postscript(&doc);
    assert!(ps.contains("\\304\\344 \\(testi\\) \\\\ showpage"));
    assert_eq!(ps.matches("grestore showpage").count(), 1);
}
#[cfg(feature = "desktop")]
#[test]
fn selected_printer_is_user_specific_and_survives_reopening() {
    use as2026::preferences::PreferencesStore;
    let f = Fixture::new();
    let path = f.directory.join("asetukset.toml");
    let root = f.directory.join("Kilpailut");
    let mut store = PreferencesStore::open_at(&path, &root).unwrap();
    store.save_printer("Testitulostin Ää").unwrap();
    assert_eq!(
        PreferencesStore::open_at(&path, &root)
            .unwrap()
            .get()
            .tulostin,
        "Testitulostin Ää"
    );
    assert!(store.save_printer("virhe\nnimi").is_err());
    store.save_printer("").unwrap();
    assert!(
        PreferencesStore::open_at(&path, &root)
            .unwrap()
            .get()
            .tulostin
            .is_empty()
    );
}
