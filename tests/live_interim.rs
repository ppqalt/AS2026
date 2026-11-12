#![cfg(feature = "desktop")]
use as2026::{
    application, interpretation,
    live_pages::{self, Settings},
    native::Project,
    tulkki_yhteys::{self, ChangePage, Config},
};
use serde_json::{Value, json};
fn setup() -> (Project, Config) {
    let mut p = Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    p.kilpailu.ajanottotapa = 0;
    p.leimaluvut.clear();
    p.kilpailijat[0].emit = 0;
    p.kilpailijat[0].emit_korjaus_sekuntia = 0;
    let old = p.radat[0].asemat.clone();
    let mut jat = old[2].clone();
    jat.tyyppi = "JAT".into();
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
    let rows = ["08", "AT", "JAT", "ULA", "15", "AT", "M"]
        .into_iter()
        .enumerate()
        .map(|(i, s)| (i as u32 + 1, s.to_owned()))
        .collect::<Vec<_>>();
    interpretation::import_route(
        &mut p,
        1,
        &interpretation::preview_rows("Syotto_live", &rows),
    )
    .unwrap();
    let c = Config {
        competition_id: 7,
        revision: 1,
        published_manifest: tulkki_yhteys::manifest(&p).unwrap().to_string(),
        ..Config::default()
    };
    (p, c)
}
fn receive(p: &mut Project, c: &mut Config, id: u64, j: u32, missing: Vec<&str>, times: Value) {
    let event=serde_json::from_value(json!({"eventId":id,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":format!("r1-j{j}"),"missingEntryIds":missing,"times":times,"note":"","otherErrors":0,"missingSigns":missing.len(),"missingTimes":0,"interpreted":true}]})).unwrap();
    tulkki_yhteys::apply_page(
        c,
        ChangePage {
            events: vec![event],
            next_cursor: id,
            has_more: false,
        },
    )
    .unwrap();
    let mut result = as2026::manual_timing::capture(p, 7).unwrap();
    for receipt in &c.received {
        for (id, value) in &receipt.section.times {
            let row = match id.as_str() {
                "r1-e2" => 2,
                "r1-e3" => 3,
                "r1-e4" => 4,
                "r1-e6" => 5,
                "r1-e7" => 6,
                _ => panic!("Tuntematon testiaika"),
            };
            let parts = value
                .split(':')
                .map(|s| s.parse::<i64>().unwrap())
                .collect::<Vec<_>>();
            result
                .set_clock(row, [parts[0], parts[1], parts[2]])
                .unwrap();
        }
    }
    let result = application::recalculate_stored_result(p, 7, &result).unwrap();
    let changes = tulkki_yhteys::penalty_changes(p, c).unwrap();
    for (n, penalties) in changes {
        p.kilpailijat
            .iter_mut()
            .find(|d| d.numero == n)
            .unwrap()
            .lisavirheet = penalties;
    }
    p.kilpailijat[0].tallennettu_tulos = Some(result);
}
fn page(p: &Project) -> String {
    live_pages::pages(
        p,
        &Settings {
            base: "koe.html".into(),
            classes: vec!["A".into()],
        },
    )
    .unwrap()
    .remove(0)
    .1
}
fn cells(html: &str, number: u32) -> Vec<String> {
    html.split("<tr>")
        .find(|row| row.contains(&format!("</td><td>{number}</td>")))
        .unwrap()
        .split("<td")
        .skip(1)
        .map(|cell| {
            cell.split_once('>')
                .unwrap()
                .1
                .split("</td>")
                .next()
                .unwrap()
                .to_owned()
        })
        .collect()
}
#[test]
fn initially_blank_then_partial_points_and_next_section_without_approval() {
    let (mut p, mut c) = setup();
    let row = cells(&page(&p), 7);
    assert_eq!(row[0], "");
    assert_eq!(row[4], "Jakso 1");
    assert_eq!(row[8], "");
    receive(
        &mut p,
        &mut c,
        1,
        1,
        vec!["r1-e1"],
        json!({"r1-e2":"12:04:30","r1-e3":"12:10:00","r1-e4":"12:12:00"}),
    );
    let mut future = p.kilpailijat[0].tallennettu_tulos.clone().unwrap();
    future.set_clock(5, [12, 24, 0]).unwrap();
    future.set_clock(6, [12, 24, 0]).unwrap();
    future = application::recalculate_stored_result(&p, 7, &future).unwrap();
    assert!(
        future
            .rivit
            .iter()
            .any(|r| r.output_index == 5 && r.total_penalty > 0)
    );
    p.kilpailijat[0].tallennettu_tulos = Some(future);
    let before = p.clone();
    let html = page(&p);
    let row = cells(&html, 7);
    assert_eq!(row[0], "1");
    assert_eq!(row[4], "Jakso 2");
    // Yksi RT 600 + AT:n 30 s ennakko, 10 s toleranssi, 3 vp/s = 60.
    // Myöhemmän, tulkkaamattoman jakson kellot eivät kuulu välipisteisiin.
    assert_eq!(row[8], "660");
    assert_eq!(&row[5..8], &["600", "60", "0"]);
    assert_eq!(p, before);
    assert!(
        !p.kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    assert!(html.contains("<th>Tilanne</th>"));
    assert!(html.contains("<th class=\"numeric\">Yhteensä</th>"));
    assert!(!html.contains(">AJAA</td>"));
}
#[test]
fn full_unapproved_result_has_points_and_finished_status() {
    let (mut p, mut c) = setup();
    receive(
        &mut p,
        &mut c,
        1,
        1,
        vec![],
        json!({"r1-e2":"12:05:00","r1-e3":"12:10:00","r1-e4":"12:12:00"}),
    );
    receive(
        &mut p,
        &mut c,
        2,
        2,
        vec!["r1-e5"],
        json!({"r1-e6":"12:22:00","r1-e7":"12:22:00"}),
    );
    let row = cells(&page(&p), 7);
    assert_eq!(row[4], "Maalissa");
    assert_eq!(row[8], "600");
    assert_eq!(row[0], "1");
    assert!(
        !p.kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    let official = application::calculated_project(&p).unwrap();
    let ranked = application::rank_stored(
        &official,
        as2026::ranking::TiePolicy::PairwiseControlComparison,
    )
    .unwrap();
    assert!(
        ranked.groups[0]
            .entries
            .iter()
            .all(|e| e.status != as2026::ranking::Status::Classified)
    );
}
#[test]
fn out_of_order_interpretation_does_not_claim_earlier_section_is_done() {
    let (mut p, mut c) = setup();
    receive(
        &mut p,
        &mut c,
        1,
        2,
        vec![],
        json!({"r1-e6":"12:22:00","r1-e7":"12:22:00"}),
    );
    let row = cells(&page(&p), 7);
    assert_eq!(row[4], "Jakso 1");
    assert!(!row[8].is_empty());
}
#[test]
fn provisional_positions_include_all_scored_pairs_but_not_unstarted_pairs() {
    let (mut p, mut c) = setup();
    receive(
        &mut p,
        &mut c,
        1,
        1,
        vec!["r1-e1"],
        json!({"r1-e2":"12:05:00","r1-e3":"12:10:00","r1-e4":"12:12:00"}),
    );
    let mut second = p.kilpailijat[0].clone();
    second.numero = 8;
    second.lisavirheet.puuttuvat_rt_kpl = 0;
    second.lisavirheet.puuttuvat_rt_pisteet = 0;
    second
        .lisavirheet
        .tulkki_siirto
        .as_mut()
        .unwrap()
        .puuttuvat_rt
        .clear();
    p.kilpailijat.push(second);
    let mut third = p.kilpailijat[0].clone();
    third.numero = 9;
    third.lisavirheet = Default::default();
    third.tallennettu_tulos = None;
    p.kilpailijat.push(third);
    let html = page(&p);
    assert_eq!(cells(&html, 8)[0], "1");
    assert_eq!(cells(&html, 8)[8], "0");
    assert_eq!(cells(&html, 7)[0], "2");
    assert_eq!(cells(&html, 9)[0], "");
    assert_eq!(cells(&html, 9)[8], "");
}
#[test]
fn terminal_status_is_separate_from_points_and_has_no_numeric_position() {
    let (mut p, mut c) = setup();
    receive(
        &mut p,
        &mut c,
        1,
        1,
        vec!["r1-e1"],
        json!({"r1-e2":"12:05:00","r1-e3":"12:10:00","r1-e4":"12:12:00"}),
    );
    p.kilpailijat[0].tila =
        Some(serde_json::from_value(json!({"tunnus":"HYL","syy":"Synteettinen päätös"})).unwrap());
    let row = cells(&page(&p), 7);
    assert_eq!(row[0], "");
    assert_eq!(row[4], "HYL");
    assert_eq!(row[8], "");
}
#[test]
fn equal_provisional_totals_share_position() {
    let (mut p, mut c) = setup();
    receive(
        &mut p,
        &mut c,
        1,
        1,
        vec![],
        json!({"r1-e2":"12:05:00","r1-e3":"12:10:00","r1-e4":"12:12:00"}),
    );
    let mut other = p.kilpailijat[0].clone();
    other.numero = 8;
    p.kilpailijat.push(other);
    let html = page(&p);
    assert_eq!(cells(&html, 7)[0], "1");
    assert_eq!(cells(&html, 8)[0], "1");
    assert_eq!(cells(&html, 7)[8], "0");
}
#[test]
fn all_nonzero_point_components_sum_to_total_before_approval() {
    let (mut p, mut c) = setup();
    receive(
        &mut p,
        &mut c,
        1,
        1,
        vec!["r1-e1"],
        json!({"r1-e2":"12:04:30","r1-e3":"12:10:00","r1-e4":"12:12:00"}),
    );
    p.kilpailijat[0].lisavirheet.muut_virhepisteet = 600;
    let row = cells(&page(&p), 7);
    assert_eq!(&row[5..9], &["600", "60", "600", "1260"]);
}
