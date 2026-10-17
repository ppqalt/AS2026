use as2026::{
    native::{ManualStatus, Project},
    starts,
    terminal::parse_clock,
};

fn project() -> Project {
    let mut project =
        Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    project.leimaluvut.clear();
    let first = project.kilpailijat[0].clone();
    project.kilpailijat = (1..=10)
        .map(|number| {
            let mut driver = first.clone();
            driver.numero = number;
            driver.emit = 1000 + number;
            driver.lahtoaika = "11:00:00".into();
            driver.kuljettaja = format!("Kuljettaja {number}");
            driver
        })
        .collect();
    project
}

#[test]
fn draw_is_repeatable_preview_and_only_confirmed_times_are_saved() {
    let mut project = project();
    project.kilpailijat[0].kuljettajan_lisenssi = "123X".into();
    project.kilpailijat[0].lisavirheet.muut_virhepisteet = 14;
    let original = project.clone();
    let draw = starts::draw(&project, 42).unwrap();
    assert_eq!(project, original);
    assert_eq!(
        draw.assignments,
        starts::draw(&project, 42).unwrap().assignments
    );
    assert_ne!(
        draw.assignments,
        starts::draw(&project, 43).unwrap().assignments
    );
    assert_eq!(draw.assignments.len(), 10);
    for (index, assignment) in draw.assignments.iter().enumerate() {
        assert_eq!(
            assignment.time.seconds_since_midnight().unwrap(),
            12 * 3600 + index as i64 * 60
        );
    }
    draw.apply(&mut project).unwrap();
    for (before, after) in original.kilpailijat.iter().zip(&project.kilpailijat) {
        let mut expected = before.clone();
        let assigned = draw
            .assignments
            .iter()
            .find(|item| item.number == before.numero)
            .unwrap();
        expected.lahtoaika = format!(
            "{:02}:{:02}:{:02}",
            assigned.time.hour, assigned.time.minute, assigned.time.second
        );
        assert_eq!(*after, expected);
    }
    assert_eq!(project.leimaluvut, original.leimaluvut);
}

#[test]
fn only_registered_entrants_participate_and_other_statuses_remain_unchanged() {
    let mut project = project();
    for (driver, status) in project
        .kilpailijat
        .iter_mut()
        .zip(["PER", "HYL", "SULJ", "EILÄ", "KESK", "TRK", "ILM"])
    {
        driver.tila = Some(ManualStatus::new(status, "Syy").unwrap());
    }
    let original = project.clone();
    let draw = starts::draw(&project, 1).unwrap();
    assert_eq!(draw.assignments.len(), 4);
    assert!(draw.assignments.iter().all(|item| item.number >= 7));
    draw.apply(&mut project).unwrap();
    assert_eq!(project.kilpailijat[..6], original.kilpailijat[..6]);
}

#[test]
fn each_class_uses_its_configured_schedule_and_midnight_wraps() {
    let mut project = project();
    project.kilpailijat.truncate(4);
    project.luokat[0].lahtoaika = "23:59:00".into();
    project.luokat[0].lahtovali_minuutteina = 2;
    let mut second_class = project.luokat[0].clone();
    second_class.tunnus = "B".into();
    second_class.lahtoaika = "08:00:00".into();
    second_class.lahtovali_minuutteina = 3;
    project.luokat.push(second_class);
    project.kilpailijat[2].luokka = "B".into();
    project.kilpailijat[3].luokka = "B".into();
    let draw = starts::draw(&project, 4).unwrap();
    assert_eq!(
        draw.assignments
            .iter()
            .map(|row| row.time)
            .collect::<Vec<_>>(),
        [
            parse_clock("23:59:00").unwrap(),
            parse_clock("00:01:00").unwrap(),
            parse_clock("08:00:00").unwrap(),
            parse_clock("08:03:00").unwrap(),
        ]
    );
}

#[test]
fn read_competitors_are_protected_from_redraw() {
    let mut project = project();
    let mut read = Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml"))
        .unwrap()
        .leimaluvut
        .remove(0);
    read.emit = project.kilpailijat[0].emit;
    project.leimaluvut.push(read);
    let before = project.clone();
    assert!(
        starts::draw(&project, 1)
            .unwrap_err()
            .to_string()
            .contains("jo luettu")
    );
    assert_eq!(project, before);
    // Perutun kilpailijan historia ei estä muiden lähtöaikojen arvontaa.
    project.kilpailijat[0].tila = Some(ManualStatus::new("PER", "").unwrap());
    let draw = starts::draw(&project, 1).unwrap();
    draw.apply(&mut project).unwrap();
    assert_eq!(project.leimaluvut, before.leimaluvut);
}

#[test]
fn stored_result_is_protected_even_if_readout_history_is_missing() {
    let mut project =
        Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    let result = as2026::application::capture_project_result(&project, 7).unwrap();
    project.kilpailijat[0].tallennettu_tulos = Some(result);
    project.leimaluvut.clear();
    assert!(
        starts::draw(&project, 1)
            .unwrap_err()
            .to_string()
            .contains("asematulos")
    );
}

#[test]
fn stale_or_modified_preview_is_rejected_atomically() {
    let mut project = project();
    let mut draw = starts::draw(&project, 9).unwrap();
    project.luokat[0].lahtovali_minuutteina = 2;
    let before = project.clone();
    assert!(
        draw.apply(&mut project)
            .unwrap_err()
            .to_string()
            .contains("muuttuneet")
    );
    assert_eq!(project, before);
    draw = starts::draw(&project, 9).unwrap();
    draw.assignments[0].number = draw.assignments[1].number;
    assert!(
        draw.apply(&mut project)
            .unwrap_err()
            .to_string()
            .contains("esikatselua")
    );
    assert_eq!(project, before);
}

#[test]
fn invalid_spacing_and_multi_day_schedules_are_rejected() {
    let mut project = project();
    project.luokat[0].lahtovali_minuutteina = 0;
    assert!(
        starts::draw(&project, 1)
            .unwrap_err()
            .to_string()
            .contains("vähintään")
    );
    project.luokat[0].lahtovali_minuutteina = 160;
    assert!(
        starts::draw(&project, 1)
            .unwrap_err()
            .to_string()
            .contains("vuorokauden")
    );
}

#[test]
fn start_lists_are_finnish_escaped_and_in_clock_order() {
    let mut project = project();
    project.kilpailijat.truncate(2);
    project.kilpailu.nimi = "<Kilpailu & testi>".into();
    project.kilpailijat[0].kuljettaja = "=SUM(1;2)<script>".into();
    project.kilpailijat[0].kartanlukija = "Ääkkönen".into();
    project.kilpailijat[0].lahtoaika = "13:00:00".into();
    project.kilpailijat[1].lahtoaika = "12:00:00".into();
    let html = starts::start_list_html(&project).unwrap();
    assert!(html.contains("lang=\"fi\""));
    assert!(html.contains("Lähtöluettelo"));
    assert!(html.contains("&lt;Kilpailu &amp; testi&gt;"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(html.find("12:00:00").unwrap() < html.find("13:00:00").unwrap());
    let csv = starts::start_list_csv(&project).unwrap();
    assert!(csv.starts_with("\"Lähtöaika\""));
    assert!(csv.contains("\"'=SUM(1;2)<script>\""));
    assert!(csv.contains("Ääkkönen"));
    assert!(csv.find("12:00:00").unwrap() < csv.find("13:00:00").unwrap());
}

#[test]
fn midnight_list_keeps_each_class_in_departure_order() {
    let mut project = project();
    project.kilpailijat.truncate(3);
    project.luokat[0].lahtoaika = "23:59:00".into();
    project.kilpailijat[0].lahtoaika = "00:02:00".into();
    project.kilpailijat[1].lahtoaika = "23:59:00".into();
    project.kilpailijat[2].lahtoaika = "00:01:00".into();
    for output in [
        starts::start_list_html(&project).unwrap(),
        starts::start_list_csv(&project).unwrap(),
    ] {
        assert!(output.find("23:59:00").unwrap() < output.find("00:01:00").unwrap());
        assert!(output.find("00:01:00").unwrap() < output.find("00:02:00").unwrap());
    }
}
