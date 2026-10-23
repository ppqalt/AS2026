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
fn local_pages_replace_contents_without_touching_a_previous_fixed_temporary_name() {
    let directory = std::env::temp_dir().join(format!(
        "as2026-live-sivut-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let legacy_temp = directory.join("omat.html.as2026-tmp");
    std::fs::write(&legacy_temp, "säilytettävä vieras tiedosto").unwrap();
    let paths = [directory.join("omat.html"), directory.join("omat1.html")];
    for value in ["ensimmäinen tilanne", "korjattu tilanne"] {
        live_pages::write_pages(
            &paths
                .iter()
                .map(|path| (path.clone(), value.into()))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        for path in &paths {
            assert_eq!(std::fs::read_to_string(path).unwrap(), value);
        }
    }
    assert_eq!(
        std::fs::read_to_string(legacy_temp).unwrap(),
        "säilytettävä vieras tiedosto"
    );
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 3);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_local_page_replace_removes_only_its_own_temporary_file() {
    let directory = std::env::temp_dir().join(format!(
        "as2026-live-virhe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let destination = directory.join("omat.html");
    std::fs::create_dir_all(&destination).unwrap();
    std::fs::write(destination.join("säilytä.txt"), "aiempi sisältö").unwrap();
    assert!(live_pages::write_pages(&[(destination.clone(), "uusi tilanne".into())]).is_err());
    assert_eq!(
        std::fs::read_to_string(destination.join("säilytä.txt")).unwrap(),
        "aiempi sisältö"
    );
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    std::fs::remove_dir_all(directory).unwrap();
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

#[cfg(feature = "desktop")]
#[test]
#[ignore = "requires an isolated loopback FTP/SFTP server"]
fn native_transport_replaces_three_class_pages_after_a_saved_correction() {
    use as2026::live_publish::{Config, Publisher};
    use std::{
        fs, thread,
        time::{Duration, Instant},
    };
    let path = std::env::var_os("AS2026_NATIVE_TRANSFER_CONFIG").unwrap();
    let mut config: Config = toml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(config.host, "127.0.0.1");
    config.password = "test".into();
    config.filename = "omat.html".into();
    let remote = PathBuf::from(std::env::var_os("AS2026_NATIVE_TRANSFER_REMOTE").unwrap());
    as2026::live_publish::transfer(&config, None).unwrap();
    let publisher = Publisher::start(config, "synthetic.toml".into()).unwrap();
    let mut p = project();
    for name in ["Ensimmäinen julkaisu", "Tallennettu korjaus"] {
        p.kilpailu.nimi = name.into();
        publisher.offer(&p);
        let deadline = Instant::now() + Duration::from_secs(25);
        loop {
            if ["omat.html", "omat1.html", "omat2.html"]
                .iter()
                .all(|f| fs::read_to_string(remote.join(f)).is_ok_and(|s| s.contains(name)))
            {
                break;
            }
            assert!(Instant::now() < deadline, "{}", publisher.status());
            thread::sleep(Duration::from_millis(100));
        }
    }
}
