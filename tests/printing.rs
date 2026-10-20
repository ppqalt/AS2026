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
    assert!(doc.pages[0].texts.iter().any(|t| t.value == "12:05:00"));
    #[cfg(feature = "desktop")]
    if let Some(root) = std::env::var_os("AS2026_PRINT_PREVIEW_DIR") {
        let root = PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("kilpailija.json"),
            serde_json::to_vec(&doc).unwrap(),
        )
        .unwrap();
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
            std::fs::write(
                root.join(format!("{name}.json")),
                serde_json::to_vec(&doc).unwrap(),
            )
            .unwrap();
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
