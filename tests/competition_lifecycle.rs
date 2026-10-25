//! Yksi toimitsijan kilpailutyönkulku tuonnista julkaistaviin tulostiedostoihin.
//! Kaikki henkilöt, radat ja korttikehykset ovat synteettisiä; laitteita ei avata.
use as2026::{
    ClockTime, emit, excel_import, native, ranking, report, starts,
    terminal::parse_clock,
    workspace::{ClassField, DriverField, OverrideField, ResultField, Workspace},
};
use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "as2026-kilpailun-elinkaari-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture(extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("fixtures/excel-import/synteettinen.{extension}"))
}
fn clock_after(start: &str, seconds: i64) -> ClockTime {
    let value = (parse_clock(start)
        .unwrap()
        .seconds_since_midnight()
        .unwrap()
        + seconds)
        .rem_euclid(86400);
    ClockTime::new(
        (value / 3600) as u32,
        (value / 60 % 60) as u32,
        (value % 60) as u32,
    )
    .unwrap()
}
fn clock_text(clock: ClockTime) -> String {
    format!("{:02}:{:02}:{:02}", clock.hour, clock.minute, clock.second)
}
fn frame(card: u32, jat_time: u16, reader_time: u16) -> emit::EmitFrame {
    let mut decoded = [0_u8; emit::FRAME_LENGTH];
    decoded[..2].copy_from_slice(&[0xff, 0xff]);
    decoded[2..5].copy_from_slice(&card.to_le_bytes()[..3]);
    // 88 on tuntematon leima varsinaisen käsimaalin jälkeen. Se säilytetään,
    // mutta 1400 s ei saa muuttua käsimaalin ajaksi.
    for (index, (code, time)) in [
        (0_u8, 0_u16),
        (140, jat_time),
        (88, 1400),
        (250, reader_time),
    ]
    .into_iter()
    .enumerate()
    {
        let offset = 10 + index * 3;
        decoded[offset] = code;
        decoded[offset + 1..offset + 3].copy_from_slice(&time.to_le_bytes());
    }
    emit::parse_frame(&decoded.map(|byte| byte ^ emit::XOR_MASK)).unwrap()
}

#[test]
fn imported_competition_reaches_approved_reopened_and_exported_results_without_data_loss() {
    let sources = [fixture("xls"), fixture("xlsx")];
    let original_sources = sources
        .iter()
        .map(|path| fs::read(path).unwrap())
        .collect::<Vec<_>>();
    let previews = sources
        .iter()
        .map(|path| excel_import::read(path).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(previews[0], previews[1]);
    assert!(previews[0].can_import());
    let mut project = previews[1].to_project("Synteettinen syysajo Ää").unwrap();
    assert_eq!(project.kilpailijat.len(), 2);
    assert_eq!(project.radat.len(), 2);
    assert!(project.kilpailijat.iter().all(|d| d.numero != 999));
    assert_eq!(project.kilpailijat[0].kuljettajan_seura, "Seura A");
    assert_eq!(project.kilpailijat[0].kartanlukijan_seura, "Seura B");
    assert_eq!(project.radat[0].asemat[1].tavoiteaika_sekuntia, 448);
    assert_eq!(project.radat[0].asemat[1].saanto, 1);
    // Laajennetaan vain keksittyä testikilpailua kahteen arvottavaan C-pariin.
    // Tuotu AT2 on jo käsiasema; myös maali kirjataan tässä kilpailussa käsin.
    let mut third = project.kilpailijat[0].clone();
    third.numero = 3;
    third.emit = 34567;
    third.kuljettaja = "Kolmas mallikuljettaja".into();
    project.kilpailijat.push(third);
    project.radat[0].asemat.last_mut().unwrap().emit_koodi = 0;
    let original_b_start = project
        .kilpailijat
        .iter()
        .find(|d| d.luokka == "B")
        .unwrap()
        .lahtoaika
        .clone();
    let directory = Directory::new();
    let file = directory.0.join(native::FILE_NAME);
    let mut workspace = Workspace::create_imported(&directory.0, project.clone()).unwrap();
    let original_file = fs::read(&file).unwrap();
    assert!(Workspace::create_imported(&directory.0, project).is_err());
    assert_eq!(fs::read(&file).unwrap(), original_file);

    workspace
        .edit_class("C", ClassField::Start, "17:00:00")
        .unwrap();
    workspace
        .edit_class("C", ClassField::Interval, "120")
        .unwrap();
    workspace.edit_class("C", ClassField::Np, "1").unwrap();
    workspace.edit_class("C", ClassField::Sp, "1").unwrap();
    workspace.set_draw_classes(&["C".into()]).unwrap();
    let before_draw = workspace.project().unwrap().clone();
    let draw = starts::draw(&before_draw, 2026).unwrap();
    assert_eq!(
        workspace.project(),
        Some(&before_draw),
        "esikatselu ei saa muuttaa lähtöaikoja"
    );
    assert_eq!(draw.assignments.len(), 2);
    assert!(
        draw.assignments
            .iter()
            .all(|entry| [1, 3].contains(&entry.number))
    );
    workspace.apply_start_draw(&draw).unwrap();
    assert_eq!(
        workspace
            .project()
            .unwrap()
            .kilpailijat
            .iter()
            .find(|d| d.numero == 2)
            .unwrap()
            .lahtoaika,
        original_b_start
    );
    let mut c_times = workspace
        .project()
        .unwrap()
        .kilpailijat
        .iter()
        .filter(|d| d.luokka == "C")
        .map(|d| d.lahtoaika.as_str())
        .collect::<Vec<_>>();
    c_times.sort_unstable();
    assert_eq!(c_times, ["17:00:00", "17:02:00"]);
    let start_list = starts::start_list_html(workspace.project().unwrap()).unwrap();
    assert!(start_list.contains("Mallikuljettaja Ää"));
    assert!(start_list.contains(&original_b_start));
    report::save_new(&directory.0.join("lahtoluettelo.html"), &start_list).unwrap();
    workspace
        .edit_override(OverrideField::ExtraRt, "25")
        .unwrap();

    let unknown = frame(99999, 568, 1460);
    let outcome = workspace
        .record_frame(&unknown, ClockTime::new(19, 0, 0).unwrap())
        .unwrap();
    assert_eq!(outcome.competitor_number, None);
    assert!(workspace.result_snapshot(1).unwrap().is_none());
    assert!(workspace.result_snapshot(3).unwrap().is_none());
    let mut first_known_event = None;
    for number in [1, 3] {
        let driver = workspace
            .project()
            .unwrap()
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .unwrap()
            .clone();
        let card = frame(driver.emit, 568, 1460);
        let outcome = workspace
            .record_frame(&card, clock_after(&driver.lahtoaika, 1340))
            .unwrap();
        assert_eq!(outcome.competitor_number, Some(number));
        assert!(outcome.content_changed);
        workspace.ensure_readout_result(number).unwrap();
        let pending = workspace.result_snapshot(number).unwrap().unwrap();
        assert_eq!(pending.pending_manual_clocks().len(), 2);
        assert_eq!(pending.tila, "ILM");
        assert_eq!(pending.maalikello_sekuntia, 0);
        assert!(workspace.approve_result(number).is_err());
        if number == 1 {
            first_known_event = workspace.project().unwrap().leimaluvut.last().cloned();
        }
        let at = pending
            .rivit
            .iter()
            .find(|r| r.station_type == "AT")
            .unwrap()
            .output_index;
        let finish = pending
            .rivit
            .iter()
            .find(|r| r.station_type == "M")
            .unwrap()
            .output_index;
        workspace
            .edit_station_result(
                number,
                at,
                ResultField::Clock,
                &clock_text(clock_after(&driver.lahtoaika, 1202)),
            )
            .unwrap();
        workspace
            .edit_station_result(
                number,
                finish,
                ResultField::Clock,
                &clock_text(clock_after(&driver.lahtoaika, 1228)),
            )
            .unwrap();
        assert!(
            !workspace
                .result_snapshot(number)
                .unwrap()
                .unwrap()
                .can_approve()
        );
        if number == 1 {
            workspace
                .edit_driver(number, DriverField::MissingRtCount, "3")
                .unwrap();
            workspace
                .edit_driver(number, DriverField::ExtraRtCount, "2")
                .unwrap();
            workspace
                .edit_driver(number, DriverField::OtherPenalties, "100")
                .unwrap();
        }
        workspace.recalculate_stored(number).unwrap();
        let checked = workspace.result_snapshot(number).unwrap().unwrap();
        assert_eq!(checked.tila, "TRK");
        assert!(checked.can_approve());
        assert!(!checked.hyvaksytty);
        let unpublished = workspace.ranking().unwrap();
        assert!(unpublished.groups.iter().all(|group| {
            group
                .entries
                .iter()
                .all(|entry| entry.number != i64::from(number))
        }));
        assert!(unpublished.routes.iter().all(|route| {
            route
                .entries
                .iter()
                .all(|entry| entry.number != i64::from(number))
        }));
        assert_eq!(checked.kesto_sekuntia, 1228);
        assert_eq!(
            checked
                .rivit
                .iter()
                .find(|r| r.station_type == "JAT")
                .unwrap()
                .actual_seconds,
            448
        );
        assert_eq!(
            checked
                .rivit
                .iter()
                .find(|r| r.station_type == "AT")
                .unwrap()
                .actual_seconds,
            754
        );
        assert_eq!(
            checked
                .rivit
                .iter()
                .find(|r| r.station_type == "M")
                .unwrap()
                .actual_seconds,
            780
        );
        assert_eq!(checked.rivit.last().unwrap().received_code, 88);
        assert_eq!(checked.rivit.last().unwrap().other_penalty, 600);
        if number == 1 {
            workspace
                .edit_station_result(number, at, ResultField::TimePenalty, "-45")
                .unwrap();
            workspace
                .edit_station_result(number, at, ResultField::OtherPenalty, "7")
                .unwrap();
        }
        workspace.approve_result(number).unwrap();
        assert!(
            workspace
                .result_snapshot(number)
                .unwrap()
                .unwrap()
                .hyvaksytty
        );
    }
    assert_eq!(workspace.project().unwrap().leimaluvut.len(), 3);
    assert_eq!(
        workspace.project().unwrap().leimaluvut[1],
        first_known_event.unwrap()
    );
    let result = workspace.results().unwrap();
    let corrected = result.competitors.iter().find(|d| d.number == 1).unwrap();
    assert_eq!(
        &corrected.header_fields[19..=26],
        ["45", "607", "3", "1800", "2", "50", "100", "2602"]
    );
    assert_eq!(corrected.total_penalties, 2602);
    // B jätettiin arvonnan ulkopuolelle ja peruuttaa ennen kortinlukua.
    workspace
        .edit_driver(2, DriverField::Status, "PER")
        .unwrap();
    workspace
        .edit_driver(2, DriverField::StatusReason, "Peruutus ennen lähtöä")
        .unwrap();

    for (index, status) in ["HYL", "PER"].into_iter().enumerate() {
        workspace
            .edit_driver(1, DriverField::Status, status)
            .unwrap();
        workspace
            .edit_driver(1, DriverField::StatusReason, "Toimitsijan päätös Ää")
            .unwrap();
        workspace.approve_result(1).unwrap();
        let approved = workspace.result_snapshot(1).unwrap().unwrap();
        let driver = workspace
            .project()
            .unwrap()
            .kilpailijat
            .iter()
            .find(|d| d.numero == 1)
            .unwrap()
            .clone();
        let changed = frame(driver.emit, 569 + index as u16, 1460);
        let outcome = workspace
            .record_frame(&changed, clock_after(&driver.lahtoaika, 1340))
            .unwrap();
        assert!(outcome.content_changed);
        workspace.ensure_readout_result(1).unwrap();
        assert_eq!(workspace.result_snapshot(1).unwrap().unwrap(), approved);
        let repeated = frame(driver.emit, 569 + index as u16, 1461);
        let outcome = workspace
            .record_frame(&repeated, clock_after(&driver.lahtoaika, 1341))
            .unwrap();
        assert!(!outcome.content_changed);
        workspace.ensure_readout_result(1).unwrap();
        assert_eq!(workspace.result_snapshot(1).unwrap().unwrap(), approved);
        let result = workspace.results().unwrap();
        let pair = result.competitors.iter().find(|d| d.number == 1).unwrap();
        assert_eq!(pair.status, status);
        assert_eq!(pair.header_fields[29], "Toimitsijan päätös Ää");
        assert_eq!(pair.total_penalties, 2602);
    }
    assert_eq!(workspace.project().unwrap().leimaluvut.len(), 7);
    assert_eq!(workspace.project().unwrap().leimaluvut[0].emit, 99999);
    assert_eq!(
        workspace
            .project()
            .unwrap()
            .leimaluvut
            .iter()
            .filter(|event| event.emit == 12345)
            .count(),
        5
    );
    let ranking = workspace.ranking().unwrap();
    let class = ranking
        .groups
        .iter()
        .find(|g| g.kind == ranking::GroupKind::Ordinary && g.code == "C")
        .unwrap();
    let winner = class.entries.iter().find(|d| d.number == 3).unwrap();
    assert_eq!(winner.status, ranking::Status::Classified);
    assert_eq!(winner.primary_position, 1);
    assert_eq!(winner.series_points, 100);
    assert_eq!(
        class.entries.iter().find(|d| d.number == 1).unwrap().status,
        ranking::Status::Cancelled
    );
    let results = workspace.results().unwrap();
    let station_html = report::project_results_html(workspace.project().unwrap()).unwrap();
    let html = report::project_ranking_html(&results, &ranking);
    let csv = report::ranking_csv(&results, &ranking);
    let xlsx = report::ranking_xlsx(&results, &ranking).unwrap();
    assert!(station_html.contains("Hyväksytty tulos."));
    assert!(station_html.contains("Kokonaistulos: 2602 p"));
    assert!(html.contains("Toimitsijan päätös Ää"));
    assert!(csv.contains("Toimitsijan päätös Ää"));
    assert!(csv.contains("\"2602\""));
    let mut zip = zip::ZipArchive::new(Cursor::new(&xlsx)).unwrap();
    let mut sheet = String::new();
    zip.by_name("xl/worksheets/sheet1.xml")
        .unwrap()
        .read_to_string(&mut sheet)
        .unwrap();
    assert!(sheet.contains("Toimitsijan päätös Ää"));
    assert!(sheet.contains("2602"));
    assert!(!sheet.contains("<f>"));
    for (name, data) in [
        ("asemat.html", station_html.as_bytes()),
        ("tulokset.html", html.as_bytes()),
        ("tulokset.csv", csv.as_bytes()),
        ("tulokset.xlsx", xlsx.as_slice()),
    ] {
        let path = directory.0.join(name);
        report::save_new_bytes(&path, data).unwrap();
        assert!(report::save_new_bytes(&path, b"ei saa korvata").is_err());
        assert_eq!(fs::read(path).unwrap(), data);
    }
    let saved = workspace.project().unwrap().clone();
    let reopened = Workspace::open(&directory.0).unwrap();
    assert_eq!(reopened.project(), Some(&saved));
    assert_eq!(
        report::ranking_csv(&reopened.results().unwrap(), &reopened.ranking().unwrap()),
        csv
    );
    assert_eq!(
        report::project_results_html(reopened.project().unwrap()).unwrap(),
        station_html
    );
    assert!(
        Workspace::create_imported(
            &directory.0,
            native::Project::new("Ei korvata kilpailua").unwrap()
        )
        .is_err()
    );
    assert_eq!(
        Workspace::open(&directory.0).unwrap().project(),
        Some(&saved)
    );
    assert_eq!(
        reopened
            .project()
            .unwrap()
            .kilpailijat
            .iter()
            .find(|d| d.numero == 2)
            .unwrap()
            .lahtoaika,
        original_b_start
    );
    for (source, original) in sources.iter().zip(original_sources) {
        assert_eq!(fs::read(source).unwrap(), original);
    }
}
