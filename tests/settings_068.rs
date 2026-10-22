//! Version 0.6.8 settings: persisted units, boundaries and backwards compatibility.
use as2026::{application, emit_corrections, native::Project, starts};

fn example() -> Project {
    Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}

#[test]
fn configurable_start_columns_share_export_order_and_escape_content() {
    let mut p = example();
    p.virhepisteet.lahtoluettelon_kentat = vec!["kartturin_seura".into(), "numero".into()];
    p.kilpailijat[0].kartanlukijan_seura = "<Seura> & \"nimi\"".into();
    let csv = starts::start_list_csv(&p).unwrap();
    assert!(csv.starts_with("\"Kartturin seura\";\"Nro\"\r\n"));
    let html = starts::start_list_html(&p).unwrap();
    assert!(html.contains("<th>Kartturin seura</th><th>Nro</th>"));
    assert!(html.contains("&lt;Seura&gt; &amp; &quot;nimi&quot;"));
    assert!(!html.contains("<th>Tila</th>"));
    let reopened = Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(starts::start_list_csv(&reopened).unwrap(), csv);
    p.virhepisteet.lahtoluettelon_kentat.push("numero".into());
    assert!(p.validate().is_err());
    p.virhepisteet.lahtoluettelon_kentat = vec!["tuntematon".into()];
    assert!(p.validate().is_err());
}

#[test]
fn defaults_and_explicit_zero_reset_correction_survive_roundtrip() {
    let mut p = example();
    assert_eq!(p.kilpailijat[0].effective_emit_correction(), 120);
    p.kilpailijat[0].emit_korjaus_annettu = true;
    p.kilpailijat[0].emit_korjaus_sekuntia = 0;
    assert_eq!(emit_corrections::punch_clock(&p, 7, 0).unwrap(), 43200);
    let result = application::capture_project_result(&p, 7).unwrap();
    assert_eq!(result.rivit[1].clock, [12, 6, 35]);
    let reopened = Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(reopened.kilpailijat[0].effective_emit_correction(), 0);
    p.kilpailijat[0].lahtoaika = "00:01:00".into();
    p.kilpailijat[0].emit_korjaus_sekuntia = 120;
    assert_eq!(
        emit_corrections::clock_text(emit_corrections::punch_clock(&p, 7, 0).unwrap()),
        "23:59:00"
    );
    p.kilpailijat[0].emit_korjaus_sekuntia = -30;
    assert_eq!(emit_corrections::punch_clock(&p, 7, 0).unwrap(), 90);
}

#[test]
fn total_lateness_setting_is_minutes_and_independent_of_tolerance_and_section_limit() {
    let mut p = example();
    p.virhepisteet.kokonaismyohastymisraja_minuutteina = Some(1);
    p.virhepisteet.jakson_raja_minuutteina = Some(60);
    let archive = p.to_archive().unwrap();
    assert_eq!(archive.basic.classes[0].fields[7], "1");
    assert_eq!(archive.scoring_parameters().unwrap().tolerance_seconds, 10);
    for (seconds, status) in [(60, "TRK"), (61, "KESK")] {
        let mut result = application::capture_project_result(&p, 7).unwrap();
        let finish = result
            .rivit
            .iter()
            .find(|r| r.station_type == "M")
            .unwrap()
            .output_index;
        result.set_clock(finish, [12, 11, seconds - 60]).unwrap();
        let result = application::recalculate_stored_result(&p, 7, &result).unwrap();
        assert_eq!(result.tila, status);
        if status == "KESK" {
            assert!(result.syy.contains("kokonaismyöhästymisraja"));
        }
    }
    p.virhepisteet.kokonaismyohastymisraja_minuutteina = None;
    assert_eq!(p.to_archive().unwrap().basic.classes[0].fields[7], "60");
    p.virhepisteet.kokonaismyohastymisraja_minuutteina = Some(-1);
    assert!(p.validate().is_err());
}
