use as2026::{
    application,
    live_pages::{self, Settings},
    native::Project,
};
use std::path::PathBuf;

fn project() -> Project {
    let mut project =
        Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    for code in ["B", "C"] {
        let mut class = project.luokat[0].clone();
        class.tunnus = code.into();
        project.luokat.push(class);
    }
    project
}
#[test]
fn three_class_pages_cycle_and_exclude_diagnostics_and_series_points() {
    let mut p = project();
    p.kilpailu.nimi = "Testi <script>".into();
    let config = Settings {
        base: PathBuf::from("oma tulos.html"),
        classes: vec!["A".into(), "B".into(), "C".into()],
    };
    let pages = live_pages::pages(&p, &config).unwrap();
    assert_eq!(pages.len(), 3);
    for (i, next) in ["oma%20tulos1.html", "oma%20tulos2.html", "oma%20tulos.html"]
        .iter()
        .enumerate()
    {
        assert!(pages[i].1.contains(&format!("content=\"60;url={next}\"")));
        assert!(pages[i].1.contains("Testi &lt;script&gt;"));
        for forbidden in [
            "Huomautukset",
            "nousupisteet",
            "sarjapisteet",
            ">Np<",
            ">Sp<",
            "Laskentaa ei voitu",
        ] {
            assert!(!pages[i].1.contains(forbidden));
        }
    }
    assert_eq!(pages[0].0, PathBuf::from("oma tulos.html"));
    assert_eq!(pages[2].0, PathBuf::from("oma tulos2.html"));
}
#[test]
fn selected_single_class_refreshes_itself_and_empty_selection_is_rejected() {
    let p = project();
    let mut config = Settings {
        base: "omat.html".into(),
        classes: vec!["B".into()],
    };
    let pages = live_pages::pages(&p, &config).unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].1.contains("60;url=omat.html"));
    assert!(pages[0].1.contains("Luokka: B"));
    assert!(!pages[0].1.contains("Luokka: A"));
    config.classes.clear();
    assert!(live_pages::pages(&p, &config).is_err());
}
#[test]
fn blank_section_uses_operational_target_and_sums_multiple_sections() {
    let p = project();
    let mut result = application::capture_project_result(&p, 7).unwrap();
    let finish = result
        .rivit
        .iter_mut()
        .find(|r| r.station_type == "M")
        .unwrap();
    finish.section_text.clear();
    finish.expected_code = 0;
    finish.actual_seconds = 660;
    finish.target_seconds = 600;
    let index = finish.output_index;
    result.kasin_annetut_kellot.push(index);
    let finish = result
        .rivit
        .iter()
        .find(|r| r.output_index == index)
        .unwrap()
        .clone();
    assert_eq!(result.section_difference(&finish), Some(-60));
    let mut jat = finish.clone();
    jat.output_index = 99;
    jat.station_type = "JAT".into();
    jat.actual_seconds = 630;
    result.kasin_annetut_kellot.push(99);
    result.rivit.push(jat);
    assert_eq!(result.cumulative_schedule_difference(), -90);
}
