use as2026::{application, manual_timing, native};

fn project() -> native::Project {
    let mut p =
        native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    p.kilpailu.ajanottotapa = 0;
    p.leimaluvut.clear();
    p.kilpailijat[0].emit = 0;
    p
}

#[test]
fn paper_times_use_existing_scoring_without_a_card_or_a_readout() {
    let mut p = project();
    let original = p.clone();
    let mut r = application::capture_project_result(&p, 7).unwrap();
    assert!(r.ilman_emit);
    assert_eq!(r.emit_kortti, None);
    assert_eq!(r.pending_manual_clocks(), vec![2, 3, 4]);
    assert!(!r.can_approve());
    r.set_clock(2, [12, 4, 35]).unwrap();
    r.set_clock(3, [12, 10, 0]).unwrap();
    r.set_clock(4, [12, 10, 0]).unwrap();
    let mut r = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert_eq!(r.rivit[1].signed_time_penalty, 45);
    assert!(r.can_approve());
    application::validate_project_result(&p, 7, &r).unwrap();
    r.hyvaksytty = true;
    p.kilpailijat[0].tallennettu_tulos = Some(r);
    let a = application::calculated_project(&p).unwrap();
    assert_eq!(a.competitors[0].results[1].clock, [12, 4, 35]);
    assert!(a.has_approved_result(7));
    assert_eq!(
        native::Project::from_toml(&p.to_toml().unwrap()).unwrap(),
        p
    );
    assert_eq!(p.radat, original.radat);
    assert_eq!(p.leimaluvut, original.leimaluvut);
    p.kilpailu.ajanottotapa = 1;
    assert!(
        application::validate_project_result(
            &p,
            7,
            p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap()
        )
        .is_err()
    );
}

#[test]
fn missing_at_is_distinct_from_a_pending_time_and_costs_1800() {
    let p = project();
    let mut r = manual_timing::capture(&p, 7).unwrap();
    assert_eq!(manual_timing::missing_at_points(&p, &r).unwrap(), 0);
    manual_timing::set_missing(&mut r, 2).unwrap();
    r.set_clock(3, [12, 10, 0]).unwrap();
    r.set_clock(4, [12, 10, 0]).unwrap();
    let mut r = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert_eq!(r.tila, "TRK");
    assert!(r.can_approve());
    assert_eq!(manual_timing::missing_at_points(&p, &r).unwrap(), 1800);
    assert_eq!(r.rivit[1].signed_time_penalty, 0);
    r.set_clock(2, [12, 5, 0]).unwrap();
    let r = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert!(r.puuttuvat_aikamerkinnat.is_empty());
    assert_eq!(manual_timing::missing_at_points(&p, &r).unwrap(), 0);
}

#[test]
fn missing_public_control_and_finish_are_interruption_and_ula_is_unresolved() {
    for (kind, row) in [("JAT", 3), ("M", 4), ("ULA", 3)] {
        let mut p = project();
        if kind != "M" {
            p.radat[0].asemat[2].tyyppi = kind.into();
        }
        let mut r = manual_timing::capture(&p, 7).unwrap();
        r.set_clock(2, [12, 5, 0]).unwrap();
        if row != 3 {
            r.set_clock(3, [12, 10, 0]).unwrap();
        }
        if row != 4 {
            r.set_clock(4, [12, 10, 0]).unwrap();
        }
        manual_timing::set_missing(&mut r, row).unwrap();
        let r = application::recalculate_stored_result(&p, 7, &r).unwrap();
        assert_eq!(r.tila, if kind == "ULA" { "ILM" } else { "KESK" });
        if kind == "ULA" {
            assert!(!r.can_approve());
        }
        assert_eq!(manual_timing::missing_at_points(&p, &r).unwrap(), 0);
        if kind != "ULA" {
            let mut r = r;
            r.hyvaksytty = true;
            p.kilpailijat[0].tallennettu_tulos = Some(r);
            assert!(
                application::calculated_project(&p)
                    .unwrap()
                    .has_confirmed_interruption(7)
            );
        }
    }
}

#[test]
fn midnight_and_zero_clock_are_real_times_not_missing_values() {
    let mut p = project();
    p.kilpailijat[0].lahtoaika = "23:50:00".into();
    let mut r = manual_timing::capture(&p, 7).unwrap();
    r.set_clock(2, [23, 55, 0]).unwrap();
    r.set_clock(3, [0, 0, 0]).unwrap();
    r.set_clock(4, [0, 1, 0]).unwrap();
    let r = application::recalculate_stored_result(&p, 7, &r).unwrap();
    assert!(r.can_approve());
    assert_eq!(r.rivit[2].clock, [0, 0, 0]);
    assert_eq!(r.kesto_sekuntia, 660);
}

#[test]
fn operational_at_caps_preserve_early_and_late_signs() {
    for (clock, expected, note) in [
        ([12, 0, 0], 1800, "max ennakko"),
        ([12, 50, 0], -1800, "max myöhä"),
    ] {
        let mut p = project();
        p.radat[0].asemat[1].tavoiteaika_sekuntia = 1000;
        let mut r = manual_timing::capture(&p, 7).unwrap();
        r.set_clock(2, clock).unwrap();
        r.set_clock(3, [12, 55, 0]).unwrap();
        r.set_clock(4, [12, 55, 0]).unwrap();
        let r = application::recalculate_stored_result(&p, 7, &r).unwrap();
        assert_eq!(r.rivit[1].signed_time_penalty, expected);
        assert_eq!(r.rivit[1].note, note);
    }
}

#[test]
fn printed_missing_signs_are_grouped_and_continue_on_another_page() {
    let mut p = project();
    let mut r = manual_timing::capture(&p, 7).unwrap();
    for (row, clock) in [(2, [12, 5, 0]), (3, [12, 10, 0]), (4, [12, 10, 0])] {
        r.set_clock(row, clock).unwrap();
    }
    let mut r = application::recalculate_stored_result(&p, 7, &r).unwrap();
    r.hyvaksytty = true;
    p.kilpailijat[0].tallennettu_tulos = Some(r);
    p.kilpailijat[0].lisavirheet.tulkki_siirto = Some(native::TulkkiPenaltySource {
        tapahtumatunniste: None,
        tulkatut_jaksot: Vec::new(),
        jaksoja: 0,
        tunniste: "a".repeat(64),
        muut_virhepisteet: 0,
        puuttuvat_rt: (1..=80)
            .map(|jakso| native::MissingRtSection {
                jakso,
                kilvet: vec!["08".into(), "15".into()],
            })
            .collect(),
    });
    let doc = as2026::printing::competitor(&p, 7, None).unwrap();
    assert!(doc.pages.len() > 1);
    let texts = doc.pages.iter().flat_map(|p| &p.texts).collect::<Vec<_>>();
    assert!(texts.iter().any(|t| t.value == "Puuttuvat"));
    assert!(texts.iter().any(|t| t.value == "Jakso 80"));
    assert!(texts.iter().any(|t| t.value == "08, 15"));
    for page in &doc.pages {
        assert!(page.texts.iter().all(|t| t.y + t.height <= 297.));
    }
}
