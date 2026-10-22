use as2026::{
    application, emit_corrections,
    native::{Project, ReadPunch},
    printing::{self, Kind},
    starts,
    workspace::{ClassEdit, ClassField, StationField, Workspace},
};
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    ws: Workspace,
    path: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "as2026-069-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let p = Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
        Self {
            ws: Workspace::create_imported(&path, p).unwrap(),
            path,
        }
    }
    fn complete(&mut self) {
        self.ws.ensure_readout_result(7).unwrap();
        let result = self.ws.result_snapshot(7).unwrap().unwrap();
        let finish = result
            .rivit
            .iter()
            .find(|r| r.station_type == "M")
            .unwrap()
            .output_index;
        self.ws
            .complete_result_entry(7, &[(finish, [12, 10, 0])], 0, 0)
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn whole_card_replace_adds_missing_removes_any_punch_preserves_manual_finish_and_reopens() {
    let mut f = Fixture::new();
    f.complete();
    let raw = f.ws.project().unwrap().leimaluvut.clone();
    f.ws.replace_emit_content(
        7,
        vec![
            ReadPunch {
                koodi: 31,
                aika_sekuntia: 420,
            },
            ReadPunch {
                koodi: 32,
                aika_sekuntia: 720,
            },
        ],
    )
    .unwrap();
    let result = f.ws.result_snapshot(7).unwrap().unwrap();
    assert_eq!((result.aikavirheet, result.asemien_muut_virheet), (0, 0));
    assert_eq!(result.maalikello_sekuntia, 12 * 3600 + 10 * 60);
    assert!(!result.hyvaksytty);
    assert_eq!(f.ws.project().unwrap().leimaluvut, raw);
    assert!(f.ws.project().unwrap().emit_korjaukset[0].syy.is_empty());
    let reopened = Workspace::open(f.ws.path()).unwrap();
    assert_eq!(reopened.result_snapshot(7).unwrap().unwrap(), result);
    // Removing even a matched control is allowed and produces the normal missing AT penalty.
    f.ws.replace_emit_content(
        7,
        vec![ReadPunch {
            koodi: 32,
            aika_sekuntia: 720,
        }],
    )
    .unwrap();
    assert_eq!(
        f.ws.result_snapshot(7)
            .unwrap()
            .unwrap()
            .asemien_muut_virheet,
        1800
    );
    f.ws.replace_emit_content(
        7,
        vec![
            ReadPunch {
                koodi: 31,
                aika_sekuntia: 420,
            },
            ReadPunch {
                koodi: 32,
                aika_sekuntia: 720,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        f.ws.result_snapshot(7)
            .unwrap()
            .unwrap()
            .asemien_muut_virheet,
        0
    );
}

#[test]
fn battery_markers_are_unscored_and_can_be_removed_without_reason() {
    let mut f = Fixture::new();
    f.complete();
    f.ws.replace_emit_content(
        7,
        vec![
            ReadPunch {
                koodi: 31,
                aika_sekuntia: 420,
            },
            ReadPunch {
                koodi: 99,
                aika_sekuntia: 421,
            },
            ReadPunch {
                koodi: 999,
                aika_sekuntia: 422,
            },
            ReadPunch {
                koodi: 32,
                aika_sekuntia: 720,
            },
        ],
    )
    .unwrap();
    let result = f.ws.result_snapshot(7).unwrap().unwrap();
    assert_eq!((result.aikavirheet, result.asemien_muut_virheet), (0, 0));
    let p = f.ws.project().unwrap();
    assert_eq!(
        emit_corrections::effective_readout(p, &p.leimaluvut[0])
            .unwrap()
            .content()
            .unwrap()
            .len(),
        5
    );
    f.ws.replace_emit_content(
        7,
        vec![
            ReadPunch {
                koodi: 31,
                aika_sekuntia: 420,
            },
            ReadPunch {
                koodi: 32,
                aika_sekuntia: 720,
            },
        ],
    )
    .unwrap();
    assert_eq!(f.ws.result_snapshot(7).unwrap().unwrap().rivit.len(), 4);
}

#[test]
fn trailing_battery_marker_does_not_change_readout_time() {
    use as2026::{ClockTime, Punch, emit};
    let records = [
        Punch {
            code: 0,
            elapsed_seconds: 0,
        },
        Punch {
            code: 31,
            elapsed_seconds: 420,
        },
        Punch {
            code: 999,
            elapsed_seconds: 500,
        },
        Punch {
            code: 250,
            elapsed_seconds: 600,
        },
    ];
    let read = emit::accepted_records(&records, ClockTime::new(12, 8, 0).unwrap()).unwrap();
    assert_eq!(read.last_punch_elapsed_seconds, 420);
    assert_eq!(read.finish_clock_seconds, 12 * 3600 + 5 * 60);
}

#[test]
fn classes_combined_classes_and_routes_remain_editable_after_scoring() {
    let mut f = Fixture::new();
    f.complete();
    f.ws.add_class(ClassEdit {
        code: "B".into(),
        name: "Toinen luokka".into(),
        route: "1".into(),
        first_number: "8".into(),
        start_time: "12:01:00".into(),
        interval_minutes: "1".into(),
        lateness_minutes: "60".into(),
        np_enabled: "0".into(),
        series_method: "0".into(),
    })
    .unwrap();
    f.ws.edit_combined_class(0, "Yhteistulos | A,B | 0 | 0")
        .unwrap();
    f.ws.edit_class("A", ClassField::Name, "Uusi nimi").unwrap();
    f.ws.edit_station(1, 1, StationField::Target, "200")
        .unwrap();
    // No implicit recalculation: old points survive until the operator chooses Yes.
    assert_eq!(f.ws.result_snapshot(7).unwrap().unwrap().aikavirheet, 45);
    f.ws.rebuild_route_results(&[1]).unwrap();
    let result = f.ws.result_snapshot(7).unwrap().unwrap();
    assert!(result.aikavirheet > 45);
    assert!(!result.hyvaksytty);
    assert_eq!(result.maalikello_sekuntia, 12 * 3600 + 10 * 60);
    f.ws.delete_class("B").unwrap();
    assert_eq!(f.ws.project().unwrap().yhdistetyt_luokat[0].luokat, ["A"]);
    assert!(f.ws.delete_class("A").is_err());
}

#[test]
fn selected_combined_print_contains_unread_drivers_and_start_list_sorts_across_classes() {
    let mut f = Fixture::new();
    f.complete();
    let mut p = f.ws.project().unwrap().clone();
    let mut class = p.luokat[0].clone();
    class.tunnus = "B".into();
    p.luokat.push(class);
    let mut driver = p.kilpailijat[0].clone();
    driver.numero = 8;
    driver.emit = 999123;
    driver.luokka = "B".into();
    driver.lahtoaika = "11:59:00".into();
    driver.tallennettu_tulos = None;
    p.kilpailijat.push(driver);
    p.yhdistetyt_luokat.push(as2026::native::CombinedClass {
        nimi: "Yhteistulos".into(),
        luokat: vec!["A".into(), "B".into()],
        nousupisteet: 0,
        sarjapistetapa: 0,
    });
    let archive = application::calculated_project(&p).unwrap();
    for kind in [Kind::Classes, Kind::Stations] {
        let doc =
            printing::classes(&archive, &["Y1: Yhteistulos".into()], kind, false, true).unwrap();
        assert_eq!(doc.pages.len(), 1);
        assert!(doc.pages[0].texts.iter().any(|t| t.value == "8"));
        assert!(doc.pages[0].texts.iter().any(|t| t.value == "AJAA"));
    }
    let html = starts::start_list_html(&p).unwrap();
    assert!(html.find("11:59:00").unwrap() < html.find("12:00:00").unwrap());
    let doc = printing::competitor(&p, 7, None).unwrap();
    assert!(
        doc.pages[0]
            .texts
            .iter()
            .any(|t| t.value == "ATp (AT pisteet)")
    );
}
