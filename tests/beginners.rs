use as2026::{
    application,
    beginners::{self, BeginnersCup},
    native::{Project, ReadPunch},
    ranking::TiePolicy,
    report, starts,
    workspace::{DriverField, ResultField, StationField, Workspace},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn project() -> Project {
    let mut p = Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    p.kilpailu.ajanottotapa = 0;
    p.kilpailijat[0].emit_korjaus_sekuntia = 0;
    p.radat[0].asemat[3].emit_koodi = 33;
    p.radat[0].asemat[3].tavoiteaika_sekuntia = 900;
    p.radat[0].asemat[3].jakso_minuutteina = Some(15);
    p.leimaluvut[0].lukukello = "12:15:00".into();
    p.leimaluvut[0].leimat = [
        (0, 0),
        (31, 330),
        (90, 450),
        (32, 600),
        (33, 900),
        (250, 900),
    ]
    .into_iter()
    .map(|(koodi, aika_sekuntia)| ReadPunch {
        koodi,
        aika_sekuntia,
    })
    .collect();
    p
}

fn config(p: &Project) -> BeginnersCup {
    BeginnersCup {
        nimi: "NAC Beginners Cup".into(),
        tunnus: "BC".into(),
        lahderata: 1,
        lahdeasemat: p.radat[0].asemat.clone(),
        mukana_asemat: (1..=p.radat[0].asemat.len() as u32)
            .filter(|&position| position != 2)
            .collect(),
    }
}

struct Fixture {
    directory: PathBuf,
    workspace: Workspace,
}
impl Fixture {
    fn new(p: Project) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "as2026-bc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let c = config(&p);
        let mut workspace = Workspace::create_imported(&directory, p).unwrap();
        workspace.save_beginners_cup(c, &[7]).unwrap();
        Self {
            directory,
            workspace,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn one_competitor_and_readout_produce_two_independent_results() {
    let mut f = Fixture::new(project());
    let original = f.workspace.project().unwrap().clone();
    f.workspace.ensure_readout_result(7).unwrap();
    let nac = f.workspace.result_snapshot(7).unwrap().unwrap();
    assert!(nac.can_approve(), "{nac:?}");
    f.workspace.approve_result(7).unwrap();
    f.workspace.set_bc_view(true).unwrap();
    let bc = f.workspace.result_snapshot(7).unwrap().unwrap();
    assert!(bc.can_approve(), "{bc:?}");
    assert!(!bc.rivit.iter().any(|r| r.expected_code == 31));
    assert!(
        bc.rivit
            .iter()
            .any(|r| r.received_code == 90 && r.other_penalty > 0)
    );
    assert_eq!(bc.maalikello_sekuntia, nac.maalikello_sekuntia);
    assert!(bc.aikavirheet < nac.aikavirheet);
    f.workspace.approve_result(7).unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::OtherPenalties, "123")
        .unwrap();
    assert!(!f.workspace.result_snapshot(7).unwrap().unwrap().hyvaksytty);
    assert!(
        f.workspace.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    assert_eq!(
        f.workspace.project().unwrap().kilpailijat[0]
            .lisavirheet
            .muut_virhepisteet,
        0
    );
    assert_eq!(
        f.workspace.project().unwrap().bc_osallistujat[0]
            .lisavirheet
            .muut_virhepisteet,
        123
    );
    f.workspace.approve_result(7).unwrap();
    f.workspace.set_bc_view(false).unwrap();
    f.workspace
        .edit_driver(7, DriverField::OtherPenalties, "456")
        .unwrap();
    assert!(
        f.workspace.project().unwrap().bc_osallistujat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    assert_eq!(f.workspace.project().unwrap().kilpailijat.len(), 1);
    assert_eq!(
        f.workspace.project().unwrap().leimaluvut,
        original.leimaluvut
    );
    let reopened = Workspace::open(&f.directory).unwrap();
    assert_eq!(reopened.project(), f.workspace.project());
    let mut preparation = f.workspace.project().unwrap().clone();
    preparation.leimaluvut.clear();
    preparation.kilpailijat[0].tallennettu_tulos = None;
    assert_eq!(starts::draw(&preparation, 1).unwrap().assignments.len(), 1);
}

#[test]
fn repeated_codes_are_filtered_by_occurrence_and_real_extras_remain() {
    let mut p = project();
    p.radat[0].asemat[2].emit_koodi = 31;
    p.leimaluvut[0].leimat[3].koodi = 31;
    let f = Fixture::new(p);
    let view = beginners::projection(f.workspace.project().unwrap()).unwrap();
    let punches = &view.leimaluvut[0].leimat;
    assert_eq!(
        punches.iter().map(|p| p.koodi).collect::<Vec<_>>(),
        [0, 90, 31, 33, 250]
    );
    assert_eq!(punches[2].aika_sekuntia, 600);
    let result = application::capture_project_result(&view, 7).unwrap();
    assert!(
        result
            .rivit
            .iter()
            .any(|r| r.expected_code == 31 && r.elapsed_seconds == 600)
    );
    assert!(
        result
            .rivit
            .iter()
            .any(|r| r.received_code == 90 && r.other_penalty > 0)
    );
}

#[test]
fn retained_shared_ula_keeps_its_punch_when_previous_at_is_omitted() {
    let mut p = project();
    let mut ula = p.radat[0].asemat[1].clone();
    ula.tyyppi = "ULA".into();
    ula.nimi = "Yhteinen ULA".into();
    ula.tavoiteaika_sekuntia = 0;
    p.radat[0].asemat.insert(2, ula);
    p.radat[0].asemat[3].tavoiteaika_sekuntia = 300;
    p.radat[0].asemat[4].tavoiteaika_sekuntia = 600;
    let mut f = Fixture::new(p);
    let mut c = config(f.workspace.project().unwrap());
    c.mukana_asemat = vec![1, 3, 4, 5];
    f.workspace.save_beginners_cup(c, &[7]).unwrap();
    let view = beginners::projection(f.workspace.project().unwrap()).unwrap();
    assert!(view.leimaluvut[0].leimat.iter().any(|p| p.koodi == 31));
    let result = application::capture_project_result(&view, 7).unwrap();
    assert!(
        result
            .rivit
            .iter()
            .any(|r| r.station_type == "ULA" && r.received_code == 31)
    );
    let mut c = config(f.workspace.project().unwrap());
    c.mukana_asemat = vec![1, 4, 5];
    assert!(f.workspace.save_beginners_cup(c, &[7]).is_err());
}

#[test]
fn ambiguous_missing_repeated_code_is_not_silently_discarded() {
    let mut p = project();
    p.radat[0].asemat[2].emit_koodi = 31;
    p.leimaluvut[0].leimat[3].koodi = 31;
    p.leimaluvut[0].leimat.remove(1);
    let mut f = Fixture::new(p);
    assert!(beginners::projection(f.workspace.project().unwrap()).is_err());
    assert!(f.workspace.ensure_readout_result(7).is_err());
    assert!(
        f.workspace.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .is_some()
    );
    assert!(
        f.workspace.project().unwrap().bc_osallistujat[0]
            .tallennettu_tulos
            .is_none()
    );
    assert!(application::calculated_project(f.workspace.project().unwrap()).is_ok());
}

#[test]
fn bc_route_change_invalidates_only_bc_and_requires_review() {
    let mut f = Fixture::new(project());
    f.workspace.ensure_readout_result(7).unwrap();
    f.workspace.approve_result(7).unwrap();
    f.workspace.set_bc_view(true).unwrap();
    f.workspace.approve_result(7).unwrap();
    let mut c = config(f.workspace.project().unwrap());
    c.mukana_asemat = vec![1, 2, 4];
    f.workspace.save_beginners_cup(c, &[7]).unwrap();
    assert!(!f.workspace.result_snapshot(7).unwrap().unwrap().hyvaksytty);
    assert!(f.workspace.approve_result(7).is_err());
    assert!(
        f.workspace.project().unwrap().kilpailijat[0]
            .tallennettu_tulos
            .as_ref()
            .unwrap()
            .hyvaksytty
    );
    f.workspace.review_results(7).unwrap();
    f.workspace.approve_result(7).unwrap();
    f.workspace
        .edit_station(1, 1, StationField::Target, "310")
        .unwrap();
    assert!(f.workspace.results().is_err());
    assert!(f.workspace.set_bc_view(false).is_ok());
    let p = f.workspace.project().unwrap();
    let mut c = config(p);
    c.mukana_asemat = vec![1, 2, 4];
    f.workspace.save_beginners_cup(c, &[7]).unwrap();
    f.workspace.set_bc_view(true).unwrap();
    f.workspace.review_results(7).unwrap();
    assert!(
        f.workspace
            .result_snapshot(7)
            .unwrap()
            .unwrap()
            .can_approve()
    );
}

#[test]
fn publication_keeps_nac_and_bc_approvals_separate() {
    let mut f = Fixture::new(project());
    f.workspace.ensure_readout_result(7).unwrap();
    f.workspace.approve_result(7).unwrap();
    let p = f.workspace.project().unwrap();
    let before = report::competition_ranking_csv(p, TiePolicy::KeepSharedAndFlag).unwrap();
    assert!(!before.lines().any(|line| line.contains("BC")));
    f.workspace.set_bc_view(true).unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::OtherPenalties, "123")
        .unwrap();
    f.workspace.approve_result(7).unwrap();
    let p = f.workspace.project().unwrap();
    let csv = report::competition_ranking_csv(p, TiePolicy::KeepSharedAndFlag).unwrap();
    assert!(csv.contains("BC"));
    assert_eq!(csv.lines().filter(|l| l.contains("Ryhmä")).count(), 1);
    let html = report::competition_ranking_html(p, TiePolicy::KeepSharedAndFlag).unwrap();
    assert!(html.contains("BC"));
    assert_eq!(html.matches("<!doctype html>").count(), 1);
    assert_eq!(html.matches("</main>").count(), 1);
    assert!(report::xlsx_from_csv(&csv).unwrap().starts_with(b"PK"));
    let nac = application::calculated_project(p).unwrap();
    let bc = beginners::results(p).unwrap();
    assert!(nac.has_approved_result(7) && bc.has_approved_result(7));
    assert_eq!(bc.competitors[0].class_code, "BC");
    assert_eq!(bc.competitors[0].header_fields[25], "123");
    assert_eq!(nac.competitors[0].header_fields[25], "0");
}

#[test]
fn deselection_preserves_bc_data_and_reselection_restores_it() {
    let mut f = Fixture::new(project());
    f.workspace.set_bc_view(true).unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::OtherPenalties, "88")
        .unwrap();
    let c = config(f.workspace.project().unwrap());
    f.workspace.save_beginners_cup(c.clone(), &[]).unwrap();
    assert!(f.workspace.results().unwrap().competitors.is_empty());
    assert_eq!(
        f.workspace.project().unwrap().bc_osallistujat[0]
            .lisavirheet
            .muut_virhepisteet,
        88
    );
    f.workspace.save_beginners_cup(c, &[7]).unwrap();
    assert_eq!(
        f.workspace.results().unwrap().competitors[0].header_fields[25],
        "88"
    );
    assert_eq!(f.workspace.project().unwrap().kilpailijat.len(), 1);
}

#[test]
fn invalid_configuration_does_not_change_disk_or_competitors() {
    let mut f = Fixture::new(project());
    let before = f.workspace.project().unwrap().clone();
    let bytes = std::fs::read(f.workspace.path()).unwrap();
    for positions in [vec![3, 4], vec![1, 3], vec![1, 3, 3, 4], vec![1, 99, 4]] {
        let mut c = config(&before);
        c.mukana_asemat = positions;
        assert!(f.workspace.save_beginners_cup(c, &[7]).is_err());
    }
    let mut c = config(&before);
    c.tunnus = "A".into();
    assert!(f.workspace.save_beginners_cup(c, &[7]).is_err());
    assert!(
        f.workspace
            .save_beginners_cup(config(&before), &[7, 7])
            .is_err()
    );
    assert!(
        f.workspace
            .save_beginners_cup(config(&before), &[999])
            .is_err()
    );
    assert_eq!(f.workspace.project().unwrap(), &before);
    assert_eq!(std::fs::read(f.workspace.path()).unwrap(), bytes);
}

#[test]
fn bc_station_penalties_and_manual_status_are_independent() {
    let mut f = Fixture::new(project());
    f.workspace.ensure_readout_result(7).unwrap();
    let nac = f.workspace.result_snapshot(7).unwrap().unwrap();
    f.workspace.set_bc_view(true).unwrap();
    let row = f
        .workspace
        .result_snapshot(7)
        .unwrap()
        .unwrap()
        .rivit
        .iter()
        .find(|r| r.expected_code == 32)
        .unwrap()
        .output_index;
    f.workspace
        .edit_station_result(7, row, ResultField::OtherPenalty, "55")
        .unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::Status, "HYL")
        .unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::StatusReason, "BC:n päätös")
        .unwrap();
    f.workspace.approve_result(7).unwrap();
    assert_eq!(f.workspace.results().unwrap().competitors[0].status, "HYL");
    f.workspace.set_bc_view(false).unwrap();
    assert_eq!(f.workspace.result_snapshot(7).unwrap().unwrap(), nac);
    assert!(f.workspace.project().unwrap().kilpailijat[0].tila.is_none());
}

#[test]
fn bc_interruption_can_be_confirmed_without_a_card_independently() {
    let mut p = project();
    p.kilpailijat[0].emit = 0;
    p.leimaluvut.clear();
    let mut f = Fixture::new(p);
    f.workspace.set_bc_view(true).unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::Status, "KESK")
        .unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::StatusReason, "BC-osuus keskeytyi")
        .unwrap();
    assert!(f.workspace.can_confirm_interruption(7));
    f.workspace.approve_result(7).unwrap();
    assert!(f.workspace.results().unwrap().has_confirmed_interruption(7));
    f.workspace.set_bc_view(false).unwrap();
    assert!(!f.workspace.results().unwrap().has_confirmed_interruption(7));
    assert!(f.workspace.project().unwrap().kilpailijat[0].tila.is_none());
    assert!(
        f.workspace.project().unwrap().bc_osallistujat[0]
            .tallennettu_tulos
            .is_none()
    );
}

#[test]
fn changing_competitor_number_preserves_its_bc_membership_and_points() {
    let mut f = Fixture::new(project());
    f.workspace.set_bc_view(true).unwrap();
    f.workspace
        .edit_result_driver(7, DriverField::OtherPenalties, "42")
        .unwrap();
    f.workspace.set_bc_view(false).unwrap();
    f.workspace
        .edit_driver(7, DriverField::Number, "27")
        .unwrap();
    f.workspace.set_bc_view(true).unwrap();
    assert_eq!(f.workspace.project().unwrap().bc_osallistujat[0].numero, 27);
    assert_eq!(f.workspace.results().unwrap().competitors[0].number, 27);
    assert_eq!(
        f.workspace.results().unwrap().competitors[0].header_fields[25],
        "42"
    );
}

#[test]
fn skipped_jat_extends_the_next_section_without_changing_finish() {
    let mut p = project();
    p.radat[0].asemat[1].tyyppi = "JAT".into();
    p.radat[0].asemat[2].tavoiteaika_sekuntia = 300;
    p.radat[0].asemat[3].tavoiteaika_sekuntia = 600;
    p.radat[0].asemat[3].jakso_minuutteina = Some(10);
    let c = config(&p);
    let stations = beginners::derived_stations(&c).unwrap();
    assert_eq!(stations[1].tavoiteaika_sekuntia, 600);
    assert_eq!(stations[2].tavoiteaika_sekuntia, 900);
    assert_eq!(stations[2].jakso_minuutteina, Some(15));
}
