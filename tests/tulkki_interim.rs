#![cfg(feature = "desktop")]
use as2026::{
    application, interpretation,
    native::Project,
    storage::ProjectStore,
    tulkki_yhteys::{self, ChangePage, Config},
    workspace::Workspace,
};
use serde_json::json;

#[test]
fn phone_times_are_provisional_and_emit_readout_wins_without_reinterpreting() {
    let mut project =
        Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    let reads = std::mem::take(&mut project.leimaluvut);
    let preview = interpretation::preview_rows(
        "Koe",
        &[(1, "AT31".into()), (2, "AT32".into()), (3, "M".into())],
    );
    interpretation::replace_route(&mut project, 1, &preview).unwrap();
    for entry in &mut project.oikeat_rivit[0].merkinnat {
        entry.kasiaika = Some(true);
    }
    let mut config = Config {
        competition_id: 1,
        revision: 1,
        published_manifest: tulkki_yhteys::manifest(&project).unwrap().to_string(),
        ..Config::default()
    };
    let event = serde_json::from_value(json!({"eventId":1,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-19T12:00:00Z",
        "sections":[{"id":"r1-j1","missingEntryIds":[],"times":{"r1-e1":"12:05:00","r1-e2":"12:10:00","r1-e3":"12:15:00"},
        "note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
    tulkki_yhteys::apply_page(
        &mut config,
        ChangePage {
            events: vec![event],
            next_cursor: 1,
            has_more: false,
        },
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!(
        "as2026-interim-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("kilpailu.toml");
    drop(ProjectStore::create(&path, project).unwrap());
    let mut ws = Workspace::open(&path).unwrap();
    assert!(ws.apply_tulkki_penalties(&config).unwrap());
    let interim = ws.result_snapshot(7).unwrap().unwrap();
    assert!(interim.tulkin_valiaikatulos);
    assert!(!interim.can_approve());
    config.automatic_results = true;
    assert!(tulkki_yhteys::automatic_result_key(ws.project().unwrap(), &config, 7).is_none());
    assert_eq!(interim.rivit[1].clock, [12, 5, 0]);
    let pages = as2026::live_pages::pages(
        ws.project().unwrap(),
        &as2026::live_pages::Settings {
            base: dir.join("tulokset.html"),
            classes: vec!["A".into()],
        },
    )
    .unwrap();
    assert!(
        pages.iter().any(|(_, html)| html.contains("Maalissa")),
        "Puhelimen valmis väliaikatulos näkyy livetuloksissa"
    );
    assert!(
        !application::calculated_project(ws.project().unwrap())
            .unwrap()
            .competitors[0]
            .results
            .is_empty()
    );
    assert!(
        !ws.apply_tulkki_penalties(&config).unwrap(),
        "Sama vastaanotto ei toista muutosta"
    );
    drop(ws);
    let mut store = ProjectStore::open(&path).unwrap();
    store
        .update(|p| {
            p.leimaluvut = reads;
            Ok(())
        })
        .unwrap();
    drop(store);
    let mut ws = Workspace::open(&path).unwrap();
    let expected = application::capture_project_result(ws.project().unwrap(), 7).unwrap();
    let final_result = ws.result_snapshot(7).unwrap().unwrap();
    assert!(!final_result.tulkin_valiaikatulos);
    assert_eq!(final_result.rivit, expected.rivit);
    assert_ne!(final_result.rivit[1].clock, interim.rivit[1].clock);
    assert!(ws.apply_tulkki_penalties(&config).unwrap());
    assert!(
        !ws.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .tulkin_valiaikatulos
    );
    assert!(
        !ws.apply_tulkki_penalties(&config).unwrap(),
        "Puhelin ei korvaa Emit-aikoja"
    );
    let mut later = config.history[0].clone();
    later.event_id = 2;
    later.sections[0]
        .times
        .insert("r1-e1".into(), "12:06:00".into());
    tulkki_yhteys::apply_page(
        &mut config,
        ChangePage {
            events: vec![later],
            next_cursor: 2,
            has_more: false,
        },
    )
    .unwrap();
    ws.apply_tulkki_penalties(&config).unwrap();
    assert_eq!(
        ws.result_snapshot(7).unwrap().unwrap().rivit,
        expected.rivit,
        "Myöhempi korjattu puhelinaika ei korvaa Emit-aikoja"
    );
    drop(ws);
    std::fs::remove_dir_all(dir).unwrap();
}
