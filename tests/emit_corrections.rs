//! Toimitsijan yhteisen leimakorjauksen vaikutus molempiin tuloksiin ja levyyn.
use as2026::{
    beginners::BeginnersCup,
    emit_corrections::{self, Action, Request},
    native::{Project, ReadPunch},
    workspace::{ResultField, Workspace},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    path: PathBuf,
    ws: Workspace,
}
impl Fixture {
    fn new(mut project: Project) -> Self {
        project.beginners_cup = Some(Box::new(BeginnersCup {
            nimi: "Synteettinen BC".into(),
            tunnus: "BC".into(),
            lahderata: 1,
            lahdeasemat: project.radat[0].asemat.clone(),
            mukana_asemat: vec![1, 2, 3, 4],
        }));
        project.bc_osallistujat.push(as2026::beginners::Entry {
            numero: 7,
            osallistuu: true,
            ..Default::default()
        });
        let path = std::env::temp_dir().join(format!(
            "as2026-leimakorjaus-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let ws = Workspace::create_imported(&path, project).unwrap();
        Self { path, ws }
    }
    fn complete_and_approve(&mut self, clock: &str) {
        self.ws.ensure_readout_result(7).unwrap();
        for bc in [false, true] {
            self.ws.set_bc_view(bc).unwrap();
            let snapshot = self.ws.result_snapshot(7).unwrap().unwrap();
            let finish = snapshot
                .rivit
                .iter()
                .find(|r| r.station_type == "M")
                .unwrap()
                .output_index;
            self.ws
                .edit_station_result(7, finish, ResultField::Clock, clock)
                .unwrap();
            self.ws.recalculate_stored(7).unwrap();
            self.ws.approve_result(7).unwrap();
        }
        self.ws.set_bc_view(false).unwrap();
    }
    fn file(&self) -> PathBuf {
        self.path.join("kilpailu.toml")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
fn project() -> Project {
    Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}
fn request(row: u32, code: u32, elapsed_seconds: u32) -> Request {
    Request {
        row,
        source_row: None,
        action: Action::Edit,
        code,
        elapsed_seconds,
        reason: "Synteettinen varakello vahvistaa leiman".into(),
    }
}

#[test]
fn time_correction_recalculates_nac_bc_preserves_clocks_raw_and_reopens_unapproved() {
    let mut f = Fixture::new(project());
    f.complete_and_approve("12:10:00");
    let raw = f.ws.project().unwrap().leimaluvut.clone();
    assert_eq!(f.ws.result_snapshot(7).unwrap().unwrap().aikavirheet, 45);
    f.ws.correct_emit_row(7, &request(2, 31, 420)).unwrap();
    let saved = f.ws.project().unwrap();
    assert_eq!(saved.leimaluvut, raw);
    assert_eq!(saved.emit_korjaukset.len(), 1);
    assert_eq!(saved.emit_korjaukset[0].ennen[1].aika_sekuntia, 395);
    assert_eq!(saved.emit_korjaukset[0].jalkeen[1].aika_sekuntia, 420);
    let mut reopened = Workspace::open(&f.file()).unwrap();
    for bc in [false, true] {
        reopened.set_bc_view(bc).unwrap();
        let result = reopened.result_snapshot(7).unwrap().unwrap();
        assert_eq!(result.aikavirheet, 0);
        assert_eq!(result.asemien_muut_virheet, 600);
        assert!(!result.hyvaksytty);
        assert_eq!(
            result
                .rivit
                .iter()
                .find(|r| r.station_type == "M")
                .unwrap()
                .clock,
            [12, 10, 0]
        );
        assert!(result.can_approve());
        let archive = reopened.results().unwrap();
        assert!(!archive.has_approved_result(7));
        reopened.approve_result(7).unwrap();
        assert!(reopened.result_snapshot(7).unwrap().unwrap().hyvaksytty);
    }
}

#[test]
fn wrong_code_can_be_mapped_from_extra_to_missing_station_without_double_penalty() {
    let mut p = project();
    p.leimaluvut[0].leimat[1].koodi = 91;
    let mut f = Fixture::new(p);
    f.complete_and_approve("12:10:00");
    let rows = emit_corrections::rows(f.ws.project().unwrap(), 7).unwrap();
    let missing = rows.iter().find(|r| r.expected_code == 31).unwrap();
    let extra = rows
        .iter()
        .find(|r| r.sequence == 0 && r.code == 91)
        .unwrap();
    let mut edit = request(missing.index, 31, 420);
    edit.source_row = Some(extra.index);
    f.ws.correct_emit_row(7, &edit).unwrap();
    let result = f.ws.result_snapshot(7).unwrap().unwrap();
    assert_eq!(
        result
            .rivit
            .iter()
            .find(|r| r.expected_code == 31)
            .unwrap()
            .received_code,
        31
    );
    assert!(!result.rivit.iter().any(|r| r.received_code == 91));
    assert_eq!(result.asemien_muut_virheet, 600); // Vain oikeasti ylimääräinen 90 jää.
    assert_eq!(f.ws.project().unwrap().leimaluvut[0].leimat[1].koodi, 91);
}

#[test]
fn missing_punch_can_be_added_extra_removed_and_restored_with_append_only_history() {
    let mut p = project();
    p.leimaluvut[0].leimat.remove(1);
    let mut f = Fixture::new(p);
    f.complete_and_approve("12:10:00");
    f.ws.correct_emit_row(7, &request(2, 31, 420)).unwrap();
    let extra = emit_corrections::rows(f.ws.project().unwrap(), 7)
        .unwrap()
        .iter()
        .find(|r| r.sequence == 0)
        .unwrap()
        .index;
    let mut remove = request(extra, 0, 0);
    remove.action = Action::Remove;
    f.ws.correct_emit_row(7, &remove).unwrap();
    let result = f.ws.result_snapshot(7).unwrap().unwrap();
    assert_eq!(result.aikavirheet, 0);
    assert_eq!(result.asemien_muut_virheet, 0);
    assert_eq!(result.rivit.len(), 4);
    let mut restore = request(2, 0, 0);
    restore.action = Action::Restore;
    f.ws.correct_emit_row(7, &restore).unwrap();
    let saved = f.ws.project().unwrap();
    assert_eq!(saved.emit_korjaukset.len(), 3);
    let effective = emit_corrections::effective_readout(saved, &saved.leimaluvut[0]).unwrap();
    assert_eq!(effective, saved.leimaluvut[0]);
    assert!(!f.ws.result_snapshot(7).unwrap().unwrap().hyvaksytty);
}

#[test]
fn invalid_input_or_external_save_conflict_preserves_file_history_and_approvals() {
    let mut f = Fixture::new(project());
    f.complete_and_approve("12:10:00");
    let before = f.ws.project().unwrap().clone();
    let bytes = std::fs::read(f.file()).unwrap();
    let mut blank = request(2, 31, 420);
    blank.reason.clear();
    for invalid in [
        blank,
        request(2, 250, 420),
        request(2, 31, 100_000),
        request(2, 31, 721),
    ] {
        assert!(f.ws.correct_emit_row(7, &invalid).is_err());
        assert_eq!(f.ws.project().unwrap(), &before);
        assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    }
    std::fs::write(
        f.file(),
        format!("{}\n# Ulkoinen muutos\n", String::from_utf8(bytes).unwrap()),
    )
    .unwrap();
    let external = std::fs::read(f.file()).unwrap();
    assert!(f.ws.correct_emit_row(7, &request(2, 31, 420)).is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), external);
    assert_eq!(f.ws.project().unwrap(), &before);
}

#[test]
fn repeat_read_keeps_overlay_changed_card_does_not_inherit_it_and_raw_audit_survives() {
    let mut f = Fixture::new(project());
    f.complete_and_approve("12:10:00");
    f.ws.correct_emit_row(7, &request(2, 31, 420)).unwrap();
    let mut p = f.ws.project().unwrap().clone();
    let mut repeat = p.leimaluvut[0].clone();
    repeat.lukukello = "12:12:00".into();
    p.record_readout(repeat).unwrap();
    assert_eq!(
        emit_corrections::effective_readout(&p, p.active_readout(123456).unwrap())
            .unwrap()
            .leimat[1]
            .aika_sekuntia,
        420
    );
    let mut changed = p.leimaluvut[0].clone();
    changed.leimat[1].aika_sekuntia = 400;
    p.record_readout(changed).unwrap();
    assert_eq!(
        emit_corrections::effective_readout(&p, p.active_readout(123456).unwrap())
            .unwrap()
            .leimat[1]
            .aika_sekuntia,
        400
    );
    let reloaded = Project::from_toml(&p.to_toml().unwrap()).unwrap();
    assert_eq!(reloaded.leimaluvut[0].leimat[1].aika_sekuntia, 395);
    assert_eq!(reloaded.emit_korjaukset.len(), 1);
}

#[test]
fn correction_crosses_midnight_and_bc_view_cannot_target_the_wrong_nac_row() {
    let mut p = project();
    p.kilpailijat[0].lahtoaika = "23:59:00".into();
    p.leimaluvut[0].lukukello = "00:10:00".into();
    let mut f = Fixture::new(p);
    f.complete_and_approve("00:09:00");
    f.ws.set_bc_view(true).unwrap();
    assert!(f.ws.correct_emit_row(7, &request(2, 31, 420)).is_err());
    f.ws.set_bc_view(false).unwrap();
    f.ws.correct_emit_row(7, &request(2, 31, 420)).unwrap();
    let r = f.ws.result_snapshot(7).unwrap().unwrap();
    assert_eq!(r.aikavirheet, 0);
    assert_eq!(r.kesto_sekuntia, 600);
    // Nykyinen ajanottopolku säilyttää seuraavan vuorokauden eksplisiittisesti.
    assert_eq!(
        r.rivit
            .iter()
            .find(|r| r.expected_code == 31)
            .unwrap()
            .clock,
        [24, 4, 0]
    );
}

#[test]
fn tampered_history_is_rejected() {
    let mut f = Fixture::new(project());
    f.complete_and_approve("12:10:00");
    f.ws.correct_emit_row(7, &request(2, 31, 420)).unwrap();
    let mut p = f.ws.project().unwrap().clone();
    p.emit_korjaukset[0].ennen[1] = ReadPunch {
        koodi: 31,
        aika_sekuntia: 1,
    };
    assert!(p.to_toml().is_err());
}
