//! 72 täysin synteettistä paria: sama tallennus- ja laskentapolku kuin GUI:ssa.
//! Kehykset simuloidaan; testi ei avaa Emit-laitetta tai fyysistä tulostinta.
use as2026::{
    ClockTime, beginners, emit, excel_import, native, printing, ranking, report, starts,
    terminal::parse_clock,
    workspace::{DriverField, Workspace},
};
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

struct Directory {
    path: PathBuf,
    keep: bool,
}
impl Directory {
    fn new() -> Self {
        let output = std::env::var_os("AS2026_COMPETITION_DAY_OUTPUT");
        let keep = output.is_some();
        let path = output.map_or_else(
            || {
                std::env::temp_dir().join(format!(
                    "as2026-paiva070-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ))
            },
            PathBuf::from,
        );
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::create_dir(&path).unwrap();
        Self { path, keep }
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
fn clock(start: &str, seconds: u32) -> ClockTime {
    let value = (parse_clock(start)
        .unwrap()
        .seconds_since_midnight()
        .unwrap() as u32
        + seconds)
        % 86400;
    ClockTime::new(value / 3600, value / 60 % 60, value % 60).unwrap()
}
fn frame(card: u32, manual: bool) -> emit::EmitFrame {
    let mut decoded = [0u8; emit::FRAME_LENGTH];
    decoded[..2].copy_from_slice(&[255, 255]);
    decoded[2..5].copy_from_slice(&card.to_le_bytes()[..3]);
    let mut punches = vec![(0u8, 0u16), (31, 300)];
    if !manual {
        punches.push((32, 600));
    }
    punches.push((250, 660));
    for (i, (code, seconds)) in punches.into_iter().enumerate() {
        let offset = 10 + i * 3;
        decoded[offset] = code;
        decoded[offset + 1..offset + 3].copy_from_slice(&seconds.to_le_bytes());
    }
    emit::parse_frame(&decoded.map(|byte| byte ^ emit::XOR_MASK)).unwrap()
}
fn expected(number: u32) -> i64 {
    i64::from(number) * 2 + if number.is_multiple_of(6) { 600 } else { 0 }
}

#[test]
fn seventy_two_pairs_finish_a_synthetic_competition_day() {
    let started = std::time::Instant::now();
    let input =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/excel-import/synteettinen.xlsx");
    let original_excel = fs::read(&input).unwrap();
    let mut project = excel_import::read(&input)
        .unwrap()
        .to_project("Synteettinen kilpailupäivä 0.7.0")
        .unwrap();
    let demo =
        native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    project.kilpailu.ajanottotapa = 0;
    project.leimaluvut.clear();
    project.yhdistetyt_luokat.clear();
    project.virhepisteet.puuttuva_rt = Some(600);
    let mut route = demo.radat[0].clone();
    route.asemat = vec![
        route.asemat[0].clone(),
        route.asemat[1].clone(),
        route.asemat[3].clone(),
    ];
    route.asemat[1].saanto = 0;
    route.asemat[2].emit_koodi = 32;
    route.asemat[2].saanto = 0;
    route.asemat[2].tavoiteaika_sekuntia = 600;
    route.asemat[2].jakso_minuutteina = Some(10);
    route.nimi = "Leimallinen maali".into();
    let mut manual = route.clone();
    manual.numero = 2;
    manual.nimi = "Käsimaali".into();
    manual.asemat[2].emit_koodi = 0;
    project.radat = vec![route, manual];
    let class = demo.luokat[0].clone();
    project.luokat = (0..3)
        .map(|i| {
            let mut c = class.clone();
            c.tunnus = ["A", "B", "C"][i].into();
            c.nimi = format!("Synteettinen {}", c.tunnus);
            c.rata = if i == 2 { 2 } else { 1 };
            c.ensimmainen_numero = i as u32 * 24 + 1;
            c.lahtoaika = format!("12:{i:02}:00");
            c.lahtovali_minuutteina = 3;
            c.osallistuu_lahtoarvontaan = true;
            c
        })
        .collect();
    let driver = demo.kilpailijat[0].clone();
    project.kilpailijat = (1..=72)
        .map(|number| {
            let mut d = driver.clone();
            d.numero = number;
            d.emit = 500_000 + number;
            d.luokka = ["A", "B", "C"][(number as usize - 1) / 24].into();
            d.kuljettaja = format!("Synteettinen {number:03}");
            d.kartanlukija = format!("Mallikartanlukija {number:03}");
            d.emit_korjaus_sekuntia = 0;
            d.emit_korjaus_annettu = true;
            d
        })
        .collect();
    project.yhdistetyt_luokat.push(native::CombinedClass {
        nimi: "A ja B".into(),
        luokat: vec!["A".into(), "B".into()],
        nousupisteet: 0,
        sarjapistetapa: 0,
    });
    let directory = Directory::new();
    let mut ws = Workspace::create_imported(&directory.path, project).unwrap();
    let draw = starts::draw(ws.project().unwrap(), 700).unwrap();
    assert_eq!(draw.assignments.len(), 72);
    ws.apply_start_draw(&draw).unwrap();
    let route = ws.project().unwrap().radat[0].clone();
    ws.save_beginners_cup(
        beginners::BeginnersCup {
            nimi: "Synteettinen Beginners Cup".into(),
            tunnus: "BC".into(),
            lahderata: 1,
            lahdeasemat: route.asemat,
            mukana_asemat: vec![1, 2, 3],
        },
        &(1..=12).collect::<Vec<_>>(),
    )
    .unwrap();
    for number in 1..=71 {
        if number == 70 {
            ws.confirm_interruption(number, "Synteettinen moottorivaurio")
                .unwrap();
            continue;
        }
        let driver = ws
            .project()
            .unwrap()
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .unwrap()
            .clone();
        let manual = driver.luokka == "C";
        let card = frame(driver.emit, manual);
        assert_eq!(
            ws.record_frame(&card, clock(&driver.lahtoaika, 660))
                .unwrap()
                .competitor_number,
            Some(number)
        );
        if number == 1 {
            assert!(
                !ws.record_frame(&card, clock(&driver.lahtoaika, 661))
                    .unwrap()
                    .content_changed
            );
        }
        ws.ensure_readout_result(number).unwrap();
        let result = ws.result_snapshot(number).unwrap().unwrap();
        let mut clocks = vec![];
        if manual {
            assert!(!result.pending_manual_clocks().is_empty());
            assert!(ws.approve_result(number).is_err());
            let row = result.rivit.iter().find(|r| r.station_type == "M").unwrap();
            let end = clock(&driver.lahtoaika, 600);
            clocks.push((
                row.output_index,
                [
                    i64::from(end.hour),
                    i64::from(end.minute),
                    i64::from(end.second),
                ],
            ));
        }
        ws.complete_result_entry(
            number,
            &clocks,
            u32::from(number.is_multiple_of(6)),
            i64::from(number) * 2,
        )
        .unwrap();
        assert!(ws.result_snapshot(number).unwrap().unwrap().hyvaksytty);
    }
    ws.edit_result_driver(71, DriverField::Status, "HYL")
        .unwrap();
    ws.edit_result_driver(71, DriverField::StatusReason, "Synteettinen väärä reitti")
        .unwrap();
    ws.approve_result(71).unwrap();
    assert!(ws.result_snapshot(72).unwrap().is_none());
    assert!(ws.approve_result(72).is_err());
    let nac = ws.results().unwrap();
    for number in (1..=69).chain([71]) {
        let driver = nac.competitors.iter().find(|d| d.number == number).unwrap();
        assert_eq!(
            driver.total_penalties,
            expected(number),
            "Pari {number}: vain 2 × numero ja joka kuudennelle 600 RT-pistettä"
        );
        assert_eq!(
            ws.result_snapshot(number).unwrap().unwrap().kesto_sekuntia,
            600
        );
    }
    let ranked = ws.ranking().unwrap();
    assert!(
        ranked
            .groups
            .iter()
            .flat_map(|g| &g.entries)
            .any(|e| e.number == 70 && e.status == ranking::Status::Interrupted)
    );
    assert!(
        ranked
            .groups
            .iter()
            .flat_map(|g| &g.entries)
            .any(|e| e.number == 71 && e.status == ranking::Status::Disqualified)
    );
    for group in &ranked.groups {
        let mut expected_order: Vec<_> = (1..=69)
            .filter(|&n| match group.kind {
                ranking::GroupKind::Combined => n <= 48,
                _ => ["A", "B", "C"][(n as usize - 1) / 24] == group.code,
            })
            .collect();
        expected_order.sort_by_key(|&n| expected(n));
        for (position, number) in expected_order.into_iter().enumerate() {
            assert_eq!(
                group
                    .entries
                    .iter()
                    .find(|e| e.number == i64::from(number))
                    .unwrap()
                    .primary_position,
                position as u32 + 1
            );
        }
    }
    let history = ws.project().unwrap().leimaluvut.clone();
    ws.set_bc_view(true).unwrap();
    for number in 1..=12 {
        ws.ensure_readout_result(number).unwrap();
        ws.complete_result_entry(number, &[], 0, i64::from(number) * 3)
            .unwrap();
    }
    ws.set_bc_view(false).unwrap();
    assert_eq!(
        report::ranking_csv(&ws.results().unwrap(), &ws.ranking().unwrap()),
        report::ranking_csv(&nac, &ranked)
    );
    let bc = beginners::results(ws.project().unwrap()).unwrap();
    assert_eq!(bc.competitors.len(), 12);
    for driver in &bc.competitors {
        assert_eq!(driver.total_penalties, i64::from(driver.number) * 3);
        assert!(bc.has_approved_result(driver.number));
    }
    let saved = ws.project().unwrap().clone();
    let reopened = Workspace::open(&directory.path).unwrap();
    assert_eq!(reopened.project(), Some(&saved));
    assert_eq!(reopened.project().unwrap().leimaluvut, history);
    let csv =
        report::competition_ranking_csv(&saved, ranking::TiePolicy::KeepSharedAndFlag).unwrap();
    let html =
        report::competition_ranking_html(&saved, ranking::TiePolicy::KeepSharedAndFlag).unwrap();
    let xlsx = report::xlsx_from_csv(&csv).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(&xlsx)).unwrap();
    let mut xml = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    for text in [
        "Synteettinen 001",
        "Synteettinen 069",
        "Synteettinen väärä reitti",
        "BC",
    ] {
        assert!(csv.contains(text));
        assert!(html.contains(text));
        assert!(xml.contains(text));
    }
    assert_eq!(
        report::competition_ranking_csv(
            reopened.project().unwrap(),
            ranking::TiePolicy::KeepSharedAndFlag
        )
        .unwrap(),
        csv
    );
    let print = printing::classes(
        &nac,
        &["A".into(), "B".into(), "C".into()],
        printing::Kind::Classes,
        false,
        false,
    )
    .unwrap();
    assert!(!print.pages.is_empty());
    for (name, bytes) in [
        ("tulokset.csv", csv.into_bytes()),
        ("tulokset.html", html.into_bytes()),
        ("tulokset.xlsx", xlsx),
        (
            "asemat.html",
            report::project_results_html(&saved).unwrap().into_bytes(),
        ),
        (
            "tulostusmalli.toml",
            toml::to_string(&print).unwrap().into_bytes(),
        ),
    ] {
        report::save_new_bytes(&directory.path.join(name), &bytes).unwrap();
    }
    assert_eq!(fs::read(input).unwrap(), original_excel);
    let summary = format!(
        "72 täysin synteettistä paria; 3 luokkaa, 2 rataa, yhdistetty A+B, 12 BC-paria.\n69 normaalia hyväksyttyä tulosta, 1 vahvistettu KESK, 1 HYL, 1 lukematon.\nEmit- ja käsimaali, RT 600 p, lisävirheet 2 × numero, BC 3 × numero.\nGUI- ja laitetestiä ei suoriteta tässä integraatiotestissä.\nKesto: {:.2} s\n",
        started.elapsed().as_secs_f64()
    );
    fs::write(directory.path.join("KOOSTE.txt"), summary).unwrap();
}
