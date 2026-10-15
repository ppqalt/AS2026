//! Synteettisiä sovelluspolkuja, jotka eivät suorita vanhaa ohjelmaa tai avaa laitteita.
use as2015_rs::application::{calculate, rank_stored};
use as2015_rs::emit::parse_frame;
use as2015_rs::legacy::{Archive, Encoding, parse_document, parse_saved_emit};
use as2015_rs::ranking::{GroupKind, TiePolicy};
use as2015_rs::timing::CalculationStatus;
use as2015_rs::{ClockTime, CoreError};
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/synthetic")
}
fn archive() -> Archive {
    Archive::load(&fixture(), Encoding::Windows1252).unwrap()
}
fn replace_log(archive: &mut Archive, raw: &[u8]) {
    archive.documents.insert(
        "emit.txt".into(),
        parse_document(raw, Encoding::Windows1252).unwrap(),
    );
}
fn calculation_error(archive: &Archive) -> String {
    let report = calculate(archive).unwrap();
    report.competitors[0]
        .calculation
        .as_ref()
        .unwrap_err()
        .to_string()
}

#[test]
fn imported_archive_saved_emit_and_matched_route_calculate_645_without_mutations() {
    let a = archive();
    let files = [
        "Aspara.txt",
        "Asperus.txt",
        "Radat.txt",
        "Kilp.txt",
        "emit.txt",
    ];
    let disk_before = files.map(|name| std::fs::read(fixture().join(name)).unwrap());
    let imported_before = format!("{a:?}");
    let stored_rows_before = format!("{:?}", a.competitors[0].results);
    let report = calculate(&a).unwrap();
    assert_eq!(report.competitors.len(), 1);
    let preview = &report.competitors[0];
    assert_eq!(preview.number, 7);
    assert_eq!(preview.imported_total, 645);
    let result = preview.calculation.as_ref().unwrap();
    assert_eq!(result.totals.time_points, 45);
    assert_eq!(result.totals.other_points, 600);
    assert_eq!(result.totals.total_points, 645);
    assert_eq!(result.status, CalculationStatus::Checked);
    assert_eq!(result.rows.len(), 5);
    assert_eq!(result.rows[1].source.punch.unwrap().elapsed_seconds, 395);
    assert_eq!(result.rows[1].actual_seconds, 275);
    assert_eq!(result.rows[2].received_code, 90);
    assert_eq!(result.finish_clock_seconds, 43_800);
    assert_eq!(format!("{a:?}"), imported_before);
    assert_eq!(
        format!("{:?}", a.competitors[0].results),
        stored_rows_before
    );
    for (name, before) in files.into_iter().zip(disk_before) {
        assert_eq!(std::fs::read(fixture().join(name)).unwrap(), before);
    }
}

#[test]
fn saved_emit_log_matches_raw_frame_readout_and_keeps_original_log_bytes() {
    let a = archive();
    let document = &a.documents["emit.txt"];
    let raw_before = document.raw.clone();
    let records_before = format!("{:?}", document.records);
    let logs = parse_saved_emit(document).unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].card_number, 123456);
    assert_eq!(logs[0].records.len(), 5);
    let decoded_frame =
        parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw")).unwrap();
    let expected = decoded_frame
        .accepted_readout(ClockTime::new(12, 11, 0).unwrap())
        .unwrap();
    assert_eq!(logs[0].accepted().unwrap(), expected);
    assert_eq!(document.raw, raw_before);
    assert_eq!(format!("{:?}", document.records), records_before);
}

#[test]
fn missing_card_readout_and_absent_log_are_explicit_per_competitor_errors() {
    let mut a = archive();
    a.competitors[0].emit_card = "999999".into();
    let message = calculation_error(&a);
    assert!(message.contains("999999") && message.contains("raakaleimauksia"));
    let mut a = archive();
    a.documents.remove("emit.txt");
    assert!(calculation_error(&a).contains("raakaleimauksia"));
}

#[test]
fn duplicate_card_is_never_assigned_by_competitor_order() {
    let mut a = archive();
    let mut second = a.competitors[0].clone();
    second.number = 8;
    second.header_fields[0] = "8".into();
    a.competitors.push(second);
    let before = format!("{a:?}");
    let report = calculate(&a).unwrap();
    assert_eq!(report.competitors.len(), 2);
    for preview in report.competitors {
        let error = preview.calculation.unwrap_err().to_string();
        assert!(error.contains("123456") && error.contains("usealle kilpailijalle"));
    }
    assert_eq!(format!("{a:?}"), before);
}

#[test]
fn latest_saved_readout_is_selected_with_a_note_and_stored_total_is_retained() {
    let mut a = archive();
    let raw = String::from_utf8(a.documents["emit.txt"].raw.clone()).unwrap();
    let latest = raw.replace("31;395;", "31;420;");
    assert_ne!(raw, latest);
    replace_log(&mut a, format!("{raw}{latest}").as_bytes());
    let before = format!("{a:?}");
    let report = calculate(&a).unwrap();
    let preview = &report.competitors[0];
    assert_eq!(preview.imported_total, 645);
    let result = preview.calculation.as_ref().unwrap();
    assert_eq!(result.rows[1].source.punch.unwrap().elapsed_seconds, 420);
    assert_eq!(result.totals.total_points, 600);
    assert!(
        preview
            .notes
            .iter()
            .any(|note| note.contains("2 lukua") && note.contains("riviltä 2"))
    );
    assert!(
        preview
            .notes
            .iter()
            .any(|note| note.contains("600") && note.contains("645") && note.contains("poikkeaa"))
    );
    assert_eq!(format!("{a:?}"), before);
    assert_eq!(a.competitors[0].total_penalties, 645);
}

#[test]
fn imported_total_mismatch_produces_a_note_without_replacing_the_original() {
    let mut a = archive();
    a.competitors[0].total_penalties = 999;
    let report = calculate(&a).unwrap();
    let preview = &report.competitors[0];
    assert_eq!(preview.imported_total, 999);
    assert_eq!(
        preview.calculation.as_ref().unwrap().totals.total_points,
        645
    );
    assert!(
        preview
            .notes
            .iter()
            .any(|note| note.contains("645") && note.contains("999") && note.contains("poikkeaa"))
    );
    assert_eq!(a.competitors[0].total_penalties, 999);
}

#[test]
fn legacy_timing_method_two_stays_explicitly_unsupported() {
    let mut a = archive();
    a.basic.identity[7] = "2".into();
    let report = calculate(&a).unwrap();
    assert!(matches!(
        report.competitors[0].calculation,
        Err(CoreError::Unsupported(_))
    ));
    assert!(
        report.competitors[0]
            .calculation
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("ajanottotavan 2")
    );
    assert_eq!(a.competitors[0].total_penalties, 645);
}

#[test]
fn malformed_saved_log_counts_cannot_shift_codes_into_clock_fields() {
    for raw in [
        "999;123456;0;12;11;0;\r\n",
        "999;123456;51;12;11;0;\r\n",
        "999;123456;2;0;0;12;11;0;\r\n",
        "999;123456;1;0;0;12;11;0;31;395;\r\n",
        "999;123456;-1;0;0;12;11;0;\r\n",
        "999;123456;not-a-count;0;0;12;11;0;\r\n",
    ] {
        let document = parse_document(raw.as_bytes(), Encoding::Windows1252).unwrap();
        assert!(parse_saved_emit(&document).is_err(), "{raw:?}");
    }
    let mut a = archive();
    replace_log(&mut a, b"999;123456;51;12;11;0;\r\n");
    assert!(calculate(&a).is_err());
}

#[test]
fn saved_log_structure_and_marker_acceptance_remain_separate_checks() {
    let doc = parse_document(
        b"999;123456;2;0;0;31;395;12;11;0;\r\n",
        Encoding::Windows1252,
    )
    .unwrap();
    let logs = parse_saved_emit(&doc).unwrap();
    assert_eq!(logs.len(), 1);
    assert!(matches!(logs[0].accepted(), Err(CoreError::MissingMarker)));
    let doc = parse_document(
        b"999;123456;2;250;0;31;395;12;11;0;\r\n",
        Encoding::Windows1252,
    )
    .unwrap();
    let logs = parse_saved_emit(&doc).unwrap();
    assert!(matches!(logs[0].accepted(), Err(CoreError::MarkerAtStart)));
}

#[test]
fn stored_ranking_uses_imported_totals_even_after_a_different_preview() {
    let mut a = archive();
    a.competitors[0].total_penalties = 999;
    let mut second = a.competitors[0].clone();
    second.number = 8;
    second.header_fields[0] = "8".into();
    second.emit_card = "123457".into();
    second.total_penalties = 700;
    a.competitors.push(second);
    let before = format!("{a:?}");
    let preview = calculate(&a).unwrap();
    assert_eq!(
        preview.competitors[0]
            .calculation
            .as_ref()
            .unwrap()
            .totals
            .total_points,
        645
    );
    let ranked = rank_stored(&a, TiePolicy::KeepSharedAndFlag).unwrap();
    let group = ranked
        .groups
        .iter()
        .find(|g| g.kind == GroupKind::Ordinary)
        .unwrap();
    assert_eq!(
        group
            .entries
            .iter()
            .find(|e| e.number == 8)
            .unwrap()
            .primary_position,
        1
    );
    assert_eq!(
        group
            .entries
            .iter()
            .find(|e| e.number == 7)
            .unwrap()
            .primary_position,
        2
    );
    assert_eq!(format!("{a:?}"), before);
}

#[test]
fn custom_series_method_requires_a_table_then_uses_its_first_place_value() {
    let mut a = archive();
    a.basic.classes[0].fields[9] = "4".into();
    a.documents.remove("sarjap.txt");
    let error = rank_stored(&a, TiePolicy::KeepSharedAndFlag)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Sp 4") && error.contains("sarjap.txt"));
    a.documents.insert(
        "sarjap.txt".into(),
        parse_document(b"101;55;20;\r\n", Encoding::Windows1252).unwrap(),
    );
    let ranked = rank_stored(&a, TiePolicy::KeepSharedAndFlag).unwrap();
    let group = ranked
        .groups
        .iter()
        .find(|g| g.kind == GroupKind::Ordinary)
        .unwrap();
    assert_eq!(group.entries[0].series_points, 101);
}
