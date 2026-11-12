#![cfg(feature = "desktop")]
use as2026::{
    application, interpretation, live_pages,
    live_publish::{Config as PublishConfig, Publisher},
    native::Project,
    storage::ProjectStore,
    tulkki_yhteys::{self, ChangePage, Config},
    workspace::Workspace,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn receive(
    ws: &mut Workspace,
    c: &mut Config,
    id: u64,
    section: &str,
    missing: &[&str],
    times: Value,
    other: u32,
) {
    let signs = missing
        .iter()
        .filter(|s| matches!(**s, "r1-e1" | "r1-e5"))
        .count();
    let event=serde_json::from_value(json!({"eventId":id,"pairNumber":7,"revision":c.revision,"reviewer":"Live-koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":section,"missingEntryIds":missing,"times":times,"note":"Synteettinen live-koe","otherErrors":other,"missingSigns":signs,"missingTimes":missing.len()-signs,"interpreted":true}]})).unwrap();
    tulkki_yhteys::apply_page(
        c,
        ChangePage {
            events: vec![event],
            next_cursor: id,
            has_more: false,
        },
    )
    .unwrap();
    ws.apply_tulkki_penalties(c).unwrap();
}
fn row(html: &str) -> &str {
    html.split("<tbody>")
        .nth(1)
        .unwrap()
        .split("</tbody>")
        .next()
        .unwrap()
}
fn publish(
    publisher: &Publisher,
    ws: &Workspace,
    path: &Path,
    output: &Path,
    label: &str,
    expected: &str,
) -> Duration {
    let previous = fs::metadata(path).and_then(|m| m.modified()).ok();
    let start = Instant::now();
    publisher.offer(ws.project().unwrap());
    loop {
        let fresh = fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .is_some_and(|t| Some(t) != previous);
        if fresh
            && let Ok(html) = fs::read_to_string(path)
            && row(&html).contains(expected)
        {
            fs::write(output.join(format!("{label}.html")), &html).unwrap();
            fs::write(
                output.join(format!("{label}.toml")),
                ws.project().unwrap().to_toml().unwrap(),
            )
            .unwrap();
            println!(
                "{label}: {} ms, {}",
                start.elapsed().as_millis(),
                row(&html)
            );
            return start.elapsed();
        }
        assert!(
            start.elapsed() < Duration::from_secs(12),
            "{label}: {}",
            publisher.status()
        );
        thread::sleep(Duration::from_millis(25));
    }
}
fn score(ws: &Workspace) -> String {
    let a = application::calculated_project(ws.project().unwrap()).unwrap();
    format!(">{}</td>", a.competitors[0].total_penalties)
}
#[test]
fn phone_interpretation_approval_correction_and_void_update_the_same_live_publisher() {
    let output = std::env::var_os("AS2026_LIVE_TULKKI_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "as2026-live-tulkki-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
        });
    fs::create_dir_all(&output).unwrap();
    let mut p = Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    p.kilpailu.nimi = "AS tulkki – reaaliaikainen tuloskoe".into();
    p.kilpailu.ajanottotapa = 0;
    p.leimaluvut.clear();
    p.kilpailijat[0].emit = 0;
    p.kilpailijat[0].emit_korjaus_sekuntia = 0;
    p.kilpailijat[0].kuljettaja = "Koekuljettaja".into();
    p.kilpailijat[0].kartanlukija = "Koekartanlukija".into();
    let old = p.radat[0].asemat.clone();
    let mut jat = old[2].clone();
    jat.tyyppi = "JAT".into();
    jat.nimi = "JAT live-koe".into();
    let mut ula = jat.clone();
    ula.tyyppi = "ULA".into();
    ula.tavoiteaika_sekuntia = 0;
    p.radat[0].asemat = vec![
        old[0].clone(),
        old[1].clone(),
        jat,
        ula,
        old[2].clone(),
        old[3].clone(),
    ];
    let rows = ["08", "AT1", "JAT3", "ULA3", "15", "AT2", "M"]
        .into_iter()
        .enumerate()
        .map(|(i, s)| (i as u32 + 1, s.to_owned()))
        .collect::<Vec<_>>();
    interpretation::import_route(
        &mut p,
        1,
        &interpretation::preview_rows("Syotto_Live", &rows),
    )
    .unwrap();
    let mut c = Config {
        competition_id: 7,
        revision: 1,
        published_manifest: tulkki_yhteys::manifest(&p).unwrap().to_string(),
        ..Config::default()
    };
    let project_path = output.join("koekilpailu.toml");
    if project_path.exists() {
        panic!("Koekansion pitää olla uusi: {}", output.display());
    }
    ProjectStore::create(&project_path, p).unwrap();
    let mut ws = Workspace::open(&project_path).unwrap();
    let filename = "testitulokset.html";
    let path = output.join(filename);
    let publisher = Publisher::start(
        PublishConfig {
            filename: filename.into(),
            local_directory: output.display().to_string(),
            local_only: true,
            classes: vec!["A".into()],
            refresh_seconds: 5,
            ..PublishConfig::default()
        },
        project_path,
    )
    .unwrap();
    let mut log = Vec::new();
    log.push((
        "01-ei-tulkattu",
        publish(&publisher, &ws, &path, &output, "01-ei-tulkattu", "Jakso 1"),
    ));
    assert!(publisher.finish(ws.project().unwrap()).is_err());
    receive(
        &mut ws,
        &mut c,
        1,
        "r1-j1",
        &["r1-e1"],
        json!({"r1-e2":"12:05:00","r1-e3":"12:10:00","r1-e4":"12:12:00"}),
        0,
    );
    assert_eq!(
        ws.project().unwrap().kilpailijat[0]
            .lisavirheet
            .puuttuvat_rt_kpl,
        1
    );
    assert!(
        !ws.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .can_approve()
    );
    log.push((
        "02-jakso-1-tulkattu",
        publish(
            &publisher,
            &ws,
            &path,
            &output,
            "02-jakso-1-tulkattu",
            "Jakso 2",
        ),
    ));
    receive(
        &mut ws,
        &mut c,
        2,
        "r1-j2",
        &["r1-e5"],
        json!({"r1-e6":"12:22:00","r1-e7":"12:22:00"}),
        1,
    );
    log.push((
        "03-koko-kisa-tulkattu",
        publish(
            &publisher,
            &ws,
            &path,
            &output,
            "03-koko-kisa-tulkattu",
            "Maalissa",
        ),
    ));
    assert!(publisher.finish(ws.project().unwrap()).is_err());
    ws.approve_result(7).unwrap();
    let original = score(&ws);
    log.push((
        "04-hyvaksytty",
        publish(&publisher, &ws, &path, &output, "04-hyvaksytty", &original),
    ));
    receive(
        &mut ws,
        &mut c,
        3,
        "r1-j2",
        &[],
        json!({"r1-e6":"12:22:00","r1-e7":"12:22:00"}),
        0,
    );
    log.push((
        "05-korjaus-odottaa-hyvaksyntaa",
        publish(
            &publisher,
            &ws,
            &path,
            &output,
            "05-korjaus-odottaa-hyvaksyntaa",
            "Maalissa",
        ),
    ));
    ws.approve_result(7).unwrap();
    let corrected = score(&ws);
    assert_ne!(original, corrected);
    log.push((
        "06-korjaus-hyvaksytty",
        publish(
            &publisher,
            &ws,
            &path,
            &output,
            "06-korjaus-hyvaksytty",
            &corrected,
        ),
    ));
    assert_eq!(ws.void_right_row(1, "r1-e1", &c).unwrap(), vec![7]);
    let voided = score(&ws);
    assert_ne!(corrected, voided);
    assert!(
        ws.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    log.push((
        "07-kilpi-mitatoity",
        publish(
            &publisher,
            &ws,
            &path,
            &output,
            "07-kilpi-mitatoity",
            &voided,
        ),
    ));
    c.published_manifest = tulkki_yhteys::manifest(ws.project().unwrap())
        .unwrap()
        .to_string();
    c.scoring_revision = 1;
    c.revision = 2;
    receive(
        &mut ws,
        &mut c,
        4,
        "r1-j1",
        &["r1-e3"],
        json!({"r1-e2":"12:05:00","r1-e4":"12:12:00"}),
        0,
    );
    log.push((
        "08-jat-puuttuu-odottaa-paatosta",
        publish(
            &publisher,
            &ws,
            &path,
            &output,
            "08-jat-puuttuu-odottaa-paatosta",
            "Maalissa",
        ),
    ));
    ws.approve_result(7).unwrap();
    log.push((
        "09-keskeytys-vahvistettu",
        publish(
            &publisher,
            &ws,
            &path,
            &output,
            "09-keskeytys-vahvistettu",
            "KESK",
        ),
    ));
    publisher.finish(ws.project().unwrap()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(12);
    while !publisher.is_finished() {
        assert!(Instant::now() < deadline, "{}", publisher.status());
        thread::sleep(Duration::from_millis(25));
    }
    let final_html = fs::read_to_string(&path).unwrap();
    assert!(final_html.contains("Lopulliset tulokset"));
    assert!(row(&final_html).contains("KESK"));
    fs::write(output.join("10-lopulliset.html"), &final_html).unwrap();
    let pages = live_pages::pages(
        ws.project().unwrap(),
        &live_pages::Settings {
            base: PathBuf::from(filename),
            classes: vec!["A".into()],
        },
    )
    .unwrap();
    assert!(pages[0].1.contains("60;url="));
    assert!(final_html.contains("5;url="));
    fs::write(
        output.join("koetulokset.txt"),
        log.iter()
            .map(|(s, t)| format!("{s}: {} ms", t.as_millis()))
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    println!("Koetulokset: {}", output.display());
}
